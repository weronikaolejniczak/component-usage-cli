use crate::{Alias, AliasRoot, Definition, ExprKind, Key, Reference, Segment};

/// Follows the name defined at `index` through other names back to an import or `require`, with
/// the property accesses and destructured keys on the way: `const { Icon } = Lib` is `Lib` with
/// members `["Icon"]`. Names that refer to each other in a cycle, to anything else, or through a
/// key that isn't static or an array pattern, aren't aliases. `reassigned` says whether the name
/// defined at an index may hold something other than its declaration; a name with a default, as
/// in `const { Icon = Fallback } = Lib`, may too.
pub(crate) fn alias(
    definitions: &[Definition],
    mut index: usize,
    reassigned: impl Fn(usize) -> bool,
) -> Option<Alias> {
    let mut members = Vec::new();
    let mut seen = Vec::new();
    let mut any_reassigned = false;
    loop {
        if seen.contains(&index) {
            return None;
        }
        seen.push(index);
        any_reassigned |= reassigned(index);
        let Definition::Value {
            init,
            path,
            default,
        } = definitions.get(index)?
        else {
            return None;
        };
        any_reassigned |= default.is_some();
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
        return Some(Alias {
            root,
            members,
            location: init.location,
            reassigned: any_reassigned,
        });
    }
}
