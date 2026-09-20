#!/usr/bin/env python3
"""Materialize the standalone ORM projects and run commands shown in the guides."""

import argparse
import re
from pathlib import Path

TABS = {"drizzle": "Drizzle", "django": "Django", "sqlalchemy": "SQLAlchemy", "rails": "Rails", "efcore": "EF Core"}
FENCES = re.compile(r"^```([^\n]*)\n(.*?)^```[ \t]*$", re.MULTILINE | re.DOTALL)
GUIDES = Path(__file__).resolve().parents[2] / "docs" / "guides"


def extract(orm: str, destination: Path) -> None:
    destination.mkdir(parents=True, exist_ok=True)
    commands = ["#!/usr/bin/env bash", "set -euo pipefail", "unset OPENROUTER_API_KEY"]
    paths = set()
    pages = [GUIDES / "setup.mdx"] + sorted(p for p in GUIDES.glob("*.mdx") if p.stem != "setup")
    for page in pages:
        tab = re.search(rf'<Tab title="{TABS[orm]}">(.*?)</Tab>', page.read_text(), re.DOTALL)
        if tab is None:
            raise ValueError(f"Missing {orm} tab: {page}")
        for info, body in FENCES.findall(tab.group(1)):
            parts = info.split(maxsplit=1)
            if len(parts) == 2:
                path = Path(parts[1])
                if path.is_absolute() or ".." in path.parts or path in paths:
                    raise ValueError(f"Unsafe or duplicate file path: {path}")
                paths.add(path)
                target = destination / path
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_text(body)
            elif parts == ["bash"] and page.stem != "setup":
                commands.extend([f"echo 'Running {page.stem}'", body.rstrip()])
    (destination / "run-guides.sh").write_text("\n".join(commands) + "\n")
    print(f"Extracted {len(paths)} {orm} files into {destination}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("orm", choices=TABS)
    parser.add_argument("destination", type=Path)
    args = parser.parse_args()
    extract(args.orm, args.destination)
