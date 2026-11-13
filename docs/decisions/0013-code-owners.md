---
status: proposed
date: 2026-11-13
decision-makers: Weronika Olejniczak
---

# Attribute results to code owners

## Context and Problem Statement

Before changing a component, its maintainers need to know which teams use it, and how, so they can ask for review or announce the change. Repositories with many teams record path ownership in a `CODEOWNERS` file. How does the scan report who owns each result?

## Decision Drivers

* Ownership must match what the code host shows for the same file.
* No network access or authentication.
* Output stays deterministic.
* Scans without a `CODEOWNERS` file work as before.

## Considered Options

* Read `CODEOWNERS` from the repository and attach owners to each result
* Leave it to post-processing of the JSON output with another tool
* Query the code host's API

## Decision Outcome

Proposed option: "Read `CODEOWNERS` and attach owners to each result", because matching rules are easy to get subtly wrong in a script, and the scan already has every path.

* `--owners` enables it and finds the file the way GitHub does: `.github/CODEOWNERS`, then `CODEOWNERS`, then `docs/CODEOWNERS`, in the repository root. `--owners <path>` uses a given file.
* The repository root is the scan root or its closest ancestor that contains `.git`. Patterns match paths relative to it, not to the scan root.
* Matching follows [GitHub's rules](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-code-owners): the last matching pattern wins, a matching pattern with no owners leaves the file unowned and matching is case-sensitive on every OS.
* A file of 3 MB or more is an error, since GitHub doesn't load it.
* Usages, unidentified elements and file errors each get an `owners` list. An empty list means unowned. Without `--owners` the field is absent.
* Lines GitHub would ignore as invalid are reported as warnings, with their line number.
* Parsing and matching live in their own pure crate; `engine` reads the file.

### Consequences

* Good, because results can be grouped by team, including which teams have unidentified usages.
* Good, because the same file a repository already maintains is the only input.
* Bad, because GitLab's sections and Bitbucket's syntax differ from GitHub's. Only GitHub's syntax is supported until another is needed.
* Bad, because ownership in the output is as current as the checked-out `CODEOWNERS`, which may differ from the default branch.

### Confirmation

A fixture per rule: last match wins, anchored and unanchored patterns, directory patterns, `*` and `**`, a pattern with no owners, and an invalid line. On the [benchmark codebase](../../benchmarks/README.md), owners of a hand-picked sample of files are compared with what GitHub shows for them.

## Pros and Cons of the Options

### Post-process the JSON output

* Good, because the scan stays smaller.
* Bad, because every user reimplements matching. GitHub's rules differ from `.gitignore` (no `!` negation, no `[ ]` ranges, no `\#` escape), so reusing a gitignore matcher gives wrong owners for files those features match.

### Query the code host's API

* Good, because it returns exactly what the host computes.
* Bad, because it needs network access and a token.
