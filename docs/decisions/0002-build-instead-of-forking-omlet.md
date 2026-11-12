---
status: accepted
date: 2026-11-12
decision-makers: Weronika Olejniczak
---

# Build a new tool instead of forking Omlet

## Context and Problem Statement

[Omlet](https://github.com/zeplin/omlet) (MIT, open-sourced April 2026) has a Rust scanner that identifies components and props across a codebase. Should `component-usage` fork it or start from scratch?

## Decision Drivers

* The core feature is resolving prop values across files.
* The scanner must run without login or upload.
* Stack per [ADR-0001](./0001-rust-and-oxc.md).

## Considered Options

* Fork Omlet's CLI
* Build new, reusing ideas and code from Omlet under MIT

## Decision Outcome

Chosen option: "Build new", because Omlet's value model, swc version and login/upload flow would each need replacing.

### Consequences

* Good, because the value model is designed for resolution from the start.
* Bad, because module resolution and component identification have to be rebuilt.

## Pros and Cons of the Options

### Fork Omlet's CLI

* Good, because component identification (re-exports, barrels, aliases, HOCs) already exists.
* Bad, because prop values are recorded only up to `Identifier { name }`.
* Bad, because it's on swc 0.185 and the scanner is coupled to the web app's login and upload.

### Build new

* Good, because the parts worth reusing can be ported directly (MIT).
* Bad, because more code to write up front.

## More Information

Parts of Omlet to study or port:

* `cli/crates/core/src/module_resolver.rs` and `resolve_export_recursive`: follows re-exports, barrels, and aliases to the original definition.
* `InferredType` in `parser/inspector.rs` (`ReturnTypeOf`, `ParameterOf`, `MemberOf`, `Union`, `Object`, `Str`, `Unknown`): starting point for the value model.
* Config overrides for `exports` and `aliases`.

Facts about Omlet above come from reading its source; versions not yet pinned.
