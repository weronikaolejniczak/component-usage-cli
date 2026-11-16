---
status: accepted
date: 2026-11-13
decision-makers: Weronika Olejniczak
---

# Target every export of the module when no components are given

## Context and Problem Statement

Some questions are about every component of a library: which props set an icon on any component, which usages override styles. Listing hundreds of components by hand is error-prone and goes stale. What does a scan without `--components` do?

## Decision Drivers

* The list of targets must match what the module exports, without hand-written lists.
* Usages must still be identified by resolved import binding ([ADR-0006](./0006-component-identification.md)).

## Considered Options

* Every export of `--from`, followed through re-exports
* Require `--components`

## Decision Outcome

Chosen option: "Every export of `--from`", because the module already defines its components.

* Targets are every name `--from` exports, including through `export *`, other than `default`. Exports that aren't components never match a JSX tag, so they don't appear in the report.
* A usage's `component` is the exported name, with the members used in the tag: `<Lib.Split.Inner>` is `Split.Inner`.
* A name exported by more than one `export *`, or through a cycle, can't be resolved and isn't a target.

### Consequences

* Good, because a scan for all components needs no list.
* Bad, because exports whose origin can't be resolved aren't reported as targets.

### Confirmation

A fixture without `--components`, with a member component and a component the module doesn't export.
