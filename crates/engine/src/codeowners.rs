use std::fs;
use std::path::{Path, PathBuf};

use owners::CodeOwners;
use report::{Report, Warning};

use crate::{Owners, ScanError};

/// GitHub's search order, relative to the repository root.
const LOCATIONS: [&str; 3] = [".github/CODEOWNERS", "CODEOWNERS", "docs/CODEOWNERS"];

/// GitHub doesn't load files over this size.
const MAX_BYTES: u64 = 3_000_000;

pub(crate) struct Loaded {
    owners: CodeOwners,
    repository: PathBuf,
}

/// Reads the `CODEOWNERS` file for the repository containing `root`. Skipped lines become
/// warnings in `report`.
pub(crate) fn load(root: &Path, source: &Owners, report: &mut Report) -> Result<Loaded, ScanError> {
    let repository = root
        .ancestors()
        .find(|dir| dir.join(".git").exists())
        .ok_or_else(|| ScanError::NoRepository {
            root: root.to_path_buf(),
        })?
        .to_path_buf();
    let path = match source {
        Owners::File(path) => path.clone(),
        Owners::Discover => LOCATIONS
            .iter()
            .map(|location| repository.join(location))
            .find(|path| path.is_file())
            .ok_or_else(|| ScanError::NoCodeOwners {
                repository: repository.clone(),
            })?,
    };
    let read_error = |source| ScanError::ReadCodeOwners {
        path: path.clone(),
        source,
    };
    if fs::metadata(&path).map_err(read_error)?.len() >= MAX_BYTES {
        return Err(ScanError::CodeOwnersTooLarge { path });
    }
    let text = fs::read_to_string(&path).map_err(read_error)?;

    let (owners, warnings) = CodeOwners::parse(&text);
    let file = display(&path, root);
    report
        .warnings
        .extend(warnings.into_iter().map(|warning| Warning {
            file: file.clone(),
            line: warning.line,
            message: warning.message,
        }));
    Ok(Loaded { owners, repository })
}

impl Loaded {
    /// Sets `owners` on every result. Report paths are relative to `root` or absolute.
    pub(crate) fn attach(&self, root: &Path, report: &mut Report) {
        let owners = |file: &str| self.owners_of(&root.join(file));
        for usage in &mut report.usages {
            usage.owners = Some(owners(&usage.file));
        }
        for unidentified in &mut report.unidentified {
            unidentified.owners = Some(owners(&unidentified.file));
        }
        for error in &mut report.errors {
            error.owners = Some(owners(&error.file));
        }
    }

    fn owners_of(&self, path: &Path) -> Vec<String> {
        match path.strip_prefix(&self.repository) {
            Ok(relative) => self.owners.owners(&slashed(relative)),
            Err(_) => Vec::new(),
        }
    }
}

fn display(path: &Path, root: &Path) -> String {
    match path.strip_prefix(root) {
        Ok(relative) => slashed(relative),
        Err(_) => path.to_string_lossy().into_owned(),
    }
}

fn slashed(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}
