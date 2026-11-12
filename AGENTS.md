# AGENTS.md

Rust CLI built on Oxc. Reports React component usage and resolves prop values across files.

## Decisions

- Record design decisions as MADR 4.0 files in `docs/decisions/`, and add them to its index.
- Don't edit an accepted record to change a decision; add a new one that supersedes it.
- Keep decisions and code library-agnostic. Don't name specific libraries or codebases; link to `benchmarks/README.md` instead.

## Claims

Applies to README, docs, comments, commits, PRs, issues and release notes.

- No performance numbers or words like "fast" without a benchmark result in `benchmarks/`. Benchmarks have to be easily reproducible and the steps to reproduce documented.
- No accuracy numbers without checking against hand-verified results.
- No claims about other tools without running them on the same codebase and commit or citing their source at a specific version.
- No estimations.

## Benchmarks

- Run against the codebases listed in `benchmarks/README.md`, pinned to a commit SHA.
- Commit scripts, commands, tool versions and raw outputs to `benchmarks/`.
- Record hardware, OS, cold or warm cache, number of runs, median and spread.
- Include what the tool got wrong or couldn't resolve.

## Tests

- Add a fixture with a hand-written expected result before implementing a pattern.
- Don't change an expected result to make a test pass.
