//! Resolves the values a prop can receive, following variables, imports and props as configured.
//!
//! Pure. Every value has a kind and a location; unresolved values carry a code.

use model::{Entry, Item, Literal, Part, Prop, Step, UnresolvedCode, Value, ValueKind};
use parse::{Attribute, Expr, ExprKind, Reference};

/// The attributes of `element` in source order. With `filter`, only the named ones, and spreads,
/// which may set any prop.
pub fn props(element: &parse::JsxElement, filter: Option<&[String]>) -> Vec<Prop> {
    element
        .attributes
        .iter()
        .filter_map(|attribute| match attribute {
            Attribute::Named { name, value } => {
                let wanted = filter.is_none_or(|names| names.contains(name));
                wanted.then(|| Prop::Named {
                    name: name.clone(),
                    value: value_of(value),
                })
            }
            Attribute::Spread(spread) => Some(Prop::Spread {
                spread: value_of(spread),
            }),
        })
        .collect()
}

fn value_of(expr: &Expr) -> Value {
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
                        value: value_of(value),
                    },
                    parse::Entry::Computed { key, value } => Entry::Computed {
                        computed: key.clone(),
                        value: value_of(value),
                    },
                    parse::Entry::Spread(spread) => Entry::Spread {
                        spread: value_of(spread),
                    },
                })
                .collect(),
        },
        ExprKind::Array(items) => ValueKind::Array {
            items: items
                .iter()
                .map(|item| match item {
                    parse::Item::Value(value) => Item::Value {
                        value: value_of(value),
                    },
                    parse::Item::Spread(spread) => Item::Spread {
                        spread: value_of(spread),
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
                        value: value_of(value),
                    },
                })
                .collect(),
        },
        ExprKind::Element { tag } => ValueKind::Element { tag: tag.clone() },
        ExprKind::Function => ValueKind::Function,
        ExprKind::OneOf(values) => ValueKind::OneOf {
            values: values.iter().map(value_of).collect(),
        },
        ExprKind::Reference { name, text } => {
            let (code, step) = match name {
                Reference::Import(_) => (UnresolvedCode::NotFollowed, Some(Step::Imports)),
                Reference::Variable => (UnresolvedCode::NotFollowed, Some(Step::Variables)),
                Reference::Parameter => (UnresolvedCode::Parameter, None),
                Reference::Global => (UnresolvedCode::Global, None),
            };
            unresolved(code, step, text)
        }
        ExprKind::Call { text } => unresolved(UnresolvedCode::Call, None, text),
        ExprKind::Unsupported { text } => unresolved(UnresolvedCode::Unsupported, None, text),
    };
    Value {
        kind,
        location: expr.location,
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
