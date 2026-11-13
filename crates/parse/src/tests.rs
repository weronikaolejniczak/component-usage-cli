use std::path::Path;

use model::Location;

use crate::{
    Alias, AliasRoot, Binding, Export, FileSummary, Import, Imported, JsxElement, ParseError, parse,
};

fn tsx(source: &str) -> FileSummary {
    parse(Path::new("App.tsx"), source).unwrap()
}

fn import(source: &str, imported: Imported, local: &str) -> Import {
    Import {
        source: source.to_owned(),
        imported,
        local: local.to_owned(),
    }
}

fn name(name: &str) -> Imported {
    Imported::Name(name.to_owned())
}

fn element(line: u32, column: u32, root: &str, members: &[&str], binding: Binding) -> JsxElement {
    JsxElement {
        location: Location { line, column },
        root: root.to_owned(),
        members: members.iter().map(|&member| member.to_owned()).collect(),
        binding,
    }
}

#[test]
fn named_import() {
    let summary = tsx("import { Icon } from 'lib';\n\nconst a = <Icon type=\"x\" />;\n");
    assert_eq!(summary.imports, [import("lib", name("Icon"), "Icon")]);
    assert_eq!(
        summary.elements,
        [element(3, 11, "Icon", &[], Binding::Import(0))]
    );
}

#[test]
fn aliased_import() {
    let summary = tsx("import { Icon as I } from 'lib';\nconst a = <I />;\n");
    assert_eq!(summary.imports, [import("lib", name("Icon"), "I")]);
    assert_eq!(
        summary.elements,
        [element(2, 11, "I", &[], Binding::Import(0))]
    );
}

#[test]
fn default_import() {
    let summary = tsx("import Icon from 'lib';\nconst a = <Icon />;\n");
    assert_eq!(summary.imports, [import("lib", name("default"), "Icon")]);
    assert_eq!(summary.elements[0].binding, Binding::Import(0));
}

#[test]
fn namespace_import() {
    let summary = tsx("import * as L from 'lib';\nconst a = <L.Icon />;\n");
    assert_eq!(summary.imports, [import("lib", Imported::Namespace, "L")]);
    assert_eq!(
        summary.elements,
        [element(2, 11, "L", &["Icon"], Binding::Import(0))]
    );
}

#[test]
fn member_expressions() {
    let summary = tsx(
        "import { Icon } from 'lib';\nimport * as L from 'lib';\nconst a = (\n  <Icon>\n    <Icon.Item />\n    <L.Icon.Item />\n  </Icon>\n);\n",
    );
    assert_eq!(
        summary.elements,
        [
            element(4, 3, "Icon", &[], Binding::Import(0)),
            element(5, 5, "Icon", &["Item"], Binding::Import(0)),
            element(6, 5, "L", &["Icon", "Item"], Binding::Import(1)),
        ]
    );
}

#[test]
fn deep_import_keeps_specifier() {
    let summary = tsx("import { Icon } from 'lib/components/icon';\n");
    assert_eq!(
        summary.imports,
        [import("lib/components/icon", name("Icon"), "Icon")]
    );
}

#[test]
fn shadowed_import_is_local() {
    let summary = tsx(
        "import { Icon } from 'lib';\n\nconst a = <Icon />;\n\nfunction B() {\n  const Icon = () => null;\n  return <Icon />;\n}\n",
    );
    assert_eq!(
        summary.elements,
        [
            element(3, 11, "Icon", &[], Binding::Import(0)),
            element(7, 10, "Icon", &[], Binding::Local),
        ]
    );
}

#[test]
fn local_and_global_bindings() {
    let summary = tsx("const Local = () => null;\nconst a = <><Local /><Missing /></>;\n");
    assert_eq!(
        summary.elements,
        [
            element(2, 13, "Local", &[], Binding::Local),
            element(2, 22, "Missing", &[], Binding::Global),
        ]
    );
}

#[test]
fn skips_intrinsic_namespaced_and_this_tags() {
    let summary =
        tsx("class C { render() { return <div><svg:rect /><this.Item /><this.a.B /></div>; } }\n");
    assert_eq!(summary.elements, []);
}

#[test]
fn skips_type_only_imports_and_exports() {
    let summary = tsx(
        "import type { A } from 'lib';\nimport { type B, C } from 'lib';\nexport type { D } from 'lib';\nexport type * from 'lib';\n",
    );
    assert_eq!(summary.imports, [import("lib", name("C"), "C")]);
    assert_eq!(summary.exports, []);
}

#[test]
fn reexports() {
    let summary = tsx(
        "export { Icon } from 'lib';\nexport { Icon as Glyph } from 'lib';\nexport * from 'other';\nexport * as ns from 'third';\n",
    );
    assert_eq!(
        summary.exports,
        [
            Export::Reexport {
                exported: "Icon".to_owned(),
                source: "lib".to_owned(),
                imported: "Icon".to_owned(),
            },
            Export::Reexport {
                exported: "Glyph".to_owned(),
                source: "lib".to_owned(),
                imported: "Icon".to_owned(),
            },
            Export::Star {
                source: "other".to_owned(),
            },
            Export::Namespace {
                exported: "ns".to_owned(),
                source: "third".to_owned(),
            },
        ]
    );
}

#[test]
fn local_exports() {
    let summary = tsx(
        "const a = 1;\nexport { a as b };\nexport const Icon = () => null;\nexport function Item() {}\nexport default a;\n",
    );
    assert_eq!(
        summary.exports,
        [
            Export::Local {
                exported: "b".to_owned(),
                local: Some("a".to_owned()),
            },
            Export::Local {
                exported: "Icon".to_owned(),
                local: Some("Icon".to_owned()),
            },
            Export::Local {
                exported: "Item".to_owned(),
                local: Some("Item".to_owned()),
            },
            Export::Local {
                exported: "default".to_owned(),
                local: Some("a".to_owned()),
            },
        ]
    );
}

#[test]
fn anonymous_default_export() {
    let summary = tsx("export default () => null;\n");
    assert_eq!(
        summary.exports,
        [Export::Local {
            exported: "default".to_owned(),
            local: None,
        }]
    );
}

#[test]
fn imported_binding_exported_again() {
    let summary = tsx("import { Icon } from 'lib';\nexport { Icon };\n");
    assert_eq!(summary.imports, [import("lib", name("Icon"), "Icon")]);
    assert_eq!(summary.exports.len(), 1);
}

#[test]
fn jsx_in_js_files() {
    let summary = parse(
        Path::new("App.js"),
        "import { Icon } from 'lib';\nconst a = <Icon />;\n",
    )
    .unwrap();
    assert_eq!(summary.elements.len(), 1);
}

#[test]
fn unsupported_extension() {
    assert_eq!(
        parse(Path::new("styles.css"), ""),
        Err(ParseError::UnsupportedExtension)
    );
}

#[test]
fn recovered_errors_keep_results() {
    let summary = tsx("import { Icon } from 'lib';\nconst a;\nconst b = <Icon />;\n");
    assert_eq!(summary.elements.len(), 1);
    assert_eq!(summary.errors.len(), 1);
    assert_eq!(summary.errors[0].location, Location { line: 2, column: 7 });
}

#[test]
fn unrecoverable_error_has_location() {
    let Err(ParseError::Syntax(error)) = parse(Path::new("App.tsx"), "const a = 1;\nconst = ;\n")
    else {
        panic!("expected a syntax error");
    };
    assert_eq!(error.location.line, 2);
}

#[test]
fn unrecoverable_error_is_reported_over_earlier_recovered_ones() {
    let Err(ParseError::Syntax(error)) =
        parse(Path::new("App.tsx"), "const a;\nconst b = 1;\nconst = ;\n")
    else {
        panic!("expected a syntax error");
    };
    assert_eq!(error.location.line, 3);
}

fn alias(root: AliasRoot, members: &[&str]) -> Alias {
    Alias {
        root,
        members: members.iter().map(|&member| member.to_owned()).collect(),
    }
}

#[test]
fn constants_from_imports_are_aliases() {
    let summary = tsx(concat!(
        "import * as Lib from 'lib';\n",
        "const A = Lib.Icon;\n",
        "const { Icon: B, Nested: { Item: C } } = Lib;\n",
        "const D = (A as any);\n",
        "const E = A;\n",
        "<A />; <B />; <C />; <D />; <E.Part />; <A />;\n",
    ));
    let bindings: Vec<Binding> = summary
        .elements
        .iter()
        .map(|element| element.binding)
        .collect();
    assert_eq!(
        bindings,
        [
            Binding::Alias(0),
            Binding::Alias(1),
            Binding::Alias(2),
            Binding::Alias(3),
            Binding::Alias(4),
            Binding::Alias(0),
        ]
    );
    assert_eq!(
        summary.aliases,
        [
            alias(AliasRoot::Import(0), &["Icon"]),
            alias(AliasRoot::Import(0), &["Icon"]),
            alias(AliasRoot::Import(0), &["Nested", "Item"]),
            alias(AliasRoot::Import(0), &["Icon"]),
            alias(AliasRoot::Import(0), &["Icon"]),
        ]
    );
    assert_eq!(summary.elements[4].members, ["Part"]);
}

#[test]
fn require_calls_are_aliases() {
    let summary = parse(
        Path::new("App.js"),
        "const { Icon } = require('lib');\nconst Lib = require('lib').default;\n<Icon />; <Lib />;\n",
    )
    .unwrap();
    assert_eq!(
        summary.aliases,
        [
            alias(AliasRoot::Require("lib".to_owned()), &["Icon"]),
            alias(AliasRoot::Require("lib".to_owned()), &["default"]),
        ]
    );
}

#[test]
fn other_constants_are_local() {
    let summary = tsx(concat!(
        "import { Icon } from 'lib';\n",
        "const require = (name: string) => Icon;\n",
        "const A = require('lib');\n",
        "const B = make(Icon);\n",
        "const [C] = [Icon];\n",
        "const D = E, E = D;\n",
        "const F = Unknown.Icon;\n",
        "<A />; <B />; <C />; <D />; <F />;\n",
    ));
    assert!(
        summary
            .elements
            .iter()
            .all(|element| element.binding == Binding::Local),
        "{:?}",
        summary.elements
    );
    assert!(summary.aliases.is_empty());
}
