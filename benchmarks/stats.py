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

print(f"errors: {len(report['errors'])}")
for code, count in collections.Counter(e["code"] for e in report["errors"]).most_common():
    print(f"  {code}: {count}")
print(f"warnings: {len(report['warnings'])}")
