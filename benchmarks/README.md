# Benchmarks

## Reproduce

```sh
# In a Kibana checkout at the commit below, bootstrapped so `node_modules` exists.
benchmarks/run.sh <kibana> 5 <name> -- . \
  --components EuiIcon,EuiBasicTable,EuiInMemoryTable,EuiComboBox,EuiSelectable,EuiPopover,EuiContextMenu,EuiContextMenuPanel,EuiFlyout,EuiDataGrid \
  --from @elastic/eui --owners
python3 benchmarks/stats.py benchmarks/results/<name>/report.json.gz
```

`run.sh` records the environment, the time of each run and the last report in `results/<name>/`.

```sh
# Every EUI component in a checkout, with react-scanner and with this tool; needs Node and npm.
benchmarks/alternatives/run.sh <checkout> <runs> <name> @elastic/eui

# Owners in a report against another implementation, here github.com/hmarr/codeowners v1.2.1.
benchmarks/owners.py <checkout> benchmarks/results/<name>/report.json.gz codeowners
```

```sh
# The use cases in use-cases/, from one report of every EUI export with owners, following
# variables. Exports Kibana
# at <sha> into <work-dir>, with @elastic/eui and its @kbn packages; needs npm and codeowners.
benchmarks/use-cases/run.sh <kibana> <sha> <work-dir>
```

## Results

Kibana at `6dce6e1bbcf1fcc46be7011088c3aa1e81feba19`: 105,517 source files scanned. Apple M3 Pro, 11 cores, macOS 26.7.3, rustc 1.99.1, warm cache, 5 runs.

| Tool commit | Median | Min | Max |
|---|---|---|---|
| [`b348c32`](./results/kibana-b348c32/) | 26.56 s | 26.06 s | 27.73 s |
| [`55fb6a9`](./results/kibana-55fb6a9/) | 4.41 s | 4.00 s | 5.46 s |
| [`09dad13`](./results/kibana-09dad13/) | 3.90 s | 3.81 s | 4.00 s |

`b348c32` and `55fb6a9` produce byte-identical reports. `55fb6a9` caches export lookups and runs file discovery and `CODEOWNERS` matching in parallel; matching was most of the time before. `09dad13` gives unidentified elements from CommonJS modules their own code; usages are unchanged.

Report: 4,640 usages in 3,084 files, 0 errors, 0 warnings.

| Component | Usages | Files | Owners |
|---|---|---|---|
| `EuiIcon` | 1,827 | 1,280 | 30 |
| `EuiPopover` | 819 | 791 | 29 |
| `EuiComboBox` | 505 | 426 | 27 |
| `EuiFlyout` | 358 | 350 | 27 |
| `EuiBasicTable` | 312 | 294 | 27 |
| `EuiContextMenuPanel` | 223 | 212 | 24 |
| `EuiInMemoryTable` | 204 | 187 | 23 |
| `EuiSelectable` | 186 | 174 | 25 |
| `EuiContextMenu` | 178 | 156 | 22 |
| `EuiDataGrid` | 28 | 28 | 13 |

### Following variables and imports

The same scan with `--follow none`, `variables` and `imports`, at tool commit `973261e`, on an export of the same Kibana commit with only `@elastic/eui` 122.0.0 and the `@kbn` packages in `node_modules`, as [`use-cases/run.sh`](./use-cases/run.sh) sets up. Without the other dependencies, elements imported from them are `unidentified` (16,498); usages are the same 4,640. Same machine, warm cache, 5 runs.

```sh
CODEBASE="<origin> <sha> (git archive, ...)" benchmarks/run.sh <export> 5 <name> -- . --components <the 10 above> --from @elastic/eui --owners --follow <none|variables|imports>
python3 benchmarks/stats.py benchmarks/results/<name>/report.json.gz 0
```

| `--follow` | Median | Min | Max | Peak memory | Values at any depth |
|---|---|---|---|---|---|
| [`none`](./results/kibana-973261e-follow-none/) | 5.87 s | 5.72 s | 9.56 s | 2.35–2.77 GB | 28,395 |
| [`variables`](./results/kibana-973261e-follow-variables/) | 5.45 s | 5.07 s | 5.66 s | 2.80 GB | 36,781 |
| [`imports`](./results/kibana-973261e-follow-imports/) | 5.57 s | 5.20 s | 10.15 s | 3.05 GB | 37,394 |

The differences in time are smaller than the spread between runs of the same command; one run of `none` and one of `imports` took about twice the others. Peak memory varied between runs only without following. Of the 23,195 prop values, without following, 5,960 are `not-followed` variables and 597 `not-followed` imports. With `--follow imports`, 8 are left, all `not-followed` with step `variables`: 5 enum members, such as `TRANSFORM_LIST_COLUMN.ID`, and 3 names from destructuring with a rest element or a default, such as `const { isResizable, ...restOptions } = options`. The values that don't resolve are, by code: `call` 1,621 and 4,532 followed, `parameter` 1,440 and 82 followed, `unsupported` 304 and 79 followed, `mutated` 74, `unparsed` 72. Counts for each kind are in each result's `stats.txt`.

### Following props

The same scan with `--follow props`, at tool commit `5be0b90`, on the same export, with `--follow imports` rerun at that commit right before it for comparison. Same machine, warm cache, 5 runs each.

| `--follow` | Median | Min | Max | Peak memory | Report | Values at any depth |
|---|---|---|---|---|---|---|
| [`imports`](./results/kibana-5be0b90-follow-imports/) | 5.48 s | 5.20 s | 5.83 s | 3.07 GB | 22.9 MB | 37,394 |
| [`props`](./results/kibana-5be0b90-follow-props/) | 7.33 s | 6.86 s | 7.59 s | 3.57–3.64 GB | 60.5 MB | 55,120 |

The `imports` report differs from the one at `973261e` in 4 values: a destructuring default now replaces `undefined` inside a `one-of`, as in `const { icon = 'processor' } = map.get(id) ?? {}`.

Of the 23,195 prop values, `parameter` goes from 1,440 and 82 followed to 214 and 48 followed, and `one-of` followed from 316 to 1,251. 23 and 3 followed are `no-callers`. At any depth, 655 values are `other-callers`: a component on the way is also used other than as a JSX tag.

The largest value, 5.2 MB of the 19.3 MB of prop values as compact JSON, is a `{...otherProps}` spread on `EuiDataGrid` in `x-pack/solutions/security/packages/data-table/components/data_table/index.tsx`. It is a `one-of` of 15 callers; 12 are in `index.test.tsx` and each spreads the test's `props` object, directly or through another object, which holds imported mock data such as `mockTimelineData`: about 430 KB per caller.

### Following calls

The same scan with `--follow props,calls`, at tool commit `a2588c2`, on the same export, with `--follow props` rerun at that commit right before it for comparison. Same machine, warm cache, 5 runs each. `stats.py` now also counts values in spreads, calls and returns, so values at any depth aren't comparable with the rows above.

| `--follow` | Median | Min | Max | Peak memory | Report | Values at any depth |
|---|---|---|---|---|---|---|
| [`props`](./results/kibana-a2588c2-follow-props/) | 5.67 s | 5.59 s | 5.80 s | 3.63–3.64 GB | 60.3 MB | 56,487 |
| [`props,calls`](./results/kibana-a2588c2-follow-props-calls/) | 5.88 s | 5.87 s | 5.99 s | 3.96–3.97 GB | 92.0 MB | 128,797 |

Of the 23,195 prop values, the 6,348 that were `unresolved` `call`, 4,727 of them followed, are now `call`. The most common callees among them are `i18n.translate` (1,837), `useMemo` (1,019), `useState` (884) and `useCallback` (679). At any depth, 13,775 values are names in a call's arguments, `in-call` ([ADR-0025](../docs/decisions/0025-names-in-calls.md)).

5,421 receivers are `mutated`, such as `i18n` in `i18n.translate(…)`: `src/core/packages/i18n/browser-internal/src/i18n_eui_mapping.test.ts` assigns `i18n.translate = i18nTranslateMock`, so the imported `i18n` is changed in place ([ADR-0021](../docs/decisions/0021-follow-imports.md)).

Before ADR-0025, names in a call's arguments were followed, and a value was written out wherever it was reached. One run of the same scan wrote a 102 GB report in 210.5 seconds, with a peak memory footprint of 30.2 GB.

### Wrappers and references

The same scan with `--follow props`, at tool commit `e7ba3c1`, which adds `references` ([ADR-0027](../docs/decisions/0027-references.md), [ADR-0028](../docs/decisions/0028-references-of-unrendered-wrappers.md)), with and without `--wrappers` ([ADR-0026](../docs/decisions/0026-wrappers.md)), then at `a2588c2` for comparison. Same export and machine, warm cache, 5 runs each, back to back. The load average was 5.4 before the first scan and rose to 14.6 by the end of the last, with the scans using every core.

| Commit | Flags | Median | Min | Max | Peak memory | Report | Usages | Through wrappers | References |
|---|---|---|---|---|---|---|---|---|---|
| [`a2588c2`](./results/kibana-a2588c2-follow-props-rerun/) | | 5.87 s | 5.77 s | 6.15 s | 3.63–3.64 GB | 60.3 MB | 4,640 | | |
| [`e7ba3c1`](./results/kibana-e7ba3c1-follow-props/) | | 6.17 s | 5.94 s | 8.24 s | 3.74–3.75 GB | 60.3 MB | 4,640 | | 136 |
| [`e7ba3c1`](./results/kibana-e7ba3c1-follow-props-wrappers/) | `--wrappers` | 6.20 s | 5.92 s | 7.12 s | 3.76–3.77 GB | 80.8 MB | 4,937 | 297 | 113 |

Finding references needs each node's parent, which the parser now records for every file, with or without `--wrappers`.

With `--wrappers`, the other 4,640 usages are the same, and 297 more go through 76 wrappers: 278 through one, 19 through two. By target: `EuiIcon` 142, `EuiComboBox` 49, `EuiPopover` 38, `EuiInMemoryTable` 22, `EuiBasicTable` 16, `EuiDataGrid` 16, `EuiFlyout` 9, `EuiSelectable` 4, `EuiContextMenu` 1. Eight of them, six picked at random with seed 7 and the first two through two wrappers, were read in the source: each wrapper spreads its props or rest into the next step, or holds `styled(EuiBasicTable as …)`, and the props left out are the ones its rest pattern takes.

Of the 136 references without `--wrappers`, 127 are call arguments, 6 variables, 2 other and 1 an attribute. Most are in tests: `find` on a rendered tree, as in `wrapper.find(EuiComboBox)`, is the callee of 102. `styled` (19) and `euiStyled` (3) are others. With `--wrappers`, 23 references are left out, each a step in a usage's `through`, among them every `styled` and `euiStyled` call. A callee is its source text, line breaks included.

### Checked by hand

`EuiIcon` positions were compared with a text search for `<EuiIcon` followed by whitespace, `>`, `/` or the end of the line. The scan found every match the search found, except 20, and none the search missed. Each of the 20 was read: all are in comments or template strings, such as inline snapshots, so they aren't usages. The search can't see usages through generics (`<EuiBasicTable<T>`), which is why `EuiBasicTable` has more usages than a text search finds.

### Owners

Owners in the [`kibana-alternatives`](./results/kibana-alternatives/) report, 12,564 files, were compared with two other implementations of the `CODEOWNERS` rules. They match for every file, including the order of owners:

- `codeowners` from [hmarr/codeowners](https://github.com/hmarr/codeowners) v1.2.1, with `owners.py`.
- Kibana's own `getOwningTeamsForPath` from `@kbn/code-owners` at the pinned SHA.
### What the tool got wrong or couldn't resolve

Of 2,375 unidentified elements in the `09dad13` report:

- 2,373 are `commonjs-module`: tags imported from packages without ES exports, mainly `react` and `@elastic/charts`, whose exports aren't read.
- 1 is `missing-export`: an example app imports `SideNavigation` from a package index that doesn't export it.
- 1 is `alias`: `EuiPopover` through a local constant.

The [`kibana-alternatives`](./results/kibana-alternatives/) report, which covers every EUI component, has one more `alias`: `let ButtonComponent = EuiButton`, reassigned to `EuiButtonIcon` in an `if`. It is reported with `component: EuiButton`, because reassignments of `let` aren't checked, though the element can render either.

Not reported at all, because they aren't JSX elements whose name is bound to an import:

- Components passed as values: `styled(EuiIcon)`, `createElement(EuiIcon)`, `component={EuiIcon}`, maps of components.
- Constants exported from one file and rendered in another.

## Alternatives

Same machine as above. Each tool ran once to warm the cache, then 3 times. Both tools were asked for every component react-scanner found imported from `@elastic/eui`: 221, members such as `EuiSplitPanel.Inner` included. This tool also matched owners.

| Tool | Median | Min–max | Peak memory | Usages |
|---|---|---|---|---|
| component-usage-cli | 7.77 s | 6.20–8.89 s | 2.7 GB | 87,239 |
| react-scanner 1.2.0 | 103.20 s | 95.19–139.05 s | 1.1 GB | 87,219 |

Every usage react-scanner found, this tool found at the same position. The 20 more are in two files that import EUI components through a local module re-exporting `@elastic/eui` (`remote_clusters/public/shared_imports.ts`); react-scanner only matches the import source by name. react-scanner also skipped files it couldn't parse, printing `Failed to parse` for 480 files ([`react-scanner-stderr.txt`](./results/kibana-alternatives/react-scanner-stderr.txt)); this tool parsed them all. None of those files had EUI usages in this tool's report.

react-scanner records props, which this tool doesn't yet. It records literal values, such as `type: "warning"`, and only the kind of any other expression, such as `(Identifier)`.

For `EuiIcon` only, on Kibana, with [`measure.py`](./alternatives/measure.py):

| Tool | Median of 3 | Peak memory | Found | Not usages | Missed |
|---|---|---|---|---|---|
| [ast-grep](https://ast-grep.github.io) 0.51.2, [`ast-grep-rule.yml`](./alternatives/ast-grep-rule.yml) | 2.06 s | 22 MB | 1,792 | 0 | 35 |
| [ripgrep](https://github.com/BurntSushi/ripgrep) 15, `<EuiIcon\b` | 4.87 s | 34 MB | 1,914 | 87 | 0 |

ast-grep missed the 35 usages in `.js` files, because a rule has one language and `tsx` doesn't cover `.js`. Neither checks where `EuiIcon` is imported from. The 87 ripgrep matches that aren't usages are in Markdown, snapshots, templates, comments and strings.

## Use cases

- [Removing icon aliases](./use-cases/icon-aliases.md)
- [Select components](./use-cases/select-components.md)
- [Style overrides](./use-cases/style-overrides.md)

## Codebases

| Role | Codebase | Notes |
|---|---|---|
| Primary benchmark | [elastic/kibana](https://github.com/elastic/kibana) | Large public TypeScript monorepo that consumes `@elastic/eui`. File count and lines of code are recorded with each run at the pinned SHA. |
| Component library | [elastic/eui](https://github.com/elastic/eui) | Source of the scanned components. Example query: every value `EuiIcon` `type` and `iconType` receive ([elastic/eui#9967](https://github.com/elastic/eui/issues/9967)). |

## Each run records

- Codebase and commit SHA
- Tool version and exact command
- Hardware, OS
- Cold or warm cache
- Number of runs, median and spread
- Raw output
- Counts per value kind, including `unresolved` with reasons
