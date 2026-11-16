# Style overrides

Hypothetical. We want to know how Kibana overrides the styles of EUI components, and find patterns: which components, which properties, by which means and by which teams.

## What a complete answer needs

Overrides come in through:

1. `css`, `className` and `style` props on EUI components, with what they set.
2. Wrappers: `styled(EuiPanel)`.
3. Stylesheets and CSS-in-JS that select EUI's classes: `.euiDataGridRowCell { … }`, often with `!important`.

For patterns, the CSS properties and values matter as much as the components.

## Run

```sh
benchmarks/use-cases/run.sh <kibana> 6dce6e1bbcf1fcc46be7011088c3aa1e81feba19 /tmp/kibana
```

Props come from this tool's report, with `--follow variables`, owners from it and `codeowners`. CSS properties are object keys at any depth, and declarations in template text, such as `color:` in `` css`color: red` ``. Wrappers and selectors come from a text search: `styled(Eui…)` and `euiStyled(Eui…)` calls, and `.eui…` after a character that can start a selector, in source files and stylesheets. Selectors in tests, stories and mocks are counted apart. The output, [`styles.json.gz`](../results/use-cases-kibana/styles.json.gz), has `overrides` and `selectors`, keyed by owner.

## Results

Kibana at `6dce6e1bbcf1fcc46be7011088c3aa1e81feba19`; full tables in the [summary](../results/use-cases-kibana/styles.md).

7,291 of 87,239 EUI usages (8.4%) have `css`, `className` or `style`. What the values are:

| Prop | Usages | Read inline | Read through a variable | Not read |
|---|---|---|---|---|
| `css` | 4,844 | `` css`…` `` 1,585, object 1,117, ternary 107, array 101 | `` css`…` `` 516, object 61, ternary 9, array 1 | call 1,037, function 122, import 147, parameter 25 |
| `className` | 1,541 | literal 1,124, template 111, ternary 29 | template 68, literal 14, ternary 7 | call 98, parameter 49, import 29, unsupported 12 |
| `style` | 1,189 | object 1,072, ternary 23 | object 53, ternary 3 | call 16, parameter 13, import 5, mutated 4 |

Without `--follow variables` ([results at `2b65387`](https://github.com/weronikaolejniczak/component-usage-cli/tree/2b65387/benchmarks/results/use-cases-kibana)), the 1,543 `css`, 138 `className` and 76 `style` values in variables weren't read. Following them reads 587, 89 and 56; most of the other `css` variables hold a call (866): `useMemoCss` 192, `useStyles` 188, `useMemo` 133 and `css(…)` 127, among others; or a function (87).

The first version, with react-scanner, read only the 1,121 literal `className`s; it has 20 fewer usages, all behind a local module re-exporting EUI.

What `css` and `style` set, by usages:

| `css` | Usages | `style` | Usages |
|---|---|---|---|
| `height` | 348 | `width` | 214 |
| `width` | 344 | `minWidth` | 174 |
| `padding` | 253 | `maxWidth` | 140 |
| `display` | 253 | `height` | 127 |
| `overflow` | 223 | `minHeight` | 72 |
| `min-width` | 197 | `padding` | 47 |
| `font-weight` | 166 | `fontWeight` | 41 |
| `minWidth` | 158 | `marginRight` | 40 |

In `style`, the five most set are widths and heights; in `css`, `height` and `width` lead. Only values that are read are counted: following variables raised `height` in `css` from 275 to 348 and `width` from 274 to 344.

Most overridden, among components with at least 100 usages: `EuiModal` 30.4%, `EuiSplitPanel.Inner` 28.7%, `EuiBetaBadge` 27.9%, `EuiCard` 23.8%, `EuiPanel` 23.3%. In absolute numbers: `EuiFlexItem` 1,694, `EuiFlexGroup` 1,164, `EuiText` 823, `EuiPanel` 443.

The `className` tokens from EUI, in literals and template text, are its utility classes, such as `eui-textTruncate` (163) and `eui-textBreakWord` (104), not component internals.

| Means | Count |
|---|---|
| `styled(Eui…)` wrappers | 492, mostly `EuiFlexItem` (119) and `EuiFlexGroup` (79) |
| EUI class selectors outside tests | 1,507 in 676 files |
| `!important` in those files | 350 in 118 files |
| EUI class selectors in tests | 1,225 |

Most selected: `euiAccordion` 119, `euiDataGridRowCell` 93, `euiFlyoutBody` 83, `euiDataGrid` 70, `euiFormControlLayout` 60. 9 of 10 sampled selectors were overrides; one was in a comment.

## What each tool gives

| Need | component-usage-cli | react-scanner 1.2.0 | ripgrep |
|---|---|---|---|
| Which usages have `css`, `className`, `style` | Yes | Yes | No |
| What they set | When written inline or in a variable, with positions | `className` literals only | Text only |
| `styled()` wrappers | No | No | Text only |
| Selectors on EUI classes | No | No | Text only |
| Owners | Yes, for usages | No | No |

## What would answer it

Following imports, for 147 `css` values; reading the argument of `css(…)` calls, 127 more in variables; and components passed as values, so that `styled(EuiPanel)` is a usage of `EuiPanel` with its styles. Selectors in stylesheets are outside a JSX scan; a text search, as here, covers them.
