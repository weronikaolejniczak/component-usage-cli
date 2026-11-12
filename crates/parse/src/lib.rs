//! Parses one source file and extracts an owned `FileSummary`: imports, exports and JSX elements.
//!
//! Pure: takes a path and source text, does no I/O. The Oxc AST and its arena never leave this crate.
//! The path only selects the dialect from its extension. JavaScript files always allow JSX;
//! `.ts` files never do, since `<T>` there is a type assertion or generic.
//! Type-only imports and exports are left out: they can't be rendered.

mod extract;
mod lines;

use std::path::Path;

use model::Location;

/// What later stages need from one file. Every list is in source order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileSummary {
    pub imports: Vec<Import>,
    pub exports: Vec<Export>,
    pub elements: Vec<JsxElement>,
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsxElement {
    /// Position of the `<`.
    pub location: Location,
    /// The identifier the tag starts with: `L` in `<L.Icon.Item>`.
    pub root: String,
    /// Property accesses after the root: `["Icon", "Item"]` in `<L.Icon.Item>`.
    pub members: Vec<String>,
    pub binding: Binding,
}

/// What the root identifier of a JSX tag refers to in its scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Binding {
    /// Index into [`FileSummary::imports`].
    Import(usize),
    /// Declared in this file, including a declaration that shadows an import.
    Local,
    /// Not declared anywhere in this file.
    Global,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ParseError {
    #[error("unsupported file extension")]
    UnsupportedExtension,
    #[error("syntax error at {location}: {message}")]
    Syntax { location: Location, message: String },
}

/// Parses `source` as the dialect implied by `path`'s extension.
///
/// Fails on the first syntax error, even if the parser could recover, so a partial AST
/// never produces partial results.
pub fn parse(path: &Path, source: &str) -> Result<FileSummary, ParseError> {
    extract::extract(path, source)
}

#[cfg(test)]
mod tests;
