//! Scans that can't run: they exit with a failure and say why on stderr.

use std::fs;
use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

/// A folder with `lib` in `node_modules`, whose `index.js` is `index`, and the given files.
fn project(index: &str, files: &[(&str, &str)]) -> TempDir {
    let temp = TempDir::new().unwrap();
    let lib = temp.path().join("node_modules/lib");
    fs::create_dir_all(&lib).unwrap();
    fs::write(
        lib.join("package.json"),
        r#"{ "name": "lib", "main": "index.js" }"#,
    )
    .unwrap();
    fs::write(lib.join("index.js"), index).unwrap();
    for (path, contents) in files {
        let path = temp.path().join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }
    temp
}

/// Runs a scan of `root` with `args`, expecting it to fail, and returns its stderr.
fn failure(root: &Path, args: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_component-usage-cli"))
        .arg("scan")
        .arg(root)
        .args(args)
        .output()
        .unwrap();
    assert!(!output.status.success(), "the scan succeeded");
    assert!(output.stdout.is_empty(), "the scan wrote a report");
    String::from_utf8(output.stderr).unwrap()
}

fn assert_error(stderr: &str, expected: &str) {
    assert!(
        stderr.starts_with("error: ") && stderr.contains(expected),
        "{stderr:?} doesn't say {expected:?}"
    );
}

#[test]
fn missing_root() {
    let temp = TempDir::new().unwrap();
    let stderr = failure(&temp.path().join("missing"), &["--from", "lib"]);
    assert_error(&stderr, "can't open ");
}

#[test]
fn root_is_a_file() {
    let temp = project("", &[("App.tsx", "")]);
    let stderr = failure(&temp.path().join("App.tsx"), &["--from", "lib"]);
    assert_error(&stderr, "App.tsx is not a directory");
}

#[test]
fn unresolved_from() {
    let temp = project("", &[]);
    let stderr = failure(temp.path(), &["--from", "missing-lib"]);
    assert_error(&stderr, "can't resolve 'missing-lib' from ");
}

#[test]
fn missing_component() {
    let temp = project("export const Icon = () => null;", &[]);
    let stderr = failure(temp.path(), &["--from", "lib", "--components", "Button"]);
    assert_error(
        &stderr,
        "can't find component 'Button': node_modules/lib/index.js has no export 'Button'",
    );
}

#[test]
fn ambiguous_component() {
    let temp = project(
        "export * from './a';\nexport * from './b';",
        &[
            ("node_modules/lib/a.js", "export const Icon = 1;"),
            ("node_modules/lib/b.js", "export const Icon = 2;"),
        ],
    );
    let stderr = failure(temp.path(), &["--from", "lib", "--components", "Icon"]);
    assert_error(
        &stderr,
        "can't find component 'Icon': 'Icon' is exported by more than one `export *` in \
         node_modules/lib/index.js",
    );
}

#[test]
fn component_in_a_cycle() {
    let temp = project(
        "export { Icon } from './a';",
        &[("node_modules/lib/a.js", "export { Icon } from './index';")],
    );
    let stderr = failure(temp.path(), &["--from", "lib", "--components", "Icon"]);
    assert_error(
        &stderr,
        "can't find component 'Icon': re-exports of 'Icon' form a cycle through ",
    );
}

#[test]
fn component_of_a_commonjs_module() {
    let temp = project("module.exports = { Icon: () => null };", &[]);
    let stderr = failure(temp.path(), &["--from", "lib", "--components", "Icon"]);
    assert_error(
        &stderr,
        "can't find component 'Icon': node_modules/lib/index.js has no ES exports",
    );
}

#[test]
fn owners_without_a_repository() {
    let temp = project("", &[]);
    let stderr = failure(temp.path(), &["--from", "lib", "--owners"]);
    assert_error(&stderr, "no git repository at or above ");
}

#[test]
fn owners_without_a_codeowners_file() {
    let temp = project("", &[]);
    fs::create_dir(temp.path().join(".git")).unwrap();
    let stderr = failure(temp.path(), &["--from", "lib", "--owners"]);
    assert_error(&stderr, "no CODEOWNERS file in ");
}

#[test]
fn owners_file_that_cant_be_read() {
    let temp = project("", &[]);
    fs::create_dir(temp.path().join(".git")).unwrap();
    let missing = temp.path().join("OWNERS");
    let stderr = failure(
        temp.path(),
        &["--from", "lib", &format!("--owners={}", missing.display())],
    );
    assert_error(&stderr, &format!("can't read {}", missing.display()));
}

#[test]
fn owners_file_too_large_for_github() {
    let temp = project("", &[]);
    fs::create_dir(temp.path().join(".git")).unwrap();
    fs::write(temp.path().join("CODEOWNERS"), "#".repeat(3_000_000)).unwrap();
    let stderr = failure(temp.path(), &["--from", "lib", "--owners"]);
    assert_error(
        &stderr,
        "CODEOWNERS is 3 MB or larger, so GitHub doesn't load it",
    );
}
