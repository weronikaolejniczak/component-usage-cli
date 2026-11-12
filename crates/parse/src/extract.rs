use std::collections::HashMap;
use std::path::Path;

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    IdentifierReference, JSXElementName, JSXMemberExpression, JSXMemberExpressionObject,
    JSXOpeningElement,
};
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_semantic::{Scoping, SemanticBuilder};
use oxc_span::SourceType;
use oxc_syntax::module_record::{
    ExportEntry, ExportExportName, ExportImportName, ImportEntry, ImportImportName,
};

use crate::lines::LineIndex;
use crate::{Binding, Export, FileSummary, Import, Imported, JsxElement, ParseError};

pub(crate) fn extract(path: &Path, source: &str) -> Result<FileSummary, ParseError> {
    let source_type = SourceType::from_path(path).map_err(|_| ParseError::UnsupportedExtension)?;
    let source_type = source_type.with_jsx(source_type.is_jsx() || source_type.is_javascript());
    let lines = LineIndex::new(source);
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, source_type).parse();

    if let Some(error) = parsed.diagnostics.errors().next() {
        let offset = error.labels.first().map_or(0, |label| label.offset());
        return Err(ParseError::Syntax {
            location: lines.location(offset),
            message: error.message.to_string(),
        });
    }

    let record = &parsed.module_record;

    let value_imports: Vec<&ImportEntry> = record
        .import_entries
        .iter()
        .filter(|entry| !entry.is_type)
        .collect();
    let import_at: HashMap<u32, usize> = value_imports
        .iter()
        .enumerate()
        .map(|(index, entry)| (entry.local_name.span.start, index))
        .collect();
    let imports = value_imports.into_iter().map(import).collect();

    let mut export_entries: Vec<&ExportEntry> = record
        .local_export_entries
        .iter()
        .chain(&record.indirect_export_entries)
        .chain(&record.star_export_entries)
        .filter(|entry| !entry.is_type)
        .collect();
    export_entries.sort_by_key(|entry| entry.span.start);
    let exports = export_entries.into_iter().filter_map(export).collect();

    let semantic = SemanticBuilder::new().build(&parsed.program).semantic;
    let mut collector = Elements {
        scoping: semantic.scoping(),
        lines: &lines,
        import_at: &import_at,
        elements: Vec::new(),
    };
    collector.visit_program(&parsed.program);
    let mut elements = collector.elements;
    elements.sort_by_key(|element| element.location);

    Ok(FileSummary {
        imports,
        exports,
        elements,
    })
}

fn import(entry: &ImportEntry) -> Import {
    Import {
        source: entry.module_request.name.to_string(),
        imported: match &entry.import_name {
            ImportImportName::Name(name) => Imported::Name(name.name.to_string()),
            ImportImportName::Default(_) => Imported::Name("default".to_owned()),
            ImportImportName::NamespaceObject => Imported::Namespace,
        },
        local: entry.local_name.name.to_string(),
    }
}

fn export(entry: &ExportEntry) -> Option<Export> {
    let exported = match &entry.export_name {
        ExportExportName::Name(name) => Some(name.name.to_string()),
        ExportExportName::Default(_) => Some("default".to_owned()),
        ExportExportName::Null => None,
    };
    match (&entry.module_request, &entry.import_name, exported) {
        (Some(source), ExportImportName::Name(imported), Some(exported)) => {
            Some(Export::Reexport {
                exported,
                source: source.name.to_string(),
                imported: imported.name.to_string(),
            })
        }
        (Some(source), ExportImportName::All, Some(exported)) => Some(Export::Namespace {
            exported,
            source: source.name.to_string(),
        }),
        (Some(source), ExportImportName::AllButDefault, _) => Some(Export::Star {
            source: source.name.to_string(),
        }),
        (None, _, Some(exported)) => Some(Export::Local {
            exported,
            local: entry.local_name.name().map(|name| name.to_string()),
        }),
        _ => None,
    }
}

struct Elements<'s> {
    scoping: &'s Scoping,
    lines: &'s LineIndex<'s>,
    import_at: &'s HashMap<u32, usize>,
    elements: Vec<JsxElement>,
}

impl<'a> Visit<'a> for Elements<'_> {
    fn visit_jsx_opening_element(&mut self, element: &JSXOpeningElement<'a>) {
        if let Some((root, members)) = tag(&element.name) {
            self.elements.push(JsxElement {
                location: self.lines.location(element.span.start),
                root: root.name.to_string(),
                members,
                binding: binding(self.scoping, root, self.import_at),
            });
        }
        walk::walk_jsx_opening_element(self, element);
    }
}

fn tag<'t, 'a>(name: &'t JSXElementName<'a>) -> Option<(&'t IdentifierReference<'a>, Vec<String>)> {
    match name {
        JSXElementName::IdentifierReference(root) => Some((root, Vec::new())),
        JSXElementName::MemberExpression(member) => member_tag(member),
        JSXElementName::Identifier(_)
        | JSXElementName::NamespacedName(_)
        | JSXElementName::ThisExpression(_) => None,
    }
}

fn member_tag<'t, 'a>(
    member: &'t JSXMemberExpression<'a>,
) -> Option<(&'t IdentifierReference<'a>, Vec<String>)> {
    let (root, mut members) = match &member.object {
        JSXMemberExpressionObject::IdentifierReference(root) => (&**root, Vec::new()),
        JSXMemberExpressionObject::MemberExpression(inner) => member_tag(inner)?,
        JSXMemberExpressionObject::ThisExpression(_) => return None,
    };
    members.push(member.property.name.to_string());
    Some((root, members))
}

fn binding(
    scoping: &Scoping,
    root: &IdentifierReference,
    import_at: &HashMap<u32, usize>,
) -> Binding {
    let Some(symbol) = scoping.get_reference(root.reference_id()).symbol_id() else {
        return Binding::Global;
    };
    match import_at.get(&scoping.symbol_span(symbol).start) {
        Some(&index) => Binding::Import(index),
        None => Binding::Local,
    }
}
