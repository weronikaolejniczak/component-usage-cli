use std::collections::HashSet;

use oxc_ast::ast::{CallExpression, Expression, FormalParameters, Program};
use oxc_ast_visit::{Visit, walk};
use oxc_semantic::SymbolId;

use crate::CallbackParameter;
use crate::attributes::Exprs;
use crate::definitions::mutated;
use crate::patterns::leaves;

/// Array methods that call their first argument with each item first.
const ITEM_METHODS: &[&str] = &[
    "every",
    "filter",
    "find",
    "findIndex",
    "findLast",
    "findLastIndex",
    "flatMap",
    "forEach",
    "map",
    "some",
];

/// The names the first parameter of a callback of [`ITEM_METHODS`] declares. Sets
/// [`Exprs::callbacks`], so expressions built afterwards refer to them by
/// [`crate::Reference::Callback`] index.
pub(crate) fn callbacks(
    program: &Program,
    exprs: &Exprs,
    changed: &HashSet<SymbolId>,
) -> Vec<CallbackParameter> {
    let mut collector = Collector {
        exprs,
        changed,
        found: Vec::new(),
    };
    collector.visit_program(program);
    collector.found
}

struct Collector<'e, 's, 'c> {
    exprs: &'e Exprs<'s>,
    changed: &'c HashSet<SymbolId>,
    found: Vec<CallbackParameter>,
}

impl<'a> Visit<'a> for Collector<'_, '_, '_> {
    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        if let Some((receiver, params)) = item_callback(call) {
            self.add(receiver, params);
        }
        walk::walk_call_expression(self, call);
    }
}

impl Collector<'_, '_, '_> {
    fn add(&mut self, receiver: &Expression, params: &FormalParameters) {
        let Some(first) = params.items.first() else {
            return;
        };
        let leaves: Vec<_> = leaves(&first.pattern)
            .into_iter()
            .filter(|leaf| !leaf.inner_default)
            .collect();
        if leaves.is_empty() {
            return;
        }
        let receiver = self.exprs.expr(receiver);
        let scoping = self.exprs.scope.scoping;
        for leaf in leaves {
            self.exprs
                .callbacks
                .borrow_mut()
                .insert(leaf.symbol, self.found.len());
            self.found.push(CallbackParameter {
                location: self
                    .exprs
                    .scope
                    .lines
                    .location(scoping.symbol_span(leaf.symbol).start),
                receiver: receiver.clone(),
                path: leaf.path,
                default: leaf.default.map(|default| self.exprs.expr(default)),
                rest: leaf.rest,
                mutated: mutated(scoping, self.changed, leaf.symbol),
            });
        }
    }
}

/// The receiver of `call` and the parameters of its callback, when it calls one of
/// [`ITEM_METHODS`] with a function written as its first argument.
fn item_callback<'c, 'a>(
    call: &'c CallExpression<'a>,
) -> Option<(&'c Expression<'a>, &'c FormalParameters<'a>)> {
    let member = call.callee.get_inner_expression().as_member_expression()?;
    let method = member.static_property_name()?;
    if !ITEM_METHODS.contains(&method.as_str()?) {
        return None;
    }
    let params = match call
        .arguments
        .first()?
        .as_expression()?
        .get_inner_expression()
    {
        Expression::ArrowFunctionExpression(arrow) => &arrow.params,
        Expression::FunctionExpression(function) => &function.params,
        _ => return None,
    };
    Some((member.object(), params))
}
