---
status: accepted
date: 2026-11-19
decision-makers: Weronika Olejniczak
---

# Follow `this` in class components

Supersedes, in [ADR-0017](./0017-prop-values.md), that `this.props.x` is `unresolved` with the code `unsupported`, and, in [ADR-0022](./0022-follow-props.md), that classes reading `this.props` aren't components whose props are followed.

## Context and Problem Statement

Class components read their props as `this.props`, their state as `this.state`, and their handlers as `this.onClick`. Every one of these was `unsupported`; they are the largest group of `unsupported` values left on the codebases in [the benchmarks](../../benchmarks/README.md) after [ADR-0037](./0037-callback-items.md). How should `this` be reported in a class component?

## Considered Options

* Treat `this.props` as the component's props, and follow the class's own fields and methods
* Only `this.props`
* As before

## Decision Outcome

Chosen option: "Treat `this.props` as the component's props, and follow the class's own fields and methods", because those are what `this` holds in a component, and each can be stated without running it.

* A class with a superclass and a `render` method is a component, named and exported like a function component. Its props are `this.props`, as a function's are its first parameter, wherever `this` is the instance: in its methods and in the arrow functions and values of its fields that aren't `static`, but not in functions written with `function` or in other classes.
* `this.props` is a prop as for functions: `parameter` without `--follow props`, and followed to callers with it, with the class's position for the `one-of`. `{...this.props}` passes all props on, for `--wrappers`.
* `defaultProps`, as `static defaultProps = …` or `Component.defaultProps = …`, replaces `undefined` in what each caller passes for a prop, as React does, for function components too.
* `this.state` and what is read from it are `unresolved`:

| `code` | Meaning |
|---|---|
| `state` | The component's state, which `setState` changes while it runs |

* `this.name`, for a field that isn't `static`, is the field's value, and for a method, a `function` at the method, with `--follow variables`. `via` has `this.name`. A name assigned as `this.name = …` in the class, such as a method bound in the constructor, is `mutated`. Other names, getters and `this` alone stay `unsupported`.

### Consequences

* Good, because class components are followed like function components: their props to callers, their handlers to their definitions.
* Good, because `defaultProps` no longer leaves a prop a caller omits as `undefined` when React gives it a value.
* Bad, because a superclass other than React's that has a `render` method makes a class a component.
* Bad, because a caller's spread that sets a prop to `undefined` hides its default when all props are read at once, as in `{...this.props}`.
* Neutral, because state isn't followed: its value depends on what happened while the component ran.

### Confirmation

A props fixture with `--follow props`: a class with `defaultProps`, `this.props` read directly, destructured and as a callback's receiver, `this.state`, a field, a method, a method bound in the constructor, a name the class doesn't declare and a `function` inside a method; and a function component with `Component.defaultProps`. The `expressions` fixture, without `--follow`, has `this.props.type` as `parameter`.
