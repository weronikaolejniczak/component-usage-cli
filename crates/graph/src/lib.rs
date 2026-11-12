//! Builds the module graph from file summaries: resolves import specifiers and follows re-exports
//! to the module that defines each export.
//!
//! Reads `package.json` and `tsconfig.json` through the resolver; no other I/O.
//! [`ModuleGraph`] itself is pure: the caller resolves specifiers and parses the files they
//! point to, using [`export_specifiers`] and [`element_specifiers`] to know which are needed.

mod resolver;

use std::collections::{HashMap, HashSet};

use model::{ExportRef, FileId};
use parse::{Binding, Export, FileSummary, Imported};

pub use resolver::{ResolveError, Resolver};

/// One file in the graph.
#[derive(Debug, Clone)]
pub struct Module {
    pub summary: FileSummary,
    /// Specifiers this file uses, resolved to files in the graph.
    pub resolved: HashMap<String, Result<FileId, ResolveError>>,
}

/// What an import or export ultimately refers to.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Origin {
    /// A binding defined in `module`.
    Export(ExportRef),
    /// All exports of `module`: `import * as ns` or `export * as ns`.
    Namespace(FileId),
}

/// Why an origin couldn't be determined.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OriginError {
    #[error("can't resolve '{specifier}': {reason}")]
    Unresolved {
        file: FileId,
        specifier: String,
        reason: ResolveError,
    },
    #[error("module wasn't parsed")]
    Unparsed { module: FileId },
    #[error("module has no export '{name}'")]
    MissingExport { module: FileId, name: String },
    #[error("'{name}' is exported by more than one `export *`")]
    Ambiguous { module: FileId, name: String },
    #[error("re-exports of '{name}' form a cycle")]
    Cycle { module: FileId, name: String },
}

/// Files indexed by [`FileId`]. `None` marks a file that couldn't be parsed.
pub struct ModuleGraph {
    modules: Vec<Option<Module>>,
}

impl ModuleGraph {
    #[must_use]
    pub fn build(modules: Vec<Option<Module>>) -> Self {
        Self { modules }
    }

    #[must_use]
    pub fn module(&self, file: FileId) -> Option<&Module> {
        self.modules.get(file.0 as usize)?.as_ref()
    }

    /// Origin of the import at `import` in `file`'s [`FileSummary::imports`].
    pub fn origin(&self, file: FileId, import: usize) -> Result<Origin, OriginError> {
        self.import_origin(file, import, &mut HashSet::new())
    }

    /// Origin of the export `name` of `module`.
    pub fn export_origin(&self, module: FileId, name: &str) -> Result<Origin, OriginError> {
        self.find_export(module, name, &mut HashSet::new())?
            .ok_or_else(|| OriginError::MissingExport {
                module,
                name: name.to_owned(),
            })
    }

    fn import_origin(
        &self,
        file: FileId,
        import: usize,
        visiting: &mut HashSet<(FileId, String)>,
    ) -> Result<Origin, OriginError> {
        let module = self.parsed(file)?;
        let import = &module.summary.imports[import];
        let target = self.target(file, module, &import.source)?;
        match &import.imported {
            Imported::Namespace => Ok(Origin::Namespace(target)),
            Imported::Name(name) => self.find_export(target, name, visiting)?.ok_or_else(|| {
                OriginError::MissingExport {
                    module: target,
                    name: name.clone(),
                }
            }),
        }
    }

    /// `Ok(None)` when `module` doesn't export `name`, so `export *` lookups can tell a missing
    /// name apart from a failure. As in ECMAScript's ResolveExport, an `export *` branch that
    /// cycles back is skipped; the cycle is only an error if no other branch has the name.
    fn find_export(
        &self,
        id: FileId,
        name: &str,
        visiting: &mut HashSet<(FileId, String)>,
    ) -> Result<Option<Origin>, OriginError> {
        if !visiting.insert((id, name.to_owned())) {
            return Err(OriginError::Cycle {
                module: id,
                name: name.to_owned(),
            });
        }
        let result = self.lookup(id, name, visiting);
        visiting.remove(&(id, name.to_owned()));
        result
    }

    fn lookup(
        &self,
        id: FileId,
        name: &str,
        visiting: &mut HashSet<(FileId, String)>,
    ) -> Result<Option<Origin>, OriginError> {
        let module = self.parsed(id)?;
        let summary = &module.summary;

        for export in &summary.exports {
            match export {
                Export::Local { exported, local } if exported == name => {
                    let import = local.as_ref().and_then(|local| {
                        summary
                            .imports
                            .iter()
                            .position(|import| &import.local == local)
                    });
                    return match import {
                        Some(import) => self.import_origin(id, import, visiting).map(Some),
                        None => Ok(Some(Origin::Export(ExportRef {
                            module: id,
                            name: name.to_owned(),
                        }))),
                    };
                }
                Export::Reexport {
                    exported,
                    source,
                    imported,
                } if exported == name => {
                    let target = self.target(id, module, source)?;
                    return self
                        .find_export(target, imported, visiting)?
                        .ok_or_else(|| OriginError::MissingExport {
                            module: target,
                            name: imported.clone(),
                        })
                        .map(Some);
                }
                Export::Namespace { exported, source } if exported == name => {
                    return self
                        .target(id, module, source)
                        .map(|target| Some(Origin::Namespace(target)));
                }
                _ => {}
            }
        }

        if name == "default" {
            return Ok(None);
        }
        let mut found: Option<Origin> = None;
        let mut cycle = None;
        for export in &summary.exports {
            let Export::Star { source } = export else {
                continue;
            };
            let target = self.target(id, module, source)?;
            let origin = match self.find_export(target, name, visiting) {
                Err(error @ OriginError::Cycle { .. }) => {
                    cycle.get_or_insert(error);
                    None
                }
                result => result?,
            };
            if let Some(origin) = origin {
                match &found {
                    Some(existing) if *existing != origin => {
                        return Err(OriginError::Ambiguous {
                            module: id,
                            name: name.to_owned(),
                        });
                    }
                    _ => found = Some(origin),
                }
            }
        }
        match (found, cycle) {
            (None, Some(cycle)) => Err(cycle),
            (found, _) => Ok(found),
        }
    }

    fn parsed(&self, id: FileId) -> Result<&Module, OriginError> {
        self.module(id).ok_or(OriginError::Unparsed { module: id })
    }

    fn target(
        &self,
        file: FileId,
        module: &Module,
        specifier: &str,
    ) -> Result<FileId, OriginError> {
        match module.resolved.get(specifier) {
            Some(Ok(target)) => Ok(*target),
            Some(Err(reason)) => Err(OriginError::Unresolved {
                file,
                specifier: specifier.to_owned(),
                reason: reason.clone(),
            }),
            None => Err(OriginError::Unresolved {
                file,
                specifier: specifier.to_owned(),
                reason: ResolveError::not_requested(),
            }),
        }
    }
}

/// Specifiers needed to follow this file's exports to their origins.
pub fn export_specifiers(summary: &FileSummary) -> impl Iterator<Item = &str> {
    let reexports = summary.exports.iter().filter_map(|export| match export {
        Export::Reexport { source, .. }
        | Export::Namespace { source, .. }
        | Export::Star { source } => Some(source.as_str()),
        Export::Local { .. } => None,
    });
    let exported_imports = summary.imports.iter().filter_map(|import| {
        summary
            .exports
            .iter()
            .any(|export| matches!(export, Export::Local { local: Some(local), .. } if *local == import.local))
            .then_some(import.source.as_str())
    });
    reexports.chain(exported_imports)
}

/// Specifiers of the imports that JSX elements in this file refer to.
pub fn element_specifiers(summary: &FileSummary) -> impl Iterator<Item = &str> {
    summary
        .elements
        .iter()
        .filter_map(|element| match element.binding {
            Binding::Import(import) => Some(summary.imports[import].source.as_str()),
            Binding::Local | Binding::Global => None,
        })
}

#[cfg(test)]
mod tests;
