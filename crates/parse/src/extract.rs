use std::cell::OnceCell;
use std::collections::{HashMap, HashSet};
use std::path::Path;

use oxc_allocator::Allocator;
use oxc_ast::ast::Program;
use oxc_diagnostics::OxcDiagnostic;
use oxc_parser::Parser;
use oxc_semantic::{SemanticBuilder, SymbolId};
use oxc_span::SourceType;
use oxc_syntax::module_record::{
    ExportEntry, ExportExportName, ExportImportName, ImportEntry, ImportImportName,
};

use crate::aliases::alias;
use crate::attributes::Exprs;
use crate::callbacks::callbacks;
use crate::classes::{self, ClassMember, classes};
use crate::components::{components, spreads};
use crate::definitions::{Mode, changed, definitions, mutated};
use crate::elements::elements;
use crate::exported::exported;
use crate::lines::LineIndex;
use crate::references::references;
use crate::scope::{Declared, Scope};
use crate::{
    Alias, Binding, CallSite, Definition, Export, Expr, ExprKind, FileSummary, Import, Imported,
    Item, JsxElement, Member, Options, ParseError, SyntaxError,
};

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
    let mut exprs = Exprs::new(source, &scope);
    exprs.calls = options.calls;
    let changed = OnceCell::new();
    let changed = || changed.get_or_init(|| self::changed(&parsed.program, &scope));
    let follow = options.variables || options.imports || options.props;
    let mut classes = classes(&parsed.program, source);
    exprs.this = std::mem::take(&mut classes.this);
    let mut members = Vec::new();
    let mut spans = HashSet::new();
    for (class, component) in classes.components.iter().enumerate() {
        let names = component
            .members
            .iter()
            .map(|(name, member)| (name, Some(member)))
            .chain(component.assigned.iter().map(|name| (name, None)));
        for (name, member) in names {
            let key = (class, name.clone());
            if exprs.members.contains_key(&key) {
                continue;
            }
            exprs.members.insert(key, members.len());
            let assigned = component.assigned.contains(name);
            if let (Some(ClassMember::Field(span)), true, false) = (member, follow, assigned) {
                spans.insert(*span);
            }
            members.push((member, assigned));
        }
        spans.extend(component.default_props.filter(|_| follow));
    }
    if follow {
        spans.extend(classes.default_props.iter().map(|&(_, span)| span));
    }
    let found = (options.props || options.wrappers).then(|| {
        components(
            &parsed.program,
            &exprs,
            &classes,
            options.wrappers,
            changed(),
        )
    });
    let callbacks = if follow {
        callbacks(&parsed.program, &exprs, changed())
    } else {
        Vec::new()
    };
    let called = found
        .as_ref()
        .filter(|_| options.props)
        .map_or(&[][..], |found| found.calls.as_slice());
    spans.extend(
        called
            .iter()
            .flat_map(|(_, call)| call.arguments.iter().map(|&(_, span)| span)),
    );
    let mut values = classes::values(&parsed.program, &exprs, spans);
    let calls = called
        .iter()
        .map(|(callee, call)| CallSite {
            callee: *callee,
            text: call.text.clone(),
            location: lines.location(call.start),
            arguments: call
                .arguments
                .iter()
                .map(|(spread, span)| {
                    let expr = values.get(span).cloned().unwrap_or_else(|| Expr {
                        location: lines.location(span.start),
                        kind: ExprKind::Unsupported {
                            text: source[span.start as usize..span.end as usize].to_owned(),
                        },
                    });
                    if *spread {
                        Item::Spread(expr)
                    } else {
                        Item::Value(expr)
                    }
                })
                .collect(),
        })
        .collect();
    let members = members
        .into_iter()
        .map(|(member, assigned)| match member {
            _ if assigned => Member::Mutated,
            _ if !follow => Member::NotFollowed,
            Some(ClassMember::Field(span)) => values
                .get(span)
                .cloned()
                .map_or(Member::NotFollowed, Member::Value),
            Some(ClassMember::Method(start)) => Member::Method(lines.location(*start)),
            None => Member::Mutated,
        })
        .collect();
    let mut elements = elements(&parsed.program, &exprs);
    if let Some(found) = &found {
        for (element, local) in &mut elements {
            if element.members.is_empty() {
                element.component = local.and_then(|symbol| found.by_symbol.get(&symbol).copied());
            }
        }
    }
    let (elements, aliases) = bind_aliases(&scope, source, elements, &parsed.program, changed);
    let exported = if options.imports || options.props || options.wrappers {
        exported(&parsed.program, &exprs)
    } else {
        HashMap::new()
    };
    let none = HashSet::new();
    let changed = if follow { changed() } else { &none };
    let definitions = if follow {
        definitions(&parsed.program, &exprs, Mode::Values, changed)
    } else {
        vec![Definition::NotFollowed; exprs.variables.borrow().symbols.len()]
    };
    let mut changed_imports: Vec<usize> = if options.imports || options.props {
        changed
            .iter()
            .filter_map(|&symbol| match scope.declared(symbol) {
                Declared::Import(import) => Some(import),
                Declared::Parameter | Declared::Local => None,
            })
            .collect()
    } else {
        Vec::new()
    };
    changed_imports.sort_unstable();
    let (mut components, parameters, escaping_imports, by_symbol) =
        found.map_or_else(Default::default, |found| {
            (
                found.components,
                found.parameters,
                found.escaping_imports,
                found.by_symbol,
            )
        });
    for (class, component) in classes.components.iter().enumerate() {
        let index = exprs
            .class_props
            .borrow()
            .get(&class)
            .map(|&parameter| parameters[parameter].component);
        if let (Some(index), Some(span)) = (index, component.default_props) {
            components[index].default_props = values.remove(&span);
        }
    }
    for (reference, span) in &classes.default_props {
        let index = scope
            .scoping
            .get_reference(*reference)
            .symbol_id()
            .and_then(|symbol| by_symbol.get(&symbol));
        if let Some(&index) = index {
            components[index].default_props = values.remove(span);
        }
    }
    if options.wrappers {
        spreads(&mut components, &parameters, &elements);
    }
    let references = references(&parsed.program, &scope, source);

    Ok(FileSummary {
        imports,
        exports,
        elements,
        aliases,
        definitions,
        exported,
        changed_imports,
        components,
        parameters,
        callbacks,
        members,
        calls,
        escaping_imports,
        references,
        errors,
    })
}

/// Sets [`Binding::Alias`] on elements whose tag starts with a local name that refers to an
/// import or `require`. Tags are followed by their declarations, separately from attribute
/// values and whatever `--follow` says: a tag that may be a component is always reported.
fn bind_aliases<'c>(
    scope: &Scope,
    source: &str,
    elements: Vec<(JsxElement, Option<SymbolId>)>,
    program: &Program,
    changed: impl Fn() -> &'c HashSet<SymbolId>,
) -> (Vec<JsxElement>, Vec<Alias>) {
    let tags = Exprs::new(source, scope);
    let locals: Vec<Option<usize>> = elements
        .iter()
        .map(|(_, local)| local.map(|symbol| tags.variable(symbol)))
        .collect();
    let definitions = definitions(program, &tags, Mode::Tags, &HashSet::new());
    let reassigned = |index: usize| {
        let symbol = tags.variables.borrow().symbols[index];
        mutated(scope.scoping, changed(), symbol)
    };
    let mut aliases = Vec::new();
    let mut alias_of: HashMap<usize, Option<usize>> = HashMap::new();
    let elements = elements
        .into_iter()
        .zip(locals)
        .map(|((mut element, _), local)| {
            if let Some(local) = local {
                let alias = *alias_of.entry(local).or_insert_with(|| {
                    let alias = alias(&definitions, local, reassigned)?;
                    aliases.push(alias);
                    Some(aliases.len() - 1)
                });
                if let Some(alias) = alias {
                    element.binding = Binding::Alias(alias);
                }
            }
            element
        })
        .collect();
    (elements, aliases)
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
