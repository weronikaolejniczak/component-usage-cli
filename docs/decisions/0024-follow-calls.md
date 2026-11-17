---
status: accepted
date: 2026-11-17
decision-makers: Weronika Olejniczak
---

# Describe calls: what's called, with what, and what the functions passed return

## Context and Problem Statement

A value built by a call is `unresolved` with code `call` and the call's text ([ADR-0017](./0017-prop-values.md)). In the [benchmarks](../../benchmarks/README.md), calls are the largest group of values that don't resolve, through every kind of following: options built in `useMemo(() => [...])` or by `items.map(…)`, styles from `useStyles()` and `css(…)`, icon sets sorted or searched before they're passed. The values an agent needs are often in the call's arguments, or returned by a function passed to it. How should the report show a call, without running code it can't see?

## Decision Drivers

* Accurate: a value is reported only as what the source says, never as what a function would return if it behaved as its name suggests.
* The values inside a call are read like any other value, with positions, through the same following.
* Opt-in, as other following is ([ADR-0005](./0005-choose-what-to-follow.md)): describing calls makes the report larger.

## Considered Options

* Describe each call: what's called, its arguments, and what the functions among them return
* Evaluate calls whose result is known, such as `useMemo(fn)` returning what `fn` returns
* Keep calls `unresolved`

## Decision Outcome

Chosen option: "Describe each call", because the arguments and returned values are in the source, while what a call returns depends on code the scan doesn't read or run.

With `--follow calls`, a call is a value of kind `call`, at the call's position:

* `callee`: the source text of what's called, such as `useMemo`, `items.map` or `strings.format`. It isn't followed.
* `receiver`: for a call of a member, as in `items.map(…)`, the value of the object, `items`.
* `arguments`: the arguments in order, as `{ "value" }` or `{ "spread" }`, like the items of an array.
* `path`: the properties read from the call's result, as in `useStyles().panel` or `const { theme } = useTheme()`, when there are any. A computed key that isn't a literal is `null`.

A function that is an argument of a call, written in place or reached by following, such as `compute` in `useMemo(compute)`, has `returns`: the value of each `return` statement in its body, outside nested functions, in source order, with `undefined` for a `return` without a value; or the body of an arrow function without braces. It doesn't say whether the end of the body is reached. Async functions and generators have no `returns`, since they don't return those values.

A call is described, not evaluated: `useMemo(() => [...])` is a call of `useMemo` whose argument returns an array, not the array. Calls with optional chaining, as in `api?.get(key)`, are described the same way. `require(…)` stays `unresolved` with code `call`: `module.exports` isn't read ([ADR-0015](./0015-es-modules-only.md)). A function whose `returns` lead back to itself, as in `function loop() { return wrap(loop); }`, is `unsupported` where it comes back, as names that refer back to themselves are ([ADR-0019](./0019-follow-variables.md)).

`--follow calls` combines with the other values of `--follow`, such as `--follow props,calls`, and doesn't imply them: the arguments are followed as far as the other values say. Without it, calls stay `unresolved` with code `call`.

`parse` records, when following calls, the callee, receiver and arguments of each call, and the returns of each function; `values` describes what's recorded, so its interface doesn't change.

### Consequences

* Good, because options, styles and icons built from literals by a call are read, at their positions, with the call that builds them.
* Good, because nothing is assumed about what a function does, so a value is never reported as something the code doesn't say.
* Bad, because the report is larger: calls such as `useMemo(fn, [deps])` list their dependencies too.
* Bad, because what a call returns is still left to the reader, even for well-known functions.
* Neutral, because a callee isn't followed, so helpers such as `getIcon(name)` show their arguments but not what they return.

## Pros and Cons of the Options

### Evaluate calls whose result is known

* Good, because `useMemo(() => [...])` would be the array, the way it's used.
* Bad, because the result depends on what the function is, which a name doesn't prove: a local `useMemo` or a different version could behave differently.
* Bad, because the list of known functions would name libraries.

### Keep calls `unresolved`

* Good, because the report stays small.
* Bad, because the values in the largest group that doesn't resolve stay unread.
