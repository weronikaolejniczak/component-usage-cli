use std::collections::{HashMap, HashSet};

use oxc_ast::ast::{
    AssignmentExpression, Class, ClassElement, ExportDefaultDeclaration,
    ExportDefaultDeclarationKind, Expression, Function, MemberExpression, MethodDefinitionKind,
    Program, ThisExpression, UpdateExpression, VariableDeclarator,
};
use oxc_ast_visit::{Visit, walk};
use oxc_semantic::{ReferenceId, SymbolId};
use oxc_span::{GetSpan, Span};
use oxc_syntax::scope::ScopeFlags;

use crate::Expr;
use crate::attributes::Exprs;

/// A class with a superclass and a `render` method.
pub(crate) struct ClassComponent {
    /// The name it's declared as, or `default` for an anonymous default export.
    pub(crate) name: String,
    pub(crate) symbol: Option<SymbolId>,
    /// The start of its name, or of the class when it has none.
    pub(crate) start: u32,
    /// `static defaultProps = …`.
    pub(crate) default_props: Option<Span>,
    /// The fields and methods that aren't `static`, in source order.
    pub(crate) members: Vec<(String, ClassMember)>,
    /// The names assigned to as `this.name`.
    pub(crate) assigned: HashSet<String>,
}

pub(crate) enum ClassMember {
    /// A field, with its value.
    Field(Span),
    /// A method, at its start.
    Method(u32),
}

/// What [`classes`] finds in a file.
#[derive(Default)]
pub(crate) struct Classes {
    pub(crate) components: Vec<ClassComponent>,
    /// The class component each `this` is the instance of, by the start of `this`.
    pub(crate) this: HashMap<u32, usize>,
    /// `Name.defaultProps = …`: the reference to `Name` and the value.
    pub(crate) default_props: Vec<(ReferenceId, Span)>,
}

/// The class components of the file and the `this` in them that is the instance.
pub(crate) fn classes(program: &Program, source: &str) -> Classes {
    if !source.contains("class") && !source.contains("defaultProps") {
        return Classes::default();
    }
    let mut collector = Collector {
        found: Classes::default(),
        current: None,
        named: None,
    };
    collector.visit_program(program);
    collector.found
}

struct Collector {
    found: Classes,
    /// The class component `this` is the instance of here.
    current: Option<usize>,
    /// The name the class starting at the given position is declared as, when the class is an
    /// expression: `const Name = class …`, or an anonymous `export default class`.
    named: Option<(u32, String, Option<SymbolId>)>,
}

impl Collector {
    fn component(&mut self, class: &Class) -> Option<usize> {
        class.heritage.as_ref()?;
        let renders = class.body.body.iter().any(|element| {
            matches!(element, ClassElement::MethodDefinition(method)
                if !method.r#static
                    && method.kind == MethodDefinitionKind::Method
                    && method.key.static_name().as_ref().and_then(|name| name.as_str()) == Some("render"))
        });
        if !renders {
            return None;
        }
        let (name, symbol, start) = match (&class.id, self.named.take()) {
            (Some(id), _) => (id.name.to_string(), Some(id.symbol_id()), id.span.start),
            (None, Some((start, name, symbol))) if start == class.span.start => {
                (name, symbol, start)
            }
            _ => return None,
        };
        self.found.components.push(ClassComponent {
            name,
            symbol,
            start,
            default_props: None,
            members: Vec::new(),
            assigned: HashSet::new(),
        });
        Some(self.found.components.len() - 1)
    }

    /// `this.name` assigned to or changed in place.
    fn assigned(&mut self, target: Option<&MemberExpression>) {
        if let Some(index) = self.current
            && let Some(member) = target
            && matches!(
                member.object().get_inner_expression(),
                Expression::ThisExpression(_)
            )
            && let Some(name) = member.static_property_name().and_then(|name| name.as_str())
        {
            self.found.components[index]
                .assigned
                .insert(name.to_owned());
        }
    }
}

impl<'a> Visit<'a> for Collector {
    fn visit_class(&mut self, class: &Class<'a>) {
        let index = self.component(class);
        if let Some(heritage) = &class.heritage {
            self.visit_expression(&heritage.expression);
        }
        let outer = self.current.take();
        for element in &class.body.body {
            match (element, index) {
                (ClassElement::MethodDefinition(method), Some(index)) if !method.r#static => {
                    if method.kind == MethodDefinitionKind::Method
                        && !method.computed
                        && let Some(name) = method.key.static_name()
                        && let Some(name) = name.as_str()
                    {
                        self.found.components[index]
                            .members
                            .push((name.to_string(), ClassMember::Method(method.span.start)));
                    }
                    self.current = Some(index);
                    self.visit_formal_parameters(&method.value.params);
                    if let Some(body) = &method.value.body {
                        self.visit_function_body(body);
                    }
                    self.current = None;
                }
                (ClassElement::PropertyDefinition(property), Some(index)) => {
                    let name = (!property.computed)
                        .then(|| property.key.static_name())
                        .flatten();
                    let name = name.as_ref().and_then(|name| name.as_str());
                    if let (Some(name), Some(value)) = (name, &property.value) {
                        let component = &mut self.found.components[index];
                        if !property.r#static {
                            component
                                .members
                                .push((name.to_string(), ClassMember::Field(value.span())));
                        } else if name == "defaultProps" {
                            component.default_props = Some(value.span());
                        }
                    }
                    self.current = (!property.r#static).then_some(index);
                    if let Some(value) = &property.value {
                        self.visit_expression(value);
                    }
                    self.current = None;
                }
                (element, _) => self.visit_class_element(element),
            }
        }
        self.current = outer;
    }

    fn visit_function(&mut self, function: &Function<'a>, flags: ScopeFlags) {
        let outer = self.current.take();
        walk::walk_function(self, function, flags);
        self.current = outer;
    }

    fn visit_this_expression(&mut self, this: &ThisExpression) {
        if let Some(index) = self.current {
            self.found.this.insert(this.span.start, index);
        }
    }

    fn visit_variable_declarator(&mut self, declarator: &VariableDeclarator<'a>) {
        if let Some(init) = &declarator.init
            && let Expression::ClassExpression(class) = init.get_inner_expression()
            && let Some(id) = declarator.id.get_binding_identifier()
        {
            self.named = Some((class.span.start, id.name.to_string(), Some(id.symbol_id())));
        }
        walk::walk_variable_declarator(self, declarator);
    }

    fn visit_export_default_declaration(&mut self, export: &ExportDefaultDeclaration<'a>) {
        let class = match &export.declaration {
            ExportDefaultDeclarationKind::ClassDeclaration(class) => Some(class.span.start),
            declaration => match declaration
                .as_expression()
                .map(|e| e.get_inner_expression())
            {
                Some(Expression::ClassExpression(class)) => Some(class.span.start),
                _ => None,
            },
        };
        if let Some(start) = class {
            self.named = Some((start, "default".to_owned(), None));
        }
        walk::walk_export_default_declaration(self, export);
    }

    fn visit_assignment_expression(&mut self, assignment: &AssignmentExpression<'a>) {
        self.assigned(assignment.left.as_member_expression());
        if let Some(member) = assignment.left.as_member_expression()
            && let Expression::Identifier(object) = member.object().get_inner_expression()
            && member.static_property_name().and_then(|name| name.as_str()) == Some("defaultProps")
        {
            self.found
                .default_props
                .push((object.reference_id(), assignment.right.span()));
        }
        walk::walk_assignment_expression(self, assignment);
    }

    fn visit_update_expression(&mut self, update: &UpdateExpression<'a>) {
        self.assigned(update.argument.as_member_expression());
        walk::walk_update_expression(self, update);
    }
}

/// The expressions at `spans`, built once [`Exprs`] knows what `this` refers to.
pub(crate) fn values(
    program: &Program,
    exprs: &Exprs,
    spans: HashSet<Span>,
) -> HashMap<Span, Expr> {
    let mut builder = Builder {
        exprs,
        spans,
        found: HashMap::new(),
    };
    if !builder.spans.is_empty() {
        builder.visit_program(program);
    }
    builder.found
}

struct Builder<'e, 's> {
    exprs: &'e Exprs<'s>,
    spans: HashSet<Span>,
    found: HashMap<Span, Expr>,
}

impl<'a> Visit<'a> for Builder<'_, '_> {
    fn visit_expression(&mut self, expression: &Expression<'a>) {
        let span = expression.span();
        if self.spans.remove(&span) {
            self.found.insert(span, self.exprs.expr(expression));
        }
        walk::walk_expression(self, expression);
    }
}
