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

        /// Target components, by exported name
        #[arg(long, value_delimiter = ',', required = true)]
        components: Vec<String>,

        /// Only count components imported from this module
        #[arg(long)]
        from: String,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Command::Scan {
            dir,
            components,
            from,
        } => {
            eprintln!(
                "scan is not implemented yet: {} ({} from {from})",
                dir.display(),
                components.join(", "),
            );
            ExitCode::FAILURE
        }
    }
}
