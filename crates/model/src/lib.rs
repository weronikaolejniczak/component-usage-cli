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
pub struct Usage {
    pub file: FileId,
    pub location: Location,
    /// Exported name, with member access for compound components, e.g. `Icon.Item`.
    pub component: String,
    /// Index of the element in its file's JSX elements.
    pub element: usize,
    /// Enclosing usages, nearest first.
    pub parents: Vec<Parent>,
    /// The wrappers from the element to the component, outermost first; empty for an element
    /// of the component itself.
    pub through: Vec<Through>,
}

/// A place a target is read as a value rather than rendered as a JSX tag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reference {
    pub file: FileId,
    pub location: Location,
    pub component: String,
    pub passed: Passed,
}

/// Where a value goes.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize),
    serde(tag = "kind", rename_all = "kebab-case")
)]
pub enum Passed {
    /// A JSX attribute of the element with the tag `element`, as written.
    Attribute {
        element: String,
        name: String,
    },
    /// An argument of a call or `new`, from 0.
    Argument {
        callee: String,
        index: usize,
    },
    /// An object's property, with its static `key`, or the source text of a `computed` one.
    Property {
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        key: Option<String>,
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        computed: Option<String>,
    },
    /// An array's element, from 0.
    Item {
        index: usize,
    },
    /// The value of a variable declared with a name.
    Variable {
        name: String,
    },
    Other,
}

/// A wrapper a usage goes through, and where it passes its props on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Through {
    pub component: String,
    pub file: FileId,
    pub location: Location,
    /// Index of the wrapper in its file's components.
    pub wrapper: usize,
    /// Indices of the element the wrapper spreads its props on and of the spread among its
    /// attributes; `None` when a call passes them on.
    pub spread: Option<(usize, usize)>,
    /// The props the wrapper's rest pattern takes, which aren't passed on.
    pub excluded: Vec<String>,
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
    Named {
        name: String,
        value: Value,
    },
    Spread {
        spread: Value,
        /// Whether it's the spread a wrapper passes its props on with, whose values belong to
        /// the usages through the wrapper.
        #[cfg_attr(feature = "serde", serde(skip))]
        forwards: bool,
    },
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Value {
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub kind: ValueKind,
    /// The file the value is in, when it isn't the usage's.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub file: Option<String>,
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub location: Location,
    /// The names followed to reach the value, outermost first.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Vec::is_empty"))]
    pub via: Vec<Via>,
}

/// A name followed to reach a value, with its source text and position.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Via {
    pub text: String,
    /// The file the name is in, when it isn't the usage's.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub file: Option<String>,
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
    Function {
        /// With `--follow calls`, for a function that is an argument of a call: the value of each
        /// `return` statement in its body, or its body without braces. `None` otherwise, and for
        /// async functions and generators.
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        returns: Option<Vec<Value>>,
    },
    /// With `--follow calls`, a call, described without evaluating it.
    Call(Box<Call>),
    OneOf {
        values: Vec<Value>,
    },
    /// A unary or binary operator, as written, and its operands in source order. It isn't
    /// evaluated.
    Operation {
        operator: String,
        operands: Vec<Value>,
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

/// Largest magnitude below which every integer is exactly representable as an `f64`.
const SAFE_INTEGER: f64 = 9_007_199_254_740_992.0;

impl Literal {
    /// [`Literal::Integer`] when `number` has no fractional part and converts exactly.
    #[must_use]
    pub fn number(number: f64) -> Self {
        if number.fract() == 0.0 && number.abs() < SAFE_INTEGER {
            Self::Integer(number as i64)
        } else {
            Self::Float(number)
        }
    }
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
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Call {
    /// The source text of what's called.
    pub callee: String,
    /// The object of a member call, as `items` in `items.map(…)`.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub receiver: Option<Value>,
    pub arguments: Vec<Item>,
    /// Properties read from the call's result; `None` for a computed key that isn't a literal.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Vec::is_empty"))]
    pub path: Vec<Option<String>>,
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
    /// A binding that is assigned to after its declaration, declared more than once, or whose
    /// contents are changed in the file.
    Mutated,
    /// A function parameter, or a property of one, that isn't followed: following props is off,
    /// the function isn't a component, or it isn't rendered as a JSX tag and is used another way.
    Parameter,
    /// A component's prop, where the component isn't rendered as a JSX tag or used any other way.
    NoCallers,
    /// An item of an empty array, passed to a callback that isn't called.
    NoItems,
    /// One of a prop's values: the component is also used other than as a JSX tag, so it may
    /// receive values that aren't listed.
    OtherCallers,
    /// A name not declared in the file.
    Global,
    /// The result of a call, without `--follow calls`.
    Call,
    /// A name in a call's arguments or in what a function among them returns, which isn't
    /// followed.
    InCall,
    /// An import whose specifier doesn't resolve to a file.
    UnresolvedImport,
    /// An import of a file that couldn't be parsed, or isn't JavaScript or TypeScript.
    Unparsed,
    /// An import of a name the module doesn't export.
    MissingExport,
    /// An import from a module without ES exports.
    CommonjsModule,
    /// An import of a name exported by more than one `export *`.
    Ambiguous,
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
