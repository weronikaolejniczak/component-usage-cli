//! Matches JSX elements to target components by the export their binding resolves to.
//!
//! Pure: takes the module graph, the files to search and targets; returns usages.
//! Targets and JSX tags are both reduced to an origin plus the member accesses left after
//! following namespaces, so `<L.Icon>`, `<Icon>` and `<I>` match the same target.

use std::collections::HashMap;

use graph::{ModuleGraph, Origin, OriginError};
use model::{FileId, Location, Usage};
use parse::{AliasRoot, Binding, FileSummary, Imported, JsxElement};

/// Resolved components to look for.
#[derive(Debug)]
pub struct Targets {
    by_key: HashMap<Key, String>,
}

/// `Icon.Item` is `Icon` with members `["Item"]`; a namespace hop removes one member.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Key {
    origin: Origin,
    members: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("can't find component '{component}': {error}")]
pub struct TargetError {
    pub component: String,
    pub error: OriginError,
}

impl Targets {
    /// Resolves each of `components`, e.g. `Icon` or `Icon.Item`, as exported from `module`.
    pub fn resolve(
        graph: &ModuleGraph,
        module: FileId,
        components: &[String],
    ) -> Result<Self, TargetError> {
        let mut origins = Origins::new(graph);
        let mut by_key = HashMap::new();
        for component in components {
            let mut parts = component.split('.').map(str::to_owned);
            let export = parts.next().unwrap_or_default();
            let key = origins
                .export(module, &export)
                .and_then(|origin| origins.key(origin, parts.collect()))
                .map_err(|error| TargetError {
                    component: component.clone(),
                    error,
                })?;
            by_key.entry(key).or_insert_with(|| component.clone());
        }
        Ok(Self { by_key })
    }
}

/// A JSX element that may be a usage but isn't counted as one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unidentified {
    pub file: FileId,
    pub location: Location,
    /// The tag as written: `L.Icon` in `<L.Icon>`.
    pub tag: String,
    /// The specifier of the import or `require` the tag refers to.
    pub source: String,
    pub cause: Cause,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cause {
    /// The origin couldn't be determined.
    Origin(OriginError),
    /// Refers to `component` through a local constant; constants aren't followed yet.
    Alias { component: String },
    /// Refers to `component` through `require`; only ES modules are supported.
    Require { component: String },
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Identified {
    /// Sorted by file, then location.
    pub usages: Vec<Usage>,
    /// Sorted by file, then location.
    pub unidentified: Vec<Unidentified>,
}

/// Finds usages of `targets` among the JSX elements of `files`.
#[must_use]
pub fn identify(graph: &ModuleGraph, files: &[FileId], targets: &Targets) -> Identified {
    let mut origins = Origins::new(graph);
    let mut identified = Identified::default();
    for &file in files {
        let Some(module) = graph.module(file) else {
            continue;
        };
        for element in &module.summary.elements {
            let (origin, members, via) = match element.binding {
                Binding::Import(import) => (origins.import(file, import), Vec::new(), None),
                Binding::Alias(index) => {
                    let Some(alias) = module.summary.aliases.get(index) else {
                        continue;
                    };
                    (
                        origins.alias(file, &alias.root),
                        alias.members.clone(),
                        Some(&alias.root),
                    )
                }
                Binding::Local | Binding::Global => continue,
            };
            let unidentified = |cause| Unidentified {
                file,
                location: element.location,
                tag: tag(element),
                source: source(&module.summary, element).unwrap_or_default(),
                cause,
            };
            let members = members.into_iter().chain(element.members.iter().cloned());
            let key = origin.and_then(|origin| origins.key(origin, members.collect()));
            match key {
                Ok(key) => {
                    let Some(component) = targets.by_key.get(&key).cloned() else {
                        continue;
                    };
                    match via {
                        None => identified.usages.push(Usage {
                            file,
                            location: element.location,
                            component,
                        }),
                        Some(AliasRoot::Import(_)) => identified
                            .unidentified
                            .push(unidentified(Cause::Alias { component })),
                        Some(AliasRoot::Require(_)) => identified
                            .unidentified
                            .push(unidentified(Cause::Require { component })),
                    }
                }
                Err(error) => identified
                    .unidentified
                    .push(unidentified(Cause::Origin(error))),
            }
        }
    }
    identified.usages.sort();
    identified
        .unidentified
        .sort_by_key(|unidentified| (unidentified.file, unidentified.location));
    identified
}

fn tag(element: &JsxElement) -> String {
    std::iter::once(element.root.as_str())
        .chain(element.members.iter().map(String::as_str))
        .collect::<Vec<_>>()
        .join(".")
}

fn source(summary: &FileSummary, element: &JsxElement) -> Option<String> {
    let import = |index: usize| {
        summary
            .imports
            .get(index)
            .map(|import| import.source.clone())
    };
    match element.binding {
        Binding::Import(index) => import(index),
        Binding::Alias(index) => match &summary.aliases.get(index)?.root {
            AliasRoot::Import(index) => import(*index),
            AliasRoot::Require(specifier) => Some(specifier.clone()),
        },
        Binding::Local | Binding::Global => None,
    }
}

/// Origin lookups, memoised by export: many elements across files import the same names.
struct Origins<'g> {
    graph: &'g ModuleGraph,
    exports: HashMap<(FileId, String), Result<Origin, OriginError>>,
}

impl<'g> Origins<'g> {
    fn new(graph: &'g ModuleGraph) -> Self {
        Self {
            graph,
            exports: HashMap::new(),
        }
    }

    fn export(&mut self, module: FileId, name: &str) -> Result<Origin, OriginError> {
        let graph = self.graph;
        self.exports
            .entry((module, name.to_owned()))
            .or_insert_with(|| graph.export_origin(module, name))
            .clone()
    }

    fn import(&mut self, file: FileId, import: usize) -> Result<Origin, OriginError> {
        match self.graph.import_target(file, import)? {
            (target, Imported::Namespace) => Ok(Origin::Namespace(target)),
            (target, Imported::Name(name)) => self.export(target, name),
        }
    }

    /// Origin of an alias's root, before its members. `require` returns the module's namespace.
    fn alias(&mut self, file: FileId, root: &AliasRoot) -> Result<Origin, OriginError> {
        match root {
            AliasRoot::Import(import) => self.import(file, *import),
            AliasRoot::Require(specifier) => self
                .graph
                .specifier_target(file, specifier)
                .map(Origin::Namespace),
        }
    }

    fn key(&mut self, mut origin: Origin, members: Vec<String>) -> Result<Key, OriginError> {
        let mut members = members.into_iter();
        let mut rest = Vec::new();
        for member in members.by_ref() {
            match origin {
                Origin::Namespace(module) => origin = self.export(module, &member)?,
                Origin::Export(_) => {
                    rest.push(member);
                    break;
                }
            }
        }
        rest.extend(members);
        Ok(Key {
            origin,
            members: rest,
        })
    }
}

#[cfg(test)]
mod tests;
