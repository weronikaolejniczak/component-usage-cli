---
status: accepted; reporting local constants as unidentified superseded by ADR-0029
date: 2026-11-13
decision-makers: Weronika Olejniczak
---

# Follow ES module imports only, and report what isn't followed

## Context and Problem Statement

[ADR-0006](./0006-component-identification.md) identifies components by resolved import binding. A component can also reach a JSX tag through `require`, or through a local constant: `const I = Icon`, `const { Icon } = Lib`. Silently skipping those undercounts usages without saying so. What does the scan do with them?

## Decision Drivers

* No silent misses: anything that may be a usage is either counted or reported.
* Following constants is the same data flow as resolving prop values, which is planned separately ([ADR-0003](./0003-no-type-checker.md)).
* Counted usages must be exact.

## Considered Options

* Count them as usages
* Report them as unidentified, with the component they refer to
* Skip them

## Decision Outcome

Chosen option: "Report them as unidentified", because the scan can tell which target they refer to without committing to the semantics of constants and CommonJS before value resolution exists.

* ES module imports are followed, as in [ADR-0006](./0006-component-identification.md).
* A JSX tag whose root is a constant initialised from an import, a `require('…')` call with a string literal, or a static property chain on one of them, followed through other such constants and object destructuring, is resolved. If it refers to a target, it's reported as unidentified with code `alias` or `commonjs` and the target's name in `component`.
* `require` is resolved with the same resolver and conditions as `import`. `module.exports` isn't read, so a target defined only through it isn't found.
* Re-exported constants (`export const MyIcon = Icon` imported elsewhere) aren't followed; they need value resolution.

### Consequences

* Good, because usages through aliases and `require` show up in the output, with the component they refer to.
* Bad, because they aren't counted as usages until value resolution follows constants.
* Bad, because re-exported constants are still missed silently.

### Confirmation

Fixtures for local aliases and for `require`.
