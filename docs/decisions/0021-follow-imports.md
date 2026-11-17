---
status: accepted; values interface superseded by ADR-0022
date: 2026-11-17
decision-makers: Weronika Olejniczak
---

# Follow imports to the exported value, and say which file each value is in

Supersedes the `values` interface in [ADR-0020](./0020-architecture.md).

## Context and Problem Statement

With `--follow variables` ([ADR-0019](./0019-follow-variables.md)), an imported name is `unresolved` with code `not-followed` and step `imports`. In the [benchmarks](../../benchmarks/README.md), these are often constants in another file of the same codebase, imported by name or through a namespace: `import * as i18n from './translations'`, then `i18n.TITLE`. How are imports followed, how does the report say a value is in another file, and what does it say when an import can't be followed?

## Decision Drivers

* A followed value must say where it is, in any file, so a tool or agent can change it.
* No silent misses: an import that can't be followed is reported with a reason.
* The same rules as variables, so a value reads the same wherever it's defined.
* Only files a followed value needs are resolved and parsed.

## Considered Options

* Follow imports through the module graph, with a `file` on values in other files
* Follow imports only within the scanned directory
* Inline values from other files without saying which file

## Decision Outcome

Chosen option: "Follow imports through the module graph, with a `file` on values in other files", because identification already resolves imports and re-exports, and a value without its file can't be changed.

`--follow imports` follows an imported name to the export it refers to, through re-exports, as components are identified ([ADR-0006](./0006-component-identification.md)), and then follows the exported binding as `variables` does. It implies `variables`: an exported binding is a variable of its file.

* `import { NAME }` and `import NAME` follow the export `NAME` or `default`.
* `import * as ns`, then `ns.NAME`, follows the export `NAME`. A key that isn't static, as in `ns[key]`, gives `unsupported`.
* `export default` followed by an expression gives that expression.
* Files outside the scanned directory, such as packages in `node_modules`, are followed too, as long as they resolve and parse.

A value, or a name in `via`, that is in another file than the usage has a `file`, the path as in the usage's `file`. Values in the usage's file have none.

An exported binding is `mutated` when [ADR-0019](./0019-follow-variables.md) says so in its file, or when any parsed file that imports it changes it in place: `ICONS.edit = …`, `ICONS.push(…)`, `Object.assign(ICONS, …)`. A namespace import changed in place, such as `ns.ICONS.edit = …`, marks every export of that module.

New `unresolved` codes, in addition to [ADR-0017](./0017-prop-values.md) and [ADR-0019](./0019-follow-variables.md):

| Code | Meaning |
|---|---|
| `unresolved-import` | The import's specifier doesn't resolve to a file |
| `unparsed` | The file it resolves to couldn't be parsed, or isn't JavaScript or TypeScript, such as `.json` |
| `missing-export` | The module doesn't export the name |
| `commonjs-module` | The module has no ES exports; CommonJS exports aren't read ([ADR-0015](./0015-es-modules-only.md)) |
| `ambiguous` | The name is exported by more than one `export *` |

A name declared without a value, such as `declare const NAME: string`, is `unsupported`, in the file and through imports. Re-exports that form a cycle, and names that refer back to themselves across files, are `unsupported`, as in [ADR-0019](./0019-follow-variables.md).

`values` needs the module graph. Its interface becomes `Context::new(&graph, &paths, imports)` once per scan, where `paths` are the display paths by `FileId`, and `props(&context, file, &element, filter) -> Vec<Prop>`. `parse` records, when following imports, what each export holds and which imports the file changes in place; `graph` lists the specifiers the recorded values need, so only those are resolved.

### Consequences

* Good, because constants, icon sets and translations in other files are read, with the file to change.
* Good, because a value is followed the same way in any file.
* Good, because every import that can't be followed says why.
* Bad, because every parsed file records what its exports hold, which costs time and memory whether or not a usage reads them.
* Bad, because a path outside the scanned directory is absolute, so it differs between machines.
* Bad, because a value shared through an import is repeated in every usage that reads it, now across files.

### Confirmation

A fixture for `--follow imports` covering each form and code above. The existing fixtures don't change. Benchmarks report time, memory and counts per kind with and without it.

## Pros and Cons of the Options

### Follow imports only within the scanned directory

* Good, because no files outside it are parsed for values, and every path is relative.
* Bad, because constants from packages, such as a design system's tokens, stay unread.
* Bad, because scanning a subdirectory changes what is followed, in a way the report doesn't show.

### Inline values without their file

* Good, because smaller output.
* Bad, because a location without a file points at the wrong file.
