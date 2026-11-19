use regex::Regex;
use r_name::rename::{build_plan, build_season_plan, execute};
use r_name::rules::{CaseMode, RuleSet, SortOrder};
use std::fs;
use std::path::PathBuf;

fn rules(replaces: Vec<(Regex, String)>, case: CaseMode) -> RuleSet {
    RuleSet {
        replaces,
        case,
        trim: false,
        by_date: false,
        number: false,
        ext: None,
        sort_by: r_name::rules::SortOrder::Name,
    }
}

fn write(dir: &std::path::Path, name: &str) -> PathBuf {
    let p = dir.join(name);
    fs::write(&p, b"x").unwrap();
    p
}

#[test]
fn test_number_and_ext() {
    let dir = tempfile::tempdir().unwrap();
    let a = write(dir.path(), "b.txt");
    let b = write(dir.path(), "a.txt");

    let mut rs = rules(vec![], CaseMode::None);
    rs.number = true;
    rs.ext = Some("md".into());

    let plan = build_plan(&[a, b], &rs);
    assert_eq!(plan.entries.len(), 2);
    let names: Vec<String> = plan
        .entries
        .iter()
        .map(|e| e.destino.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, vec!["a_0001.md", "b_0002.md"]);

    let report = execute(&plan);
    assert_eq!(report.renamed, 2, "{:?}", report.errors);
    assert!(dir.path().join("a_0001.md").exists());
    assert!(dir.path().join("b_0002.md").exists());
    assert!(!dir.path().join("a.txt").exists());
}

#[test]
fn test_collision_between_two_sources_gets_suffix() {
    let dir = tempfile::tempdir().unwrap();
    let a = write(dir.path(), "foo1.txt");
    let b = write(dir.path(), "foo2.txt");

    let rs = rules(
        vec![(Regex::new(r"foo\d").unwrap(), "f".to_string())],
        CaseMode::None,
    );
    let plan = build_plan(&[a, b], &rs);
    let names: Vec<String> = plan
        .entries
        .iter()
        .map(|e| e.destino.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, vec!["f.txt", "f (1).txt"]);
}

#[test]
fn test_never_overwrites_existing_file() {
    let dir = tempfile::tempdir().unwrap();
    let a = write(dir.path(), "a.txt");
    let existing = write(dir.path(), "target.txt");

    let rs = rules(
        vec![(Regex::new("^a$").unwrap(), "target".to_string())],
        CaseMode::None,
    );
    let plan = build_plan(&[a, existing.clone()], &rs);
    assert_eq!(plan.entries.len(), 1);
    assert_eq!(
        plan.entries[0].destino,
        dir.path().join("target (1).txt"),
        "debe respetar el archivo existente"
    );

    let report = execute(&plan);
    assert_eq!(report.renamed, 1, "{:?}", report.errors);
    assert_eq!(fs::read_to_string(&existing).unwrap(), "x");
    assert!(dir.path().join("target (1).txt").exists());
}

#[test]
fn test_swap_names_via_two_phase() {
    let dir = tempfile::tempdir().unwrap();
    let a = write(dir.path(), "a.txt");
    let b = write(dir.path(), "b.txt");

    let mut plan = build_plan(&[], &rules(vec![], CaseMode::None));
    plan.entries.push(r_name::rename::RenameEntry {
        origen: a.clone(),
        destino: b.clone(),
    });
    plan.entries.push(r_name::rename::RenameEntry {
        origen: b.clone(),
        destino: a.clone(),
    });

    let report = execute(&plan);
    assert_eq!(report.renamed, 2, "{:?}", report.errors);
    assert!(a.exists() && b.exists());
    assert_eq!(fs::read_to_string(&a).unwrap(), "x");
}

#[test]
fn test_season_plan_naming() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    std::fs::create_dir_all(root.join("t1")).unwrap();
    std::fs::create_dir_all(root.join("t2")).unwrap();
    std::fs::write(root.join("t1/video.mkv"), b"x").unwrap();
    std::fs::write(root.join("t1/intro.mp4"), b"x").unwrap();
    std::fs::write(root.join("t2/capitulo.avi"), b"x").unwrap();

    let files: Vec<std::path::PathBuf> = vec![
        root.join("t1/video.mkv"),
        root.join("t1/intro.mp4"),
        root.join("t2/capitulo.avi"),
    ];

    let plan = build_season_plan(&files, root, "episodio", SortOrder::Name);
    assert_eq!(plan.skipped.len(), 0, "{:?}", plan.skipped);
    assert_eq!(plan.entries.len(), 3);

    let mut names: Vec<String> = plan
        .entries
        .iter()
        .map(|e| e.destino.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    names.sort();

    // t1: intro.mp4 (1), video.mkv (2) ; t2: capitolo.avi (1) → T2 1 episodio.avi
    let expected = ["1 episodio.mp4", "2 episodio.mkv", "T2 1 episodio.avi"];
    for e in &expected {
        assert!(names.contains(&e.to_string()), "falta {e} en {names:?}");
    }
}

#[test]
fn test_season_plan_uses_one_title_per_series() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    for f in [
        "serieA/t1/a.mkv",
        "serieA/t1/b.mkv",
        "serieB/t1/c.mkv",
        "serieB/t2/d.mkv",
    ] {
        let p = root.join(f);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, b"x").unwrap();
    }

    let files = vec![
        root.join("serieA/t1/b.mkv"),
        root.join("serieA/t1/a.mkv"),
        root.join("serieB/t1/c.mkv"),
        root.join("serieB/t2/d.mkv"),
    ];
    let plan = build_season_plan(&files, root, "episodio", SortOrder::Name);
    assert!(plan.skipped.is_empty(), "{:?}", plan.skipped);
    assert_eq!(plan.entries.len(), 4);

    let actual: std::collections::HashSet<(String, String)> = plan
        .entries
        .iter()
        .map(|e| {
            let folder = e
                .destino
                .parent()
                .unwrap()
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .into_owned();
            let name = e.destino.file_name().unwrap().to_string_lossy().into_owned();
            (folder, name)
        })
        .collect();
    let expected: std::collections::HashSet<(String, String)> = [
        ("serieA/t1", "1 serieA.mkv"),
        ("serieA/t1", "2 serieA.mkv"),
        ("serieB/t1", "1 serieB.mkv"),
        ("serieB/t2", "T2 1 serieB.mkv"),
    ]
    .into_iter()
    .map(|(a, b)| (a.to_string(), b.to_string()))
    .collect();
    assert_eq!(actual, expected);
}

#[test]
fn test_season_plan_loose_files_are_season_one() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let a = write(root, "cap2.mkv");
    let b = write(root, "cap1.mkv");

    let plan = build_season_plan(&[a, b], root, "serie", SortOrder::Name);
    assert!(plan.skipped.is_empty(), "{:?}", plan.skipped);
    let names: Vec<String> = plan
        .entries
        .iter()
        .map(|e| e.destino.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, vec!["1 serie.mkv", "2 serie.mkv"]);
    assert!(plan.entries.iter().all(|e| e.destino.parent() == Some(root)));
}

#[test]
fn test_season_plan_folder_without_number_is_season_one() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let folder = root.join("serie");
    fs::create_dir_all(&folder).unwrap();
    let a = write(&folder, "cap2.mkv");
    let b = write(&folder, "cap1.mkv");

    let plan = build_season_plan(&[a, b], root, "titulo", SortOrder::Name);
    assert!(plan.skipped.is_empty(), "{:?}", plan.skipped);
    let names: Vec<String> = plan
        .entries
        .iter()
        .map(|e| e.destino.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, vec!["1 titulo.mkv", "2 titulo.mkv"]);
    assert!(plan.entries.iter().all(|e| e.destino.parent() == Some(folder.as_path())));
}

#[test]
fn test_season_plan_multiple_series_without_season_folders() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    for f in ["alpha/a.mkv", "beta/b.mkv"] {
        let p = root.join(f);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, b"x").unwrap();
    }

    let files = vec![root.join("alpha/a.mkv"), root.join("beta/b.mkv")];
    let plan = build_season_plan(&files, root, "episodio", SortOrder::Name);
    assert!(plan.skipped.is_empty(), "{:?}", plan.skipped);
    let names: Vec<String> = plan
        .entries
        .iter()
        .map(|e| e.destino.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, vec!["1 alpha.mkv", "1 beta.mkv"]);
}
