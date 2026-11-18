---
status: accepted
date: 2026-11-18
decision-makers: Weronika Olejniczak
---

# Show paths outside the scan root relative to it

Supersedes, in [ADR-0021](./0021-follow-imports.md), that a path outside the scanned directory is absolute.

## Context and Problem Statement

Report paths are relative to the scan root. Files outside it are parsed when an import reaches them, such as a value in a shared folder or, with `--wrappers`, a wrapper that renders a target ([ADR-0026](./0026-wrappers.md)), and their paths were absolute. A report then differed between machines, and between checkouts on one machine. How is a path outside the root shown?

## Considered Options

* Relative to the root, with `..`
* Absolute, as before

## Decision Outcome

Chosen option: "Relative to the root, with `..`", because the same checkout gives the same report wherever it is.

* A path outside the root goes up from it with a `..` per folder: `../shared/Wrapper.jsx` when scanning `app`.
* A path that no folder above the root contains, as on another drive, stays absolute.

### Consequences

* Good, because reports of the same checkout can be compared across machines.
* Bad, because a path with several `..` is harder to read than an absolute one.

### Confirmation

A unit test of paths inside, above and beside the root, and a props fixture scanning a folder whose wrapper is beside it.
