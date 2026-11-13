use std::collections::HashMap;
use std::path::{Path, PathBuf};

use model::FileId;
use report::{ErrorCode, FileError};

const EXTENSIONS: [&str; 8] = ["tsx", "ts", "jsx", "js", "mjs", "cjs", "mts", "cts"];

/// Every file the scan touches, indexed by [`FileId`].
#[derive(Default)]
pub(crate) struct Files {
    paths: Vec<PathBuf>,
    ids: HashMap<PathBuf, FileId>,
}

impl Files {
    /// Returns the file's ID and whether it was newly added.
    pub(crate) fn intern(&mut self, path: PathBuf) -> (FileId, bool) {
        if let Some(&id) = self.ids.get(&path) {
            return (id, false);
        }
        let id = FileId(u32::try_from(self.paths.len()).expect("more than u32::MAX files"));
        self.paths.push(path.clone());
        self.ids.insert(path, id);
        (id, true)
    }

    pub(crate) fn path(&self, id: FileId) -> &Path {
        &self.paths[id.0 as usize]
    }

    pub(crate) fn len(&self) -> usize {
        self.paths.len()
    }

    pub(crate) fn display(&self, id: FileId, root: &Path) -> String {
        display(self.path(id), root)
    }
}

/// Relative to `root` with `/` separators, `.` for `root` itself, or absolute if outside it.
pub(crate) fn display(path: &Path, root: &Path) -> String {
    match path.strip_prefix(root) {
        Ok(relative) if relative.as_os_str().is_empty() => ".".to_owned(),
        Ok(relative) => slashed(relative),
        Err(_) => path.to_string_lossy().into_owned(),
    }
}

pub(crate) fn slashed(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

/// Source files under `root`, sorted. Walk failures are added to `errors`.
pub(crate) fn discover(root: &Path, errors: &mut Vec<FileError>) -> Vec<PathBuf> {
    let walker = ignore::WalkBuilder::new(root)
        .hidden(false)
        .filter_entry(|entry| entry.file_name() != "node_modules" && entry.file_name() != ".git")
        .build();
    let mut paths = Vec::new();
    for entry in walker {
        match entry {
            Ok(entry) => {
                let path = entry.path();
                if entry.file_type().is_some_and(|kind| kind.is_file()) && is_source(path) {
                    paths.push(path.to_path_buf());
                }
            }
            Err(error) => errors.push(FileError {
                file: display(error_path(&error).unwrap_or(root), root),
                location: None,
                code: ErrorCode::Read,
                message: error.to_string(),
                skipped: true,
                owners: None,
            }),
        }
    }
    paths.sort();
    paths
}

fn error_path(error: &ignore::Error) -> Option<&Path> {
    match error {
        ignore::Error::WithPath { path, .. } => Some(path),
        ignore::Error::Loop { child, .. } => Some(child),
        ignore::Error::WithDepth { err, .. } | ignore::Error::WithLineNumber { err, .. } => {
            error_path(err)
        }
        ignore::Error::Partial(errors) => errors.iter().find_map(error_path),
        _ => None,
    }
}

fn is_source(path: &Path) -> bool {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_default();
    let declaration = [".d.ts", ".d.mts", ".d.cts"]
        .iter()
        .any(|suffix| name.ends_with(suffix));
    !declaration
        && path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| EXTENSIONS.contains(&extension))
}
