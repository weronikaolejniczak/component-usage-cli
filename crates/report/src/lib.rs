//! Writes a scan report in a given format to a given writer.
//!
//! Output is deterministic: sorted by file path, then position.

use std::io::{self, Write};

use model::Location;
use serde::Serialize;

/// Paths are relative to the scan root, with `/` separators.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct Report {
    pub usages: Vec<Usage>,
    /// JSX elements that may or may not be usages, because their origin couldn't be determined.
    pub unidentified: Vec<Unidentified>,
    /// Scanned files that couldn't be read or parsed, fully or partly.
    pub errors: Vec<FileError>,
    /// Problems in inputs other than source files, such as skipped `CODEOWNERS` lines.
    pub warnings: Vec<Warning>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct Usage {
    pub file: String,
    #[serde(flatten)]
    pub location: Location,
    pub component: String,
    /// Code owners of `file`; empty if unowned. Absent unless owners were requested.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owners: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct Unidentified {
    pub file: String,
    #[serde(flatten)]
    pub location: Location,
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
    pub message: String,
}

impl Report {
    /// Sorts every list by file path, then position.
    pub fn sort(&mut self) {
        self.usages.sort();
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
    use model::Location;

    use super::{FileError, Format, Report, Usage, write};

    #[test]
    fn json() {
        let report = Report {
            usages: vec![Usage {
                file: "src/App.tsx".to_owned(),
                location: Location { line: 3, column: 5 },
                component: "Icon".to_owned(),
                owners: Some(vec!["@org/team".to_owned()]),
            }],
            unidentified: vec![],
            errors: vec![FileError {
                file: "src/Broken.tsx".to_owned(),
                location: None,
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
                "usages": [{ "file": "src/App.tsx", "line": 3, "column": 5, "component": "Icon", "owners": ["@org/team"] }],
                "unidentified": [],
                "errors": [{ "file": "src/Broken.tsx", "message": "unsupported file extension", "skipped": true }],
                "warnings": [],
            })
        );
    }
}
