---
status: accepted
date: 2026-11-17
decision-makers: Weronika Olejniczak
---

# Report a wrapper's usages as usages of the components it passes its props to

## Context and Problem Statement

A codebase often renders a library's component through its own: `const Panel = (props) => <LibraryPanel {...props} />`, or `const Panel = wrap(LibraryPanel)`. Only the element inside the wrapper is a usage, so a question such as "who renders this component, and with what" misses the wrapper's usages and the props they pass. In the use cases ([benchmarks](../../benchmarks/README.md)), wrappers that spread props into a select and components wrapped by a call are the main thing left out. [ADR-0005](./0005-choose-what-to-follow.md) planned `--wrappers`. Which components are wrappers, and how does the report show their usages?

## Decision Drivers

* Accurate: a wrapper's usage is reported only when the props it's given reach the target.
* A usage through wrappers says which wrappers, and where each passes the props on, so it can be checked.
* Opt-in: it adds usages that aren't elements of the target.

## Considered Options

* Wrappers pass all their props on: with a spread of their props or of a rest element of them, or as the argument of a call
* Wrappers pass any of their props on, such as `<Target size={size} />`
* Only calls with the target as argument

## Decision Outcome

Chosen option: "Wrappers pass all their props on", because a spread or a call passes whatever the caller gives, so the wrapper's attributes are the target's props, while a component passing one prop is more often a component of its own, such as a page passing an `onClick`.

With `--wrappers`, a component, as [ADR-0022](./0022-follow-props.md) defines it, is a wrapper of a target when it:

* renders the target, or another wrapper of it, with a spread of its props or of a rest element of them: `<Target {...props} />`, `<Target {...rest} />` with `({ size, ...rest })`; or
* is a `const`, or the default export, holding a call whose first argument is the target or another wrapper of it, including through member calls and tagged templates on the call's result: `wrap(Target)`, `` wrap(Target)`…` ``, `` wrap(Target).with(…)`…` ``.

Each element whose tag refers to a wrapper, found as callers are ([ADR-0022](./0022-follow-props.md)), is a usage of each target the wrapper reaches, with `through`: the wrappers from the element to the target, outermost first, each with its name and the position where it passes the props on, the spreading element or the call's argument. Its props are the element's attributes, without the keys a rest pattern on the way takes: `({ size, ...rest })` doesn't pass `size`. A wrapper that reaches several targets gives a usage of each, in the order it passes its props on to them. When it reaches a target more than one way, the first in source order is reported. A wrapper that leads back to itself reaches nothing through that way. Without `--wrappers`, the report doesn't change.

### Consequences

* Good, because usages through wrappers are counted, with the props callers pass, and each step can be checked at its position.
* Good, because it works without `--follow`: a wrapper's usages are found from its tag.
* Bad, because a wrapper that sets props itself, as in `<Target {...props} size="s" />`, or a call that changes the props, is reported with the caller's attributes only.
* Bad, because a component that passes some props one by one isn't a wrapper.
* Neutral, because components passed as values, such as `component={Target}`, aren't usages; they're another decision.

## Pros and Cons of the Options

### Wrappers pass any of their props on

* Good, because more wrappers are found.
* Bad, because a component that passes one prop to a target, as most components rendering an icon do, would make its usages usages of the target, with attributes the target doesn't receive.

### Only calls with the target as argument

* Good, because a call such as `wrap(Target)` is the clearest sign of a wrapper.
* Bad, because wrappers written as components, which spread their props, are left out.
