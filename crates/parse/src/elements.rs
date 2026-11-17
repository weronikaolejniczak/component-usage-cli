use oxc_ast::ast::{
    IdentifierReference, JSXAttribute, JSXChild, JSXElement, JSXElementName, JSXMemberExpression,
    JSXMemberExpressionObject, Program,
};
use oxc_ast_visit::{Visit, walk};
use oxc_semantic::SymbolId;
use oxc_span::GetSpan;

use model::Location;

use crate::attributes::{Exprs, attribute_name};
use crate::scope::Declared;
use crate::{Binding, JsxElement, Parent};

/// The JSX elements whose tag refers to a binding, in source order, each with the local name its
/// tag starts with, if any. Those elements are [`Binding::Local`] until `alias` says otherwise.
pub(crate) fn elements(program: &Program, exprs: &Exprs) -> Vec<(JsxElement, Option<SymbolId>)> {
    let mut collector = Elements {
        exprs,
        frames: Vec::new(),
        elements: Vec::new(),
    };
    collector.visit_program(program);
    let mut elements = collector.elements;
    elements.sort_by_key(|(element, _)| element.location);
    elements
}

struct Elements<'e, 's> {
    exprs: &'e Exprs<'s>,
    frames: Vec<Frame>,
    elements: Vec<(JsxElement, Option<SymbolId>)>,
}

/// An element being visited. `location` is set when it's in [`Elements::elements`].
struct Frame {
    location: Option<Location>,
    attribute: Option<String>,
}

impl<'a> Visit<'a> for Elements<'_, '_> {
    fn visit_jsx_element(&mut self, element: &JSXElement<'a>) {
        let opening = &element.opening_element;
        let location = tag(&opening.name).map(|(root, members)| {
            let location = self.exprs.scope.lines.location(opening.span.start);
            let (binding, local) = self.binding(root);
            let parent = self.frames.iter().rev().find_map(|frame| {
                Some(Parent {
                    location: frame.location?,
                    attribute: frame.attribute.clone(),
                })
            });
            let attributes = opening
                .attributes
                .iter()
                .map(|attribute| self.exprs.attribute(attribute))
                .collect();
            let children = element
                .children
                .iter()
                .find(
                    |child| !matches!(child, JSXChild::Text(text) if text.value.trim().is_empty()),
                )
                .map(|child| self.exprs.scope.lines.location(child.span().start));
            self.elements.push((
                JsxElement {
                    location,
                    root: root.name.to_string(),
                    members,
                    binding,
                    parent,
                    attributes,
                    children,
                    component: None,
                },
                local,
            ));
            location
        });
        self.frames.push(Frame {
            location,
            attribute: None,
        });
        walk::walk_jsx_element(self, element);
        self.frames.pop();
    }

    fn visit_jsx_attribute(&mut self, attribute: &JSXAttribute<'a>) {
        self.set_attribute(Some(attribute_name(&attribute.name)));
        walk::walk_jsx_attribute(self, attribute);
        self.set_attribute(None);
    }
}

impl Elements<'_, '_> {
    fn set_attribute(&mut self, attribute: Option<String>) {
        if let Some(frame) = self.frames.last_mut() {
            frame.attribute = attribute;
        }
    }

    fn binding(&self, root: &IdentifierReference) -> (Binding, Option<SymbolId>) {
        let scope = self.exprs.scope;
        let Some(symbol) = scope.symbol(root) else {
            return (Binding::Global, None);
        };
        match scope.declared(symbol) {
            Declared::Import(import) => (Binding::Import(import), None),
            Declared::Parameter | Declared::Local => (Binding::Local, Some(symbol)),
        }
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
