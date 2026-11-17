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

Usages, props, parents and owners all come from one report of this tool, with `--follow props,calls`, which also follows variables and imports and describes calls, and `--wrappers`. The output, [`selects.json.gz`](../results/use-cases-kibana/selects.json.gz), lists the usages behind each pattern below, keyed by owner, with their props and parents.

## Results

Kibana at `6dce6e1bbcf1fcc46be7011088c3aa1e81feba19`; full tables in the [summary](../results/use-cases-kibana/selects.md).

| Component | Usages | Files | Teams | With `{...spread}` | Through wrappers |
|---|---|---|---|---|---|
| `EuiSelect` | 426 | 329 | 31 | 29 | 5 |
| `EuiSuperSelect` | 148 | 135 | 23 | 6 | 12 |
| `EuiComboBox` | 505 | 426 | 27 | 62 | 49 |
| `EuiSelectable` | 186 | 174 | 25 | 11 | 4 |

Usages through wrappers ([ADR-0026](../../docs/decisions/0026-wrappers.md)) are elements of 19 local components that spread their props, or a rest of them, into a select, such as `SingleFieldSelect` in maps (15) and `ConnectorsDropdownComponent` in cases (9), both read in the source. They have only the attributes their callers write, not the props the wrapper sets, such as `options`, so the counts below leave them out. They change the per-team totals in the summary: `@elastic/kibana-dashboards` has 200 usages with them, 174 without.

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

`options` is a literal array in 270 of 1,265 usages, with a median of 3 items and at most 14: 67 written inline and 203 in a variable, in the same file or imported, or passed by a caller. Without following ([results at `2b65387`](https://github.com/weronikaolejniczak/component-usage-cli/tree/2b65387/benchmarks/results/use-cases-kibana)), only the 67 could be read, and 797 were variables; following variables but not imports, 166 more were read, and 42 were imports. Through variables, imports and callers, most options end in a call (627: `useMemo` 316, `useState` 74), and 23 are arrays changed after they're declared, such as `availableAggs.push(...)`, reported `mutated`. What's left: calls written in the prop (189) and parameters (7), and 1 component that isn't rendered anywhere. Following imports but not props ([results at `9c5b591`](https://github.com/weronikaolejniczak/component-usage-cli/tree/9c5b591/benchmarks/results/use-cases-kibana)), 83 were parameters; following them, 26 more are a `one-of` of what each caller passes, 52 in all, and 2 more are literal arrays.

Describing calls, the calls are read but not evaluated, so the counts above stay the same. Of the 316 `useMemo` calls, the function passed returns an array literal in 60, another call in 200, such as `options.map(…)` or `.filter(…)`, a ternary or `||` in 30, and only names from the call's arguments, `in-call`, in 20. The other calls are `useState` (74), whose argument is the initial value, `.map` on a variable or a sequence, such as `makeSequence(0, 59).map(…)`, and local helpers such as `getTimeOptions`.

`singleSelection` on `EuiComboBox`, when set: `{ asPlainText: true }` 238, `true` 33, a ternary 8, a call 5, `{ asPlainText: false }` 2, an object with `asPlainText` from callers 2, which pass the result of a `useValues(…)` call, `unsupported` 3. Without following, 21 of the `{ asPlainText: true }` objects weren't read: 17 in variables and 4 imported.

Patterns:

| Pattern | Usages | Meaning |
|---|---|---|
| `EuiComboBox` with `singleSelection`, no `onCreateOption`, no `onSearchChange` | 203 | A searchable single select, which `EuiSuperSelect` and `EuiSelect` don't offer. |
| …of which `singleSelection={{ asPlainText: true }}` | 173 | A single select that looks like one. |
| `EuiSelect`, `EuiSuperSelect` or `EuiComboBox` with no `aria-label`, `aria-labelledby` or `id`, nearest EUI parent an `EuiFormRow` | 453 | Probably labelled by the row. 5 sampled were each the row's direct child. |
| The same, not in an `EuiFormRow` | 13 | Each is in an `EuiFlexItem`. Of 2 read, one is in an `EuiFlexGroup` inside the row, which labels only its direct child; the other has no row. |
| `EuiSelectable` inside a popover | 117 | A dropdown. |
| Props passed with `{...spread}` | 108 | 19 are the last step of usages through wrappers. The others spread something other than their component's props, or are in components that aren't rendered as a JSX tag, e.g. `es_ui_shared/.../select_field.tsx`, `field_input/input/select_input.tsx`. |
| Through wrappers | 70 | Elements of a local component that passes all its props on to a select. |

## What each tool gives

| Need | component-usage-cli | react-scanner 1.2.0 |
|---|---|---|
| Usages, imports resolved | Yes | Yes, by import name |
| Owners | Yes | No |
| Prop names | Yes | Yes |
| Prop values | Literals, objects, arrays, ternaries, with positions, through variables, imports and callers; `unresolved` with a code otherwise | Literals; kind only for expressions |
| Options' shape | When an array, inline, in a variable, imported or passed by a caller, 270 usages; when a call, what's called and what the functions passed return | No |
| Parents | Yes | No |
| Wrappers and their usages | With `--wrappers`, usages of components that spread their props into a select, with their callers' attributes | No; spreads only |

## What would answer it

Most `options` are built by a call, which the tool describes but doesn't evaluate: what `useMemo(() => items.map(…))` returns depends on `items`, a name in the call's arguments, which isn't followed. The props a wrapper sets itself, alongside the ones it passes on, so that usages through wrappers can be counted with the others.
