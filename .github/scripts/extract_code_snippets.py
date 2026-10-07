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


def classify(info):
    """Resolve a labeled application fence to its verification target."""
    parts = set(info.lower().split())
    if info.lower().startswith("sql"):
        return "sql"
    for target, language, label in (
        ("django", "python", "django"),
        ("sqlalchemy", "python", "sqlalchemy"),
        ("rails", "ruby", "rails"),
        ("drizzle", "ts", "drizzle"),
        ("efcore", "cs", "core"),
    ):
        if info.lower().startswith(language) and (label in parts or target == "efcore"):
            return target
    return ""


def prepare_drizzle_snippet(source):
    """Capture complete index-builder expressions so the harness can migrate them."""
    pattern = re.compile(r"^indexing(?=\s*\.paradedbIndex)[\s\S]*?;", re.MULTILINE)
    return pattern.sub(
        lambda match: (
            "docsIndexes.push((docsTable) => "
            + re.sub(r"\b(?:mockItems|arrayDemo)\.", "docsTable.", match.group()[:-1])
            + ");"
        ),
        source,
    )


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
                "sha256": hashlib.sha256((info + "\n" + body).encode()).hexdigest(),
            }
    return groups, outside


def validate_coverage(outside, coverage):
    """Reject unreviewed fences, changed decisions and unknown fixtures."""
    missing = outside.keys() - coverage.keys()
    stale = coverage.keys() - outside.keys()
    if missing or stale:
        raise ValueError(
            f"Unclassified snippets: {sorted(missing)}; stale inventory entries: {sorted(stale)}"
        )
    for key, entry in coverage.items():
        if entry.get("mode") not in {"sql", "setup", "skip"}:
            raise ValueError(f"Invalid coverage mode for {key}")
        if not entry.get("reason"):
            raise ValueError(f"Coverage decision needs a reason: {key}")
        if entry.get("sha256") != outside[key]["sha256"]:
            raise ValueError(
                f"Snippet changed; review its coverage decision and digest: {key}"
            )
        if entry["mode"] == "setup" and not key.startswith(
            "start/configure-your-environment.mdx::"
        ):
            raise ValueError(f"No setup scenario registered for {key}")
        for dependency in entry.get("setup", []):
            if dependency not in outside:
                raise ValueError(f"Unknown scenario fixture {dependency} for {key}")


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
    if len(sys.argv) > 1:
        if len(sys.argv) != 3 or sys.argv[1] != "--prepare-drizzle":
            raise ValueError("Usage: extract_code_snippets.py [--prepare-drizzle FILE]")
        print(prepare_drizzle_snippet(Path(sys.argv[2]).read_text(encoding="utf-8")))
        return 0
    groups, outside = inventory()
    if not groups:
        raise ValueError("No documentation CodeGroups found")
    coverage = json.loads(COVERAGE_PATH.read_text())
    validate_coverage(outside, coverage)
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
