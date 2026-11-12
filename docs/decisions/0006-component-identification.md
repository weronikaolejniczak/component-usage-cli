---
status: accepted
date: 2026-11-12
decision-makers: Weronika Olejniczak
---

# Identify components by resolved import binding

## Context and Problem Statement

`<Icon />` in a file may or may not be the target component. How does the tool decide a JSX element is a usage of the target?

## Considered Options

* Match the resolved import binding to the target's export
* Match the JSX tag name

## Decision Outcome

Chosen option: "Resolved import binding", because tag names are not unique and aliases change them.

Must handle:

| Pattern | Example |
|---|---|
| Named import | `import { Icon } from 'lib'` |
| Aliased import | `import { Icon as I } from 'lib'` |
| Namespace import | `import * as L from 'lib'; <L.Icon />` |
| Re-export / barrel | `export { Icon } from './icon'`, `export * from './icon'` |
| Deep import | `import { Icon } from 'lib/components/icon'` |
| Member expression | `<Icon.Item />` |

Import specifiers are resolved with [`oxc_resolver`](https://github.com/oxc-project/oxc-resolver): `tsconfig.json` `paths` and `baseUrl` (including project references), `package.json` `exports` / `main` / `module`, and monorepo workspace packages. Specifiers that can't be resolved are reported.

### Consequences

* Good, because aliased and re-exported usages are counted.
* Good, because same-named components from other modules aren't counted.
* Bad, because a module index must be built before matching.

### Confirmation

One fixture per pattern in the table.

## Pros and Cons of the Options

### Match the JSX tag name

* Good, because it needs no module resolution.
* Bad, because it misses aliases and counts unrelated components with the same name.
