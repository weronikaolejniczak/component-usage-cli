use std::path::Path;

use model::Location;

use crate::{
    Alias, AliasRoot, Attribute, Binding, Export, Expr, ExprKind, FileSummary, Import, Imported,
    Item, JsxElement, Key, Literal, Parent, ParseError, Reference, Segment, parse,
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
        parent: None,
        attributes: Vec::new(),
        children: None,
        component: None,
    }
}

/// Elements without their parents, attributes and children, which have their own tests.
fn identities(summary: &FileSummary) -> Vec<JsxElement> {
    summary
        .elements
        .iter()
        .map(|element| JsxElement {
            parent: None,
            attributes: Vec::new(),
            children: None,
            ..element.clone()
        })
        .collect()
}

fn parents(summary: &FileSummary) -> Vec<Option<Parent>> {
    summary
        .elements
        .iter()
        .map(|element| element.parent.clone())
        .collect()
}

fn parent(line: u32, column: u32, attribute: Option<&str>) -> Option<Parent> {
    Some(Parent {
        location: Location { line, column },
        attribute: attribute.map(str::to_owned),
    })
}

fn expr(line: u32, column: u32, kind: ExprKind) -> Expr {
    Expr {
        location: Location { line, column },
        kind,
    }
}

#[test]
fn parent_is_the_closest_element_with_a_binding() {
    let summary = tsx(concat!(
        "<A x={<div y={1}><B /></div>}>\n",
        "  <>\n",
        "    <C />\n",
        "  </>\n",
        "  <D render={() => <E />} {...{ f: <F /> }} />\n",
        "</A>;\n",
    ));
    assert_eq!(
        parents(&summary),
        [
            None,
            parent(1, 1, Some("x")),
            parent(1, 1, None),
            parent(1, 1, None),
            parent(5, 3, Some("render")),
            parent(5, 3, None),
        ]
    );
}

#[test]
fn attributes_in_source_order() {
    let summary = tsx("<A a=\"s\" b c:d={x} {...y} e={<B />} f={<></>} g={} />;\n");
    let global = |text: &str| ExprKind::Reference {
        name: Reference::Global,
        text: text.to_owned(),
        path: Vec::new(),
    };
    assert_eq!(
        summary.elements[0].attributes,
        [
            Attribute::Named {
                name: "a".to_owned(),
                value: expr(1, 6, ExprKind::Literal(Literal::String("s".to_owned()))),
            },
            Attribute::Named {
                name: "b".to_owned(),
                value: expr(1, 10, ExprKind::Literal(Literal::Boolean(true))),
            },
            Attribute::Named {
                name: "c:d".to_owned(),
                value: expr(1, 17, global("x")),
            },
            Attribute::Spread(expr(1, 24, global("y"))),
            Attribute::Named {
                name: "e".to_owned(),
                value: expr(
                    1,
                    30,
                    ExprKind::Element {
                        tag: Some("B".to_owned()),
                    },
                ),
            },
            Attribute::Named {
                name: "f".to_owned(),
                value: expr(1, 40, ExprKind::Element { tag: None }),
            },
            Attribute::Named {
                name: "g".to_owned(),
                value: expr(1, 50, ExprKind::Undefined),
            },
        ]
    );
}

#[test]
fn references_are_classified_by_declaration() {
    let summary = tsx(concat!(
        "import { a } from 'lib';\n",
        "const b = 1;\n",
        "function F(c, { d }, ...e) {\n",
        "  return <A p={[a, b, c, d, e, g, undefined, a.x?.y, h().i, h()?.i!, b?.[0]!, 1e999]} />;\n",
        "}\n",
    ));
    let [Attribute::Named { value, .. }] = summary.elements[0].attributes.as_slice() else {
        panic!("expected one attribute");
    };
    let ExprKind::Array(items) = &value.kind else {
        panic!("expected an array");
    };
    let kinds: Vec<&ExprKind> = items
        .iter()
        .map(|item| match item {
            Item::Value(value) | Item::Spread(value) => &value.kind,
        })
        .collect();
    let reference = |name, text: &str, path: &[&str]| ExprKind::Reference {
        name,
        text: text.to_owned(),
        path: path
            .iter()
            .map(|key| Key::Static((*key).to_owned()))
            .collect(),
    };
    assert_eq!(
        kinds,
        [
            &reference(Reference::Import(0), "a", &[]),
            &reference(Reference::Variable(0), "b", &[]),
            &reference(Reference::Parameter(None), "c", &[]),
            &reference(Reference::Parameter(None), "d", &[]),
            &reference(Reference::Parameter(None), "e", &[]),
            &reference(Reference::Global, "g", &[]),
            &ExprKind::Undefined,
            &reference(Reference::Import(0), "a.x?.y", &["x", "y"]),
            &ExprKind::Call {
                text: "h().i".to_owned()
            },
            &ExprKind::Call {
                text: "h()?.i!".to_owned()
            },
            &reference(Reference::Variable(0), "b?.[0]!", &["0"]),
            &ExprKind::Unsupported {
                text: "1e999".to_owned()
            },
        ]
    );
}

#[test]
fn named_import() {
    let summary = tsx("import { Icon } from 'lib';\n\nconst a = <Icon type=\"x\" />;\n");
    assert_eq!(summary.imports, [import("lib", name("Icon"), "Icon")]);
    assert_eq!(
        identities(&summary),
        [element(3, 11, "Icon", &[], Binding::Import(0))]
    );
}

#[test]
fn aliased_import() {
    let summary = tsx("import { Icon as I } from 'lib';\nconst a = <I />;\n");
    assert_eq!(summary.imports, [import("lib", name("Icon"), "I")]);
    assert_eq!(
        identities(&summary),
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
        identities(&summary),
        [element(2, 11, "L", &["Icon"], Binding::Import(0))]
    );
}

#[test]
fn member_expressions() {
    let summary = tsx(
        "import { Icon } from 'lib';\nimport * as L from 'lib';\nconst a = (\n  <Icon>\n    <Icon.Item />\n    <L.Icon.Item />\n  </Icon>\n);\n",
    );
    assert_eq!(
        identities(&summary),
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
        identities(&summary),
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
        identities(&summary),
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

fn with_imports(source: &str) -> FileSummary {
    crate::parse_with(
        Path::new("App.tsx"),
        source,
        crate::Options {
            variables: true,
            imports: true,
            props: false,
        },
    )
    .unwrap()
}

#[test]
fn exports_record_what_they_hold() {
    let summary = with_imports(
        "import { other } from './other';\n\
         const local = 'a';\n\
         export const { a, b: [c] } = value;\n\
         export function handle() {}\n\
         export { local as renamed, other };\n\
         export default 'gear';\n\
         export type T = string;\n",
    );
    let mut names: Vec<&str> = summary.exported.keys().map(String::as_str).collect();
    names.sort_unstable();
    assert_eq!(names, ["a", "c", "default", "handle", "renamed"]);
    assert_eq!(
        summary.exported["default"],
        crate::Exported::Value(expr(
            6,
            16,
            ExprKind::Literal(Literal::String("gear".to_owned()))
        ))
    );
    let crate::Exported::Variable(renamed) = summary.exported["renamed"] else {
        panic!("not a variable");
    };
    assert!(matches!(
        &summary.definitions[renamed],
        crate::Definition::Value { init, .. }
            if init.kind == ExprKind::Literal(Literal::String("a".to_owned()))
    ));
}

#[test]
fn exports_are_recorded_only_when_following_imports() {
    assert!(tsx("export const a = 'a';").exported.is_empty());
}

#[test]
fn declarations_without_a_value_are_declared() {
    let summary = with_imports(
        "declare const a: string;\n\
         export declare function f(): void;\n\
         export const b = a;\n",
    );
    let crate::Exported::Variable(f) = summary.exported["f"] else {
        panic!("not a variable");
    };
    assert_eq!(summary.definitions[f], crate::Definition::Declared);
    let crate::Exported::Variable(b) = summary.exported["b"] else {
        panic!("not a variable");
    };
    let crate::Definition::Value { init, .. } = &summary.definitions[b] else {
        panic!("not a value");
    };
    let ExprKind::Reference {
        name: Reference::Variable(a),
        ..
    } = init.kind
    else {
        panic!("not a variable reference");
    };
    assert_eq!(summary.definitions[a], crate::Definition::Declared);
}

#[test]
fn imports_changed_in_place() {
    let summary = with_imports(
        "import { list, map, read } from './a';\n\
         import * as all from './b';\n\
         list.push(1);\n\
         map.key = 1;\n\
         all.x.y = 1;\n\
         read.slice();\n",
    );
    assert_eq!(summary.changed_imports, [0, 1, 3]);
}

#[test]
fn value_imports_follow_definitions_from_exports() {
    let summary = with_imports(
        "import { a } from './a';\n\
         import { b } from './b';\n\
         import { c } from './c';\n\
         import { Button } from 'lib';\n\
         const local = { a };\n\
         export const value = local;\n\
         const unused = c;\n\
         <Button icon={b} />;\n",
    );
    assert_eq!(summary.value_imports(false), [0]);
    assert_eq!(summary.value_imports(true), [0, 1]);
}

fn with_props(source: &str) -> FileSummary {
    crate::parse_with(
        Path::new("App.tsx"),
        source,
        crate::Options {
            variables: true,
            imports: true,
            props: true,
        },
    )
    .unwrap()
}

#[test]
fn children_skip_whitespace() {
    let summary = tsx("const a = <A>\n  </A>;\nconst b = <B>\n  text</B>;\n");
    assert_eq!(summary.elements[0].children, None);
    assert_eq!(
        summary.elements[1].children,
        Some(Location {
            line: 3,
            column: 14
        })
    );
}

#[test]
fn components_are_named_functions_and_constants() {
    let summary = with_props(
        "function A(props) { return <A />; }\n\
         const B = (props) => null;\n\
         const C = memo(forwardRef(function (props, ref) { return null; }));\n\
         const D = React.memo(B);\n\
         let E = () => null;\n\
         const f = compute();\n\
         export default function (props) { return null; }\n",
    );
    let names: Vec<(&str, bool)> = summary
        .components
        .iter()
        .map(|component| (component.name.as_str(), component.default))
        .collect();
    assert_eq!(
        names,
        [("A", false), ("B", false), ("C", false), ("default", true)]
    );
    assert_eq!(summary.components[1].variables.len(), 2, "B and D");
    assert_eq!(summary.elements[0].component, Some(0));
}

#[test]
fn props_record_paths_defaults_and_rest() {
    let summary = with_props(
        "function A({ icon = 'gear', a: { b }, c: { d } = {}, [key]: e, ...rest }, ref) {\n\
         \x20 return <X icon={icon} b={b} d={d} e={e} rest={rest} ref={ref} />;\n}\n",
    );
    let parameters: Vec<(Vec<Segment>, bool, Option<Vec<String>>)> = summary
        .parameters
        .iter()
        .map(|parameter| {
            (
                parameter.path.clone(),
                parameter.default.is_some(),
                parameter.rest.clone(),
            )
        })
        .collect();
    assert_eq!(
        parameters,
        [
            (vec![Segment::Key("icon".to_owned())], true, None),
            (
                vec![Segment::Key("a".to_owned()), Segment::Key("b".to_owned())],
                false,
                None
            ),
            (
                Vec::new(),
                false,
                Some(vec!["icon".to_owned(), "a".to_owned(), "c".to_owned()])
            ),
        ]
    );
    let references: Vec<Reference> = summary.elements[0]
        .attributes
        .iter()
        .map(|attribute| match attribute {
            Attribute::Named {
                value:
                    Expr {
                        kind: ExprKind::Reference { name, .. },
                        ..
                    },
                ..
            } => *name,
            other => panic!("unexpected {other:?}"),
        })
        .collect();
    assert_eq!(
        references,
        [
            Reference::Parameter(Some(0)),
            Reference::Parameter(Some(1)),
            Reference::Parameter(None),
            Reference::Parameter(None),
            Reference::Parameter(Some(2)),
            Reference::Parameter(None),
        ]
    );
}

#[test]
fn components_and_imports_escape_when_used_as_values() {
    let summary = with_props(
        "import { Used, Passed, Typed } from 'lib';\n\
         function Tag() { return <Used />; }\n\
         function Value() { return null; }\n\
         const Wrapped = memo(Tag);\n\
         const styledValue = styled(Value);\n\
         export { Tag };\n\
         export default Wrapped;\n\
         const a = <X component={Passed} />;\n\
         let t: typeof Typed;\n",
    );
    let escapes: Vec<(&str, bool)> = summary
        .components
        .iter()
        .map(|component| (component.name.as_str(), component.escapes))
        .collect();
    assert_eq!(escapes, [("Tag", false), ("Value", true)]);
    assert_eq!(summary.escaping_imports, [1]);
}

#[test]
fn members_of_components_dont_escape_but_calls_and_members_of_imports_do() {
    let summary = with_props(
        "import * as C from 'lib';\n\
         function Named() { return null; }\n\
         function Called() { return null; }\n\
         Named.displayName = 'Named';\n\
         const name = Named.displayName;\n\
         Called.call(null, {});\n\
         const panel = styled(C.Panel);\n",
    );
    let escapes: Vec<(&str, bool)> = summary
        .components
        .iter()
        .map(|component| (component.name.as_str(), component.escapes))
        .collect();
    assert_eq!(escapes, [("Named", false), ("Called", true)]);
    assert_eq!(summary.escaping_imports, [0]);
}

#[test]
fn components_are_recorded_only_when_following_props() {
    let summary = with_imports("function A({ icon }) { return <X icon={icon} />; }\n");
    assert!(summary.components.is_empty());
    assert!(summary.parameters.is_empty());
}
