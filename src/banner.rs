use std::io::IsTerminal;

const APP_NAME: &str = "r-name";
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

const COLOR: &str = "\x1b[1;36m";
const RESET: &str = "\x1b[0m";

fn color_enabled() -> bool {
    std::env::var("NO_COLOR").is_err() && std::io::stdout().is_terminal()
}

pub fn print_banner() {
    let line_len = APP_NAME.chars().count() + APP_VERSION.chars().count() + 6;
    let line = "═".repeat(line_len);

    let color = if color_enabled() { COLOR } else { "" };
    let reset = if color_enabled() { RESET } else { "" };

    println!("{}╭{}╮{}", color, line, reset);
    println!("{}│  {} {}  │{}", color, APP_NAME, APP_VERSION, reset);
    println!("{}╰{}╯{}", color, line, reset);
    println!("{}  renombrador masivo de archivos{}", if color_enabled() { "\x1b[2;37m" } else { "" }, reset);
    println!();
}
