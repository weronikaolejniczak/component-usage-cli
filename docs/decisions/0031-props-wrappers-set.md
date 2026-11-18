---
status: accepted
date: 2026-11-18
decision-makers: Weronika Olejniczak
---

# Add the props wrappers set to the usages that go through them

Supersedes, in [ADR-0026](./0026-wrappers.md), that a usage through wrappers has only the attributes of its own element.

## Context and Problem Statement

With `--wrappers`, a usage through wrappers reports only its caller's attributes ([ADR-0026](./0026-wrappers.md)). A wrapper often sets props itself, as in `<Target size="s" {...props} />` or `<Target options={toOptions(fields)} {...rest} />`, so those usages miss props the target is given. In the select use case ([benchmarks](../../benchmarks/README.md)), usages through wrappers had to be counted apart, because the wrappers set `options` and `selectedOptions`. What are the props of a usage through wrappers?

## Decision Drivers

* A usage's props are what the target is given, as far as the scan can tell.
* Each value can be checked at its position.
* A wrapper's own props are what this caller passes, not what every caller passes.

## Considered Options

* The props in the order React applies them, with the wrapper's props followed to this caller
* The props in the order React applies them, with the wrapper's props followed to every caller
* The caller's attributes only, as before

## Decision Outcome

Chosen option: "The props in the order React applies them, with the wrapper's props followed to this caller", because then a usage through wrappers reads like an element of the target: a later prop with the same name wins.

* Starting from the caller's attributes, at each wrapper in `through`, outermost first, the props so far lose the keys its rest pattern takes, then take the place of the spread that passes them on, between the attributes before and after it on the wrapper's element. A wrapper made by a call adds nothing.
* Attributes of a wrapper's element are evaluated as in the full report, with `file` when they're in another file than the usage. With `--follow props`, a wrapper's own props, such as `tone` in `<Target color={tone} {...rest} />`, are followed to the element this usage goes through, the caller or the wrapper before, and only to it: no `one-of` of every caller, and no `other-callers`.
* `--props` keeps the named props among them, and spreads.

### Consequences

* Good, because usages through wrappers can be counted with the others.
* Good, because a prop set twice, by the wrapper and the caller, shows both, in the order that decides which applies.
* Bad, because a usage through wrappers has props from several files and elements, which the reader tells apart by `file` and position.

### Confirmation

A props fixture with props set before and after the spread, a rest pattern, a wrapper's prop followed to the caller, and a wrapper inside another.

## Pros and Cons of the Options

### The props in the order React applies them, with the wrapper's props followed to every caller

* Good, because the wrapper's attributes are the same as in the wrapper's own usage.
* Bad, because a usage gets the values every other caller passes too.

### The caller's attributes only, as before

* Good, because each prop is written at the usage.
* Bad, because props wrappers set are missing.
