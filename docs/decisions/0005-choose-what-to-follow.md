---
status: superseded by ADR-0019
date: 2026-11-12
decision-makers: Weronika Olejniczak
---

# Let the user choose what the scan follows to find values

## Context and Problem Statement

A prop value can be written on the tag, stored in a variable, imported from another file or passed in as a prop by the component's callers. Following each of these costs time. How does the user control how far a scan goes, in terms they understand without reading the docs?

## Decision Drivers

* Some questions only need literals; others need values passed in by callers.
* The option must read as plain language in the command.
* Each kind of step must be testable on its own.

## Considered Options

* `--follow` with a comma-separated list of steps
* Named levels (`--level none|local|cross|flow`)
* Always follow everything

## Decision Outcome

Chosen option: "`--follow` with a comma-separated list", because each value names what the scan follows and when a value stops resolving the reason names the step that was off.

| Value | Follows |
|---|---|
| `variables` | Same-file bindings: `const`, object/array lookups with static keys, enum members, local functions with literal returns |
| `imports` | Imported bindings to the exported value in another file |
| `props` | A component's prop to what each call site of that component passes, recursively |

* `none`: literals only. `all`: every step.
* Default: `variables,imports`.
* Always evaluated, regardless of `--follow`: literals, template literals, ternaries, `??`, `||`, `&&`.
* `--wrappers` also treats components that forward a prop to a target as targets, including through `memo`, `forwardRef` and `styled`.
* Limits: `--max-depth`, cycle detection, memoization per symbol.

### Consequences

* Good, because `--follow imports` and `--follow props` read as what they do.
* Good, because a value that stops at a disabled step is reported as `unresolved` with that step as the reason, for example `imports not followed` (see [ADR-0004](./0004-explicit-value-kinds.md)).
* Bad, because more combinations to test than a fixed set of levels.

### Confirmation

A fixture per step, plus fixtures for combinations. Benchmarks report time and counts per kind for each combination measured.

## Pros and Cons of the Options

### Named levels

* Good, because fewer combinations.
* Bad, because names like `local`, `cross`, `flow` don't say what they include without the docs.

### Always follow everything

* Bad, because no way to trade depth for time.
