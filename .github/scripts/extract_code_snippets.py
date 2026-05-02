#!/usr/bin/env python3
"""Extract verification snippets from docs CodeGroups."""

from pathlib import Path
import re
import shutil
import sys


CODEGROUP_PATTERN = re.compile(r"<CodeGroup[ >].*?</CodeGroup>", re.S)
FENCE_PATTERN = re.compile(r"^```([^\n]*)\n(.*?)^```[ \t]*$", re.M | re.S)
TARGET_SUFFIXES = {
    "sql": "sql",
    "django": "py",
    "rails": "rb",
    "sqlalchemy": "py",
}
IGNORED_CODEGROUPS = {
    # CodeGroup is used here to switch between Chinese, Korean, and Japanese
    # not SQL vs ORMs
    "documentation__tokenizers__available-tokenizers__lindera__group-001",
}
SCRIPT_DIR = Path(__file__).resolve().parent
REPO_ROOT = SCRIPT_DIR.parent.parent
DOCS_ROOT = REPO_ROOT / "docs"
OUTPUT_ROOT = SCRIPT_DIR / "verify"


def classify(info: str) -> str:
    """Map a fenced code block info string to a verification target."""
    info = info.strip().lower()
    parts = set(info.split())

    if info.startswith("sql"):
        return "sql"
    if info.startswith("python") and "django" in parts:
        return "django"
    if info.startswith("ruby") and "rails" in parts:
        return "rails"
    if info.startswith("python") and "sqlalchemy" in parts:
        return "sqlalchemy"
    return ""


def codegroup_name(path: Path, group_index: int) -> str:
    """Return the stable verification name for a CodeGroup."""
    stem = path.with_suffix("").as_posix().replace("/", "__")
    return f"{stem}__group-{group_index:03d}"


def extract_snippets(codegroup: str) -> dict[str, str]:
    """Extract supported snippets from a CodeGroup body."""
    snippets = {}

    for info, body in FENCE_PATTERN.findall(codegroup):
        target = classify(info)
        if not target:
            continue
        snippets[target] = body.rstrip("\n") + "\n"

    return snippets


def build_output_dirs() -> dict[str, Path]:
    """Create and reset output directories for each supported target."""
    output_dirs = {target: OUTPUT_ROOT / target for target in TARGET_SUFFIXES}
    for path in output_dirs.values():
        shutil.rmtree(path, ignore_errors=True)
        path.mkdir(parents=True, exist_ok=True)
    return output_dirs


def process_doc(doc: Path, output_dirs: dict[str, Path]) -> None:
    """Extract supported snippets from one doc and write them to disk."""
    rel_path = doc.relative_to(DOCS_ROOT)
    text = doc.read_text()

    for group_index, codegroup in enumerate(CODEGROUP_PATTERN.findall(text), start=1):
        group_name = codegroup_name(rel_path, group_index)
        if group_name in IGNORED_CODEGROUPS:
            continue

        snippets = extract_snippets(codegroup)
        for target, suffix in TARGET_SUFFIXES.items():
            if target not in snippets:
                continue
            snippet_path = output_dirs[target] / f"{group_name}.{suffix}"
            snippet_path.write_text(snippets[target])


def main() -> int:
    """Extract all supported verification snippets from the docs tree."""
    output_dirs = build_output_dirs()

    docs = sorted(
        path for path in DOCS_ROOT.rglob("*.mdx") if "legacy" not in path.parts
    )
    if not docs:
        print(f"No .mdx files found under {DOCS_ROOT}", file=sys.stderr)
        return 1

    for doc in docs:
        process_doc(doc, output_dirs)

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
