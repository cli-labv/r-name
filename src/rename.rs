use crate::rules::{RuleSet, SortOrder};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct RenameEntry {
    pub origen: PathBuf,
    pub destino: PathBuf,
}

#[derive(Debug, Default)]
pub struct RenamePlan {
    pub entries: Vec<RenameEntry>,
    pub skipped: Vec<String>,
}

#[derive(Debug, Default)]
pub struct RenameReport {
    pub renamed: usize,
    pub errors: Vec<String>,
}

pub fn build_plan(files: &[PathBuf], rules: &RuleSet) -> RenamePlan {
    let mut sorted = files.to_vec();
    sort_files(&mut sorted, rules.sort_by);

    let mut plan = RenamePlan::default();
    let mut desired: Vec<RenameEntry> = Vec::new();

    for (i, path) in sorted.iter().enumerate() {
        let stem = rules.apply(path);
        if stem.is_empty() {
            plan.skipped
                .push(format!("{}: el nombre quedaría vacío", path.display()));
            continue;
        }
        let mut name = if rules.number {
            format!("{}_{:04}", stem, i + 1)
        } else {
            stem
        };
        match &rules.ext {
            Some(ext) => {
                name.push('.');
                name.push_str(ext.trim_start_matches('.'));
            }
            None => {
                if let Some(ext) = path.extension() {
                    name.push('.');
                    name.push_str(&ext.to_string_lossy());
                }
            }
        }
        let destino = path.with_file_name(&name);
        if &destino == path {
            continue;
        }
        desired.push(RenameEntry {
            origen: path.clone(),
            destino,
        });
    }

    let moving: HashSet<PathBuf> = desired.iter().map(|e| e.origen.clone()).collect();
    let mut used: HashSet<PathBuf> = HashSet::new();
    for mut e in desired {
        e.destino = resolve_collision(e.destino, &used, &moving);
        used.insert(e.destino.clone());
        plan.entries.push(e);
    }
    plan
}

pub fn execute(plan: &RenamePlan) -> RenameReport {
    let mut report = RenameReport::default();
    let mut pending: Vec<(PathBuf, PathBuf, PathBuf)> = Vec::new();

    for (i, e) in plan.entries.iter().enumerate() {
        let temp = e.origen.with_file_name(format!(".r-name-tmp-{i}"));
        match fs::rename(&e.origen, &temp) {
            Ok(()) => pending.push((temp, e.origen.clone(), e.destino.clone())),
            Err(err) => report
                .errors
                .push(format!("{}: {}", e.origen.display(), err)),
        }
    }

    for (temp, origen, destino) in &pending {
        match fs::rename(temp, destino) {
            Ok(()) => report.renamed += 1,
            Err(err) => {
                report.errors.push(format!(
                    "{} -> {}: {}",
                    origen.display(),
                    destino.display(),
                    err
                ));
                let _ = fs::rename(temp, origen);
            }
        }
    }
    report
}

fn occupied(path: &Path, used: &HashSet<PathBuf>, moving: &HashSet<PathBuf>) -> bool {
    used.contains(path) || (path.exists() && !moving.contains(path))
}

fn resolve_collision(
    target: PathBuf,
    used: &HashSet<PathBuf>,
    moving: &HashSet<PathBuf>,
) -> PathBuf {
    if !occupied(&target, used, moving) {
        return target;
    }
    let stem = target
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let ext = target
        .extension()
        .map(|s| format!(".{}", s.to_string_lossy()))
        .unwrap_or_default();
    let mut n = 1u32;
    loop {
        let candidate = target.with_file_name(format!("{stem} ({n}){ext}"));
        if !occupied(&candidate, used, moving) {
            return candidate;
        }
        n += 1;
    }
}

/// Tiempo de creación del archivo, con fallback a modificación.
fn file_time(path: &Path) -> std::time::SystemTime {
    std::fs::metadata(path)
        .and_then(|m| m.created())
        .unwrap_or_else(|_| mtime(path))
}

/// Tiempo de última modificación del archivo.
fn mtime(path: &Path) -> std::time::SystemTime {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .unwrap_or(std::time::UNIX_EPOCH)
}

/// Extrae el número de temporada del nombre de una carpeta (ej. "t1" → 1, "t2" → 2, "season 01" → 1).
pub fn season_number(folder: &str) -> Option<u32> {
    let re = Regex::new(r"(\d+)").ok()?;
    re.captures(folder)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse::<u32>().ok())
}

/// Nombre de la serie: primera carpeta bajo la raíz, salvo que ya sea una temporada.
/// `None` si el archivo está directamente en la raíz o bajo una carpeta numerada.
pub fn series_folder(path: &Path, root: &Path) -> Option<String> {
    let parent = path.parent()?;
    if parent == root {
        return None;
    }
    let rel = parent.strip_prefix(root).ok()?;
    let Component::Normal(name) = rel.components().next()? else {
        return None;
    };
    let name = name.to_string_lossy().into_owned();
    if season_number(&name).is_some() {
        None
    } else {
        Some(name)
    }
}

/// Nombres de las series detectadas, ordenados y sin duplicados.
pub fn detected_series(files: &[PathBuf], root: &Path) -> Vec<String> {
    let mut names: Vec<String> = files.iter().filter_map(|p| series_folder(p, root)).collect();
    names.sort();
    names.dedup();
    names
}

/// Temporada de un archivo. Sin carpeta numerada se trata como temporada 1.
fn season_of(path: &Path, root: &Path) -> u32 {
    path.parent()
        .filter(|parent| *parent != root)
        .and_then(|parent| parent.file_name())
        .and_then(|name| season_number(&name.to_string_lossy()))
        .unwrap_or(1)
}

fn sort_files(files: &mut [PathBuf], sort_by: SortOrder) {
    match sort_by {
        SortOrder::Name => files.sort(),
        SortOrder::NameDesc => files.sort_by(|a, b| b.cmp(a)),
        SortOrder::CreatedDesc => files.sort_by(|a, b| file_time(b).cmp(&file_time(a))),
        SortOrder::ModifiedAsc => files.sort_by(|a, b| mtime(a).cmp(&mtime(b))),
        SortOrder::ModifiedDesc => files.sort_by(|a, b| mtime(b).cmp(&mtime(a))),
    }
}

/// Construye un plan de renombrado agrupado por serie y temporada.
///
/// Cada carpeta de primer nivel bajo la raíz es una serie, salvo que ya tenga
/// número (en ese caso la serie es la propia raíz). Dentro de cada serie, las
/// carpetas con número son temporadas; los archivos sin carpeta numerada se
/// tratan como temporada 1 y se renombran en su sitio, sin crear subcarpetas.
///
/// Con una sola serie se usa `title`. Con varias, cada una toma el nombre de su
/// carpeta. Los archivos se renombran como `{episodio} {titulo}.{ext}` (T2, T3…
/// delante desde la segunda temporada), ordenados según `sort_by`.
pub fn build_season_plan(
    files: &[PathBuf],
    root: &Path,
    title: &str,
    sort_by: SortOrder,
) -> RenamePlan {
    let use_series_titles = detected_series(files, root).len() > 1;

    let mut groups: HashMap<(Option<String>, u32), Vec<PathBuf>> = HashMap::new();
    for path in files {
        let key = (series_folder(path, root), season_of(path, root));
        groups.entry(key).or_default().push(path.clone());
    }

    let mut keys: Vec<(Option<String>, u32)> = groups.keys().cloned().collect();
    keys.sort_by(|a, b| match (&a.0, &b.0) {
        (Some(x), Some(y)) => x.cmp(y).then(a.1.cmp(&b.1)),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => a.1.cmp(&b.1),
    });

    let mut entries: Vec<RenameEntry> = Vec::new();
    let skipped: Vec<String> = Vec::new();

    for (series, season) in keys {
        let mut group = groups.remove(&(series.clone(), season)).unwrap_or_default();
        sort_files(&mut group, sort_by);

        let group_title = if use_series_titles {
            series.unwrap_or_else(|| title.to_string())
        } else {
            title.to_string()
        };
        let group_title = if group_title.trim().is_empty() {
            "episodio".to_string()
        } else {
            group_title
        };

        for (i, path) in group.iter().enumerate() {
            let ext = path
                .extension()
                .map(|e| format!(".{}", e.to_string_lossy()))
                .unwrap_or_default();
            let episode = i + 1;
            let name = if season == 1 {
                format!("{episode} {group_title}{ext}")
            } else {
                format!("T{season} {episode} {group_title}{ext}")
            };
            let destino = path.with_file_name(&name);
            if &destino == path {
                continue;
            }
            entries.push(RenameEntry {
                origen: path.clone(),
                destino,
            });
        }
    }

    let moving: HashSet<PathBuf> = entries.iter().map(|e| e.origen.clone()).collect();
    let mut used: HashSet<PathBuf> = HashSet::new();
    let mut final_entries: Vec<RenameEntry> = Vec::new();
    for mut e in entries {
        e.destino = resolve_collision(e.destino, &used, &moving);
        used.insert(e.destino.clone());
        final_entries.push(e);
    }

    RenamePlan {
        entries: final_entries,
        skipped,
    }
}
