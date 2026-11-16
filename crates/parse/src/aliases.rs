use std::collections::HashMap;

use oxc_ast::ast::{Argument, Expression, Program, VariableDeclarator};
use oxc_ast_visit::{Visit, walk};
use oxc_semantic::SymbolId;

use crate::patterns::{Segment, leaves};
use crate::scope::{Declared, Scope};
use crate::{Alias, AliasRoot};

/// Variable declarations whose initialiser is a binding, a `require` call or a static property
/// chain on one, keyed by the declared name. Destructured keys extend the chain; names in array
/// patterns aren't aliases.
pub(crate) struct Aliases<'s> {
    scope: &'s Scope<'s>,
    accesses: HashMap<SymbolId, Access>,
}

/// What an initialiser refers to: `Lib.Icon` is `Lib` with members `["Icon"]`.
#[derive(Clone)]
struct Access {
    root: AccessRoot,
    members: Vec<String>,
}

#[derive(Clone)]
enum AccessRoot {
    Symbol(SymbolId),
    Require(String),
}

impl<'s> Aliases<'s> {
    pub(crate) fn new(program: &Program, scope: &'s Scope<'s>) -> Self {
        let mut aliases = Self {
            scope,
            accesses: HashMap::new(),
        };
        aliases.visit_program(program);
        aliases
    }

    /// Follows `symbol` through other aliases back to an import or `require`. Aliases that refer
    /// to each other in a cycle, or to anything else, aren't aliases.
    pub(crate) fn resolve(&self, mut symbol: SymbolId) -> Option<Alias> {
        let mut members = Vec::new();
        let mut seen = Vec::new();
        loop {
            if seen.contains(&symbol) {
                return None;
            }
            seen.push(symbol);
            let access = self.accesses.get(&symbol)?;
            members.splice(0..0, access.members.iter().cloned());
            match &access.root {
                AccessRoot::Require(specifier) => {
                    return Some(Alias {
                        root: AliasRoot::Require(specifier.clone()),
                        members,
                    });
                }
                AccessRoot::Symbol(next) => {
                    if let Declared::Import(import) = self.scope.declared(*next) {
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

    fn access(&self, init: &Expression) -> Option<Access> {
        match init.get_inner_expression() {
            Expression::Identifier(identifier) => Some(Access {
                root: AccessRoot::Symbol(self.scope.symbol(identifier)?),
                members: Vec::new(),
            }),
            Expression::StaticMemberExpression(member) => {
                let mut access = self.access(&member.object)?;
                access.members.push(member.property.name.to_string());
                Some(access)
            }
            Expression::CallExpression(call) => {
                let Expression::Identifier(callee) = call.callee.get_inner_expression() else {
                    return None;
                };
                let [Argument::StringLiteral(specifier)] = call.arguments.as_slice() else {
                    return None;
                };
                let specifier = specifier.value.as_str()?;
                (callee.name == "require" && self.scope.symbol(callee).is_none()).then(|| Access {
                    root: AccessRoot::Require(specifier.to_owned()),
                    members: Vec::new(),
                })
            }
            _ => None,
        }
    }
}

impl<'a> Visit<'a> for Aliases<'_> {
    fn visit_variable_declarator(&mut self, declarator: &VariableDeclarator<'a>) {
        if let Some(access) = declarator.init.as_ref().and_then(|init| self.access(init)) {
            for leaf in leaves(&declarator.id) {
                let keys: Option<Vec<String>> = leaf
                    .path
                    .into_iter()
                    .map(|segment| match segment {
                        Segment::Key(key) => Some(key),
                        Segment::Index(_) => None,
                    })
                    .collect();
                if let Some(keys) = keys {
                    let mut access = access.clone();
                    access.members.extend(keys);
                    self.accesses.insert(leaf.symbol, access);
                }
            }
        }
        walk::walk_variable_declarator(self, declarator);
    }
}
