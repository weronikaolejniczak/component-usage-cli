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
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Usage {
    pub file: FileId,
    pub location: Location,
    /// Exported name, with member access for compound components, e.g. `Icon.Item`.
    pub component: String,
}
