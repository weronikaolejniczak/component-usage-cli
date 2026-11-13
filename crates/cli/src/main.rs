use std::io;
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

        /// Add code owners to each result, from the repository's CODEOWNERS file or the given one
        #[arg(long, num_args = 0..=1, value_name = "FILE")]
        owners: Option<Option<PathBuf>>,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Command::Scan {
            dir,
            components,
            from,
            owners,
        } => {
            let config = engine::Config {
                root: dir,
                components,
                from,
                owners: owners
                    .map(|file| file.map_or(engine::Owners::Discover, engine::Owners::File)),
            };
            let report = match engine::scan(&config) {
                Ok(report) => report,
                Err(error) => {
                    eprintln!("error: {error}");
                    return ExitCode::FAILURE;
                }
            };
            if let Err(error) = report::write(&report, report::Format::Json, io::stdout().lock()) {
                eprintln!("error: can't write report: {error}");
                return ExitCode::FAILURE;
            }
            ExitCode::SUCCESS
        }
    }
}
