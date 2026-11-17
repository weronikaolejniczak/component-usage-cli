use std::collections::{HashMap, HashSet};

use oxc_ast::ast::{
    Argument, CallExpression, Expression, ForStatementLeft, Function, Program,
    SimpleAssignmentTarget, UnaryExpression, VariableDeclaration, VariableDeclarator,
};
use oxc_ast_visit::{Visit, walk};
use oxc_semantic::{SymbolFlags, SymbolId};
use oxc_span::GetSpan;
use oxc_syntax::operator::UnaryOperator;
use oxc_syntax::scope::ScopeFlags;

use crate::attributes::Exprs;
use crate::patterns::leaves;
use crate::scope::Scope;
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

/// What definitions are for.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mode {
    /// Prop values: names changed after their declaration are [`Definition::Mutated`], and names
    /// behind a default on an inner pattern aren't followed.
    Values,
    /// JSX tags, which are reported when they may be a component: every name is defined by its
    /// declaration, whatever happens to it later or on the way.
    Tags,
}

/// Defines every name `exprs` has referred to, including names the definitions refer to. Each
/// round walks the program for the names added by the round before.
/// `changed` are the names changed in place, from [`changed`], for [`Mode::Values`].
pub(crate) fn definitions(
    program: &Program,
    exprs: &Exprs,
    mode: Mode,
    changed: &HashSet<SymbolId>,
) -> Vec<Definition> {
    let mutations = mode == Mode::Values;
    let scoping = exprs.scope.scoping;
    let mut definitions = Vec::new();
    loop {
        let symbols = exprs.variables.borrow().symbols[definitions.len()..].to_vec();
        if symbols.is_empty() {
            return definitions;
        }
        let mut round = Round {
            exprs,
            mode,
            wanted: symbols.iter().copied().collect(),
            found: HashMap::new(),
            in_loop_head: false,
            in_declare: false,
        };
        round.visit_program(program);
        for symbol in symbols {
            let flags = scoping.symbol_flags(symbol);
            let redeclared = flags.contains(SymbolFlags::FunctionScopedVariable)
                && scoping.symbol_declarations(symbol).count() > 1;
            let mutated =
                scoping.symbol_is_mutated(symbol) || redeclared || changed.contains(&symbol);
            let definition = if mutations && mutated {
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
    mode: Mode,
    wanted: HashSet<SymbolId>,
    found: HashMap<SymbolId, Definition>,
    in_loop_head: bool,
    in_declare: bool,
}

impl<'a> Visit<'a> for Round<'_, '_> {
    fn visit_variable_declaration(&mut self, declaration: &VariableDeclaration<'a>) {
        let outer = std::mem::replace(&mut self.in_declare, declaration.declare);
        walk::walk_variable_declaration(self, declaration);
        self.in_declare = outer;
    }

    fn visit_for_statement_left(&mut self, left: &ForStatementLeft<'a>) {
        let outer = std::mem::replace(&mut self.in_loop_head, true);
        walk::walk_for_statement_left(self, left);
        self.in_loop_head = outer;
    }

    fn visit_variable_declarator(&mut self, declarator: &VariableDeclarator<'a>) {
        if !self.in_loop_head {
            let mut init: Option<Expr> = None;
            for leaf in leaves(&declarator.id) {
                if (leaf.inner_default && self.mode == Mode::Values)
                    || !self.wanted.contains(&leaf.symbol)
                {
                    continue;
                }
                if self.in_declare {
                    self.found.insert(leaf.symbol, Definition::Declared);
                    continue;
                }
                let init = init
                    .get_or_insert_with(|| match &declarator.init {
                        Some(init) => self.exprs.expr(init),
                        None => Expr {
                            location: self.exprs.scope.lines.location(declarator.id.span().start),
                            kind: ExprKind::Undefined,
                        },
                    })
                    .clone();
                let path = leaf.path;
                let default = leaf.default.map(|default| self.exprs.expr(default));
                self.found.insert(
                    leaf.symbol,
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
            && self.wanted.contains(&id.symbol_id())
        {
            if function.body.is_some() {
                self.found.insert(
                    id.symbol_id(),
                    Definition::Function {
                        location: self.exprs.scope.lines.location(function.span.start),
                        returns: self.exprs.function_returns(function),
                    },
                );
            } else if function.declare {
                self.found.insert(id.symbol_id(), Definition::Declared);
            }
        }
        walk::walk_function(self, function, flags);
    }
}

/// Names whose contents are changed in place: `a.b = c`, `a.b++`, `delete a.b`, `a.push(b)`,
/// `Object.assign(a, b)`.
pub(crate) fn changed(program: &Program, scope: &Scope) -> HashSet<SymbolId> {
    let mut changed = Changed {
        scope,
        symbols: HashSet::new(),
    };
    changed.visit_program(program);
    changed.symbols
}

struct Changed<'s> {
    scope: &'s Scope<'s>,
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
            && let Some(symbol) = self.scope.symbol(identifier)
        {
            self.symbols.insert(symbol);
        }
    }
}
