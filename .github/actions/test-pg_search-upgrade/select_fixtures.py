"""Select upgrade fixtures using explicit, inclusive-min/exclusive-max versions."""

import argparse
import json
from pathlib import Path
import re
import shutil


def version(value):
    match = re.fullmatch(r"v?(\d+)\.(\d+)\.(\d+)", value)
    if not match:
        raise ValueError(f"Expected a stable release version, got {value!r}")
    return tuple(map(int, match.groups()))


def select(root, prior):
    catalog = json.loads((root / "fixture-versions.json").read_text())
    directories = {p.name for p in root.iterdir() if p.is_dir() and (p / "setup.sql").exists()}
    if directories != set(catalog):
        raise ValueError(f"Fixture metadata differs from SQL directories: {directories ^ set(catalog)}")
    selected = []
    for name, bounds in sorted(catalog.items()):
        minimum = version(bounds["min_version"]) if "min_version" in bounds else None
        maximum = version(bounds["max_version_exclusive"]) if "max_version_exclusive" in bounds else None
        if minimum and maximum and minimum >= maximum:
            raise ValueError(f"Invalid version interval for {name}")
        if (minimum and prior < minimum) or (maximum and prior >= maximum):
            print(f"Skipping {name}: {bounds['reason']}")
        else:
            selected.append(name)
    return selected


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--prior", required=True)
    parser.add_argument("--destination", type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parent
    names = select(root, version(args.prior))
    for name in names:
        print(f"Including {name}")
        shutil.copytree(root / name, args.destination / name)
