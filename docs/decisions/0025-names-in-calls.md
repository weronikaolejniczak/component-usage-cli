---
status: accepted
date: 2026-11-17
decision-makers: Weronika Olejniczak
---

# Names in a call's arguments aren't followed

Supersedes, in [ADR-0024](./0024-follow-calls.md), that the arguments of a call are followed as far as the other values of `--follow` say, and that a function reached by following has `returns`.

## Context and Problem Statement

A value is written out in full wherever it's reached. With [ADR-0024](./0024-follow-calls.md), a name in a call's arguments, or in what a function among them returns, is followed like any other. Calls often repeat a name: `useMemo(() => options.filter(…), [options])` reads `options` twice, and a second `useMemo` that reads the first's result twice reads `options` four times. With `--follow props`, each read is every caller's value. On the benchmark codebase, with the components and command of the [benchmarks](../../benchmarks/README.md) and `--follow props,calls`, one run wrote a 102 GB report. How should the report stay usable?

## Considered Options

* Names in a call's arguments, and in what functions among them return, aren't followed
* A name followed once in a prop's value isn't followed again in it, with a code pointing back to the first
* Keep following them

## Decision Outcome

Chosen option: "Names in a call's arguments aren't followed", because it bounds a call's description by its source text, and what's written in place is still read.

Within a call's arguments, including what a function among them returns, a variable, an import, or a parameter that `--follow props` would follow is `unresolved` with code `in-call`, at the name. Parameters that wouldn't be followed and globals keep their codes. A call's receiver is followed as before: in `ICONS.map(…)`, `ICONS` is the array it names, unless the call is itself within another call's arguments. Only functions written in place in the arguments have `returns`; `useMemo(compute)` is a call whose argument is `compute`, `in-call`.

The [benchmarks](../../benchmarks/README.md) have the same scan with this decision.

### Consequences

* Good, because the report grows with what's written in the calls, not with how often their arguments repeat a name.
* Good, because imports used only in arguments aren't resolved.
* Bad, because values passed to a call by name, such as a list of options declared above `useMemo`, aren't read; the name and its position are, and the agent can follow it.
* Bad, because a function passed by name has no `returns`.

## Pros and Cons of the Options

### A name followed once in a prop's value isn't followed again in it

* Good, because every value is read once, so nothing is lost.
* Bad, because a value's meaning depends on what was found before it in the prop, so it can't be read on its own.
* Bad, because it changes values without calls too.

### Keep following them

* Good, because every value a call is given is read.
* Bad, because the report is too large to write or read.
