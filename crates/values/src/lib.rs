//! Resolves the values a prop can receive, following the names [`parse`] recorded definitions
//! for, with [`Context::new`]'s `imports`, imported names through the module graph, and with its
//! `props`, the props of components to what their JSX callers pass. What is recorded, and so
//! followed, is set by [`parse::Options`].
//!
//! Pure. Every value has a kind and a location; unresolved values carry a code. Values and names
//! in another file than the usage's say which.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};

use graph::{ModuleGraph, Origin, OriginError};
use model::{
    Entry, ExportRef, FileId, Item, Literal, Location, Part, Prop, Step, UnresolvedCode, Value,
    ValueKind, Via,
};
use parse::{Attribute, Definition, Exported, Expr, ExprKind, JsxElement, Key, Reference, Segment};

/// A component: its file and its index in that file's [`parse::FileSummary::components`].
type ComponentRef = (FileId, usize);

/// What every usage of a scan is evaluated against.
pub struct Context<'g> {
    graph: &'g ModuleGraph,
    /// Display paths, by [`FileId`].
    paths: &'g [String],
    imports: bool,
    /// Exports changed in place by a file that imports them.
    changed: HashSet<ExportRef>,
    /// Modules changed in place through a namespace import.
    changed_modules: HashSet<FileId>,
    props: bool,
    /// The elements that render each component, as files and indices into their
    /// [`parse::FileSummary::elements`], in order of path and position.
    callers: HashMap<ComponentRef, Vec<(FileId, usize)>>,
    /// Components used other than as a JSX tag, an export or the argument of `memo` or
    /// `forwardRef`, in their file or through an import.
    escaped: HashSet<ComponentRef>,
}

impl<'g> Context<'g> {
    /// With `imports`, imported names are followed; the graph must then have resolved
    /// [`graph::value_specifiers`]. With `props`, the props of components are followed to their
    /// callers; the graph must also have resolved [`graph::element_specifiers`].
    #[must_use]
    pub fn new(graph: &'g ModuleGraph, paths: &'g [String], imports: bool, props: bool) -> Self {
        let mut changed = HashSet::new();
        let mut changed_modules = HashSet::new();
        if imports {
            for (file, module) in graph.modules() {
                for &import in &module.summary.changed_imports {
                    match graph.origin(file, import) {
                        Ok(Origin::Export(export)) => {
                            changed.insert(export);
                        }
                        Ok(Origin::Namespace(module)) => {
                            changed_modules.insert(module);
                        }
                        Err(_) => {}
                    }
                }
            }
        }
        let mut context = Self {
            graph,
            paths,
            imports: imports || props,
            changed,
            changed_modules,
            props,
            callers: HashMap::new(),
            escaped: HashSet::new(),
        };
        if props {
            context.index_callers();
        }
        context
    }

    fn index_callers(&mut self) {
        let graph = self.graph;
        let mut origins = Origins {
            graph,
            exports: HashMap::new(),
        };
        for (file, module) in graph.modules() {
            let summary = &module.summary;
            for (index, element) in summary.elements.iter().enumerate() {
                if let Some(component) = self.rendered(file, summary, element, &mut origins) {
                    self.callers
                        .entry(component)
                        .or_default()
                        .push((file, index));
                }
            }
            for (index, component) in summary.components.iter().enumerate() {
                if component.escapes {
                    self.escaped.insert((file, index));
                }
            }
            for &import in &summary.escaping_imports {
                match origins.import(file, import) {
                    Some(Origin::Export(export)) => {
                        if let Some(component) = self.component(export.module, &export.name) {
                            self.escaped.insert(component);
                        }
                    }
                    Some(Origin::Namespace(module)) => {
                        let count = graph
                            .module(module)
                            .map_or(0, |module| module.summary.components.len());
                        self.escaped.extend((0..count).map(|index| (module, index)));
                    }
                    None => {}
                }
            }
        }
        let paths = self.paths;
        for callers in self.callers.values_mut() {
            callers.sort_by_cached_key(|&(file, index)| {
                let location = graph
                    .module(file)
                    .map(|module| module.summary.elements[index].location);
                (&paths[file.0 as usize], location)
            });
        }
    }

    /// The component `element`, an element of `file`, renders.
    fn rendered(
        &self,
        file: FileId,
        summary: &parse::FileSummary,
        element: &JsxElement,
        origins: &mut Origins,
    ) -> Option<ComponentRef> {
        let (import, members) = match element.binding {
            parse::Binding::Local => return element.component.map(|index| (file, index)),
            parse::Binding::Import(import) => (import, element.members.clone()),
            parse::Binding::Alias(index) => {
                let alias = summary.aliases.get(index)?;
                let members = alias.members.iter().chain(&element.members).cloned();
                (alias.import()?, members.collect())
            }
            parse::Binding::Global => return None,
        };
        let mut origin = origins.import(file, import)?;
        for member in &members {
            match origin {
                Origin::Namespace(module) => origin = origins.export(module, member)?,
                Origin::Export(_) => return None,
            }
        }
        match origin {
            Origin::Export(export) => self.component(export.module, &export.name),
            Origin::Namespace(_) => None,
        }
    }

    /// The component `module` exports as `name`.
    fn component(&self, module: FileId, name: &str) -> Option<ComponentRef> {
        Some((module, self.graph.component(module, name)?))
    }
}

/// The attributes of `element`, an element of `file`, in source order. With `filter`, only the
/// named ones, and spreads, which may set any prop.
pub fn props(
    context: &Context,
    file: FileId,
    element: &parse::JsxElement,
    filter: Option<&[String]>,
) -> Vec<Prop> {
    let resolver = Resolver::new(context, file, Vec::new());
    resolver
        .attributes(file, &element.attributes)
        .filter(|prop| wanted(prop, filter))
        .collect()
}

/// The props `usage` gives its component, in the order they apply: those of its element, of
/// `element`, and those each wrapper in [`model::Usage::through`] sets around the spread that
/// passes them on, without those its rest pattern takes. A wrapper's own props are followed to
/// the element this usage goes through only. With `filter`, only the named ones, and spreads.
pub fn usage_props(
    context: &Context,
    usage: &model::Usage,
    element: &parse::JsxElement,
    filter: Option<&[String]>,
) -> Vec<Prop> {
    let mut caller = (usage.file, usage.element);
    let mut pins = Vec::with_capacity(usage.through.len());
    for step in &usage.through {
        pins.push(((step.file, step.wrapper), caller));
        if let Some((element, _)) = step.spread {
            caller = (step.file, element);
        }
    }
    let resolver = Resolver::new(context, usage.file, pins);
    let mut props: Vec<Prop> = resolver
        .attributes(usage.file, &element.attributes)
        .filter(|prop| usage.through.is_empty() || !is_key(prop))
        .collect();
    for step in &usage.through {
        props = props
            .into_iter()
            .filter_map(|prop| match prop {
                Prop::Named { name, .. } if step.excluded.contains(&name) => None,
                Prop::Spread { spread } => Some(Prop::Spread {
                    spread: without(spread, &step.excluded),
                }),
                prop => Some(prop),
            })
            .collect();
        let Some((element, attribute)) = step.spread else {
            continue;
        };
        let Some(element) = context
            .graph
            .module(step.file)
            .and_then(|module| module.summary.elements.get(element))
        else {
            continue;
        };
        let (before, after) = element
            .attributes
            .split_at(attribute.min(element.attributes.len()));
        let after = after.get(1..).unwrap_or_default();
        props = resolver
            .attributes(step.file, before)
            .chain(props)
            .chain(resolver.attributes(step.file, after))
            .collect();
    }
    props.retain(|prop| wanted(prop, filter));
    props
}

/// React keeps `key` for itself rather than passing it in the props.
const KEY: &str = "key";

fn is_key(prop: &Prop) -> bool {
    matches!(prop, Prop::Named { name, .. } if name == KEY)
}

fn wanted(prop: &Prop, filter: Option<&[String]>) -> bool {
    match prop {
        Prop::Named { name, .. } => filter.is_none_or(|names| names.contains(name)),
        Prop::Spread { .. } => true,
    }
}

/// A name a component's props declare, in the component's file.
struct Declared<'p> {
    file: FileId,
    parameter: &'p parse::Parameter,
}

/// Origins of exports looked up so far, by module and name, shared by every file.
struct Origins<'g> {
    graph: &'g ModuleGraph,
    exports: HashMap<(FileId, String), Option<Origin>>,
}

impl Origins<'_> {
    fn export(&mut self, module: FileId, name: &str) -> Option<Origin> {
        let graph = self.graph;
        self.exports
            .entry((module, name.to_owned()))
            .or_insert_with(|| graph.export_origin(module, name).ok())
            .clone()
    }

    fn import(&mut self, file: FileId, import: usize) -> Option<Origin> {
        match self.graph.import_target(file, import).ok()? {
            (target, parse::Imported::Namespace) => Some(Origin::Namespace(target)),
            (target, parse::Imported::Name(name)) => self.export(target, name),
        }
    }
}

struct Resolver<'c, 'g> {
    context: &'c Context<'g>,
    usage: FileId,
    /// Components whose props are followed to one element only, the one the usage goes through.
    pins: Vec<(ComponentRef, (FileId, usize))>,
    /// What is being followed, to stop at names that refer back to themselves.
    active: RefCell<Vec<Active>>,
    /// Whether the value being found is, or is within, an argument of a call; only then are the
    /// returns of its functions needed.
    argument: Cell<bool>,
    /// Whether the value being found is within a call's arguments, where names aren't followed.
    in_call: Cell<bool>,
}

#[derive(PartialEq, Eq)]
enum Active {
    Definition(FileId, usize),
    Export(FileId, String),
    Component(FileId, usize),
}

/// A name being followed: what's written, and where.
struct Name<'n> {
    file: FileId,
    text: &'n str,
    location: Location,
}

impl<'c, 'g> Resolver<'c, 'g> {
    fn new(
        context: &'c Context<'g>,
        usage: FileId,
        pins: Vec<(ComponentRef, (FileId, usize))>,
    ) -> Self {
        Self {
            context,
            usage,
            pins,
            active: RefCell::new(Vec::new()),
            argument: Cell::new(false),
            in_call: Cell::new(false),
        }
    }

    /// `attributes`, of an element of `file`, as props.
    fn attributes<'a>(
        &'a self,
        file: FileId,
        attributes: &'a [Attribute],
    ) -> impl Iterator<Item = Prop> + 'a {
        attributes.iter().map(move |attribute| match attribute {
            Attribute::Named { name, value } => Prop::Named {
                name: name.clone(),
                value: returned_by_arguments(self.value(file, value), false),
            },
            Attribute::Spread(spread) => Prop::Spread {
                spread: returned_by_arguments(self.value(file, spread), false),
            },
        })
    }

    fn value(&self, file: FileId, expr: &Expr) -> Value {
        let kind = match &expr.kind {
            ExprKind::Literal(literal) => ValueKind::Literal {
                value: literal_of(literal),
            },
            ExprKind::Undefined => ValueKind::Undefined,
            ExprKind::Object(entries) => ValueKind::Object {
                entries: entries
                    .iter()
                    .map(|entry| match entry {
                        parse::Entry::Property { key, value } => Entry::Property {
                            key: key.clone(),
                            value: self.value(file, value),
                        },
                        parse::Entry::Computed { key, value } => Entry::Computed {
                            computed: key.clone(),
                            value: self.value(file, value),
                        },
                        parse::Entry::Spread(spread) => Entry::Spread {
                            spread: self.value(file, spread),
                        },
                    })
                    .collect(),
            },
            ExprKind::Array(items) => ValueKind::Array {
                items: items
                    .iter()
                    .map(|item| match item {
                        parse::Item::Value(value) => Item::Value {
                            value: self.value(file, value),
                        },
                        parse::Item::Spread(spread) => Item::Spread {
                            spread: self.value(file, spread),
                        },
                    })
                    .collect(),
            },
            ExprKind::Template { tag, parts } => ValueKind::Template {
                tag: tag.clone(),
                parts: parts
                    .iter()
                    .map(|part| match part {
                        parse::Part::Text(text) => Part::Text { text: text.clone() },
                        parse::Part::Value(value) => Part::Value {
                            value: self.value(file, value),
                        },
                    })
                    .collect(),
            },
            ExprKind::Element { tag } => ValueKind::Element { tag: tag.clone() },
            ExprKind::Function { returns } => ValueKind::Function {
                returns: self.returns(file, returns.as_deref()),
            },
            ExprKind::OneOf(values) => ValueKind::OneOf {
                values: values.iter().map(|value| self.value(file, value)).collect(),
            },
            ExprKind::Reference { name, text, path } => {
                let reference = Name {
                    file,
                    text,
                    location: expr.location,
                };
                match name {
                    Reference::Variable(_) | Reference::Import(_) if self.in_call.get() => {
                        unresolved(UnresolvedCode::InCall, None, text)
                    }
                    Reference::Parameter(Some(_)) if self.context.props && self.in_call.get() => {
                        unresolved(UnresolvedCode::InCall, None, text)
                    }
                    Reference::Variable(_) => return self.value_at(file, expr, &[]),
                    Reference::Parameter(Some(_)) if self.context.props => {
                        return self.value_at(file, expr, &[]);
                    }
                    Reference::Import(index) if self.context.imports => {
                        return self.follow_import(&reference, *index, path);
                    }
                    Reference::Import(_) => {
                        unresolved(UnresolvedCode::NotFollowed, Some(Step::Imports), text)
                    }
                    Reference::Parameter(_) => unresolved(UnresolvedCode::Parameter, None, text),
                    Reference::Global => unresolved(UnresolvedCode::Global, None, text),
                }
            }
            ExprKind::Call {
                call: Some(call), ..
            } => ValueKind::Call(Box::new(model::Call {
                callee: call.callee.clone(),
                receiver: call
                    .receiver
                    .as_ref()
                    .map(|receiver| self.within(false, || self.value(file, receiver))),
                arguments: call
                    .arguments
                    .iter()
                    .map(|argument| match argument {
                        parse::Item::Value(value) => Item::Value {
                            value: self.in_call(true, || self.value(file, value)),
                        },
                        parse::Item::Spread(spread) => Item::Spread {
                            spread: self.in_call(false, || self.value(file, spread)),
                        },
                    })
                    .collect(),
                path: call.path.iter().map(key_name).collect(),
            })),
            ExprKind::Call { text, call: None } | ExprKind::Require { text, .. } => {
                unresolved(UnresolvedCode::Call, None, text)
            }
            ExprKind::Unsupported { text } => unresolved(UnresolvedCode::Unsupported, None, text),
        };
        self.at(file, expr.location, kind)
    }

    /// The value of `expr`, an expression of `file`, then `keys` in it. Props and variables
    /// holding props are followed with the keys, so only the attributes that set the first key
    /// are evaluated.
    fn value_at(&self, file: FileId, expr: &Expr, keys: &[Key]) -> Value {
        if let ExprKind::Reference { name, text, path } = &expr.kind {
            let reference = Name {
                file,
                text,
                location: expr.location,
            };
            let keys: Vec<Key> = path.iter().chain(keys).cloned().collect();
            let followed = match name {
                Reference::Variable(index) => {
                    Some(self.definition(&reference, file, *index, &keys))
                }
                Reference::Parameter(Some(index)) if self.context.props => {
                    Some(self.parameter(&reference, file, *index, keys))
                }
                _ => None,
            };
            if let Some(followed) = followed {
                return match followed {
                    Ok(value) => self.followed(&reference, value, &[]),
                    Err(stop) => stop,
                };
            }
        }
        let value = self.value(file, expr);
        keys.iter().fold(value, |value, key| {
            let text = match key {
                Key::Static(key) => key.as_str(),
                Key::Dynamic => "",
            };
            let name = Name {
                file,
                text,
                location: expr.location,
            };
            self.lookup(value, key, &name)
        })
    }

    /// What callers pass for the prop the name at `index` in `file`'s
    /// [`parse::FileSummary::parameters`] declares, then `keys` in it. `Err` is the value to
    /// report for `name`.
    fn parameter(
        &self,
        name: &Name,
        file: FileId,
        index: usize,
        keys: Vec<Key>,
    ) -> Result<Value, Value> {
        let context = self.context;
        let Some(summary) = context.graph.module(file).map(|module| &module.summary) else {
            return Err(self.stop(name, UnresolvedCode::Unparsed, None));
        };
        let parameter = &summary.parameters[index];
        if parameter.mutated {
            return Err(self.stop(name, UnresolvedCode::Mutated, None));
        }
        if let (Some(taken), Some(Key::Static(key))) = (&parameter.rest, keys.first())
            && taken.contains(key)
        {
            return Ok(self.at(file, parameter.location, ValueKind::Undefined));
        }
        let component = (file, parameter.component);
        let active = Active::Component(file, parameter.component);
        if self.active.borrow().contains(&active) {
            return Err(self.stop(name, UnresolvedCode::Unsupported, None));
        }
        let pinned = self
            .pins
            .iter()
            .find(|(pinned, _)| *pinned == component)
            .map(|(_, caller)| std::slice::from_ref(caller));
        let escaped = pinned.is_none() && context.escaped.contains(&component);
        let callers = pinned.unwrap_or_else(|| {
            context
                .callers
                .get(&component)
                .map_or(&[][..], Vec::as_slice)
        });
        if callers.is_empty() {
            let code = if escaped {
                UnresolvedCode::Parameter
            } else {
                UnresolvedCode::NoCallers
            };
            return Err(self.stop(name, code, None));
        }
        let keys: Vec<Key> = parameter
            .path
            .iter()
            .map(|segment| Key::Static(segment_key(segment)))
            .chain(keys)
            .collect();
        let declared = Declared { file, parameter };
        self.active.borrow_mut().push(active);
        let mut values: Vec<Value> = callers
            .iter()
            .filter_map(|&(caller, element)| {
                let element = context
                    .graph
                    .module(caller)?
                    .summary
                    .elements
                    .get(element)?;
                let mut value = self.caller(caller, element, &keys, &declared, name);
                value.via.insert(
                    0,
                    Via {
                        text: tag(element),
                        file: self.file(caller),
                        location: element.location,
                    },
                );
                Some(value)
            })
            .collect();
        self.active.borrow_mut().pop();
        if escaped {
            let component = &summary.components[parameter.component];
            values.push(self.at(
                file,
                component.location,
                unresolved(UnresolvedCode::OtherCallers, None, &component.name),
            ));
        }
        Ok(if values.len() == 1 {
            values.remove(0)
        } else {
            self.at(file, parameter.location, ValueKind::OneOf { values })
        })
    }

    /// What `element`, an element of `file`, passes for `keys`: the attribute that sets the
    /// first key, or every attribute as an object when there's none, without the keys a rest
    /// element leaves out. The keys up to the end of the parameter's path lead to its name, whose
    /// default replaces `undefined`.
    fn caller(
        &self,
        file: FileId,
        element: &JsxElement,
        keys: &[Key],
        declared: &Declared,
        name: &Name,
    ) -> Value {
        let parameter = declared.parameter;
        let taken = parameter.rest.as_deref().unwrap_or_default();
        let (mut value, consumed) = match keys.first() {
            Some(Key::Static(key)) => (self.caller_prop(file, element, key), 1),
            _ => (self.caller_props(file, element, taken), 0),
        };
        let boundary = parameter.path.len().max(consumed);
        for key in &keys[consumed..boundary] {
            value = self.lookup(value, key, name);
        }
        if let Some(default) = &parameter.default {
            value = self.with_default(value, declared.file, default);
        }
        for key in &keys[boundary..] {
            value = self.lookup(value, key, name);
        }
        value
    }

    /// The last attribute of `element` that sets `key`, or its children for `children`.
    fn caller_prop(&self, file: FileId, element: &JsxElement, key: &str) -> Value {
        if key == KEY {
            return self.at(file, element.location, ValueKind::Undefined);
        }
        let mut found = None;
        for attribute in &element.attributes {
            match attribute {
                Attribute::Named { name, value } if name == key => {
                    found = Some(self.value(file, value));
                }
                Attribute::Named { .. } => {}
                Attribute::Spread(spread) => {
                    let value = self.value_at(file, spread, &[Key::Static(key.to_owned())]);
                    if value.kind != ValueKind::Undefined {
                        found = Some(value);
                    }
                }
            }
        }
        if key == "children"
            && let Some(location) = element.children
        {
            found = Some(self.children(file, location));
        }
        found.unwrap_or_else(|| self.at(file, element.location, ValueKind::Undefined))
    }

    /// Every attribute of `element` as an object, with its children, but the `taken` ones.
    fn caller_props(&self, file: FileId, element: &JsxElement, taken: &[String]) -> Value {
        let mut entries: Vec<Entry> = element
            .attributes
            .iter()
            .filter_map(|attribute| match attribute {
                Attribute::Named { name, .. } if name == KEY || taken.contains(name) => None,
                Attribute::Named { name, value } => Some(Entry::Property {
                    key: name.clone(),
                    value: self.value(file, value),
                }),
                Attribute::Spread(spread) => Some(Entry::Spread {
                    spread: without(self.value(file, spread), taken),
                }),
            })
            .collect();
        if let Some(location) = element.children
            && !taken.iter().any(|key| key == "children")
        {
            entries.push(Entry::Property {
                key: "children".to_owned(),
                value: self.children(file, location),
            });
        }
        self.at(file, element.location, ValueKind::Object { entries })
    }

    fn children(&self, file: FileId, location: Location) -> Value {
        self.at(
            file,
            location,
            unresolved(UnresolvedCode::Unsupported, None, "children"),
        )
    }

    /// `value`, with `default`, an expression of `file`, in place of `undefined`, in each branch.
    fn with_default(&self, value: Value, file: FileId, default: &Expr) -> Value {
        match value.kind {
            ValueKind::Undefined => self.value(file, default),
            ValueKind::OneOf { values } => Value {
                kind: ValueKind::OneOf {
                    values: values
                        .into_iter()
                        .map(|value| self.with_default(value, file, default))
                        .collect(),
                },
                ..value
            },
            kind => Value { kind, ..value },
        }
    }

    fn follow_import(&self, name: &Name, index: usize, path: &[Key]) -> Value {
        let graph = self.context.graph;
        let mut origin = graph.origin(name.file, index);
        let mut path = path;
        let (module, export) = loop {
            match origin {
                Ok(Origin::Export(ExportRef { module, name })) => break (module, name),
                Ok(Origin::Namespace(module)) => match path.split_first() {
                    Some((Key::Static(export), rest)) => {
                        origin = graph.export_origin(module, export);
                        path = rest;
                    }
                    _ => return self.stop(name, UnresolvedCode::Unsupported, None),
                },
                Err(OriginError::MissingExport {
                    module,
                    name: export,
                }) if self.declared(module, &export) => break (module, export),
                Err(error) => return self.stop(name, code(&error), None),
            }
        };
        match self.export(name, module, export, path) {
            Ok(value) => self.followed(name, value, &[]),
            Err(stop) => stop,
        }
    }

    /// Whether `module` exports `export` only as a declaration without a value, such as
    /// `export declare const`, which isn't an ES export.
    fn declared(&self, module: FileId, export: &str) -> bool {
        let Some(summary) = self
            .context
            .graph
            .module(module)
            .map(|module| &module.summary)
        else {
            return false;
        };
        matches!(
            summary.exported.get(export),
            Some(Exported::Variable(index)) if summary.definitions[*index] == Definition::Declared
        )
    }

    /// `value`, reached through `name`, then `path` in it.
    fn followed(&self, name: &Name, value: Value, path: &[Key]) -> Value {
        let mut value = path
            .iter()
            .fold(value, |value, key| self.lookup(value, key, name));
        value.via.insert(
            0,
            Via {
                text: name.text.to_owned(),
                file: self.file(name.file),
                location: name.location,
            },
        );
        value
    }

    /// What the export `export` of `module` holds, then `keys` in it. `Err` is the value to
    /// report for `name`.
    fn export(
        &self,
        name: &Name,
        module: FileId,
        export: String,
        keys: &[Key],
    ) -> Result<Value, Value> {
        let context = self.context;
        let changed = context.changed_modules.contains(&module)
            || context.changed.contains(&ExportRef {
                module,
                name: export.clone(),
            });
        if changed {
            return Err(self.stop(name, UnresolvedCode::Mutated, None));
        }
        let exported = context
            .graph
            .module(module)
            .and_then(|module| module.summary.exported.get(&export));
        let active = Active::Export(module, export);
        let Some(exported) = exported.filter(|_| !self.active.borrow().contains(&active)) else {
            return Err(self.stop(name, UnresolvedCode::Unsupported, None));
        };
        self.active.borrow_mut().push(active);
        let value = match exported {
            Exported::Variable(index) => self.definition(name, module, *index, keys),
            Exported::Value(expr) => {
                Ok(keys.iter().fold(self.value(module, expr), |value, key| {
                    self.lookup(value, key, name)
                }))
            }
        };
        self.active.borrow_mut().pop();
        value
    }

    /// What the name defined at `index` in `file` holds, then `keys` in it. `Err` is the value to
    /// report for `name`.
    fn definition(
        &self,
        name: &Name,
        file: FileId,
        index: usize,
        keys: &[Key],
    ) -> Result<Value, Value> {
        let active = Active::Definition(file, index);
        if self.active.borrow().contains(&active) {
            return Err(self.stop(name, UnresolvedCode::Unsupported, None));
        }
        let Some(summary) = self
            .context
            .graph
            .module(file)
            .map(|module| &module.summary)
        else {
            return Err(self.stop(name, UnresolvedCode::Unparsed, None));
        };
        match &summary.definitions[index] {
            Definition::NotFollowed => {
                Err(self.stop(name, UnresolvedCode::NotFollowed, Some(Step::Variables)))
            }
            Definition::Mutated => Err(self.stop(name, UnresolvedCode::Mutated, None)),
            Definition::Declared => Err(self.stop(name, UnresolvedCode::Unsupported, None)),
            Definition::Function(location) => {
                let value = self.at(file, *location, ValueKind::Function { returns: None });
                Ok(keys
                    .iter()
                    .fold(value, |value, key| self.lookup(value, key, name)))
            }
            Definition::Value {
                init,
                path,
                default,
            } => {
                self.active.borrow_mut().push(active);
                let segments = path.iter().map(|segment| Key::Static(segment_key(segment)));
                let props = matches!(
                    init.kind,
                    ExprKind::Reference {
                        name: Reference::Parameter(Some(_)),
                        ..
                    }
                ) && self.context.props;
                let (mut value, rest) = if props {
                    let mut lazy: Vec<Key> = segments.collect();
                    let rest = if default.is_some() {
                        keys
                    } else {
                        lazy.extend_from_slice(keys);
                        &[]
                    };
                    (self.value_at(file, init, &lazy), rest)
                } else {
                    let value = segments.fold(self.value(file, init), |value, key| {
                        self.lookup(value, &key, name)
                    });
                    (value, keys)
                };
                if let Some(default) = default {
                    value = self.with_default(value, file, default);
                }
                for key in rest {
                    value = self.lookup(value, key, name);
                }
                self.active.borrow_mut().pop();
                Ok(value)
            }
        }
    }

    /// `value[key]`. Keys that can't be looked up are reported at `name`.
    fn lookup(&self, value: Value, key: &Key, name: &Name) -> Value {
        let unsupported = || self.stop(name, UnresolvedCode::Unsupported, None);
        let Value {
            kind,
            file,
            location: base,
            via,
        } = value;
        let here = |kind| Value {
            kind,
            file: file.clone(),
            location: base,
            via: Vec::new(),
        };
        let mut found = match (kind, key) {
            (ValueKind::Object { entries }, Key::Static(key)) => match property(&entries, key) {
                Found::Value(value) => value,
                Found::Absent => here(ValueKind::Undefined),
                Found::Unknown => unsupported(),
            },
            (ValueKind::Array { items }, Key::Static(key)) => match key.parse::<usize>() {
                Ok(index) => match element(&items, index) {
                    Found::Value(value) => value,
                    Found::Absent => here(ValueKind::Undefined),
                    Found::Unknown => unsupported(),
                },
                Err(_) => unsupported(),
            },
            (ValueKind::Object { entries }, Key::Dynamic) => {
                let mut values = Vec::new();
                entry_values(entries, &mut values);
                here(one_of(values))
            }
            (ValueKind::Array { items }, Key::Dynamic) => {
                let mut values = Vec::new();
                item_values(items, &mut values);
                here(one_of(values))
            }
            (ValueKind::OneOf { values }, key) => here(ValueKind::OneOf {
                values: values
                    .into_iter()
                    .map(|value| self.lookup(value, key, name))
                    .collect(),
            }),
            (ValueKind::Call(mut call), key) => {
                call.path.push(key_name(key));
                here(ValueKind::Call(call))
            }
            (kind @ ValueKind::Unresolved { .. }, _) => here(kind),
            _ => unsupported(),
        };
        found.via.splice(0..0, via);
        found
    }

    /// The values of a function's `returns`, when it's within a call's argument.
    fn returns(&self, file: FileId, returns: Option<&[Expr]>) -> Option<Vec<Value>> {
        let returns = returns.filter(|_| self.argument.get())?;
        Some(self.within(false, || {
            returns.iter().map(|expr| self.value(file, expr)).collect()
        }))
    }

    fn within<T>(&self, argument: bool, find: impl FnOnce() -> T) -> T {
        let outer = self.argument.replace(argument);
        let found = find();
        self.argument.set(outer);
        found
    }

    /// `find` within a call's arguments, with `argument` as [`Self::within`].
    fn in_call<T>(&self, argument: bool, find: impl FnOnce() -> T) -> T {
        let outer = self.in_call.replace(true);
        let found = self.within(argument, find);
        self.in_call.set(outer);
        found
    }

    /// `name` as an unresolved value with `code`.
    fn stop(&self, name: &Name, code: UnresolvedCode, step: Option<Step>) -> Value {
        self.at(name.file, name.location, unresolved(code, step, name.text))
    }

    fn at(&self, file: FileId, location: Location, kind: ValueKind) -> Value {
        Value {
            kind,
            file: self.file(file),
            location,
            via: Vec::new(),
        }
    }

    /// The display path of `file`, unless it's the usage's.
    fn file(&self, file: FileId) -> Option<String> {
        (file != self.usage).then(|| self.context.paths[file.0 as usize].clone())
    }
}

/// `value` without the properties `keys` names, in an object and in each object spread into it
/// or branch of a `one-of`.
fn without(mut value: Value, keys: &[String]) -> Value {
    if keys.is_empty() {
        return value;
    }
    value.kind = match value.kind {
        ValueKind::Object { entries } => ValueKind::Object {
            entries: entries
                .into_iter()
                .filter_map(|entry| match entry {
                    Entry::Property { key, .. } if keys.contains(&key) => None,
                    Entry::Spread { spread } => Some(Entry::Spread {
                        spread: without(spread, keys),
                    }),
                    entry => Some(entry),
                })
                .collect(),
        },
        ValueKind::OneOf { values } => ValueKind::OneOf {
            values: values
                .into_iter()
                .map(|value| without(value, keys))
                .collect(),
        },
        kind => kind,
    };
    value
}

/// `value` with `returns` only on functions that are `argument`s of a call, directly or as a
/// branch of a `one-of`.
fn returned_by_arguments(mut value: Value, argument: bool) -> Value {
    value.kind = match value.kind {
        ValueKind::Function { returns } => ValueKind::Function {
            returns: returns.filter(|_| argument).map(|returns| {
                returns
                    .into_iter()
                    .map(|value| returned_by_arguments(value, false))
                    .collect()
            }),
        },
        ValueKind::OneOf { values } => ValueKind::OneOf {
            values: values
                .into_iter()
                .map(|value| returned_by_arguments(value, argument))
                .collect(),
        },
        ValueKind::Call(call) => ValueKind::Call(Box::new(model::Call {
            receiver: call
                .receiver
                .map(|receiver| returned_by_arguments(receiver, false)),
            arguments: call
                .arguments
                .into_iter()
                .map(|item| match item {
                    Item::Value { value } => Item::Value {
                        value: returned_by_arguments(value, true),
                    },
                    Item::Spread { spread } => Item::Spread {
                        spread: returned_by_arguments(spread, false),
                    },
                })
                .collect(),
            callee: call.callee,
            path: call.path,
        })),
        ValueKind::Object { entries } => ValueKind::Object {
            entries: entries
                .into_iter()
                .map(|entry| match entry {
                    Entry::Property { key, value } => Entry::Property {
                        key,
                        value: returned_by_arguments(value, false),
                    },
                    Entry::Computed { computed, value } => Entry::Computed {
                        computed,
                        value: returned_by_arguments(value, false),
                    },
                    Entry::Spread { spread } => Entry::Spread {
                        spread: returned_by_arguments(spread, false),
                    },
                })
                .collect(),
        },
        ValueKind::Array { items } => ValueKind::Array {
            items: items
                .into_iter()
                .map(|item| match item {
                    Item::Value { value } => Item::Value {
                        value: returned_by_arguments(value, false),
                    },
                    Item::Spread { spread } => Item::Spread {
                        spread: returned_by_arguments(spread, false),
                    },
                })
                .collect(),
        },
        ValueKind::Template { tag, parts } => ValueKind::Template {
            tag,
            parts: parts
                .into_iter()
                .map(|part| match part {
                    Part::Value { value } => Part::Value {
                        value: returned_by_arguments(value, false),
                    },
                    text => text,
                })
                .collect(),
        },
        kind => kind,
    };
    value
}

fn key_name(key: &Key) -> Option<String> {
    match key {
        Key::Static(key) => Some(key.clone()),
        Key::Dynamic => None,
    }
}

fn segment_key(segment: &Segment) -> String {
    match segment {
        Segment::Key(key) => key.clone(),
        Segment::Index(index) => index.to_string(),
    }
}

/// The tag of `element` as written: `Panel` or `C.Panel`.
fn tag(element: &JsxElement) -> String {
    std::iter::once(element.root.as_str())
        .chain(element.members.iter().map(String::as_str))
        .collect::<Vec<_>>()
        .join(".")
}

fn code(error: &OriginError) -> UnresolvedCode {
    match error {
        OriginError::Unresolved { .. } => UnresolvedCode::UnresolvedImport,
        OriginError::Unparsed { .. } => UnresolvedCode::Unparsed,
        OriginError::MissingExport { .. } => UnresolvedCode::MissingExport,
        OriginError::NoEsExports { .. } => UnresolvedCode::CommonjsModule,
        OriginError::Ambiguous { .. } => UnresolvedCode::Ambiguous,
        OriginError::Cycle { .. } | OriginError::MissingImport { .. } => {
            UnresolvedCode::Unsupported
        }
    }
}

enum Found {
    Value(Value),
    /// Not written, so `undefined`.
    Absent,
    /// May be set by a spread or a computed key.
    Unknown,
}

/// The last entry that sets `key`, looking into spread objects.
fn property(entries: &[Entry], key: &str) -> Found {
    let mut found = Found::Absent;
    for entry in entries {
        match entry {
            Entry::Property { key: name, value } if name == key => {
                found = Found::Value(value.clone());
            }
            Entry::Property { .. } => {}
            Entry::Computed { .. } => found = Found::Unknown,
            Entry::Spread { spread } => match &spread.kind {
                ValueKind::Object { entries } => match property(entries, key) {
                    Found::Absent => {}
                    inner => found = inner,
                },
                _ => found = Found::Unknown,
            },
        }
    }
    found
}

/// The item at `index`, expanding spread arrays.
fn element(items: &[Item], mut index: usize) -> Found {
    for item in items {
        match item {
            Item::Value { value } if index == 0 => return Found::Value(value.clone()),
            Item::Value { .. } => index -= 1,
            Item::Spread { spread } => match &spread.kind {
                ValueKind::Array { items } => {
                    let length = items.len();
                    if items.iter().any(|item| matches!(item, Item::Spread { .. })) {
                        return Found::Unknown;
                    }
                    if index < length {
                        return element(items, index);
                    }
                    index -= length;
                }
                _ => return Found::Unknown,
            },
        }
    }
    Found::Absent
}

/// Every value an object can hold, expanding spread objects. Other spreads are listed as they are.
fn entry_values(entries: Vec<Entry>, values: &mut Vec<Value>) {
    for entry in entries {
        match entry {
            Entry::Property { value, .. } | Entry::Computed { value, .. } => values.push(value),
            Entry::Spread { spread } => match spread.kind {
                ValueKind::Object { entries } => entry_values(entries, values),
                _ => values.push(spread),
            },
        }
    }
}

/// Every value an array can hold, expanding spread arrays. Other spreads are listed as they are.
fn item_values(items: Vec<Item>, values: &mut Vec<Value>) {
    for item in items {
        match item {
            Item::Value { value } => values.push(value),
            Item::Spread { spread } => match spread.kind {
                ValueKind::Array { items } => item_values(items, values),
                _ => values.push(spread),
            },
        }
    }
}

/// `undefined` for an empty object or array.
fn one_of(values: Vec<Value>) -> ValueKind {
    if values.is_empty() {
        ValueKind::Undefined
    } else {
        ValueKind::OneOf { values }
    }
}

fn unresolved(code: UnresolvedCode, step: Option<Step>, text: &str) -> ValueKind {
    ValueKind::Unresolved {
        code,
        step,
        text: text.to_owned(),
    }
}

fn literal_of(literal: &parse::Literal) -> Literal {
    match literal {
        parse::Literal::String(string) => Literal::String(string.clone()),
        parse::Literal::Number(number) => Literal::number(*number),
        parse::Literal::Boolean(boolean) => Literal::Boolean(*boolean),
        parse::Literal::Null => Literal::Null,
    }
}
