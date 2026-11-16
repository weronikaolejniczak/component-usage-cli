use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about = "Scan React codebases for component usages")]
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

        /// Target components, by exported name, e.g. `Icon` or `Icon.Item`; every export of
        /// `--from` if left out
        #[arg(long, value_delimiter = ',', value_parser = component_name)]
        components: Option<Vec<String>>,

        /// Props to report; all if left out. Spreads are always reported
        #[arg(long, value_delimiter = ',', value_parser = prop_name)]
        props: Option<Vec<String>>,

        /// Module the components are exported from, resolved from DIR; matches any import that
        /// resolves to the same export
        #[arg(long)]
        from: String,

        /// Add code owners to each result, from the repository's CODEOWNERS file, or from FILE
        /// with `--owners=FILE`
        #[arg(long, num_args = 0..=1, require_equals = true, value_name = "FILE")]
        owners: Option<Option<PathBuf>>,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Command::Scan {
            dir,
            components,
            props,
            from,
            owners,
        } => {
            let config = engine::Config {
                root: dir,
                components,
                from,
                props,
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
            let mut out = io::BufWriter::new(io::stdout().lock());
            match report::write(&report, report::Format::Json, &mut out).and_then(|()| out.flush())
            {
                Ok(()) => ExitCode::SUCCESS,
                Err(error) if error.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
                Err(error) => {
                    eprintln!("error: can't write report: {error}");
                    ExitCode::FAILURE
                }
            }
        }
    }
}

fn component_name(value: &str) -> Result<String, String> {
    if value.split('.').any(str::is_empty) {
        return Err(format!("'{value}' isn't a component name"));
    }
    Ok(value.to_owned())
}

fn prop_name(value: &str) -> Result<String, String> {
    if value.is_empty() {
        return Err("a prop name can't be empty".to_owned());
    }
    Ok(value.to_owned())
}
