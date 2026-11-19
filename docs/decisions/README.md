# Decisions

Architectural decision records in [MADR 4.0](https://adr.github.io/madr/) format.

| # | Decision | Status |
|---|---|---|
| [0000](./0000-use-madr.md) | Use Markdown Architectural Decision Records | accepted |
| [0001](./0001-rust-and-oxc.md) | Implement in Rust on top of Oxc | superseded by [0016](./0016-rust-and-oxc.md) |
| [0002](./0002-build-instead-of-forking-omlet.md) | Build a new tool instead of forking Omlet | accepted |
| [0003](./0003-no-type-checker.md) | Resolve values with static data flow, without a type checker | accepted |
| [0004](./0004-explicit-value-kinds.md) | Report every value with an explicit kind and a trace | superseded by [0017](./0017-prop-values.md) |
| [0005](./0005-choose-what-to-follow.md) | Let the user choose what the scan follows to find values | superseded by [0019](./0019-follow-variables.md) |
| [0006](./0006-component-identification.md) | Identify components by resolved import binding | accepted |
| [0007](./0007-react-only.md) | Support React JSX only | superseded by [0014](./0014-file-discovery.md) |
| [0008](./0008-distribute-via-npx.md) | Distribute through npm with per-platform binaries | accepted |
| [0009](./0009-opt-in-cache.md) | Cache scan results between runs, opt-in | proposed |
| [0010](./0010-output.md) | Output formats and query options | proposed; `--summary` superseded by [0030](./0030-summary.md) |
| [0011](./0011-crate-layout.md) | Split the code into workspace crates by pipeline stage | superseded by [0012](./0012-architecture.md) |
| [0012](./0012-architecture.md) | Crate boundaries and interfaces | superseded by [0020](./0020-architecture.md) |
| [0013](./0013-code-owners.md) | Attribute results to code owners | accepted |
| [0014](./0014-file-discovery.md) | Support React JSX only, and discover files including hidden folders | accepted |
| [0015](./0015-es-modules-only.md) | Follow ES module imports only, and report what isn't followed | accepted; reporting local constants as unidentified superseded by [0029](./0029-local-aliases.md) |
| [0016](./0016-rust-and-oxc.md) | Implement in Rust on top of Oxc | accepted |
| [0017](./0017-prop-values.md) | Report every prop of a usage as a tree of values with kinds | accepted; operators superseded by [0036](./0036-operations.md); `this.props` superseded by [0038](./0038-class-components.md) |
| [0018](./0018-all-components.md) | Target every export of the module when no components are given | accepted |
| [0019](./0019-follow-variables.md) | Follow variables on request, and report the path to each value | accepted |
| [0020](./0020-architecture.md) | Crate boundaries and interfaces, with values | accepted; `values` interface superseded by [0021](./0021-follow-imports.md) |
| [0021](./0021-follow-imports.md) | Follow imports to the exported value, and say which file each value is in | accepted; `values` interface superseded by [0022](./0022-follow-props.md); absolute paths superseded by [0034](./0034-paths-outside-the-root.md) |
| [0022](./0022-follow-props.md) | Follow a component's props to what its callers pass | accepted; `other-callers` definition superseded by [0023](./0023-component-properties.md); class components superseded by [0038](./0038-class-components.md); called functions superseded by [0039](./0039-function-parameters.md) |
| [0023](./0023-component-properties.md) | Reading or setting a property of a component doesn't add `other-callers` | accepted |
| [0024](./0024-follow-calls.md) | Describe calls: what's called, with what, and what the functions passed return | accepted; following in arguments superseded by [0025](./0025-names-in-calls.md) |
| [0025](./0025-names-in-calls.md) | Names in a call's arguments aren't followed | accepted |
| [0026](./0026-wrappers.md) | Report a wrapper's usages as usages of the components it passes its props to | accepted; props of usages through wrappers superseded by [0031](./0031-props-wrappers-set.md) |
| [0027](./0027-references.md) | Report the places a target is passed as a value | accepted; leaving out wrappers' arguments superseded by [0028](./0028-references-of-unrendered-wrappers.md); not following local names superseded by [0029](./0029-local-aliases.md) |
| [0028](./0028-references-of-unrendered-wrappers.md) | Keep the references of wrappers nothing renders | accepted |
| [0029](./0029-local-aliases.md) | Count usages through local names that are never assigned again | accepted |
| [0030](./0030-summary.md) | Summarise a scan with counts per component, prop and owner | accepted; counting props superseded by [0033](./0033-summary-spreads.md) |
| [0031](./0031-props-wrappers-set.md) | Add the props wrappers set to the usages that go through them | accepted |
| [0032](./0032-export-aliases.md) | Name a component exported under several names by the name it's imported as | accepted |
| [0033](./0033-summary-spreads.md) | Apply spreads when counting props in the summary | accepted; wrappers' forwarding spreads superseded by [0035](./0035-summary-forwarding-spreads.md) |
| [0034](./0034-paths-outside-the-root.md) | Show paths outside the scan root relative to it | accepted |
| [0035](./0035-summary-forwarding-spreads.md) | Count what a wrapper passes on once in the summary | accepted |
| [0036](./0036-operations.md) | Report operators with their operands | accepted |
| [0037](./0037-callback-items.md) | Follow the items of arrays into callbacks | accepted |
| [0038](./0038-class-components.md) | Follow `this` in class components | accepted |
| [0039](./0039-function-parameters.md) | Follow the parameters of functions to their calls | accepted |
