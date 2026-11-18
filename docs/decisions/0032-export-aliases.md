---
status: accepted
date: 2026-11-18
decision-makers: Weronika Olejniczak
---

# Name a component exported under several names by the name it's imported as

## Context and Problem Statement

A module can export one component under several names, as in `export { Icon, Icon as Glyph }`. Usages are matched by the export a tag resolves to ([ADR-0006](./0006-component-identification.md)), so `<Icon>` and `<Glyph>` match the same component, and each was reported under one of its names: the first requested with `--components`, or the first in alphabetical order without. `<Icon>` was reported as `Glyph`, and with `--components Icon,Glyph`, `Glyph` had no usages. Which name does a usage get?

## Considered Options

* The name the file imports
* One name per component, as before

## Decision Outcome

Chosen option: "The name the file imports", because it's the name the reader sees in the file.

* The name is the export name last written on the way to the component: `Glyph` in `import { Glyph } from …` or in `<L.Glyph>` with `import * as L`. When it's one of the component's names, the usage, or the reference, gets it.
* Otherwise, such as for a local name for a default import or an import from a module that renames it, the usage gets the first name as before: the first requested, or the first in alphabetical order.

### Consequences

* Good, because `<Icon>` and `<Glyph>` are reported as written, and each requested name gets its usages.
* Bad, because a file importing through a module that renames the component gets the first name, not the one that module uses.

### Confirmation

Identify fixtures with both names imported and through a namespace, with and without `--components`.
