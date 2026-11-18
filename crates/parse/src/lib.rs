//! Parses one source file and extracts an owned `FileSummary`: imports, exports and JSX elements
//! with their attributes.
//!
//! Pure: takes a path and source text, does no I/O. The Oxc AST and its arena never leave this crate.
//! The path only selects the dialect from its extension. JavaScript files always allow JSX;
//! `.ts` files never do, since `<T>` there is a type assertion or generic.
//! Type-only imports and exports are left out: they can't be rendered.

mod aliases;
mod attributes;
mod components;
mod definitions;
mod elements;
mod exported;
mod extract;
mod lines;
mod patterns;
mod reads;
mod references;
mod scope;

use std::collections::HashMap;
use std::path::Path;

use model::{Location, Passed};

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
    /// The components declared in the file, when [`Options::props`] or [`Options::wrappers`] is
    /// set.
    pub components: Vec<Component>,
    /// The names declared by the props of [`FileSummary::components`], indexed by
    /// [`Reference::Parameter`].
    pub parameters: Vec<Parameter>,
    /// Imports used other than as a JSX tag or an export, as indices into
    /// [`FileSummary::imports`], when [`Options::props`] is set.
    pub escaping_imports: Vec<usize>,
    /// Imports read as values, other than as JSX tags, exports, types and the objects of
    /// property accesses that aren't followed.
    pub references: Vec<ValueReference>,
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
    /// The position of the first child other than whitespace, if any.
    pub children: Option<Location>,
    /// The component of [`FileSummary::components`] the tag refers to by its local name, when
    /// [`Options::props`] or [`Options::wrappers`] is set.
    pub component: Option<usize>,
}

/// An import read as a value: `L.Icon` in `<Panel icon={L.Icon} />`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueReference {
    /// Index into [`FileSummary::imports`].
    pub import: usize,
    /// Property accesses after the import: `["Icon"]` in `L.Icon`.
    pub members: Vec<String>,
    /// The import's name.
    pub location: Location,
    pub passed: Passed,
}

/// A function that can be rendered as a JSX tag: declared with a name, or assigned to a `const`,
/// optionally inside `memo(…)` or `forwardRef(…)`, or the default export.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Component {
    /// The name it's declared as, or `default` for an anonymous default export.
    pub name: String,
    /// The position of its name, or of the function when it has none.
    pub location: Location,
    /// The names it's declared as, including `const Wrapper = memo(Inner)`, as indices into
    /// [`FileSummary::definitions`].
    pub variables: Vec<usize>,
    /// Whether it's the default export without being one of [`Component::variables`].
    pub default: bool,
    /// Whether one of its names is used other than as a JSX tag, an export or the argument of
    /// `memo` or `forwardRef`.
    pub escapes: bool,
    /// With [`Options::wrappers`], where it passes all its props on, in source order.
    pub forwards: Vec<Forward>,
}

/// Where a component passes all its props on: an element given a spread of its props or of a
/// rest element of them, or the first argument of the call a `const` or the default export holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Forward {
    /// The spreading element's `<`, or the argument.
    pub location: Location,
    pub to: Forwarded,
    /// The keys the rest pattern takes, which aren't passed on.
    pub excluded: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Forwarded {
    /// Indices into [`FileSummary::elements`] and the element's [`JsxElement::attributes`], of
    /// the spread.
    Element { element: usize, attribute: usize },
    /// An import, with the property accesses after it: `L.Button` in `wrap(L.Button)`.
    Import { import: usize, members: Vec<String> },
    /// Index into [`FileSummary::components`].
    Component(usize),
}

/// A name declared by a component's props, its first parameter: `props`, or `iconType` and
/// `rest` in `{ iconType = 'gear', ...rest }`.
#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    /// Index into [`FileSummary::components`].
    pub component: usize,
    /// Where the name is declared.
    pub location: Location,
    /// The steps from the props to the name.
    pub path: Vec<Segment>,
    /// The name's own default: `'gear'` in `{ iconType = 'gear' }`.
    pub default: Option<Expr>,
    /// For a rest element, the keys the pattern takes before it.
    pub rest: Option<Vec<String>>,
    /// Whether the name is assigned, or changed in place, in the component.
    pub mutated: bool,
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
    /// A function. With [`Options::calls`], for a function written as a call's argument, `returns`
    /// are the values of its `return` statements, outside nested functions, or its body without
    /// braces; `None` for async functions and generators.
    Function {
        returns: Option<Vec<Expr>>,
    },
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
    /// A call, or a member access on one. With [`Options::calls`], `call` describes it.
    Call {
        text: String,
        call: Option<Box<Call>>,
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

/// A call: what's called, on what, with what, and which properties are read from its result.
#[derive(Debug, Clone, PartialEq)]
pub struct Call {
    /// The source text of what's called.
    pub callee: String,
    /// The object of a member call, as `items` in `items.map(…)`.
    pub receiver: Option<Expr>,
    pub arguments: Vec<Item>,
    /// Properties read from the result, as in `useStyles().panel`.
    pub path: Vec<Key>,
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
    /// A function or `catch` parameter, including a name destructured from one. With
    /// [`Options::props`], the index into [`FileSummary::parameters`] when it's a component's prop.
    Parameter(Option<usize>),
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
    /// Index into [`FileSummary::aliases`]: a local name that refers to an import or a
    /// `require` call.
    Alias(usize),
    /// Declared in this file, including a declaration that shadows an import.
    Local,
    /// Not declared anywhere in this file.
    Global,
}

/// A local name initialised from an import or `require`, followed through other names:
/// `const I = Lib.Icon`, `const { Icon } = Lib`, `const { Icon } = require('lib')`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Alias {
    pub root: AliasRoot,
    /// Property accesses after the root: `["Icon"]` in each example above.
    pub members: Vec<String>,
    /// The position of the root in the declaration that names it: `Lib` in `const I = Lib.Icon`.
    pub location: Location,
    /// Whether a name on the way is assigned after its declaration, declared more than once, or
    /// changed in place, so it may hold something else.
    pub reassigned: bool,
}

impl Alias {
    /// The index into [`FileSummary::imports`] of the import it holds, unless it's `require` or
    /// may hold something else.
    #[must_use]
    pub fn import(&self) -> Option<usize> {
        match self.root {
            AliasRoot::Import(import) if !self.reassigned => Some(import),
            AliasRoot::Import(_) | AliasRoot::Require(_) => None,
        }
    }
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
    /// Also record components, their props, the elements that render them and the imports used
    /// other than as a JSX tag. Implies `imports`.
    pub props: bool,
    /// Describe calls, with [`Call`], and what functions return.
    pub calls: bool,
    /// Also record components, the elements that render them, what each export holds, and
    /// [`Component::forwards`].
    pub wrappers: bool,
}

impl FileSummary {
    /// Imports that the recorded values read: in exports, in definitions those lead to, in the
    /// defaults of props, and with `attributes`, in JSX attributes. Changed imports, and imports
    /// used other than as a JSX tag, too. Sorted indices into
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
