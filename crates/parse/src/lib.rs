//! Parses one source file and extracts an owned `FileSummary`: imports, exports and JSX elements.
//!
//! Pure: takes a path and source text, does no I/O. The Oxc AST and its arena never leave this crate.
