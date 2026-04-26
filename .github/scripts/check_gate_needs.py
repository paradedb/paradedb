#!/usr/bin/env python3
"""Assert every job in a gated workflow is listed in report-results.needs:.

The `report-results` job in each gated workflow acts as a required status check
for branch protection. Its `needs:` list must include every other job in the
workflow — otherwise a future job added without updating the gate's needs list
will be silently ignored, defeating the purpose of the gate.

This script parses all workflows under .github/workflows/ and, for any that
declare a `report-results` job, verifies the invariant. Exits non-zero on
violations so it can run as a CI step.
"""

from __future__ import annotations

import sys
from pathlib import Path

import yaml

WORKFLOWS_DIR = Path(".github/workflows")
GATE_JOB = "report-results"


def check_workflow(path: Path) -> list[str]:
    """Return a list of violation messages for a single workflow file."""
    with path.open() as f:
        doc = yaml.safe_load(f)

    if not isinstance(doc, dict):
        return []

    jobs = doc.get("jobs") or {}
    if GATE_JOB not in jobs:
        return []

    gate = jobs[GATE_JOB] or {}
    needs = gate.get("needs") or []
    if isinstance(needs, str):
        needs = [needs]
    needs_set = set(needs)

    expected = {name for name in jobs if name != GATE_JOB}
    missing = sorted(expected - needs_set)

    if not missing:
        return []

    return [
        f"{path}: {GATE_JOB}.needs is missing job(s): {', '.join(missing)}. "
        f"Every job in a gated workflow must be listed in {GATE_JOB}.needs "
        f"so its failures are not silently ignored by the gate."
    ]


def main() -> int:
    if not WORKFLOWS_DIR.is_dir():
        print(f"error: {WORKFLOWS_DIR} not found", file=sys.stderr)
        return 2

    violations: list[str] = []
    checked = 0
    for path in sorted(WORKFLOWS_DIR.glob("*.yml")):
        checked += 1
        violations.extend(check_workflow(path))

    if violations:
        for v in violations:
            print(v, file=sys.stderr)
        print(
            f"\n{len(violations)} gate-needs violation(s) across "
            f"{checked} workflow(s).",
            file=sys.stderr,
        )
        return 1

    print(f"[gate-needs lint] checked {checked} workflow(s), all gates complete.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
