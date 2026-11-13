#!/usr/bin/env python3
"""Answer the use cases in this folder from the reports of each tool.

Usage:
  analyse.py icons   <checkout> <react-scanner.json> <report.json> <out.json> -- <codeowners command...>
  analyse.py selects <checkout> <react-scanner.json> <report.json> <out.json>
  analyse.py styles  <checkout> <react-scanner.json> <report.json> <out.json> -- <codeowners command...>

<react-scanner.json> is a react-scanner raw report of every component
imported from the library. <report.json> is a `scan --owners` report of the
same components and is used for owners. Prints a summary and writes the full
findings, grouped by owner, to <out.json>.
"""

import collections
import gzip
import json
import os
import re
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
SOURCES = ["*.ts", "*.tsx", "*.js", "*.jsx", "*.mjs", "*.cjs"]


def load(path):
    opener = gzip.open if path.endswith(".gz") else open
    with opener(path, "rt") as f:
        return json.load(f)


def instances(scanner, checkout):
    def walk(name, node):
        for instance in node.get("instances", []):
            yield name, instance
        for member, child in node.get("components", {}).items():
            yield from walk(f"{name}.{member}", child)

    prefix = checkout.rstrip("/") + "/"
    for component, node in scanner.items():
        for name, instance in walk(component, node):
            location = instance["location"]
            yield {
                "component": name,
                "file": location["file"].removeprefix(prefix),
                "line": location["start"]["line"],
                "column": location["start"]["column"],
                "props": instance.get("props") or {},
                "spread": instance.get("propsSpread", False),
            }


def literal(value):
    return not (isinstance(value, str) and value.startswith("(") and value.endswith(")"))


def rg(checkout, pattern, globs):
    args = ["rg", "--line-number", "--column", "--no-heading", "--only-matching", "--hidden"]
    args += ["--glob", "!.git", "--glob", "!**/node_modules/**"]
    for glob in globs:
        args += ["--glob", glob]
    out = subprocess.run(args + [pattern], cwd=checkout, capture_output=True, text=True).stdout
    for row in out.splitlines():
        file, line, column, match = row.split(":", 3)
        yield file.removeprefix("./"), int(line), int(column), match


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


ICON_PROPS = {"iconType", "icon", "iconLeft", "iconRight"}


def icon_prop(component, prop):
    return prop in ICON_PROPS or (prop == "type" and component == "EuiIcon")


def icon_attribute(name, before):
    if "icon" in name.lower():
        return True
    tags = re.findall(r"<([\w.]+)", before)
    return name == "type" and bool(tags) and "Icon" in tags[-1]


def icons(checkout, scanner, report, out, command):
    aliases = load(os.path.join(HERE, "icon-aliases.json"))["aliases"]
    found, review = [], []
    unmatched = collections.defaultdict(list)
    for usage in instances(scanner, checkout):
        for prop, value in usage["props"].items():
            if not icon_prop(usage["component"], prop):
                continue
            entry = {key: usage[key] for key in ("file", "line", "column", "component")} | {"prop": prop}
            if not literal(value):
                review.append(entry | {"expression": value})
            elif value in aliases:
                found.append(entry | {"kind": "eui-jsx", "alias": value, "replacement": aliases[value]})
                unmatched[(usage["file"], value)].append(usage["line"])

    names = "|".join(sorted(aliases, key=len, reverse=True))
    attribute = re.compile(r"\b([\w-]+)=\{?\s*$")
    icon_key = re.compile(r"\w*icon\w*['\"]?\s*:\s*$", re.IGNORECASE)
    cache = {}
    for file, line, column, match in rg(checkout, rf"['\"`]({names})['\"`]", SOURCES):
        alias = match[1:-1]
        source = cache.setdefault(file, lines(checkout, file))
        text = source[line - 1]
        before = text[: column - 1]
        jsx = attribute.search(before)
        if jsx:
            starts = unmatched.get((file, alias), [])
            near = [start for start in starts if start <= line]
            if near:
                starts.remove(max(near))
                continue
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

    kinds = collections.Counter(entry["kind"] for entry in found)
    print(f"aliases: {len(aliases)}; findings: {len(found)}; dynamic icon props to review: {len(review)}\n")
    table(["kind", "findings"], kinds.most_common())
    per_alias = collections.defaultdict(collections.Counter)
    for entry in found:
        per_alias[entry["alias"]][entry["kind"]] += 1
    order = ["eui-jsx", "other-jsx", "icon-key", "other-string"]
    table(["alias"] + order, sorted(
        ([alias] + [counts[kind] for kind in order] for alias, counts in per_alias.items()),
        key=lambda row: (-sum(row[1:4]), -row[4], row[0]),
    ))
    precise = [entry for entry in found if entry["kind"] != "other-string"]
    grouped = by_owner(precise, owners)
    table(["owner", "eui-jsx, other-jsx, icon-key findings"], [(o, len(e)) for o, e in list(grouped.items())[:15]])
    write(out, {"findings": by_owner(found, owners), "review": by_owner(review, owners)})


SELECTS = ["EuiSelect", "EuiSuperSelect", "EuiComboBox", "EuiSelectable"]
NOISE = re.compile(r"^(data-test-subj|aria-.*|id|key|className|css|style|name)$")
FEATURES = {
    "multiple selection": {"EuiComboBox": lambda p: p.get("singleSelection") in (None, False),
                           "EuiSelectable": lambda p: p.get("singleSelection") in (None, False)},
    "search": {"EuiComboBox": lambda p: True, "EuiSelectable": lambda p: "searchable" in p},
    "async search": {"EuiComboBox": lambda p: "onSearchChange" in p, "EuiSelectable": lambda p: "searchProps" in p and "isLoading" in p},
    "create option": {"EuiComboBox": lambda p: "onCreateOption" in p},
    "renderOption": {"EuiComboBox": lambda p: "renderOption" in p, "EuiSelectable": lambda p: "renderOption" in p},
    "clearable": {"EuiComboBox": lambda p: p.get("isClearable", True) is not False},
    "no initial selection": {"EuiSelect": lambda p: "hasNoInitialSelection" in p},
    "loading": {c: (lambda p: "isLoading" in p) for c in SELECTS},
    "invalid state": {c: (lambda p: "isInvalid" in p) for c in ["EuiSelect", "EuiSuperSelect", "EuiComboBox"]},
    "compressed": {c: (lambda p: "compressed" in p) for c in ["EuiSelect", "EuiSuperSelect", "EuiComboBox"]},
    "full width": {c: (lambda p: "fullWidth" in p) for c in ["EuiSelect", "EuiSuperSelect", "EuiComboBox"]},
    "prepend or append": {c: (lambda p: "prepend" in p or "append" in p) for c in ["EuiSelect", "EuiComboBox"]},
}


def selects(checkout, scanner, report, out):
    owners = owners_from_report(report)
    usages = [u for u in instances(scanner, checkout) if u["component"] in SELECTS]
    per = {c: [u for u in usages if u["component"] == c] for c in SELECTS}

    rows = []
    for component, items in per.items():
        files = {u["file"] for u in items}
        teams = {o for f in files for o in owners.get(f, [])}
        spread = sum(u["spread"] for u in items)
        rows.append((component, len(items), len(files), len(teams), f"{spread} ({percent(spread, len(items))})"))
    table(["component", "usages", "files", "teams", "with {...spread}"], rows)

    teams = collections.Counter()
    for u in usages:
        for owner in owners.get(u["file"]) or ["(unowned)"]:
            teams[(owner, u["component"])] += 1
    totals = collections.Counter()
    for (owner, _), count in teams.items():
        totals[owner] += count
    table(["owner"] + SELECTS + ["total"],
          [[o] + [teams[(o, c)] for c in SELECTS] + [n] for o, n in totals.most_common(15)])

    for component, items in per.items():
        props = collections.Counter(p for u in items for p in u["props"])
        print(f"{component}: most used props")
        table(["prop", "usages"], [(p, f"{n} ({percent(n, len(items))})") for p, n in props.most_common(15)])
        sets = collections.Counter(
            ", ".join(sorted(p for p in u["props"] if not NOISE.match(p))) for u in items)
        print(f"{component}: most common prop sets (ignoring {NOISE.pattern})")
        table(["props", "usages"], [(s or "(none)", n) for s, n in sets.most_common(8)])

    print("Features used, by prop presence (\"-\" means the component doesn't offer it)")
    table(["feature"] + SELECTS, [
        [feature] + [percent(sum(bool(test[c](u["props"])) for u in per[c]), len(per[c])) if c in test else "-"
                     for c in SELECTS]
        for feature, test in FEATURES.items()])

    single = [u for u in per["EuiComboBox"]
              if u["props"].get("singleSelection") not in (None, False)
              and not {"onCreateOption", "onSearchChange", "async"} & u["props"].keys()]
    plain = [u for u in single if u["props"].get("singleSelection") == "(ObjectExpression)"]
    labelless = [u for u in per["EuiSelect"] + per["EuiSuperSelect"] + per["EuiComboBox"]
                 if not u["spread"] and not {"aria-label", "aria-labelledby", "id"} & u["props"].keys()]
    findings = {
        "EuiComboBox single selection without create or async search": single,
        "EuiComboBox single selection with an object value": plain,
        "no aria-label, aria-labelledby or id (may still get a label from EuiFormRow)": labelless,
        "props passed with {...spread} (wrappers or forwarding)": [u for u in usages if u["spread"]],
    }
    table(["pattern", "usages"], [(name, len(items)) for name, items in findings.items()])

    wrappers = collections.Counter(u["file"] for u in usages if u["spread"])
    print("Files that spread props into a select (wrapper candidates)")
    table(["file", "usages"], wrappers.most_common(15))
    write(out, {name: by_owner(items, owners) for name, items in findings.items()})


TEST = re.compile(r"(\.test\.|\.spec\.|\.stories\.|/__jest__/|/__mocks__/|/test/|/tests?_helpers?/|/cypress/|/e2e/|/ftr/|/scout/)")


def styles(checkout, scanner, report, out, command):
    owners = owners_from_report(report)
    usages = list(instances(scanner, checkout))
    per = collections.defaultdict(list)
    for u in usages:
        per[u["component"]].append(u)

    kinds = ("css", "className", "style")
    overridden = [u for u in usages if any(k in u["props"] for k in kinds)]
    print(f"EUI usages: {len(usages)}; with css, className or style: {len(overridden)} ({percent(len(overridden), len(usages))})\n")
    table(["prop", "usages", "literal values"], [
        (k, sum(k in u["props"] for u in usages), sum(k in u["props"] and literal(u["props"][k]) for u in usages))
        for k in kinds])

    rows = []
    for component, items in per.items():
        if len(items) < 100:
            continue
        counts = [sum(k in u["props"] for u in items) for k in kinds]
        any_ = sum(any(k in u["props"] for k in kinds) for u in items)
        rows.append((component, len(items), any_, percent(any_, len(items)), *counts))
    rows.sort(key=lambda row: -row[2] / row[1])
    print("Components with at least 100 usages, by share of overridden usages")
    table(["component", "usages", "overridden", "share"] + list(kinds), rows[:20])

    classes = collections.Counter()
    for u in usages:
        value = u["props"].get("className")
        if isinstance(value, str) and literal(value):
            classes.update(token for token in value.split() if token.startswith("eui"))
    print("Literal className tokens that are EUI's own classes")
    table(["class", "usages"], classes.most_common(15))

    styled = collections.Counter(
        m.split("(", 1)[1].rstrip(")").split(".")[0]
        for _, _, _, m in rg(checkout, r"\b(styled|euiStyled)\((Eui[A-Z]\w*(\.\w+)?)\)", SOURCES))
    print(f"styled(Eui*) wrappers: {sum(styled.values())}")
    table(["component", "wrappers"], styled.most_common(10))

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
            [{k: u[k] for k in ("file", "line", "column", "component")} | {k: u["props"][k] for k in kinds if k in u["props"]}
             for u in overridden], owners),
        "selectors": by_owner([{"file": f, "line": l, "column": c, "class": m[1:]} for f, l, c, m in selectors], owners),
    })


def main():
    use_case, checkout, scanner, report, out, *rest = sys.argv[1:]
    scanner, report = load(scanner), load(report)
    if use_case == "icons":
        icons(checkout, scanner, report, out, rest[1:])
    elif use_case == "selects":
        selects(checkout, scanner, report, out)
    elif use_case == "styles":
        styles(checkout, scanner, report, out, rest[1:])
    else:
        sys.exit(f"unknown use case: {use_case}")


if __name__ == "__main__":
    main()
