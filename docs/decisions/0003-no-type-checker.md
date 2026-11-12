---
status: accepted
date: 2026-11-12
decision-makers: Weronika Olejniczak
---

# Resolve values with static data flow, without a type checker

## Context and Problem Statement

Resolving `type={icon}` means finding what `icon` can be. That can be done by following symbols, imports, and call sites or by asking a TypeScript type checker for the type of `icon`. Which approach?

## Decision Drivers

* Must run on plain JS as well as TS.
* Must not require a successful `tsc` build of the target project.
* Results must be traceable to source locations.

## Considered Options

* Static data flow over symbols, imports and call sites
* TypeScript compiler API (`tsc`)
* `tsgo` (TypeScript 7 native) API

## Decision Outcome

Chosen option: "Static data flow", because it works on any source that parses and each step maps to a source location for the trace.

### Consequences

* Good, because no dependency on the target project's TypeScript setup.
* Good, because every resolved value has a `file:line:column` trace.
* Bad, because values that only the type system knows (for example, a prop typed as a string literal union and never assigned) need special handling.

## Pros and Cons of the Options

### Static data flow

* Good, because literal unions in type annotations can still be read syntactically.
* Bad, because the data-flow engine has to be written.

### TypeScript compiler API

* Good, because full type information.
* Bad, because it needs a Node runtime and a working `tsconfig` for every project scanned.
* Bad, because types describe what's allowed, not what's passed.

### `tsgo` API

* Bad, because no Rust TypeScript checker is available (`stc` is abandoned).
