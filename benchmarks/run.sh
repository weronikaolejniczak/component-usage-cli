#!/usr/bin/env bash
# Times a scan of a checkout and saves its report and environment.
#
# Usage: benchmarks/run.sh <checkout> <runs> <name> -- <scan arguments>
# Example: benchmarks/run.sh ../kibana 5 kibana-components -- . --components Icon --from some-library
#
# Builds the release binary, runs the scan once to warm the file system cache, then <runs>
# times. Writes benchmarks/results/<name>/{environment.txt,times.txt,report.json.gz}.
set -euo pipefail

checkout=$(cd "$1" && pwd)
runs=$2
name=$3
shift 4

repo=$(cd "$(dirname "$0")/.." && pwd)
out="$repo/benchmarks/results/$name"
mkdir -p "$out"

cargo build --release --quiet --manifest-path "$repo/Cargo.toml"
bin="$(cargo metadata --format-version 1 --no-deps --manifest-path "$repo/Cargo.toml" |
  python3 -c 'import json, sys; print(json.load(sys.stdin)["target_directory"])')/release/component-usage-cli"

{
  echo "tool commit: $(git -C "$repo" rev-parse HEAD)$(git -C "$repo" diff --quiet -- . ":!benchmarks/results" || echo ' (uncommitted changes)')"
  echo "codebase: $(git -C "$checkout" remote get-url origin) $(git -C "$checkout" rev-parse HEAD)"
  echo "command: component-usage-cli scan $*"
  echo "rustc: $(rustc --version)"
  echo "os: $(uname -sr) $(sw_vers -productVersion 2>/dev/null || true)"
  echo "cpu: $(sysctl -n machdep.cpu.brand_string 2>/dev/null || grep -m1 'model name' /proc/cpuinfo)"
  echo "cores: $(getconf _NPROCESSORS_ONLN)"
  echo "cache: warm (one discarded run first)"
  echo "runs: $runs"
} > "$out/environment.txt"

cd "$checkout"
"$bin" scan "$@" > /dev/null
: > "$out/times.txt"
for _ in $(seq "$runs"); do
  /usr/bin/time -p "$bin" scan "$@" 2>> "$out/times.txt" > "$out/report.json"
done
gzip -f "$out/report.json"

python3 - "$out/times.txt" <<'EOF'
import statistics, sys
lines = open(sys.argv[1]).read().split()
real = [float(lines[i + 1]) for i, word in enumerate(lines) if word == "real"]
print(f"real seconds: median {statistics.median(real):.2f}, min {min(real):.2f}, max {max(real):.2f}, n {len(real)}")
EOF
