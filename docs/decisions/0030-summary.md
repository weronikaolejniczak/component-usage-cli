---
status: accepted
date: 2026-11-17
decision-makers: Weronika Olejniczak
---

# Summarise a scan with counts per component, prop and owner

Supersedes the `--summary` option proposed in [ADR-0010](./0010-output.md).

## Context and Problem Statement

A report lists every usage with its props, so it grows with the codebase: in the [benchmarks](../../benchmarks/README.md), a scan of ten components is tens of megabytes. Many questions only need counts: how often a component is used and by whom, which props are set, which values a prop takes. Reading them from the full report means loading all of it. What does `--summary` report?

## Decision Drivers

* An agent or a person can read it whole.
* Counts are exact and deterministic, from the same scan as the full report.
* Problems with the scan stay visible.

## Considered Options

* Counts per component, with its props, their literal values and its owners
* Counts per component only
* Distinct values of every kind, as ADR-0010 proposed

## Decision Outcome

Chosen option: "Counts per component, with its props, their literal values and its owners", because they answer who uses a component and how, and literal values are the ones that can be counted as written.

With `--summary`, the report has, instead of `usages`, `references` and `unidentified`:

* `components`: one entry per target, sorted by name: every target given with `--components`, or without it every target with a usage, a reference or an unidentified element. Each has:
  * `usages`, `files`, the files they're in, `spreads`, the usages with a spread, and `references`;
  * `through`, with `--wrappers`: the usages through wrappers;
  * `props`: each named prop set on a usage, with `usages` and `values`: the literal values usages give it, the value itself or a branch of a `one-of`, each counted once per usage, sorted by count, then by their JSON text. Props are sorted by count, then by name;
  * `owners` and `unowned`, with `--owners`: usages per owner, sorted by count, then by name, and the usages in files without owners. A usage in a file with several owners counts for each.
* `owners` and `unowned`, with `--owners`: the same over all components.
* `unidentified`: the elements per `code` and `component`, the target when known, sorted by code, then component.

`errors` and `warnings` are listed in full, as in the full report. `--props`, `--follow` and `--wrappers` apply as they do to the full report: `--follow` finds more literal values, and `--wrappers` adds usages.

### Consequences

* Good, because a summary of a large codebase can be read whole.
* Good, because unidentified elements, errors and warnings are still reported, so missing counts are visible.
* Bad, because values that aren't literals, such as a name that isn't followed, aren't counted by kind: the full report has them.

### Confirmation

A fixture with a hand-written summary, covering wrappers, local names, references, unidentified elements and owners.

## Pros and Cons of the Options

### Counts per component only

* Good, because it's the smallest.
* Bad, because which props are set, and to what, needs the full report.

### Distinct values of every kind, as ADR-0010 proposed

* Good, because every value is counted.
* Bad, because values that aren't literals are rarely the same twice: names, calls and templates are counted one by one.
