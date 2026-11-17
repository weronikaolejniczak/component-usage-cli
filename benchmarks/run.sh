#!/usr/bin/env bash
# Times a scan of a checkout and saves its report and environment.
#
# Usage: benchmarks/run.sh <checkout> <runs> <name> -- <scan arguments>
# Example: benchmarks/run.sh ../kibana 5 kibana-components -- . --components Icon --from some-library
#
# Builds the release binary, runs the scan once to warm the file system cache, then <runs>
# times, recording time and peak memory. Writes
# benchmarks/results/<name>/{environment.txt,times.txt,report.json.gz}.
# For a checkout without git history, such as an export, set CODEBASE to "<origin> <sha>".
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
  echo "codebase: ${CODEBASE:-$(git -C "$checkout" remote get-url origin) $(git -C "$checkout" rev-parse HEAD)}"
  echo "command: component-usage-cli scan $*"
  echo "rustc: $(rustc --version)"
  echo "os: $(uname -sr) $(sw_vers -productVersion 2>/dev/null || true)"
  echo "cpu: $(sysctl -n machdep.cpu.brand_string 2>/dev/null || grep -m1 'model name' /proc/cpuinfo)"
  echo "cores: $(getconf _NPROCESSORS_ONLN)"
  echo "cache: warm (one discarded run first)"
  echo "runs: $runs"
} > "$out/environment.txt"

if [ "$(uname -s)" = Darwin ]; then time=(/usr/bin/time -lp); else time=(/usr/bin/time -v); fi
cd "$checkout"
"$bin" scan "$@" > /dev/null
: > "$out/times.txt"
for _ in $(seq "$runs"); do
  "${time[@]}" "$bin" scan "$@" 2>> "$out/times.txt" > "$out/report.json"
done
gzip -nf "$out/report.json"

python3 - "$out/times.txt" <<'EOF'
import re, statistics, sys

text = open(sys.argv[1]).read()
# macOS: "real 4.41" and "<bytes> maximum resident set size"; Linux: "Elapsed (wall clock) time
# (h:mm:ss or m:ss): 0:04.41" and "Maximum resident set size (kbytes): <kB>".
real = [float(s) for s in re.findall(r"^real\s+([\d.]+)", text, re.M)]
real += [
    sum(float(part) * 60**i for i, part in enumerate(reversed(clock.split(":"))))
    for clock in re.findall(r"Elapsed \(wall clock\) time .*: ([\d:.]+)", text)
]
memory = [int(b) for b in re.findall(r"^\s*(\d+)\s+maximum resident set size", text, re.M)]
memory += [int(kb) * 1024 for kb in re.findall(r"Maximum resident set size \(kbytes\): (\d+)", text)]
print(f"real seconds: median {statistics.median(real):.2f}, min {min(real):.2f}, max {max(real):.2f}, n {len(real)}")
gb = [m / 1e9 for m in memory]
print(f"peak memory GB: median {statistics.median(gb):.2f}, min {min(gb):.2f}, max {max(gb):.2f}, n {len(gb)}")
EOF
