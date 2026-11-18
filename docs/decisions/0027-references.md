---
status: accepted; leaving out wrappers' arguments superseded by ADR-0028; not following local names superseded by ADR-0029
date: 2026-11-17
decision-makers: Weronika Olejniczak
---

# Report the places a target is passed as a value

## Context and Problem Statement

A component is often passed rather than rendered: `<Panel icon={Target} />`, `{ component: Target }`, `[Target, Other]`, `wrap(Target)`. The receiver renders it later, with props the report can't see. None of these is an element, so a question such as "where is this component used" misses them. In the use cases ([benchmarks](../../benchmarks/README.md)), icons and buttons passed in attributes and objects are the main case. [ADR-0026](./0026-wrappers.md) left components passed as values to another decision. Should the report list them, and how?

## Decision Drivers

* Accurate: an entry is a place where the target's value is passed, not any mention of its name.
* Each entry says where the value goes, so it can be grouped and checked without reading the source.
* Usages keep meaning elements.

## Considered Options

* A `references` list, always reported, each with where the value is passed
* Usages without props, with a marker
* Behind a flag

## Decision Outcome

Chosen option: "A `references` list, always reported", because a passed component is a use of it that no flag makes an element, and a separate list keeps `usages` to elements with props.

The report has `references`: each place in a scanned file where an imported name, or a property access on a namespace import, including `?.` and a static key in brackets, that refers to a target is read as a value, with the target's name, its position, and `in`, where the value goes:

* `attribute`: a JSX attribute, with the element's tag as written and the attribute's name, as in `<Panel icon={Target} />`;
* `argument`: an argument of a call or `new`, with the callee's source text and the argument's index from 0, as in `wrap(Target)`;
* `property`: an object's property, with its key, or with `computed` and the key's source text when the key isn't static;
* `item`: an array's element, with its index from 0;
* `variable`: the value of a `const`, `let` or `var` declared with a name, with the name;
* `other`: any other way, such as a return or an assignment.

The value goes where the expression around the name goes, through parentheses, type assertions, the branches of `?:`, either side of `||` and `??`, and the right side of `&&`. Not references: JSX tags, which are usages; exports, which the module graph follows; types; calls of it, as in `useTarget()`, which don't pass it on; and the object of a property access that isn't itself a target, as in `Target.displayName`. With every export of `--from` as targets, as [ADR-0018](./0018-all-components.md) defines, a property access after an export isn't one, since it's more often a property than a component. With `--wrappers`, the argument of a call that makes a wrapper isn't a reference: the wrapper's usages are. A local name that refers to a target, as in `const { Target } = Library`, isn't followed.

### Consequences

* Good, because every value of a target that can be rendered elsewhere is counted, with where it goes.
* Good, because `in` groups references, such as every `icon` attribute, without reading the source.
* Bad, because what the receiver renders, and with which props, isn't known.
* Neutral, because reads that don't pass the value on, such as `if (Target)` or `typeof Target`, are `other`.

## Pros and Cons of the Options

### Usages without props, with a marker

* Good, because one list answers "where is it used".
* Bad, because every reader of `usages` has to skip entries that aren't elements, and `parents` and `props` mean nothing for them.

### Behind a flag

* Good, because the report doesn't change for scans that don't need them.
* Bad, because the default report would leave out uses of the target without saying so.
