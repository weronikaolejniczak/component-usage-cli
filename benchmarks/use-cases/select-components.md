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
benchmarks/alternatives/run.sh <kibana> 3 kibana-alternatives @elastic/eui
benchmarks/use-cases/analyse.py selects <kibana> \
  benchmarks/results/kibana-alternatives/react-scanner.json.gz \
  benchmarks/results/kibana-alternatives/report.json.gz out.json
```

Props come from react-scanner, owners from this tool. The output, [`selects.json.gz`](../results/use-cases-kibana/selects.json.gz), lists the usages behind each pattern below, keyed by owner.

## Results

Kibana at `6dce6e1bbcf1fcc46be7011088c3aa1e81feba19`; full tables in the [summary](../results/use-cases-kibana/selects.md).

| Component | Usages | Files | Teams | With `{...spread}` |
|---|---|---|---|---|
| `EuiSelect` | 426 | 329 | 31 | 29 |
| `EuiSuperSelect` | 148 | 135 | 23 | 6 |
| `EuiComboBox` | 505 | 426 | 27 | 62 |
| `EuiSelectable` | 186 | 174 | 25 | 11 |

Features, by whether the prop is present. Props passed through a spread aren't seen.

| Feature | `EuiSelect` | `EuiSuperSelect` | `EuiComboBox` | `EuiSelectable` |
|---|---|---|---|---|
| Multiple selection | - | - | 44.8% | 70.4% |
| `searchable` | - | - | always | 47.3% |
| Async search | - | - | 16.8% | 9.1% |
| Create option | - | - | 27.1% | - |
| `renderOption` | - | - | 10.5% | 32.8% |
| `isLoading` | 6.8% | 14.2% | 30.1% | 15.1% |
| `isInvalid` | 11.5% | 10.1% | 35.8% | - |
| `compressed` | 31.2% | 27.0% | 23.8% | - |
| `fullWidth` | 43.7% | 60.8% | 64.2% | - |

Patterns:

| Pattern | Usages | Meaning |
|---|---|---|
| `EuiComboBox` with `singleSelection`, no `onCreateOption`, no `onSearchChange` | 199 | A searchable single select, which `EuiSuperSelect` and `EuiSelect` don't offer. |
| `EuiComboBox` with `singleSelection={{ asPlainText: true }}` | 159 | All 159 object values were read in the source. The same: a single select that looks like one. |
| `EuiSelect`, `EuiSuperSelect` or `EuiComboBox` with no `aria-label`, `aria-labelledby` or `id` | 466 | Candidates only: most are probably labelled by an `EuiFormRow`, which neither tool can see. |
| Props passed with `{...spread}` | 108 | Wrapper candidates, e.g. `es_ui_shared/.../select_field.tsx`, `field_input/input/select_input.tsx`. |

## What each tool gives

| Need | component-usage-cli | react-scanner 1.2.0 |
|---|---|---|
| Usages, imports resolved | Yes | Yes, by import name |
| Owners | Yes | No |
| Prop names | No | Yes |
| Prop values | No | Literals; kind only for expressions |
| Options' shape | No | No |
| Parents | No | No |
| Wrappers and their usages | No | No; spreads only |

## What would answer it

Props and values, as in [icon aliases](./icon-aliases.md), plus two things: the parent EUI element of each usage, for the popover and form row questions; and wrappers as targets, so that a local component rendering a select, with the props it forwards, is reported and its own usages counted like an EUI component's.
