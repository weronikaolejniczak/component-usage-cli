---
status: accepted
date: 2026-11-19
decision-makers: Weronika Olejniczak
---

# Follow the items of arrays into callbacks

## Context and Problem Statement

Lists are rendered in callbacks, as in `items.map((item) => <Badge color={item.color} />)`. The callback's parameter was `unresolved` with the code `parameter`, the largest group of unresolved values on the codebases in [the benchmarks](../../benchmarks/README.md), even when `items` is an array in the file or one a caller passes. How should `--follow` treat a callback's parameter?

## Considered Options

* Follow the first parameter of a callback passed to an array method to the array's items
* Follow callbacks of any call
* As before

## Decision Outcome

Chosen option: "Follow the first parameter of a callback passed to an array method to the array's items", because those methods call it with each item, which is what the value is, and other calls say nothing about what they pass.

* A function written as the first argument of a call of `map`, `flatMap`, `forEach`, `filter`, `find`, `findLast`, `findIndex`, `findLastIndex`, `some` or `every` on a receiver, as in `receiver.map(…)`, has its first parameter followed with `--follow variables`.
* Its value is any item of the receiver, as `receiver[index]` is with an index that isn't known ([ADR-0019](./0019-follow-variables.md)), then the steps of a destructuring pattern and its defaults, as for props ([ADR-0022](./0022-follow-props.md)). `via` has the name, then the names followed to the receiver.
* The receiver is followed like any value: a prop to what callers pass, with `--follow props`, and another item for nested callbacks.
* A receiver that isn't an array is looked up as before: an `unresolved` receiver gives its own value, and with `--follow calls` a `call` gives a `call` with the step added.
* A parameter assigned or changed in place in the callback is `mutated`, as a variable is.
* An empty array has no items, so the callback isn't called with it. Its value is `unresolved` at the array:

| `code` | Meaning |
|---|---|
| `no-items` | The receiver is an empty array |
* Other parameters, such as the index, and callbacks of other calls, such as `Children.map(children, …)`, stay `parameter`.

### Consequences

* Good, because the values of lists rendered from arrays are reported, with their positions.
* Bad, because a method of the same name on something that isn't an array, such as an observable's `map`, is treated as an array's.
* Bad, because a `filter` or `find` before the callback isn't applied, so every item is listed.

### Confirmation

Props fixtures with `--follow variables`: an array of literals, of objects destructured with a default, a receiver that's a parameter, a mutated item, an index and a callback that isn't the first argument; and with `--follow props`, an array a caller passes and an empty one.
