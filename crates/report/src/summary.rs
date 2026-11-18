use std::collections::{BTreeMap, BTreeSet, HashMap};

use model::{Entry, Literal, Prop, Value, ValueKind};
use serde::Serialize;

use crate::{FileError, Report, UnidentifiedCode, Usage, Warning};

/// Counts per component, prop and owner instead of every usage. Sorted, so the same report
/// gives the same summary.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Summary {
    pub components: Vec<ComponentSummary>,
    /// Usages per owner over all components. Absent unless owners were requested.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owners: Option<Vec<OwnerCount>>,
    /// Usages in files without owners. Absent unless owners were requested.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unowned: Option<usize>,
    pub unidentified: Vec<UnidentifiedCount>,
    pub errors: Vec<FileError>,
    pub warnings: Vec<Warning>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ComponentSummary {
    pub component: String,
    pub usages: usize,
    pub files: usize,
    /// Usages with a spread.
    pub spreads: usize,
    pub references: usize,
    /// Usages through wrappers. Absent unless wrappers were requested.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub through: Option<usize>,
    pub props: Vec<PropCount>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owners: Option<Vec<OwnerCount>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unowned: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PropCount {
    pub name: String,
    pub usages: usize,
    /// The literal values usages give the prop, each counted once per usage.
    pub values: Vec<ValueCount>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ValueCount {
    pub value: Literal,
    pub usages: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OwnerCount {
    pub owner: String,
    pub usages: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UnidentifiedCount {
    pub code: UnidentifiedCode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub component: Option<String>,
    pub elements: usize,
}

/// What the scan was asked for, which decides what the summary lists.
#[derive(Debug, Clone, Copy, Default)]
pub struct Options<'a> {
    /// The targets given, each listed even without usages.
    pub components: Option<&'a [String]>,
    pub wrappers: bool,
    pub owners: bool,
}

impl Report {
    #[must_use]
    pub fn summarize(&self, options: Options) -> Summary {
        let mut names: BTreeSet<&str> = options
            .components
            .unwrap_or_default()
            .iter()
            .map(String::as_str)
            .collect();
        names.extend(self.usages.iter().map(|usage| usage.component.as_str()));
        names.extend(
            self.references
                .iter()
                .map(|reference| reference.component.as_str()),
        );
        names.extend(
            self.unidentified
                .iter()
                .filter_map(|unidentified| unidentified.component.as_deref()),
        );

        let mut usages: HashMap<&str, Vec<&Usage>> = HashMap::new();
        for usage in &self.usages {
            usages.entry(&usage.component).or_default().push(usage);
        }
        let mut references: HashMap<&str, usize> = HashMap::new();
        for reference in &self.references {
            *references.entry(&reference.component).or_default() += 1;
        }
        let components = names
            .into_iter()
            .map(|name| {
                let usages = usages.get(name).map_or(&[][..], Vec::as_slice);
                let (owners, unowned) = owners(usages);
                ComponentSummary {
                    component: name.to_owned(),
                    usages: usages.len(),
                    files: usages
                        .iter()
                        .map(|usage| &usage.file)
                        .collect::<BTreeSet<_>>()
                        .len(),
                    spreads: usages
                        .iter()
                        .filter(|usage| {
                            usage
                                .props
                                .iter()
                                .any(|prop| matches!(prop, Prop::Spread { .. }))
                        })
                        .count(),
                    references: references.get(name).copied().unwrap_or_default(),
                    through: options.wrappers.then(|| {
                        usages
                            .iter()
                            .filter(|usage| !usage.through.is_empty())
                            .count()
                    }),
                    props: props(usages),
                    owners: options.owners.then_some(owners),
                    unowned: options.owners.then_some(unowned),
                }
            })
            .collect();

        let all: Vec<&Usage> = self.usages.iter().collect();
        let (owners, unowned) = owners(&all);
        let mut unidentified: BTreeMap<(String, Option<&str>), (UnidentifiedCode, usize)> =
            BTreeMap::new();
        for element in &self.unidentified {
            let code = serde_json::to_string(&element.code).unwrap_or_default();
            unidentified
                .entry((code, element.component.as_deref()))
                .or_insert((element.code, 0))
                .1 += 1;
        }
        Summary {
            components,
            owners: options.owners.then_some(owners),
            unowned: options.owners.then_some(unowned),
            unidentified: unidentified
                .into_iter()
                .map(|((_, component), (code, elements))| UnidentifiedCount {
                    code,
                    component: component.map(str::to_owned),
                    elements,
                })
                .collect(),
            errors: self.errors.clone(),
            warnings: self.warnings.clone(),
        }
    }
}

/// Props by count, then name, each with its literal values by count, then JSON text. Props apply
/// in order, as React does: a prop set twice on a usage is counted once, with the value of the
/// last, and a spread of an object sets its properties. Any other spread may set any prop, so
/// the values of those set before it aren't counted, except a wrapper's spread that passes its
/// props on, which sets nothing: what it passes is counted on the usages through the wrapper.
fn props(usages: &[&Usage]) -> Vec<PropCount> {
    struct Counts {
        usages: usize,
        values: HashMap<String, (Literal, usize)>,
    }
    let mut counts: HashMap<&str, Counts> = HashMap::new();
    for usage in usages {
        let mut set: HashMap<&str, Option<&Value>> = HashMap::new();
        for prop in &usage.props {
            match prop {
                Prop::Named { name, value } => {
                    set.insert(name, Some(value));
                }
                Prop::Spread { forwards: true, .. } => {}
                Prop::Spread { spread, .. } => spread_into(spread, &mut set),
            }
        }
        for (name, value) in set {
            let entry = counts.entry(name).or_insert_with(|| Counts {
                usages: 0,
                values: HashMap::new(),
            });
            entry.usages += 1;
            let Some(value) = value else {
                continue;
            };
            let mut found = BTreeMap::new();
            literals(value, &mut found);
            for (text, literal) in found {
                entry.values.entry(text).or_insert((literal, 0)).1 += 1;
            }
        }
    }
    let mut props: Vec<(String, PropCount)> = counts
        .into_iter()
        .map(|(name, counts)| {
            let mut values: Vec<(String, ValueCount)> = counts
                .values
                .into_iter()
                .map(|(text, (value, usages))| (text, ValueCount { value, usages }))
                .collect();
            values.sort_by(|(a, x), (b, y)| y.usages.cmp(&x.usages).then_with(|| a.cmp(b)));
            let prop = PropCount {
                name: name.to_owned(),
                usages: counts.usages,
                values: values.into_iter().map(|(_, value)| value).collect(),
            };
            (name.to_owned(), prop)
        })
        .collect();
    props.sort_by(|(a, x), (b, y)| y.usages.cmp(&x.usages).then_with(|| a.cmp(b)));
    props.into_iter().map(|(_, prop)| prop).collect()
}

/// The literals `value` is, itself or as a branch of a `one-of`, by JSON text.
fn literals(value: &Value, found: &mut BTreeMap<String, Literal>) {
    match &value.kind {
        ValueKind::Literal { value } => {
            let text = serde_json::to_string(value).unwrap_or_default();
            found.entry(text).or_insert_with(|| value.clone());
        }
        ValueKind::OneOf { values } => {
            for value in values {
                literals(value, found);
            }
        }
        _ => {}
    }
}

/// Usages per owner, by count, then name, and the usages in files without owners.
fn owners(usages: &[&Usage]) -> (Vec<OwnerCount>, usize) {
    let mut counts: HashMap<&str, usize> = HashMap::new();
    let mut unowned = 0;
    for usage in usages {
        match usage.owners.as_deref() {
            Some([]) | None => unowned += 1,
            Some(owners) => {
                for owner in owners {
                    *counts.entry(owner).or_default() += 1;
                }
            }
        }
    }
    let mut owners: Vec<OwnerCount> = counts
        .into_iter()
        .map(|(owner, usages)| OwnerCount {
            owner: owner.to_owned(),
            usages,
        })
        .collect();
    owners.sort_by(|a, b| b.usages.cmp(&a.usages).then_with(|| a.owner.cmp(&b.owner)));
    (owners, unowned)
}

/// Applies a spread of `value` to the props `set` so far: an object sets its properties in
/// order, `undefined` and `null` set nothing, and anything else may set any prop.
fn spread_into<'v>(value: &'v Value, set: &mut HashMap<&'v str, Option<&'v Value>>) {
    match &value.kind {
        ValueKind::Object { entries } => {
            for entry in entries {
                match entry {
                    Entry::Property { key, value } => {
                        set.insert(key, Some(value));
                    }
                    Entry::Spread { spread } => spread_into(spread, set),
                    Entry::Computed { .. } => set.values_mut().for_each(|value| *value = None),
                }
            }
        }
        ValueKind::Undefined
        | ValueKind::Literal {
            value: Literal::Null,
        } => {}
        _ => set.values_mut().for_each(|value| *value = None),
    }
}
