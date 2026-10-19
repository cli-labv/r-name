use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "renamr",
    version,
    about = "renamr · renombra archivos en masa de forma segura"
)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Carpeta a escanear. Por defecto usa <app>/input
    #[arg(value_name = "PATH")]
    pub path: Option<PathBuf>,

    /// Buscar=reemplazo con regex (se puede repetir)
    #[arg(long, value_name = "BUSCAR=REEMPLAZO")]
    pub replace: Vec<String>,

    /// Pone los nombres en minúsculas
    #[arg(long)]
    pub lower: bool,

    /// Pone los nombres en mayúsculas
    #[arg(long)]
    pub upper: bool,

    /// Quita espacios y caracteres raros (deja A-Za-z0-9._-)
    #[arg(long)]
    pub trim: bool,

    /// Numeración secuencial: nombre_0001.ext
    #[arg(long)]
    pub number: bool,

    /// Cambia la extensión
    #[arg(long, value_name = "EXT")]
    pub ext: Option<String>,

    /// Prefijo con fecha EXIF/mtime: AAAA-MM-DD_nombre.ext
    #[arg(long)]
    pub by_date: bool,

    /// Muestra qué se renombraría sin renombrar nada
    #[arg(long)]
    pub dry_run: bool,

    /// Aplica sin preguntar
    #[arg(long, short = 'y')]
    pub yes: bool,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Revierte una ejecución (la última si no se indica un log)
    Undo {
        /// Ruta del log JSON. Si se omite, revierte la última ejecución
        #[arg(value_name = "LOG")]
        log: Option<PathBuf>,
    },
    /// Limpia el historial JSON local (nunca toca tus archivos)
    Clean,
}
