use std::cell::RefCell;
use std::collections::HashMap;

use oxc_ast::ast::{
    Argument, ArrayExpressionElement, CallExpression, ChainElement, Expression,
    IdentifierReference, JSXAttributeItem, JSXAttributeName, JSXAttributeValue, JSXExpression,
    MemberExpression, ObjectPropertyKind, TemplateLiteral,
};
use oxc_semantic::SymbolId;
use oxc_span::{GetSpan, Span};

use crate::scope::{Declared, Scope};
use crate::{Attribute, Entry, Expr, ExprKind, Item, Key, Literal, Part, Reference};

/// Converts attributes and the expressions in them to owned [`Expr`] trees.
pub(crate) struct Exprs<'s> {
    pub(crate) source: &'s str,
    pub(crate) scope: &'s Scope<'s>,
    pub(crate) variables: RefCell<Variables>,
}

/// The local names referred to so far, in the order of [`Reference::Variable`] indices.
#[derive(Default)]
pub(crate) struct Variables {
    index: HashMap<SymbolId, usize>,
    pub(crate) symbols: Vec<SymbolId>,
}

impl<'s> Exprs<'s> {
    pub(crate) fn new(source: &'s str, scope: &'s Scope<'s>) -> Self {
        Self {
            source,
            scope,
            variables: RefCell::new(Variables::default()),
        }
    }
}

impl<'a> Exprs<'_> {
    pub(crate) fn attribute(&self, item: &JSXAttributeItem<'a>) -> Attribute {
        match item {
            JSXAttributeItem::Attribute(attribute) => {
                let value = match &attribute.value {
                    None => self.at(attribute.span, ExprKind::Literal(Literal::Boolean(true))),
                    Some(JSXAttributeValue::StringLiteral(string)) => self.at(
                        string.span,
                        ExprKind::Literal(Literal::String(string.value.display().to_string())),
                    ),
                    Some(JSXAttributeValue::ExpressionContainer(container)) => {
                        match &container.expression {
                            JSXExpression::EmptyExpression(empty) => {
                                self.at(empty.span, ExprKind::Undefined)
                            }
                            expression => self.expr(expression.to_expression()),
                        }
                    }
                    Some(JSXAttributeValue::Element(element)) => self.at(
                        element.span,
                        ExprKind::Element {
                            tag: Some(self.text(element.opening_element.name.span())),
                        },
                    ),
                    Some(JSXAttributeValue::Fragment(fragment)) => {
                        self.at(fragment.span, ExprKind::Element { tag: None })
                    }
                };
                Attribute::Named {
                    name: attribute_name(&attribute.name),
                    value,
                }
            }
            JSXAttributeItem::SpreadAttribute(spread) => {
                Attribute::Spread(self.expr(&spread.argument))
            }
        }
    }

    pub(crate) fn expr(&self, expression: &Expression<'a>) -> Expr {
        let expression = expression.get_inner_expression();
        let span = expression.span();
        let kind = match expression {
            Expression::StringLiteral(string) => {
                ExprKind::Literal(Literal::String(string.value.display().to_string()))
            }
            Expression::NumericLiteral(number) if number.value.is_finite() => {
                ExprKind::Literal(Literal::Number(number.value))
            }
            Expression::BooleanLiteral(boolean) => {
                ExprKind::Literal(Literal::Boolean(boolean.value))
            }
            Expression::NullLiteral(_) => ExprKind::Literal(Literal::Null),
            Expression::UnaryExpression(unary) => {
                match (
                    unary.operator.as_str(),
                    unary.argument.get_inner_expression(),
                ) {
                    ("void", _) => ExprKind::Undefined,
                    ("-", Expression::NumericLiteral(number)) if number.value.is_finite() => {
                        ExprKind::Literal(Literal::Number(-number.value))
                    }
                    _ => self.unsupported(span),
                }
            }
            Expression::Identifier(identifier) => {
                let name = self.reference(identifier);
                if identifier.name == "undefined" && name == Reference::Global {
                    ExprKind::Undefined
                } else {
                    ExprKind::Reference {
                        name,
                        text: self.text(span),
                        path: Vec::new(),
                    }
                }
            }
            Expression::TemplateLiteral(template) => match template.single_quasi() {
                Some(text) => ExprKind::Literal(Literal::String(text.display().to_string())),
                None => ExprKind::Template {
                    tag: None,
                    parts: self.parts(template),
                },
            },
            Expression::TaggedTemplateExpression(tagged) => ExprKind::Template {
                tag: Some(self.text(tagged.tag.span())),
                parts: self.parts(&tagged.quasi),
            },
            Expression::ObjectExpression(object) => ExprKind::Object(
                object
                    .properties
                    .iter()
                    .map(|property| match property {
                        ObjectPropertyKind::ObjectProperty(property) => {
                            let value = if property.method || property.kind.is_accessor() {
                                self.at(property.span, ExprKind::Function)
                            } else {
                                self.expr(&property.value)
                            };
                            match property.key.static_name() {
                                Some(key) => Entry::Property {
                                    key: key.display().to_string(),
                                    value,
                                },
                                None => Entry::Computed {
                                    key: self.text(property.key.span()),
                                    value,
                                },
                            }
                        }
                        ObjectPropertyKind::SpreadProperty(spread) => {
                            Entry::Spread(self.expr(&spread.argument))
                        }
                    })
                    .collect(),
            ),
            Expression::ArrayExpression(array) => ExprKind::Array(
                array
                    .elements
                    .iter()
                    .map(|element| match element {
                        ArrayExpressionElement::SpreadElement(spread) => {
                            Item::Spread(self.expr(&spread.argument))
                        }
                        ArrayExpressionElement::Elision(elision) => {
                            Item::Value(self.at(elision.span, ExprKind::Undefined))
                        }
                        element => Item::Value(self.expr(element.to_expression())),
                    })
                    .collect(),
            ),
            Expression::ConditionalExpression(conditional) => self.one_of([
                self.expr(&conditional.consequent),
                self.expr(&conditional.alternate),
            ]),
            Expression::LogicalExpression(logical) => {
                self.one_of([self.expr(&logical.left), self.expr(&logical.right)])
            }
            Expression::JSXElement(element) => ExprKind::Element {
                tag: Some(self.text(element.opening_element.name.span())),
            },
            Expression::JSXFragment(_) => ExprKind::Element { tag: None },
            Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_) => {
                ExprKind::Function
            }
            Expression::CallExpression(call) => match self.require(call) {
                Some(specifier) => ExprKind::Require {
                    specifier,
                    text: self.text(span),
                    path: Vec::new(),
                },
                None => ExprKind::Call {
                    text: self.text(span),
                },
            },
            Expression::ChainExpression(_)
            | Expression::StaticMemberExpression(_)
            | Expression::ComputedMemberExpression(_)
            | Expression::PrivateFieldExpression(_) => self.member(expression, span),
            _ => self.unsupported(span),
        };
        self.at(span, kind)
    }

    /// A member access takes its kind from what it starts with, and keeps its keys when that is
    /// a name.
    fn member(&self, expression: &Expression<'a>, span: Span) -> ExprKind {
        let text = self.text(span);
        let mut path = Vec::new();
        let mut current = expression;
        let root = loop {
            let inner = current.get_inner_expression();
            let member = match inner {
                Expression::ChainExpression(chain) => match &chain.expression {
                    ChainElement::CallExpression(_) => return ExprKind::Call { text },
                    ChainElement::TSNonNullExpression(non_null) => {
                        current = &non_null.expression;
                        continue;
                    }
                    element => match element.as_member_expression() {
                        Some(member) => member,
                        None => return ExprKind::Unsupported { text },
                    },
                },
                other => match other.as_member_expression() {
                    Some(member) => member,
                    None => break inner,
                },
            };
            let (object, key) = match member {
                MemberExpression::StaticMemberExpression(member) => (
                    &member.object,
                    Key::Static(member.property.name.to_string()),
                ),
                MemberExpression::ComputedMemberExpression(member) => {
                    (&member.object, key(&member.expression))
                }
                MemberExpression::PrivateFieldExpression(_) => {
                    return ExprKind::Unsupported { text };
                }
            };
            path.push(key);
            current = object;
        };
        path.reverse();
        match root {
            Expression::Identifier(identifier) => ExprKind::Reference {
                name: self.reference(identifier),
                text,
                path,
            },
            Expression::CallExpression(call) => match self.require(call) {
                Some(specifier) => ExprKind::Require {
                    specifier,
                    text,
                    path,
                },
                None => ExprKind::Call { text },
            },
            _ => ExprKind::Unsupported { text },
        }
    }

    fn one_of(&self, operands: [Expr; 2]) -> ExprKind {
        ExprKind::OneOf(
            operands
                .into_iter()
                .flat_map(|operand| match operand.kind {
                    ExprKind::OneOf(values) => values,
                    _ => vec![operand],
                })
                .collect(),
        )
    }

    fn parts(&self, template: &TemplateLiteral<'a>) -> Vec<Part> {
        let mut parts = Vec::new();
        for (index, quasi) in template.quasis.iter().enumerate() {
            let text = quasi
                .value
                .cooked
                .and_then(|cooked| cooked.as_str())
                .unwrap_or(quasi.value.raw.as_str());
            if !text.is_empty() {
                parts.push(Part::Text(text.to_owned()));
            }
            if let Some(expression) = template.expressions.get(index) {
                parts.push(Part::Value(self.expr(expression)));
            }
        }
        parts
    }

    fn reference(&self, identifier: &IdentifierReference) -> Reference {
        let Some(symbol) = self.scope.symbol(identifier) else {
            return Reference::Global;
        };
        match self.scope.declared(symbol) {
            Declared::Import(import) => Reference::Import(import),
            Declared::Parameter => Reference::Parameter,
            Declared::Local => Reference::Variable(self.variable(symbol)),
        }
    }

    /// The specifier of `require('specifier')`, when `require` isn't declared in the file.
    fn require(&self, call: &CallExpression) -> Option<String> {
        let Expression::Identifier(callee) = call.callee.get_inner_expression() else {
            return None;
        };
        let [Argument::StringLiteral(specifier)] = call.arguments.as_slice() else {
            return None;
        };
        let specifier = specifier.value.as_str()?;
        (callee.name == "require" && self.scope.symbol(callee).is_none())
            .then(|| specifier.to_owned())
    }

    /// The index of `symbol` in [`Variables::symbols`], adding it if it isn't there.
    pub(crate) fn variable(&self, symbol: SymbolId) -> usize {
        let mut variables = self.variables.borrow_mut();
        let next = variables.symbols.len();
        let index = *variables.index.entry(symbol).or_insert(next);
        if index == next {
            variables.symbols.push(symbol);
        }
        index
    }

    fn unsupported(&self, span: Span) -> ExprKind {
        ExprKind::Unsupported {
            text: self.text(span),
        }
    }

    fn at(&self, span: Span, kind: ExprKind) -> Expr {
        Expr {
            location: self.scope.lines.location(span.start),
            kind,
        }
    }

    fn text(&self, span: Span) -> String {
        self.source[span.start as usize..span.end as usize].to_owned()
    }
}

fn key(expression: &Expression) -> Key {
    match expression.get_inner_expression() {
        Expression::StringLiteral(string) => Key::Static(string.value.display().to_string()),
        Expression::NumericLiteral(number) => match model::Literal::number(number.value) {
            model::Literal::Integer(index) if index >= 0 => Key::Static(index.to_string()),
            _ => Key::Dynamic,
        },
        Expression::TemplateLiteral(template) => match template.single_quasi() {
            Some(text) => Key::Static(text.display().to_string()),
            None => Key::Dynamic,
        },
        _ => Key::Dynamic,
    }
}

pub(crate) fn attribute_name(name: &JSXAttributeName) -> String {
    match name {
        JSXAttributeName::Identifier(identifier) => identifier.name.to_string(),
        JSXAttributeName::NamespacedName(name) => {
            format!("{}:{}", name.namespace.name, name.name.name)
        }
    }
}
