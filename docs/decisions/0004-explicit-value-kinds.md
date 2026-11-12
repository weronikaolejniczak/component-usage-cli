---
status: accepted
date: 2026-11-12
decision-makers: Weronika Olejniczak
---

# Report every value with an explicit kind and a trace

## Context and Problem Statement

Static analysis can't resolve every prop value. What should the output contain for values it can't resolve?

## Decision Drivers

* Results are used to decide whether a change breaks consumers; a missing value is worse than an unknown one.
* Results must be checkable by hand.

## Considered Options

* Explicit value kinds with reasons and traces
* Report only resolved values
* Best-effort guesses

## Decision Outcome

Chosen option: "Explicit value kinds with reasons and traces".

Every value has exactly one kind:

| Kind | Meaning |
|---|---|
| `literal` | Resolved to a concrete primitive |
| `component` | Resolved to a component reference |
| `undefined` | Prop not passed or resolved to `undefined` |
| `runtime` | Depends on runtime data (function call results, API responses, user input) |
| `spread` | Passed through `{...props}`; contents not statically known |
| `unresolved` | Not handled by the analyzer; includes a reason code |

Each non-literal value includes a trace: the ordered `file:line:column` locations visited to resolve it.

### Consequences

* Good, because for every queried prop the counts across kinds add up to the number of call sites.
* Good, because `unresolved` reasons show which patterns to support next.
* Bad, because output is larger than a list of resolved values.

### Confirmation

Fixture tests assert the kind, reason and trace for each pattern. Benchmarks report counts per kind.

## Pros and Cons of the Options

### Report only resolved values

* Good, because smaller output.
* Bad, because there's no way to tell whether a value is missing.

### Best-effort guesses

* Bad, because a wrong guess can't be told apart from a correct result.
