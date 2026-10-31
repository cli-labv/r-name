use regex::Regex;
use r_name::rules::{self, CaseMode, RuleSet};
use std::path::Path;

fn rs(replaces: Vec<(Regex, String)>, case: CaseMode, trim: bool) -> RuleSet {
    RuleSet {
        replaces,
        case,
        trim,
        by_date: false,
        number: false,
        ext: None,
        sort_by: r_name::rules::SortOrder::Name,
    }
}

fn re(pattern: &str, rep: &str) -> (Regex, String) {
    (Regex::new(pattern).unwrap(), rep.to_string())
}

#[test]
fn test_replace_regex_on_stem() {
    let rules = rs(vec![re(r"IMG_|DSC_", "")], CaseMode::None, false);
    assert_eq!(rules.apply(Path::new("IMG_0001.jpg")), "0001");
    assert_eq!(rules.apply(Path::new("DSC_42.png")), "42");
}

#[test]
fn test_case_modes() {
    let rules = rs(vec![], CaseMode::Lower, false);
    assert_eq!(rules.apply(Path::new("MiFoto.JPG")), "mifoto");
    let rules = rs(vec![], CaseMode::Upper, false);
    assert_eq!(rules.apply(Path::new("mifoto.jpg")), "MIFOTO");
}

#[test]
fn test_trim_cleans_spaces_and_weird_chars() {
    assert_eq!(rules::clean("  Hola  Mundo!!  "), "Hola-Mundo");
    assert_eq!(rules::clean("año--ñ  b"), "a-o-b");
    assert_eq!(rules::clean("--##--"), "");
    assert_eq!(rules::clean("a-b"), "a-b");
    let rules = rs(vec![], CaseMode::None, true);
    assert_eq!(rules.apply(Path::new(" informe  final #2.pdf")), "informe-final-2");
}

#[test]
fn test_parse_replace() {
    let (re, rep) = rules::parse_replace("foo=bar").unwrap();
    assert_eq!(re.replace_all("foo foo", rep.as_str()), "bar bar");
    assert!(rules::parse_replace("sin_igual").is_err());
    assert!(rules::parse_replace("[=x").is_err());
}

#[test]
fn test_by_date_uses_mtime_prefix() {
    let mut rules = rs(vec![], CaseMode::None, false);
    rules.by_date = true;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("foto.jpg");
    std::fs::write(&path, b"x").unwrap();
    let name = rules.apply(&path);
    assert!(
        regex::Regex::new(r"^\d{4}-\d{2}-\d{2}_foto$")
            .unwrap()
            .is_match(&name),
        "nombre inesperado: {name}"
    );
}
