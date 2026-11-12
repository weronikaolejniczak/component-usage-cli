# component-usage-cli

CLI that statically analyzes a React codebase and reports, for selected components: every call site, the props passed and the set of values each prop can receive, including resolved values that come from variables, imports, maps or parent component props.

## Why

Literal props are easy to find with `grep`. Expressions aren't:

```tsx
<Icon type={icon} />
<Icon type={isOpen ? 'arrowDown' : 'arrowRight'} />
<Icon type={ICONS[status]} />
<MyIcon name="gear" /> // forwards `name` to <Icon type>
```

To know every value `type` can receive, you have to follow the data across files. `component-usage-cli` does that and reports anything it can't resolve along with the reason, instead silently skipping it.

## Usage (planned)

```sh
npx component-usage-cli scan ./src \
  --components EuiIcon --from @elastic/eui \
  --props type,iconType --follow all --wrappers --summary
```

## License

[MIT](./LICENSE)
