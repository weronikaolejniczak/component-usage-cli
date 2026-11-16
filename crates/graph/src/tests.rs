use std::path::Path;

use model::{ExportRef, FileId};

use crate::{
    Module, ModuleGraph, Origin, OriginError, ResolveError, element_specifiers, export_specifiers,
};

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

fn export(module: u32, name: &str) -> Origin {
    Origin::Export(ExportRef {
        module: FileId(module),
        name: name.to_owned(),
    })
}

const ICON: &str = "export const Icon = () => null;";

#[test]
fn named_import_of_local_export() {
    let graph = graph(&[("import { Icon } from 'lib';", &[("lib", 1)]), (ICON, &[])]);
    assert_eq!(graph.origin(FileId(0), 0), Ok(export(1, "Icon")));
}

#[test]
fn aliased_import_keeps_exported_name() {
    let graph = graph(&[
        ("import { Icon as I } from 'lib';", &[("lib", 1)]),
        (ICON, &[]),
    ]);
    assert_eq!(graph.origin(FileId(0), 0), Ok(export(1, "Icon")));
}

#[test]
fn namespace_import() {
    let graph = graph(&[("import * as L from 'lib';", &[("lib", 1)]), (ICON, &[])]);
    assert_eq!(graph.origin(FileId(0), 0), Ok(Origin::Namespace(FileId(1))));
}

#[test]
fn reexport_chain() {
    let graph = graph(&[
        ("import { Glyph } from './ui';", &[("./ui", 1)]),
        ("export { Icon as Glyph } from 'lib';", &[("lib", 2)]),
        ("export { Icon } from './icon';", &[("./icon", 3)]),
        (ICON, &[]),
    ]);
    assert_eq!(graph.origin(FileId(0), 0), Ok(export(3, "Icon")));
}

#[test]
fn star_reexport() {
    let graph = graph(&[
        ("import { Icon } from './ui';", &[("./ui", 1)]),
        (
            "export * from './other';\nexport * from './icon';",
            &[("./other", 2), ("./icon", 3)],
        ),
        ("export const Other = 1;", &[]),
        (ICON, &[]),
    ]);
    assert_eq!(graph.origin(FileId(0), 0), Ok(export(3, "Icon")));
}

#[test]
fn star_does_not_reexport_default() {
    let graph = graph(&[
        ("import Icon from './ui';", &[("./ui", 1)]),
        ("export * from './icon';", &[("./icon", 2)]),
        ("export default () => null;", &[]),
    ]);
    assert_eq!(
        graph.origin(FileId(0), 0),
        Err(OriginError::MissingExport {
            module: FileId(1),
            name: "default".to_owned(),
        })
    );
}

#[test]
fn same_binding_through_two_stars_is_not_ambiguous() {
    let graph = graph(&[
        (
            "export * from './a';\nexport * from './b';",
            &[("./a", 1), ("./b", 2)],
        ),
        ("export { Icon } from './icon';", &[("./icon", 3)]),
        ("export { Icon } from './icon';", &[("./icon", 3)]),
        (ICON, &[]),
    ]);
    assert_eq!(
        graph.export_origin(FileId(0), "Icon"),
        Ok(export(3, "Icon"))
    );
}

#[test]
fn different_bindings_through_two_stars_are_ambiguous() {
    let graph = graph(&[
        (
            "export * from './a';\nexport * from './b';",
            &[("./a", 1), ("./b", 2)],
        ),
        (ICON, &[]),
        (ICON, &[]),
    ]);
    assert_eq!(
        graph.export_origin(FileId(0), "Icon"),
        Err(OriginError::Ambiguous {
            module: FileId(0),
            name: "Icon".to_owned(),
        })
    );
}

#[test]
fn explicit_export_wins_over_star() {
    let graph = graph(&[
        (
            "export * from './a';\nexport { Icon } from './b';",
            &[("./a", 1), ("./b", 2)],
        ),
        (ICON, &[]),
        (ICON, &[]),
    ]);
    assert_eq!(
        graph.export_origin(FileId(0), "Icon"),
        Ok(export(2, "Icon"))
    );
}

#[test]
fn imported_then_exported() {
    let graph = graph(&[
        (
            "import { Icon } from './icon';\nexport { Icon as Glyph };",
            &[("./icon", 1)],
        ),
        (ICON, &[]),
    ]);
    assert_eq!(
        graph.export_origin(FileId(0), "Glyph"),
        Ok(export(1, "Icon"))
    );
}

#[test]
fn default_export_of_import() {
    let graph = graph(&[
        (
            "import { Icon } from './icon';\nexport default Icon;",
            &[("./icon", 1)],
        ),
        (ICON, &[]),
    ]);
    assert_eq!(
        graph.export_origin(FileId(0), "default"),
        Ok(export(1, "Icon"))
    );
}

#[test]
fn namespace_reexport() {
    let graph = graph(&[
        ("export * as L from './lib';", &[("./lib", 1)]),
        (ICON, &[]),
    ]);
    assert_eq!(
        graph.export_origin(FileId(0), "L"),
        Ok(Origin::Namespace(FileId(1)))
    );
}

#[test]
fn cycle() {
    let graph = graph(&[
        ("export { Icon } from './b';", &[("./b", 1)]),
        ("export { Icon } from './a';", &[("./a", 0)]),
    ]);
    assert!(matches!(
        graph.export_origin(FileId(0), "Icon"),
        Err(OriginError::Cycle { .. })
    ));
}

#[test]
fn star_cycle_still_finds_export() {
    let graph = graph(&[
        (
            "export * from './b';\nexport * from './icon';",
            &[("./b", 1), ("./icon", 2)],
        ),
        ("export * from './a';", &[("./a", 0)]),
        (ICON, &[]),
    ]);
    assert_eq!(
        graph.export_origin(FileId(0), "Icon"),
        Ok(export(2, "Icon"))
    );
}

#[test]
fn unresolved_specifier() {
    let mut graph = graph(&[("import { Icon } from 'lib';", &[])]);
    graph.modules[0]
        .as_mut()
        .unwrap()
        .resolved
        .insert("lib".to_owned(), Err(ResolveError::not_requested()));
    assert!(matches!(
        graph.origin(FileId(0), 0),
        Err(OriginError::Unresolved { specifier, .. }) if specifier == "lib"
    ));
}

#[test]
fn unparsed_module() {
    let mut graph = graph(&[("import { Icon } from 'lib';", &[("lib", 1)]), (ICON, &[])]);
    graph.modules[1] = None;
    assert_eq!(
        graph.origin(FileId(0), 0),
        Err(OriginError::Unparsed { module: FileId(1) })
    );
}

#[test]
fn specifiers_needed() {
    let summary = parse::parse(
        Path::new("file.tsx"),
        "import { Icon } from 'lib';\nimport { used } from 'util';\nimport { Item } from './item';\n\
         export { Item };\nexport * from './all';\nexport { X } from './x';\nconst a = <Icon />;",
    )
    .unwrap();
    let mut exports: Vec<_> = export_specifiers(&summary).collect();
    exports.sort_unstable();
    assert_eq!(exports, ["./all", "./item", "./x"]);
    assert_eq!(element_specifiers(&summary).collect::<Vec<_>>(), ["lib"]);
}

#[test]
fn invalid_import_index() {
    let graph = graph(&[("import { Icon } from 'lib';", &[("lib", 1)]), (ICON, &[])]);
    assert_eq!(
        graph.origin(FileId(0), 5),
        Err(OriginError::MissingImport {
            file: FileId(0),
            import: 5,
        })
    );
}

#[test]
fn export_names_follow_stars_and_skip_default() {
    let graph = graph(&[
        (
            "export * from './a';\nexport { X as Y } from './a';\nexport * as ns from './a';\nexport default 1;",
            &[("./a", 1)],
        ),
        (
            "export const A = 1;\nexport default 2;\nexport * from './b';",
            &[("./b", 2)],
        ),
        ("export const B = 1;\nexport * from './a';", &[("./a", 1)]),
    ]);
    assert_eq!(graph.export_names(FileId(0)), ["A", "B", "Y", "ns"]);
}
