---
status: accepted
date: 2026-11-16
decision-makers: Weronika Olejniczak
---

# Crate boundaries and interfaces, with values

Supersedes [ADR-0012](./0012-architecture.md).

## Context and Problem Statement

[ADR-0012](./0012-architecture.md) left `values` for later, and its interfaces no longer match the code: `identify` returns usages and unidentified elements, `owners` isn't listed, and `parse` now takes options. Values need declarations, which only `parse` can read, since the AST can't leave it. Tags and prop values both follow names declared in the file. Where does following names live, and what crosses each boundary?

## Decision Drivers

* Each crate has one responsibility and a small public interface.
* Stages are testable without the filesystem.
* No AST lifetimes leak out of parsing.
* Parallelism and I/O live in one place.
* One way to follow a name, wherever the name appears.

## Considered Options

* `parse` records definitions of the names it reads; `values` evaluates them
* `parse` evaluates values itself
* Keep ASTs alive so `values` can read declarations

## Decision Outcome

Chosen option: "`parse` records definitions; `values` evaluates them", because the AST stays in `parse` and evaluation stays a plain function over owned data.

```text
cli ─ args → Config ─► engine::scan ─► Report ─► report::write
                          │
          discover files ─► parse ─► graph ─► identify ─► values, per usage
                                                          owners, per file
```

| Crate | Owns | Public interface | I/O |
|---|---|---|---|
| `model` | Shared data types: `FileId`, `Location`, `ExportRef`, `Usage`, `Parent`, `Prop`, `Value` | Types only; `serde` behind a feature | No |
| `parse` | `oxc_parser`, `oxc_semantic`, extraction, what each name in the file is declared as | `parse_with(path, source, Options) -> Result<FileSummary, ParseError>` | No |
| `graph` | `oxc_resolver`, module graph, re-export chains | `ModuleGraph::build`, `export_origin`, `import_target`, `export_names` | Reads `package.json`, `tsconfig.json` |
| `identify` | Matching JSX elements to target components | `Targets::resolve`, `Targets::all`, `identify(&graph, &files, &targets) -> Identified` | No |
| `values` | Evaluating attributes and the definitions `parse` recorded ([ADR-0017](./0017-prop-values.md), [ADR-0019](./0019-follow-variables.md)) | `props(&summary, &element, filter) -> Vec<Prop>` | No |
| `owners` | `CODEOWNERS` parsing and matching ([ADR-0013](./0013-code-owners.md)) | `CodeOwners::parse`, `CodeOwners::owners(path)` | No |
| `report` | Output formats | `write(&report, format, writer)` | Writes to a given `Write` |
| `engine` | File discovery, parallel parsing, stage orchestration | `scan(&Config) -> Result<Report, ScanError>` | Yes |
| `cli` | Argument parsing, error display, exit codes | Binary `component-usage-cli` | Yes |

Following names:

* `parse` resolves each name it reads, in a tag or an attribute, to what it's declared as: an import, a parameter, a global, or a local name with an index into `FileSummary::definitions`.
* One definitions pass records what local names hold, as owned expressions. It runs for tags always, defining every name by its declaration, so a tag that may be a component is reported; and for attributes when `Options::variables` is set, marking names that are changed later as mutated. Tag aliases are read from the first.
* `values` evaluates attributes against the definitions: lookups, defaults, cycles and `via`. It doesn't read options; a definition that wasn't recorded is not followed.

Rules, as in [ADR-0012](./0012-architecture.md):

* `FileSummary` is owned data with no AST references. The arena is dropped at the end of `parse`.
* Files are referenced by `FileId` (index into one file table). Paths appear only at the edges.
* Library crates return typed errors (`thiserror`). Only `cli` formats errors for people. No panics on user input.
* Parallelism (`rayon`) only in `engine`.
* Each crate exposes one entry point and its types; everything else is `pub(crate)`. Each `lib.rs` documents its contract.
* Output is sorted by file path, then position.
* Crates are aliased in `Cargo.toml` (`parse = { package = "component-usage-cli-parse", … }`) so code uses short names.

### Consequences

* Good, because `parse`, `identify` and `values` are unit-tested with inline source strings.
* Good, because a name is followed the same way in a tag and in a prop.
* Good, because `cli` stays thin; end-to-end behavior is testable through `engine::scan`.
* Bad, because `parse` has to decide up front which declarations later stages need; new needs mean extending `FileSummary`.
* Bad, because following imports (`--follow imports`) needs definitions of exported names in other files, so `values` will also need the module graph.

### Confirmation

Crate dependencies in `Cargo.toml` match the diagram. Fixture tests in `crates/cli/tests` pass end to end.

## Pros and Cons of the Options

### `parse` evaluates values itself

* Good, because no definitions cross the boundary.
* Bad, because evaluation, and later following imports across files, would sit in the one crate that can't see other files.

### Keep ASTs alive for the whole scan

* Good, because later stages can read anything from the AST.
* Bad, because every stage carries the arena lifetime, and memory grows with the whole codebase.
