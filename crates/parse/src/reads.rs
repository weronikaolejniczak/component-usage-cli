use crate::{
    Attribute, Definition, Entry, Exported, Expr, ExprKind, FileSummary, Item, Part, Reference,
};

pub(crate) fn value_imports(summary: &FileSummary, attributes: bool) -> Vec<usize> {
    let mut reads = Reads {
        definitions: &summary.definitions,
        visited: vec![false; summary.definitions.len()],
        imports: summary
            .changed_imports
            .iter()
            .chain(&summary.escaping_imports)
            .copied()
            .collect(),
    };
    for default in summary
        .parameters
        .iter()
        .filter_map(|parameter| parameter.default.as_ref())
    {
        reads.expr(default);
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

    fn expr(&mut self, expr: &Expr) {
        match &expr.kind {
            ExprKind::Reference { name, .. } => match name {
                Reference::Import(import) => self.imports.push(*import),
                Reference::Variable(index) => self.definition(*index),
                Reference::Parameter(_) | Reference::Global => {}
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
            ExprKind::OneOf(values) => {
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
