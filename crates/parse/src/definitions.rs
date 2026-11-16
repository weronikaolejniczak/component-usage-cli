use std::collections::{HashMap, HashSet};

use oxc_ast::ast::{
    Argument, BindingPattern, CallExpression, Expression, ForStatementLeft, Function, Program,
    SimpleAssignmentTarget, UnaryExpression, VariableDeclarator,
};
use oxc_ast_visit::{Visit, walk};
use oxc_semantic::{Scoping, SymbolFlags, SymbolId};
use oxc_span::GetSpan;
use oxc_syntax::operator::UnaryOperator;
use oxc_syntax::scope::ScopeFlags;

use crate::attributes::Exprs;
use crate::lines::LineIndex;
use crate::{Definition, Expr, ExprKind};

/// Array methods that change the array they're called on.
const MUTATING_METHODS: &[&str] = &[
    "copyWithin",
    "fill",
    "pop",
    "push",
    "reverse",
    "shift",
    "sort",
    "splice",
    "unshift",
];

/// `Object` functions that change their first argument.
const MUTATING_OBJECT_FUNCTIONS: &[&str] = &[
    "assign",
    "defineProperties",
    "defineProperty",
    "setPrototypeOf",
];

/// Defines every name [`Exprs`] has referred to, including names the definitions refer to. Each
/// round walks the program for the names added by the round before.
pub(crate) fn definitions(
    program: &Program,
    exprs: &Exprs,
    scoping: &Scoping,
    lines: &LineIndex,
) -> Vec<Definition> {
    let mut changed = Changed {
        scoping,
        symbols: HashSet::new(),
    };
    changed.visit_program(program);

    let mut definitions = Vec::new();
    loop {
        let symbols = exprs.variables.borrow().symbols[definitions.len()..].to_vec();
        if symbols.is_empty() {
            return definitions;
        }
        let mut round = Round {
            exprs,
            lines,
            wanted: symbols.iter().copied().collect(),
            found: HashMap::new(),
            in_loop_head: false,
        };
        round.visit_program(program);
        for symbol in symbols {
            let flags = scoping.symbol_flags(symbol);
            let redeclared = flags.contains(SymbolFlags::FunctionScopedVariable)
                && scoping.symbol_declarations(symbol).count() > 1;
            let definition = if scoping.symbol_is_mutated(symbol)
                || redeclared
                || changed.symbols.contains(&symbol)
            {
                Definition::Mutated
            } else {
                round
                    .found
                    .remove(&symbol)
                    .unwrap_or(Definition::NotFollowed)
            };
            definitions.push(definition);
        }
    }
}

struct Round<'e, 's> {
    exprs: &'e Exprs<'s>,
    lines: &'e LineIndex<'s>,
    wanted: HashSet<SymbolId>,
    found: HashMap<SymbolId, Definition>,
    in_loop_head: bool,
}

impl<'a> Visit<'a> for Round<'_, '_> {
    fn visit_for_statement_left(&mut self, left: &ForStatementLeft<'a>) {
        let outer = std::mem::replace(&mut self.in_loop_head, true);
        walk::walk_for_statement_left(self, left);
        self.in_loop_head = outer;
    }

    fn visit_variable_declarator(&mut self, declarator: &VariableDeclarator<'a>) {
        if !self.in_loop_head {
            let mut names = Vec::new();
            leaves(&declarator.id, &mut Vec::new(), None, &mut names);
            let mut init: Option<Expr> = None;
            for (symbol, path, default) in names {
                if !self.wanted.contains(&symbol) {
                    continue;
                }
                let init = init
                    .get_or_insert_with(|| match &declarator.init {
                        Some(init) => self.exprs.expr(init),
                        None => Expr {
                            location: self.lines.location(declarator.id.span().start),
                            kind: ExprKind::Undefined,
                        },
                    })
                    .clone();
                let default = default.map(|default| self.exprs.expr(default));
                self.found.insert(
                    symbol,
                    Definition::Value {
                        init,
                        path,
                        default,
                    },
                );
            }
        }
        walk::walk_variable_declarator(self, declarator);
    }

    fn visit_function(&mut self, function: &Function<'a>, flags: ScopeFlags) {
        if let Some(id) = &function.id
            && function.body.is_some()
            && self.wanted.contains(&id.symbol_id())
        {
            self.found.insert(
                id.symbol_id(),
                Definition::Function(self.lines.location(function.span.start)),
            );
        }
        walk::walk_function(self, function, flags);
    }
}

/// The names a pattern declares, each with the keys that lead to it and its default. Names
/// behind a rest element or a default on an inner pattern are left out.
fn leaves<'p, 'a>(
    pattern: &'p BindingPattern<'a>,
    path: &mut Vec<String>,
    default: Option<&'p Expression<'a>>,
    names: &mut Vec<(SymbolId, Vec<String>, Option<&'p Expression<'a>>)>,
) {
    match pattern {
        BindingPattern::BindingIdentifier(identifier) => {
            names.push((identifier.symbol_id(), path.clone(), default));
        }
        _ if default.is_some() => {}
        BindingPattern::AssignmentPattern(assignment) => {
            leaves(&assignment.left, path, Some(&assignment.right), names);
        }
        BindingPattern::ObjectPattern(object) => {
            for property in &object.properties {
                let key = property.key.static_name().filter(|_| !property.computed);
                let Some(key) = key.as_ref().and_then(|key| key.as_str()) else {
                    continue;
                };
                path.push(key.to_owned());
                leaves(&property.value, path, None, names);
                path.pop();
            }
        }
        BindingPattern::ArrayPattern(array) => {
            for (index, element) in array.elements.iter().enumerate() {
                if let Some(element) = element {
                    path.push(index.to_string());
                    leaves(element, path, None, names);
                    path.pop();
                }
            }
        }
    }
}

/// Names whose contents are changed in place: `a.b = c`, `a.b++`, `delete a.b`, `a.push(b)`,
/// `Object.assign(a, b)`.
struct Changed<'s> {
    scoping: &'s Scoping,
    symbols: HashSet<SymbolId>,
}

impl<'a> Visit<'a> for Changed<'_> {
    fn visit_simple_assignment_target(&mut self, target: &SimpleAssignmentTarget<'a>) {
        if let Some(member) = target.as_member_expression() {
            self.mark(member.object());
        }
        walk::walk_simple_assignment_target(self, target);
    }

    fn visit_unary_expression(&mut self, unary: &UnaryExpression<'a>) {
        if unary.operator == UnaryOperator::Delete
            && let Some(member) = unary.argument.get_inner_expression().as_member_expression()
        {
            self.mark(member.object());
        }
        walk::walk_unary_expression(self, unary);
    }

    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        if let Some(member) = call.callee.get_inner_expression().as_member_expression()
            && let Some(name) = member.static_property_name()
            && let Some(method) = name.as_str()
        {
            let object = member.object().get_inner_expression();
            if MUTATING_METHODS.contains(&method) {
                self.mark(object);
            } else if MUTATING_OBJECT_FUNCTIONS.contains(&method)
                && let Expression::Identifier(callee) = object
                && callee.name == "Object"
                && let Some(first) = call.arguments.first().and_then(Argument::as_expression)
            {
                self.mark(first);
            }
        }
        walk::walk_call_expression(self, call);
    }
}

impl Changed<'_> {
    /// Marks the name a member access starts with.
    fn mark(&mut self, object: &Expression) {
        let mut object = object.get_inner_expression();
        while let Some(member) = object.as_member_expression() {
            object = member.object().get_inner_expression();
        }
        if let Expression::Identifier(identifier) = object
            && let Some(symbol) = self
                .scoping
                .get_reference(identifier.reference_id())
                .symbol_id()
        {
            self.symbols.insert(symbol);
        }
    }
}
