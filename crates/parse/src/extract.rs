use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;

use oxc_allocator::Allocator;
use oxc_diagnostics::OxcDiagnostic;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use oxc_syntax::module_record::{
    ExportEntry, ExportExportName, ExportImportName, ImportEntry, ImportImportName,
};

use crate::aliases::Aliases;
use crate::attributes::{Exprs, Variables};
use crate::definitions::definitions;
use crate::elements::elements;
use crate::lines::LineIndex;
use crate::scope::Scope;
use crate::{Definition, Export, FileSummary, Import, Imported, Options, ParseError, SyntaxError};

pub(crate) fn extract(
    path: &Path,
    source: &str,
    options: Options,
) -> Result<FileSummary, ParseError> {
    let source_type = SourceType::from_path(path).map_err(|_| ParseError::UnsupportedExtension)?;
    let source_type = source_type.with_jsx(source_type.is_jsx() || source_type.is_javascript());
    let lines = LineIndex::new(source);
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, source_type).parse();

    let to_error = |error: &OxcDiagnostic| SyntaxError {
        location: lines.location(error.labels.first().map_or(0, |label| label.offset())),
        message: error.message.to_string(),
    };
    let mut errors: Vec<SyntaxError> = parsed.diagnostics.errors().map(to_error).collect();
    if parsed.fatal_error {
        // Oxc appends the error it couldn't recover from after the ones it could.
        let error = errors.pop().unwrap_or(SyntaxError {
            location: lines.location(0),
            message: "unrecoverable syntax error".to_owned(),
        });
        return Err(ParseError::Syntax(error));
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

    let built = SemanticBuilder::new().build(&parsed.program);
    errors.extend(built.diagnostics.errors().map(to_error));
    errors.sort_by_key(|error| error.location);
    let semantic = built.semantic;
    let scope = Scope::new(&parsed.program, semantic.scoping(), &lines, import_at);
    let aliases = Aliases::new(&parsed.program, &scope);
    let exprs = Exprs {
        source,
        scope: &scope,
        variables: RefCell::new(Variables::default()),
    };
    let (elements, aliases) = elements(&parsed.program, &exprs, &aliases);
    let definitions = if options.variables {
        definitions(&parsed.program, &exprs)
    } else {
        vec![Definition::NotFollowed; exprs.variables.borrow().symbols.len()]
    };

    Ok(FileSummary {
        imports,
        exports,
        elements,
        aliases,
        definitions,
        errors,
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
