"""Require immutable references in workflows and local action manifests."""

import re
import sys
from pathlib import Path

import yaml


def check_document(document):
    """Return mutable action references, including nested/inline YAML mappings."""
    errors = []
    visited = set()

    def visit(value):
        if not isinstance(value, (dict, list)) or id(value) in visited:
            return
        visited.add(id(value))
        if isinstance(value, dict):
            if "uses" in value:
                ref = value["uses"]
                if not isinstance(ref, str) or not (
                    ref.startswith("./")
                    or re.fullmatch(r"[\w.-]+/[\w./-]+@[a-fA-F0-9]{40}", ref)
                    or re.fullmatch(r"docker://[^\s]+@sha256:[a-fA-F0-9]{64}", ref)
                ):
                    errors.append(str(ref))
            for child in value.values():
                visit(child)
        else:
            for child in value:
                visit(child)

    visit(document)
    return errors


def main():
    """Scan workflow and composite-action YAML, failing on mutable references."""
    root = Path(__file__).resolve().parents[2]
    files = set((root / ".github/workflows").glob("*.yml"))
    files.update((root / ".github/workflows").glob("*.yaml"))
    for name in ("action.yml", "action.yaml"):
        files.update((root / ".github/actions").rglob(name))
    failed = False
    for path in sorted(files):
        for ref in check_document(yaml.safe_load(path.read_text(encoding="utf-8"))):
            print(f"{path.relative_to(root)}: immutable SHA required for uses: {ref}")
            failed = True
    return int(failed)


if __name__ == "__main__":
    sys.exit(main())
