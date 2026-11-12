use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;

#[derive(Deserialize)]
struct Case {
    query: Query,
    usages: Vec<Usage>,
}

#[derive(Deserialize)]
struct Query {
    components: Vec<String>,
    from: String,
}

#[derive(Deserialize)]
struct Report {
    usages: Vec<Usage>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
struct Usage {
    component: String,
    file: String,
    line: u32,
    column: u32,
}

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

fn run_case(group: &str, name: &str) {
    let dir = fixtures_dir().join(group).join(name);
    let case: Case =
        serde_json::from_str(&fs::read_to_string(dir.join("expected.json")).unwrap()).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_component-usage-cli"))
        .arg("scan")
        .arg(&dir)
        .arg("--components")
        .arg(case.query.components.join(","))
        .arg("--from")
        .arg(&case.query.from)
        .output()
        .unwrap();

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
