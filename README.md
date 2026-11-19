# component-usage-cli

CLI that statically analyzes a React codebase and reports every usage of selected components, with its props as written, or followed through variables, imports and the props of the components they're in, with calls described.

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
- `--follow imports`: also follows imported names to the exported value, through re-exports and namespace imports. A value or name in another file than the usage has a `file`.
- `--follow props`: also follows the props of the component a usage is in to what each JSX caller of that component passes, in any file, through chains of components and `{...props}` spreads. The value is a `one-of` with a value per caller, whose `via` starts with the caller's tag. A component also used other than as a JSX tag, such as `styled(Wrapper)`, adds an `other-callers` value.
- `--follow calls`: describes each call instead of leaving it `unresolved`, without evaluating it: a `call` value with its `callee` as written, the `receiver` of a member call such as `items.map(…)`, its `arguments`, and the `path` read from its result. A function written as an argument has `returns`, the values of its `return` statements. Names in the arguments aren't followed: they're `unresolved` with code `in-call`. Combine it with the others, as in `--follow props,calls`.
- `--wrappers`: also reports the elements of components that pass all their props on to a target as usages of the target: components that render it with `{...props}` or `{...rest}`, and `const`s or default exports holding a call with it as first argument, such as `wrap(Target)`. Each has `through`, the wrappers from the element to the target, with the position where each passes the props on. Its props are those the target is given, in the order they apply: a wrapper's own attributes, like `size` in `<Target size="s" {...props} />`, go around the element's, and props a rest pattern takes, like `size` in `({ size, ...rest })`, and `key`, which React keeps, are left out. With `--follow props`, a wrapper's own props are followed to the element the usage goes through only ([ADR-0031](./docs/decisions/0031-props-wrappers-set.md)).
- `--owners`: adds the owners of each file from the repository's `CODEOWNERS`. `--owners=<file>` uses a given file.
- `--summary`: reports counts instead of every usage: per component, its usages, files, spreads, references and usages through wrappers; per prop, how many usages set it and the literal values they give it, applying props in order as React does, so a spread that may set any prop hides the values set before it, except a wrapper's spread of its props, which are counted on the usages through it; and with `--owners`, usages per owner. Unidentified elements are counted per `code`; errors and warnings are listed in full.

The JSON report lists:

- `usages`: every opening JSX element of the components, including through local names that are never assigned again, as in `const I = Icon; <I />`, with:
  - `props`: attributes in source order, as `{ "name", "value" }` or `{ "spread" }`. Each value has a `kind`, such as `literal`, `object`, `template`, `one-of` or `operation`, and a position. Objects, arrays, templates and operations, such as `!isOpen`, contain values. Values that aren't resolved are `unresolved`, with a `code`, such as `not-followed` for a variable or import, `mutated` for a followed variable that is changed in place, or `missing-export` for an import the module doesn't export.
  - `through`: with `--wrappers`, for a usage through wrappers.
  - `parents`: the enclosing usages, nearest first. `prop` names the parent's attribute the usage is in, such as `button` in `<Popover button={<Button />}>`.
- `references`: places the components are read as values rather than rendered, such as `<Panel icon={Icon} />` or `{ icon: Icon }`, with `in`: where the value goes, an `attribute`, `argument`, `property`, `item`, `variable` or `other`.
- `unidentified`: elements that may be usages but aren't counted, with a `code` and a reason, such as `alias` for a local name that's assigned again or `commonjs` for `require`.
- `errors`: files that couldn't be read or parsed.
- `warnings`: skipped `CODEOWNERS` lines.

The exit code is 0 whenever a report is written.

### Planned

Other output formats. See [decisions](./docs/decisions/README.md).

## License

[MIT](./LICENSE)
