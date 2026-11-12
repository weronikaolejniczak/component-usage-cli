---
status: accepted
date: 2026-11-12
decision-makers: Weronika Olejniczak
---

# Split the code into workspace crates by pipeline stage

## Context and Problem Statement

The scan runs as a pipeline: discover and parse files, index modules, find targets, resolve values, filter, report. How is the code organized?

## Considered Options

* One Cargo workspace crate per pipeline stage
* A single crate with modules

## Decision Outcome

Chosen option: "One crate per stage".

| Crate | Responsibility |
|---|---|
| `parse` | File discovery; `oxc_parser` + `oxc_semantic` per file, in parallel |
| `graph` | Module resolution, export index, component call graph |
| `resolve` | Value model and data-flow engine |
| `query` | Filtering by component, prop, value kind |
| `report` | Output formats |
| `cli` | Argument parsing, entry point; binary `component-usage` |

### Consequences

* Good, because dependencies between stages are explicit in `Cargo.toml`.
* Good, because each stage can be tested on its own.
* Bad, because more boilerplate than a single crate.
