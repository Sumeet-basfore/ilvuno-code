//! `ilvuno` — headless runner CLI. Commands are added milestone by milestone.

use anyhow::Result;
use clap::{Parser, Subcommand};
use runner::{Home, Store};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "ilvuno", version, about = "Headless multi-agent round runner")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create or migrate the database under ILVUNO_HOME.
    InitDb,
}

fn home() -> Home {
    let root = std::env::var_os("ILVUNO_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/share/ilvuno"));
    Home::new(root)
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::InitDb => {
            let home = home();
            Store::open(&home.db_path())?;
            println!("database ready: {}", home.db_path().display());
        }
    }
    Ok(())
}
