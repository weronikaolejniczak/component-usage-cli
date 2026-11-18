---
status: accepted; wrappers' forwarding spreads superseded by ADR-0035
date: 2026-11-18
decision-makers: Weronika Olejniczak
---

# Apply spreads when counting props in the summary

Supersedes, in [ADR-0030](./0030-summary.md), that `props` counts each named prop set on a usage with the value of the last.

## Context and Problem Statement

`--summary` counted the named attributes of each usage, the last one per name winning, and ignored spreads. In `<Button type="wrong" {...{ type: 'right' }} />` it counted `"wrong"`, which React overrides, and in `<Button type="before" {...rest} />` it counted `"before"` as the value, though `rest` may set `type`. How do spreads count?

## Considered Options

* Apply props in order, as React does
* Count named attributes only, as before

## Decision Outcome

Chosen option: "Apply props in order, as React does", because then a counted value is one the component can be given.

* A spread of an object sets its properties in order, and those properties count as props set on the usage. A spread inside it applies the same way.
* A spread of `undefined` or `null` sets nothing.
* Any other spread, or a computed key, may set any prop, so the props set before it still count as set, but their values aren't counted.

### Consequences

* Good, because values a later prop overrides aren't counted.
* Good, because props set through a spread of an object, as `--follow` often finds, are counted.
* Bad, because a prop set before a spread that never sets it loses its values; the full report has them.

### Confirmation

A props fixture with `--summary` and named props before and after spreads of objects and of a name that isn't followed.
