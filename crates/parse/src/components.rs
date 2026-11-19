use std::collections::{HashMap, HashSet};

use oxc_ast::ast::{
    Argument, AssignmentPattern, BindingPattern, CallExpression, ExportDefaultDeclaration,
    ExportDefaultDeclarationKind, ExportNamedDeclaration, Expression, FormalParameter,
    FormalParameters, Function, IdentifierReference, JSXElementName, JSXMemberExpressionObject,
    ModuleExportName, Program, StaticMemberExpression, TSTypeQuery, VariableDeclaration,
    VariableDeclarationKind, VariableDeclarator,
};
use oxc_ast_visit::{Visit, walk};
use oxc_semantic::{ReferenceId, SymbolId};
use oxc_span::{GetSpan, Span};
use oxc_syntax::scope::ScopeFlags;

use crate::attributes::Exprs;
use crate::classes::Classes;
use crate::definitions::mutated;
use crate::patterns::leaves;
use crate::scope::Declared;
use crate::{
    Attribute, Callee, Component, ExprKind, Forward, Forwarded, JsxElement, Parameter, Reference,
    Segment,
};

/// Adds to [`Component::forwards`] the elements given a spread of a component's props, or of a
/// rest element of them.
pub(crate) fn spreads(
    components: &mut [Component],
    parameters: &[Parameter],
    elements: &[JsxElement],
) {
    for (index, element) in elements.iter().enumerate() {
        for (attribute, value) in element.attributes.iter().enumerate() {
            let Attribute::Spread(spread) = value else {
                continue;
            };
            let ExprKind::Reference {
                name: Reference::Parameter(Some(parameter)),
                path,
                ..
            } = &spread.kind
            else {
                continue;
            };
            let Some(parameter) = parameters.get(*parameter) else {
                continue;
            };
            if !path.is_empty()
                || parameter.position != 0
                || !parameter.path.is_empty()
                || parameter.mutated
            {
                continue;
            }
            components[parameter.component].forwards.push(Forward {
                location: element.location,
                to: Forwarded::Element {
                    element: index,
                    attribute,
                },
                excluded: parameter.rest.clone().unwrap_or_default(),
            });
        }
    }
}

/// What [`components`] finds in a file.
pub(crate) struct Components {
    pub(crate) components: Vec<Component>,
    pub(crate) parameters: Vec<Parameter>,
    /// Components by the symbols of the names they're declared as.
    pub(crate) by_symbol: HashMap<SymbolId, usize>,
    /// Calls by name of the components and of imports, with the spans of their arguments,
    /// whether each is spread.
    pub(crate) calls: Vec<(Callee, ByName)>,
    /// Sorted indices into [`crate::FileSummary::imports`].
    pub(crate) escaping_imports: Vec<usize>,
}

/// The components of the file and the names their props declare. Sets [`Exprs::parameters`],
/// so expressions built afterwards refer to props by [`crate::Reference::Parameter`] index.
/// With `calls`, also the `const`s and default exports holding a call that passes a name on, as
/// [`Forwarded::Import`] or [`Forwarded::Component`].
pub(crate) fn components(
    program: &Program,
    exprs: &Exprs,
    classes: &Classes,
    calls: bool,
    changed: &HashSet<SymbolId>,
) -> Components {
    let mut collector = Collector {
        exprs,
        in_const: false,
        in_type: false,
        found: Vec::new(),
        wrappers: Vec::new(),
        calls: calls.then(Vec::new),
        called: Vec::new(),
        allowed: HashSet::new(),
        members: HashSet::new(),
        aliases: Vec::new(),
    };
    collector.visit_program(program);
    let Collector {
        mut found,
        wrappers,
        calls,
        called: by_name,
        mut allowed,
        members,
        aliases,
        ..
    } = collector;

    let scoping = exprs.scope.scoping;
    found.extend(
        classes
            .components
            .iter()
            .enumerate()
            .map(|(class, component)| Found {
                name: component.name.clone(),
                symbol: component.symbol,
                location: exprs.scope.lines.location(component.start),
                props: Vec::new(),
                class: Some(class),
            }),
    );
    let mut class_props = HashMap::new();
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
            forwards: Vec::new(),
            default_props: None,
        });
        if let Some(symbol) = found.symbol {
            by_symbol.insert(symbol, index);
        }
        if let Some(class) = found.class {
            class_props.insert(class, parameters.len());
            parameters.push(Parameter {
                component: index,
                position: 0,
                location: found.location,
                path: Vec::new(),
                default: None,
                initializer: None,
                rest: None,
                mutated: false,
            });
        }
        for leaf in found.props {
            parameter_of.insert(leaf.symbol, parameters.len());
            parameters.push(Parameter {
                component: index,
                position: leaf.position,
                location: exprs
                    .scope
                    .lines
                    .location(scoping.symbol_span(leaf.symbol).start),
                path: leaf.path,
                default: None,
                initializer: None,
                rest: leaf.rest,
                mutated: mutated(scoping, changed, leaf.symbol),
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
    for called in calls.into_iter().flatten() {
        if allowed.contains(&called.argument) {
            continue;
        }
        let Some(symbol) = scoping.get_reference(called.argument).symbol_id() else {
            continue;
        };
        let to = match (by_symbol.get(&symbol), exprs.scope.declared(symbol)) {
            (Some(&index), _) if called.members.is_empty() => Forwarded::Component(index),
            (None, Declared::Import(import)) => Forwarded::Import {
                import,
                members: called.members,
            },
            _ => continue,
        };
        let lines = &exprs.scope.lines;
        if let Some(symbol) = called.symbol {
            by_symbol.insert(symbol, components.len());
        }
        components.push(Component {
            name: called.name,
            location: lines.location(called.start),
            variables: called
                .symbol
                .map(|symbol| exprs.variable(symbol))
                .into_iter()
                .collect(),
            default: called.symbol.is_none(),
            escapes: false,
            forwards: vec![Forward {
                location: lines.location(called.at),
                to,
                excluded: Vec::new(),
            }],
            default_props: None,
        });
    }

    let calls = by_name
        .into_iter()
        .filter_map(|call| {
            let symbol = scoping.get_reference(call.reference).symbol_id()?;
            let callee = match (by_symbol.get(&symbol), exprs.scope.declared(symbol)) {
                (Some(&index), _) => Callee::Component(index),
                (None, Declared::Import(import)) => Callee::Import(import),
                _ => return None,
            };
            allowed.insert(call.reference);
            Some((callee, call))
        })
        .collect();
    *exprs.parameters.borrow_mut() = parameter_of;
    *exprs.class_props.borrow_mut() = class_props;
    let mut defaults = Defaults {
        exprs,
        parameters: &mut parameters,
    };
    defaults.visit_program(program);

    let escapes_with = |allowed: &HashSet<ReferenceId>, symbol, members: &HashSet<ReferenceId>| {
        scoping
            .get_resolved_reference_ids(symbol)
            .iter()
            .any(|&id| {
                !allowed.contains(&id)
                    && !members.contains(&id)
                    && scoping.get_reference(id).is_value()
            })
    };
    // A name that holds an import and doesn't escape is the import, so declaring it doesn't make
    // the import escape: `Icon` in `const I = Icon; <I />`.
    if !aliases.is_empty() {
        let names: HashSet<SymbolId> = aliases.iter().map(|&(name, _)| name).collect();
        let mut added = true;
        while added {
            added = false;
            for &(name, reference) in &aliases {
                let Some(root) = scoping.get_reference(reference).symbol_id() else {
                    continue;
                };
                let import = matches!(exprs.scope.declared(root), Declared::Import(_));
                if !allowed.contains(&reference)
                    && (import || names.contains(&root))
                    && !mutated(scoping, changed, name)
                    && !escapes_with(&allowed, name, &HashSet::new())
                {
                    allowed.insert(reference);
                    added = true;
                }
            }
        }
    }
    let escapes =
        |symbol: SymbolId, members: &HashSet<ReferenceId>| escapes_with(&allowed, symbol, members);
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
        calls,
        escaping_imports,
    }
}

/// A function found to be a component.
struct Found {
    name: String,
    symbol: Option<SymbolId>,
    location: model::Location,
    props: Vec<Leaf>,
    /// Index into [`Classes::components`], whose props are `this.props`.
    class: Option<usize>,
}

struct Collector<'e, 's> {
    exprs: &'e Exprs<'s>,
    in_const: bool,
    in_type: bool,
    found: Vec<Found>,
    /// `const Wrapper = memo(Inner)`, or `export default memo(Inner)` without a symbol: the
    /// wrapper's symbol and the reference to `Inner`.
    wrappers: Vec<(Option<SymbolId>, ReferenceId)>,
    /// When recorded, `const`s and default exports holding a call that passes a name on.
    calls: Option<Vec<Called>>,
    /// Calls whose callee is a name.
    called: Vec<ByName>,
    /// References that don't make a name escape: JSX tags, exports and types.
    allowed: HashSet<ReferenceId>,
    /// Objects of member expressions, such as `Name` in `Name.displayName = …`: they don't make
    /// a component escape, but do an import, whose member can be a component.
    members: HashSet<ReferenceId>,
    /// Names declared as a name or static property accesses on one, directly or with static
    /// keys: `I` and the reference to `Icon` in `const I = Icon`, or `const { Icon: I } = Lib`.
    aliases: Vec<(SymbolId, ReferenceId)>,
}

/// `name(…)`.
pub(crate) struct ByName {
    reference: ReferenceId,
    pub(crate) text: String,
    pub(crate) start: u32,
    /// The spans of the arguments, with whether each is spread.
    pub(crate) arguments: Vec<(bool, Span)>,
}

/// `const Styled = wrap(Button)`, or `export default wrap(Button)` without a symbol.
struct Called {
    name: String,
    symbol: Option<SymbolId>,
    /// The name's start, or the expression's.
    start: u32,
    argument: ReferenceId,
    members: Vec<String>,
    /// The argument's start.
    at: u32,
}

/// The name a call passes on as its first argument, through member calls and tagged templates
/// on its result and calls in the argument: `Button` in `` wrap(Button).with(…)`…` `` or
/// `wrap(memo(Button))`. With its property accesses and its start.
fn passed(expression: &Expression) -> Option<(ReferenceId, Vec<String>, u32)> {
    match expression.get_inner_expression() {
        Expression::TaggedTemplateExpression(tagged) => passed(&tagged.tag),
        Expression::CallExpression(call) => {
            if let Some(member) = call.callee.get_inner_expression().as_member_expression()
                && matches!(
                    member.object().get_inner_expression(),
                    Expression::CallExpression(_) | Expression::TaggedTemplateExpression(_)
                )
            {
                return passed(member.object());
            }
            let argument = call.arguments.first()?.as_expression()?;
            named(argument).or_else(|| passed(argument))
        }
        _ => None,
    }
}

/// An identifier, or static property accesses on one.
fn named(expression: &Expression) -> Option<(ReferenceId, Vec<String>, u32)> {
    let mut members = Vec::new();
    let mut current = expression.get_inner_expression();
    loop {
        match current {
            Expression::Identifier(identifier) => {
                members.reverse();
                let start = expression.get_inner_expression().span().start;
                return Some((identifier.reference_id(), members, start));
            }
            Expression::StaticMemberExpression(member) => {
                members.push(member.property.name.to_string());
                current = member.object.get_inner_expression();
            }
            _ => return None,
        }
    }
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
        let props = params
            .items
            .iter()
            .enumerate()
            .flat_map(|(position, parameter)| {
                leaves(&parameter.pattern)
                    .into_iter()
                    .filter(|leaf| !leaf.inner_default)
                    .map(move |leaf| Leaf {
                        symbol: leaf.symbol,
                        position,
                        path: leaf.path,
                        rest: leaf.rest,
                    })
            })
            .collect();
        self.found.push(Found {
            name,
            symbol,
            location: self.exprs.scope.lines.location(start),
            props,
            class: None,
        });
    }

    fn call_default(&mut self, expression: &Expression) {
        let start = expression.span().start;
        self.call("default".to_owned(), None, start, expression);
    }

    fn call(&mut self, name: String, symbol: Option<SymbolId>, start: u32, held: &Expression) {
        if let Some(calls) = &mut self.calls
            && let Some((argument, members, at)) = passed(held)
        {
            calls.push(Called {
                name,
                symbol,
                start,
                argument,
                members,
                at,
            });
        }
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
        if let Some((reference, _, _)) = declarator.init.as_ref().and_then(named) {
            match &declarator.id {
                BindingPattern::BindingIdentifier(id) => {
                    self.aliases.push((id.symbol_id(), reference));
                }
                BindingPattern::ObjectPattern(pattern) if pattern.rest.is_none() => {
                    for property in &pattern.properties {
                        if !property.computed
                            && let BindingPattern::BindingIdentifier(id) = &property.value
                        {
                            self.aliases.push((id.symbol_id(), reference));
                        }
                    }
                }
                _ => {}
            }
        }
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
                held => {
                    if let Some(Held::Name(argument)) = held {
                        self.wrappers.push((Some(id.symbol_id()), argument));
                    }
                    self.call(
                        id.name.to_string(),
                        Some(id.symbol_id()),
                        id.span.start,
                        init,
                    );
                }
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
                        held => {
                            if let Some(Held::Name(argument)) = held {
                                self.wrappers.push((None, argument));
                            }
                            self.call_default(expression);
                        }
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

    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        if let Expression::Identifier(callee) = call.callee.get_inner_expression() {
            self.called.push(ByName {
                reference: callee.reference_id(),
                text: callee.name.to_string(),
                start: call.span.start,
                arguments: call
                    .arguments
                    .iter()
                    .map(|argument| match argument {
                        Argument::SpreadElement(spread) => (true, spread.argument.span()),
                        argument => (false, argument.to_expression().span()),
                    })
                    .collect(),
            });
        }
        walk::walk_call_expression(self, call);
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
    fn visit_formal_parameter(&mut self, parameter: &FormalParameter<'a>) {
        if let Some(initializer) = &parameter.initializer {
            let indices: Vec<usize> = {
                let parameters = self.exprs.parameters.borrow();
                leaves(&parameter.pattern)
                    .iter()
                    .filter_map(|leaf| parameters.get(&leaf.symbol).copied())
                    .collect()
            };
            if !indices.is_empty() {
                let initializer = self.exprs.expr(initializer);
                for index in indices {
                    self.parameters[index].initializer = Some(initializer.clone());
                }
            }
        }
        walk::walk_formal_parameter(self, parameter);
    }

    fn visit_assignment_pattern(&mut self, pattern: &AssignmentPattern<'a>) {
        if let BindingPattern::BindingIdentifier(identifier) = &pattern.left {
            let index = self
                .exprs
                .parameters
                .borrow()
                .get(&identifier.symbol_id())
                .copied();
            if let Some(index) = index {
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

/// A name declared by a parameter.
struct Leaf {
    symbol: SymbolId,
    position: usize,
    path: Vec<Segment>,
    rest: Option<Vec<String>>,
}
