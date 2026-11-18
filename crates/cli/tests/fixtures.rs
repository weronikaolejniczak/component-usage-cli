use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;
use tempfile::TempDir;

#[derive(Deserialize)]
struct Case {
    query: Query,
    #[serde(default)]
    usages: Vec<Usage>,
    /// With `query.summary`, the whole report.
    #[serde(default)]
    summary: Option<serde_json::Value>,
    #[serde(default)]
    references: Vec<Reference>,
    #[serde(default)]
    unidentified: Vec<Located>,
    #[serde(default)]
    errors: Vec<Located>,
    #[serde(default)]
    warnings: Vec<Warning>,
}

#[derive(Deserialize)]
struct Query {
    /// Absent scans every export of `from`.
    #[serde(default)]
    components: Option<Vec<String>>,
    from: String,
    #[serde(default)]
    props: Option<Vec<String>>,
    #[serde(default)]
    follow: Option<Vec<String>>,
    #[serde(default)]
    wrappers: bool,
    #[serde(default)]
    summary: bool,
    /// Scan root, relative to the case folder.
    #[serde(default)]
    dir: Option<String>,
    /// `""` passes `--owners` without a path.
    #[serde(default)]
    owners: Option<String>,
}

#[derive(Deserialize)]
struct Report {
    usages: Vec<Usage>,
    references: Vec<Reference>,
    unidentified: Vec<Located>,
    errors: Vec<Located>,
    warnings: Vec<Warning>,
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
struct Located {
    file: String,
    #[serde(default)]
    line: Option<u32>,
    #[serde(default)]
    column: Option<u32>,
    code: String,
    #[serde(default)]
    component: Option<String>,
}

/// `parents` and `props` are compared only when the expected usage lists them.
#[derive(Debug, PartialEq, Eq, Deserialize)]
struct Usage {
    component: String,
    file: String,
    line: u32,
    column: u32,
    #[serde(default)]
    owners: Option<Vec<String>>,
    #[serde(default)]
    through: Option<serde_json::Value>,
    #[serde(default)]
    parents: Option<serde_json::Value>,
    #[serde(default)]
    props: Option<serde_json::Value>,
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
struct Reference {
    component: String,
    file: String,
    line: u32,
    column: u32,
    #[serde(rename = "in")]
    passed: serde_json::Value,
    #[serde(default)]
    owners: Option<Vec<String>>,
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
struct Warning {
    file: String,
    line: u32,
    code: String,
}

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

/// Copies the case and the group's `node_modules` to a temporary folder and marks the case
/// folder as a repository root, since a real `.git` folder can't be committed inside this
/// repository.
fn as_repository(group: &str, name: &str) -> TempDir {
    let temp = TempDir::new().unwrap();
    let group_dir = fixtures_dir().join(group);
    copy_dir(&group_dir.join(name), &temp.path().join(name));
    copy_dir(
        &group_dir.join("node_modules"),
        &temp.path().join("node_modules"),
    );
    fs::create_dir(temp.path().join(name).join(".git")).unwrap();
    temp
}

fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

fn run_case(group: &str, name: &str) {
    let case_dir = fixtures_dir().join(group).join(name);
    let case: Case =
        serde_json::from_str(&fs::read_to_string(case_dir.join("expected.json")).unwrap()).unwrap();
    let temp = case
        .query
        .owners
        .is_some()
        .then(|| as_repository(group, name));
    let case_dir = match &temp {
        Some(temp) => temp.path().join(name),
        None => case_dir,
    };
    let scan_dir = match &case.query.dir {
        Some(dir) => case_dir.join(dir),
        None => case_dir.clone(),
    };

    let mut command = Command::new(env!("CARGO_BIN_EXE_component-usage-cli"));
    command.arg("scan").arg(&scan_dir);
    if let Some(components) = &case.query.components {
        command.arg("--components").arg(components.join(","));
    }
    command.arg("--from").arg(&case.query.from);
    if let Some(props) = &case.query.props {
        command.arg("--props").arg(props.join(","));
    }
    if let Some(follow) = &case.query.follow {
        command.arg("--follow").arg(follow.join(","));
    }
    if case.query.wrappers {
        command.arg("--wrappers");
    }
    if case.query.summary {
        command.arg("--summary");
    }
    match case.query.owners.as_deref() {
        Some("") => {
            command.arg("--owners");
        }
        Some(path) => {
            command.arg(format!("--owners={}", case_dir.join(path).display()));
        }
        None => {}
    }
    let output = command.output().unwrap();

    assert!(
        output.status.success(),
        "scan failed for {group}/{name}:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    if case.query.summary {
        let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(Some(report), case.summary, "{group}/{name} summary");
        return;
    }
    let mut report: Report = serde_json::from_slice(&output.stdout).unwrap();
    for (actual, expected) in report.usages.iter_mut().zip(&case.usages) {
        if expected.parents.is_none() {
            actual.parents = None;
        }
        if expected.props.is_none() {
            actual.props = None;
        }
    }
    assert_eq!(report.usages, case.usages, "{group}/{name} usages");
    assert_eq!(
        report.references, case.references,
        "{group}/{name} references"
    );
    assert_eq!(
        report.unidentified, case.unidentified,
        "{group}/{name} unidentified"
    );
    assert_eq!(report.errors, case.errors, "{group}/{name} errors");
    assert_eq!(report.warnings, case.warnings, "{group}/{name} warnings");
}

macro_rules! cases {
    ($group:ident: $($name:ident),* $(,)?) => {
        mod $group {
            $(
                #[test]
                fn $name() {
                    super::run_case(stringify!($group), &stringify!($name).replace('_', "-"));
                }
            )*

            #[test]
            fn all_cases_listed() {
                let listed = [$(stringify!($name).replace('_', "-")),*];
                let mut on_disk: Vec<String> = std::fs::read_dir(super::fixtures_dir().join(stringify!($group)))
                    .unwrap()
                    .map(|entry| entry.unwrap().file_name().into_string().unwrap())
                    .filter(|name| name != "node_modules")
                    .collect();
                on_disk.sort();
                let mut listed = listed.to_vec();
                listed.sort();
                assert_eq!(listed, on_disk, "fixture folders and test list differ");
            }
        }
    };
}

cases!(identify:
    aliased_import,
    barrel_reexport,
    commonjs,
    commonjs_module,
    deep_import,
    export_aliases,
    export_aliases_all,
    hidden_directory,
    local_alias,
    member_expression,
    named_import,
    namespace_import,
    package_exports,
    same_name_other_module,
    shadowed_local,
    tsconfig_paths,
    unresolved_import,
);

cases!(errors:
    syntax_error,
);

cases!(props:
    all_components,
    default_export,
    expressions,
    follow_calls,
    follow_imports,
    follow_props,
    follow_variables,
    literals,
    local_aliases,
    objects,
    parents,
    props_filter,
    references,
    summary,
    summary_spreads,
    templates,
    wrapper_props,
    wrappers,
);

cases!(owners:
    anchored_and_unanchored,
    case_sensitive,
    direct_children,
    double_star,
    email_owners,
    explicit_file,
    invalid_lines,
    last_match_wins,
    later_lines_kept,
    location_order,
    multiple_owners,
    no_owners,
    subdirectory_scan,
);
