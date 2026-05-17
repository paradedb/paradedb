"""Check GitHub workflow path-filter guardrails.

`dorny/paths-filter` evaluates each pattern in a filter according to the
configured predicate quantifier. With the default `some`, a negated pattern such
as `!pg_search/README.md` becomes a broad match for almost every other file.
That is not equivalent to GitHub's native `on.pull_request.paths` exclusion
semantics, and can accidentally run expensive jobs on unrelated PRs.

This script fails CI when a dorny filter uses negated patterns without
`predicate-quantifier: every`.
"""

from __future__ import annotations

import sys
from collections.abc import Iterable
from pathlib import Path
from typing import Any

import yaml  # pylint: disable=import-error

WORKFLOWS_DIR = Path(".github/workflows")
DORNY_ACTION = "dorny/paths-filter"


def iter_patterns(item: Any) -> Iterable[str]:
    """Yield string path patterns from a dorny filter item."""
    if isinstance(item, str):
        yield item
        return

    if isinstance(item, list):
        for entry in item:
            yield from iter_patterns(entry)
        return

    if isinstance(item, dict):
        for value in item.values():
            yield from iter_patterns(value)


def check_dorny_step(path: Path, job_name: str, step: dict[str, Any]) -> list[str]:
    """Return violation messages for a single dorny/paths-filter step."""
    step_with = step.get("with") or {}
    filters_yaml = step_with.get("filters")
    if not isinstance(filters_yaml, str) or ":" not in filters_yaml:
        return []

    try:
        filters = yaml.safe_load(filters_yaml)
    except yaml.YAMLError as exc:
        return [f"{path}: job {job_name}: invalid dorny filters YAML: {exc}"]

    if not isinstance(filters, dict):
        return [f"{path}: job {job_name}: dorny filters must be a mapping"]

    predicate_quantifier = step_with.get("predicate-quantifier", "some")
    violations: list[str] = []

    for filter_name, filter_item in filters.items():
        patterns = list(iter_patterns(filter_item))
        negated = [pattern for pattern in patterns if pattern.startswith("!")]
        positive = [pattern for pattern in patterns if not pattern.startswith("!")]

        if negated and predicate_quantifier != "every":
            violations.append(
                f"{path}: job {job_name}: filter {filter_name!r} uses negated "
                "pattern(s) without `predicate-quantifier: every`; dorny would "
                "treat the negation as a broad positive match."
            )

        if negated and not positive:
            violations.append(
                f"{path}: job {job_name}: filter {filter_name!r} has negated "
                "pattern(s) but no positive include pattern."
            )

    return violations


def check_workflow(path: Path) -> list[str]:
    """Return dorny path-filter violations for a workflow file."""
    with path.open() as f:
        doc = yaml.safe_load(f)

    if not isinstance(doc, dict):
        return []

    jobs = doc.get("jobs") or {}
    if not isinstance(jobs, dict):
        return []

    violations: list[str] = []
    for job_name, job in jobs.items():
        if not isinstance(job, dict):
            continue
        steps = job.get("steps") or []
        if not isinstance(steps, list):
            continue

        for step in steps:
            if not isinstance(step, dict):
                continue
            uses = step.get("uses")
            if isinstance(uses, str) and uses.startswith(DORNY_ACTION):
                violations.extend(check_dorny_step(path, str(job_name), step))

    return violations


def main() -> int:
    """Run the workflow filter lint."""
    if not WORKFLOWS_DIR.is_dir():
        print(f"error: {WORKFLOWS_DIR} not found", file=sys.stderr)
        return 2

    violations: list[str] = []
    checked = 0
    for path in sorted(WORKFLOWS_DIR.glob("*.yml")):
        checked += 1
        violations.extend(check_workflow(path))

    if violations:
        for violation in violations:
            print(violation, file=sys.stderr)
        print(
            f"\n{len(violations)} path-filter violation(s) across "
            f"{checked} workflow(s).",
            file=sys.stderr,
        )
        return 1

    print(f"[workflow-filter lint] checked {checked} workflow(s), all filters safe.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
