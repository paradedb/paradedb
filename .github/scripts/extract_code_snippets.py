#!/usr/bin/env python3
"""Inventory executable docs fences and extract explicitly covered examples."""

import hashlib
import json
import re
import shutil
import sys
from collections import Counter
from pathlib import Path

CODEGROUP_PATTERN = re.compile(r"<CodeGroup[ >].*?</CodeGroup>", re.DOTALL)
FENCE_PATTERN = re.compile(r"^```([^\n]*)\n(.*?)^```[ \t]*$", re.MULTILINE | re.DOTALL)
TARGET_SUFFIXES = {
    "sql": "sql",
    "django": "py",
    "rails": "rb",
    "sqlalchemy": "py",
    "drizzle": "ts",
    "efcore": "cs",
}
EXECUTABLE_LANGUAGES = {
    "sql",
    "python",
    "ruby",
    "ts",
    "typescript",
    "cs",
    "csharp",
    "bash",
    "sh",
    "yml",
    "yaml",
    "js",
    "javascript",
}
IGNORED_CODEGROUPS = {
    "reference__tokenizers__available-tokenizers__lindera__group-001": (
        "Language alternatives; checked as separate SQL fences "
        "by the standalone inventory."
    ),
    "reference__indexing__indexing-vectors__group-002": (
        "Non-executable distance metric fragments; "
        "covered by the runnable vector index examples."
    ),
}
SCRIPT_DIR = Path(__file__).resolve().parent
DOCS_ROOT = SCRIPT_DIR.parent.parent / "docs"
OUTPUT_ROOT = SCRIPT_DIR / "verify"
COVERAGE_PATH = SCRIPT_DIR / "docs_snippet_coverage.json"


def fence_digest(info, body):
    """Fingerprint a fence's language label and unchanged source."""
    return hashlib.sha256((info + "\n" + body).encode()).hexdigest()


def classify(info):
    """Resolve a labeled application fence to its verification target."""
    parts = set(info.lower().replace("ef core", "efcore").split())
    if info.lower().startswith("sql"):
        return "sql"
    for target, language in (
        ("django", "python"),
        ("sqlalchemy", "python"),
        ("rails", "ruby"),
        ("drizzle", "ts"),
        ("efcore", "cs"),
    ):
        if info.lower().startswith(language) and target in parts:
            return target
    return ""


def prepare_drizzle_snippet(source):
    """Capture complete index-builder expressions so the harness can migrate them."""
    pattern = re.compile(r"^indexing(?=\s*\.paradedbIndex)[\s\S]*?;", re.MULTILINE)

    def capture_index(match):
        expression = match.group()[:-1]
        tables = set(re.findall(r"\b(mockItems|arrayDemo)\.", expression))
        if len(tables) != 1:
            raise ValueError("Drizzle index must reference exactly one fixture table")
        table = {"mockItems": "mock_items", "arrayDemo": "array_demo"}[tables.pop()]
        expression = re.sub(r"\b(?:mockItems|arrayDemo)\.", "docsTable.", expression)
        return (
            f'docsIndexes.push({{ table: "{table}", '
            f"build: (docsTable) => {expression} }});"
        )

    return pattern.sub(capture_index, source)


def codegroup_name(path, index):
    """Build a stable identity for a page CodeGroup."""
    return path.with_suffix("").as_posix().replace("/", "__") + f"__group-{index:03d}"


def extract_snippets(codegroup):
    """Require exactly one executable example for each supported target."""
    snippets = {}
    for info, body in FENCE_PATTERN.findall(codegroup):
        target = classify(info)
        if not target:
            raise ValueError(f"Unrecognized CodeGroup language: {info}")
        if target in snippets:
            raise ValueError(f"Duplicate {target} fence in CodeGroup")
        snippets[target] = body.rstrip("\n") + "\n"
    missing = set(TARGET_SUFFIXES) - snippets.keys()
    if missing:
        raise ValueError(f"CodeGroup is missing variants: {', '.join(sorted(missing))}")
    return snippets


def inventory(docs_root=DOCS_ROOT):
    """Give every relevant fence an identity, including Tabs and standalone SQL."""
    groups, outside = {}, {}
    for doc in sorted(docs_root.rglob("*.mdx")):
        rel = doc.relative_to(docs_root)
        if rel.parts[:2] == ("project", "changelog"):
            continue  # Historical API examples are not current-client contracts.
        text = doc.read_text()
        spans = list(CODEGROUP_PATTERN.finditer(text))
        for index, group in enumerate(spans, 1):
            groups[codegroup_name(rel, index)] = group.group()
        for index, fence in enumerate(FENCE_PATTERN.finditer(text), 1):
            enclosing = next(
                (
                    i
                    for i, group in enumerate(spans, 1)
                    if group.start() <= fence.start() < group.end()
                ),
                None,
            )
            if (
                enclosing
                and codegroup_name(rel, enclosing) not in IGNORED_CODEGROUPS
                and any(
                    classify(info)
                    for info, _ in FENCE_PATTERN.findall(spans[enclosing - 1].group())
                )
            ):
                continue
            info, body = fence.groups()
            if info.split()[0].lower() not in EXECUTABLE_LANGUAGES:
                continue
            key = f"{rel.as_posix()}::fence-{index:03d}"
            outside[key] = {
                "info": info,
                "body": body,
                "sha256": fence_digest(info, body),
            }
    return groups, outside


def skipped_coverage(outside, groups):
    """Require a reason and unchanged source for each unexecuted exception."""
    coverage = {}
    for group in groups:
        if not group["reason"]:
            raise ValueError("Skipped snippets need a reason")
        for key, digest in group["fences"]:
            if key not in outside or outside[key]["sha256"] != digest:
                raise ValueError(f"Skipped snippet changed or disappeared: {key}")
            if key in coverage:
                raise ValueError(f"Duplicate skipped snippet: {key}")
            coverage[key] = {"mode": "skip"}
    return coverage


def resolve_coverage(outside, exceptions):
    """Run ordinary SQL automatically; require reviewed exceptions for other fences.

    Only skips retain digests: editing an unexecuted example requires a new review.
    Setup fences are checked against the tutorial runner's consumed source blocks.
    """
    coverage = skipped_coverage(outside, exceptions["skips"])
    for key, fence in outside.items():
        if key in coverage:
            continue
        if key.startswith("start/configure-your-environment.mdx::"):
            mode = "setup"
        elif fence["info"].split()[0].lower() == "sql":
            mode = "sql"
        else:
            raise ValueError(f"Unclassified non-SQL snippet: {key}")
        coverage[key] = {"mode": mode}
    for key, entry in exceptions["sql"].items():
        if key not in coverage or coverage[key]["mode"] != "sql":
            raise ValueError(f"Unknown SQL scenario: {key}")
        for dependency in entry.get("setup", []):
            if dependency not in outside:
                raise ValueError(f"Unknown scenario fixture {dependency} for {key}")
        coverage[key].update(entry)
    return coverage


def write_standalone_snippets(outputs, outside, coverage):
    """Emit SQL scenarios with their prerequisites and expected warnings."""
    count = 0
    for key, entry in coverage.items():
        if entry["mode"] != "sql":
            continue
        body = (
            "\n".join(
                outside[dependency]["body"] for dependency in entry.get("setup", [])
            )
            + "\n"
            + outside[key]["body"]
        )
        # Standalone SQL runs with isolated fixtures, not whatever the previous example left behind.
        prefix = f"\\i {SCRIPT_DIR / 'bootstrap_code_snippet_tables.sql'}\n"
        if "CREATE INDEX" not in body:
            prefix += f"\\i {SCRIPT_DIR / 'create_code_snippet_indexes.sql'}\n"
        filename = key.replace("/", "__").replace(".mdx::", "__") + ".sql"
        (outputs["sql"] / filename).write_text(prefix + body + "\n")
        if entry.get("warning"):
            (outputs["sql"] / filename).with_suffix(".warning").write_text(
                entry["warning"] + "\n"
            )
        count += 1
    return count


def main():
    """Validate coverage and emit snippets for the smoke-test harnesses."""
    groups, outside = inventory()
    if not groups:
        raise ValueError("No documentation CodeGroups found")
    coverage = resolve_coverage(outside, json.loads(COVERAGE_PATH.read_text()))
    outputs = {target: OUTPUT_ROOT / target for target in TARGET_SUFFIXES}
    for path in outputs.values():
        shutil.rmtree(path, ignore_errors=True)
        path.mkdir(parents=True, exist_ok=True)
    counts = Counter()
    for name, group in groups.items():
        if name in IGNORED_CODEGROUPS or not any(
            classify(info) for info, _ in FENCE_PATTERN.findall(group)
        ):
            continue
        try:
            snippets = extract_snippets(group)
        except ValueError as error:
            raise ValueError(f"{name}: {error}") from error
        for target, body in snippets.items():
            if (
                target == "sql"
                and name == "reference__filtering__external-indexes__group-001"
            ):
                body = (
                    f"\\i {SCRIPT_DIR / 'bootstrap_code_snippet_tables.sql'}\n"
                    + outside["reference/filtering/external-indexes.mdx::fence-001"][
                        "body"
                    ]
                    + body
                )
            if target == "drizzle":
                body = prepare_drizzle_snippet(body)
            (outputs[target] / f"{name}.{TARGET_SUFFIXES[target]}").write_text(body)
            counts[target] += 1
    counts["sql"] += write_standalone_snippets(outputs, outside, coverage)
    for target in TARGET_SUFFIXES:
        if not counts[target]:
            raise ValueError(f"No snippets extracted for {target}")
    print(
        "Documentation snippets: "
        + ", ".join(f"{target}={counts[target]}" for target in TARGET_SUFFIXES)
    )
    print(
        f"Standalone inventory: {Counter(entry['mode'] for entry in coverage.values())}"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except ValueError as error:
        print(error, file=sys.stderr)
        raise SystemExit(1) from error
