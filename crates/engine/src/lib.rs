//! Runs a scan: discovers files, parses them in parallel, builds the module graph, identifies usages.
//!
//! The only library crate that walks the filesystem and the only one that uses threads.
//! Files under the root are discovered like ripgrep does (respecting `.gitignore`, skipping
//! hidden files and `node_modules`). Files outside that set, such as library entry points in
//! `node_modules`, are parsed only when a re-export chain or a JSX import reaches them.

mod codeowners;
mod files;

use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use graph::{Module, ModuleGraph, OriginError, ResolveError, Resolver};
use identify::Targets;
use model::FileId;
use parse::{FileSummary, ParseError};
use rayon::prelude::*;
use report::{FileError, Report};

use crate::files::Files;

pub struct Config {
    pub root: PathBuf,
    /// Exported names, e.g. `Icon` or `Icon.Item`.
    pub components: Vec<String>,
    /// Module specifier the components are exported from, resolved from `root`.
    pub from: String,
    /// Attach code owners to results.
    pub owners: Option<Owners>,
}

/// Which `CODEOWNERS` file to use. Patterns always match paths relative to the repository root:
/// `root` or its closest ancestor containing `.git`.
pub enum Owners {
    /// The first of `.github/CODEOWNERS`, `CODEOWNERS`, `docs/CODEOWNERS`, as GitHub does.
    Discover,
    File(PathBuf),
}

#[derive(Debug, thiserror::Error)]
pub enum ScanError {
    #[error("can't open {}: {source}", path.display())]
    Root { path: PathBuf, source: io::Error },
    #[error("can't resolve '{specifier}' from {}: {reason}", root.display())]
    From {
        specifier: String,
        root: PathBuf,
        reason: ResolveError,
    },
    #[error("can't find component '{component}': {reason}")]
    Component { component: String, reason: String },
    #[error("no git repository at or above {}, needed to match CODEOWNERS paths", root.display())]
    NoRepository { root: PathBuf },
    #[error("no CODEOWNERS file in {}", repository.display())]
    NoCodeOwners { repository: PathBuf },
    #[error("can't read {}: {source}", path.display())]
    ReadCodeOwners { path: PathBuf, source: io::Error },
    #[error("{} is 3 MB or larger, so GitHub doesn't load it", path.display())]
    CodeOwnersTooLarge { path: PathBuf },
}

/// Stands in for a file at the root when resolving `--from`, so `tsconfig.json` lookup works.
const ROOT_IMPORTER: &str = "index.tsx";

pub fn scan(config: &Config) -> Result<Report, ScanError> {
    let root = config
        .root
        .canonicalize()
        .map_err(|source| ScanError::Root {
            path: config.root.clone(),
            source,
        })?;
    let mut report = Report::default();
    let codeowners = config
        .owners
        .as_ref()
        .map(|source| codeowners::load(&root, source, &mut report))
        .transpose()?;
    let resolver = Resolver::new();
    let mut files = Files::default();
    let mut errors = Vec::new();

    let scanned: Vec<FileId> = files::discover(&root, &mut errors)
        .into_iter()
        .map(|path| files.intern(path).0)
        .collect();
    let scanned_set: HashSet<FileId> = scanned.iter().copied().collect();

    let from = resolver
        .resolve(&root.join(ROOT_IMPORTER), &config.from)
        .map_err(|reason| ScanError::From {
            specifier: config.from.clone(),
            root: root.clone(),
            reason,
        })?;
    let (from, _) = files.intern(from);

    let mut modules: Vec<Option<Module>> = Vec::new();
    let mut pending: Vec<FileId> = scanned.clone();
    if !scanned_set.contains(&from) {
        pending.push(from);
    }
    while !pending.is_empty() {
        let batch: Vec<_> = pending
            .par_iter()
            .map(|&id| {
                let path = files.path(id);
                let summary = read_and_parse(path);
                let resolved = summary.as_ref().ok().map(|summary| {
                    needed_specifiers(summary, scanned_set.contains(&id))
                        .into_iter()
                        .map(|specifier| {
                            let target = resolver.resolve(path, specifier);
                            (specifier.to_owned(), target)
                        })
                        .collect::<Vec<_>>()
                });
                (id, summary, resolved)
            })
            .collect();

        pending.clear();
        for (id, summary, resolved) in batch {
            let module = match summary {
                Ok(summary) => {
                    if scanned_set.contains(&id) {
                        errors.extend(summary.errors.iter().map(|error| FileError {
                            file: files.display(id, &root),
                            location: Some(error.location),
                            message: error.message.clone(),
                            skipped: false,
                            owners: None,
                        }));
                    }
                    let resolved = resolved
                        .unwrap_or_default()
                        .into_iter()
                        .map(|(specifier, target)| {
                            let target = target.map(|path| {
                                let (target, new) = files.intern(path);
                                if new {
                                    pending.push(target);
                                }
                                target
                            });
                            (specifier, target)
                        })
                        .collect();
                    Some(Module { summary, resolved })
                }
                Err(error) => {
                    if scanned_set.contains(&id) {
                        errors.push(file_error(files.display(id, &root), error));
                    }
                    None
                }
            };
            let index = id.0 as usize;
            if modules.len() <= index {
                modules.resize_with(index + 1, || None);
            }
            modules[index] = module;
        }
    }
    modules.resize_with(files.len(), || None);

    let graph = ModuleGraph::build(modules);
    let targets = Targets::resolve(&graph, from, &config.components).map_err(|error| {
        ScanError::Component {
            component: error.component,
            reason: describe(&error.error, &files, &root),
        }
    })?;
    let identified = identify::identify(&graph, &scanned, &targets);

    report.usages = identified
        .usages
        .into_iter()
        .map(|usage| report::Usage {
            file: files.display(usage.file, &root),
            location: usage.location,
            component: usage.component,
            owners: None,
        })
        .collect();
    report.unidentified = identified
        .unidentified
        .into_iter()
        .map(|unidentified| report::Unidentified {
            file: files.display(unidentified.file, &root),
            location: unidentified.location,
            reason: describe(&unidentified.error, &files, &root),
            owners: None,
        })
        .collect();
    report.errors = errors;
    if let Some(codeowners) = &codeowners {
        codeowners.attach(&root, &mut report);
    }
    report.sort();
    Ok(report)
}

enum ReadError {
    Io(io::Error),
    Parse(ParseError),
}

fn read_and_parse(path: &Path) -> Result<FileSummary, ReadError> {
    let source = fs::read_to_string(path).map_err(ReadError::Io)?;
    parse::parse(path, &source).map_err(ReadError::Parse)
}

fn file_error(file: String, error: ReadError) -> FileError {
    let (location, message) = match error {
        ReadError::Io(error) => (None, error.to_string()),
        ReadError::Parse(ParseError::Syntax(error)) => (Some(error.location), error.message),
        ReadError::Parse(error @ ParseError::UnsupportedExtension) => (None, error.to_string()),
    };
    FileError {
        file,
        location,
        message,
        skipped: true,
        owners: None,
    }
}

fn needed_specifiers(summary: &FileSummary, scanned: bool) -> Vec<&str> {
    let mut specifiers: Vec<&str> = graph::export_specifiers(summary).collect();
    if scanned {
        specifiers.extend(graph::element_specifiers(summary));
    }
    specifiers.sort_unstable();
    specifiers.dedup();
    specifiers
}

fn describe(error: &OriginError, files: &Files, root: &Path) -> String {
    match error {
        OriginError::Unresolved {
            file,
            specifier,
            reason,
        } => format!(
            "can't resolve '{specifier}' from {}: {reason}",
            files.display(*file, root)
        ),
        OriginError::Unparsed { module } => {
            format!("{} couldn't be parsed", files.display(*module, root))
        }
        OriginError::MissingExport { module, name } => {
            format!("{} has no export '{name}'", files.display(*module, root))
        }
        OriginError::Ambiguous { module, name } => format!(
            "'{name}' is exported by more than one `export *` in {}",
            files.display(*module, root)
        ),
        OriginError::Cycle { module, name } => format!(
            "re-exports of '{name}' form a cycle through {}",
            files.display(*module, root)
        ),
    }
}
