use std::collections::HashMap;

use oxc_ast::ast::{
    Declaration, ExportDefaultDeclarationKind, ModuleExportName, Program, Statement,
};
use oxc_semantic::SymbolId;

use crate::attributes::Exprs;
use crate::patterns::leaves;
use crate::scope::Declared;
use crate::{Exported, Expr, ExprKind};

/// What each export declared in this file holds. Names exported from an import, and re-exports,
/// are left out: the module graph follows them.
pub(crate) fn exported(program: &Program, exprs: &Exprs) -> HashMap<String, Exported> {
    let scoping = exprs.scope.scoping;
    let mut exported = HashMap::new();
    let variable = |exported: &mut HashMap<String, Exported>, name: String, symbol: SymbolId| {
        exported.insert(name, Exported::Variable(exprs.variable(symbol)));
    };
    for statement in &program.body {
        match statement {
            Statement::ExportDeclaration(export) => match &export.declaration {
                Declaration::VariableDeclaration(declaration) => {
                    for declarator in &declaration.declarations {
                        for leaf in leaves(&declarator.id)
                            .into_iter()
                            .filter(|leaf| leaf.rest.is_none())
                        {
                            variable(
                                &mut exported,
                                scoping.symbol_name(leaf.symbol).to_owned(),
                                leaf.symbol,
                            );
                        }
                    }
                }
                Declaration::FunctionDeclaration(function) => {
                    if let Some(id) = &function.id {
                        variable(&mut exported, id.name.to_string(), id.symbol_id());
                    }
                }
                Declaration::ClassDeclaration(class) => {
                    if let Some(id) = &class.id {
                        variable(&mut exported, id.name.to_string(), id.symbol_id());
                    }
                }
                Declaration::TSEnumDeclaration(enumeration) => {
                    variable(
                        &mut exported,
                        enumeration.id.name.to_string(),
                        enumeration.id.symbol_id(),
                    );
                }
                _ => {}
            },
            Statement::ExportNamedDeclaration(export) if !export.export_kind.is_type() => {
                for specifier in &export.specifiers {
                    if specifier.export_kind.is_type() {
                        continue;
                    }
                    let ModuleExportName::IdentifierReference(local) = &specifier.local else {
                        continue;
                    };
                    if let Some(symbol) = exprs.scope.symbol(local)
                        && matches!(exprs.scope.declared(symbol), Declared::Local)
                    {
                        variable(&mut exported, specifier.exported.name().to_string(), symbol);
                    }
                }
            }
            Statement::ExportDefaultDeclaration(export) => {
                let value = match &export.declaration {
                    ExportDefaultDeclarationKind::FunctionDeclaration(function) => {
                        match &function.id {
                            Some(id) => Exported::Variable(exprs.variable(id.symbol_id())),
                            None => Exported::Value(Expr {
                                location: exprs.scope.lines.location(function.span.start),
                                kind: ExprKind::Function { returns: None },
                            }),
                        }
                    }
                    ExportDefaultDeclarationKind::ClassDeclaration(class) => match &class.id {
                        Some(id) => Exported::Variable(exprs.variable(id.symbol_id())),
                        None => continue,
                    },
                    ExportDefaultDeclarationKind::TSInterfaceDeclaration(_) => continue,
                    ExportDefaultDeclarationKind::Identifier(local)
                        if let Some(symbol) = exprs.scope.symbol(local)
                            && matches!(exprs.scope.declared(symbol), Declared::Local) =>
                    {
                        Exported::Variable(exprs.variable(symbol))
                    }
                    declaration => match declaration.as_expression() {
                        Some(expression) => Exported::Value(exprs.expr(expression)),
                        None => continue,
                    },
                };
                exported.insert("default".to_owned(), value);
            }
            _ => {}
        }
    }
    exported
}
