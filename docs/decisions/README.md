# Decisions

Architectural decision records in [MADR 4.0](https://adr.github.io/madr/) format.

| # | Decision | Status |
|---|---|---|
| [0000](./0000-use-madr.md) | Use Markdown Architectural Decision Records | accepted |
| [0001](./0001-rust-and-oxc.md) | Implement in Rust on top of Oxc | superseded by [0016](./0016-rust-and-oxc.md) |
| [0002](./0002-build-instead-of-forking-omlet.md) | Build a new tool instead of forking Omlet | accepted |
| [0003](./0003-no-type-checker.md) | Resolve values with static data flow, without a type checker | accepted |
| [0004](./0004-explicit-value-kinds.md) | Report every value with an explicit kind and a trace | superseded by [0017](./0017-prop-values.md) |
| [0005](./0005-choose-what-to-follow.md) | Let the user choose what the scan follows to find values | accepted |
| [0006](./0006-component-identification.md) | Identify components by resolved import binding | accepted |
| [0007](./0007-react-only.md) | Support React JSX only | superseded by [0014](./0014-file-discovery.md) |
| [0008](./0008-distribute-via-npx.md) | Distribute through npm with per-platform binaries | accepted |
| [0009](./0009-opt-in-cache.md) | Cache scan results between runs, opt-in | proposed |
| [0010](./0010-output.md) | Output formats and query options | proposed |
| [0011](./0011-crate-layout.md) | Split the code into workspace crates by pipeline stage | superseded by [0012](./0012-architecture.md) |
| [0012](./0012-architecture.md) | Crate boundaries and interfaces | accepted |
| [0013](./0013-code-owners.md) | Attribute results to code owners | accepted |
| [0014](./0014-file-discovery.md) | Support React JSX only, and discover files including hidden folders | accepted |
| [0015](./0015-es-modules-only.md) | Follow ES module imports only, and report what isn't followed | accepted |
| [0016](./0016-rust-and-oxc.md) | Implement in Rust on top of Oxc | accepted |
| [0017](./0017-prop-values.md) | Report every prop of a usage as a tree of values with kinds | accepted |
| [0018](./0018-all-components.md) | Target every export of the module when no components are given | accepted |
