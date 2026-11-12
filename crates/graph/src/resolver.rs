use std::path::{Path, PathBuf};

use oxc_resolver::{ResolveOptions, TsconfigDiscovery};

/// Resolves import specifiers the way bundlers do for browser code: `tsconfig.json` `paths` and
/// `baseUrl`, `package.json` `exports` (ESM conditions first), `module`, then `main`.
pub struct Resolver(oxc_resolver::Resolver);

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct ResolveError(String);

impl ResolveError {
    pub(crate) fn not_requested() -> Self {
        Self("specifier wasn't resolved before building the graph".to_owned())
    }
}

impl Resolver {
    #[must_use]
    pub fn new() -> Self {
        Self(oxc_resolver::Resolver::new(ResolveOptions {
            tsconfig: Some(TsconfigDiscovery::Auto),
            condition_names: ["import", "module", "browser", "default", "require"]
                .map(String::from)
                .to_vec(),
            extensions: [".tsx", ".ts", ".mts", ".cts", ".jsx", ".js", ".mjs", ".cjs"]
                .map(String::from)
                .to_vec(),
            extension_alias: vec![
                (
                    ".js".into(),
                    [".ts", ".tsx", ".js", ".jsx"].map(String::from).to_vec(),
                ),
                (".mjs".into(), [".mts", ".mjs"].map(String::from).to_vec()),
                (".cjs".into(), [".cts", ".cjs"].map(String::from).to_vec()),
            ],
            main_fields: ["module", "main"].map(String::from).to_vec(),
            builtin_modules: true,
            ..ResolveOptions::default()
        }))
    }

    /// Resolves `specifier` as imported from the file at `importer`, which must be absolute.
    pub fn resolve(&self, importer: &Path, specifier: &str) -> Result<PathBuf, ResolveError> {
        self.0
            .resolve_file(importer, specifier)
            .map(oxc_resolver::Resolution::into_path_buf)
            .map_err(|error| ResolveError(error.to_string()))
    }
}

impl Default for Resolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::Resolver;

    fn fixtures() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/identify")
            .canonicalize()
            .unwrap()
    }

    #[test]
    fn package_main() {
        let root = fixtures();
        let resolved = Resolver::new()
            .resolve(&root.join("named-import/App.tsx"), "lib")
            .unwrap();
        assert_eq!(resolved, root.join("node_modules/lib/index.js"));
    }

    #[test]
    fn deep_import_to_directory_index() {
        let root = fixtures();
        let resolved = Resolver::new()
            .resolve(&root.join("deep-import/App.tsx"), "lib/components/icon")
            .unwrap();
        assert_eq!(
            resolved,
            root.join("node_modules/lib/components/icon/index.js")
        );
    }

    #[test]
    fn relative_without_extension() {
        let root = fixtures();
        let resolved = Resolver::new()
            .resolve(&root.join("barrel-reexport/Named.tsx"), "./ui")
            .unwrap();
        assert!(resolved.starts_with(root.join("barrel-reexport/ui")));
    }

    #[test]
    fn missing_package() {
        let root = fixtures();
        assert!(
            Resolver::new()
                .resolve(&root.join("named-import/App.tsx"), "does-not-exist")
                .is_err()
        );
    }
}
