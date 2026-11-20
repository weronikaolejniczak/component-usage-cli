//! Writes a scan report in a given format to a given writer.
//!
//! Output is deterministic: sorted by file path, then position.

mod summary;

use std::io::{self, Write};

use model::Location;
pub use model::{Parent, Passed, Prop};
use rayon::prelude::*;
use serde::Serialize;
pub use summary::{
    ComponentSummary, Options, OwnerCount, PropCount, Summary, UnidentifiedCount, ValueCount,
};

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
    /// The props the component is given, in the order they apply: through wrappers, those each
    /// sets around the caller's, without those a rest pattern takes.
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
    /// Refers to a target through a local name that may hold something else.
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

/// Writes a [`Report`] or a [`Summary`].
pub fn write(report: &impl Serialize, format: Format, mut writer: impl Write) -> io::Result<()> {
    match format {
        Format::Json => {
            serde_json::to_writer_pretty(&mut writer, report)?;
            writeln!(writer)
        }
    }
}

/// [`write`] for a [`Report`], with its entries serialized in parallel, a batch at a time.
pub fn write_report(report: &Report, format: Format, mut writer: impl Write) -> io::Result<()> {
    match format {
        Format::Json => {
            writer.write_all(b"{\n  \"usages\": ")?;
            json_array(&report.usages, &mut writer)?;
            writer.write_all(b",\n  \"references\": ")?;
            json_array(&report.references, &mut writer)?;
            writer.write_all(b",\n  \"unidentified\": ")?;
            json_array(&report.unidentified, &mut writer)?;
            writer.write_all(b",\n  \"errors\": ")?;
            json_array(&report.errors, &mut writer)?;
            writer.write_all(b",\n  \"warnings\": ")?;
            json_array(&report.warnings, &mut writer)?;
            writer.write_all(b"\n}\n")
        }
    }
}

/// Entries serialized together, then written, so a report isn't held twice.
const BATCH: usize = 4096;

/// `items` as a pretty JSON array a field of the top-level object holds.
fn json_array<T: Serialize + Sync>(items: &[T], writer: &mut impl Write) -> io::Result<()> {
    if items.is_empty() {
        return writer.write_all(b"[]");
    }
    writer.write_all(b"[")?;
    for (batch, items) in items.chunks(BATCH).enumerate() {
        let written: Vec<Vec<u8>> = items
            .par_iter()
            .map(|item| {
                let json = serde_json::to_vec_pretty(item)?;
                // Strings escape line breaks, so each one starts a line to indent.
                let mut indented = Vec::with_capacity(json.len() + json.len() / 8);
                indented.extend_from_slice(b"\n    ");
                for &byte in &json {
                    indented.push(byte);
                    if byte == b'\n' {
                        indented.extend_from_slice(b"    ");
                    }
                }
                Ok(indented)
            })
            .collect::<serde_json::Result<_>>()?;
        for (index, item) in written.iter().enumerate() {
            if batch > 0 || index > 0 {
                writer.write_all(b",")?;
            }
            writer.write_all(item)?;
        }
    }
    writer.write_all(b"\n  ]")
}

#[cfg(test)]
mod tests {
    use model::{Literal, Location, Step, UnresolvedCode, Value, ValueKind};

    use super::{
        BATCH, ErrorCode, FileError, Format, Parent, Prop, Report, Usage, write, write_report,
    };

    fn sample() -> Report {
        Report {
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
                        forwards: false,
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
        }
    }

    #[test]
    fn json() {
        let mut out = Vec::new();
        write(&sample(), Format::Json, &mut out).unwrap();
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

    #[test]
    fn report_in_batches_as_written_whole() {
        let mut batches = sample();
        batches.usages = vec![batches.usages[0].clone(); BATCH + 1];
        for report in [sample(), Report::default(), batches] {
            let (mut whole, mut batched) = (Vec::new(), Vec::new());
            write(&report, Format::Json, &mut whole).unwrap();
            write_report(&report, Format::Json, &mut batched).unwrap();
            assert_eq!(String::from_utf8(batched), String::from_utf8(whole));
        }
    }
}
