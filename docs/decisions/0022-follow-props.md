---
status: accepted; other-callers definition superseded by ADR-0023; class components superseded by ADR-0038; called functions superseded by ADR-0039
date: 2026-11-17
decision-makers: Weronika Olejniczak
---

# Follow a component's props to what its callers pass

Supersedes the `values` interface in [ADR-0021](./0021-follow-imports.md).

## Context and Problem Statement

With `--follow imports` ([ADR-0021](./0021-follow-imports.md)), a value read from a prop of the enclosing component is `unresolved` with code `parameter`: `function HeaderMenu({ iconType }) { return <Icon type={iconType} /> }`. In the [benchmarks](../../benchmarks/README.md), these are one of the largest groups of values that don't resolve, and the components they are in are wrappers whose own callers hold the literals. [ADR-0005](./0005-choose-what-to-follow.md) and [ADR-0019](./0019-follow-variables.md) plan `--follow props`: "a component's prop to what each call site of that component passes, recursively". Which functions are components, how are their callers found, and what does the report say when the callers may not all be known?

## Decision Drivers

* No silent misses: when a component may receive values from callers the scan can't see, the report says so.
* Each value says which caller passes it, in which file, so it can be changed.
* The same rules as variables and imports, so a value reads the same wherever it comes from.
* Only JSX callers: a component is a function rendered as a JSX tag.

## Considered Options

* Follow the first parameter of named functions to the attributes of their JSX callers, with a one-of of the callers' values
* Report wrappers as targets instead (`--wrappers`), and leave parameters unresolved
* Follow parameters of every function to the arguments of every call

## Decision Outcome

Chosen option: "Follow the first parameter of named functions to the attributes of their JSX callers", because the callers of a component are JSX elements, which the scan already reads in every file, with their attributes.

`--follow props` implies `imports`: callers are usually in other files.

A component is a function declared with a name, or an arrow function or function expression assigned to a `const`, optionally inside `memo(…)` or `forwardRef(…)`, as a call or a member call such as `React.memo(…)`, nested in any order. `export default` with such a function, named or not, is a component too. `const Wrapper = memo(Inner)`, with `Inner` a component of the file, adds `Wrapper`'s callers to `Inner`'s.

Its props are its first parameter: a name, as in `props`, then `props.iconType`, or a pattern, as in `{ iconType = 'gear', ...rest }`. A name destructured from the props in the body, as in `const { iconType } = props`, is followed through the variable. A pattern with a default on the way to the name, such as `{ a: { b } = {} }`, and parameters of other functions, such as `item` in `items.map((item) => …)`, stay `parameter`.

Its callers are the JSX elements of every parsed file whose tag refers to it: by its local name, through an import and re-exports as components are identified ([ADR-0006](./0006-component-identification.md)), or as a member of a namespace import. For each caller, in order of file and position, a prop is the last attribute that sets it: a named attribute, or a spread whose value has the prop. A caller that doesn't set it gives `undefined`, or the default in the pattern. `children` comes from the element's children when it has any, which are `unsupported`. Without a prop name, as in `{...props}`, a caller gives an object of its attributes; for a rest element, as in `{...rest}`, without the keys the pattern takes before it. Values passed by a caller are followed in its file, including its own props, so a chain of wrappers is followed to the literals.

The value of a prop is a `one-of` with a value for each caller, at the parameter's position. Each value's `via` starts with the caller's tag, with its `file` and position. With a single caller, the value is that caller's.

New `unresolved` codes, in addition to [ADR-0017](./0017-prop-values.md), [ADR-0019](./0019-follow-variables.md) and [ADR-0021](./0021-follow-imports.md):

| Code | Meaning |
|---|---|
| `no-callers` | The component isn't rendered as a JSX tag in any parsed file, or used any other way |
| `other-callers` | Added to the callers' values: the component, or an import of it, is also used other than as a JSX tag, an export or the argument of `memo` or `forwardRef`, such as `styled(Wrapper)` or `component={Wrapper}`, so it can receive values that aren't listed |

A component with no JSX callers that is used another way, such as a function called as `renderRow(item)`, stays `parameter`: it isn't used as a component.

A component that renders itself, directly or through other components, is `unsupported` where the chain comes back, as names that refer back to themselves are ([ADR-0019](./0019-follow-variables.md)). Callers outside the scanned directory, which aren't parsed, aren't known; a component exported from a package the scan doesn't cover may have more callers than the report lists.

Without `--follow props`, the props of components stay `parameter`, as before.

`values::Context::new` takes `props` alongside `imports`. `parse` records, when following props, the components of a file, what each of their props names refers to, which elements render a component of the file, and which imports are used other than as a JSX tag; `values` indexes callers once per scan.

### Consequences

* Good, because values passed through wrappers are read, with each caller's file and position.
* Good, because a component used in a way the scan doesn't follow says so with `other-callers`, instead of listing callers as if they were all of them.
* Good, because spreads that forward props, `{...props}` and `{...rest}`, are followed with the rest of the chain.
* Bad, because a component rendered in many places repeats all its callers' values in every usage inside it, which makes the report larger.
* Bad, because only the first parameter is read, so components that take their props another way, such as classes reading `this.props`, stay `unsupported`.
* Bad, because callers that set a prop through a spread of an unknown value give that value, even when an earlier attribute sets the prop.

### Confirmation

A fixture for `--follow props` covering each form and code above. The existing fixtures don't change. Benchmarks report time, memory and counts per kind with and without it.

## Pros and Cons of the Options

### Wrappers as targets

* Good, because a wrapper's own usages are counted, which answers "who renders this, indirectly".
* Bad, because the values a target receives through a wrapper stay `parameter`, so the literals aren't found.
* Neutral, because it can be added later, alongside following props: [ADR-0005](./0005-choose-what-to-follow.md) plans `--wrappers`.

### Parameters of every function, through every call

* Good, because helpers such as `renderIcon(name)` would be followed too.
* Bad, because calls pass values positionally, through callbacks and higher-order functions the scan can't see, so most would end in `other-callers`.
* Bad, because every call of every function in a codebase would be indexed.
