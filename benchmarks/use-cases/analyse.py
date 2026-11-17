#!/usr/bin/env python3
"""Answer the use cases in this folder from a component-usage-cli report.

Usage:
  analyse.py icons   <checkout> <report.json> <out.json> -- <codeowners command...>
  analyse.py selects <checkout> <report.json> <out.json>
  analyse.py styles  <checkout> <report.json> <out.json> -- <codeowners command...>

<report.json> is a `scan --from <library> --owners --follow props,calls --wrappers` report,
without `--components`, so every export of the library is a target. Prints a summary
and writes the full findings, grouped by owner, to <out.json>.
"""

import collections
import gzip
import json
import os
import re
import statistics
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
SOURCES = ["*.ts", "*.tsx", "*.js", "*.jsx", "*.mjs", "*.cjs"]


def load(path):
    opener = gzip.open if path.endswith(".gz") else open
    with opener(path, "rt") as f:
        return json.load(f)


def named(usage):
    """Named props of a usage; a later attribute overrides an earlier one, as in React."""
    return {prop["name"]: prop["value"] for prop in usage["props"] if "name" in prop}


def spread(usage):
    return any("spread" in prop for prop in usage["props"])


CALL = ("(receiver)", "(argument)", "(returns)")


def walk(value, path=(), steps=()):
    """Every value inside `value`, with its path: object keys, `[]` for array items,
    `...` for spreads, `${}` for template parts, and `(receiver)`, `(argument)` and `(returns)`
    for a call's receiver, its arguments and what functions among them return; and the `via`
    steps followed to reach it (`--follow`), when it's written at a definition or a caller
    rather than at the usage. JSX elements are usages of their own."""
    steps = steps + tuple(value.get("via", ()))
    yield path, value, steps
    kind = value["kind"]
    if kind == "one-of":
        for inner in value["values"]:
            yield from walk(inner, path, steps)
    elif kind == "object":
        for entry in value["entries"]:
            if "spread" in entry:
                yield from walk(entry["spread"], path + ("...",), steps)
            else:
                yield from walk(entry["value"], path + (entry.get("key", f"[{entry.get('computed')}]"),), steps)
    elif kind == "array":
        for item in value["items"]:
            yield from walk(item.get("value") or item["spread"], path + ("[]" if "value" in item else "...",), steps)
    elif kind == "template":
        for part in value["parts"]:
            if "value" in part:
                yield from walk(part["value"], path + ("${}",), steps)
    elif kind == "call":
        if "receiver" in value:
            yield from walk(value["receiver"], path + ("(receiver)",), steps)
        for item in value["arguments"]:
            yield from walk(item.get("value") or item["spread"], path + ("(argument)",), steps)
    elif kind == "function":
        for returned in value.get("returns", ()):
            yield from walk(returned, path + ("(returns)",), steps)


def in_call(path):
    return any(key in CALL for key in path)


def string(value):
    return value["kind"] == "literal" and isinstance(value["value"], str)


def reason(value):
    """Why a value isn't a literal: an unresolved code, with the step that would follow it."""
    if value["kind"] == "unresolved":
        return value["code"] + (f" ({value['step']})" if "step" in value else "")
    return value["kind"]


def at(usage, value):
    """Where `value` is: in the usage's file, or in the file it was followed to."""
    return {"file": value.get("file", usage["file"]), "line": value["line"], "column": value["column"]}


def rg(checkout, pattern, globs):
    args = ["rg", "--line-number", "--column", "--no-heading", "--only-matching", "--hidden"]
    args += ["--glob", "!.git", "--glob", "!**/node_modules/**"]
    for glob in globs:
        args += ["--glob", glob]
    out = subprocess.run(args + [pattern], cwd=checkout, capture_output=True, text=True).stdout
    rows = []
    for row in out.splitlines():
        file, line, column, match = row.split(":", 3)
        rows.append((file.removeprefix("./"), int(line), int(column), match))
    return sorted(rows)


def lines(checkout, file):
    with open(os.path.join(checkout, file), encoding="utf-8", errors="replace") as f:
        return f.read().splitlines()


def owners_from_report(report):
    return {usage["file"]: usage["owners"] for usage in report["usages"]}


def owners_from_command(checkout, command, files):
    owners = {}
    files = sorted(files)
    for start in range(0, len(files), 500):
        out = subprocess.run(
            command + files[start : start + 500], cwd=checkout, capture_output=True, text=True, check=True
        ).stdout
        for line in out.splitlines():
            file, *rest = line.split()
            owners[file] = [] if rest == ["(unowned)"] else rest
    return owners


def by_owner(entries, owners):
    grouped = collections.defaultdict(list)
    for entry in entries:
        for owner in owners.get(entry["file"]) or ["(unowned)"]:
            grouped[owner].append(entry)
    return dict(sorted(grouped.items(), key=lambda item: (-len(item[1]), item[0])))


def percent(part, whole):
    return f"{100 * part / whole:.1f}%" if whole else "-"


def table(header, rows):
    print("| " + " | ".join(header) + " |")
    print("|" + "---|" * len(header))
    for row in rows:
        print("| " + " | ".join(str(cell) for cell in row) + " |")
    print()


def write(path, data):
    with open(path, "w") as f:
        json.dump(data, f, indent=1)


# Props and keys typed `IconType` in EUI: `icon`, `iconType`, `iconLeft`, `iconRight`,
# `childIconType`, `betaBadgeIconType`, `eventIcon`; `type` on `EuiIcon` and `EuiIconTip`, and
# in an icon object such as `icon={{ type: 'gear', side: 'left' }}`; `logo` on `EuiLoadingLogo`.
ICON_KEYS = {"icon", "iconType", "iconLeft", "iconRight", "childIconType", "betaBadgeIconType", "eventIcon"}
TYPE_IS_ICON = {"EuiIcon", "EuiIconTip"}


def icon_value(component, name, path):
    if path and path[-1] in ("[]", "...", "${}"):
        return False
    keys = (name,) + tuple(key for key in path if key not in ("[]", "..."))
    if keys[-1] in ICON_KEYS:
        return True
    if keys[-1] == "type":
        return len(keys) == 1 and component in TYPE_IS_ICON or len(keys) > 1 and keys[-2] in ICON_KEYS
    return keys == ("logo",) and component == "EuiLoadingLogo"


def icon_attribute(name, before):
    if "icon" in name.lower():
        return True
    tags = re.findall(r"<([\w.]+)", before)
    return name == "type" and bool(tags) and "Icon" in tags[-1]


# Kinds of findings, most precise first. `eui-variable` is a literal in a variable that an icon
# prop of an EUI usage reads, in the same file or another, found with `--follow imports`.
# `eui-caller` is a literal that a caller of the component the usage is in passes, found with
# `--follow props`: a `via` step is the caller's tag. `eui-call` is a literal inside a call, found
# with `--follow calls`, in its receiver or arguments or returned by a function among them, where
# the call is an icon prop's value or the literal is at an icon key; the call decides what's used.
ORDER = ["eui-jsx", "eui-data", "eui-variable", "eui-caller", "eui-call", "other-jsx", "icon-key", "eui-other-prop", "other-string"]
PRECISE = ORDER[: ORDER.index("icon-key") + 1]


def through_caller(checkout, usage, steps, cache):
    for step in steps:
        file = step.get("file", usage["file"])
        text = cache.setdefault(file, lines(checkout, file))[step["line"] - 1]
        if text.encode("utf-16-le")[2 * (step["column"] - 1) : 2 * step["column"]].decode("utf-16-le") == "<":
            return True
    return False


def icons(checkout, report, out, command):
    aliases = load(os.path.join(HERE, "icon-aliases.json"))["aliases"]
    # A value in a variable is one place to change, however many usages read it: findings and
    # values to review are kept once per position, with the most precise kind.
    found, review, cache = {}, {}, {}
    for usage in report["usages"]:
        base = {"component": usage["component"]}
        for name, value in named(usage).items():
            for path, node, steps in walk(value):
                followed = bool(steps)
                if node["kind"] == "one-of":
                    continue
                outside = path[: next((i for i, key in enumerate(path) if key in CALL), len(path))]
                icon = icon_value(usage["component"], name, path)
                called = in_call(path) and (icon or icon_value(usage["component"], name, outside))
                entry = base | {"prop": ".".join((name,) + path)}
                position = tuple(at(usage, node).values())
                if string(node):
                    found.setdefault(position, None)
                    if node["value"] in aliases:
                        kind = ("eui-call" if called
                                else "eui-other-prop" if not icon
                                else "eui-caller" if followed and through_caller(checkout, usage, steps, cache)
                                else "eui-variable" if followed else "eui-data" if path else "eui-jsx")
                        current = found[position]
                        if current is None or ORDER.index(kind) < ORDER.index(current["kind"]):
                            found[position] = at(usage, node) | entry | {
                                "kind": kind, "alias": node["value"], "replacement": aliases[node["value"]]}
                elif icon and node["kind"] not in ("element", "function", "object", "array", "literal", "undefined"):
                    review.setdefault(position, at(usage, node) | entry | {
                        "reason": reason(node), "text": node.get("text"), "followed": followed})
    seen = set(found)
    found = [entry for entry in found.values() if entry is not None]
    review = list(review.values())

    names = "|".join(sorted(aliases, key=len, reverse=True))
    attribute = re.compile(r"\b([\w-]+)=\{?\s*$")
    icon_key = re.compile(r"\w*icon\w*['\"]?\s*:\s*$", re.IGNORECASE)
    for file, line, column, match in rg(checkout, rf"['\"`]({names})['\"`]", SOURCES):
        alias = match[1:-1]
        source = cache.setdefault(file, lines(checkout, file))
        text = source[line - 1]
        before = text.encode()[: column - 1].decode(errors="replace")
        if (file, line, len(before.encode("utf-16-le")) // 2 + 1) in seen:
            continue
        jsx = attribute.search(before)
        if jsx and icon_attribute(jsx.group(1), "\n".join(source[max(0, line - 10) : line - 1] + [before])):
            kind = "other-jsx"
        elif icon_key.search(before):
            kind = "icon-key"
        else:
            kind = "other-string"
        found.append(
            {"file": file, "line": line, "column": column, "kind": kind, "alias": alias,
             "replacement": aliases[alias], "text": text.strip()[:200]}
        )

    owners = owners_from_report(report)
    missing = {entry["file"] for entry in found + review} - owners.keys()
    owners |= owners_from_command(checkout, command, missing)

    order = ORDER
    kinds = collections.Counter(entry["kind"] for entry in found)
    print(f"aliases: {len(aliases)}; findings: {len(found)}; icon values to review: {len(review)}\n")
    table(["kind", "findings"], [(kind, kinds[kind]) for kind in order])
    table(["review reason", "values"], collections.Counter(
        entry["reason"] + (", followed" if entry["followed"] else "") for entry in review).most_common())
    per_alias = collections.defaultdict(collections.Counter)
    for entry in found:
        per_alias[entry["alias"]][entry["kind"]] += 1
    table(["alias"] + order, sorted(
        ([alias] + [counts[kind] for kind in order] for alias, counts in per_alias.items()),
        key=lambda row: (-sum(row[1 : len(PRECISE) + 1]), -row[-1], row[0]),
    ))
    precise = [entry for entry in found if entry["kind"] in PRECISE]
    grouped = by_owner(precise, owners)
    table(["owner", ", ".join(PRECISE) + " findings"],
          [(o, len(e)) for o, e in list(grouped.items())[:15]])
    write(out, {"findings": by_owner(found, owners), "review": by_owner(review, owners)})


SELECTS = ["EuiSelect", "EuiSuperSelect", "EuiComboBox", "EuiSelectable"]
NOISE = re.compile(r"^(data-test-subj|aria-.*|id|key|className|css|style|name)$")


def false(value):
    return value is not None and value["kind"] == "literal" and value["value"] is False


FEATURES = {
    "multiple selection": {"EuiComboBox": lambda p: p.get("singleSelection") is None or false(p["singleSelection"]),
                           "EuiSelectable": lambda p: p.get("singleSelection") is None or false(p["singleSelection"])},
    "search": {"EuiComboBox": lambda p: True, "EuiSelectable": lambda p: "searchable" in p},
    "async search": {"EuiComboBox": lambda p: "onSearchChange" in p, "EuiSelectable": lambda p: "searchProps" in p and "isLoading" in p},
    "create option": {"EuiComboBox": lambda p: "onCreateOption" in p},
    "renderOption": {"EuiComboBox": lambda p: "renderOption" in p, "EuiSelectable": lambda p: "renderOption" in p},
    "clearable": {"EuiComboBox": lambda p: not false(p.get("isClearable"))},
    "no initial selection": {"EuiSelect": lambda p: "hasNoInitialSelection" in p},
    "loading": {c: (lambda p: "isLoading" in p) for c in SELECTS},
    "invalid state": {c: (lambda p: "isInvalid" in p) for c in ["EuiSelect", "EuiSuperSelect", "EuiComboBox"]},
    "compressed": {c: (lambda p: "compressed" in p) for c in ["EuiSelect", "EuiSuperSelect", "EuiComboBox"]},
    "full width": {c: (lambda p: "fullWidth" in p) for c in ["EuiSelect", "EuiSuperSelect", "EuiComboBox"]},
    "prepend or append": {c: (lambda p: "prepend" in p or "append" in p) for c in ["EuiSelect", "EuiComboBox"]},
}
POPOVERS = {"EuiPopover", "EuiInputPopover", "EuiWrappingPopover"}


def entry(value, key):
    if value is None or value["kind"] != "object":
        return None
    for item in value["entries"]:
        if item.get("key") == key:
            return item["value"]
    return None


def followed(value):
    return ", followed" if value.get("via") else ""


def options_shape(value):
    if value is None:
        return "absent"
    if value["kind"] != "array":
        return reason(value) + followed(value)
    return "array literal" + followed(value)


def returns_array(call):
    return any(
        returned["kind"] == "array"
        for item in call["arguments"] if item.get("value", {}).get("kind") == "function"
        for returned in item["value"].get("returns", ()))


def selects(checkout, report, out):
    owners = owners_from_report(report)
    # A usage through wrappers has only the attributes its caller writes, not the props the
    # wrappers set, so props and features are counted on the other usages.
    through = [u for u in report["usages"] if u["component"] in SELECTS and "through" in u]
    usages = [u for u in report["usages"] if u["component"] in SELECTS and "through" not in u]
    per = {c: [u for u in usages if u["component"] == c] for c in SELECTS}

    rows = []
    for component, items in per.items():
        files = {u["file"] for u in items}
        teams = {o for f in files for o in owners.get(f, [])}
        spreads = sum(spread(u) for u in items)
        wrapped = sum(u["component"] == component for u in through)
        rows.append((component, len(items), len(files), len(teams), f"{spreads} ({percent(spreads, len(items))})", wrapped))
    table(["component", "usages", "files", "teams", "with {...spread}", "through wrappers"], rows)

    wrappers = collections.Counter(
        (u["component"], u["through"][-1]["component"], u["through"][-1]["file"]) for u in through)
    print("Wrappers whose usages reach a select, by the last wrapper before it")
    table(["component", "wrapper", "file", "usages"], [(*key, n) for key, n in wrappers.most_common(15)])

    teams = collections.Counter()
    for u in usages + through:
        for owner in owners.get(u["file"]) or ["(unowned)"]:
            teams[(owner, u["component"])] += 1
    totals = collections.Counter()
    for (owner, _), count in teams.items():
        totals[owner] += count
    table(["owner"] + SELECTS + ["total"],
          [[o] + [teams[(o, c)] for c in SELECTS] + [n] for o, n in totals.most_common(15)])

    for component, items in per.items():
        props = collections.Counter(p for u in items for p in named(u))
        print(f"{component}: most used props")
        table(["prop", "usages"], [(p, f"{n} ({percent(n, len(items))})") for p, n in props.most_common(15)])
        sets = collections.Counter(
            ", ".join(sorted(p for p in named(u) if not NOISE.match(p))) for u in items)
        print(f"{component}: most common prop sets (ignoring {NOISE.pattern})")
        table(["props", "usages"], [(s or "(none)", n) for s, n in sets.most_common(8)])

    print("Features used, by prop presence and literal values (\"-\" means the component doesn't offer it)")
    table(["feature"] + SELECTS, [
        [feature] + [percent(sum(bool(test[c](named(u))) for u in per[c]), len(per[c])) if c in test else "-"
                     for c in SELECTS]
        for feature, test in FEATURES.items()])

    print("`options`: what the value is")
    shapes = {c: collections.Counter(options_shape(named(u).get("options")) for u in per[c]) for c in SELECTS}
    keys = sorted({k for counter in shapes.values() for k in counter}, key=lambda k: -sum(s[k] for s in shapes.values()))
    table(["options"] + SELECTS, [[k] + [shapes[c][k] for c in SELECTS] for k in keys])
    calls = [named(u)["options"] for u in usages if options_shape(named(u).get("options")).startswith("call")]
    print("`options` that are a call: what's called, and whether a function among its arguments returns an array literal")
    table(["callee", "calls", "returns an array literal"], [
        (callee, n, sum(returns_array(v) for v in calls if v["callee"] == callee))
        for callee, n in collections.Counter(v["callee"] for v in calls).most_common(10)])
    lengths = [sum("value" in i for i in named(u)["options"]["items"]) for u in usages
               if options_shape(named(u).get("options")).startswith("array literal")]
    if lengths:
        print(f"array literal `options`: {len(lengths)}; items median {statistics.median(lengths)}, max {max(lengths)}\n")

    combo = per["EuiComboBox"]
    single_values = collections.Counter()
    for u in combo:
        value = named(u).get("singleSelection")
        if value is None or false(value):
            continue
        plain = entry(value, "asPlainText")
        if value["kind"] == "object":
            single_values["object, asPlainText: " + ("absent" if plain is None else
                          json.dumps(plain["value"]) if plain["kind"] == "literal" else reason(plain))] += 1
        else:
            single_values[f"{value['kind']} {json.dumps(value.get('value'))}" if value["kind"] == "literal" else reason(value)] += 1
    print("EuiComboBox `singleSelection` values")
    table(["value", "usages"], single_values.most_common())

    single = [u for u in combo
              if named(u).get("singleSelection") is not None and not false(named(u)["singleSelection"])
              and not {"onCreateOption", "onSearchChange", "async"} & named(u).keys()]
    plain = [u for u in single if (lambda v: v is not None and v["kind"] == "literal" and v["value"] is True)(
        entry(named(u)["singleSelection"], "asPlainText"))]
    labelless = [u for u in per["EuiSelect"] + per["EuiSuperSelect"] + per["EuiComboBox"]
                 if not spread(u) and not {"aria-label", "aria-labelledby", "id"} & named(u).keys()]

    def in_row(u):
        return bool(u["parents"]) and u["parents"][0]["component"] == "EuiFormRow" and "prop" not in u["parents"][0]

    in_popover = [u for u in per["EuiSelectable"] if any(p["component"] in POPOVERS for p in u["parents"])]
    findings = {
        "EuiComboBox single selection without create or async search": single,
        "EuiComboBox single selection with asPlainText: true": plain,
        "no aria-label, aria-labelledby or id, inside an EuiFormRow": [u for u in labelless if in_row(u)],
        "no aria-label, aria-labelledby or id, not inside an EuiFormRow": [u for u in labelless if not in_row(u)],
        "EuiSelectable inside a popover": in_popover,
        "props passed with {...spread} (wrappers or forwarding)": [u for u in usages if spread(u)],
        "through wrappers": through,
    }
    table(["pattern", "usages"], [(name, len(items)) for name, items in findings.items()])

    wrappers = collections.Counter(u["file"] for u in usages if spread(u))
    print("Files that spread props into a select")
    table(["file", "usages"], wrappers.most_common(15))
    write(out, {name: by_owner(items, owners) for name, items in findings.items()})


TEST = re.compile(r"(\.test\.|\.spec\.|\.stories\.|/__jest__/|/__mocks__/|/test/|/tests?_helpers?/|/cypress/|/e2e/|/ftr/|/scout/)")
DECLARATION = re.compile(r"(?:^|[;{\s])([a-z-]+)\s*:(?!:)", re.MULTILINE)


def shape(value):
    if value["kind"] == "template":
        return (f"template, tag {value['tag']}" if "tag" in value else "template") + followed(value)
    return reason(value) + followed(value)


def css_properties(value):
    """CSS properties set in object keys and template text, at any depth outside calls, whose
    arguments may be anything."""
    for path, node, _ in walk(value):
        if in_call(path):
            continue
        if path and path[-1] not in ("[]", "...", "${}") and node["kind"] != "object":
            yield path[-1]
        if node["kind"] == "template":
            text = "".join(part.get("text", "") for part in node["parts"])
            yield from DECLARATION.findall(text)


def styles(checkout, report, out, command):
    owners = owners_from_report(report)
    # A usage through wrappers has only the attributes its caller writes, so it's counted apart.
    usages = [u for u in report["usages"] if "through" not in u]
    through = [u for u in report["usages"] if "through" in u]
    per = collections.defaultdict(list)
    for u in usages:
        per[u["component"]].append(u)

    kinds = ("css", "className", "style")
    overridden = [u for u in usages if kinds[0] in named(u) or kinds[1] in named(u) or kinds[2] in named(u)]
    print(f"EUI usages: {len(usages)}; with css, className or style: {len(overridden)} ({percent(len(overridden), len(usages))})\n")
    for k in kinds:
        values = [named(u)[k] for u in usages if k in named(u)]
        print(f"`{k}`: {len(values)} usages; what the value is")
        table(["value", "usages"], collections.Counter(shape(v) for v in values).most_common(10))
        callees = collections.Counter(node["callee"] for v in values for path, node, _ in walk(v)
                                      if node["kind"] == "call" and not in_call(path))
        print(f"`{k}`: calls it's built by, outside other calls")
        table(["callee", "calls"], callees.most_common(10))

    rows = []
    for component, items in per.items():
        if len(items) < 100:
            continue
        counts = [sum(k in named(u) for u in items) for k in kinds]
        any_ = sum(any(k in named(u) for k in kinds) for u in items)
        rows.append((component, len(items), any_, percent(any_, len(items)), *counts))
    rows.sort(key=lambda row: -row[2] / row[1])
    print("Components with at least 100 usages, by share of overridden usages")
    table(["component", "usages", "overridden", "share"] + list(kinds), rows[:20])

    for k in ("css", "style"):
        properties = collections.Counter(p for u in usages if k in named(u) for p in set(css_properties(named(u)[k])))
        print(f"Properties set in `{k}`, in object keys and template text, by usages")
        table(["property", "usages"], properties.most_common(20))

    classes = collections.Counter()
    for u in usages:
        value = named(u).get("className")
        if value is None:
            continue
        tokens = set()
        for _, node, _ in walk(value):
            texts = [node["value"]] if string(node) else [p["text"] for p in node.get("parts", []) if "text" in p]
            tokens.update(t for text in texts for t in text.split() if t.startswith("eui"))
        classes.update(tokens)
    print("className tokens that are EUI's own classes, in literals and template text")
    table(["class", "usages"], classes.most_common(15))

    styled = collections.Counter(
        m.split("(", 1)[1].rstrip(")").split(".")[0]
        for _, _, _, m in rg(checkout, r"\b(styled|euiStyled)\((Eui[A-Z]\w*(\.\w+)?)\)", SOURCES))
    print(f"styled(Eui*) wrappers: {sum(styled.values())}")
    table(["component", "wrappers"], styled.most_common(10))

    cache = {}

    def styled_step(u):
        """Whether a wrapper on the way is a `styled(…)` or `euiStyled(…)` call."""
        for step in u["through"]:
            text = cache.setdefault(step["file"], lines(checkout, step["file"]))[step["line"] - 1]
            before = text.encode("utf-16-le")[: 2 * (step["column"] - 1)].decode("utf-16-le")
            if re.search(r"\b(styled|euiStyled)\(\s*$", before):
                return True
        return False

    by_styled = [u for u in through if styled_step(u)]
    wrapped = [u for u in through if any(k in named(u) for k in kinds)]
    print(f"usages through wrappers: {len(through)}, of which through a styled(…) or euiStyled(…) call: {len(by_styled)}; "
          f"with css, className or style written by their callers: {len(wrapped)}")
    table(["component", "through wrappers", "through styled(…)"], [
        (c, n, sum(u["component"] == c for u in by_styled))
        for c, n in collections.Counter(u["component"] for u in through).most_common(10)])

    selectors = [
        (f, l, c, m[m.index("."):])
        for f, l, c, m in rg(checkout, r"(^|[\s'\"`&>,(+~])\.eui[A-Z][A-Za-z0-9]*", ["*.scss", "*.css", "*.ts", "*.tsx", "*.js"])]
    tests = [s for s in selectors if TEST.search(s[0])]
    print(f"selectors targeting EUI classes in test files ({TEST.pattern}): {len(tests)}; left out below")
    selectors = [s for s in selectors if not TEST.search(s[0])]
    blocks = collections.Counter(m[1:] for _, _, _, m in selectors)
    files = {f for f, *_ in selectors}
    important = collections.Counter(f for f, *_ in rg(checkout, r"!important", ["*.scss", "*.css", "*.ts", "*.tsx", "*.js"]) if f in files)
    print(f"selectors targeting EUI classes: {len(selectors)} in {len(files)} files; "
          f"!important in those files: {sum(important.values())} in {len(important)} files")
    table(["class", "selectors"], blocks.most_common(20))

    owners |= owners_from_command(checkout, command, {f for f, *_ in selectors} - owners.keys())
    teams = collections.Counter(o for u in overridden for o in owners.get(u["file"]) or ["(unowned)"])
    selector_teams = collections.Counter(o for f, *_ in selectors for o in owners.get(f) or ["(unowned)"])
    table(["owner", "overridden usages", "EUI class selectors"],
          [(o, n, selector_teams[o]) for o, n in teams.most_common(15)])
    write(out, {
        "overrides": by_owner(
            [{k: u[k] for k in ("file", "line", "column", "component")} | {k: named(u)[k] for k in kinds if k in named(u)}
             for u in overridden], owners),
        "selectors": by_owner([{"file": f, "line": l, "column": c, "class": m[1:]} for f, l, c, m in selectors], owners),
    })


def main():
    use_case, checkout, report, out, *rest = sys.argv[1:]
    report = load(report)
    if use_case == "icons":
        icons(checkout, report, out, rest[1:])
    elif use_case == "selects":
        selects(checkout, report, out)
    elif use_case == "styles":
        styles(checkout, report, out, rest[1:])
    else:
        sys.exit(f"unknown use case: {use_case}")


if __name__ == "__main__":
    main()
