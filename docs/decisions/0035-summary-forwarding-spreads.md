---
status: accepted
date: 2026-11-18
decision-makers: Weronika Olejniczak
---

# Count what a wrapper passes on once in the summary

Supersedes, in [ADR-0033](./0033-summary-spreads.md), that every spread of something other than an object may set any prop, for the spread a wrapper passes its props on with.

## Context and Problem Statement

With `--wrappers`, a wrapper's own element, such as `<Target {...props} />`, is a usage, and so is each caller's usage through it ([ADR-0026](./0026-wrappers.md)), with the caller's props ([ADR-0031](./0031-props-wrappers-set.md)). With `--follow props`, the wrapper's spread is its callers' props too, so `--summary` counted a caller's values twice: on the wrapper's element, when it has one caller, and on the usage through it. With several callers the spread is a `one-of`, so their values weren't counted there, and counts depended on the number of callers. How does the summary count the wrapper's spread?

## Considered Options

* The spread sets nothing
* No props of the wrapper's element count when usages go through it
* The wrapper's element isn't a usage when usages go through it
* As before

## Decision Outcome

Chosen option: "The spread sets nothing", because what it passes is counted on the usages through the wrapper, and the props the wrapper sets itself still count on its element.

* With `--wrappers`, the spread a wrapper passes its props on with sets no prop in the summary and hides no values set before it.
* Other spreads apply as [ADR-0033](./0033-summary-spreads.md) says.
* The full report is unchanged.

### Consequences

* Good, because a caller's value is counted once, however many callers the wrapper has.
* Bad, because a prop the wrapper sets itself is counted on its element and on each usage through it, as both are usages.
* Bad, because callers that aren't JSX elements, such as a wrapper passed as a value, have no usages through it, so what they pass isn't counted.

### Confirmation

A props fixture with `--summary --wrappers --follow props`: a wrapper with one caller and one with two, one setting a prop of its own.
