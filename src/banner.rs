const APP_NAME: &str = "renamr";
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

use std::io::IsTerminal;

const CYAN: &str = "\x1b[1;36m";
const DIM: &str = "\x1b[2;37m";
const RESET: &str = "\x1b[0m";

fn color_enabled() -> bool {
    std::env::var("NO_COLOR").is_err() && std::io::stdout().is_terminal()
}

pub fn print_banner() {
    if !color_enabled() {
        return;
    }
    println!("{CYAN}┌─────────────────────────────┐{RESET}");
    println!(
        "{CYAN}│{RESET} {CYAN}{APP_NAME} v{APP_VERSION}{RESET} {DIM}│{RESET}"
    );
    println!(
        "{CYAN}│{RESET} {DIM}renombrado masivo de archivos{RESET} {DIM}│{RESET}"
    );
    println!("{CYAN}└─────────────────────────────┘{RESET}");
    println!();
}
