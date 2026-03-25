#!/usr/bin/env python3

from pathlib import Path
import re
import shutil
import sys


CODEGROUP_PATTERN = re.compile(r"<CodeGroup[ >].*?</CodeGroup>", re.S)
FENCE_PATTERN = re.compile(r"^```([^\n]*)\n(.*?)^```[ \t]*$", re.M | re.S)


def classify(info: str) -> str:
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
    return path.with_suffix("").as_posix().replace("/", "__")


def reset_dir(path: Path) -> None:
    shutil.rmtree(path, ignore_errors=True)
    path.mkdir(parents=True, exist_ok=True)


def main() -> int:
    script_dir = Path(__file__).resolve().parent
    docs_root = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else script_dir.parent
    output_root = (
        Path(sys.argv[2]).resolve() if len(sys.argv) > 2 else docs_root / "verify"
    )
    sql_dir = output_root / "sql"
    rails_dir = output_root / "rails"
    sqlalchemy_dir = output_root / "sqlalchemy"

    reset_dir(sql_dir)
    reset_dir(rails_dir)
    reset_dir(sqlalchemy_dir)

    docs = sorted(
        path for path in docs_root.rglob("*.mdx") if "legacy" not in path.parts
    )
    if not docs:
        print(f"No .mdx files found under {docs_root}", file=sys.stderr)
        return 1

    counts = {"sql": 0, "rails": 0, "sqlalchemy": 0}

    for doc in docs:
        rel_path = doc.relative_to(docs_root)
        text = doc.read_text()

        for group_index, codegroup in enumerate(
            CODEGROUP_PATTERN.findall(text), start=1
        ):
            snippets = {}

            for info, body in FENCE_PATTERN.findall(codegroup):
                target = classify(info)
                if not target:
                    continue
                snippets[target] = body.rstrip("\n") + "\n"

            if "sql" in snippets:
                snippet_path = (
                    sql_dir / f"{stem(rel_path)}__group-{group_index:03d}.sql"
                )
                snippet_path.write_text(snippets["sql"])
                counts["sql"] += 1

            if "rails" in snippets:
                snippet_path = (
                    rails_dir / f"{stem(rel_path)}__group-{group_index:03d}.rb"
                )
                snippet_path.write_text(snippets["rails"])
                counts["rails"] += 1

            if "sqlalchemy" in snippets:
                snippet_path = (
                    sqlalchemy_dir / f"{stem(rel_path)}__group-{group_index:03d}.py"
                )
                snippet_path.write_text(snippets["sqlalchemy"])
                counts["sqlalchemy"] += 1

    print(f"Wrote {counts['sql']} SQL snippets to {sql_dir}")
    print(f"Wrote {counts['rails']} Rails snippets to {rails_dir}")
    print(f"Wrote {counts['sqlalchemy']} SQLAlchemy snippets to {sqlalchemy_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
