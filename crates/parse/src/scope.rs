use std::collections::{HashMap, HashSet};

use oxc_ast::ast::{
    BindingIdentifier, CatchParameter, FormalParameters, FunctionBody, IdentifierReference, Program,
};
use oxc_ast_visit::{Visit, walk};
use oxc_semantic::{Scoping, SymbolId};

use crate::lines::LineIndex;

/// What a name refers to, shared by everything that reads names: JSX tags, attribute values and
/// definitions.
pub(crate) struct Scope<'s> {
    pub(crate) scoping: &'s Scoping,
    pub(crate) lines: &'s LineIndex<'s>,
    /// Value imports by the start of their local name, as indices into
    /// [`crate::FileSummary::imports`].
    imports: HashMap<u32, usize>,
    parameters: HashSet<SymbolId>,
}

pub(crate) enum Declared {
    /// Index into [`crate::FileSummary::imports`].
    Import(usize),
    /// A function or `catch` parameter, including a name destructured from one.
    Parameter,
    Local,
}

impl<'s> Scope<'s> {
    pub(crate) fn new(
        program: &Program,
        scoping: &'s Scoping,
        lines: &'s LineIndex<'s>,
        imports: HashMap<u32, usize>,
    ) -> Self {
        let mut parameters = Parameters {
            symbols: HashSet::new(),
            inside: false,
        };
        parameters.visit_program(program);
        Self {
            scoping,
            lines,
            imports,
            parameters: parameters.symbols,
        }
    }

    /// The declaration `identifier` refers to; `None` if it isn't declared in the file.
    pub(crate) fn symbol(&self, identifier: &IdentifierReference) -> Option<SymbolId> {
        self.scoping
            .get_reference(identifier.reference_id())
            .symbol_id()
    }

    pub(crate) fn declared(&self, symbol: SymbolId) -> Declared {
        if let Some(&import) = self.imports.get(&self.scoping.symbol_span(symbol).start) {
            Declared::Import(import)
        } else if self.parameters.contains(&symbol) {
            Declared::Parameter
        } else {
            Declared::Local
        }
    }
}

/// The names function and `catch` parameters declare.
struct Parameters {
    symbols: HashSet<SymbolId>,
    inside: bool,
}

impl<'a> Visit<'a> for Parameters {
    fn visit_formal_parameters(&mut self, parameters: &FormalParameters<'a>) {
        let outer = std::mem::replace(&mut self.inside, true);
        walk::walk_formal_parameters(self, parameters);
        self.inside = outer;
    }

    fn visit_catch_parameter(&mut self, parameter: &CatchParameter<'a>) {
        let outer = std::mem::replace(&mut self.inside, true);
        walk::walk_catch_parameter(self, parameter);
        self.inside = outer;
    }

    fn visit_function_body(&mut self, body: &FunctionBody<'a>) {
        let outer = std::mem::replace(&mut self.inside, false);
        walk::walk_function_body(self, body);
        self.inside = outer;
    }

    fn visit_binding_identifier(&mut self, identifier: &BindingIdentifier<'a>) {
        if self.inside {
            self.symbols.insert(identifier.symbol_id());
        }
    }
}
