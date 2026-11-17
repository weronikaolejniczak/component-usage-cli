//! Parses one source file and extracts an owned `FileSummary`: imports, exports and JSX elements
//! with their attributes.
//!
//! Pure: takes a path and source text, does no I/O. The Oxc AST and its arena never leave this crate.
//! The path only selects the dialect from its extension. JavaScript files always allow JSX;
//! `.ts` files never do, since `<T>` there is a type assertion or generic.
//! Type-only imports and exports are left out: they can't be rendered.

mod aliases;
mod attributes;
mod definitions;
mod elements;
mod exported;
mod extract;
mod lines;
mod patterns;
mod reads;
mod scope;

use std::collections::HashMap;
use std::path::Path;

use model::Location;

/// What later stages need from one file. Every list is in source order.
#[derive(Debug, Clone, PartialEq)]
pub struct FileSummary {
    pub imports: Vec<Import>,
    pub exports: Vec<Export>,
    pub elements: Vec<JsxElement>,
    /// Local names that JSX tags refer to imports or `require` through, indexed by
    /// [`Binding::Alias`].
    pub aliases: Vec<Alias>,
    /// What the names in attributes and exports are declared as, indexed by
    /// [`Reference::Variable`].
    pub definitions: Vec<Definition>,
    /// What each export of this file holds, by exported name, when [`Options::imports`] is set.
    /// Re-exports and exported imports are left to the module graph.
    pub exported: HashMap<String, Exported>,
    /// Imports whose contents this file changes in place, such as `ICONS.push(…)`, as indices
    /// into [`FileSummary::imports`], when [`Options::imports`] is set.
    pub changed_imports: Vec<usize>,
    /// Errors the parser recovered from. The lists above may be incomplete when this isn't empty.
    pub errors: Vec<SyntaxError>,
}

/// One imported binding: `import { imported as local } from 'source'`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Import {
    pub source: String,
    pub imported: Imported,
    pub local: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Imported {
    /// A named binding. Default imports are `Name("default")`.
    Name(String),
    /// `import * as local from 'source'`
    Namespace,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Export {
    /// A binding of this file: `export { local as exported }`, `export const exported`,
    /// `export default local`. `local` is `None` for an anonymous default export.
    Local {
        exported: String,
        local: Option<String>,
    },
    /// `export { imported as exported } from 'source'`
    Reexport {
        exported: String,
        source: String,
        imported: String,
    },
    /// `export * as exported from 'source'`
    Namespace { exported: String, source: String },
    /// `export * from 'source'`
    Star { source: String },
}

/// A JSX opening element whose tag refers to a binding, e.g. `<Icon>` or `<L.Icon.Item>`.
/// Intrinsic (`<div>`), namespaced (`<a:b>`) and `this`-rooted tags are left out.
#[derive(Debug, Clone, PartialEq)]
pub struct JsxElement {
    /// Position of the `<`.
    pub location: Location,
    /// The identifier the tag starts with: `L` in `<L.Icon.Item>`.
    pub root: String,
    /// Property accesses after the root: `["Icon", "Item"]` in `<L.Icon.Item>`.
    pub members: Vec<String>,
    pub binding: Binding,
    /// The closest enclosing element of [`FileSummary::elements`].
    pub parent: Option<Parent>,
    pub attributes: Vec<Attribute>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parent {
    pub location: Location,
    /// The parent's attribute this element is in, such as `button` in
    /// `<Popover button={<Button />}>`. `None` for children.
    pub attribute: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Attribute {
    /// `name="value"`, `name={value}`, or `name` alone, which is `true`.
    Named { name: String, value: Expr },
    /// `{...value}`
    Spread(Expr),
}

/// An expression as written, with what's needed to evaluate it.
#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub location: Location,
    pub kind: ExprKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    Literal(Literal),
    /// `undefined`, `void 0`, or an empty `{}`.
    Undefined,
    Object(Vec<Entry>),
    Array(Vec<Item>),
    /// A template literal with expressions, or any tagged template. `tag` is its source text.
    Template {
        tag: Option<String>,
        parts: Vec<Part>,
    },
    /// A JSX element, with the tag as written, or a fragment.
    Element {
        tag: Option<String>,
    },
    Function,
    /// Either branch of `?:`, either side of `||`, `&&` and `??`.
    OneOf(Vec<Expr>),
    /// An identifier, or a member access starting with one, such as `item.icon` or `ICONS[key]`.
    Reference {
        name: Reference,
        text: String,
        /// Property accesses after the name: `[Static("icon")]` in `item.icon`.
        path: Vec<Key>,
    },
    /// `require('specifier')`, with `require` not declared in the file, or a member access on
    /// it.
    Require {
        specifier: String,
        text: String,
        path: Vec<Key>,
    },
    /// A call, or a member access on one.
    Call {
        text: String,
    },
    Unsupported {
        text: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    String(String),
    Number(f64),
    Boolean(bool),
    Null,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Entry {
    Property {
        key: String,
        value: Expr,
    },
    /// A key that isn't static, with its source text.
    Computed {
        key: String,
        value: Expr,
    },
    Spread(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Value(Expr),
    Spread(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Part {
    Text(String),
    Value(Expr),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Key {
    /// A property name, or an array index as a string: `a.b`, `a['b']`, `a[0]`.
    Static(String),
    /// Any other key, such as `a[key]`.
    Dynamic,
}

/// One step into a destructured value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Segment {
    /// A static key of an object pattern.
    Key(String),
    /// A position in an array pattern.
    Index(usize),
}

/// What a name declared in the file holds, when [`Options::variables`] is set.
#[derive(Debug, Clone, PartialEq)]
pub enum Definition {
    /// `const name = init`, or a `let` or `var` that is never assigned to again. `path` is the
    /// destructuring that leads to the name, as in `const { a: [name] } = init`, and `default`
    /// its default, as in `const { name = default } = init`.
    Value {
        init: Expr,
        path: Vec<Segment>,
        default: Option<Expr>,
    },
    /// A function declaration, at its start.
    Function(Location),
    /// Assigned to after its declaration, declared more than once, or changed in place.
    Mutated,
    /// Declared without a value: `declare const name: T`.
    Declared,
    /// Not followed: following is off, or the declaration isn't supported.
    NotFollowed,
}

/// What an export of the file holds.
#[derive(Debug, Clone, PartialEq)]
pub enum Exported {
    /// A name declared in the file. Index into [`FileSummary::definitions`].
    Variable(usize),
    /// `export default` followed by an expression or an anonymous declaration.
    Value(Expr),
}

/// What the name a [`ExprKind::Reference`] starts with is bound to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reference {
    /// Index into [`FileSummary::imports`].
    Import(usize),
    /// A function or `catch` parameter, including a name destructured from one.
    Parameter,
    /// Any other declaration in this file. Index into [`FileSummary::definitions`].
    Variable(usize),
    /// Not declared in this file.
    Global,
}

/// What the root identifier of a JSX tag refers to in its scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Binding {
    /// Index into [`FileSummary::imports`].
    Import(usize),
    /// Index into [`FileSummary::aliases`]: a local constant that refers to an import or a
    /// `require` call.
    Alias(usize),
    /// Declared in this file, including a declaration that shadows an import.
    Local,
    /// Not declared anywhere in this file.
    Global,
}

/// A local constant initialised from an import or `require`, followed through other constants:
/// `const I = Lib.Icon`, `const { Icon } = Lib`, `const { Icon } = require('lib')`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Alias {
    pub root: AliasRoot,
    /// Property accesses after the root: `["Icon"]` in each example above.
    pub members: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AliasRoot {
    /// Index into [`FileSummary::imports`].
    Import(usize),
    /// The specifier of `require('specifier')`.
    Require(String),
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("syntax error at {location}: {message}")]
pub struct SyntaxError {
    pub location: Location,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ParseError {
    #[error("unsupported file extension")]
    UnsupportedExtension,
    /// The parser couldn't recover, so nothing was extracted.
    #[error(transparent)]
    Syntax(SyntaxError),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Options {
    /// Record what the names in attributes are declared as; otherwise every
    /// [`Definition`] is [`Definition::NotFollowed`].
    pub variables: bool,
    /// Also record what each export holds and which imports are changed in place. Implies
    /// `variables`.
    pub imports: bool,
}

impl FileSummary {
    /// Imports that the recorded values read: in exports, in definitions those lead to, and with
    /// `attributes`, in JSX attributes. Changed imports too. Sorted indices into
    /// [`FileSummary::imports`].
    #[must_use]
    pub fn value_imports(&self, attributes: bool) -> Vec<usize> {
        reads::value_imports(self, attributes)
    }
}

/// Parses `source` as the dialect implied by `path`'s extension, with default [`Options`].
pub fn parse(path: &Path, source: &str) -> Result<FileSummary, ParseError> {
    parse_with(path, source, Options::default())
}

pub fn parse_with(path: &Path, source: &str, options: Options) -> Result<FileSummary, ParseError> {
    extract::extract(path, source, options)
}

#[cfg(test)]
mod tests;
