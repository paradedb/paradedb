#!/usr/bin/env python3
"""Extract verification snippets from docs CodeGroups."""

from pathlib import Path
import re
import shutil
import sys


CODEGROUP_PATTERN = re.compile(r"<CodeGroup[ >].*?</CodeGroup>", re.S)
FENCE_PATTERN = re.compile(r"^```([^\n]*)\n(.*?)^```[ \t]*$", re.M | re.S)
IGNORED_CODEGROUPS = {
    "documentation__tokenizers__available-tokenizers__lindera__group-001",
    # Once https://github.com/paradedb/paradedb/issues/4456 is fixed,
    # we should unignore this snippet.
    "documentation__aggregates__overview__group-003",
}


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


def stem(path: Path) -> str:
    """Convert a docs path into the verification filename stem."""
    return path.with_suffix("").as_posix().replace("/", "__")


def reset_dir(path: Path) -> None:
    """Recreate an output directory from scratch."""
    shutil.rmtree(path, ignore_errors=True)
    path.mkdir(parents=True, exist_ok=True)


def codegroup_name(path: Path, group_index: int) -> str:
    """Return the stable verification name for a CodeGroup."""
    return f"{stem(path)}__group-{group_index:03d}"


def parse_args() -> tuple[Path, Path]:
    """Resolve the docs root and output directory from CLI arguments."""
    script_dir = Path(__file__).resolve().parent
    repo_root = script_dir.parent.parent
    docs_root = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else repo_root / "docs"
    output_root = (
        Path(sys.argv[2]).resolve() if len(sys.argv) > 2 else script_dir / "verify"
    )
    return docs_root, output_root


def extract_snippets(codegroup: str) -> dict[str, str]:
    """Extract supported snippets from a CodeGroup body."""
    snippets = {}

    for info, body in FENCE_PATTERN.findall(codegroup):
        target = classify(info)
        if not target:
            continue
        snippets[target] = body.rstrip("\n") + "\n"

    return snippets


def write_snippets(
    group_name: str,
    snippets: dict[str, str],
    output_dirs: dict[str, Path],
    counts: dict[str, int],
) -> None:
    """Write extracted snippets to their target verification directories."""
    suffixes = {"sql": "sql", "django": "py", "rails": "rb", "sqlalchemy": "py"}

    for target, suffix in suffixes.items():
        if target not in snippets:
            continue
        snippet_path = output_dirs[target] / f"{group_name}.{suffix}"
        snippet_path.write_text(snippets[target])
        counts[target] += 1


def build_output_dirs(output_root: Path) -> dict[str, Path]:
    """Create and reset output directories for each supported target."""
    output_dirs = {
        "sql": output_root / "sql",
        "django": output_root / "django",
        "rails": output_root / "rails",
        "sqlalchemy": output_root / "sqlalchemy",
    }
    for path in output_dirs.values():
        reset_dir(path)
    return output_dirs


def main() -> int:
    """Extract all supported verification snippets from the docs tree."""
    docs_root, output_root = parse_args()
    output_dirs = build_output_dirs(output_root)

    docs = sorted(
        path for path in docs_root.rglob("*.mdx") if "legacy" not in path.parts
    )
    if not docs:
        print(f"No .mdx files found under {docs_root}", file=sys.stderr)
        return 1

    counts = {"sql": 0, "django": 0, "rails": 0, "sqlalchemy": 0}

    for doc in docs:
        rel_path = doc.relative_to(docs_root)
        text = doc.read_text()

        for group_index, codegroup in enumerate(
            CODEGROUP_PATTERN.findall(text), start=1
        ):
            group_name = codegroup_name(rel_path, group_index)
            if group_name in IGNORED_CODEGROUPS:
                continue

            snippets = extract_snippets(codegroup)
            write_snippets(group_name, snippets, output_dirs, counts)

    print(f"Wrote {counts['sql']} SQL snippets to {output_dirs['sql']}")
    print(f"Wrote {counts['django']} Django snippets to {output_dirs['django']}")
    print(f"Wrote {counts['rails']} Rails snippets to {output_dirs['rails']}")
    print(
        f"Wrote {counts['sqlalchemy']} SQLAlchemy snippets "
        f"to {output_dirs['sqlalchemy']}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
