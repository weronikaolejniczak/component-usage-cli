# component-usage-cli

CLI that statically analyzes a React codebase and reports every usage of selected components. Resolving the values each prop can receive is planned.

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

- `--components`: exported names of the components to find.
- `--from`: the module they're exported from, resolved from the scanned folder. Imports from any path that resolves to the same export count, including re-exports.
- `--owners`: adds the owners of each file from the repository's `CODEOWNERS`. `--owners=<file>` uses a given file.

The JSON report lists:

- `usages`: every opening JSX element of the components.
- `unidentified`: elements that may be usages but aren't counted, with a `code` and a reason.
- `errors`: files that couldn't be read or parsed.
- `warnings`: skipped `CODEOWNERS` lines.

The exit code is 0 whenever a report is written.

### Planned

Prop values, `--props`, `--follow`, `--wrappers`, `--summary` and other output formats. See [decisions](./docs/decisions/README.md).

## License

[MIT](./LICENSE)
