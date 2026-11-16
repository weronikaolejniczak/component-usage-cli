//! Resolves the values a prop can receive, following variables, imports and props as configured.
//!
//! Pure. Every value has a kind and a location; unresolved values carry a code.

use std::cell::RefCell;

use model::{
    Entry, Item, Literal, Location, Part, Prop, Step, UnresolvedCode, Value, ValueKind, Via,
};
use parse::{Attribute, Definition, Expr, ExprKind, FileSummary, Key, Reference};

/// The attributes of `element`, an element of `file`, in source order. With `filter`, only the
/// named ones, and spreads, which may set any prop.
pub fn props(
    file: &FileSummary,
    element: &parse::JsxElement,
    filter: Option<&[String]>,
) -> Vec<Prop> {
    let resolver = Resolver {
        definitions: &file.definitions,
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
                    value: resolver.value(value),
                })
            }
            Attribute::Spread(spread) => Some(Prop::Spread {
                spread: resolver.value(spread),
            }),
        })
        .collect()
}

struct Resolver<'f> {
    definitions: &'f [Definition],
    /// Definitions being followed, to stop at names that refer back to themselves.
    active: RefCell<Vec<usize>>,
}

impl Resolver<'_> {
    fn value(&self, expr: &Expr) -> Value {
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
                            value: self.value(value),
                        },
                        parse::Entry::Computed { key, value } => Entry::Computed {
                            computed: key.clone(),
                            value: self.value(value),
                        },
                        parse::Entry::Spread(spread) => Entry::Spread {
                            spread: self.value(spread),
                        },
                    })
                    .collect(),
            },
            ExprKind::Array(items) => ValueKind::Array {
                items: items
                    .iter()
                    .map(|item| match item {
                        parse::Item::Value(value) => Item::Value {
                            value: self.value(value),
                        },
                        parse::Item::Spread(spread) => Item::Spread {
                            spread: self.value(spread),
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
                            value: self.value(value),
                        },
                    })
                    .collect(),
            },
            ExprKind::Element { tag } => ValueKind::Element { tag: tag.clone() },
            ExprKind::Function => ValueKind::Function,
            ExprKind::OneOf(values) => ValueKind::OneOf {
                values: values.iter().map(|value| self.value(value)).collect(),
            },
            ExprKind::Reference { name, text, path } => match name {
                Reference::Variable(index) => {
                    return self.follow(*index, text, path, expr.location);
                }
                Reference::Import(_) => {
                    unresolved(UnresolvedCode::NotFollowed, Some(Step::Imports), text)
                }
                Reference::Parameter => unresolved(UnresolvedCode::Parameter, None, text),
                Reference::Global => unresolved(UnresolvedCode::Global, None, text),
            },
            ExprKind::Call { text } => unresolved(UnresolvedCode::Call, None, text),
            ExprKind::Unsupported { text } => unresolved(UnresolvedCode::Unsupported, None, text),
        };
        at(expr.location, kind)
    }

    /// The value of the name defined at `index`, then of `path` in it.
    fn follow(&self, index: usize, text: &str, path: &[Key], location: Location) -> Value {
        let stop = |code, step| at(location, unresolved(code, step, text));
        if self.active.borrow().contains(&index) {
            return stop(UnresolvedCode::Unsupported, None);
        }
        let base = match &self.definitions[index] {
            Definition::NotFollowed => {
                return stop(UnresolvedCode::NotFollowed, Some(Step::Variables));
            }
            Definition::Mutated => return stop(UnresolvedCode::Mutated, None),
            Definition::Function(location) => at(*location, ValueKind::Function),
            Definition::Value {
                init,
                path: pattern,
                default,
            } => {
                self.active.borrow_mut().push(index);
                let mut value = self.value(init);
                for key in pattern {
                    value = lookup(value, &Key::Static(key.clone()), text, location);
                }
                if let Some(default) = default
                    && value.kind == ValueKind::Undefined
                {
                    value = self.value(default);
                }
                self.active.borrow_mut().pop();
                value
            }
        };
        let mut value = path
            .iter()
            .fold(base, |value, key| lookup(value, key, text, location));
        value.via.insert(
            0,
            Via {
                text: text.to_owned(),
                location,
            },
        );
        value
    }
}

/// `value[key]`. `text` and `location` are the reference's, for keys that can't be looked up.
fn lookup(value: Value, key: &Key, text: &str, location: Location) -> Value {
    let unsupported = || {
        at(
            location,
            unresolved(UnresolvedCode::Unsupported, None, text),
        )
    };
    let Value {
        kind,
        location: base,
        via,
    } = value;
    let mut found = match (kind, key) {
        (ValueKind::Object { entries }, Key::Static(key)) => match property(&entries, key) {
            Found::Value(value) => value,
            Found::Absent => at(base, ValueKind::Undefined),
            Found::Unknown => unsupported(),
        },
        (ValueKind::Array { items }, Key::Static(key)) => match key.parse::<usize>() {
            Ok(index) => match element(&items, index) {
                Found::Value(value) => value,
                Found::Absent => at(base, ValueKind::Undefined),
                Found::Unknown => unsupported(),
            },
            Err(_) => unsupported(),
        },
        (ValueKind::Object { entries }, Key::Dynamic) => {
            let mut values = Vec::new();
            entry_values(entries, &mut values);
            one_of(base, values)
        }
        (ValueKind::Array { items }, Key::Dynamic) => {
            let mut values = Vec::new();
            item_values(items, &mut values);
            one_of(base, values)
        }
        (ValueKind::OneOf { values }, key) => at(
            base,
            ValueKind::OneOf {
                values: values
                    .into_iter()
                    .map(|value| lookup(value, key, text, location))
                    .collect(),
            },
        ),
        (kind @ ValueKind::Unresolved { .. }, _) => Value {
            kind,
            location: base,
            via: Vec::new(),
        },
        _ => unsupported(),
    };
    found.via.splice(0..0, via);
    found
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
fn one_of(location: Location, values: Vec<Value>) -> Value {
    if values.is_empty() {
        at(location, ValueKind::Undefined)
    } else {
        at(location, ValueKind::OneOf { values })
    }
}

fn at(location: Location, kind: ValueKind) -> Value {
    Value {
        kind,
        location,
        via: Vec::new(),
    }
}

fn unresolved(code: UnresolvedCode, step: Option<Step>, text: &str) -> ValueKind {
    ValueKind::Unresolved {
        code,
        step,
        text: text.to_owned(),
    }
}

/// Largest magnitude below which every integer is exactly representable as an `f64`.
const SAFE_INTEGER: f64 = 9_007_199_254_740_992.0;

fn literal_of(literal: &parse::Literal) -> Literal {
    match literal {
        parse::Literal::String(string) => Literal::String(string.clone()),
        parse::Literal::Number(number) if number.fract() == 0.0 && number.abs() < SAFE_INTEGER => {
            Literal::Integer(*number as i64)
        }
        parse::Literal::Number(number) => Literal::Float(*number),
        parse::Literal::Boolean(boolean) => Literal::Boolean(*boolean),
        parse::Literal::Null => Literal::Null,
    }
}
