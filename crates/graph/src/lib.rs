//! Builds the module graph from file summaries: resolves import specifiers and follows re-exports
//! to the module that defines each export.
//!
//! Reads `package.json` and `tsconfig.json` through the resolver; no other I/O.
//! [`ModuleGraph`] itself is pure: the caller resolves specifiers and parses the files they
//! point to, using [`export_specifiers`] and [`element_specifiers`] to know which are needed.

mod resolver;

use std::collections::{HashMap, HashSet};

use model::{ExportRef, FileId};
use parse::{AliasRoot, Binding, Export, Exported, FileSummary, Forwarded, Imported};

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
    #[error("module has no import #{import}")]
    MissingImport { file: FileId, import: usize },
    #[error("module wasn't parsed")]
    Unparsed { module: FileId },
    #[error("module has no export '{name}'")]
    MissingExport { module: FileId, name: String },
    #[error("module has no ES exports, so '{name}' can't be found; CommonJS exports aren't read")]
    NoEsExports { module: FileId, name: String },
    #[error("'{name}' is exported by more than one `export *`")]
    Ambiguous { module: FileId, name: String },
    #[error("re-exports of '{name}' form a cycle")]
    Cycle { module: FileId, name: String },
}

/// Files indexed by [`FileId`]. `None` marks a file that couldn't be parsed.
#[derive(Debug)]
pub struct ModuleGraph {
    modules: Vec<Option<Module>>,
}

impl ModuleGraph {
    #[must_use]
    pub fn build(modules: Vec<Option<Module>>) -> Self {
        Self { modules }
    }

    /// Drops [`FileSummary::references`], once they've been read, to free their memory.
    pub fn clear_references(&mut self) {
        for module in self.modules.iter_mut().flatten() {
            module.summary.references = Vec::new();
        }
    }

    /// Every parsed module, by [`FileId`].
    pub fn modules(&self) -> impl Iterator<Item = (FileId, &Module)> {
        self.modules
            .iter()
            .enumerate()
            .filter_map(|(index, module)| {
                let id = FileId(u32::try_from(index).ok()?);
                Some((id, module.as_ref()?))
            })
    }

    #[must_use]
    pub fn module(&self, file: FileId) -> Option<&Module> {
        self.modules.get(file.0 as usize)?.as_ref()
    }

    /// Origin of the import at `import` in `file`'s [`FileSummary::imports`].
    pub fn origin(&self, file: FileId, import: usize) -> Result<Origin, OriginError> {
        self.import_origin(file, import, &mut HashSet::new())
    }

    /// The module the import at `import` in `file`'s [`FileSummary::imports`] resolves to, and
    /// what it imports from it.
    pub fn import_target(
        &self,
        file: FileId,
        import: usize,
    ) -> Result<(FileId, &Imported), OriginError> {
        let module = self.parsed(file)?;
        let import = module
            .summary
            .imports
            .get(import)
            .ok_or(OriginError::MissingImport { file, import })?;
        Ok((self.target(file, module, &import.source)?, &import.imported))
    }

    /// The module `specifier` resolves to from `file`.
    pub fn specifier_target(&self, file: FileId, specifier: &str) -> Result<FileId, OriginError> {
        self.target(file, self.parsed(file)?, specifier)
    }

    /// Names `module` exports, including through `export *`, other than `default`, sorted.
    /// Modules that can't be resolved or parsed contribute none.
    #[must_use]
    pub fn export_names(&self, module: FileId) -> Vec<String> {
        let mut names = HashSet::new();
        let mut visited = HashSet::new();
        self.collect_names(module, &mut names, &mut visited);
        let mut names: Vec<String> = names.into_iter().collect();
        names.sort();
        names
    }

    fn collect_names(
        &self,
        id: FileId,
        names: &mut HashSet<String>,
        visited: &mut HashSet<FileId>,
    ) {
        if !visited.insert(id) {
            return;
        }
        let Some(module) = self.module(id) else {
            return;
        };
        for export in &module.summary.exports {
            match export {
                Export::Local { exported, .. }
                | Export::Reexport { exported, .. }
                | Export::Namespace { exported, .. } => {
                    if exported != "default" {
                        names.insert(exported.clone());
                    }
                }
                Export::Star { source } => {
                    if let Ok(target) = self.target(id, module, source) {
                        self.collect_names(target, names, visited);
                    }
                }
            }
        }
    }

    /// The component `module` declares and exports as `name`, as an index into its
    /// [`FileSummary::components`].
    #[must_use]
    pub fn component(&self, module: FileId, name: &str) -> Option<usize> {
        let summary = &self.module(module)?.summary;
        match summary.exported.get(name)? {
            Exported::Variable(variable) => summary
                .components
                .iter()
                .position(|component| component.variables.contains(variable)),
            Exported::Value(_) => (name == "default")
                .then(|| {
                    summary
                        .components
                        .iter()
                        .position(|component| component.default)
                })
                .flatten(),
        }
    }

    /// Origin of the export `name` of `module`.
    pub fn export_origin(&self, module: FileId, name: &str) -> Result<Origin, OriginError> {
        self.find_export(module, name, &mut HashSet::new())?
            .ok_or_else(|| self.missing(module, name))
    }

    fn import_origin(
        &self,
        file: FileId,
        import: usize,
        visiting: &mut HashSet<(FileId, String)>,
    ) -> Result<Origin, OriginError> {
        let (target, imported) = self.import_target(file, import)?;
        match imported {
            Imported::Namespace => Ok(Origin::Namespace(target)),
            Imported::Name(name) => self
                .find_export(target, name, visiting)?
                .ok_or_else(|| self.missing(target, name)),
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
                        .ok_or_else(|| self.missing(target, imported))
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

    /// A module without ES exports is likely CommonJS, whose exports aren't read.
    fn missing(&self, module: FileId, name: &str) -> OriginError {
        let name = name.to_owned();
        match self.module(module) {
            Some(parsed) if parsed.summary.exports.is_empty() => {
                OriginError::NoEsExports { module, name }
            }
            _ => OriginError::MissingExport { module, name },
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

/// Specifiers of the imports that recorded values read, for following imports: in exports, and
/// with `attributes`, in JSX attributes. See [`FileSummary::value_imports`].
pub fn value_specifiers(summary: &FileSummary, attributes: bool) -> impl Iterator<Item = &str> {
    summary
        .value_imports(attributes)
        .into_iter()
        .filter_map(|index| Some(summary.imports.get(index)?.source.as_str()))
}

/// Specifiers of the imports and `require` calls that JSX elements in this file refer to, of the
/// imports components pass their props on to, and of the imports read as values.
pub fn element_specifiers(summary: &FileSummary) -> impl Iterator<Item = &str> {
    let import = |index: usize| {
        summary
            .imports
            .get(index)
            .map(|import| import.source.as_str())
    };
    let elements = summary
        .elements
        .iter()
        .filter_map(move |element| match element.binding {
            Binding::Import(index) => import(index),
            Binding::Alias(index) => match &summary.aliases.get(index)?.root {
                AliasRoot::Import(index) => import(*index),
                AliasRoot::Require(specifier) => Some(specifier.as_str()),
            },
            Binding::Local | Binding::Global => None,
        });
    let forwards = summary
        .components
        .iter()
        .flat_map(|component| &component.forwards)
        .filter_map(move |forward| match forward.to {
            Forwarded::Import { import: index, .. } => import(index),
            Forwarded::Element(_) | Forwarded::Component(_) => None,
        });
    let references = summary
        .references
        .iter()
        .filter_map(move |reference| import(reference.import));
    elements.chain(forwards).chain(references)
}

#[cfg(test)]
mod tests;
