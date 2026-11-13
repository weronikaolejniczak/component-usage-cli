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
benchmarks/alternatives/run.sh <kibana> 3 kibana-alternatives @elastic/eui
benchmarks/use-cases/analyse.py icons <kibana> \
  benchmarks/results/kibana-alternatives/react-scanner.json.gz \
  benchmarks/results/kibana-alternatives/report.json.gz out.json -- codeowners
```

`analyse.py icons` combines:

- react-scanner props: literal icon props on EUI components (`eui-jsx`), and icon props whose value is an expression (`review`). Icon props are `type` on `EuiIcon` and `iconType`, `icon`, `iconLeft` and `iconRight` on any component.
- A search for every quoted alias in source files, sorted by what precedes it: an icon attribute on another element (`other-jsx`), an object key containing `icon` (`icon-key`), anything else (`other-string`).
- Owners from this tool's report, and from `codeowners` for files without EUI usages.

The output, [`icons.json.gz`](../results/use-cases-kibana/icons.json.gz), has `findings` and `review`, each keyed by owner. A file with several owners is listed under each.

```json
{"file": "src/platform/packages/shared/kbn-coloring/src/shared_components/color_mapping/components/container/assignments.tsx", "line": 254, "column": 21, "component": "EuiButtonIcon", "prop": "iconType", "kind": "eui-jsx", "alias": "boxesVertical", "replacement": "ellipsis"}
```

## Results

Kibana at `6dce6e1bbcf1fcc46be7011088c3aa1e81feba19` ([summary](../results/use-cases-kibana/icons.md)):

| Kind | Findings | Reliable? |
|---|---|---|
| `eui-jsx` | 256 | Yes: an EUI component and a literal. |
| `other-jsx` | 52 | 12 sampled were all icons: wrappers such as `HeaderMenu` and `FlyoutTitle`, and test queries like `[data-euiicon-type="fold"]`. |
| `icon-key` | 140 | 12 sampled were all icons: `iconType: 'apps'` in item arrays. |
| `other-string` | 22,269 | No: `'string'`, `'index'`, `'search'`, `'list'`, `'alert'` and `'ip'` alone are 20,393 of them. |
| `review` | 1,421 | Unknown: `type={iconType}`, `iconType={isOpen ? 'fold' : 'unfold'}`, … |

### Checked against the hand-made list

[elastic/eui#9832](https://github.com/elastic/eui/issues/9832) lists 16 Kibana files, found by hand, that still use the persisted aliases. All 16 have findings, but for 13 of them only as `other-string`: the alias is stored as `ALERT: 'alert'` or `id: 'mapMarker'` in an icon set and reaches `iconType` through other code. So the precise kinds miss most of the hard cases, and the kind that has them can't be read by a person: 22,269 matches.

## What each tool gives

| Need | component-usage-cli | react-scanner 1.2.0 | ripgrep, ast-grep |
|---|---|---|---|
| Find EUI icon components, imports resolved | Yes | Yes, by import name | No import check |
| Literal prop values | No | Yes | Text only |
| Values from expressions | No | Kind only, e.g. `(Identifier)` | No |
| Wrappers, `styled()`, data passed to icon props | No | No | Text only |
| Owners | Yes, for usages | No | No |

## What would answer it

Resolving prop values: follow an icon prop's expression through constants, imports, object properties, conditionals and the props of the enclosing component, and report each value it can take, or why it can't. That would turn most of the 1,421 `review` entries and the icon sets behind the 13 hand-found files into findings with positions. It would also find wrappers, since a component forwarding its own prop to `iconType` is where the value chain leaves the file. Without it, an agent has the 448 precise findings plus 1,421 expressions and 22,269 strings to judge one by one.
