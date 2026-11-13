use std::collections::HashMap;
use std::path::Path;

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    Argument, BindingPattern, Expression, IdentifierReference, JSXElementName, JSXMemberExpression,
    JSXMemberExpressionObject, JSXOpeningElement, VariableDeclarator,
};
use oxc_ast_visit::{Visit, walk};
use oxc_diagnostics::OxcDiagnostic;
use oxc_parser::Parser;
use oxc_semantic::{Scoping, SemanticBuilder, SymbolId};
use oxc_span::SourceType;
use oxc_syntax::module_record::{
    ExportEntry, ExportExportName, ExportImportName, ImportEntry, ImportImportName,
};

use crate::lines::LineIndex;
use crate::{
    Alias, AliasRoot, Binding, Export, FileSummary, Import, Imported, JsxElement, ParseError,
    SyntaxError,
};

pub(crate) fn extract(path: &Path, source: &str) -> Result<FileSummary, ParseError> {
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
    let mut constants = Constants {
        scoping: semantic.scoping(),
        paths: HashMap::new(),
    };
    constants.visit_program(&parsed.program);
    let mut collector = Elements {
        scoping: semantic.scoping(),
        lines: &lines,
        import_at: &import_at,
        constants: &constants.paths,
        alias_of: HashMap::new(),
        aliases: Vec::new(),
        elements: Vec::new(),
    };
    collector.visit_program(&parsed.program);
    let mut elements = collector.elements;
    elements.sort_by_key(|element| element.location);

    Ok(FileSummary {
        imports,
        exports,
        elements,
        aliases: collector.aliases,
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

/// What a constant's initialiser refers to: `Lib.Icon` is `Lib` with members `["Icon"]`.
struct Access {
    root: AccessRoot,
    members: Vec<String>,
}

enum AccessRoot {
    Symbol(SymbolId),
    Require(String),
}

/// Variable declarations whose initialiser is a binding, a `require` call or a static property
/// chain on one, keyed by the declared symbol. Destructured names extend the path.
struct Constants<'s> {
    scoping: &'s Scoping,
    paths: HashMap<SymbolId, Access>,
}

impl<'a> Visit<'a> for Constants<'_> {
    fn visit_variable_declarator(&mut self, declarator: &VariableDeclarator<'a>) {
        if let Some(path) = declarator
            .init
            .as_ref()
            .and_then(|init| init_path(self.scoping, init))
        {
            self.bind(&declarator.id, path);
        }
        walk::walk_variable_declarator(self, declarator);
    }
}

impl Constants<'_> {
    fn bind(&mut self, pattern: &BindingPattern, path: Access) {
        match pattern {
            BindingPattern::BindingIdentifier(identifier) => {
                self.paths.insert(identifier.symbol_id(), path);
            }
            BindingPattern::ObjectPattern(object) => {
                for property in &object.properties {
                    let name = property.key.static_name().filter(|_| !property.computed);
                    let Some(name) = name.as_ref().and_then(|name| name.as_str()) else {
                        continue;
                    };
                    let root = match &path.root {
                        AccessRoot::Symbol(symbol) => AccessRoot::Symbol(*symbol),
                        AccessRoot::Require(specifier) => AccessRoot::Require(specifier.clone()),
                    };
                    let mut members = path.members.clone();
                    members.push(name.to_owned());
                    self.bind(&property.value, Access { root, members });
                }
            }
            BindingPattern::AssignmentPattern(assignment) => self.bind(&assignment.left, path),
            BindingPattern::ArrayPattern(_) => {}
        }
    }
}

fn init_path(scoping: &Scoping, init: &Expression) -> Option<Access> {
    match init.get_inner_expression() {
        Expression::Identifier(identifier) => Some(Access {
            root: AccessRoot::Symbol(symbol(scoping, identifier)?),
            members: Vec::new(),
        }),
        Expression::StaticMemberExpression(member) => {
            let mut path = init_path(scoping, &member.object)?;
            path.members.push(member.property.name.to_string());
            Some(path)
        }
        Expression::CallExpression(call) => {
            let Expression::Identifier(callee) = call.callee.get_inner_expression() else {
                return None;
            };
            let [Argument::StringLiteral(specifier)] = call.arguments.as_slice() else {
                return None;
            };
            let specifier = specifier.value.as_str()?;
            (callee.name == "require" && symbol(scoping, callee).is_none()).then(|| Access {
                root: AccessRoot::Require(specifier.to_owned()),
                members: Vec::new(),
            })
        }
        _ => None,
    }
}

fn symbol(scoping: &Scoping, identifier: &IdentifierReference) -> Option<SymbolId> {
    scoping.get_reference(identifier.reference_id()).symbol_id()
}

struct Elements<'s> {
    scoping: &'s Scoping,
    lines: &'s LineIndex<'s>,
    import_at: &'s HashMap<u32, usize>,
    constants: &'s HashMap<SymbolId, Access>,
    alias_of: HashMap<SymbolId, usize>,
    aliases: Vec<Alias>,
    elements: Vec<JsxElement>,
}

impl<'a> Visit<'a> for Elements<'_> {
    fn visit_jsx_opening_element(&mut self, element: &JSXOpeningElement<'a>) {
        if let Some((root, members)) = tag(&element.name) {
            let binding = self.binding(root);
            self.elements.push(JsxElement {
                location: self.lines.location(element.span.start),
                root: root.name.to_string(),
                members,
                binding,
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

impl Elements<'_> {
    fn binding(&mut self, root: &IdentifierReference) -> Binding {
        let Some(symbol) = symbol(self.scoping, root) else {
            return Binding::Global;
        };
        if let Some(&import) = self.import(symbol) {
            return Binding::Import(import);
        }
        if let Some(&alias) = self.alias_of.get(&symbol) {
            return Binding::Alias(alias);
        }
        let Some(alias) = self.alias(symbol) else {
            return Binding::Local;
        };
        self.aliases.push(alias);
        self.alias_of.insert(symbol, self.aliases.len() - 1);
        Binding::Alias(self.aliases.len() - 1)
    }

    fn import(&self, symbol: SymbolId) -> Option<&usize> {
        self.import_at.get(&self.scoping.symbol_span(symbol).start)
    }

    /// Follows constants back to an import or `require`. Constants that refer to each other in
    /// a cycle, or to anything else, aren't aliases.
    fn alias(&self, mut symbol: SymbolId) -> Option<Alias> {
        let mut members = Vec::new();
        let mut seen = Vec::new();
        loop {
            if seen.contains(&symbol) {
                return None;
            }
            seen.push(symbol);
            let path = self.constants.get(&symbol)?;
            members.splice(0..0, path.members.iter().cloned());
            match &path.root {
                AccessRoot::Require(specifier) => {
                    return Some(Alias {
                        root: AliasRoot::Require(specifier.clone()),
                        members,
                    });
                }
                AccessRoot::Symbol(next) => {
                    if let Some(&import) = self.import(*next) {
                        return Some(Alias {
                            root: AliasRoot::Import(import),
                            members,
                        });
                    }
                    symbol = *next;
                }
            }
        }
    }
}
