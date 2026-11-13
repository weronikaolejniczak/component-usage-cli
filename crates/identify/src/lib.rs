//! Matches JSX elements to target components by the export their binding resolves to.
//!
//! Pure: takes the module graph, the files to search and targets; returns usages.
//! Targets and JSX tags are both reduced to an origin plus the member accesses left after
//! following namespaces, so `<L.Icon>`, `<Icon>` and `<I>` match the same target.

use std::collections::HashMap;

use graph::{ModuleGraph, Origin, OriginError};
use model::{FileId, Location, Usage};
use parse::Binding;

/// Resolved components to look for.
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
        let mut by_key = HashMap::new();
        for component in components {
            let mut parts = component.split('.').map(str::to_owned);
            let export = parts.next().unwrap_or_default();
            let key = graph
                .export_origin(module, &export)
                .and_then(|origin| key(graph, origin, parts.collect()))
                .map_err(|error| TargetError {
                    component: component.clone(),
                    error,
                })?;
            by_key.entry(key).or_insert_with(|| component.clone());
        }
        Ok(Self { by_key })
    }
}

/// A JSX element whose origin couldn't be determined, so it may or may not be a usage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unidentified {
    pub file: FileId,
    pub location: Location,
    pub error: OriginError,
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
    let mut identified = Identified::default();
    for &file in files {
        let Some(module) = graph.module(file) else {
            continue;
        };
        for element in &module.summary.elements {
            let Binding::Import(import) = element.binding else {
                continue;
            };
            let key = graph
                .origin(file, import)
                .and_then(|origin| key(graph, origin, element.members.clone()));
            match key {
                Ok(key) => {
                    if let Some(component) = targets.by_key.get(&key) {
                        identified.usages.push(Usage {
                            file,
                            location: element.location,
                            component: component.clone(),
                        });
                    }
                }
                Err(error) => identified.unidentified.push(Unidentified {
                    file,
                    location: element.location,
                    error,
                }),
            }
        }
    }
    identified.usages.sort();
    identified
        .unidentified
        .sort_by_key(|unidentified| (unidentified.file, unidentified.location));
    identified
}

fn key(graph: &ModuleGraph, mut origin: Origin, members: Vec<String>) -> Result<Key, OriginError> {
    let mut members = members.into_iter();
    let mut rest = Vec::new();
    for member in members.by_ref() {
        match origin {
            Origin::Namespace(module) => origin = graph.export_origin(module, &member)?,
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

#[cfg(test)]
mod tests;
