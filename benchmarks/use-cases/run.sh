#!/usr/bin/env bash
# Answers the use cases in this folder for Kibana at a pinned commit.
#
# Usage: benchmarks/use-cases/run.sh <kibana-git-dir> <sha> <work-dir> [codeowners-command]
# Example: benchmarks/use-cases/run.sh ../kibana 6dce6e1bbcf1fcc46be7011088c3aa1e81feba19 /tmp/kibana
#
# Exports <sha> into <work-dir> without touching <kibana-git-dir>, installs the @elastic/eui
# version its package.json pins and links its @kbn packages into node_modules, as bootstrap does.
# Other dependencies aren't installed, so elements imported from them are `unidentified`; that
# doesn't change EUI usages. Then scans every export of @elastic/eui with owners, following the
# variables of each file, and writes
# benchmarks/results/use-cases-kibana/{environment.txt,report.json.gz,<use case>.md,<use case>.json.gz}.
# [codeowners-command] lists owners of files without EUI usages; defaults to `codeowners` from
# github.com/hmarr/codeowners.
set -euo pipefail

kibana=$(cd "$1" && pwd)
sha=$2
work=$3
codeowners=${4:-codeowners}

here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../.." && pwd)
out="$repo/benchmarks/results/use-cases-kibana"
mkdir -p "$out" "$work"

if [ ! -e "$work/package.json" ]; then
  git -C "$kibana" archive "$sha" | tar -x -C "$work"
  git -C "$work" init -q
fi
eui=$(python3 -c 'import json, sys; print(json.load(open(sys.argv[1]))["dependencies"]["@elastic/eui"])' "$work/package.json")
if [ ! -e "$work/node_modules/@elastic/eui/package.json" ]; then
  mkdir -p "$work/node_modules/@elastic/eui"
  tarball=$(cd "$work" && npm pack --silent "@elastic/eui@$eui")
  tar -xzf "$work/$tarball" -C "$work/node_modules/@elastic/eui" --strip-components 1
  rm "$work/$tarball"
fi
python3 - "$work" <<'EOF'
import json, os, re, sys

root = sys.argv[1]
for directory, dirs, files in os.walk(root):
    dirs[:] = [d for d in dirs if d not in ("node_modules", ".git", "target", "build")]
    name = None
    if "kibana.jsonc" in files:
        match = re.search(r'"id"\s*:\s*"(@kbn/[^"]+)"', open(os.path.join(directory, "kibana.jsonc")).read())
        name = match and match.group(1)
    elif "package.json" in files and directory != root:
        try:
            name = json.load(open(os.path.join(directory, "package.json"))).get("name")
        except ValueError:
            name = None
        if not (isinstance(name, str) and name.startswith("@kbn/")):
            name = None
    if name:
        link = os.path.join(root, "node_modules", name)
        os.makedirs(os.path.dirname(link), exist_ok=True)
        if not os.path.lexists(link):
            os.symlink(os.path.abspath(directory), link)
EOF

cargo build --release --quiet --manifest-path "$repo/Cargo.toml"
bin="$(cargo metadata --format-version 1 --no-deps --manifest-path "$repo/Cargo.toml" |
  python3 -c 'import json, sys; print(json.load(sys.stdin)["target_directory"])')/release/component-usage-cli"

{
  echo "tool commit: $(git -C "$repo" rev-parse HEAD)$(git -C "$repo" diff --quiet -- . ":!benchmarks/results" || echo ' (uncommitted changes)')"
  echo "codebase: $(git -C "$kibana" remote get-url origin) $sha"
  echo "library: @elastic/eui $eui"
  echo "codeowners: $(go version -m "$(command -v "$codeowners")" 2>/dev/null | awk '$1 == "mod" { print $2, $3 }')"
  echo "python: $(python3 --version)"
  echo "rg: $(rg --version | head -1)"
} > "$out/environment.txt"

cd "$work"
"$bin" scan . --from @elastic/eui --owners --follow variables > "$out/report.json"
python3 "$here/analyse.py" icons . "$out/report.json" "$out/icons.json" -- "$codeowners" > "$out/icons.md"
python3 "$here/analyse.py" selects . "$out/report.json" "$out/selects.json" > "$out/selects.md"
python3 "$here/analyse.py" styles . "$out/report.json" "$out/styles.json" -- "$codeowners" > "$out/styles.md"
gzip -nf "$out/report.json" "$out/icons.json" "$out/selects.json" "$out/styles.json"
