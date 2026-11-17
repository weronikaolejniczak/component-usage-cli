//! Resolves the values a prop can receive, following the names [`parse`] recorded definitions
//! for, and with [`Context::new`]'s `imports`, imported names through the module graph. What is
//! recorded, and so followed, is set by [`parse::Options`].
//!
//! Pure. Every value has a kind and a location; unresolved values carry a code. Values and names
//! in another file than the usage's say which.

use std::cell::RefCell;
use std::collections::HashSet;

use graph::{ModuleGraph, Origin, OriginError};
use model::{
    Entry, ExportRef, FileId, Item, Literal, Location, Part, Prop, Step, UnresolvedCode, Value,
    ValueKind, Via,
};
use parse::{Attribute, Definition, Exported, Expr, ExprKind, Key, Reference, Segment};

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
}

impl<'g> Context<'g> {
    /// With `imports`, imported names are followed; the graph must then have resolved
    /// [`graph::value_specifiers`].
    #[must_use]
    pub fn new(graph: &'g ModuleGraph, paths: &'g [String], imports: bool) -> Self {
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
        Self {
            graph,
            paths,
            imports,
            changed,
            changed_modules,
        }
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
    let resolver = Resolver {
        context,
        usage: file,
        active: RefCell::new(Vec::new()),
    };
    element
        .attributes
        .iter()
        .filter_map(|attribute| match attribute {
            Attribute::Named { name, value } => {
                let wanted = filter.is_none_or(|names| names.contains(name));
                wanted.then(|| Prop::Named {
                    name: name.clone(),
                    value: resolver.value(file, value),
                })
            }
            Attribute::Spread(spread) => Some(Prop::Spread {
                spread: resolver.value(file, spread),
            }),
        })
        .collect()
}

struct Resolver<'c, 'g> {
    context: &'c Context<'g>,
    usage: FileId,
    /// What is being followed, to stop at names that refer back to themselves.
    active: RefCell<Vec<Active>>,
}

#[derive(PartialEq, Eq)]
enum Active {
    Definition(FileId, usize),
    Export(FileId, String),
}

/// A name being followed: what's written, and where.
struct Name<'n> {
    file: FileId,
    text: &'n str,
    location: Location,
}

impl Resolver<'_, '_> {
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
            ExprKind::Function => ValueKind::Function,
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
                    Reference::Variable(index) => {
                        return self.follow_variable(&reference, *index, path);
                    }
                    Reference::Import(index) if self.context.imports => {
                        return self.follow_import(&reference, *index, path);
                    }
                    Reference::Import(_) => {
                        unresolved(UnresolvedCode::NotFollowed, Some(Step::Imports), text)
                    }
                    Reference::Parameter => unresolved(UnresolvedCode::Parameter, None, text),
                    Reference::Global => unresolved(UnresolvedCode::Global, None, text),
                }
            }
            ExprKind::Call { text } | ExprKind::Require { text, .. } => {
                unresolved(UnresolvedCode::Call, None, text)
            }
            ExprKind::Unsupported { text } => unresolved(UnresolvedCode::Unsupported, None, text),
        };
        self.at(file, expr.location, kind)
    }

    fn follow_variable(&self, name: &Name, index: usize, path: &[Key]) -> Value {
        match self.definition(name, name.file, index) {
            Ok(value) => self.followed(name, value, path),
            Err(stop) => stop,
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
        match self.export(name, module, export) {
            Ok(value) => self.followed(name, value, path),
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

    /// What the export `export` of `module` holds. `Err` is the value to report for `name`.
    fn export(&self, name: &Name, module: FileId, export: String) -> Result<Value, Value> {
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
            Exported::Variable(index) => self.definition(name, module, *index),
            Exported::Value(expr) => Ok(self.value(module, expr)),
        };
        self.active.borrow_mut().pop();
        value
    }

    /// What the name defined at `index` in `file` holds. `Err` is the value to report for `name`.
    fn definition(&self, name: &Name, file: FileId, index: usize) -> Result<Value, Value> {
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
            Definition::Function(location) => Ok(self.at(file, *location, ValueKind::Function)),
            Definition::Value {
                init,
                path,
                default,
            } => {
                self.active.borrow_mut().push(active);
                let mut value = self.value(file, init);
                for segment in path {
                    let key = match segment {
                        Segment::Key(key) => key.clone(),
                        Segment::Index(index) => index.to_string(),
                    };
                    value = self.lookup(value, &Key::Static(key), name);
                }
                if let Some(default) = default
                    && value.kind == ValueKind::Undefined
                {
                    value = self.value(file, default);
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
            (kind @ ValueKind::Unresolved { .. }, _) => here(kind),
            _ => unsupported(),
        };
        found.via.splice(0..0, via);
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
