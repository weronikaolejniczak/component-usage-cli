---
status: superseded by ADR-0020
date: 2026-11-12
decision-makers: Weronika Olejniczak
---

# Crate boundaries and interfaces

Supersedes [ADR-0011](./0011-crate-layout.md).

## Context and Problem Statement

[ADR-0011](./0011-crate-layout.md) split the code by pipeline stage but didn't define what crosses each boundary. Oxc's AST is allocated in a per-file arena (`oxc_allocator::Allocator`) and borrows from it, so it can't outlive parsing or move freely between threads. What does each crate own, and what data passes between them?

## Decision Drivers

* Each crate has one responsibility and a small public interface.
* Stages are testable without the filesystem.
* No AST lifetimes leak out of parsing.
* Parallelism and I/O live in one place.

## Considered Options

* Owned per-file summaries between stages, I/O at the edges
* Keep ASTs alive for the whole scan and pass references

## Decision Outcome

Chosen option: "Owned per-file summaries between stages", because it keeps lifetimes inside `parse` and makes every later stage a plain function over owned data.

```text
cli ─ args → Config ─► engine::scan ─► Report ─► report::write
                          │
          discover files ─► parse ─► graph ─► identify ─► values (later)
```

| Crate | Owns | Public interface | I/O |
|---|---|---|---|
| `model` | Shared data types: `FileId`, `Location`, `ExportRef`, `Usage` | Types only; `serde` behind a feature | No |
| `parse` | `oxc_parser`, `oxc_semantic`, extraction | `parse(path, source) -> Result<FileSummary, ParseError>` | No |
| `graph` | `oxc_resolver`, module graph, re-export chains | `ModuleGraph::build`, `ModuleGraph::origin(file, local) -> Option<ExportRef>` | Reads `package.json`, `tsconfig.json` |
| `identify` | Matching JSX elements to target components | `identify(&graph, &summaries, &targets) -> Vec<Usage>` | No |
| `values` | Value resolution ([ADR-0004](./0004-explicit-value-kinds.md), [ADR-0005](./0005-choose-what-to-follow.md)) | Later | No |
| `report` | Output formats | `write(&report, format, writer)` | Writes to a given `Write` |
| `engine` | File discovery, parallel parsing, stage orchestration | `scan(&Config) -> Result<Report, ScanError>` | Yes |
| `cli` | Argument parsing, error display, exit codes | Binary `component-usage-cli` | Yes |

Rules:

* `FileSummary` is owned data with no AST references. The arena is dropped at the end of `parse`.
* Files are referenced by `FileId` (index into one file table). Paths appear only at the edges.
* Library crates return typed errors (`thiserror`). Only `cli` formats errors for people. No panics on user input.
* Parallelism (`rayon`) only in `engine`.
* Each crate exposes one entry point and its types; everything else is `pub(crate)`. Each `lib.rs` documents its contract.
* Output is sorted by file path, then position.
* Crates are aliased in `Cargo.toml` (`parse = { package = "component-usage-cli-parse", … }`) so code uses short names.

### Consequences

* Good, because `parse`, `identify`, and `values` are unit-tested with inline source strings.
* Good, because `cli` stays thin; end-to-end behavior is testable through `engine::scan`.
* Bad, because extraction has to decide up front what each later stage needs from the AST; new needs mean extending `FileSummary`.

### Confirmation

Crate dependencies in `Cargo.toml` match the diagram. Fixture tests in `crates/cli/tests` pass end to end.

## Pros and Cons of the Options

### Keep ASTs alive for the whole scan

* Good, because later stages can read anything from the AST.
* Bad, because every stage carries the arena lifetime, and memory grows with the whole codebase.
