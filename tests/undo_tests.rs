use regex::Regex;
use renamr::rename::{build_plan, execute};
use renamr::rules::{CaseMode, RuleSet};
use renamr::undo::{undo, write_log, UndoLog};
use std::fs;
use std::path::PathBuf;

fn rules(replaces: Vec<(Regex, String)>, case: CaseMode, number: bool) -> RuleSet {
    RuleSet {
        replaces,
        case,
        trim: false,
        by_date: false,
        number,
        ext: None,
    }
}

#[test]
fn test_undo_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let a = root.join("Foto Vieja.png");
    let b = root.join("IMG_0002.png");
    fs::write(&a, b"contenido-a").unwrap();
    fs::write(&b, b"contenido-b").unwrap();

    let rs = rules(
        vec![(Regex::new(r"IMG_").unwrap(), "".to_string())],
        CaseMode::Lower,
        true,
    );
    let plan = build_plan(&[a.clone(), b.clone()], &rs);
    assert_eq!(plan.entries.len(), 2);

    let log = UndoLog {
        fecha: "0".into(),
        revertido_en: None,
        carpeta_escaneada: root.to_path_buf(),
        renames: plan.entries.iter().map(Into::into).collect(),
    };
    let log_path: PathBuf = root.join("log.json");
    write_log(&log_path, &log).unwrap();

    let report = execute(&plan);
    assert_eq!(report.renamed, 2, "{:?}", report.errors);
    assert!(!a.exists());
    assert!(!b.exists());

    let restored = undo(&log_path).unwrap();
    assert_eq!(restored.restored, 2, "{:?}", restored.errors);
    assert!(a.exists());
    assert!(b.exists());
    assert_eq!(fs::read_to_string(&a).unwrap(), "contenido-a");
    assert_eq!(fs::read_to_string(&b).unwrap(), "contenido-b");

    let again = undo(&log_path).unwrap();
    assert_eq!(again.restored, 0, "ya revertido no debe tocar nada");
}

#[test]
fn test_undo_skips_when_target_occupied() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let orig = root.join("viejo.txt");
    let nuevo = root.join("nuevo.txt");
    fs::write(&nuevo, b"renamed").unwrap();
    fs::write(&orig, b"conflicto").unwrap();

    let log = UndoLog {
        fecha: "0".into(),
        revertido_en: None,
        carpeta_escaneada: root.to_path_buf(),
        renames: vec![renamr::undo::LogEntry {
            origen: orig.clone(),
            destino: nuevo.clone(),
            revertido: false,
        }],
    };
    let log_path = root.join("log.json");
    write_log(&log_path, &log).unwrap();

    let report = undo(&log_path).unwrap();
    assert_eq!(report.restored, 0);
    assert_eq!(report.errors.len(), 1);
    assert!(report.errors[0].contains("no se sobrescribe"));
    assert_eq!(fs::read_to_string(&orig).unwrap(), "conflicto");
    assert_eq!(fs::read_to_string(&nuevo).unwrap(), "renamed");
}
