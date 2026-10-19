use crate::paths;
use crate::rename::RenameEntry;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LogEntry {
    pub origen: PathBuf,
    pub destino: PathBuf,
    #[serde(default)]
    pub revertido: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UndoLog {
    pub fecha: String,
    #[serde(default)]
    pub revertido_en: Option<String>,
    pub carpeta_escaneada: PathBuf,
    pub renames: Vec<LogEntry>,
}

impl From<&RenameEntry> for LogEntry {
    fn from(e: &RenameEntry) -> Self {
        Self {
            origen: e.origen.clone(),
            destino: e.destino.clone(),
            revertido: false,
        }
    }
}

#[derive(Debug, Default)]
pub struct UndoReport {
    pub restored: usize,
    pub errors: Vec<String>,
}

pub fn write_log(path: &Path, log: &UndoLog) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
    }
    let json = serde_json::to_string_pretty(log).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())
}

pub fn undo(log_path: &Path) -> Result<UndoReport, String> {
    let data = fs::read_to_string(log_path).map_err(|e| e.to_string())?;
    let mut log: UndoLog = serde_json::from_str(&data).map_err(|e| e.to_string())?;

    let mut report = UndoReport::default();
    for e in log.renames.iter_mut().rev() {
        if e.revertido || !e.destino.exists() {
            continue;
        }
        if e.origen.exists() {
            report
                .errors
                .push(format!("ya existe {}: no se sobrescribe", e.origen.display()));
            continue;
        }
        match fs::rename(&e.destino, &e.origen) {
            Ok(()) => {
                e.revertido = true;
                report.restored += 1;
            }
            Err(err) => report.errors.push(format!(
                "{} -> {}: {}",
                e.destino.display(),
                e.origen.display(),
                err
            )),
        }
    }

    if report.restored > 0 {
        log.revertido_en = Some(paths::timestamp());
        write_log(log_path, &log)?;
    }
    Ok(report)
}

pub fn undo_latest() -> Result<UndoReport, String> {
    let log = paths::latest_log()
        .ok_or_else(|| "no hay historial de renombrados para revertir".to_string())?;
    undo(&log)
}
