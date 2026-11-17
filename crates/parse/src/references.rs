use model::Passed;
use oxc_ast::AstKind;
use oxc_ast::ast::{BindingPattern, IdentifierReference, LogicalOperator, Program};
use oxc_ast_visit::{Visit, walk};
use oxc_span::{GetSpan, Span};

use crate::ValueReference;
use crate::scope::{Declared, Scope};

/// The imports read as values, with where each value goes, in source order.
pub(crate) fn references(program: &Program, scope: &Scope, source: &str) -> Vec<ValueReference> {
    let mut collector = Collector {
        scope,
        source,
        ancestors: Vec::new(),
        references: Vec::new(),
    };
    collector.visit_program(program);
    collector.references
}

struct Collector<'s, 'a> {
    scope: &'s Scope<'s>,
    source: &'s str,
    /// The nodes enclosing the one being visited, outermost first.
    ancestors: Vec<AstKind<'a>>,
    references: Vec<ValueReference>,
}

impl<'a> Visit<'a> for Collector<'_, 'a> {
    fn enter_node(&mut self, kind: AstKind<'a>) {
        self.ancestors.push(kind);
    }

    fn leave_node(&mut self, _kind: AstKind<'a>) {
        self.ancestors.pop();
    }

    fn visit_identifier_reference(&mut self, identifier: &IdentifierReference<'a>) {
        let scoping = self.scope.scoping;
        if scoping.get_reference(identifier.reference_id()).is_value()
            && let Some(symbol) = self.scope.symbol(identifier)
            && let Declared::Import(import) = self.scope.declared(symbol)
        {
            let node = Node {
                depth: self.ancestors.len(),
                span: identifier.span,
            };
            let (node, members) = members(&self.ancestors, node);
            let node = value(&self.ancestors, node);
            if let Some(passed) = passed(&self.ancestors, node, self.source) {
                self.references.push(ValueReference {
                    import,
                    members,
                    location: self.scope.lines.location(identifier.span.start),
                    passed,
                });
            }
        }
        walk::walk_identifier_reference(self, identifier);
    }
}

/// A node, by its span and its number of ancestors: its parent is `ancestors[depth - 1]`.
#[derive(Clone, Copy)]
struct Node {
    depth: usize,
    span: Span,
}

impl Node {
    fn parent<'a>(self, ancestors: &[AstKind<'a>]) -> Option<AstKind<'a>> {
        ancestors.get(self.depth.checked_sub(1)?).copied()
    }

    fn up(self, parent: AstKind) -> Self {
        Self {
            depth: self.depth - 1,
            span: parent.span(),
        }
    }
}

/// The outermost of the static property accesses on `node`, through `?.`, and their names.
fn members(ancestors: &[AstKind], mut node: Node) -> (Node, Vec<String>) {
    let mut members = Vec::new();
    while let Some(parent) = node.parent(ancestors) {
        match parent {
            AstKind::StaticMemberExpression(member) if member.object.span() == node.span => {
                members.push(member.property.name.to_string());
            }
            AstKind::ComputedMemberExpression(member) if member.object.span() == node.span => {
                let Some(name) = member.static_property_name().and_then(|name| name.as_str())
                else {
                    break;
                };
                members.push(name.to_string());
            }
            AstKind::ChainExpression(_) => {}
            _ => break,
        }
        node = node.up(parent);
    }
    (node, members)
}

/// The outermost expression whose value `node`'s can be: through parentheses, type assertions,
/// the branches of `?:`, either side of `||` and `??`, and the right side of `&&`.
fn value(ancestors: &[AstKind], mut node: Node) -> Node {
    while let Some(parent) = node.parent(ancestors) {
        let through = match parent {
            AstKind::ParenthesizedExpression(_)
            | AstKind::TSAsExpression(_)
            | AstKind::TSSatisfiesExpression(_)
            | AstKind::TSNonNullExpression(_)
            | AstKind::TSTypeAssertion(_) => true,
            AstKind::ConditionalExpression(conditional) => conditional.test.span() != node.span,
            AstKind::LogicalExpression(logical) => {
                logical.operator != LogicalOperator::And || logical.right.span() == node.span
            }
            _ => false,
        };
        if !through {
            break;
        }
        node = node.up(parent);
    }
    node
}

/// Where the value of `node` goes; `None` when it isn't a reference.
fn passed(ancestors: &[AstKind], node: Node, source: &str) -> Option<Passed> {
    let span = node.span;
    let text = |span: Span| source[span.start as usize..span.end as usize].to_owned();
    let ancestor = |up: usize| ancestors.get(node.depth.checked_sub(up)?).copied();
    Some(match ancestor(1)? {
        AstKind::JSXOpeningElement(_)
        | AstKind::JSXClosingElement(_)
        | AstKind::JSXMemberExpression(_)
        | AstKind::ExportSpecifier(_)
        | AstKind::ExportDefaultDeclaration(_)
        | AstKind::TSTypeQuery(_)
        | AstKind::TSQualifiedName(_) => return None,
        AstKind::StaticMemberExpression(member) if member.object.span() == span => return None,
        AstKind::ComputedMemberExpression(member) if member.object.span() == span => return None,
        AstKind::JSXExpressionContainer(_) => match ancestor(2) {
            Some(AstKind::JSXAttribute(attribute)) => match ancestor(3) {
                Some(AstKind::JSXOpeningElement(element)) => Passed::Attribute {
                    element: text(element.name.span()),
                    name: text(attribute.name.span()),
                },
                _ => Passed::Other,
            },
            _ => Passed::Other,
        },
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
