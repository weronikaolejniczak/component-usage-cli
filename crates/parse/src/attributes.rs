use std::cell::RefCell;
use std::collections::HashMap;

use oxc_ast::ast::{
    Argument, ArrayExpressionElement, ArrowFunctionBody, ArrowFunctionExpression, CallExpression,
    ChainElement, Class, Expression, Function, IdentifierReference, JSXAttributeItem,
    JSXAttributeName, JSXAttributeValue, JSXExpression, MemberExpression, ObjectPropertyKind,
    ReturnStatement, Statement, TemplateLiteral,
};
use oxc_ast_visit::Visit;
use oxc_semantic::SymbolId;
use oxc_span::{GetSpan, Span};
use oxc_syntax::scope::ScopeFlags;

use crate::scope::{Declared, Scope};
use crate::{Attribute, Call, Entry, Expr, ExprKind, Item, Key, Literal, Part, Reference};

/// Converts attributes and the expressions in them to owned [`Expr`] trees.
pub(crate) struct Exprs<'s> {
    pub(crate) source: &'s str,
    pub(crate) scope: &'s Scope<'s>,
    pub(crate) variables: RefCell<Variables>,
    /// Indices into [`crate::FileSummary::parameters`] by the symbols of the names props
    /// declare, set by [`crate::components`].
    pub(crate) parameters: RefCell<HashMap<SymbolId, usize>>,
    /// Indices into [`crate::FileSummary::callbacks`] by symbol, set by [`crate::callbacks`].
    pub(crate) callbacks: RefCell<HashMap<SymbolId, usize>>,
    /// Describe calls and what functions return: [`crate::Options::calls`].
    pub(crate) calls: bool,
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
            parameters: RefCell::new(HashMap::new()),
            callbacks: RefCell::new(HashMap::new()),
            calls: false,
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
                    (operator, _) => ExprKind::Operation {
                        operator,
                        operands: vec![self.expr(&unary.argument)],
                    },
                }
            }
            Expression::BinaryExpression(binary) => ExprKind::Operation {
                operator: binary.operator.as_str(),
                operands: vec![self.expr(&binary.left), self.expr(&binary.right)],
            },
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
                                let returns = match &property.value {
                                    Expression::FunctionExpression(function) => {
                                        self.function_returns(function)
                                    }
                                    _ => None,
                                };
                                self.at(property.span, ExprKind::Function { returns })
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
            Expression::ArrowFunctionExpression(arrow) => ExprKind::Function {
                returns: self.arrow_returns(arrow),
            },
            Expression::FunctionExpression(function) => ExprKind::Function {
                returns: self.function_returns(function),
            },
            Expression::CallExpression(call) => match self.require(call) {
                Some(specifier) => ExprKind::Require {
                    specifier,
                    text: self.text(span),
                    path: Vec::new(),
                },
                None => self.call(call, self.text(span), Vec::new()),
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
                    ChainElement::CallExpression(call) => {
                        path.reverse();
                        return self.call(call, text, path);
                    }
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
                None => self.call(call, text, path),
            },
            _ => ExprKind::Unsupported { text },
        }
    }

    /// A call, then `path` in its result: described with [`Exprs::calls`].
    fn call(&self, call: &CallExpression<'a>, text: String, path: Vec<Key>) -> ExprKind {
        let call = self.calls.then(|| {
            Box::new(Call {
                callee: self.text(call.callee.span()),
                receiver: call
                    .callee
                    .get_inner_expression()
                    .as_member_expression()
                    .map(|member| self.expr(member.object())),
                arguments: call
                    .arguments
                    .iter()
                    .map(|argument| match argument {
                        Argument::SpreadElement(spread) => {
                            Item::Spread(self.expr(&spread.argument))
                        }
                        argument => Item::Value(self.expr(argument.to_expression())),
                    })
                    .collect(),
                path,
            })
        });
        ExprKind::Call { text, call }
    }

    /// What `function` returns, with [`Exprs::calls`], unless it's async or a generator.
    pub(crate) fn function_returns(&self, function: &Function<'a>) -> Option<Vec<Expr>> {
        if !self.calls || function.r#async || function.generator {
            return None;
        }
        Some(self.returns(&function.body.as_ref()?.statements))
    }

    fn arrow_returns(&self, arrow: &ArrowFunctionExpression<'a>) -> Option<Vec<Expr>> {
        if !self.calls || arrow.r#async {
            return None;
        }
        Some(match &arrow.body {
            ArrowFunctionBody::FunctionBody(body) => self.returns(&body.statements),
            body => body
                .as_expression()
                .map(|body| self.expr(body))
                .into_iter()
                .collect(),
        })
    }

    /// The values of the `return` statements in `statements`, outside nested functions and
    /// classes.
    fn returns(&self, statements: &[Statement<'a>]) -> Vec<Expr> {
        let mut returns = Returns {
            exprs: self,
            found: Vec::new(),
        };
        for statement in statements {
            returns.visit_statement(statement);
        }
        returns.found
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
            Declared::Parameter => {
                if let Some(&index) = self.parameters.borrow().get(&symbol) {
                    Reference::Parameter(Some(index))
                } else if let Some(&index) = self.callbacks.borrow().get(&symbol) {
                    Reference::Callback(index)
                } else {
                    Reference::Parameter(None)
                }
            }
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

struct Returns<'e, 's> {
    exprs: &'e Exprs<'s>,
    found: Vec<Expr>,
}

impl<'a> Visit<'a> for Returns<'_, '_> {
    fn visit_return_statement(&mut self, statement: &ReturnStatement<'a>) {
        let value = match &statement.argument {
            Some(argument) => self.exprs.expr(argument),
            None => self.exprs.at(statement.span, ExprKind::Undefined),
        };
        self.found.push(value);
    }

    fn visit_function(&mut self, _: &Function<'a>, _: ScopeFlags) {}

    fn visit_arrow_function_expression(&mut self, _: &ArrowFunctionExpression<'a>) {}

    fn visit_class(&mut self, _: &Class<'a>) {}
}
