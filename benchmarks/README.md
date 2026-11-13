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

The use cases in [`use-cases/`](./use-cases/) are answered from the reports of `alternatives/run.sh`.

```sh
# Every EUI component in a checkout, with react-scanner and with this tool; needs Node and npm.
benchmarks/alternatives/run.sh <checkout> <runs> <name> @elastic/eui

# Owners in a report against another implementation, here github.com/hmarr/codeowners v1.2.1.
benchmarks/owners.py <checkout> benchmarks/results/<name>/report.json.gz codeowners
```

The use cases in [`use-cases/`](./use-cases/) are answered from the reports of `alternatives/run.sh`.

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
