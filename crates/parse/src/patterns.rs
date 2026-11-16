use oxc_ast::ast::{BindingPattern, Expression};
use oxc_semantic::SymbolId;

use crate::Segment;

/// A name a pattern declares.
pub(crate) struct Leaf<'p, 'a> {
    pub(crate) symbol: SymbolId,
    /// The steps from the destructured value to the name: `[Key("a"), Index(0)]` in
    /// `{ a: [name] }`.
    pub(crate) path: Vec<Segment>,
    /// The name's own default: `x` in `{ name = x }`.
    pub(crate) default: Option<&'p Expression<'a>>,
    /// Whether a pattern on the way has a default, as in `{ a: { name } = {} }`.
    pub(crate) inner_default: bool,
}

/// The names `pattern` declares, other than those behind computed keys and rest elements.
pub(crate) fn leaves<'p, 'a>(pattern: &'p BindingPattern<'a>) -> Vec<Leaf<'p, 'a>> {
    let mut leaves = Vec::new();
    walk(pattern, &mut Vec::new(), None, false, &mut leaves);
    leaves
}

fn walk<'p, 'a>(
    pattern: &'p BindingPattern<'a>,
    path: &mut Vec<Segment>,
    default: Option<&'p Expression<'a>>,
    inner_default: bool,
    leaves: &mut Vec<Leaf<'p, 'a>>,
) {
    match pattern {
        BindingPattern::BindingIdentifier(identifier) => leaves.push(Leaf {
            symbol: identifier.symbol_id(),
            path: path.clone(),
            default,
            inner_default,
        }),
        BindingPattern::AssignmentPattern(assignment) => walk(
            &assignment.left,
            path,
            Some(&assignment.right),
            inner_default,
            leaves,
        ),
        BindingPattern::ObjectPattern(object) => {
            let inner_default = inner_default || default.is_some();
            for property in &object.properties {
                let key = property.key.static_name().filter(|_| !property.computed);
                let Some(key) = key.as_ref().and_then(|key| key.as_str()) else {
                    continue;
                };
                path.push(Segment::Key(key.to_owned()));
                walk(&property.value, path, None, inner_default, leaves);
                path.pop();
            }
        }
        BindingPattern::ArrayPattern(array) => {
            let inner_default = inner_default || default.is_some();
            for (index, element) in array.elements.iter().enumerate() {
                if let Some(element) = element {
                    path.push(Segment::Index(index));
                    walk(element, path, None, inner_default, leaves);
                    path.pop();
                }
            }
        }
    }
}
