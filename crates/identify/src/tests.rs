use std::path::Path;

use graph::{Module, ModuleGraph, OriginError};
use model::{FileId, Location, Usage};

use crate::{Targets, identify};

/// Each file is its source plus the specifiers it uses, mapped to indices in this list.
fn graph(files: &[(&str, &[(&str, u32)])]) -> ModuleGraph {
    ModuleGraph::build(
        files
            .iter()
            .map(|(source, resolved)| {
                Some(Module {
                    summary: parse::parse(Path::new("file.tsx"), source).unwrap(),
                    resolved: resolved
                        .iter()
                        .map(|&(specifier, target)| (specifier.to_owned(), Ok(FileId(target))))
                        .collect(),
                })
            })
            .collect(),
    )
}

fn targets(graph: &ModuleGraph, module: u32, components: &[&str]) -> Targets {
    let components: Vec<String> = components.iter().map(|&c| c.to_owned()).collect();
    Targets::resolve(graph, FileId(module), &components).unwrap()
}

fn usage(line: u32, column: u32, component: &str) -> Usage {
    Usage {
        file: FileId(0),
        location: Location { line, column },
        component: component.to_owned(),
    }
}

/// File 1 is the library entry, file 2 defines the components.
const LIB: (&str, &[(&str, u32)]) = ("export * from './icon';", &[("./icon", 2)]);
const ICON: (&str, &[(&str, u32)]) = (
    "export const Icon = () => null;\nIcon.Item = () => null;\nexport const Other = () => null;",
    &[],
);

fn usages(app: &str, components: &[&str]) -> Vec<Usage> {
    let graph = graph(&[(app, &[("lib", 1), ("other-lib", 3)]), LIB, ICON, ICON]);
    let targets = targets(&graph, 1, components);
    let identified = identify(&graph, &[FileId(0)], &targets);
    assert_eq!(identified.unidentified, []);
    identified.usages
}

#[test]
fn named_import() {
    assert_eq!(
        usages(
            "import { Icon } from 'lib';\nconst a = <Icon />;",
            &["Icon"]
        ),
        [usage(2, 11, "Icon")]
    );
}

#[test]
fn aliased_import() {
    assert_eq!(
        usages(
            "import { Icon as I } from 'lib';\nconst a = <I />;",
            &["Icon"]
        ),
        [usage(2, 11, "Icon")]
    );
}

#[test]
fn namespace_import() {
    assert_eq!(
        usages(
            "import * as L from 'lib';\nconst a = <L.Icon />;",
            &["Icon"]
        ),
        [usage(2, 11, "Icon")]
    );
}

#[test]
fn import_from_defining_module() {
    let graph = graph(&[
        (
            "import { Icon } from 'lib/icon';\nconst a = <Icon />;",
            &[("lib/icon", 2)],
        ),
        LIB,
        ICON,
    ]);
    let identified = identify(&graph, &[FileId(0)], &targets(&graph, 1, &["Icon"]));
    assert_eq!(identified.usages, [usage(2, 11, "Icon")]);
}

#[test]
fn member_expression_is_a_separate_component() {
    let app = "import { Icon } from 'lib';\nconst a = <Icon><Icon.Item /></Icon>;";
    assert_eq!(usages(app, &["Icon"]), [usage(2, 11, "Icon")]);
    assert_eq!(
        usages(app, &["Icon", "Icon.Item"]),
        [usage(2, 11, "Icon"), usage(2, 17, "Icon.Item")]
    );
}

#[test]
fn member_through_namespace() {
    assert_eq!(
        usages(
            "import * as L from 'lib';\nconst a = <L.Icon.Item />;",
            &["Icon.Item"]
        ),
        [usage(2, 11, "Icon.Item")]
    );
}

#[test]
fn namespace_reexport_target() {
    let graph = graph(&[
        (
            "import { Icons } from 'lib';\nconst a = <Icons.Icon />;",
            &[("lib", 1)],
        ),
        ("export * as Icons from './icon';", &[("./icon", 2)]),
        ICON,
    ]);
    let identified = identify(&graph, &[FileId(0)], &targets(&graph, 1, &["Icons.Icon"]));
    assert_eq!(identified.usages, [usage(2, 11, "Icons.Icon")]);
}

#[test]
fn same_name_from_other_module() {
    assert_eq!(
        usages(
            "import { Icon } from 'other-lib';\nconst a = <Icon />;",
            &["Icon"]
        ),
        []
    );
}

#[test]
fn other_export_of_same_module() {
    assert_eq!(
        usages(
            "import { Other } from 'lib';\nconst a = <Other />;",
            &["Icon"]
        ),
        []
    );
}

#[test]
fn shadowed_and_local_elements() {
    let app = "import { Icon } from 'lib';\nconst a = <Icon />;\nfunction B() {\n  const Icon = () => null;\n  return <Icon />;\n}";
    assert_eq!(usages(app, &["Icon"]), [usage(2, 11, "Icon")]);
}

#[test]
fn unidentified_elements_are_reported() {
    let graph = graph(&[
        (
            "import { Icon } from 'lib';\nimport * as L from 'lib';\nconst a = <><Icon /><L.Missing /></>;",
            &[],
        ),
        LIB,
        ICON,
    ]);
    let identified = identify(&graph, &[FileId(0)], &targets(&graph, 1, &["Icon"]));
    assert_eq!(identified.usages, []);
    assert_eq!(
        identified
            .unidentified
            .iter()
            .map(|unidentified| unidentified.location.column)
            .collect::<Vec<_>>(),
        [13, 21]
    );
    assert!(matches!(
        identified.unidentified[0].error,
        OriginError::Unresolved { .. }
    ));
}

#[test]
fn missing_namespace_member_is_unidentified() {
    let graph = graph(&[
        (
            "import * as L from 'lib';\nconst a = <L.Missing />;",
            &[("lib", 1)],
        ),
        LIB,
        ICON,
    ]);
    let identified = identify(&graph, &[FileId(0)], &targets(&graph, 1, &["Icon"]));
    assert!(matches!(
        &identified.unidentified[..],
        [unidentified] if matches!(unidentified.error, OriginError::MissingExport { .. })
    ));
}

#[test]
fn unknown_target() {
    let graph = graph(&[LIB, LIB, ICON]);
    let error = Targets::resolve(&graph, FileId(1), &["Missing".to_owned()])
        .err()
        .unwrap();
    assert_eq!(error.component, "Missing");
}
