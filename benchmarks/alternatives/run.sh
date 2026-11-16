#!/usr/bin/env bash
# Runs react-scanner and component-usage-cli on the same checkout and compares them.
#
# Usage: benchmarks/alternatives/run.sh <checkout> <runs> <name> <library>
# Example: benchmarks/alternatives/run.sh ../kibana 3 kibana-alternatives @elastic/eui
#
# react-scanner reports every component imported from <library>; component-usage-cli is then
# asked for the same components, with owners. Each tool runs once to warm the file system
# cache, then <runs> times. Writes benchmarks/results/<name>/{environment.txt,
# times-react-scanner.txt,react-scanner-stderr.txt,times-component-usage-cli.txt,
# react-scanner.json.gz,report.json.gz,comparison.txt}.
set -euo pipefail

checkout=$(cd "$1" && pwd)
runs=$2
name=$3
library=$4

here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../.." && pwd)
out="$repo/benchmarks/results/$name"
mkdir -p "$out"

npm ci --silent --prefix "$here"
cargo build --release --quiet --manifest-path "$repo/Cargo.toml"
bin="$(cargo metadata --format-version 1 --no-deps --manifest-path "$repo/Cargo.toml" |
  python3 -c 'import json, sys; print(json.load(sys.stdin)["target_directory"])')/release/component-usage-cli"

{
  echo "tool commit: $(git -C "$repo" rev-parse HEAD)$(git -C "$repo" diff --quiet -- . ":!benchmarks/results" || echo ' (uncommitted changes)')"
  echo "codebase: $(git -C "$checkout" remote get-url origin) $(git -C "$checkout" rev-parse HEAD)"
  echo "library: $library"
  echo "react-scanner: $("$here/node_modules/.bin/react-scanner" --version)"
  echo "node: $(node --version)"
  echo "rustc: $(rustc --version)"
  echo "os: $(uname -sr) $(sw_vers -productVersion 2>/dev/null || true)"
  echo "cpu: $(sysctl -n machdep.cpu.brand_string 2>/dev/null || grep -m1 'model name' /proc/cpuinfo)"
  echo "cores: $(getconf _NPROCESSORS_ONLN)"
  echo "cache: warm (one discarded run of each tool first)"
  echo "runs: $runs"
} > "$out/environment.txt"

export CHECKOUT="$checkout" LIBRARY="$library" OUT="$out/react-scanner.json"
scanner=("$here/node_modules/.bin/react-scanner" -c "$here/react-scanner.config.js")
python3 "$here/measure.py" 1 /dev/null -- "${scanner[@]}" > /dev/null 2>&1
python3 "$here/measure.py" "$runs" /dev/null -- "${scanner[@]}" 2> "$out/react-scanner-stderr.txt" |
  tee "$out/times-react-scanner.txt"

components=$(python3 "$here/compare.py" components "$OUT")
cd "$checkout"
scan=("$bin" scan . --components "$components" --from "$library" --owners)
python3 "$here/measure.py" 1 /dev/null -- "${scan[@]}" > /dev/null
python3 "$here/measure.py" "$runs" "$out/report.json" -- "${scan[@]}" | tee "$out/times-component-usage-cli.txt"

python3 "$here/compare.py" usages "$checkout" "$OUT" "$out/report.json" | tee "$out/comparison.txt"
for file in "$OUT" "$out/react-scanner-stderr.txt"; do
  python3 -c 'import sys; p, f = sys.argv[1] + "/", sys.argv[2]; t = open(f).read(); open(f, "w").write(t.replace(p, ""))' \
    "$checkout" "$file"
done
gzip -f "$OUT" "$out/report.json"
