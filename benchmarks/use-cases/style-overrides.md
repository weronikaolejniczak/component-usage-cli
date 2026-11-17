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

Props come from this tool's report, with `--follow props,calls`, which also follows variables and imports and describes calls, and `--wrappers`, owners from it and `codeowners`. CSS properties are object keys at any depth, and declarations in template text, such as `color:` in `` css`color: red` ``. Wrappers and selectors come from a text search: `styled(Eui…)` and `euiStyled(Eui…)` calls, and `.eui…` after a character that can start a selector, in source files and stylesheets. Selectors in tests, stories and mocks are counted apart. The output, [`styles.json.gz`](../results/use-cases-kibana/styles.json.gz), has `overrides` and `selectors`, keyed by owner.

## Results

Kibana at `6dce6e1bbcf1fcc46be7011088c3aa1e81feba19`; full tables in the [summary](../results/use-cases-kibana/styles.md).

7,291 of 87,239 EUI usages (8.4%) have `css`, `className` or `style`. What the values are:

| Prop | Usages | Read inline | Read through a variable, an import or a caller | Not read |
|---|---|---|---|---|
| `css` | 4,844 | `` css`…` `` 1,585, object 1,117, ternary 107, array 101 | `` css`…` `` 548, object 93, one-of 24, array 1 | call 1,076, function 172, parameter 4 |
| `className` | 1,541 | literal 1,124, template 111, ternary 29 | template 78, literal 27, one-of 34 | call 105, parameter 17, unsupported 12, no callers 3 |
| `style` | 1,189 | object 1,072, ternary 23 | object 58, one-of 14 | call 16, mutated 4, parameter 1, no callers 1 |

Without `--follow variables` ([results at `2b65387`](https://github.com/weronikaolejniczak/component-usage-cli/tree/2b65387/benchmarks/results/use-cases-kibana)), the 1,543 `css`, 138 `className` and 76 `style` values in variables weren't read. Following them reads 587, 89 and 56; most of the other `css` variables hold a call (866): `useMemoCss` 192, `useStyles` 188, `useMemo` 133 and `css(…)` 127, among others; or a function (87).

Following imports too reads 64 of the 147 imported `css` values, 23 of the 29 `className`s and all 5 `style`s. The other imported `css` values hold a function (50) or a call (33), all `css(…)`, which now hold 160 values.

Following props too, `parameter` values go from 25 to 4 for `css`, from 49 to 17 for `className` and from 13 to 1 for `style`, and the followed `one-of` values from 9, 7 and 3 to 24, 34 and 14: each value the callers pass is a branch. Of their branches, 83 of 117 for `css`, 170 of 221 for `className` and 50 of 89 for `style` are `undefined`: a caller that doesn't set the prop, or a ternary branch that sets nothing.

Describing calls too, the calls are read: 1,076 for `css`, 105 for `className` and 16 for `style`. For `css`, they're `useMemoCss` 194, `useStyles` 188, `css(…)` 232, of which 72 are written in the prop, and `useMemo` 134. 223 of the `css(…)` calls are given an object, which sets `minWidth` in 42, `height` in 39 and `width` in 26. For `className`, they're `classNames`, `cx` and `classnames` (62 in all), given literals, objects such as `{ 'cm-favorite-button--active': … }` in `favorite_button.tsx`, and names from their scope, `in-call` (34 arguments). The tables below count only what's set outside calls, since a function's arguments aren't necessarily CSS.

The first version, with react-scanner, read only the 1,121 literal `className`s; it has 20 fewer usages, all behind a local module re-exporting EUI.

What `css` and `style` set, by usages:

| `css` | Usages | `style` | Usages |
|---|---|---|---|
| `width` | 365 | `width` | 214 |
| `height` | 351 | `minWidth` | 174 |
| `display` | 265 | `maxWidth` | 140 |
| `padding` | 258 | `height` | 130 |
| `overflow` | 245 | `minHeight` | 73 |
| `min-width` | 214 | `padding` | 48 |
| `minWidth` | 169 | `fontWeight` | 43 |
| `font-weight` | 166 | `marginRight` | 40 |

In `style`, the five most set are widths and heights; in `css`, `width` and `height` lead. Only values that are read are counted: following variables raised `width` in `css` from 274 to 344 and `height` from 275 to 348, and following imports to 364 and 350, and following props to 365 and 351.

Most overridden, among components with at least 100 usages: `EuiModal` 30.4%, `EuiSplitPanel.Inner` 28.7%, `EuiBetaBadge` 27.9%, `EuiCard` 23.8%, `EuiPanel` 23.3%. In absolute numbers: `EuiFlexItem` 1,694, `EuiFlexGroup` 1,164, `EuiText` 823, `EuiPanel` 443.

The `className` tokens from EUI, in literals and template text, are its utility classes, such as `eui-textTruncate` (163) and `eui-textBreakWord` (104), not component internals.

| Means | Count |
|---|---|
| `styled(Eui…)` wrappers | 492, mostly `EuiFlexItem` (119) and `EuiFlexGroup` (79) |
| EUI class selectors outside tests | 1,507 in 676 files |
| `!important` in those files | 350 in 118 files |
| EUI class selectors in tests | 1,225 |

With `--wrappers` ([ADR-0026](../../docs/decisions/0026-wrappers.md)), 3,351 more usages go through local components that spread their props into an EUI component or pass it to a call. They have only the attributes their callers write, not what the wrapper sets, so the numbers above leave them out. 736 go through a `styled(…)` or `euiStyled(…)` call: `EuiFlexItem` 180, `EuiFlexGroup` 96, `EuiText` 58, `EuiPanel` 55. Those usages get the wrapper's styles, which are in the call, not in their props. 150 of the 3,351 have `css`, `className` or `style` written by their callers. The most, `EuiCallOut` 942, go through the four callouts of `@kbn/ui-callout`, such as `KbnWarningCallout`, which renders `<EuiCallOut color="warning" {...props} />`, read in the source.

Most selected: `euiAccordion` 119, `euiDataGridRowCell` 93, `euiFlyoutBody` 83, `euiDataGrid` 70, `euiFormControlLayout` 60. 9 of 10 sampled selectors were overrides; one was in a comment.

## What each tool gives

| Need | component-usage-cli | react-scanner 1.2.0 | ripgrep |
|---|---|---|---|
| Which usages have `css`, `className`, `style` | Yes | Yes | No |
| What they set | When written inline, in a variable, imported or passed by a caller, with positions; the arguments of the calls that build them | `className` literals only | Text only |
| `styled()` wrappers | With `--wrappers`, their usages, through the call | No | Text only |
| Selectors on EUI classes | No | No | Text only |
| Owners | Yes, for usages | No | No |

## What would answer it

What hooks such as `useMemoCss` and `useStyles` return, which is in the function they're given or in another file: names in a call's arguments aren't followed. The styles a `styled(EuiPanel)` call adds, which are in its template rather than in the usages' props. Selectors in stylesheets are outside a JSX scan; a text search, as here, covers them.
