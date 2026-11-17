//! Runs a scan: discovers files, parses them in parallel, builds the module graph, identifies usages.
//!
//! The only library crate that walks the filesystem and the only one that uses threads.
//! Files under the root are discovered respecting `.gitignore`, skipping `.git` and
//! `node_modules`. Hidden files are included, since folders such as `.storybook` hold
//! components. Files outside that set, such as library entry points in `node_modules`, are
//! parsed only when a re-export chain or a JSX import reaches them.

mod codeowners;
mod files;

use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use graph::{Module, ModuleGraph, OriginError, ResolveError, Resolver};
use identify::{Cause, Targets};
use model::FileId;
use parse::{FileSummary, ParseError};
use rayon::prelude::*;
use report::{ErrorCode, FileError, Report, UnidentifiedCode};

use crate::files::Files;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub root: PathBuf,
    /// Exported names, e.g. `Icon` or `Icon.Item`. `None` targets every export of `from`.
    pub components: Option<Vec<String>>,
    /// Module specifier the components are exported from, resolved from `root`.
    pub from: String,
    /// Props to report. `None` reports all of them; spreads are always reported.
    pub props: Option<Vec<String>>,
    /// Attach code owners to results.
    pub owners: Option<Owners>,
    /// Follow names declared in the file to their values.
    pub follow_variables: bool,
    /// Follow imported names to the exported value in another file. Implies `follow_variables`.
    pub follow_imports: bool,
    /// Follow the props of components to what their JSX callers pass. Implies `follow_imports`.
    pub follow_props: bool,
}

/// Which `CODEOWNERS` file to use. Patterns always match paths relative to the repository root:
/// `root` or its closest ancestor containing `.git`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Owners {
    /// The first of `.github/CODEOWNERS`, `CODEOWNERS`, `docs/CODEOWNERS`, as GitHub does.
    Discover,
    File(PathBuf),
}

#[derive(Debug, thiserror::Error)]
pub enum ScanError {
    #[error("can't open {}: {source}", path.display())]
    Root { path: PathBuf, source: io::Error },
    #[error("{} is not a directory", path.display())]
    RootNotDirectory { path: PathBuf },
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
    if !root.is_dir() {
        return Err(ScanError::RootNotDirectory {
            path: config.root.clone(),
        });
    }
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

    let options = parse::Options {
        variables: config.follow_variables || config.follow_imports || config.follow_props,
        imports: config.follow_imports || config.follow_props,
        props: config.follow_props,
    };
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
                let summary = read_and_parse(path, options);
                let resolved = summary.as_ref().ok().map(|summary| {
                    needed_specifiers(summary, scanned_set.contains(&id), options.imports)
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
                            code: ErrorCode::Syntax,
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
    let targets = match &config.components {
        Some(components) => {
            Targets::resolve(&graph, from, components).map_err(|error| ScanError::Component {
                component: error.component,
                reason: describe(&error.error, &files, &root),
            })?
        }
        None => Targets::all(&graph, from),
    };
    let identified = identify::identify(&graph, &scanned, &targets);
    let paths = files.displays(&root);
    let context = values::Context::new(&graph, &paths, options.imports, options.props);

    report.usages = identified
        .usages
        .into_iter()
        .map(|usage| report::Usage {
            file: files.display(usage.file, &root),
            location: usage.location,
            component: usage.component,
            parents: usage.parents,
            props: graph
                .module(usage.file)
                .and_then(|module| module.summary.elements.get(usage.element))
                .map(|element| {
                    values::props(&context, usage.file, element, config.props.as_deref())
                })
                .unwrap_or_default(),
            owners: None,
        })
        .collect();
    report.unidentified = identified
        .unidentified
        .into_iter()
        .map(|unidentified| {
            let (code, component, reason) = match unidentified.cause {
                Cause::Origin(error) => (code(&error), None, describe(&error, &files, &root)),
                Cause::Alias { component } => (
                    UnidentifiedCode::Alias,
                    Some(component),
                    "refers to the component through a local constant, which isn't followed"
                        .to_owned(),
                ),
                Cause::Require { component } => (
                    UnidentifiedCode::CommonJs,
                    Some(component),
                    "refers to the component through `require`; only ES modules are supported"
                        .to_owned(),
                ),
            };
            report::Unidentified {
                file: files.display(unidentified.file, &root),
                location: unidentified.location,
                tag: unidentified.tag,
                source: unidentified.source,
                code,
                component,
                reason,
                owners: None,
            }
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

fn read_and_parse(path: &Path, options: parse::Options) -> Result<FileSummary, ReadError> {
    let source = fs::read_to_string(path).map_err(ReadError::Io)?;
    parse::parse_with(path, &source, options).map_err(ReadError::Parse)
}

fn file_error(file: String, error: ReadError) -> FileError {
    let (location, code, message) = match error {
        ReadError::Io(error) => (None, ErrorCode::Read, error.to_string()),
        ReadError::Parse(ParseError::Syntax(error)) => {
            (Some(error.location), ErrorCode::Syntax, error.message)
        }
        ReadError::Parse(error @ ParseError::UnsupportedExtension) => {
            (None, ErrorCode::UnsupportedExtension, error.to_string())
        }
    };
    FileError {
        file,
        location,
        code,
        message,
        skipped: true,
        owners: None,
    }
}

fn needed_specifiers(summary: &FileSummary, scanned: bool, imports: bool) -> Vec<&str> {
    let mut specifiers: Vec<&str> = graph::export_specifiers(summary).collect();
    if scanned {
        specifiers.extend(graph::element_specifiers(summary));
    }
    if imports {
        specifiers.extend(graph::value_specifiers(summary, scanned));
    }
    specifiers.sort_unstable();
    specifiers.dedup();
    specifiers
}

fn code(error: &OriginError) -> UnidentifiedCode {
    match error {
        OriginError::Unresolved { .. } => UnidentifiedCode::Unresolved,
        OriginError::Unparsed { .. } => UnidentifiedCode::Unparsed,
        OriginError::MissingExport { .. } => UnidentifiedCode::MissingExport,
        OriginError::NoEsExports { .. } => UnidentifiedCode::CommonJsModule,
        OriginError::Ambiguous { .. } => UnidentifiedCode::Ambiguous,
        OriginError::Cycle { .. } => UnidentifiedCode::Cycle,
        OriginError::MissingImport { .. } => UnidentifiedCode::Internal,
    }
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
        OriginError::MissingImport { file, import } => {
            format!("{} has no import #{import}", files.display(*file, root))
        }
        OriginError::Unparsed { module } => {
            format!("{} couldn't be parsed", files.display(*module, root))
        }
        OriginError::NoEsExports { module, name } => format!(
            "{} has no ES exports, so '{name}' can't be found; CommonJS exports aren't read",
            files.display(*module, root)
        ),
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
