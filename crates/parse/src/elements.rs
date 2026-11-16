use std::collections::HashMap;

use oxc_ast::ast::{
    IdentifierReference, JSXAttribute, JSXElement, JSXElementName, JSXMemberExpression,
    JSXMemberExpressionObject, Program,
};
use oxc_ast_visit::{Visit, walk};
use oxc_semantic::SymbolId;

use model::Location;

use crate::aliases::Aliases;
use crate::attributes::{Exprs, attribute_name};
use crate::scope::Declared;
use crate::{Alias, Binding, JsxElement, Parent};

/// The JSX elements whose tag refers to a binding, in source order, and the aliases their tags
/// refer to, indexed by [`Binding::Alias`].
pub(crate) fn elements(
    program: &Program,
    exprs: &Exprs,
    aliases: &Aliases,
) -> (Vec<JsxElement>, Vec<Alias>) {
    let mut collector = Elements {
        exprs,
        aliases,
        frames: Vec::new(),
        alias_of: HashMap::new(),
        used_aliases: Vec::new(),
        elements: Vec::new(),
    };
    collector.visit_program(program);
    let mut elements = collector.elements;
    elements.sort_by_key(|element| element.location);
    (elements, collector.used_aliases)
}

struct Elements<'e, 's> {
    exprs: &'e Exprs<'s>,
    aliases: &'e Aliases<'s>,
    frames: Vec<Frame>,
    alias_of: HashMap<SymbolId, usize>,
    used_aliases: Vec<Alias>,
    elements: Vec<JsxElement>,
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
            let binding = self.binding(root);
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
            self.elements.push(JsxElement {
                location,
                root: root.name.to_string(),
                members,
                binding,
                parent,
                attributes,
            });
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

    fn binding(&mut self, root: &IdentifierReference) -> Binding {
        let scope = self.exprs.scope;
        let Some(symbol) = scope.symbol(root) else {
            return Binding::Global;
        };
        if let Declared::Import(import) = scope.declared(symbol) {
            return Binding::Import(import);
        }
        if let Some(&alias) = self.alias_of.get(&symbol) {
            return Binding::Alias(alias);
        }
        let Some(alias) = self.aliases.resolve(symbol) else {
            return Binding::Local;
        };
        self.used_aliases.push(alias);
        self.alias_of.insert(symbol, self.used_aliases.len() - 1);
        Binding::Alias(self.used_aliases.len() - 1)
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
