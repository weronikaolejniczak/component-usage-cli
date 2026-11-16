# Removing icon aliases

Hypothetical. EUI removes the deprecated icon aliases listed in [elastic/eui#9847](https://github.com/elastic/eui/issues/9847) and [elastic/eui#9832](https://github.com/elastic/eui/issues/9832), 72 names in [`icon-aliases.json`](./icon-aliases.json). Kibana must not be left with a broken icon. We want every place an alias reaches an icon, for a GitHub issue, split by code owner, in a form an agent can work through.

## What a complete answer needs

An alias can reach an icon:

1. As a literal on an EUI component: `<EuiIcon type="help" />`, `<EuiButtonIcon iconType="boxesVertical" />`.
2. Through a wrapper: `<HeaderMenu iconType="boxesVertical" />`, `styled(EuiIcon)`, a component that forwards `iconType`.
3. In data that's rendered later: `{ name: 'Edit', icon: 'pencil' }` in context menu panels, tabs, table actions.
4. In a value computed elsewhere: a constant, a map, a ternary, a prop of the caller, a saved object.

Each finding needs file, position, alias, replacement and owners.

## Run

```sh
benchmarks/use-cases/run.sh <kibana> 6dce6e1bbcf1fcc46be7011088c3aa1e81feba19 /tmp/kibana
```

[`run.sh`](./run.sh) scans every export of `@elastic/eui` with owners and `--follow variables`, then `analyse.py icons` combines:

- Icon values in the props of EUI usages, at any depth: literals are findings, either on the prop (`eui-jsx`), in an object or array passed to a prop (`eui-data`), or reached through a variable (`eui-variable`), with the position of the value. Both branches of a ternary count. A value in a variable is listed once, however many props read it. Values the tool can't resolve are listed for `review` with their `unresolved` code, marked `followed` when the tool reached them through a variable. Icon props and keys are the ones EUI types as `IconType`: `icon`, `iconType`, `iconLeft`, `iconRight`, `childIconType`, `betaBadgeIconType`, `eventIcon`; `type` on `EuiIcon` and `EuiIconTip` and in an icon object; `logo` on `EuiLoadingLogo`.
- Aliases in other props of EUI usages, such as `role="alert"` (`eui-other-prop`), which aren't icons.
- A search for every other quoted alias in source files, sorted by what precedes it: an icon attribute on another element (`other-jsx`), an object key containing `icon` (`icon-key`), anything else (`other-string`).
- Owners from the report, and from `codeowners` for files without EUI usages.

The output, [`icons.json.gz`](../results/use-cases-kibana/icons.json.gz), has `findings` and `review`, each keyed by owner. A file with several owners is listed under each.

```json
{"file": "src/platform/packages/shared/kbn-coloring/src/shared_components/color_mapping/components/container/assignments.tsx", "line": 255, "column": 32, "component": "EuiButtonIcon", "prop": "iconType", "kind": "eui-jsx", "alias": "boxesVertical", "replacement": "ellipsis"}
```

## Results

Kibana at `6dce6e1bbcf1fcc46be7011088c3aa1e81feba19` ([summary](../results/use-cases-kibana/icons.md)):

| Kind | Findings | Reliable? |
|---|---|---|
| `eui-jsx` | 302 | Yes: an EUI component, an icon prop and a literal. |
| `eui-data` | 0 | Icon keys in objects passed to EUI props hold 59 literals, such as `listItems={[{ iconType: 'home' }]}`, none of them aliases. |
| `eui-variable` | 22 | Yes: 5 sampled were all aliases in a variable an icon prop reads, such as `const iconType = hasSingleColor ? 'stopFill' : 'paintBucket'` and `icon: 'indexClose'` in a status map. |
| `other-jsx` | 49 | 12 sampled from the first version's 52, which include these 49, were all icons: wrappers such as `HeaderMenu` and `FlyoutTitle`, and test queries like `[data-euiicon-type="fold"]`. |
| `icon-key` | 127 | 12 sampled from the first version's 140, which include these 127, were all icons: `iconType: 'apps'` in item arrays. |
| `eui-other-prop` | 109 | Not icons: `role="alert"`, `dataType: 'string'` in table columns and search schemas, `itemId="index"`. |
| `other-string` | 22,108 | No: `'string'`, `'index'`, `'search'`, `'list'`, `'alert'` and `'ip'` alone are 20,283 of them. |
| `review` | 771 values | Not resolved: `parameter` 318 and 34 followed, `not-followed` through imports 119 and 60 followed, `call` 74 and 114 followed, `mutated` 28, `unsupported` 3 and 11 followed, `template` 9 and 1 followed. |

Compared with the same scan without `--follow variables` ([results at `2b65387`](https://github.com/weronikaolejniczak/component-usage-cli/tree/2b65387/benchmarks/results/use-cases-kibana)), the total is the same, 22,717, but 80 strings from the text search are now values the report places in a prop. 22 are `eui-variable` findings: 12 were `icon-key`, 10 were `other-string`. 58 were `other-string` and are now `eui-other-prop`, which are not icons. The 279 values `not-followed` through variables are gone from review: the ones that weren't literals end in a call, an import, a parameter or a mutated variable, and review shrank from 802 to 771.

Compared with react-scanner's props, used in the first version of this use case: every one of its 256 `eui-jsx` findings is found. Of the 46 more, 39 are branches of ternaries, such as `iconType={isAllExpanded ? 'fold' : 'unfold'}`, which react-scanner reports as `(ConditionalExpression)` (8 read in the source), and 7 are `EuiIconTip` types. Its 1,421 `review` entries were whole expressions; these are the values left after expanding them, each with what would resolve it.

### Checked against the hand-made list

[elastic/eui#9832](https://github.com/elastic/eui/issues/9832) lists 16 Kibana files, found by hand, that still use the persisted aliases. All 16 have findings, but 13 of them only as `other-string`: the alias is stored as `ALERT: 'alert'` or `id: 'mapMarker'` in an icon set and reaches `iconType` through other code. Following variables doesn't change this: the icon sets are in other files, so the values that lead there are imports.

## What each tool gives

| Need | component-usage-cli | react-scanner 1.2.0 | ripgrep, ast-grep |
|---|---|---|---|
| Find EUI icon components, imports resolved | Yes | Yes, by import name | No import check |
| Literal prop values, with positions | Yes | Values without positions | Text only |
| Ternaries and data in props | Each value, at any depth | Kind only, e.g. `(ConditionalExpression)` | No |
| Values from variables | Followed, with where they're defined | Kind only, e.g. `(Identifier)` | No |
| Values from imports, the caller's props | `unresolved`, with the step that would follow it | Kind only | No |
| Wrappers, `styled()` | No | No | Text only |
| Owners | Yes, for usages | No | No |

## What would answer it

Following imports: the 179 `not-followed` values and the icon sets behind the 13 hand-found files. Following the props of the enclosing component: the 352 `parameter` values, which is also where wrappers such as `HeaderMenu` are found. Until then, an agent has 500 precise findings and 771 values with a reason, instead of 22,108 strings.
