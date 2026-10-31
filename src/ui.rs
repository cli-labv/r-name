use crate::paths;
use crate::rename::{self, build_season_plan, RenamePlan, season_number};
use crate::rules::{CaseMode, RuleSet, SortOrder};
use crate::table::{Align, Table};
use crate::undo::{self, LogEntry, UndoLog};
use console::style;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use std::collections::HashMap;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// Spinner animado mientras se ejecuta una operación costosa (escaneo o renombrado).
fn spin_during<T>(message: &str, f: impl FnOnce() -> T) -> T {
    let color_enabled = std::env::var("NO_COLOR").is_err() && io::stdout().is_terminal();
    if !color_enabled {
        return f();
    }
    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();
    let msg = message.to_string();
    let spinner = thread::spawn(move || {
        let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let mut i = 0;
        while running_clone.load(Ordering::Relaxed) {
            print!("\r\x1b[K\x1b[1;32m{} {}\x1b[0m", frames[i % frames.len()], msg);
            let _ = io::stdout().flush();
            i = (i + 1) % frames.len();
            thread::sleep(Duration::from_millis(70));
        }
        print!("\r\x1b[K");
        let _ = io::stdout().flush();
    });
    let result = f();
    running.store(false, Ordering::Relaxed);
    let _ = spinner.join();
    result
}

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

/// Agrupa archivos por carpeta inmediata y cuenta cuántos hay por temporada.
fn summarize(files: &[PathBuf], root: &Path) -> HashMap<String, usize> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for p in files {
        let folder = p
            .parent()
            .and_then(|par| {
                if par == root {
                    None
                } else {
                    par.file_name().map(|s| s.to_string_lossy().into_owned())
                }
            })
            .unwrap_or_else(|| String::from("raíz"));
        *counts.entry(folder).or_insert(0) += 1;
    }
    counts
}

pub fn run(cfg: &RunConfig) -> io::Result<()> {
    let files = spin_during("Escaneando archivos", || scan(&cfg.root));
    execute_plan(&cfg.root, &cfg.rules, files, cfg.dry_run, cfg.yes)
}

/// Menú interactivo directo: resumen → modo temporada o clásico → preview → confirmar.
pub fn interactive_menu(root: PathBuf) -> io::Result<()> {
    let theme = ColorfulTheme::default();

    let files = spin_during("Escaneando archivos", || scan(&root));
    if files.is_empty() {
        println!("{}", style("Sin archivos para renombrar en la carpeta actual.").yellow());
        return Ok(());
    }

    let counts = summarize(&files, &root);
    let total: usize = counts.values().sum();

    println!();
    println!("{}", style(format!("📁 Archivos encontrados: {total}")).bold());
    for (folder, n) in &counts {
        println!("   └─ {folder}: {n} archivo{}", if *n > 1 { "s" } else { "" });
    }
    println!();

    let options = [
        "Renombrar por temporada (carpetas t1, t2, …)",
        "Acciones rápidas (clásico: minúsculas, limpiar, numerar…)",
        "Salir",
    ];
    let idx = Select::with_theme(&theme)
        .with_prompt("¿Qué deseas hacer?")
        .items(&options)
        .default(0)
        .interact()
        .map_err(|e| io::Error::other(e.to_string()))?;

    match idx {
        0 => season_flow(&theme, &root, &files),
        1 => classic_flow(&theme, &root, &files),
        _ => {
            println!("{}", style("Saliendo.").yellow());
            Ok(())
        }
    }
}

/// Encuentra el ancestro común más bajo de una lista de paths.
fn common_ancestor(paths: &[&Path]) -> Option<PathBuf> {
    if paths.is_empty() {
        return None;
    }
    let mut common = paths[0].to_path_buf();
    for p in paths.iter().skip(1) {
        let mut a = common.clone();
        let mut b = p.to_path_buf();
        let mut da = a.components().count();
        let mut db = b.components().count();
        while da > db {
            a = a.parent()?.to_path_buf();
            da -= 1;
        }
        while db > da {
            b = b.parent()?.to_path_buf();
            db -= 1;
        }
        while a != b {
            a = a.parent()?.to_path_buf();
            b = b.parent()?.to_path_buf();
        }
        common = a;
    }
    Some(common)
}

/// Detecta la carpeta superior común que agrupa a todas las carpetas de temporada.
/// Ej: si hay root/re zero/t1/ y root/re zero/t2/, devuelve root/re zero.
/// Si todo está directo en root (no hay carpeta superior), devuelve None.
fn detect_common_parent(files: &[PathBuf], root: &Path) -> Option<PathBuf> {
    let season_parents: Vec<&Path> = files
        .iter()
        .filter_map(|p| {
            p.parent().filter(|par| {
                *par != root
                    && par
                        .file_name()
                        .and_then(|s| s.to_str())
                        .is_some_and(|s| season_number(s).is_some())
            })
        })
        .collect();

    if season_parents.is_empty() {
        return None;
    }

    let common = common_ancestor(&season_parents)?;
    if common == root {
        return None;
    }

    // Si el ancestro común es él mismo una carpeta de temporada (tiene número),
    // subir un nivel para obtener la carpeta de la serie.
    let mut candidate = common;
    while candidate != root {
        let name = candidate
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        if season_number(name).is_none() {
            break;
        }
        candidate = match candidate.parent() {
            Some(p) => p.to_path_buf(),
            None => break,
        };
    }

    if candidate == root {
        None
    } else {
        Some(candidate)
    }
}

/// Flujo por temporada: detecta nombre de serie por defecto, pide título y orden de renombrado.
fn season_flow(theme: &ColorfulTheme, root: &Path, files: &[PathBuf]) -> io::Result<()> {
    // 1. Detectar carpeta superior común (nombre de la serie por defecto)
    let default_title = detect_common_parent(files, root)
        .as_ref()
        .and_then(|p| p.file_name().map(|s| s.to_string_lossy().into_owned()))
        .unwrap_or_else(|| String::from("episodio"));

    let title: String = Input::with_theme(theme)
        .with_prompt("Título de la serie")
        .default(default_title)
        .interact()
        .map_err(|e| io::Error::other(e.to_string()))?;

    if title.trim().is_empty() {
        println!("{}", style("El título no puede estar vacío.").red());
        return Ok(());
    }

    // 2. Elegir orden de renombrado
    let order_options = [
        "Nombre (ascendente)",
        "Nombre (descendente)",
        "Fecha creación (reciente primero)",
        "Fecha modificación (ascendente)",
        "Fecha modificación (descendente)",
    ];
    let order_idx = Select::with_theme(theme)
        .with_prompt("Ordenar episodios por")
        .items(&order_options)
        .default(0)
        .interact()
        .map_err(|e| io::Error::other(e.to_string()))?;

    let sort_by = match order_idx {
        1 => SortOrder::NameDesc,
        2 => SortOrder::CreatedDesc,
        3 => SortOrder::ModifiedAsc,
        4 => SortOrder::ModifiedDesc,
        _ => SortOrder::Name,
    };

    // 3. Generar plan: {episodio} {titulo}.{ext} dentro de cada temporada
    let plan = spin_during("Generando plan", || build_season_plan(files, root, &title, sort_by));

    for s in &plan.skipped {
        eprintln!("{}", style(format!("  - {s}")).red());
    }
    if plan.entries.is_empty() {
        println!("{}", style("Sin cambios que aplicar.").yellow());
        return Ok(());
    }

    print_season_preview(&plan, root);

    if !confirm(plan.entries.len())? {
        println!("{}", style("Cancelado. No se renombró nada.").yellow());
        return Ok(());
    }

    let log = UndoLog {
        fecha: paths::timestamp(),
        revertido_en: None,
        carpeta_escaneada: root.to_path_buf(),
        renames: plan.entries.iter().map(LogEntry::from).collect(),
    };
    let log_path = paths::log_path();
    undo::write_log(&log_path, &log).map_err(io::Error::other)?;

    let report = spin_during("Renombrando archivos", || rename::execute(&plan));
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

/// Flujo temporada directo (./start --t): usa .env para título y orden, sin menú interactivo.
pub fn run_season_direct(
    root: PathBuf,
    title: Option<String>,
    sort_by: SortOrder,
    yes: bool,
) -> io::Result<()> {
    let _theme = ColorfulTheme::default();

    let files = spin_during("Escaneando archivos", || scan(&root));
    if files.is_empty() {
        println!("{}", style("Sin archivos para renombrar en la carpeta actual.").yellow());
        return Ok(());
    }

    let counts = summarize(&files, &root);
    let total: usize = counts.values().sum();
    println!();
    println!("{}", style(format!("📁 Archivos encontrados: {total}")).bold());
    for (folder, n) in &counts {
        println!("   └─ {folder}: {n} archivo{}", if *n > 1 { "s" } else { "" });
    }
    println!();

    // Título: del .env o detectado de carpeta superior
    let default_title = detect_common_parent(&files, &root)
        .as_ref()
        .and_then(|p| p.file_name().map(|s| s.to_string_lossy().into_owned()))
        .unwrap_or_else(|| String::from("episodio"));

    let title = title.unwrap_or(default_title);
    if title.trim().is_empty() {
        println!("{}", style("El título no puede estar vacío.").red());
        return Ok(());
    }

    let plan = spin_during("Generando plan", || build_season_plan(&files, &root, &title, sort_by));

    for s in &plan.skipped {
        eprintln!("{}", style(format!("  - {s}")).red());
    }
    if plan.entries.is_empty() {
        println!("{}", style("Sin cambios que aplicar.").yellow());
        return Ok(());
    }

    print_season_preview(&plan, &root);

    if !yes && io::stdin().is_terminal() && !confirm(plan.entries.len())? {
        println!("{}", style("Cancelado. No se renombró nada.").yellow());
        return Ok(());
    }

    let log = UndoLog {
        fecha: paths::timestamp(),
        revertido_en: None,
        carpeta_escaneada: root.clone(),
        renames: plan.entries.iter().map(LogEntry::from).collect(),
    };
    let log_path = paths::log_path();
    undo::write_log(&log_path, &log).map_err(io::Error::other)?;

    let report = spin_during("Renombrando archivos", || rename::execute(&plan));
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

/// Flujo clásico (menú de acciones rápidas original).
fn classic_flow(theme: &ColorfulTheme, root: &Path, files: &[PathBuf]) -> io::Result<()> {
    let action_options = [
        "Minúsculas",
        "Mayúsculas",
        "Limpiar nombres (espacios y caracteres raros)",
        "Numerar secuencialmente (nombre_0001.ext)",
        "Cambiar extensión",
        "Prefijo con fecha (EXIF o modificación)",
    ];
    let sel = Select::with_theme(theme)
        .with_prompt("Elige una acción rápida")
        .items(&action_options)
        .default(0)
        .interact()
        .map_err(|e| io::Error::other(e.to_string()))?;

    let mut case = CaseMode::None;
    let mut trim = false;
    let mut by_date = false;
    let mut number = false;
    let mut ext: Option<String> = None;

    match sel {
        0 => case = CaseMode::Lower,
        1 => case = CaseMode::Upper,
        2 => trim = true,
        3 => number = true,
        4 => {
            ext = Some(
                Input::with_theme(theme)
                    .with_prompt("Nueva extensión (ej. jpg)")
                    .default(String::from("jpg"))
                    .interact()
                    .map_err(|e| io::Error::other(e.to_string()))?,
            );
        }
        5 => by_date = true,
        _ => return Ok(()),
    }

    let rules = RuleSet {
        replaces: Vec::new(),
        case,
        trim,
        by_date,
        number,
        ext,
        sort_by: SortOrder::Name,
    };

    if rules.is_empty() {
        println!("{}", style("Sin acciones seleccionadas.").yellow());
        return Ok(());
    }

    execute_plan(root, &rules, files.to_vec(), false, false)
}

fn execute_plan(
    root: &Path,
    rules: &RuleSet,
    files: Vec<PathBuf>,
    dry_run: bool,
    yes: bool,
) -> io::Result<()> {
    if files.is_empty() {
        println!("{}", style("Sin archivos para renombrar.").yellow());
        return Ok(());
    }

    let plan = rename::build_plan(&files, rules);
    for s in &plan.skipped {
        eprintln!("{}", style(format!("  - {s}")).red());
    }
    if plan.entries.is_empty() {
        println!("{}", style("Sin cambios.").yellow());
        return Ok(());
    }

    print_plan(&plan, root);

    if dry_run {
        println!("{}", style("Simulación (dry-run): no se aplicó nada.").yellow());
        return Ok(());
    }
    if !yes && io::stdin().is_terminal() && !confirm(plan.entries.len())? {
        println!("{}", style("Cancelado. No se renombró nada.").yellow());
        return Ok(());
    }

    let log = UndoLog {
        fecha: paths::timestamp(),
        revertido_en: None,
        carpeta_escaneada: root.to_path_buf(),
        renames: plan.entries.iter().map(LogEntry::from).collect(),
    };
    let log_path = paths::log_path();
    undo::write_log(&log_path, &log).map_err(io::Error::other)?;

    let report = spin_during("Renombrando archivos", || rename::execute(&plan));
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

fn print_season_preview(plan: &RenamePlan, _root: &Path) {
    println!();
    println!("{}", style("Vista previa (Actual → Nuevo):").bold());
    let mut t = Table::new(&["Actual", "Nuevo"], &[Align::Left, Align::Left]);
    for e in &plan.entries {
        t.row(&[
            e.origen
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            e.destino
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
        ]);
    }
    t.print();
    println!();
}

fn display(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

pub fn has_tty_menu() -> bool {
    io::stdin().is_terminal()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn p(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    #[test]
    fn test_common_ancestor_two_paths() {
        let a = p("/home/user/input/re zero/t1");
        let b = p("/home/user/input/re zero/t2");
        let anc = common_ancestor(&[&a, &b]).unwrap();
        assert_eq!(anc, p("/home/user/input/re zero"));
    }

    #[test]
    fn test_common_ancestor_single_path() {
        let a = p("/home/user/input/re zero/t1");
        let anc = common_ancestor(&[&a]).unwrap();
        assert_eq!(anc, p("/home/user/input/re zero/t1"));
    }

    #[test]
    fn test_detect_common_parent_with_superior_folder() {
        let root = p("/home/user/input");
        let files = vec![
            p("/home/user/input/re zero/t1/video.mkv"),
            p("/home/user/input/re zero/t2/intro.mp4"),
        ];
        let result = detect_common_parent(&files, &root).unwrap();
        assert_eq!(result, p("/home/user/input/re zero"));
    }

    #[test]
    fn test_detect_common_parent_single_season_folder() {
        let root = p("/home/user/input");
        let files = vec![p("/home/user/input/re zero/t1/video.mkv")];
        let result = detect_common_parent(&files, &root).unwrap();
        assert_eq!(result, p("/home/user/input/re zero"));
    }

    #[test]
    fn test_detect_common_parent_no_superior_folder() {
        let root = p("/home/user/input");
        let files = vec![
            p("/home/user/input/t1/video.mkv"),
            p("/home/user/input/t2/intro.mp4"),
        ];
        let result = detect_common_parent(&files, &root);
        assert_eq!(result, None);
    }

    #[test]
    fn test_detect_common_parent_mixed_depths() {
        let root = p("/home/user/input");
        let files = vec![
            p("/home/user/input/serie/t1/ep1.mkv"),
            p("/home/user/input/serie/t2/ep1.mkv"),
        ];
        let result = detect_common_parent(&files, &root).unwrap();
        assert_eq!(result, p("/home/user/input/serie"));
    }
}
