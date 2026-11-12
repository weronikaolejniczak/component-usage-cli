---
status: proposed
date: 2026-11-12
decision-makers: Weronika Olejniczak
---

# Cache scan results between runs, opt-in

## Context and Problem Statement

Repeated scans of the same codebase parse mostly unchanged files. Should results be cached between runs?

## Decision Drivers

* A cache must never return stale results.
* A cache is only worth its complexity if warm scans are measurably faster.

## Considered Options

* Opt-in per-file cache
* Always-on cache
* No cache

## Decision Outcome

Proposed option: "Opt-in per-file cache", shipped only if benchmarks show warm scans are faster than cold scans.

* Enabled with `--cache`, disabled with `--no-cache`, location set with `--cache-dir`.
* Entries keyed by file content hash, tool version and config hash.
* When a file's exports change, files that depend on it are invalidated.

### Consequences

* Good, because default behavior is uncached and always correct.
* Bad, because dependent invalidation adds complexity to `graph`.

### Confirmation

Cold vs warm scan times on the [benchmark codebases](../../benchmarks/README.md). If warm scans aren't faster, this record is marked rejected and the cache is removed.
