# Fixtures

Each folder under a group (e.g. `identify/`) is one test case: a small project plus a hand-written `expected.json`.

```json
{
  "query": { "components": ["Icon"], "from": "lib" },
  "usages": [
    { "component": "Icon", "file": "App.tsx", "line": 5, "column": 5 }
  ]
}
```

- `query`: the scan to run, scoped to the case folder.
- `usages`: every expected opening JSX element, in output order: sorted by `file`, then `line`, then `column`. Closing tags aren't usages.
- `unidentified`, `errors`: optional, `{ file, line, column, code }` each, in output order, plus `component` for unidentified elements that refer to a target. `line` and `column` are omitted for errors without a location. Omitted means the scan must report none.
- `component`: the exported name, not the local alias.
- `file`: relative to the case folder.
- `line`, `column`: 1-based, pointing at `<`. `column` counts UTF-16 code units, as editors do.

Each group's `node_modules/` holds stub packages, such as `lib` and `other-lib`, shared by its cases. Imports resolve upward to them.

## identify

| Case | Covers |
|---|---|
| `named-import` | `import { Icon } from 'lib'` |
| `aliased-import` | `import { Icon as LibIcon } from 'lib'` |
| `namespace-import` | `import * as Lib from 'lib'; <Lib.Icon />` |
| `barrel-reexport` | `export { Icon } from 'lib'` and `export * from 'lib'` in a local barrel |
| `deep-import` | `import { Icon } from 'lib/components/icon'` |
| `member-expression` | `<Icon.Item />` reported as `Icon.Item`, separate from `Icon` |
| `same-name-other-module` | `Icon` from `other-lib` is not counted |
| `shadowed-local` | A local `Icon` shadowing the import is not counted |
| `local-alias` | `const I = Icon`, destructuring, chains, computed keys, optional chaining and reassigned names are unidentified with code `alias` ([ADR-0015](../docs/decisions/0015-es-modules-only.md)) |
| `commonjs` | `require('lib')` is unidentified with code `commonjs`; other modules aren't reported |
| `tsconfig-paths` | `tsconfig.json` `paths` aliases |
| `package-exports` | `package.json` `exports`, including subpaths |
| `commonjs-module` | A tag imported from a module without ES exports is unidentified with code `commonjs-module` |
| `unresolved-import` | An import that doesn't resolve is unidentified with code `unresolved` |
| `hidden-directory` | Files in hidden folders such as `.storybook` are scanned ([ADR-0014](../docs/decisions/0014-file-discovery.md)) |

## errors

Files that can't be scanned ([ADR-0010](../docs/decisions/0010-output.md)).

| Case | Covers |
|---|---|
| `syntax-error` | A file that can't be parsed is an error with code `syntax`; other files are still scanned |

## props

Each usage's `props` and `parents` ([ADR-0017](../docs/decisions/0017-prop-values.md)), compared only when the expected usage lists them. Extra `query` fields: `props` passes `--props`, `follow` passes `--follow`.

| Case | Covers |
|---|---|
| `literals` | Strings, numbers, booleans, `null` and `undefined` |
| `objects` | Object and array literals, with spreads and computed keys |
| `templates` | Template literals, with and without tags |
| `expressions` | The `unresolved` codes without `--follow`, ternaries, logical expressions, functions and elements |
| `parents` | Enclosing usages, and the parent's attribute a usage is in |
| `props-filter` | `--props` keeps the named props and spreads |
| `all-components` | Every export of `--from` without `--components` ([ADR-0018](../docs/decisions/0018-all-components.md)) |
| `follow-calls` | `--follow calls`: calls of names and members, with optional chaining and spread arguments, properties read from a result, the returns of functions written as arguments: arrow and function expressions, async functions and nested functions, and names in arguments and returns, which aren't followed, with `--follow props` too ([ADR-0024](../docs/decisions/0024-follow-calls.md), [ADR-0025](../docs/decisions/0025-names-in-calls.md)) |
| `follow-imports` | `--follow imports`: named, default and namespace imports, re-exports, values in other files, imports changed in place, and each code for an import that can't be followed ([ADR-0021](../docs/decisions/0021-follow-imports.md)) |
| `follow-props` | `--follow props`: props as names, patterns with defaults and rest elements, and in the body; callers through imports, namespaces, default exports, `memo` and `forwardRef`; chains of components, `{...props}` and `{...rest}`, `children`, a component that renders itself, callback parameters, `displayName`, and the `no-callers` and `other-callers` codes ([ADR-0022](../docs/decisions/0022-follow-props.md), [ADR-0023](../docs/decisions/0023-component-properties.md)) |
| `follow-variables` | `--follow variables`: chains, property access, destructuring and defaults, `mutated`, function declarations, names that refer back to themselves ([ADR-0019](../docs/decisions/0019-follow-variables.md)) |

## owners

Code owners of each usage ([ADR-0013](../docs/decisions/0013-code-owners.md)). Expected owners follow [GitHub's CODEOWNERS rules](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-code-owners).

Extra `query` fields:

- `owners`: `""` passes `--owners`; a path passes `--owners=<path>`, relative to the case folder.
- `dir`: scan root relative to the case folder, if not the case folder itself.

Extra top-level field: `warnings`, the skipped `CODEOWNERS` lines as `{ file, line, code }`.

A `.git` folder can't be committed inside this repository, so the test runner copies the group to a temporary folder and adds `.git` to the case folder, making it the repository root.

| Case | Covers |
|---|---|
| `last-match-wins` | A later line overrides an earlier one |
| `anchored-and-unanchored` | `apps/` matches at any depth, `/top/` only at the root |
| `direct-children` | `docs/*` matches files in `docs/` but not deeper |
| `double-star` | `**/logs` and `/src/**/*.test.tsx` |
| `no-owners` | A matching line without owners leaves files unowned |
| `multiple-owners` | Several owners on one line, email owners, comments |
| `case-sensitive` | `/SRC/` doesn't match `src/` |
| `invalid-lines` | `!` negation and invalid owners skip the whole line, with a warning |
| `later-lines-kept` | Lines after an invalid one still apply |
| `email-owners` | Email owners with long top-level domains, team names with `_` and `.` |
| `location-order` | `.github/CODEOWNERS` wins over `CODEOWNERS` and `docs/CODEOWNERS` |
| `subdirectory-scan` | Paths match from the repository root, not the scan root |
| `explicit-file` | `--owners=<path>` |
