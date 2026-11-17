//! Matches JSX elements to target components by the export their binding resolves to.
//!
//! Pure: takes the module graph, the files to search and targets; returns usages.
//! Targets and JSX tags are both reduced to an origin plus the member accesses left after
//! following namespaces, so `<L.Icon>`, `<Icon>` and `<I>` match the same target.

use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use graph::{ModuleGraph, Origin, OriginError};
use model::{FileId, Location, Parent, Reference, Through, Usage};
use parse::{AliasRoot, Binding, FileSummary, Forwarded, Imported, JsxElement};

/// Resolved components to look for.
#[derive(Debug)]
pub struct Targets {
    by_key: HashMap<Key, String>,
    /// For [`Targets::all`]: exports by origin, which match with any members.
    by_origin: HashMap<Origin, String>,
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
        Ok(Self {
            by_key,
            by_origin: HashMap::new(),
        })
    }

    /// Every export of `module` other than `default`. A tag with members after an export, like
    /// `<Panel.Inner>`, is named with them. Exports whose origin can't be determined are left out.
    #[must_use]
    pub fn all(graph: &ModuleGraph, module: FileId) -> Self {
        let mut by_origin = HashMap::new();
        for name in graph.export_names(module) {
            if let Ok(origin) = graph.export_origin(module, &name) {
                by_origin.entry(origin).or_insert(name);
            }
        }
        Self {
            by_key: HashMap::new(),
            by_origin,
        }
    }

    /// The target a value refers to. With [`Targets::all`], property accesses after an export
    /// aren't one, since they're more often properties than components.
    fn value(&self, key: &Key) -> Option<String> {
        if let Some(component) = self.by_key.get(key) {
            return Some(component.clone());
        }
        if !key.members.is_empty() {
            return None;
        }
        self.by_origin.get(&key.origin).cloned()
    }

    fn component(&self, key: &Key) -> Option<String> {
        if let Some(component) = self.by_key.get(key) {
            return Some(component.clone());
        }
        let export = self.by_origin.get(&key.origin)?;
        Some(
            std::iter::once(export.as_str())
                .chain(key.members.iter().map(String::as_str))
                .collect::<Vec<_>>()
                .join("."),
        )
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
    pub references: Vec<Reference>,
    /// Sorted by file, then location.
    pub unidentified: Vec<Unidentified>,
}

/// Finds usages of `targets` among the JSX elements of `files`. With `wrappers`, also elements
/// of components that pass all their props on to a target, through
/// [`parse::Component::forwards`].
#[must_use]
pub fn identify(
    graph: &ModuleGraph,
    files: &[FileId],
    targets: &Targets,
    wrappers: bool,
) -> Identified {
    let mut state = State {
        origins: Origins::new(graph),
        reached: HashMap::new(),
        active: HashSet::new(),
    };
    let mut identified = Identified::default();
    for &file in files {
        let Some(module) = graph.module(file) else {
            continue;
        };
        let elements = &module.summary.elements;
        let mut components: Vec<Rc<[Reached]>> = vec![Rc::from([]); elements.len()];
        for (index, element) in elements.iter().enumerate() {
            let (origin, members, via) = match element.binding {
                Binding::Import(import) => (state.origins.import(file, import), Vec::new(), None),
                Binding::Alias(index) => {
                    let Some(alias) = module.summary.aliases.get(index) else {
                        continue;
                    };
                    (
                        state.origins.alias(file, &alias.root),
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
            let key = origin.and_then(|origin| state.origins.key(origin, members.collect()));
            match key {
                Ok(key) => {
                    let Some(component) = targets.component(&key) else {
                        continue;
                    };
                    match via {
                        None => {
                            components[index] = Rc::from([Reached {
                                component,
                                through: Vec::new(),
                                excluded: Vec::new(),
                            }]);
                        }
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
        if wrappers {
            for (index, element) in elements.iter().enumerate() {
                if !components[index].is_empty() {
                    continue;
                }
                let wrapper = match element.binding {
                    Binding::Local => element.component.map(|component| (file, component)),
                    Binding::Import(import) => {
                        let origin = state.origins.import(file, import);
                        state
                            .origins
                            .resolve(origin, element.members.clone(), targets)
                            .and_then(Resolved::component)
                    }
                    Binding::Alias(_) | Binding::Global => None,
                };
                if let Some(wrapper) = wrapper {
                    components[index] = state.reaches(targets, wrapper).0;
                }
            }
        }
        let wrapping: HashSet<Location> = module
            .summary
            .components
            .iter()
            .flat_map(|component| &component.forwards)
            .filter(|forward| wrappers && matches!(forward.to, Forwarded::Import { .. }))
            .map(|forward| forward.location)
            .collect();
        for reference in &module.summary.references {
            if wrapping.contains(&reference.location) {
                continue;
            }
            let origin = state.origins.import(file, reference.import);
            let Some(key) = origin
                .and_then(|origin| state.origins.key(origin, reference.members.clone()))
                .ok()
            else {
                continue;
            };
            if let Some(component) = targets.value(&key) {
                identified.references.push(Reference {
                    file,
                    location: reference.location,
                    component,
                    passed: reference.passed.clone(),
                });
            }
        }
        let at: HashMap<Location, usize> = elements
            .iter()
            .enumerate()
            .map(|(index, element)| (element.location, index))
            .collect();
        for (index, reached) in components.iter().enumerate() {
            for reached in reached.iter() {
                identified.usages.push(Usage {
                    file,
                    location: elements[index].location,
                    component: reached.component.clone(),
                    element: index,
                    parents: parents(elements, &components, &at, index),
                    through: reached.through.clone(),
                    excluded: reached.excluded.clone(),
                });
            }
        }
    }
    identified
        .usages
        .sort_by_key(|usage| (usage.file, usage.location));
    identified
        .references
        .sort_by_key(|reference| (reference.file, reference.location));
    identified
        .unidentified
        .sort_by_key(|unidentified| (unidentified.file, unidentified.location));
    identified
}

/// The usages enclosing `elements[index]`, nearest first. Each is paired with the attribute of
/// its own that the chain enters it through.
fn parents(
    elements: &[JsxElement],
    components: &[Rc<[Reached]>],
    at: &HashMap<Location, usize>,
    index: usize,
) -> Vec<Parent> {
    let mut parents = Vec::new();
    let mut link = elements[index].parent.as_ref();
    while let Some(parent) = link {
        let Some(&index) = at.get(&parent.location) else {
            break;
        };
        for reached in components[index].iter() {
            parents.push(Parent {
                component: reached.component.clone(),
                location: parent.location,
                prop: parent.attribute.clone(),
            });
        }
        link = elements[index].parent.as_ref();
    }
    parents
}

/// A target an element renders, directly or through wrappers.
#[derive(Debug)]
struct Reached {
    component: String,
    through: Vec<Through>,
    excluded: Vec<String>,
}

/// A component, by file and index into its [`FileSummary::components`].
type ComponentRef = (FileId, usize);

/// What an import, an element or a call's argument passes props to.
enum Resolved {
    Target(String),
    Component(ComponentRef),
}

impl Resolved {
    fn component(self) -> Option<ComponentRef> {
        match self {
            Self::Component(component) => Some(component),
            Self::Target(_) => None,
        }
    }
}

struct State<'g> {
    origins: Origins<'g>,
    /// The targets each wrapper reaches, for wrappers whose result doesn't depend on a cycle.
    reached: HashMap<ComponentRef, Rc<[Reached]>>,
    /// The wrappers being followed.
    active: HashSet<ComponentRef>,
}

impl State<'_> {
    /// The targets `wrapper` passes its props on to, one per target, by the first way in
    /// source order, and whether a cycle was cut on the way, which leaves the result incomplete
    /// for the wrappers being followed.
    fn reaches(&mut self, targets: &Targets, wrapper: ComponentRef) -> (Rc<[Reached]>, bool) {
        if let Some(reached) = self.reached.get(&wrapper) {
            return (reached.clone(), false);
        }
        if !self.active.insert(wrapper) {
            return (Rc::from([]), true);
        }
        let graph = self.origins.graph;
        let (file, index) = wrapper;
        let mut reached: Vec<Reached> = Vec::new();
        let mut cyclic = false;
        if let Some(module) = graph.module(file)
            && let Some(component) = module.summary.components.get(index)
        {
            for forward in &component.forwards {
                let resolved = match &forward.to {
                    Forwarded::Element(element) => {
                        let Some(element) = module.summary.elements.get(*element) else {
                            continue;
                        };
                        match element.binding {
                            Binding::Local => element
                                .component
                                .map(|component| Resolved::Component((file, component))),
                            Binding::Import(import) => {
                                let origin = self.origins.import(file, import);
                                self.origins
                                    .resolve(origin, element.members.clone(), targets)
                            }
                            Binding::Alias(_) | Binding::Global => None,
                        }
                    }
                    Forwarded::Import { import, members } => {
                        let origin = self.origins.import(file, *import);
                        self.origins.resolve(origin, members.clone(), targets)
                    }
                    Forwarded::Component(component) => {
                        Some(Resolved::Component((file, *component)))
                    }
                };
                let step = Through {
                    component: component.name.clone(),
                    file,
                    location: forward.location,
                };
                let found: Vec<Reached> = match resolved {
                    Some(Resolved::Target(target)) => vec![Reached {
                        component: target,
                        through: vec![step],
                        excluded: forward.excluded.clone(),
                    }],
                    Some(Resolved::Component(next)) => {
                        let (inner, inner_cyclic) = self.reaches(targets, next);
                        cyclic |= inner_cyclic;
                        inner
                            .iter()
                            .map(|inner| Reached {
                                component: inner.component.clone(),
                                through: std::iter::once(step.clone())
                                    .chain(inner.through.iter().cloned())
                                    .collect(),
                                excluded: forward
                                    .excluded
                                    .iter()
                                    .chain(&inner.excluded)
                                    .cloned()
                                    .collect(),
                            })
                            .collect()
                    }
                    None => Vec::new(),
                };
                for found in found {
                    if !reached
                        .iter()
                        .any(|reached| reached.component == found.component)
                    {
                        reached.push(found);
                    }
                }
            }
        }
        self.active.remove(&wrapper);
        let reached: Rc<[Reached]> = reached.into();
        if !cyclic {
            self.reached.insert(wrapper, reached.clone());
        }
        (reached, cyclic)
    }
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

    /// A target, or a component the origin and members lead to.
    fn resolve(
        &mut self,
        origin: Result<Origin, OriginError>,
        members: Vec<String>,
        targets: &Targets,
    ) -> Option<Resolved> {
        let key = self.key(origin.ok()?, members).ok()?;
        if let Some(target) = targets.component(&key) {
            return Some(Resolved::Target(target));
        }
        match (&key.origin, &key.members[..]) {
            (Origin::Export(export), []) => self
                .graph
                .component(export.module, &export.name)
                .map(|component| Resolved::Component((export.module, component))),
            _ => None,
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
