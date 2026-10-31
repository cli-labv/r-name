use crate::rules::SortOrder;
use std::path::PathBuf;

pub const HISTORY_DIR: &str = "json";
pub const INPUT_DIR: &str = "input";
pub const ENV_FILE: &str = ".env";

/// Carpeta base de la app. Todo lo que genera la app queda aquí, sin tocar el
/// home ni el PATH del sistema. Se puede fijar con `RENAMR_HOME`.
pub fn home() -> PathBuf {
    std::env::var_os("RENAMR_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn history_dir() -> PathBuf {
    home().join(HISTORY_DIR)
}

pub fn default_input() -> PathBuf {
    home().join(INPUT_DIR)
}

fn is_log(name: &str) -> bool {
    name.starts_with("renamr-undo-") && name.ends_with(".json")
}

pub fn all_logs() -> Vec<PathBuf> {
    let dir = history_dir();
    let mut logs: Vec<PathBuf> = std::fs::read_dir(&dir)
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| is_log(&e.file_name().to_string_lossy()))
                .map(|e| e.path())
                .collect()
        })
        .unwrap_or_default();
    logs.sort();
    logs
}

pub fn latest_log() -> Option<PathBuf> {
    all_logs().pop()
}

pub fn log_path() -> PathBuf {
    history_dir().join(format!("renamr-undo-{}.json", timestamp()))
}

pub fn timestamp() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
        .to_string()
}

/// Carga variables de entorno desde el archivo `.env` de la carpeta de la app.
/// Las variables ya definidas en el entorno no se sobrescriben.
pub fn load_env() {
    let env_path = home().join(ENV_FILE);
    let Ok(contents) = std::fs::read_to_string(&env_path) else {
        return;
    };
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim().trim_matches('"').trim_matches('\'');
        if key.is_empty() {
            continue;
        }
        if std::env::var(key).is_err() {
            let _ = std::env::set_var(key, value);
        }
    }
}

/// Parsea el orden de renombrado desde una variable de entorno.
pub fn parse_sort_order(value: &str) -> SortOrder {
    match value.trim().to_lowercase().as_str() {
        "name" => SortOrder::Name,
        "name-desc" | "namedesc" => SortOrder::NameDesc,
        "created-desc" | "createddesc" => SortOrder::CreatedDesc,
        "modified-asc" | "modifiedasc" => SortOrder::ModifiedAsc,
        "modified-desc" | "modifieddesc" => SortOrder::ModifiedDesc,
        _ => SortOrder::Name,
    }
}
