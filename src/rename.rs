use crate::rules::RuleSet;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
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
    sorted.sort();

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
        let temp = e.origen.with_file_name(format!(".renamr-tmp-{i}"));
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
