use crate::paths;
use std::fs;

#[derive(Default)]
pub struct CleanReport {
    pub logs: usize,
    pub errors: Vec<String>,
}

pub fn clean() -> CleanReport {
    let mut report = CleanReport::default();
    let history = paths::history_dir();
    if !history.exists() {
        return report;
    }
    report.logs = paths::all_logs().len();
    if let Err(e) = fs::remove_dir_all(&history) {
        report.errors.push(format!("{}: {}", history.display(), e));
    }
    report
}
