#!/usr/bin/env python3
"""Run a command <runs> times and print wall time and peak memory of each run.

Usage: measure.py <runs> <stdout file> -- <command...>
"""

import os
import subprocess
import sys
import time


def main():
    runs, stdout, _, *command = sys.argv[1:]
    for _ in range(int(runs)):
        start = time.monotonic()
        with open(stdout, "w") as out:
            pid = subprocess.Popen(command, stdout=out).pid
            _, status, usage = os.wait4(pid, 0)
        seconds = time.monotonic() - start
        if status:
            sys.exit(f"command failed with status {status}")
        # ru_maxrss is in bytes on macOS and kilobytes on Linux.
        scale = 1 if sys.platform == "darwin" else 1024
        print(f"real {seconds:.2f} max_rss_mb {usage.ru_maxrss * scale / 2**20:.0f}", flush=True)


if __name__ == "__main__":
    main()
