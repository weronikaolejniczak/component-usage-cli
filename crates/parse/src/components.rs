use std::collections::{HashMap, HashSet};

use oxc_ast::ast::{
    AssignmentPattern, BindingPattern, CallExpression, ExportDefaultDeclaration,
    ExportDefaultDeclarationKind, ExportNamedDeclaration, Expression, FormalParameters, Function,
    IdentifierReference, JSXElementName, JSXMemberExpressionObject, ModuleExportName, Program,
    StaticMemberExpression, TSTypeQuery, VariableDeclaration, VariableDeclarationKind,
    VariableDeclarator,
};
use oxc_ast_visit::{Visit, walk};
use oxc_semantic::{ReferenceId, SymbolId};
use oxc_syntax::scope::ScopeFlags;

use crate::attributes::Exprs;
use crate::scope::Declared;
use crate::{Component, Parameter, Segment};

/// What [`components`] finds in a file.
pub(crate) struct Components {
    pub(crate) components: Vec<Component>,
    pub(crate) parameters: Vec<Parameter>,
    /// Components by the symbols of the names they're declared as.
    pub(crate) by_symbol: HashMap<SymbolId, usize>,
    /// Sorted indices into [`crate::FileSummary::imports`].
    pub(crate) escaping_imports: Vec<usize>,
}

/// The components of the file and the names their props declare. Sets [`Exprs::parameters`],
/// so expressions built afterwards refer to props by [`crate::Reference::Parameter`] index.
pub(crate) fn components(program: &Program, exprs: &Exprs) -> Components {
    let mut collector = Collector {
        exprs,
        in_const: false,
        in_type: false,
        found: Vec::new(),
        wrappers: Vec::new(),
        allowed: HashSet::new(),
        members: HashSet::new(),
    };
    collector.visit_program(program);
    let Collector {
        found,
        wrappers,
        mut allowed,
        members,
        ..
    } = collector;

    let scoping = exprs.scope.scoping;
    let mut components = Vec::new();
    let mut by_symbol = HashMap::new();
    let mut parameters = Vec::new();
    let mut parameter_of = HashMap::new();
    for found in found {
        let index = components.len();
        components.push(Component {
            name: found.name,
            location: found.location,
            variables: found
                .symbol
                .map(|symbol| exprs.variable(symbol))
                .into_iter()
                .collect(),
            default: found.symbol.is_none(),
            escapes: false,
        });
        if let Some(symbol) = found.symbol {
            by_symbol.insert(symbol, index);
        }
        for leaf in found.props {
            parameter_of.insert(leaf.symbol, parameters.len());
            parameters.push(Parameter {
                component: index,
                location: exprs
                    .scope
                    .lines
                    .location(scoping.symbol_span(leaf.symbol).start),
                path: leaf.path,
                default: None,
                rest: leaf.rest,
            });
        }
    }
    for (wrapper, argument) in wrappers {
        let Some(&index) = scoping
            .get_reference(argument)
            .symbol_id()
            .and_then(|symbol| by_symbol.get(&symbol))
        else {
            continue;
        };
        allowed.insert(argument);
        match wrapper {
            Some(symbol) => {
                components[index].variables.push(exprs.variable(symbol));
                by_symbol.insert(symbol, index);
            }
            None => components[index].default = true,
        }
    }

    *exprs.parameters.borrow_mut() = parameter_of;
    let mut defaults = Defaults {
        exprs,
        parameters: &mut parameters,
    };
    defaults.visit_program(program);

    let escapes = |symbol: SymbolId, members: &HashSet<ReferenceId>| {
        scoping
            .get_resolved_reference_ids(symbol)
            .iter()
            .any(|&id| {
                !allowed.contains(&id)
                    && !members.contains(&id)
                    && scoping.get_reference(id).is_value()
            })
    };
    for (&symbol, &index) in &by_symbol {
        if escapes(symbol, &members) {
            components[index].escapes = true;
        }
    }
    let mut escaping_imports: Vec<usize> = scoping
        .symbol_ids()
        .filter_map(|symbol| match exprs.scope.declared(symbol) {
            Declared::Import(import) if escapes(symbol, &HashSet::new()) => Some(import),
            _ => None,
        })
        .collect();
    escaping_imports.sort_unstable();
    Components {
        components,
        parameters,
        by_symbol,
        escaping_imports,
    }
}

/// A function found to be a component.
struct Found {
    name: String,
    symbol: Option<SymbolId>,
    location: model::Location,
    props: Vec<Leaf>,
}

struct Collector<'e, 's> {
    exprs: &'e Exprs<'s>,
    in_const: bool,
    in_type: bool,
    found: Vec<Found>,
    /// `const Wrapper = memo(Inner)`, or `export default memo(Inner)` without a symbol: the
    /// wrapper's symbol and the reference to `Inner`.
    wrappers: Vec<(Option<SymbolId>, ReferenceId)>,
    /// References that don't make a name escape: JSX tags, exports and types.
    allowed: HashSet<ReferenceId>,
    /// Objects of member expressions, such as `Name` in `Name.displayName = …`: they don't make
    /// a component escape, but do an import, whose member can be a component.
    members: HashSet<ReferenceId>,
}

/// What a `const` or `export default` holds, through `memo(…)` and `forwardRef(…)`.
enum Held<'h, 'a> {
    Function(&'h FormalParameters<'a>, u32),
    Name(ReferenceId),
}

fn held<'h, 'a>(expression: &'h Expression<'a>) -> Option<Held<'h, 'a>> {
    match expression.get_inner_expression() {
        Expression::ArrowFunctionExpression(arrow) => {
            Some(Held::Function(&arrow.params, arrow.span.start))
        }
        Expression::FunctionExpression(function) if function.body.is_some() => {
            Some(Held::Function(&function.params, function.span.start))
        }
        Expression::CallExpression(call) if wraps(call) => {
            let argument = call.arguments.first()?.as_expression()?;
            match argument.get_inner_expression() {
                Expression::Identifier(identifier) => Some(Held::Name(identifier.reference_id())),
                _ => held(argument),
            }
        }
        _ => None,
    }
}

impl Collector<'_, '_> {
    fn add(
        &mut self,
        name: String,
        symbol: Option<SymbolId>,
        start: u32,
        params: &FormalParameters,
    ) {
        let mut props = Vec::new();
        if let Some(first) = params.items.first() {
            leaves(&first.pattern, &mut Vec::new(), false, &mut props);
        }
        self.found.push(Found {
            name,
            symbol,
            location: self.exprs.scope.lines.location(start),
            props,
        });
    }
}

impl<'a> Visit<'a> for Collector<'_, '_> {
    fn visit_variable_declaration(&mut self, declaration: &VariableDeclaration<'a>) {
        let outer = std::mem::replace(
            &mut self.in_const,
            declaration.kind == VariableDeclarationKind::Const,
        );
        walk::walk_variable_declaration(self, declaration);
        self.in_const = outer;
    }

    fn visit_variable_declarator(&mut self, declarator: &VariableDeclarator<'a>) {
        if self.in_const
            && let BindingPattern::BindingIdentifier(id) = &declarator.id
            && let Some(init) = &declarator.init
        {
            match held(init) {
                Some(Held::Function(params, _)) => {
                    self.add(
                        id.name.to_string(),
                        Some(id.symbol_id()),
                        id.span.start,
                        params,
                    );
                }
                Some(Held::Name(argument)) => self.wrappers.push((Some(id.symbol_id()), argument)),
                None => {}
            }
        }
        walk::walk_variable_declarator(self, declarator);
    }

    fn visit_function(&mut self, function: &Function<'a>, flags: ScopeFlags) {
        if function.is_declaration()
            && function.body.is_some()
            && let Some(id) = &function.id
        {
            self.add(
                id.name.to_string(),
                Some(id.symbol_id()),
                id.span.start,
                &function.params,
            );
        }
        walk::walk_function(self, function, flags);
    }

    fn visit_export_default_declaration(&mut self, export: &ExportDefaultDeclaration<'a>) {
        match &export.declaration {
            ExportDefaultDeclarationKind::FunctionDeclaration(function)
                if function.id.is_none() && function.body.is_some() =>
            {
                self.add(
                    "default".to_owned(),
                    None,
                    function.span.start,
                    &function.params,
                );
            }
            declaration => {
                if let Some(expression) = declaration.as_expression() {
                    if let Expression::Identifier(identifier) = expression.get_inner_expression() {
                        self.allowed.insert(identifier.reference_id());
                    }
                    match held(expression) {
                        Some(Held::Function(params, start)) => {
                            self.add("default".to_owned(), None, start, params);
                        }
                        Some(Held::Name(argument)) => self.wrappers.push((None, argument)),
                        None => {}
                    }
                }
            }
        }
        walk::walk_export_default_declaration(self, export);
    }

    fn visit_export_named_declaration(&mut self, export: &ExportNamedDeclaration<'a>) {
        for specifier in &export.specifiers {
            if let ModuleExportName::IdentifierReference(local) = &specifier.local {
                self.allowed.insert(local.reference_id());
            }
        }
        walk::walk_export_named_declaration(self, export);
    }

    fn visit_jsx_element_name(&mut self, name: &JSXElementName<'a>) {
        if let JSXElementName::IdentifierReference(root) = name {
            self.allowed.insert(root.reference_id());
        }
        walk::walk_jsx_element_name(self, name);
    }

    fn visit_jsx_member_expression_object(&mut self, object: &JSXMemberExpressionObject<'a>) {
        if let JSXMemberExpressionObject::IdentifierReference(root) = object {
            self.allowed.insert(root.reference_id());
        }
        walk::walk_jsx_member_expression_object(self, object);
    }

    fn visit_ts_type_query(&mut self, query: &TSTypeQuery<'a>) {
        let outer = std::mem::replace(&mut self.in_type, true);
        walk::walk_ts_type_query(self, query);
        self.in_type = outer;
    }

    fn visit_static_member_expression(&mut self, member: &StaticMemberExpression<'a>) {
        if let Expression::Identifier(object) = &member.object
            && !matches!(member.property.name.as_str(), "call" | "apply" | "bind")
        {
            self.members.insert(object.reference_id());
        }
        walk::walk_static_member_expression(self, member);
    }

    fn visit_identifier_reference(&mut self, identifier: &IdentifierReference<'a>) {
        if self.in_type {
            self.allowed.insert(identifier.reference_id());
        }
    }
}

/// Sets the defaults of props, such as `'gear'` in `{ iconType = 'gear' }`.
struct Defaults<'e, 's, 'p> {
    exprs: &'e Exprs<'s>,
    parameters: &'p mut [Parameter],
}

impl<'a> Visit<'a> for Defaults<'_, '_, '_> {
    fn visit_assignment_pattern(&mut self, pattern: &AssignmentPattern<'a>) {
        if let BindingPattern::BindingIdentifier(identifier) = &pattern.left {
            let index = self
                .exprs
                .parameters
                .borrow()
                .get(&identifier.symbol_id())
                .copied();
            if let Some(index) = index
                && !self.parameters[index].path.is_empty()
            {
                self.parameters[index].default = Some(self.exprs.expr(&pattern.right));
            }
        }
        walk::walk_assignment_pattern(self, pattern);
    }
}

/// `memo(…)`, `forwardRef(…)`, or a member call such as `React.memo(…)`.
fn wraps(call: &CallExpression) -> bool {
    let name = match call.callee.get_inner_expression() {
        Expression::Identifier(identifier) => Some(identifier.name.as_str()),
        callee => callee
            .as_member_expression()
            .and_then(|member| member.static_property_name())
            .and_then(|name| name.as_str()),
    };
    matches!(name, Some("memo" | "forwardRef"))
}

/// A name declared by the props.
struct Leaf {
    symbol: SymbolId,
    path: Vec<Segment>,
    rest: Option<Vec<String>>,
}

/// The names `pattern` declares, other than those behind computed keys, array rest elements
/// and defaults on the way, such as `{ a: { b } = {} }`.
fn leaves(pattern: &BindingPattern, path: &mut Vec<Segment>, inner: bool, leaves_: &mut Vec<Leaf>) {
    match pattern {
        BindingPattern::BindingIdentifier(identifier) => leaves_.push(Leaf {
            symbol: identifier.symbol_id(),
            path: path.clone(),
            rest: None,
        }),
        BindingPattern::AssignmentPattern(assignment) => {
            leaves(&assignment.left, path, true, leaves_);
        }
        BindingPattern::ObjectPattern(object) => {
            if inner {
                return;
            }
            let mut keys = Vec::new();
            for property in &object.properties {
                let key = property.key.static_name().filter(|_| !property.computed);
                let Some(key) = key.as_ref().and_then(|key| key.as_str()) else {
                    continue;
                };
                keys.push(key.to_owned());
                path.push(Segment::Key(key.to_owned()));
                leaves(&property.value, path, false, leaves_);
                path.pop();
            }
            if let Some(rest) = &object.rest
                && let BindingPattern::BindingIdentifier(identifier) = &rest.argument
            {
                leaves_.push(Leaf {
                    symbol: identifier.symbol_id(),
                    path: path.clone(),
                    rest: Some(keys),
                });
            }
        }
        BindingPattern::ArrayPattern(array) => {
            if inner {
                return;
            }
            for (index, element) in array.elements.iter().enumerate() {
                if let Some(element) = element {
                    path.push(Segment::Index(index));
                    leaves(element, path, false, leaves_);
                    path.pop();
                }
            }
        }
    }
}
