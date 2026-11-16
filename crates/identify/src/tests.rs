use std::path::Path;

use graph::{Module, ModuleGraph, OriginError};
use model::{FileId, Location, Parent, Usage};

use crate::{Cause, Targets, identify};

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

/// A usage without its element index and parents, which have their own tests.
fn usage(line: u32, column: u32, component: &str) -> Usage {
    Usage {
        file: FileId(0),
        location: Location { line, column },
        component: component.to_owned(),
        element: 0,
        parents: Vec::new(),
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
    identities(identified.usages)
}

fn identities(usages: Vec<Usage>) -> Vec<Usage> {
    usages
        .into_iter()
        .map(|usage| Usage {
            element: 0,
            parents: Vec::new(),
            ..usage
        })
        .collect()
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
    assert_eq!(identities(identified.usages), [usage(2, 11, "Icon")]);
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
    assert_eq!(identities(identified.usages), [usage(2, 11, "Icons.Icon")]);
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
        identified.unidentified[0].cause,
        Cause::Origin(OriginError::Unresolved { .. })
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
        [unidentified] if matches!(unidentified.cause, Cause::Origin(OriginError::MissingExport { .. }))
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

#[test]
fn aliases_of_targets_are_unidentified() {
    let graph = graph(&[
        (
            "import * as L from 'lib';\nconst I = L.Icon;\nconst a = <I.Item />;\nconst b = <I />;",
            &[("lib", 1)],
        ),
        LIB,
        ICON,
    ]);
    let identified = identify(&graph, &[FileId(0)], &targets(&graph, 1, &["Icon.Item"]));
    assert_eq!(identified.usages, []);
    assert!(matches!(
        &identified.unidentified[..],
        [unidentified] if unidentified.tag == "I.Item"
            && unidentified.source == "lib"
            && unidentified.cause == Cause::Alias { component: "Icon.Item".to_owned() }
    ));
}

#[test]
fn parents_skip_elements_that_arent_usages() {
    let graph = graph(&[
        (
            "import { Icon, Other } from 'lib';\nconst Local = () => null;\nconst a = <Icon x={<Local><Other><Icon.Item /></Other></Local>} />;",
            &[("lib", 1)],
        ),
        LIB,
        ICON,
    ]);
    let identified = identify(
        &graph,
        &[FileId(0)],
        &targets(&graph, 1, &["Icon", "Icon.Item"]),
    );
    let parents: Vec<(usize, Vec<Parent>)> = identified
        .usages
        .into_iter()
        .map(|usage| (usage.element, usage.parents))
        .collect();
    assert_eq!(
        parents,
        [
            (0, vec![]),
            (
                3,
                vec![Parent {
                    component: "Icon".to_owned(),
                    location: Location {
                        line: 3,
                        column: 11
                    },
                    prop: Some("x".to_owned()),
                }]
            ),
        ]
    );
}

#[test]
fn all_exports_are_targets() {
    let graph = graph(&[
        (
            "import Default, { Icon, Other } from 'lib';\nconst a = <><Icon.Item /><Other /><Default /></>;",
            &[("lib", 1)],
        ),
        (
            "export * from './icon';\nexport default () => null;",
            &[("./icon", 2)],
        ),
        ICON,
    ]);
    let identified = identify(&graph, &[FileId(0)], &Targets::all(&graph, FileId(1)));
    assert_eq!(
        identities(identified.usages),
        [usage(2, 13, "Icon.Item"), usage(2, 26, "Other")]
    );
}
