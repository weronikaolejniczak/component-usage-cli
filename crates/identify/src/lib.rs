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
    /// Every name a key is requested as, in order.
    by_key: HashMap<Key, Vec<String>>,
    /// For [`Targets::all`]: exports by origin, sorted, which match with any members.
    by_origin: HashMap<Origin, Vec<String>>,
}

/// `Icon.Item` is `Icon` with members `["Item"]`; a namespace hop removes one member.
#[derive(Debug, Clone)]
struct Key {
    origin: Origin,
    members: Vec<String>,
    /// The export name last written on the way to the origin: `Glyph` in `<Glyph>` or
    /// `<L.Glyph>`. Not part of the key's identity; it picks among names an origin has.
    name: Option<String>,
}

impl PartialEq for Key {
    fn eq(&self, other: &Self) -> bool {
        self.origin == other.origin && self.members == other.members
    }
}

impl Eq for Key {}

impl std::hash::Hash for Key {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.origin.hash(state);
        self.members.hash(state);
    }
}

/// The name in `names` whose export is `name`, or else the first.
fn named<'n>(names: &'n [String], name: Option<&str>) -> Option<&'n String> {
    names
        .iter()
        .find(|candidate| Some(candidate.split('.').next().unwrap_or_default()) == name)
        .or_else(|| names.first())
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
                .and_then(|origin| origins.key(origin, Some(export.clone()), parts.collect()))
                .map_err(|error| TargetError {
                    component: component.clone(),
                    error,
                })?;
            let names: &mut Vec<String> = by_key.entry(key).or_default();
            if !names.contains(component) {
                names.push(component.clone());
            }
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
                by_origin.entry(origin).or_insert_with(Vec::new).push(name);
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
        let name = key.name.as_deref();
        if let Some(names) = self.by_key.get(key) {
            return named(names, name).cloned();
        }
        if !key.members.is_empty() {
            return None;
        }
        named(self.by_origin.get(&key.origin)?, name).cloned()
    }

    fn component(&self, key: &Key) -> Option<String> {
        let name = key.name.as_deref();
        if let Some(names) = self.by_key.get(key) {
            return named(names, name).cloned();
        }
        let export = named(self.by_origin.get(&key.origin)?, name)?;
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
    /// Refers to `component` through a local name that may hold something else.
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
    /// Where local names that usages go through refer to their import: `Icon` in
    /// `const I = Icon`.
    pub aliases: HashSet<(FileId, Location)>,
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
            let (origin, name, members, via) = match element.binding {
                Binding::Import(import) => (
                    state.origins.import(file, import),
                    imported(&module.summary, import),
                    Vec::new(),
                    None,
                ),
                Binding::Alias(index) => {
                    let Some(alias) = module.summary.aliases.get(index) else {
                        continue;
                    };
                    let name = match alias.root {
                        AliasRoot::Import(import) => imported(&module.summary, import),
                        AliasRoot::Require(_) => None,
                    };
                    (
                        state.origins.alias(file, &alias.root),
                        name,
                        alias.members.clone(),
                        Some(alias),
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
            let key = origin.and_then(|origin| state.origins.key(origin, name, members.collect()));
            match key {
                Ok(key) => {
                    let Some(component) = targets.component(&key) else {
                        continue;
                    };
                    match via.map(|alias| (&alias.root, alias.reassigned)) {
                        None | Some((AliasRoot::Import(_), false)) => {
                            components[index] = Rc::from([Reached {
                                component,
                                through: Vec::new(),
                            }]);
                            if let Some(alias) = via {
                                identified.aliases.insert((file, alias.location));
                            }
                        }
                        Some((AliasRoot::Import(_), true)) => identified
                            .unidentified
                            .push(unidentified(Cause::Alias { component })),
                        Some((AliasRoot::Require(_), _)) => identified
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
                let wrapper = state
                    .origins
                    .element(file, &module.summary, element, targets)
                    .and_then(Resolved::component);
                if let Some(wrapper) = wrapper {
                    components[index] = state.reaches(targets, wrapper).0;
                }
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
                });
            }
        }
    }
    identified
        .usages
        .sort_by_key(|usage| (usage.file, usage.location));
    identified
        .unidentified
        .sort_by_key(|unidentified| (unidentified.file, unidentified.location));
    identified
}

/// The places in `files` where `targets` are read as values, sorted by file, then location.
#[must_use]
pub fn references(graph: &ModuleGraph, files: &[FileId], targets: &Targets) -> Vec<Reference> {
    let mut origins = Origins::new(graph);
    let mut references = Vec::new();
    for &file in files {
        let Some(module) = graph.module(file) else {
            continue;
        };
        for reference in &module.summary.references {
            let origin = origins.import(file, reference.import);
            let Some(key) = origin
                .and_then(|origin| {
                    let name = imported(&module.summary, reference.import);
                    origins.key(origin, name, reference.members.clone())
                })
                .ok()
            else {
                continue;
            };
            if let Some(component) = targets.value(&key) {
                references.push(Reference {
                    file,
                    location: reference.location,
                    component,
                    passed: reference.passed.clone(),
                });
            }
        }
    }
    references.sort_by_key(|reference| (reference.file, reference.location));
    references
}

/// `references` without those that are a step of a usage's [`Usage::through`], or where a
/// local name that a usage goes through refers to its import.
#[must_use]
pub fn outside_usages(mut references: Vec<Reference>, identified: &Identified) -> Vec<Reference> {
    let steps: HashSet<(FileId, Location)> = identified
        .usages
        .iter()
        .flat_map(|usage| &usage.through)
        .map(|step| (step.file, step.location))
        .collect();
    references.retain(|reference| {
        let at = (reference.file, reference.location);
        !steps.contains(&at) && !identified.aliases.contains(&at)
    });
    references
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
                    Forwarded::Element { element, .. } => {
                        let Some(element) = module.summary.elements.get(*element) else {
                            continue;
                        };
                        self.origins
                            .element(file, &module.summary, element, targets)
                    }
                    Forwarded::Import { import, members } => {
                        let origin = self.origins.import(file, *import);
                        let name = imported(&module.summary, *import);
                        self.origins.resolve(origin, name, members.clone(), targets)
                    }
                    Forwarded::Component(component) => {
                        Some(Resolved::Component((file, *component)))
                    }
                };
                let step = Through {
                    component: component.name.clone(),
                    file,
                    location: forward.location,
                    wrapper: index,
                    spread: match forward.to {
                        Forwarded::Element { element, attribute } => Some((element, attribute)),
                        Forwarded::Import { .. } | Forwarded::Component(_) => None,
                    },
                    excluded: forward.excluded.clone(),
                };
                let found: Vec<Reached> = match resolved {
                    Some(Resolved::Target(target)) => vec![Reached {
                        component: target,
                        through: vec![step],
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

/// The export name `summary`'s import at `import` binds, if it binds one.
fn imported(summary: &FileSummary, import: usize) -> Option<String> {
    match &summary.imports.get(import)?.imported {
        Imported::Name(name) => Some(name.clone()),
        Imported::Namespace => None,
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

    /// The target or component `element`, an element of `file`, renders.
    fn element(
        &mut self,
        file: FileId,
        summary: &FileSummary,
        element: &JsxElement,
        targets: &Targets,
    ) -> Option<Resolved> {
        match element.binding {
            Binding::Local => element
                .component
                .map(|component| Resolved::Component((file, component))),
            Binding::Import(import) => {
                let origin = self.import(file, import);
                let name = imported(summary, import);
                self.resolve(origin, name, element.members.clone(), targets)
            }
            Binding::Alias(index) => {
                let alias = summary.aliases.get(index)?;
                let import = alias.import()?;
                let origin = self.import(file, import);
                let members = alias.members.iter().chain(&element.members).cloned();
                self.resolve(
                    origin,
                    imported(summary, import),
                    members.collect(),
                    targets,
                )
            }
            Binding::Global => None,
        }
    }

    /// A target, or a component the origin and members lead to.
    fn resolve(
        &mut self,
        origin: Result<Origin, OriginError>,
        name: Option<String>,
        members: Vec<String>,
        targets: &Targets,
    ) -> Option<Resolved> {
        let key = self.key(origin.ok()?, name, members).ok()?;
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

    /// The key `members` after `origin` lead to; `name` is the export name `origin` was
    /// imported as, if any.
    fn key(
        &mut self,
        mut origin: Origin,
        mut name: Option<String>,
        members: Vec<String>,
    ) -> Result<Key, OriginError> {
        let mut members = members.into_iter();
        let mut rest = Vec::new();
        for member in members.by_ref() {
            match origin {
                Origin::Namespace(module) => {
                    origin = self.export(module, &member)?;
                    name = Some(member);
                }
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
            name,
        })
    }
}

#[cfg(test)]
mod tests;
