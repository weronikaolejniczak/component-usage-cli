//! Writes a scan report in a given format to a given writer.
//!
//! Output is deterministic: sorted by file path, then position.

use std::io::{self, Write};

use model::Location;
pub use model::{Parent, Passed, Prop};
use serde::Serialize;

/// Paths are relative to the scan root, with `/` separators.
#[derive(Debug, Default, Clone, PartialEq, Serialize)]
pub struct Report {
    pub usages: Vec<Usage>,
    /// Places the components are read as values rather than rendered, such as
    /// `<Panel icon={Icon} />`.
    pub references: Vec<Reference>,
    /// JSX elements that may or may not be usages, because their origin couldn't be determined.
    pub unidentified: Vec<Unidentified>,
    /// Scanned files that couldn't be read or parsed, fully or partly.
    pub errors: Vec<FileError>,
    /// Problems in inputs other than source files, such as skipped `CODEOWNERS` lines.
    pub warnings: Vec<Warning>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Usage {
    pub file: String,
    #[serde(flatten)]
    pub location: Location,
    pub component: String,
    /// The wrappers from the element to the component, outermost first. Absent for an element
    /// of the component itself.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub through: Vec<Through>,
    /// Enclosing usages, nearest first.
    pub parents: Vec<Parent>,
    /// Attributes in source order, without those a wrapper doesn't pass on.
    pub props: Vec<Prop>,
    /// Code owners of `file`; empty if unowned. Absent unless owners were requested.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owners: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Reference {
    pub file: String,
    #[serde(flatten)]
    pub location: Location,
    pub component: String,
    /// Where the value goes.
    #[serde(rename = "in")]
    pub passed: Passed,
    /// Code owners of `file`; empty if unowned. Absent unless owners were requested.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owners: Option<Vec<String>>,
}

/// A wrapper a usage goes through, with the position where it passes its props on: the element
/// it spreads them into, or the argument of the call it holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Through {
    pub component: String,
    pub file: String,
    #[serde(flatten)]
    pub location: Location,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct Unidentified {
    pub file: String,
    #[serde(flatten)]
    pub location: Location,
    /// The tag as written: `L.Icon` in `<L.Icon>`.
    pub tag: String,
    /// The specifier of the import or `require` the tag refers to.
    pub source: String,
    pub code: UnidentifiedCode,
    /// The target component the element refers to, when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub component: Option<String>,
    pub reason: String,
    /// Code owners of `file`; empty if unowned. Absent unless owners were requested.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owners: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct FileError {
    pub file: String,
    #[serde(flatten)]
    pub location: Option<Location>,
    pub code: ErrorCode,
    pub message: String,
    /// `true` if nothing in the file was scanned; `false` if results from it may be incomplete.
    pub skipped: bool,
    /// Code owners of `file`; empty if unowned. Absent unless owners were requested.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owners: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct Warning {
    pub file: String,
    pub line: u32,
    pub code: WarningCode,
    pub message: String,
}

/// Why a `CODEOWNERS` line was skipped. Stable across releases; `message` is for people.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum WarningCode {
    UnsupportedNegation,
    UnsupportedRange,
    UnsupportedEscape,
    InvalidOwner,
}

/// Why an element is unidentified. Stable across releases; `reason` is for people.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum UnidentifiedCode {
    /// A specifier didn't resolve to a file.
    Unresolved,
    /// A module on the way couldn't be parsed.
    Unparsed,
    /// A module on the way doesn't export the name.
    MissingExport,
    /// A module on the way has no ES exports, likely CommonJS, whose exports aren't read.
    #[serde(rename = "commonjs-module")]
    CommonJsModule,
    /// More than one `export *` provides the name.
    Ambiguous,
    /// Re-exports form a cycle.
    Cycle,
    /// Refers to a target through a local constant, which isn't followed.
    Alias,
    /// Refers to a target through `require`; only ES modules are supported.
    #[serde(rename = "commonjs")]
    CommonJs,
    /// A bug in this tool.
    Internal,
}

/// Why a file couldn't be scanned, fully or partly. Stable across releases; `message` is for
/// people.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ErrorCode {
    /// The file or folder couldn't be read.
    Read,
    UnsupportedExtension,
    Syntax,
}

impl Report {
    /// Sorts every list by file path, then position.
    pub fn sort(&mut self) {
        self.usages
            .sort_by(|a, b| (&a.file, a.location).cmp(&(&b.file, b.location)));
        self.references
            .sort_by(|a, b| (&a.file, a.location).cmp(&(&b.file, b.location)));
        self.unidentified.sort();
        self.errors.sort();
        self.warnings.sort();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Json,
}

pub fn write(report: &Report, format: Format, mut writer: impl Write) -> io::Result<()> {
    match format {
        Format::Json => {
            serde_json::to_writer_pretty(&mut writer, report)?;
            writeln!(writer)
        }
    }
}

#[cfg(test)]
mod tests {
    use model::{Literal, Location, Step, UnresolvedCode, Value, ValueKind};

    use super::{ErrorCode, FileError, Format, Parent, Prop, Report, Usage, write};

    #[test]
    fn json() {
        let report = Report {
            usages: vec![Usage {
                file: "src/App.tsx".to_owned(),
                location: Location { line: 3, column: 5 },
                component: "Icon".to_owned(),
                through: Vec::new(),
                parents: vec![Parent {
                    component: "Popover".to_owned(),
                    location: Location { line: 2, column: 3 },
                    prop: Some("button".to_owned()),
                }],
                props: vec![
                    Prop::Named {
                        name: "size".to_owned(),
                        value: Value {
                            file: None,
                            kind: ValueKind::Literal {
                                value: Literal::Integer(16),
                            },
                            location: Location {
                                line: 3,
                                column: 17,
                            },
                            via: Vec::new(),
                        },
                    },
                    Prop::Spread {
                        spread: Value {
                            file: None,
                            kind: ValueKind::Unresolved {
                                code: UnresolvedCode::NotFollowed,
                                step: Some(Step::Variables),
                                text: "rest".to_owned(),
                            },
                            location: Location {
                                line: 3,
                                column: 25,
                            },
                            via: Vec::new(),
                        },
                    },
                ],
                owners: Some(vec!["@org/team".to_owned()]),
            }],
            references: vec![],
            unidentified: vec![],
            errors: vec![FileError {
                file: "src/Broken.tsx".to_owned(),
                location: None,
                code: ErrorCode::UnsupportedExtension,
                message: "unsupported file extension".to_owned(),
                skipped: true,
                owners: None,
            }],
            warnings: vec![],
        };
        let mut out = Vec::new();
        write(&report, Format::Json, &mut out).unwrap();
        let json: serde_json::Value = serde_json::from_slice(&out).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "usages": [{
                    "file": "src/App.tsx", "line": 3, "column": 5, "component": "Icon",
                    "parents": [{ "component": "Popover", "line": 2, "column": 3, "prop": "button" }],
                    "props": [
                        { "name": "size", "value": { "kind": "literal", "value": 16, "line": 3, "column": 17 } },
                        { "spread": { "kind": "unresolved", "code": "not-followed", "step": "variables", "text": "rest", "line": 3, "column": 25 } },
                    ],
                    "owners": ["@org/team"],
                }],
                "references": [],
                "unidentified": [],
                "errors": [{ "file": "src/Broken.tsx", "code": "unsupported-extension", "message": "unsupported file extension", "skipped": true }],
                "warnings": [],
            })
        );
    }
}
