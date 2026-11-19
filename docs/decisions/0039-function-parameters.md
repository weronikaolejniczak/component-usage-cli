---
status: accepted
date: 2026-11-19
decision-makers: Weronika Olejniczak
---

# Follow the parameters of functions to their calls

Supersedes, in [ADR-0022](./0022-follow-props.md), that only JSX tags are callers and only the first parameter is read, so that a function called by name has its parameters `parameter`.

## Context and Problem Statement

Lists and parts of components are often rendered by helpers called by name: `renderBadge(color, options)`, or `getColumns(onEdit)` imported from another file. [ADR-0022](./0022-follow-props.md) follows a component's props to its JSX callers only, and a call makes the function escape, so its parameters were `parameter`, the largest group of unresolved values on the codebases in [the benchmarks](../../benchmarks/README.md) after [ADR-0037](./0037-callback-items.md). How should the parameters of a function called by name be followed?

## Considered Options

* Calls by name are callers, for every parameter
* Only calls in the same file
* As before

## Decision Outcome

Chosen option: "Calls by name are callers, for every parameter", because a call passes each parameter as written, and helpers are as often imported as local.

* With `--follow props`, a call whose callee is the name of a function the scan treats as a component ([ADR-0022](./0022-follow-props.md)), declared in the file or imported, as in `renderBadge('red', {})`, is one of its callers, as a JSX tag is. Such a call no longer makes the function escape.
* Every parameter written as a name or a pattern is followed, with its destructuring, defaults and rest element; a rest parameter, as in `(...args)`, stays `parameter`. A call's value for a parameter is the argument at its position, or `undefined` at the call when there is none, so a default applies. `via` starts with the callee, at the call.
* A spread argument at or before a parameter's position, as in `renderBadge(...pair)`, makes that call's value `unsupported`, at the call.
* A parameter spread into an element, as `overrides` in `<Card {...defaults} {...overrides} />`, sets nothing for a call without its argument, as `renderCard()`: in each branch where a spread leaves a prop `undefined`, the attribute before it sets the prop, and identical branches are listed once.
* A JSX tag passes the props as the first parameter only; for other parameters, its value is `unsupported`, at the tag. `defaultProps` ([ADR-0038](./0038-class-components.md)) applies to JSX tags only.
* A function used another way, as in `items.map(renderItem)` or a member call such as `helpers.render(…)`, still escapes, with an `other-callers` value or, without callers, `parameter`.

### Consequences

* Good, because values rendered through helpers are followed to what each call passes, across files.
* Good, because `no-callers` now also says that a helper isn't called by name anywhere, rather than that it escapes.
* Bad, because a function passed as a callback isn't followed to what the receiver calls it with, so `items.map(renderItem)` stays `parameter` while `items.map((item) => renderItem(item))` is followed.
* Bad, because methods called as `this.render(…)` aren't functions the scan treats as components, so their parameters stay `parameter`.

### Confirmation

A props fixture with `--follow props`: a function called in its file with a name and a destructured parameter with a default and a rest element, calls with an argument missing and with a spread argument, a function with a default called with and without the argument, a function called through an import, a function passed as a value and a rest parameter, and a function spreading its parameter into an element after other props, called with and without it.
