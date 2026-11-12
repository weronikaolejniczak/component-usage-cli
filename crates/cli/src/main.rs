use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    version,
    about = "Scan codebases for component usage and resolve prop values"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Scan a directory for component usages
    Scan {
        /// Root directory to scan
        dir: PathBuf,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Command::Scan { dir } => {
            eprintln!("scan is not implemented yet: {}", dir.display());
            ExitCode::FAILURE
        }
    }
}
