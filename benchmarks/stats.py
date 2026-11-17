"""Summarises a scan report: usages per component and owner, unidentified elements and errors.

Usage: python3 benchmarks/stats.py <report.json or report.json.gz> [top owners per component]
"""

import collections
import gzip
import json
import sys

path = sys.argv[1]
top = int(sys.argv[2]) if len(sys.argv) > 2 else 5
with (gzip.open if path.endswith(".gz") else open)(path, "rt") as file:
    report = json.load(file)

usages = report["usages"]
print(f"usages: {len(usages)} in {len({usage['file'] for usage in usages})} files")
for component, count in collections.Counter(u["component"] for u in usages).most_common():
    owned = [u for u in usages if u["component"] == component]
    teams = collections.Counter(owner for u in owned for owner in (u.get("owners") or ["(unowned)"]))
    files = len({u["file"] for u in owned})
    print(f"  {component}: {count} usages in {files} files, {len(teams)} owners")
    for team, team_count in teams.most_common(top):
        print(f"    {team}: {team_count}")

unidentified = report["unidentified"]
print(f"unidentified: {len(unidentified)}")
for (code, component), count in collections.Counter(
    (u["code"], u.get("component", "")) for u in unidentified
).most_common():
    print(f"  {code} {component}: {count}")

def label(value):
    parts = [value["kind"]]
    if value["kind"] == "unresolved":
        parts.append(value["code"] + (f" ({value['step']})" if "step" in value else ""))
    if value.get("via"):
        parts.append("followed")
    return ", ".join(parts)


def nested(value):
    """Values inside a value: entries, items, branches and template parts."""
    for child in value.get("values", []):
        yield child
    for holder in value.get("entries", []) + value.get("items", []) + value.get("parts", []):
        if isinstance(holder, dict) and isinstance(holder.get("value"), dict):
            yield holder["value"]


props = [prop["value"] for u in usages for prop in u.get("props", []) if isinstance(prop.get("value"), dict)]
print(f"prop values: {len(props)}")
for kind, count in collections.Counter(label(v) for v in props).most_common():
    print(f"  {kind}: {count}")
everything, stack = collections.Counter(), list(props)
while stack:
    value = stack.pop()
    everything[label(value)] += 1
    stack.extend(nested(value))
print(f"values at any depth: {sum(everything.values())}")
for kind, count in everything.most_common():
    print(f"  {kind}: {count}")

print(f"errors: {len(report['errors'])}")
for code, count in collections.Counter(e["code"] for e in report["errors"]).most_common():
    print(f"  {code}: {count}")
print(f"warnings: {len(report['warnings'])}")
