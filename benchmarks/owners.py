#!/usr/bin/env python3
"""Compare the owners in a report with another CODEOWNERS implementation.

Usage: benchmarks/owners.py <checkout> <report.json[.gz]> <command...>

The command is run in the checkout with the report's distinct files as
arguments and must print one `<file> <owner> <owner>...` line per file, as
`codeowners` from github.com/hmarr/codeowners does. Files are passed in
batches. Prints the agreement and every disagreement.
"""

import gzip
import json
import subprocess
import sys


def load(path):
    opener = gzip.open if path.endswith(".gz") else open
    with opener(path, "rt") as f:
        return json.load(f)


def theirs(checkout, command, files):
    owners = {}
    for start in range(0, len(files), 500):
        batch = files[start : start + 500]
        out = subprocess.run(
            command + batch, cwd=checkout, capture_output=True, text=True, check=True
        ).stdout
        for line in out.splitlines():
            file, *rest = line.split()
            owners[file] = [] if rest == ["(unowned)"] else rest
    return owners


def main():
    checkout, report, *command = sys.argv[1:]
    ours = {}
    for usage in load(report)["usages"]:
        ours[usage["file"]] = usage["owners"]
    files = sorted(ours)
    other = theirs(checkout, command, files)
    differ = [f for f in files if ours[f] != other.get(f)]
    print(f"files: {len(files)}, agree: {len(files) - len(differ)}, differ: {len(differ)}")
    for f in differ:
        print(f"  {f}: ours {ours[f]}, theirs {other.get(f)}")


if __name__ == "__main__":
    main()
