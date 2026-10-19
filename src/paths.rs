use std::path::PathBuf;

pub const HISTORY_DIR: &str = "json";
pub const INPUT_DIR: &str = "input";

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
