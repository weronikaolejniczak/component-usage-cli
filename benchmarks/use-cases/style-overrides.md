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
benchmarks/alternatives/run.sh <kibana> 3 kibana-alternatives @elastic/eui
benchmarks/use-cases/analyse.py styles <kibana> \
  benchmarks/results/kibana-alternatives/react-scanner.json.gz \
  benchmarks/results/kibana-alternatives/report.json.gz out.json -- codeowners
```

Props come from react-scanner, owners from this tool and `codeowners`. Wrappers and selectors come from a text search: `styled(Eui…)` and `euiStyled(Eui…)` calls, and `.eui…` after a character that can start a selector, in source files and stylesheets. Selectors in tests, stories and mocks are counted apart. The output, [`styles.json.gz`](../results/use-cases-kibana/styles.json.gz), has `overrides` and `selectors`, keyed by owner.

## Results

Kibana at `6dce6e1bbcf1fcc46be7011088c3aa1e81feba19`; full tables in the [summary](../results/use-cases-kibana/styles.md).

7,289 of 87,219 EUI usages (8.4%) have `css`, `className` or `style`:

| Prop | Usages | Values react-scanner can read |
|---|---|---|
| `css` | 4,844 | 0 |
| `className` | 1,541 | 1,121 |
| `style` | 1,187 | 0 |

`css` values are tagged templates (1,589), objects (1,117), identifiers (976), member expressions (735) and others; `style` values are almost all objects (1,069). Neither tool reads what's in them.

Most overridden, among components with at least 100 usages: `EuiModal` 30.4%, `EuiSplitPanel.Inner` 28.7%, `EuiBetaBadge` 27.9%, `EuiCard` 24.1%, `EuiPanel` 23.3%. In absolute numbers: `EuiFlexItem` 1,692, `EuiFlexGroup` 1,164, `EuiText` 823, `EuiPanel` 443.

The literal `className` tokens from EUI are its utility classes, such as `eui-textTruncate` (159) and `eui-textBreakWord` (103), not component internals.

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
| Which usages have `css`, `className`, `style` | No | Yes | No |
| What they set | No | `className` literals only | Text only |
| `styled()` wrappers | No | No | Text only |
| Selectors on EUI classes | No | No | Text only |
| Owners | Yes, for usages | No | No |

## What would answer it

Prop values, as in [icon aliases](./icon-aliases.md), extended to object literals and tagged templates, so that `css` and `style` report the properties they set; and components passed as values, so that `styled(EuiPanel)` is a usage of `EuiPanel` with its styles. Selectors in stylesheets are outside a JSX scan; a text search, as here, covers them.
