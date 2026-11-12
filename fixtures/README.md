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
- `usages`: every expected opening JSX element, sorted by `file`, then `line`, then `column`. Closing tags aren't usages.
- `component`: the exported name, not the local alias.
- `file`: relative to the case folder.
- `line`, `column`: 1-based, pointing at `<`.

`identify/node_modules/` holds stub packages (`lib`, `other-lib`) shared by all cases in the group. Imports resolve upward to them.

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
