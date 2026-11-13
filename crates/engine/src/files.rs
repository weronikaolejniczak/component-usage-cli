use std::collections::HashMap;
use std::path::{Path, PathBuf};

use model::FileId;
use report::FileError;

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

    /// Relative to `root` with `/` separators, or absolute if outside it.
    pub(crate) fn display(&self, id: FileId, root: &Path) -> String {
        let path = self.path(id);
        match path.strip_prefix(root) {
            Ok(relative) => relative
                .components()
                .map(|component| component.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/"),
            Err(_) => path.to_string_lossy().into_owned(),
        }
    }
}

/// Source files under `root`, sorted. Walk failures are added to `errors`.
pub(crate) fn discover(root: &Path, errors: &mut Vec<FileError>) -> Vec<PathBuf> {
    let walker = ignore::WalkBuilder::new(root)
        .filter_entry(|entry| entry.file_name() != "node_modules")
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
                file: root.to_string_lossy().into_owned(),
                location: None,
                message: error.to_string(),
                skipped: true,
                owners: None,
            }),
        }
    }
    paths.sort();
    paths
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
