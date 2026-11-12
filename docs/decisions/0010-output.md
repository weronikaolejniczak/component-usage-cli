---
status: proposed
date: 2026-11-12
decision-makers: Weronika Olejniczak
---

# Output formats and query options

## Context and Problem Statement

Output is read by people, scripts, and agents. Large codebases produce many call sites. What formats and filters does the CLI offer?

## Decision Drivers

* Scripts need a stable machine-readable format.
* Agents and people usually need aggregates, not every call site.
* Same input and options must produce byte-identical output.

## Considered Options

* `json`, `jsonl`, `toon`, `csv`, with `--summary` and `--filter-kind`
* `json` only

## Decision Outcome

Proposed option: multiple formats with query options.

* `--format json|jsonl|toon|csv`
* `--summary`: distinct values and counts per component and prop, no per-site data.
* `--filter-kind <kinds>`: only values of the given kinds (see [ADR-0004](./0004-explicit-value-kinds.md)).
* `--out <file>`.

### Consequences

* Good, because `jsonl` streams and `csv` loads into spreadsheets.
* Bad, because every format has to be kept in sync with the value model.

### Confirmation

Snapshot tests per format. Token counts of `toon` vs `json` on real outputs measured before `toon` is kept; TOON's own size claims are not verified here.
