---
status: accepted
date: 2026-11-13
decision-makers: Weronika Olejniczak
---

# Implement in Rust on top of Oxc

Supersedes [ADR-0001](./0001-rust-and-oxc.md), which said a TypeScript and Babel implementation is "much slower" without a benchmark. The decision is unchanged.

## Context and Problem Statement

The tool parses every JS/TS file in large monorepos and needs scope and symbol information to follow values across files. Which language and parser stack?

## Decision Drivers

* Parse and semantic analysis must run on every file of a large monorepo.
* Needs scopes, symbols, and references, not only an AST.
* Needs a module resolver that understands `tsconfig` paths and package `exports`.
* Distributed as a standalone binary.

## Considered Options

* Rust + Oxc (`oxc_parser`, `oxc_semantic`, `oxc_resolver`)
* Rust + swc
* TypeScript + Babel (`@babel/parser`, `@babel/traverse`)
* Write a parser

## Decision Outcome

Chosen option: "Rust + Oxc", because it provides parser, semantic analysis and module resolver from one project, and compiles to a standalone binary.

The minimum supported Rust version is the pinned toolchain's, since that's the only one tested.

### Consequences

* Good, because scopes, symbols and references come from `oxc_semantic` instead of being built by hand.
* Good, because `oxc_resolver` handles `tsconfig` `paths` and package `exports`.
* Bad, because Oxc's semantic API is younger than Babel's; breaking changes between versions are possible.

### Confirmation

Scan time on the [benchmark codebases](../../benchmarks/README.md). Not measured yet.

## Pros and Cons of the Options

### Rust + Oxc

* Good, because parser, semantic analysis and resolver are maintained together.
* Neutral, because Oxc publishes its own performance benchmarks; not verified for this use case.

### Rust + swc

* Bad, because semantic analysis and resolution need extra crates or custom code.

### TypeScript + Babel

* Good, because it's the most familiar stack for JS developers.
* Bad, because it needs a Node runtime, so it can't ship as a standalone binary. Speed wasn't compared.

### Write a parser

* Bad, because JS/TS/JSX parsing is a large, solved problem.
