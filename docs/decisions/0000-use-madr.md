---
status: accepted
date: 2026-11-12
decision-makers: Weronika Olejniczak
---

# Use Markdown Architectural Decision Records

## Context and Problem Statement

Design decisions for `component-usage-cli` need to be recorded with their reasoning and alternatives, in the repository, in a format both people and agents can read.

## Considered Options

* [MADR](https://adr.github.io/madr/) 4.0
* Single specification document
* No formal records

## Decision Outcome

Chosen option: "MADR 4.0", because each decision gets its own file with context, options, and consequences, and superseded decisions stay in history.

### Consequences

* Good, because decisions can be linked from code, PRs, and issues.
* Bad, because a reader needs several files to see the whole design.

## More Information

Records live in `docs/decisions/`, named `NNNN-title-with-dashes.md`. Index: [README.md](./README.md).
