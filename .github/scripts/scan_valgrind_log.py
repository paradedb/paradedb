#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Scan a Postgres log produced by `cargo pgrx regress --valgrind` for genuine
memory-corruption errors and group identical reports by error category + call
stack, so CI output points at the offending codepath instead of dumping
thousands of identical "Conditional jump..." lines.

Leak summaries are intentionally ignored: normal Postgres operation leaks heavily
and would drown the signal in false positives.

Usage: scan_valgrind_log.py <postgres-log-path>
Prints a deduplicated report and exits 1 if any memory error is found, else 0.
"""

import collections
import os
import re
import sys

# Genuine memory-corruption categories Valgrind reports between
# VALGRINDERROR-BEGIN/END. NOT leak accounting ("definitely lost", etc.).
ERROR_RE = re.compile(
    r"Invalid read|Invalid write|Invalid free|Mismatched free|"
    r"Use of uninitialised|Conditional jump or move depends on uninitialised|"
    r"Source and destination overlap|"
    r"Syscall param .* points to (?:unaddressable|uninitialised)"
)

# Strip Valgrind's "==<pid>== " / "==<timestamp> <pid>== " line prefix.
PREFIX_RE = re.compile(r"^==[^=]*==\s?")
# A stack frame: "   at 0xADDR: <symbol> (...)" or "   by 0xADDR: <symbol> (...)".
FRAME_RE = re.compile(r"^\s*(?:at|by)\s+0x[0-9A-Fa-f]+:\s+(.*)$")
# Drop the "(in /.../pg_search.so)" object qualifier so frames group stably
# (the load address embedded in the path changes run to run).
OBJ_RE = re.compile(r"\s*\(in [^)]*\)\s*$")


def parse(path):
    """Yield (category, [frames]) for each VALGRINDERROR block in the log."""
    with open(path, encoding="utf-8", errors="replace") as fh:
        in_block = False
        category = None
        frames = []
        for raw in fh:
            line = PREFIX_RE.sub("", raw.rstrip("\n"))
            if line == "VALGRINDERROR-BEGIN":
                in_block, category, frames = True, None, []
                continue
            if line == "VALGRINDERROR-END":
                if in_block and category:
                    yield category, frames
                in_block = False
                continue
            if not in_block:
                continue
            if category is None:
                # First non-empty line after BEGIN is the error category.
                if line.strip():
                    category = line.strip()
                continue
            match = FRAME_RE.match(line)
            if match:
                frames.append(OBJ_RE.sub("", match.group(1)).strip())


def main():
    if len(sys.argv) != 2:
        sys.exit("usage: scan_valgrind_log.py <postgres-log-path>")
    path = sys.argv[1]
    if not os.path.isfile(path):
        print(f"::error::Postgres log not found at {path}")
        sys.exit(1)

    # Group identical reports by (category, full call stack).
    groups = collections.OrderedDict()
    total = 0
    for category, frames in parse(path):
        if not ERROR_RE.search(category):
            continue
        total += 1
        sig = (category, tuple(frames))
        group = groups.get(sig)
        if group is None:
            groups[sig] = {"count": 1, "category": category, "frames": frames}
        else:
            group["count"] += 1

    if not groups:
        print(f"No Valgrind memory errors found in {path}")
        return

    ordered = sorted(groups.values(), key=lambda g: -g["count"])
    print(
        f"::error::Valgrind detected {total} memory error(s) across "
        f"{len(ordered)} unique codepath(s) — see the grouped report below "
        f"and the uploaded Postgres log artifact."
    )
    for i, group in enumerate(ordered, 1):
        print()
        print(f"================ Error #{i} — seen {group['count']}x ================")
        print(group["category"])
        if group["frames"]:
            for depth, frame in enumerate(group["frames"]):
                print(f"    {'at' if depth == 0 else 'by'} {frame}")
        else:
            print("    (no stack frames captured)")
    sys.exit(1)


if __name__ == "__main__":
    main()
