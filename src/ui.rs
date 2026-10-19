use crate::paths;
use crate::rename::{self, RenamePlan};
use crate::rules::RuleSet;
use crate::table::{Align, Table};
use crate::undo::{self, LogEntry, UndoLog};
use console::style;
use dialoguer::{theme::ColorfulTheme, Confirm};
use std::io::{self, IsTerminal};
use std::path::{Path, PathBuf};

pub struct RunConfig {
    pub root: PathBuf,
    pub rules: RuleSet,
    pub dry_run: bool,
    pub yes: bool,
}

pub fn scan(root: &Path) -> Vec<PathBuf> {
    let history = paths::history_dir();
    jwalk::WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_file() && !p.starts_with(&history))
        .collect()
}

pub fn run(cfg: &RunConfig) -> io::Result<()> {
    let files = scan(&cfg.root);
    if files.is_empty() {
        println!("{}", style("Sin archivos para renombrar.").yellow());
        return Ok(());
    }

    let plan = rename::build_plan(&files, &cfg.rules);
    for s in &plan.skipped {
        eprintln!("{}", style(format!("  - {s}")).red());
    }
    if plan.entries.is_empty() {
        println!("{}", style("Sin cambios.").yellow());
        return Ok(());
    }

    print_plan(&plan, &cfg.root);

    if cfg.dry_run {
        println!("{}", style("Simulación (dry-run): no se aplicó nada.").yellow());
        return Ok(());
    }
    if !cfg.yes && io::stdin().is_terminal() && !confirm(plan.entries.len())? {
        println!("{}", style("Cancelado. No se renombró nada.").yellow());
        return Ok(());
    }

    let log = UndoLog {
        fecha: paths::timestamp(),
        revertido_en: None,
        carpeta_escaneada: cfg.root.clone(),
        renames: plan.entries.iter().map(LogEntry::from).collect(),
    };
    let log_path = paths::log_path();
    undo::write_log(&log_path, &log).map_err(io::Error::other)?;

    let report = rename::execute(&plan);
    println!(
        "{} {}",
        style("Historial:").dim(),
        style(log_path.display()).dim()
    );
    for e in &report.errors {
        eprintln!("{}", style(format!("  - {e}")).red());
    }
    if report.renamed > 0 {
        println!("{}", style(format!("Renombrados {} archivos.", report.renamed)).green());
    } else {
        println!("{}", style("No se pudo renombrar nada.").red());
    }
    Ok(())
}

fn confirm(n: usize) -> io::Result<bool> {
    Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(format!("¿Renombrar {n} archivos?"))
        .default(true)
        .interact()
        .map_err(|e| io::Error::other(e.to_string()))
}

fn print_plan(plan: &RenamePlan, root: &Path) {
    let mut t = Table::new(&["Actual", "Nuevo"], &[Align::Left, Align::Left]);
    for e in &plan.entries {
        t.row(&[display(root, &e.origen), display(root, &e.destino)]);
    }
    println!();
    t.print();
    println!();
}

fn display(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}
