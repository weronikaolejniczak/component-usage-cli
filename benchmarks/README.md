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

## Results

Kibana at `6dce6e1bbcf1fcc46be7011088c3aa1e81feba19`: 105,517 source files scanned. Apple M3 Pro, 11 cores, macOS 26.7.3, rustc 1.99.1, warm cache, 5 runs.

| Tool commit | Median | Min | Max |
|---|---|---|---|
| [`b348c32`](./results/kibana-b348c32/) | 26.56 s | 26.06 s | 27.73 s |
| [`55fb6a9`](./results/kibana-55fb6a9/) | 4.41 s | 4.00 s | 5.46 s |

Both commits produce byte-identical reports. `55fb6a9` caches export lookups and runs file discovery and `CODEOWNERS` matching in parallel; matching was most of the time before.

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

Owners weren't compared with what GitHub shows yet.

### What the tool got wrong or couldn't resolve

- 2,374 of 2,375 unidentified elements are `missing-export`: tags imported from CommonJS packages (`react`: 1,506, `@elastic/charts`: 845, others: 23), whose exports aren't read. None of them can be a target.
- 1 is `alias` (`EuiPopover` through a local constant).

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
