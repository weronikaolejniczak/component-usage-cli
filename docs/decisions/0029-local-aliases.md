---
status: accepted
date: 2026-11-17
decision-makers: Weronika Olejniczak
---

# Count usages through local names that are never assigned again

Supersedes, in [ADR-0015](./0015-es-modules-only.md), that a JSX tag whose root is a local constant initialised from an import is unidentified with code `alias`.

## Context and Problem Statement

[ADR-0015](./0015-es-modules-only.md) reports `const I = Icon; <I />` as unidentified with code `alias`, because constants weren't followed before values were. Values now follow variables ([ADR-0019](./0019-follow-variables.md)), and `alias` elements are left out of usages, wrappers and callers, though most refer to exactly one component. In the [benchmarks](../../benchmarks/README.md), one of the two `alias` elements is a `const`; the other is a `let` assigned another component in an `if`. Which local names does the scan follow to the component they refer to?

## Decision Drivers

* Counted usages must be exact: a tag that can render more than one component isn't a usage of one of them.
* A local name for a component is the same component, wherever the scan looks for one.

## Considered Options

* Names never assigned again
* `const` names only
* Keep reporting them as unidentified

## Decision Outcome

Chosen option: "Names never assigned again", because such a name holds what it was declared with, whether it's declared with `const`, `let` or `var`.

* A JSX tag whose root is a local name declared from an import, or a static property chain on one, followed through other such names and object destructuring as in [ADR-0015](./0015-es-modules-only.md), is the component the import refers to, when no name on the way is assigned after its declaration, declared more than once, or changed in place, as in `Lib.Icon = Other`. It's a usage, a wrapper's usage with `--wrappers` ([ADR-0026](./0026-wrappers.md)), and a caller with `--follow props` ([ADR-0022](./0022-follow-props.md)), like a tag that names the import.
* Otherwise it stays unidentified with code `alias`. `require` stays `commonjs`.
* The reference of the import in a followed name's declaration, `in: variable` ([ADR-0027](./0027-references.md)), is left out when a usage goes through the name, as for wrappers ([ADR-0028](./0028-references-of-unrendered-wrappers.md)), so each place the component is passed is in the report once.
* Usages don't say that their tag is a local name: the tag's position leads to it, as for an import with `as`.

### Consequences

* Good, because usages through local names are counted, with props, parents and owners.
* Bad, because a name used both as a tag and passed somewhere else, as in `styled(I)`, loses its `variable` reference, and its other uses aren't references, since only imports are tracked.
* Neutral, because names exported and rendered in another file are still not followed.

### Confirmation

The `local-alias` identify fixture, and a props fixture with a local name for a wrapper and for a component whose props are followed.

## Pros and Cons of the Options

### `const` names only

* Good, because `const` can't be assigned again.
* Bad, because a `let` that's never assigned again holds the same component and stays unidentified.

### Keep reporting them as unidentified

* Good, because nothing changes.
* Bad, because usages that refer to one component aren't counted.
