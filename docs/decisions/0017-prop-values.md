---
status: accepted; operators superseded by ADR-0036
date: 2026-11-13
decision-makers: Weronika Olejniczak
---

# Report every prop of a usage as a tree of values with kinds

Supersedes [ADR-0004](./0004-explicit-value-kinds.md).

## Context and Problem Statement

[ADR-0004](./0004-explicit-value-kinds.md) gives every value one flat kind. Props are often objects, arrays and templates whose contents are the answer: `singleSelection={{ asPlainText: true }}`, `options={[{ label, value }]}`, `` css={css`color: red`} ``, `{ name: 'Edit', icon: 'pencil' }` in a list of items. A flat kind can only say "object". And a ternary has more than one value. How are prop values shaped, and which props of a usage are reported?

## Decision Drivers

* No silent misses: a value is resolved or reported with a reason ([ADR-0004](./0004-explicit-value-kinds.md)).
* The contents of objects, arrays and templates have to be queryable, with the position of each value, so a tool or agent can change it.
* Questions about how a component is used need every prop, not only the ones asked about.

## Considered Options

* A tree of values: objects, arrays and templates contain values
* Flat kinds, with objects and templates as source text

## Decision Outcome

Chosen option: "A tree of values", because it keeps the contents structured and positioned at any depth.

Each usage lists its attributes in source order, so later ones can be seen to override earlier ones: `{ "name": "type", "value": … }` or `{ "spread": … }`. All attributes are listed unless `--props` names some; spreads are always listed, since they may set any prop. Each usage also lists its `parents`: the enclosing usages, nearest first, with the attribute of the parent the usage is in, if any, such as `button` for `<Popover button={<Button />}>`.

Every value has a `kind`, a `line` and a `column`:

| Kind | Meaning | Fields |
|---|---|---|
| `literal` | A string, number, boolean or `null` | `value` |
| `undefined` | `undefined`, `void 0` or an empty expression | |
| `object` | An object literal | `entries`: `{ "key", "value" }`, `{ "computed", "value" }` for a key that isn't static, with the key's source text, or `{ "spread" }` |
| `array` | An array literal | `items`: `{ "value" }` or `{ "spread" }` |
| `template` | A template literal with expressions, or any tagged template | `tag`: the tag's source text, if any; `parts`: `{ "text" }` or `{ "value" }` |
| `element` | A JSX element or fragment | `tag`: the tag as written; absent for a fragment |
| `function` | A function or arrow function | |
| `one-of` | Any of several values: both branches of `?:`, both sides of `\|\|`, `&&` and `??` | `values` |
| `component` | A reference to a component | |
| `runtime` | Known to depend on runtime data | `code`, `text` |
| `unresolved` | Not resolved | `code`, `text`: the expression's source |

A template literal without expressions is a `literal`. A ternary or logical expression is `one-of` its operands. Operands aren't evaluated yet, so both are listed even when one can't be the result, as in `'a' || 'b'`.

`unresolved` codes are stable:

| Code | Meaning |
|---|---|
| `not-followed` | A binding the scan would follow with the step in `step` ([ADR-0005](./0005-choose-what-to-follow.md)), which is off or not implemented yet: `variables` for a binding declared in the file, `imports` for an imported one |
| `parameter` | A function parameter, or a property of one |
| `global` | A name not declared in the file, other than `undefined` |
| `call` | The result of a call; calls aren't evaluated |
| `unsupported` | Any other expression, such as `a + b` or `this.props.x` |

Member expressions such as `item.icon` or `ICONS[status]` take the code of the name they start with.

`component` and `runtime` are emitted once the scan follows bindings across files.

### Consequences

* Good, because questions about objects, templates and lists of items are answered from the report, with the position of every value.
* Good, because `--props` keeps the report small when only some props matter.
* Bad, because reports with every prop are large.

### Confirmation

A fixture per kind and per code. Benchmarks report counts per kind and code.

## Pros and Cons of the Options

### Flat kinds with source text

* Good, because smaller output.
* Bad, because the contents have to be parsed again by whoever reads the report.
