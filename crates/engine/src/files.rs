use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

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

    /// [`Files::display`] of every file, by [`FileId`].
    pub(crate) fn displays(&self, root: &Path) -> Vec<String> {
        self.paths.iter().map(|path| display(path, root)).collect()
    }
}

/// Relative to `root` with `/` separators, `.` for `root` itself, with a `..` for each folder up
/// from `root` when outside it, or absolute when no ancestor of `root` contains it, as with
/// another drive.
pub(crate) fn display(path: &Path, root: &Path) -> String {
    for (up, base) in root.ancestors().enumerate() {
        if let Ok(relative) = path.strip_prefix(base) {
            if up == 0 && relative.as_os_str().is_empty() {
                return ".".to_owned();
            }
            let relative = slashed(relative);
            return std::iter::repeat_n("..", up)
                .chain((!relative.is_empty()).then_some(relative.as_str()))
                .collect::<Vec<_>>()
                .join("/");
        }
    }
    path.to_string_lossy().into_owned()
}

pub(crate) fn slashed(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

/// Source files under `root`, sorted. Walk failures are added to `errors`, sorted.
pub(crate) fn discover(root: &Path, errors: &mut Vec<FileError>) -> Vec<PathBuf> {
    let found = Mutex::new((Vec::new(), Vec::new()));
    ignore::WalkBuilder::new(root)
        .hidden(false)
        .filter_entry(|entry| entry.file_name() != "node_modules" && entry.file_name() != ".git")
        .build_parallel()
        .run(|| {
            Box::new(|entry| {
                let mut found = found.lock().unwrap_or_else(PoisonError::into_inner);
                match entry {
                    Ok(entry) => {
                        let path = entry.path();
                        if entry.file_type().is_some_and(|kind| kind.is_file()) && is_source(path) {
                            found.0.push(path.to_path_buf());
                        }
                    }
                    Err(error) => found.1.push(FileError {
                        file: display(error_path(&error).unwrap_or(root), root),
                        location: None,
                        code: ErrorCode::Read,
                        message: error.to_string(),
                        skipped: true,
                        owners: None,
                    }),
                }
                ignore::WalkState::Continue
            })
        });
    let (mut paths, mut walk_errors) = found.into_inner().unwrap_or_else(PoisonError::into_inner);
    paths.sort();
    walk_errors.sort();
    errors.extend(walk_errors);
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

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::display;

    #[test]
    fn displays_paths_relative_to_the_root() {
        let root = Path::new("/repo/app");
        assert_eq!(display(Path::new("/repo/app"), root), ".");
        assert_eq!(display(Path::new("/repo/app/src/a.tsx"), root), "src/a.tsx");
        assert_eq!(display(Path::new("/repo"), root), "..");
        assert_eq!(
            display(Path::new("/repo/shared/b.tsx"), root),
            "../shared/b.tsx"
        );
        assert_eq!(
            display(Path::new("/other/c.tsx"), root),
            "../../other/c.tsx"
        );
    }
}
