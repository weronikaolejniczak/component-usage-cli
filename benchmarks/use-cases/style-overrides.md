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

Props come from this tool's report, owners from it and `codeowners`. CSS properties are object keys at any depth, and declarations in template text, such as `color:` in `` css`color: red` ``. Wrappers and selectors come from a text search: `styled(Eui…)` and `euiStyled(Eui…)` calls, and `.eui…` after a character that can start a selector, in source files and stylesheets. Selectors in tests, stories and mocks are counted apart. The output, [`styles.json.gz`](../results/use-cases-kibana/styles.json.gz), has `overrides` and `selectors`, keyed by owner.

## Results

Kibana at `6dce6e1bbcf1fcc46be7011088c3aa1e81feba19`; full tables in the [summary](../results/use-cases-kibana/styles.md).

7,291 of 87,239 EUI usages (8.4%) have `css`, `className` or `style`. What the values are:

| Prop | Usages | Inline: read | Not followed yet |
|---|---|---|---|
| `css` | 4,844 | `` css`…` `` 1,585, object 1,117, array 101, ternary 107 | variable 1,543, import 145, call 171, parameter 24 |
| `className` | 1,541 | literal 1,124, template 111, ternary 29 | variable 138, import 28, call 51, parameter 48 |
| `style` | 1,189 | object 1,072, ternary 23 | variable 76, import 5, parameter 13 |

The first version, with react-scanner, read only the 1,121 literal `className`s; it has 20 fewer usages, all behind a local module re-exporting EUI.

What `css` and `style` set, by usages:

| `css` | Usages | `style` | Usages |
|---|---|---|---|
| `height` | 275 | `width` | 196 |
| `width` | 274 | `minWidth` | 159 |
| `padding` | 187 | `maxWidth` | 129 |
| `display` | 183 | `height` | 121 |
| `overflow` | 165 | `minHeight` | 72 |
| `minWidth` | 156 | `padding` | 44 |
| `font-weight` | 136 | `marginRight` | 39 |
| `min-width` | 133 | `fontWeight` | 37 |

In `style`, the five most set are widths and heights; in `css`, `height` and `width` lead. Only what's written inline is counted, not the 1,543 `css` values held in variables.

Most overridden, among components with at least 100 usages: `EuiModal` 30.4%, `EuiSplitPanel.Inner` 28.7%, `EuiBetaBadge` 27.9%, `EuiCard` 23.8%, `EuiPanel` 23.3%. In absolute numbers: `EuiFlexItem` 1,694, `EuiFlexGroup` 1,164, `EuiText` 823, `EuiPanel` 443.

The `className` tokens from EUI, in literals and template text, are its utility classes, such as `eui-textTruncate` (161) and `eui-textBreakWord` (104), not component internals.

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
| What they set | When written inline, with positions | `className` literals only | Text only |
| `styled()` wrappers | No | No | Text only |
| Selectors on EUI classes | No | No | Text only |
| Owners | Yes, for usages | No | No |

## What would answer it

Following variables and imports, for the 1,688 `css` values held in them; and components passed as values, so that `styled(EuiPanel)` is a usage of `EuiPanel` with its styles. Selectors in stylesheets are outside a JSX scan; a text search, as here, covers them.
