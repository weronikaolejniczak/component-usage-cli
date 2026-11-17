---
status: accepted
date: 2026-11-17
decision-makers: Weronika Olejniczak
---

# Reading or setting a property of a component doesn't add `other-callers`

Supersedes the definition of `other-callers` in [ADR-0022](./0022-follow-props.md).

## Context and Problem Statement

[ADR-0022](./0022-follow-props.md) adds an `other-callers` value when a component is used other than as a JSX tag, an export or the argument of `memo` or `forwardRef`. Components often get a property set, as in `Wrapper.displayName = 'Wrapper'`. That counts as another use, so the values of every prop of such a component get an `other-callers` value, though setting the property doesn't render the component. In the use cases ([benchmarks](../../benchmarks/README.md)), two components with `other-callers` values, read by hand, had no other use than `displayName`.

## Considered Options

* Property access on a component isn't another use, except `call`, `apply` and `bind`
* Only `displayName`, `propTypes` and `defaultProps` aren't another use
* Keep every property access as another use

## Decision Outcome

Chosen option: "Property access on a component isn't another use, except `call`, `apply` and `bind`", because reading or setting a property can't pass the component props, and those three call it.

`Wrapper.anything`, read or set, isn't another use of `Wrapper`. `Wrapper.call(…)`, `Wrapper.apply(…)`, `Wrapper.bind(…)` and computed access, `Wrapper[key]`, are. A property of a namespace import, `Library.Wrapper`, is still another use of the import: the property is the component.

### Consequences

* Good, because components with `displayName` keep the values their callers pass, without an `other-callers` value that doesn't stand for a caller.
* Bad, because `Wrapper.defaultProps = { … }` isn't read, so its defaults aren't applied.

## Pros and Cons of the Options

### Only `displayName`, `propTypes` and `defaultProps` aren't another use

* Good, because it lists what's known not to render.
* Bad, because other static properties, such as `Wrapper.Item = Item`, would still add `other-callers` without a caller.

### Keep every property access as another use

* Good, because nothing changes.
* Bad, because `other-callers` would stand for a caller that doesn't exist.
