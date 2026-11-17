---
status: accepted
date: 2026-11-17
decision-makers: Weronika Olejniczak
---

# Keep the references of wrappers nothing renders

Supersedes, in [ADR-0027](./0027-references.md), that with `--wrappers` the argument of a call that makes a wrapper isn't a reference.

## Context and Problem Statement

With `--wrappers`, a `const` holding a call whose first argument is a target is a wrapper ([ADR-0026](./0026-wrappers.md)), and [ADR-0027](./0027-references.md) leaves its argument out of `references`, since the wrapper's usages stand for it. Many such `const`s aren't components: in tests, `const select = wrapper.find(Target)` looks the target up in a rendered tree. Nothing renders them, so they have no usages, and with `--wrappers` the place the target is passed disappears from the report. In the [benchmarks](../../benchmarks/README.md), 16 of the 39 references `--wrappers` removes are of this kind. When is a wrapper's argument left out?

## Considered Options

* Only when a reported usage goes through it
* Always, as before
* Never

## Decision Outcome

Chosen option: "Only when a reported usage goes through it", because then every place the target is passed is in the report once: as a step in a usage's `through`, or as a reference.

With `--wrappers`, a reference is left out when its position is a step of a usage's `through`. Without `--wrappers`, nothing changes.

### Consequences

* Good, because `--wrappers` doesn't hide references.
* Neutral, because a wrapper rendered only in files outside the scan keeps its reference.

## Pros and Cons of the Options

### Always, as before

* Bad, because a call that isn't a component hides the reference without a usage in its place.

### Never

* Good, because `references` is the same with and without `--wrappers`.
* Bad, because each passed target that wrappers stand for is in the report twice.
