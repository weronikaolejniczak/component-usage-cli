# component-usage-cli

CLI that statically analyzes a React codebase and reports every usage of selected components, with its props as written, or followed through the variables of the file. Following imports and props is planned.

## Why

Literal props are easy to find with `grep`. Expressions aren't:

```tsx
<Icon type={icon} />
<Icon type={isOpen ? 'arrowDown' : 'arrowRight'} />
<Icon type={ICONS[status]} />
<MyIcon name="gear" /> // forwards `name` to <Icon type>
```

To know every value `type` can receive, you have to follow the data across files. The goal of `component-usage-cli` is to do that, and to report anything it can't resolve, with the reason, instead of silently skipping it.

## Usage

```sh
npx component-usage-cli scan ./src --components Icon,Icon.Item --from some-library
```

- `--components`: exported names of the components to find. Without it, every export of `--from` is a target.
- `--from`: the module they're exported from, resolved from the scanned folder. Imports from any path that resolves to the same export count, including re-exports.
- `--props`: names of the props to report. Without it, all are reported. Spreads are always reported, since they can set any prop.
- `--follow variables`: follows names declared in the file to their values, through destructuring and property access. Each followed value is at its definition, with `via`: the names followed to reach it, with their positions.
- `--owners`: adds the owners of each file from the repository's `CODEOWNERS`. `--owners=<file>` uses a given file.

The JSON report lists:

- `usages`: every opening JSX element of the components, with:
  - `props`: attributes in source order, as `{ "name", "value" }` or `{ "spread" }`. Each value has a `kind`, such as `literal`, `object`, `template` or `one-of`, and a position. Objects, arrays and templates contain values. Values that aren't resolved are `unresolved`, with a `code`, such as `not-followed` for a variable or import, or `mutated` for a followed variable that is changed in the file.
  - `parents`: the enclosing usages, nearest first. `prop` names the parent's attribute the usage is in, such as `button` in `<Popover button={<Button />}>`.
- `unidentified`: elements that may be usages but aren't counted, with a `code` and a reason.
- `errors`: files that couldn't be read or parsed.
- `warnings`: skipped `CODEOWNERS` lines.

The exit code is 0 whenever a report is written.

### Planned

`--follow` for imports and props, `--wrappers`, `--summary` and other output formats. See [decisions](./docs/decisions/README.md).

## License

[MIT](./LICENSE)
