use clap::Parser;
use renamr::cli::{Args, Command};
use renamr::{clean, paths, rules, ui};
use renamr::rules::CaseMode;

fn main() {
    let args = Args::parse();

    match args.command {
        Some(Command::Undo { log }) => return run_undo(log),
        Some(Command::Clean) => return run_clean(),
        None => {}
    }

    let root = args.path.clone().unwrap_or_else(paths::default_input);
    if !root.is_dir() {
        if args.path.is_none() {
            if let Err(e) = std::fs::create_dir_all(&root) {
                eprintln!("No se pudo crear {}: {e}", root.display());
                std::process::exit(1);
            }
        } else {
            eprintln!("No es una carpeta válida: {}", root.display());
            std::process::exit(1);
        }
    }

    let replaces = match args
        .replace
        .iter()
        .map(|s| rules::parse_replace(s))
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    };

    let rs = rules::RuleSet {
        replaces,
        case: if args.lower {
            CaseMode::Lower
        } else if args.upper {
            CaseMode::Upper
        } else {
            CaseMode::None
        },
        trim: args.trim,
        by_date: args.by_date,
        number: args.number,
        ext: args.ext.clone(),
    };
    if rs.is_empty() {
        eprintln!(
            "Sin reglas. Usa al menos uno de: --replace, --lower, --upper, --trim, --number, --ext, --by-date"
        );
        std::process::exit(1);
    }

    let cfg = ui::RunConfig {
        root,
        rules: rs,
        dry_run: args.dry_run,
        yes: args.yes,
    };
    if let Err(e) = ui::run(&cfg) {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

fn run_undo(log: Option<std::path::PathBuf>) {
    let result = match log {
        Some(path) => renamr::undo::undo(&path),
        None => renamr::undo::undo_latest(),
    };
    match result {
        Ok(r) => {
            println!("Revertidos {} archivos.", r.restored);
            for e in r.errors {
                eprintln!("  - {e}");
            }
        }
        Err(e) => {
            eprintln!("Error al deshacer: {e}");
            std::process::exit(1);
        }
    }
}

fn run_clean() {
    let report = clean::clean();
    if report.logs == 0 && report.errors.is_empty() {
        println!("Nada que limpiar.");
        return;
    }
    println!("Historial limpio ({} logs).", report.logs);
    for e in &report.errors {
        eprintln!("  - {e}");
    }
}
