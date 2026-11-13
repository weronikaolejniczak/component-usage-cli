#!/usr/bin/env python3
"""Compare the usages found by react-scanner and by component-usage-cli.

Usage:
  compare.py components <react-scanner.json>
      Prints every component react-scanner found, comma-separated, members included.
  compare.py usages <checkout> <react-scanner.json> <report.json[.gz]>
      Prints how many usages each tool found and every usage found by only one.
"""

import gzip
import json
import sys


def load(path):
    opener = gzip.open if path.endswith(".gz") else open
    with opener(path, "rt") as f:
        return json.load(f)


def walk(name, node):
    for instance in node.get("instances", []):
        yield name, instance
    for member, child in node.get("components", {}).items():
        yield from walk(f"{name}.{member}", child)


def main():
    if sys.argv[1] == "components":
        scanner = load(sys.argv[2])
        print(",".join(sorted({name for c, node in scanner.items() for name, _ in walk(c, node)})))
        return
    checkout, scanner, report = sys.argv[2:]
    prefix = checkout.rstrip("/") + "/"
    theirs = {
        (name, i["location"]["file"].removeprefix(prefix), i["location"]["start"]["line"], i["location"]["start"]["column"])
        for c, node in load(scanner).items()
        for name, i in walk(c, node)
    }
    ours = {(u["component"], u["file"], u["line"], u["column"]) for u in load(report)["usages"]}
    print(f"react-scanner: {len(theirs)}, component-usage-cli: {len(ours)}, both: {len(ours & theirs)}")
    for usage in sorted(theirs - ours):
        print("  only react-scanner:", *usage)
    for usage in sorted(ours - theirs):
        print("  only component-usage-cli:", *usage)


if __name__ == "__main__":
    main()
