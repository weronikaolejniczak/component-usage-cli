# Decisions

Architectural decision records in [MADR 4.0](https://adr.github.io/madr/) format.

| # | Decision | Status |
|---|---|---|
| [0000](./0000-use-madr.md) | Use Markdown Architectural Decision Records | accepted |
| [0001](./0001-rust-and-oxc.md) | Implement in Rust on top of Oxc | accepted |
| [0002](./0002-build-instead-of-forking-omlet.md) | Build a new tool instead of forking Omlet | accepted |
| [0003](./0003-no-type-checker.md) | Resolve values with static data flow, without a type checker | accepted |
| [0004](./0004-explicit-value-kinds.md) | Report every value with an explicit kind and a trace | accepted |
| [0005](./0005-choose-what-to-follow.md) | Let the user choose what the scan follows to find values | accepted |
| [0006](./0006-component-identification.md) | Identify components by resolved import binding | accepted |
| [0007](./0007-react-only.md) | Support React JSX only | accepted |
| [0008](./0008-distribute-via-npx.md) | Distribute through npm with per-platform binaries | accepted |
| [0009](./0009-opt-in-cache.md) | Cache scan results between runs, opt-in | proposed |
| [0010](./0010-output.md) | Output formats and query options | proposed |
| [0011](./0011-crate-layout.md) | Split the code into workspace crates by pipeline stage | accepted |
