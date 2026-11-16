---
status: accepted
date: 2026-11-16
decision-makers: Weronika Olejniczak
---

# Follow variables on request, and report the path to each value

Supersedes [ADR-0005](./0005-choose-what-to-follow.md).

## Context and Problem Statement

Many prop values are names: `iconType={icon}`, `css={styles.panel}`, `options={OPTIONS}`. [ADR-0017](./0017-prop-values.md) reports them as `unresolved` with code `not-followed`. Following them answers more, but costs time and memory, and the value found is somewhere else in the file. How does the user choose what the scan follows, and how does the report show a value that was found by following a name?

## Decision Drivers

* The default report must be cheap and show what's written at the usage.
* A followed value must say where it is and how it was reached, so a tool or agent can change the definition or the usage.
* No silent misses: a binding that can't be followed is reported with a reason.

## Considered Options

* Opt-in `--follow` steps; a followed value at its definition with a `via` chain
* Default `--follow variables,imports`, as in [ADR-0005](./0005-choose-what-to-follow.md)
* Replace the reference with the value, located at its definition
* Keep the reference, with the value nested in it

## Decision Outcome

Chosen option: "Opt-in `--follow` steps, with a `via` chain", because the default stays what's written, and a followed value keeps both its own position and the positions of the names that led to it.

`--follow` takes a comma-separated list of steps; nothing is followed without it.

| Step | Follows | Status |
|---|---|---|
| `variables` | Same-file bindings | Implemented |
| `imports` | Imported bindings to the exported value in another file | Planned |
| `props` | A component's prop to what each call site of that component passes, recursively | Planned |

`none` follows nothing; `all` will mean every step once they exist. `--wrappers` stays planned as in [ADR-0005](./0005-choose-what-to-follow.md).

With `variables`, a name declared in the file resolves to:

* its initialiser, for `const`, and for `let` and `var` that are never assigned to again;
* the matching part of its initialiser, for destructuring with static keys and positions, such as `const { edit } = ICONS`, using the default when the key is missing;
* `function`, at the declaration, for a function declaration.

Member access on a followed value is evaluated: `ICONS.edit`, `ICONS['edit']` and `SIZES[0]` give the entry; a key that isn't written gives `undefined`, located at the object, unless a spread or computed key could set it, which gives `unsupported`. A key that isn't static, as in `ICONS[status]`, gives `one-of` every value of the object or array, located at it.

A followed value has the `line` and `column` of the value itself, and a `via` list of the names followed to reach it, outermost first: `{ "text", "line", "column" }` with each name's source text and position. Values that weren't followed have no `via`.

New `unresolved` code, in addition to [ADR-0017](./0017-prop-values.md):

| Code | Meaning |
|---|---|
| `mutated` | A binding that is assigned to after its declaration, declared more than once, or whose contents are changed in the file: assigned to, `delete`d, or passed to `Object.assign` or a mutating array method such as `push` |

Other declarations, such as enums, classes and loop variables, stay `not-followed` with step `variables` until supported. A name that refers back to itself while being followed is `unsupported`.

### Consequences

* Good, because the default report is unchanged and as cheap as before.
* Good, because `via` lets a tool change the usage, the definition, or anything in between.
* Good, because `ICONS[status]` lists every icon the usage can receive.
* Bad, because mutation through functions in other files, or functions that aren't recognised, isn't seen.
* Bad, because a value shared by many usages is repeated in each.

### Confirmation

A fixture for `--follow variables` covering each form and code above. The existing fixtures without `--follow` don't change. Benchmarks report time, memory and counts per kind with and without it.

## Pros and Cons of the Options

### Default `--follow variables,imports`

* Good, because more is answered without options.
* Bad, because every scan pays for following, even when only literals matter.

### Replace the reference with the value

* Good, because smaller output.
* Bad, because the usage's position of the name is lost, so the usage can't be changed from the report.

### Keep the reference, with the value nested

* Good, because the reference stays where it was.
* Bad, because every reader has to unwrap references to reach a value, at every level.
