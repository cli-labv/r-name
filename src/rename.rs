use crate::rules::{RuleSet, SortOrder};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

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
    match rules.sort_by {
        SortOrder::Name => sorted.sort(),
        SortOrder::NameDesc => sorted.sort_by(|a, b| b.cmp(a)),
        SortOrder::CreatedDesc => {
            sorted.sort_by(|a, b| file_time(b).cmp(&file_time(a)));
        }
        SortOrder::ModifiedAsc => {
            sorted.sort_by(|a, b| mtime(a).cmp(&mtime(b)));
        }
        SortOrder::ModifiedDesc => {
            sorted.sort_by(|a, b| mtime(b).cmp(&mtime(a)));
        }
    }

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

/// Construye un plan de renombrado agrupado por carpeta/temporada.
///
/// Cada carpeta padre inmediata con número es una temporada (t1, t2, …).
/// Los archivos se renombran como `{episodio} {titulo}.{ext}` donde `episodio`
/// es el número secuencial dentro de esa temporada, ordenados según `sort_by`.
/// Se conserva la extensión original.
pub fn build_season_plan(
    files: &[PathBuf],
    root: &Path,
    title: &str,
    sort_by: SortOrder,
) -> RenamePlan {
    let mut by_season: HashMap<u32, Vec<PathBuf>> = HashMap::new();
    let mut no_season: Vec<PathBuf> = Vec::new();

    for path in files {
        let parent = match path.parent() {
            Some(p) => p,
            None => {
                no_season.push(path.clone());
                continue;
            }
        };

        // Ignorar la carpeta raíz misma
        if parent == root {
            no_season.push(path.clone());
            continue;
        }

        let folder_name = parent
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();

        match season_number(&folder_name) {
            Some(n) => by_season.entry(n).or_default().push(path.clone()),
            None => no_season.push(path.clone()),
        }
    }

    let mut entries: Vec<RenameEntry> = Vec::new();
    let mut skipped: Vec<String> = Vec::new();

    // Procesar temporadas ordenadas por número
    let mut seasons: Vec<u32> = by_season.keys().copied().collect();
    seasons.sort();

    for season in seasons {
        let mut group = by_season.remove(&season).unwrap_or_default();

        // Ordenar archivos dentro de la temporada según el criterio elegido
        match sort_by {
            SortOrder::Name => group.sort(),
            SortOrder::NameDesc => group.sort_by(|a, b| b.cmp(a)),
            SortOrder::CreatedDesc => {
                group.sort_by(|a, b| file_time(b).cmp(&file_time(a)));
            }
            SortOrder::ModifiedAsc => {
                group.sort_by(|a, b| mtime(a).cmp(&mtime(b)));
            }
            SortOrder::ModifiedDesc => {
                group.sort_by(|a, b| mtime(b).cmp(&mtime(a)));
            }
        }

        for (i, path) in group.iter().enumerate() {
            let ext = path
                .extension()
                .map(|e| format!(".{}", e.to_string_lossy()))
                .unwrap_or_default();
            let episode = i + 1;
            let name = if season == 1 {
                // Temporada 1: solo número al inicio, luego el título.
                format!("{episode} {title}{ext}")
            } else {
                // Temporada 2 en adelante: indicador de temporada, número, título.
                format!("T{season} {episode} {title}{ext}")
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

    // Archivos sin temporada reconocida: se dejan como están (skip)
    for p in &no_season {
        skipped.push(format!(
            "{}: carpeta sin número de temporada reconocida",
            p.display()
        ));
    }

    // Resolver colisiones (por si dos archivos quedan con mismo nombre en misma carpeta)
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
