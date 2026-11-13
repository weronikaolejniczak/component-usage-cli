use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;

#[derive(Deserialize)]
struct Case {
    query: Query,
    usages: Vec<Usage>,
    #[serde(default)]
    warnings: Vec<Warning>,
}

#[derive(Deserialize)]
struct Query {
    components: Vec<String>,
    from: String,
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
    warnings: Vec<Warning>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
struct Usage {
    component: String,
    file: String,
    line: u32,
    column: u32,
    #[serde(default)]
    owners: Option<Vec<String>>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
struct Warning {
    file: String,
    line: u32,
}

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

/// Copies the group to a temporary folder and marks the case folder as a repository root,
/// since a real `.git` folder can't be committed inside this repository.
fn as_repository(group: &str, name: &str) -> PathBuf {
    let temp = std::env::temp_dir().join(format!(
        "component-usage-cli-{group}-{name}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&temp);
    copy_dir(&fixtures_dir().join(group), &temp);
    fs::create_dir(temp.join(name).join(".git")).unwrap();
    temp.join(name)
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
    let case_dir = match case.query.owners {
        Some(_) => as_repository(group, name),
        None => case_dir,
    };
    let scan_dir = match &case.query.dir {
        Some(dir) => case_dir.join(dir),
        None => case_dir.clone(),
    };

    let mut command = Command::new(env!("CARGO_BIN_EXE_component-usage-cli"));
    command
        .arg("scan")
        .arg(&scan_dir)
        .arg("--components")
        .arg(case.query.components.join(","))
        .arg("--from")
        .arg(&case.query.from);
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

    let report: Report = serde_json::from_slice(&output.stdout).unwrap();
    let mut actual = report.usages;
    actual.sort();
    let mut expected = case.usages;
    expected.sort();
    assert_eq!(actual, expected, "{group}/{name}");

    let mut warnings = report.warnings;
    warnings.sort();
    let mut expected_warnings = case.warnings;
    expected_warnings.sort();
    assert_eq!(warnings, expected_warnings, "{group}/{name} warnings");
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
    deep_import,
    member_expression,
    named_import,
    namespace_import,
    same_name_other_module,
    shadowed_local,
);

cases!(owners:
    anchored_and_unanchored,
    case_sensitive,
    direct_children,
    double_star,
    explicit_file,
    invalid_lines,
    last_match_wins,
    location_order,
    multiple_owners,
    no_owners,
    subdirectory_scan,
);
