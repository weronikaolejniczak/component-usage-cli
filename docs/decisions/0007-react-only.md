---
status: superseded by ADR-0014
date: 2026-11-12
decision-makers: Weronika Olejniczak
---

# Support React JSX only

## Context and Problem Statement

Which frameworks should the scanner support?

## Considered Options

* React JSX only
* React plus Vue, Svelte and Angular templates

## Decision Outcome

Chosen option: "React JSX only", because the [benchmark codebases](../../benchmarks/README.md) are React.

Input files: `.js`, `.jsx`, `.ts`, `.tsx`, `.mjs`, `.cjs`, `.mts`, `.cts`. File discovery respects `.gitignore`, supports `--include` / `--exclude`, and skips `node_modules` unless included.

### Consequences

* Good, because one syntax to support.
* Bad, because other frameworks need a later decision. Keep JSX-specific code in `parse` and `graph` where it's cheap to do so.
