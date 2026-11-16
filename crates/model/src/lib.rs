//! Data types shared by every stage: file IDs, locations, export references, usages.
//!
//! Plain owned data only. No I/O, no parsing, no AST types.

use std::fmt;

/// Index into the scan's file table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FileId(pub u32);

/// Position in a source file. Both fields are 1-based; `column` counts UTF-16 code units.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Location {
    pub line: u32,
    pub column: u32,
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

/// A binding as exported by the module that defines it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ExportRef {
    pub module: FileId,
    pub name: String,
}

/// One JSX opening element that renders a target component.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Usage {
    pub file: FileId,
    pub location: Location,
    /// Exported name, with member access for compound components, e.g. `Icon.Item`.
    pub component: String,
    /// Index of the element in its file's JSX elements.
    pub element: usize,
    /// Enclosing usages, nearest first.
    pub parents: Vec<Parent>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Parent {
    pub component: String,
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub location: Location,
    /// The parent's attribute the usage is in; `None` for children.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub prop: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize), serde(untagged))]
pub enum Prop {
    Named { name: String, value: Value },
    Spread { spread: Value },
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Value {
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub kind: ValueKind,
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub location: Location,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize),
    serde(tag = "kind", rename_all = "kebab-case")
)]
pub enum ValueKind {
    Literal {
        value: Literal,
    },
    /// `undefined`, `void 0` or an empty expression.
    Undefined,
    Object {
        entries: Vec<Entry>,
    },
    Array {
        items: Vec<Item>,
    },
    /// A template literal with expressions, or any tagged template.
    Template {
        /// The tag's source text.
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        tag: Option<String>,
        parts: Vec<Part>,
    },
    /// A JSX element, or a fragment when `tag` is `None`.
    Element {
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        tag: Option<String>,
    },
    Function,
    OneOf {
        values: Vec<Value>,
    },
    Unresolved {
        code: UnresolvedCode,
        /// The step that would follow the value, for [`UnresolvedCode::NotFollowed`].
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        step: Option<Step>,
        /// The expression's source.
        text: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize), serde(untagged))]
pub enum Literal {
    String(String),
    /// A number without a fractional part that converts to `f64` and back exactly.
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Null,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize), serde(untagged))]
pub enum Entry {
    Property {
        key: String,
        value: Value,
    },
    /// A key that isn't static, with its source text.
    Computed {
        computed: String,
        value: Value,
    },
    Spread {
        spread: Value,
    },
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize), serde(untagged))]
pub enum Item {
    Value { value: Value },
    Spread { spread: Value },
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize), serde(untagged))]
pub enum Part {
    Text { text: String },
    Value { value: Value },
}

/// Why a value is unresolved. Stable across releases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize),
    serde(rename_all = "kebab-case")
)]
pub enum UnresolvedCode {
    /// A binding the scan would follow with [`Step`], which is off or not implemented.
    NotFollowed,
    /// A function parameter, or a property of one.
    Parameter,
    /// A name not declared in the file.
    Global,
    /// The result of a call.
    Call,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize),
    serde(rename_all = "kebab-case")
)]
pub enum Step {
    Variables,
    Imports,
}
