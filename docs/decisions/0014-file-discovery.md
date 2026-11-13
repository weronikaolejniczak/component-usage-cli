---
status: accepted
date: 2026-11-13
decision-makers: Weronika Olejniczak
---

# Support React JSX only, and discover files including hidden folders

Supersedes [ADR-0007](./0007-react-only.md).

## Context and Problem Statement

[ADR-0007](./0007-react-only.md) limited the scanner to React JSX and described file discovery, including `--include` / `--exclude` options that don't exist yet. Folders such as `.storybook` hold components but were skipped as hidden. Which files does a scan read?

## Decision Drivers

* Components in hidden folders are usages like any other.
* Generated and vendored files listed in `.gitignore` aren't.
* The documented behaviour must match the implemented one.

## Considered Options

* Include hidden files, skipping `.git`
* Skip hidden files

## Decision Outcome

Chosen option: "Include hidden files, skipping `.git`", because hidden folders such as `.storybook` hold real usages, and `.gitignore` already excludes what shouldn't be scanned.

React JSX only stays, as in [ADR-0007](./0007-react-only.md): the [benchmark codebases](../../benchmarks/README.md) are React.

* Input files: `.js`, `.jsx`, `.ts`, `.tsx`, `.mjs`, `.cjs`, `.mts`, `.cts`, except declaration files (`.d.ts`, `.d.mts`, `.d.cts`).
* Discovery respects `.gitignore` and always skips `.git` and `node_modules`. Files outside that set are parsed only when an import or re-export reaches them.
* `--include` / `--exclude` aren't implemented. They need their own decision once a scan needs them.

### Consequences

* Good, because usages in hidden folders are counted.
* Bad, because a hidden folder of generated code that isn't in `.gitignore` is scanned too.
* Bad, because other frameworks need a later decision. Keep JSX-specific code in `parse` and `graph` where it's cheap to do so.

### Confirmation

A fixture with a usage in `.storybook/`.
