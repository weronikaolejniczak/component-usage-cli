use crate::{
    Attribute, CallbackParameter, Callee, Definition, Entry, Exported, Expr, ExprKind, FileSummary,
    Item, Member, Part, Reference,
};

pub(crate) fn value_imports(summary: &FileSummary, attributes: bool) -> Vec<usize> {
    let mut reads = Reads {
        definitions: &summary.definitions,
        callbacks: &summary.callbacks,
        read_callbacks: vec![false; summary.callbacks.len()],
        members: &summary.members,
        read_members: vec![false; summary.members.len()],
        visited: vec![false; summary.definitions.len()],
        imports: summary
            .changed_imports
            .iter()
            .chain(&summary.escaping_imports)
            .copied()
            .collect(),
    };
    let defaults = summary.parameters.iter().flat_map(|parameter| {
        parameter
            .default
            .as_ref()
            .into_iter()
            .chain(&parameter.initializer)
    });
    let default_props = summary
        .components
        .iter()
        .filter_map(|component| component.default_props.as_ref());
    for default in defaults.chain(default_props) {
        reads.expr(default);
    }
    for call in &summary.calls {
        if let Callee::Import(import) = call.callee {
            reads.imports.push(import);
        }
        for argument in &call.arguments {
            match argument {
                Item::Value(value) | Item::Spread(value) => reads.expr(value),
            }
        }
    }
    for exported in summary.exported.values() {
        match exported {
            Exported::Variable(index) => reads.definition(*index),
            Exported::Value(expr) => reads.expr(expr),
        }
    }
    if attributes {
        for attribute in summary
            .elements
            .iter()
            .flat_map(|element| &element.attributes)
        {
            match attribute {
                Attribute::Named { value, .. } => reads.expr(value),
                Attribute::Spread(spread) => reads.expr(spread),
            }
        }
    }
    let mut imports = reads.imports;
    imports.sort_unstable();
    imports.dedup();
    imports
}

struct Reads<'s> {
    definitions: &'s [Definition],
    callbacks: &'s [CallbackParameter],
    read_callbacks: Vec<bool>,
    members: &'s [Member],
    read_members: Vec<bool>,
    visited: Vec<bool>,
    imports: Vec<usize>,
}

impl Reads<'_> {
    fn definition(&mut self, index: usize) {
        if self.visited.get(index).is_none_or(|visited| *visited) {
            return;
        }
        self.visited[index] = true;
        if let Definition::Value { init, default, .. } = &self.definitions[index] {
            self.expr(init);
            if let Some(default) = default {
                self.expr(default);
            }
        }
    }

    fn callback(&mut self, index: usize) {
        if self.read_callbacks.get(index).is_none_or(|read| *read) {
            return;
        }
        self.read_callbacks[index] = true;
        let callback = &self.callbacks[index];
        self.expr(&callback.receiver);
        if let Some(default) = &callback.default {
            self.expr(default);
        }
    }

    fn member(&mut self, index: usize) {
        if self.read_members.get(index).is_none_or(|read| *read) {
            return;
        }
        self.read_members[index] = true;
        if let Member::Value(value) = &self.members[index] {
            self.expr(value);
        }
    }

    fn expr(&mut self, expr: &Expr) {
        match &expr.kind {
            ExprKind::Reference { name, .. } => match name {
                Reference::Import(import) => self.imports.push(*import),
                Reference::Variable(index) => self.definition(*index),
                Reference::Callback(index) => self.callback(*index),
                Reference::Member(index) => self.member(*index),
                Reference::Parameter(_) | Reference::State | Reference::Global => {}
            },
            ExprKind::Object(entries) => {
                for entry in entries {
                    match entry {
                        Entry::Property { value, .. } | Entry::Computed { value, .. } => {
                            self.expr(value);
                        }
                        Entry::Spread(spread) => self.expr(spread),
                    }
                }
            }
            ExprKind::Array(items) => {
                for item in items {
                    match item {
                        Item::Value(value) | Item::Spread(value) => self.expr(value),
                    }
                }
            }
            ExprKind::Template { parts, .. } => {
                for part in parts {
                    if let Part::Value(value) = part {
                        self.expr(value);
                    }
                }
            }
            ExprKind::OneOf(values)
            | ExprKind::Operation {
                operands: values, ..
            } => {
                for value in values {
                    self.expr(value);
                }
            }
            ExprKind::Call {
                call: Some(call), ..
            } => {
                if let Some(receiver) = &call.receiver {
                    self.expr(receiver);
                }
            }
            ExprKind::Literal(_)
            | ExprKind::Undefined
            | ExprKind::Element { .. }
            | ExprKind::Function { .. }
            | ExprKind::Require { .. }
            | ExprKind::Call { .. }
            | ExprKind::Unsupported { .. } => {}
        }
    }
}
