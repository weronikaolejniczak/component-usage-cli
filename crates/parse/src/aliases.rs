use crate::{Alias, AliasRoot, Definition, ExprKind, Key, Reference, Segment};

/// Follows the name defined at `index` through other names back to an import or `require`, with
/// the property accesses and destructured keys on the way: `const { Icon } = Lib` is `Lib` with
/// members `["Icon"]`. Names that refer to each other in a cycle, to anything else, or through a
/// key that isn't static or an array pattern, aren't aliases.
pub(crate) fn alias(definitions: &[Definition], mut index: usize) -> Option<Alias> {
    let mut members = Vec::new();
    let mut seen = Vec::new();
    loop {
        if seen.contains(&index) {
            return None;
        }
        seen.push(index);
        let Definition::Value { init, path, .. } = definitions.get(index)? else {
            return None;
        };
        let (root, accesses) = match &init.kind {
            ExprKind::Reference { name, path, .. } => (Some(*name), path),
            ExprKind::Require { path, .. } => (None, path),
            _ => return None,
        };
        let accesses = accesses.iter().map(|key| match key {
            Key::Static(key) => Some(key.clone()),
            Key::Dynamic => None,
        });
        let keys = path.iter().map(|segment| match segment {
            Segment::Key(key) => Some(key.clone()),
            Segment::Index(_) => None,
        });
        let mut step = accesses.chain(keys).collect::<Option<Vec<String>>>()?;
        step.append(&mut members);
        members = step;
        let root = match (root, &init.kind) {
            (None, ExprKind::Require { specifier, .. }) => AliasRoot::Require(specifier.clone()),
            (Some(Reference::Import(import)), _) => AliasRoot::Import(import),
            (Some(Reference::Variable(next)), _) => {
                index = next;
                continue;
            }
            _ => return None,
        };
        return Some(Alias { root, members });
    }
}
