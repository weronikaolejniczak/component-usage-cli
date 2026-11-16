# Select components

Hypothetical. EUI wants one select component. To scope it, we want to know how `EuiSelect`, `EuiSuperSelect`, `EuiComboBox` and `EuiSelectable` are used in Kibana: counts and owners, props and their combinations, which features are used, wrappers and anti-patterns.

## What a complete answer needs

- Usages, files and owners per component.
- Props per usage, with their values, to tell single from multiple selection, sync from async search and so on.
- What the options look like: grouped, with custom rendering, how many.
- Parents: an `EuiSelectable` in an `EuiPopover` is a dropdown; a select in an `EuiFormRow` gets its label from it.
- Wrappers: local components that render a select, and their own usages.

## Run

```sh
benchmarks/use-cases/run.sh <kibana> 6dce6e1bbcf1fcc46be7011088c3aa1e81feba19 /tmp/kibana
```

Usages, props, parents and owners all come from one report of this tool. The output, [`selects.json.gz`](../results/use-cases-kibana/selects.json.gz), lists the usages behind each pattern below, keyed by owner, with their props and parents.

## Results

Kibana at `6dce6e1bbcf1fcc46be7011088c3aa1e81feba19`; full tables in the [summary](../results/use-cases-kibana/selects.md).

| Component | Usages | Files | Teams | With `{...spread}` |
|---|---|---|---|---|
| `EuiSelect` | 426 | 329 | 31 | 29 |
| `EuiSuperSelect` | 148 | 135 | 23 | 6 |
| `EuiComboBox` | 505 | 426 | 27 | 62 |
| `EuiSelectable` | 186 | 174 | 25 | 11 |

Features, by whether the prop is present, and for multiple selection and clearing, by its literal value. Props passed through a spread aren't seen.

| Feature | `EuiSelect` | `EuiSuperSelect` | `EuiComboBox` | `EuiSelectable` |
|---|---|---|---|---|
| Multiple selection | - | - | 42.4% | 57.0% |
| `searchable` | - | - | always | 47.3% |
| Async search | - | - | 16.8% | 9.1% |
| Create option | - | - | 27.1% | - |
| `renderOption` | - | - | 10.5% | 32.8% |
| `isLoading` | 6.8% | 14.2% | 30.1% | 15.1% |
| `isInvalid` | 11.5% | 10.1% | 35.8% | - |
| `compressed` | 31.2% | 27.0% | 23.8% | - |
| `fullWidth` | 43.7% | 60.8% | 64.2% | - |

The first version of this use case, with react-scanner's props, had 44.8% and 70.4% for multiple selection: react-scanner reports `singleSelection` without a value as `null`, the same as an absent prop, so 37 single selects were counted as multiple. Here it's `true`.

`options` is a literal array in 67 of 1,265 usages, with a median of 2 items. In most others it's a variable (797), a call (189), a parameter (78) or an import (41), which aren't followed yet, so the shape of options can't be read.

`singleSelection` on `EuiComboBox`, when set: `{ asPlainText: true }` 217, `true` 33, a variable or import 26, a ternary 8, `{ asPlainText: false }` 2, an object with `asPlainText` from a parameter 2, other 3.

Patterns:

| Pattern | Usages | Meaning |
|---|---|---|
| `EuiComboBox` with `singleSelection`, no `onCreateOption`, no `onSearchChange` | 203 | A searchable single select, which `EuiSuperSelect` and `EuiSelect` don't offer. |
| …of which `singleSelection={{ asPlainText: true }}` | 159 | A single select that looks like one. |
| `EuiSelect`, `EuiSuperSelect` or `EuiComboBox` with no `aria-label`, `aria-labelledby` or `id`, nearest EUI parent an `EuiFormRow` | 453 | Probably labelled by the row. 5 sampled were each the row's direct child. |
| The same, not in an `EuiFormRow` | 13 | Each is in an `EuiFlexItem`. Of 2 read, one is in an `EuiFlexGroup` inside the row, which labels only its direct child; the other has no row. |
| `EuiSelectable` inside a popover | 117 | A dropdown. |
| Props passed with `{...spread}` | 108 | Wrapper candidates, e.g. `es_ui_shared/.../select_field.tsx`, `field_input/input/select_input.tsx`. |

## What each tool gives

| Need | component-usage-cli | react-scanner 1.2.0 |
|---|---|---|
| Usages, imports resolved | Yes | Yes, by import name |
| Owners | Yes | No |
| Prop names | Yes | Yes |
| Prop values | Literals, objects, arrays, ternaries, with positions; `unresolved` with a code otherwise | Literals; kind only for expressions |
| Options' shape | When written inline; 67 usages | No |
| Parents | Yes | No |
| Wrappers and their usages | No; spreads only | No; spreads only |

## What would answer it

Following variables, calls aside, so that the 838 `options` values held in variables and imports can be read. Wrappers as targets, so that a local component rendering a select, with the props it forwards, is reported and its own usages counted like an EUI component's.
