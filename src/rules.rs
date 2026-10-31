use regex::Regex;
use std::path::Path;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CaseMode {
    None,
    Lower,
    Upper,
}

/// Orden de recorrido de los archivos antes de renombrar.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum SortOrder {
    #[default]
    Name,
    NameDesc,
    CreatedDesc,
    ModifiedAsc,
    ModifiedDesc,
}

pub struct RuleSet {
    pub replaces: Vec<(Regex, String)>,
    pub case: CaseMode,
    pub trim: bool,
    pub by_date: bool,
    pub number: bool,
    pub ext: Option<String>,
    pub sort_by: SortOrder,
}

impl RuleSet {
    pub fn is_empty(&self) -> bool {
        self.replaces.is_empty()
            && self.case == CaseMode::None
            && !self.trim
            && !self.by_date
            && !self.number
            && self.ext.is_none()
    }

    pub fn apply(&self, path: &Path) -> String {
        let mut stem = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        for (re, rep) in &self.replaces {
            stem = re.replace_all(&stem, rep.as_str()).into_owned();
        }
        match self.case {
            CaseMode::Upper => stem = stem.to_uppercase(),
            CaseMode::Lower => stem = stem.to_lowercase(),
            CaseMode::None => {}
        }
        if self.trim {
            stem = clean(&stem);
        }
        if self.by_date {
            stem = format!("{}_{}", date_prefix(path), stem);
        }
        stem
    }
}

pub fn parse_replace(spec: &str) -> Result<(Regex, String), String> {
    let (find, rep) = spec
        .split_once('=')
        .ok_or_else(|| format!("--replace inválido (use buscar=reemplazo): {spec}"))?;
    let re = Regex::new(find).map_err(|e| format!("regex inválida ({find}): {e}"))?;
    Ok((re, rep.to_string()))
}

pub fn clean(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut dash = false;
    for c in name.chars() {
        if c.is_ascii_alphanumeric() || c == '.' || c == '_' {
            out.push(c);
            dash = false;
        } else if !dash && !out.is_empty() {
            out.push('-');
            dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}

fn date_prefix(path: &Path) -> String {
    exif_date(path).unwrap_or_else(|| mtime_date(path))
}

fn mtime_date(path: &Path) -> String {
    let secs = std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (y, mo, d) = civil_from_days(secs.div_euclid(86_400) as i64);
    format!("{y:04}-{mo:02}-{d:02}")
}

fn exif_date(path: &Path) -> Option<String> {
    use exif::{In, Tag, Value};
    let file = std::fs::File::open(path).ok()?;
    let mut reader = std::io::BufReader::new(file);
    let exif = exif::Reader::new().read_from_container(&mut reader).ok()?;
    for tag in [Tag::DateTimeOriginal, Tag::DateTime] {
        let Some(field) = exif.get_field(tag, In::PRIMARY) else {
            continue;
        };
        if let Value::Ascii(list) = &field.value {
            let raw = String::from_utf8(list.first()?.clone()).ok()?;
            if raw.len() >= 10 {
                return Some(raw[..10].replace(':', "-"));
            }
        }
    }
    None
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}
