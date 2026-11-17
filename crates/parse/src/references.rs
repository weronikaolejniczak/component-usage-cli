use model::Passed;
use oxc_ast::AstKind;
use oxc_ast::ast::{BindingPattern, LogicalOperator};
use oxc_semantic::{AstNodes, NodeId};
use oxc_span::{GetSpan, Span};

use crate::ValueReference;
use crate::scope::{Declared, Scope};

/// The imports read as values, with where each value goes, in source order.
pub(crate) fn references(nodes: &AstNodes, scope: &Scope, source: &str) -> Vec<ValueReference> {
    let scoping = scope.scoping;
    let mut references = Vec::new();
    for symbol in scoping.symbol_ids() {
        let Declared::Import(import) = scope.declared(symbol) else {
            continue;
        };
        for &id in scoping.get_resolved_reference_ids(symbol) {
            let reference = scoping.get_reference(id);
            if !reference.is_value() {
                continue;
            }
            let node = reference.node_id();
            let location = scope.lines.location(nodes.kind(node).span().start);
            let (node, members) = members(nodes, node);
            if let Some(passed) = passed(nodes, value(nodes, node), source) {
                references.push(ValueReference {
                    import,
                    members,
                    location,
                    passed,
                });
            }
        }
    }
    references.sort_by_key(|reference| reference.location);
    references
}

/// The outermost of the static property accesses on `node`, through `?.`, and their names.
fn members(nodes: &AstNodes, mut node: NodeId) -> (NodeId, Vec<String>) {
    let mut members = Vec::new();
    loop {
        let span = nodes.kind(node).span();
        let parent = nodes.parent_id(node);
        match nodes.kind(parent) {
            AstKind::StaticMemberExpression(member) if member.object.span() == span => {
                members.push(member.property.name.to_string());
                node = parent;
            }
            AstKind::ComputedMemberExpression(member) if member.object.span() == span => {
                let Some(name) = member.static_property_name().and_then(|name| name.as_str())
                else {
                    return (node, members);
                };
                members.push(name.to_string());
                node = parent;
            }
            AstKind::ChainExpression(_) => node = parent,
            _ => return (node, members),
        }
    }
}

/// The outermost expression whose value `node`'s can be: through parentheses, type assertions,
/// the branches of `?:`, either side of `||` and `??`, and the right side of `&&`.
fn value(nodes: &AstNodes, mut node: NodeId) -> NodeId {
    loop {
        let span = nodes.kind(node).span();
        let parent = nodes.parent_id(node);
        let through = match nodes.kind(parent) {
            AstKind::ParenthesizedExpression(_)
            | AstKind::TSAsExpression(_)
            | AstKind::TSSatisfiesExpression(_)
            | AstKind::TSNonNullExpression(_)
            | AstKind::TSTypeAssertion(_) => true,
            AstKind::ConditionalExpression(conditional) => conditional.test.span() != span,
            AstKind::LogicalExpression(logical) => {
                logical.operator != LogicalOperator::And || logical.right.span() == span
            }
            _ => false,
        };
        if !through {
            return node;
        }
        node = parent;
    }
}

/// Where the value of `node` goes; `None` when it isn't a reference.
fn passed(nodes: &AstNodes, node: NodeId, source: &str) -> Option<Passed> {
    let span = nodes.kind(node).span();
    let text = |span: Span| source[span.start as usize..span.end as usize].to_owned();
    let parent = nodes.parent_id(node);
    Some(match nodes.kind(parent) {
        AstKind::JSXOpeningElement(_)
        | AstKind::JSXClosingElement(_)
        | AstKind::JSXMemberExpression(_)
        | AstKind::ExportSpecifier(_)
        | AstKind::ExportDefaultDeclaration(_)
        | AstKind::TSTypeQuery(_)
        | AstKind::TSQualifiedName(_) => return None,
        AstKind::StaticMemberExpression(member) if member.object.span() == span => return None,
        AstKind::ComputedMemberExpression(member) if member.object.span() == span => return None,
        AstKind::JSXExpressionContainer(_) => {
            let attribute = nodes.parent_id(parent);
            match nodes.kind(attribute) {
                AstKind::JSXAttribute(attribute_) => {
                    let element = nodes.parent_id(attribute);
                    match nodes.kind(element) {
                        AstKind::JSXOpeningElement(element) => Passed::Attribute {
                            element: text(element.name.span()),
                            name: text(attribute_.name.span()),
                        },
                        _ => Passed::Other,
                    }
                }
                _ => Passed::Other,
            }
        }
        AstKind::ObjectProperty(property) if property.value.span() == span => {
            let key = property.key.static_name().filter(|_| !property.computed);
            match key.as_ref().and_then(|key| key.as_str()) {
                Some(key) => Passed::Property {
                    key: Some(key.to_owned()),
                    computed: None,
                },
                None => Passed::Property {
                    key: None,
                    computed: Some(text(property.key.span())),
                },
            }
        }
        AstKind::ArrayExpression(array) => {
            match array.elements.iter().position(|item| item.span() == span) {
                Some(index) => Passed::Item { index },
                None => Passed::Other,
            }
        }
        AstKind::CallExpression(call) if call.callee.span() == span => return None,
        AstKind::NewExpression(call) if call.callee.span() == span => return None,
        AstKind::CallExpression(call) => {
            argument(&call.arguments, span).map_or(Passed::Other, |index| Passed::Argument {
                callee: text(call.callee.span()),
                index,
            })
        }
        AstKind::NewExpression(call) => {
            argument(&call.arguments, span).map_or(Passed::Other, |index| Passed::Argument {
                callee: text(call.callee.span()),
                index,
            })
        }
        AstKind::VariableDeclarator(declarator)
            if declarator
                .init
                .as_ref()
                .is_some_and(|init| init.span() == span) =>
        {
            match &declarator.id {
                BindingPattern::BindingIdentifier(id) => Passed::Variable {
                    name: id.name.to_string(),
                },
                _ => Passed::Other,
            }
        }
        _ => Passed::Other,
    })
}

fn argument(arguments: &[oxc_ast::ast::Argument], span: Span) -> Option<usize> {
    arguments
        .iter()
        .position(|argument| argument.as_expression().is_some_and(|e| e.span() == span))
}
