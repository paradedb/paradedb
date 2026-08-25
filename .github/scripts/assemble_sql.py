#!/usr/bin/env python3
# pylint: disable=duplicate-code
"""scripts/assemble_sql.py.

Assembles unreleased SQL migration fragments from pg_search/sql/unreleased/
into a versioned migration file (pg_search/sql/pg_search--<prev>--<target>.sql).
"""

import argparse
import re
import subprocess
import sys
from pathlib import Path


def detect_target_version(repo_root, explicit_version):
    """Resolve target version from CLI args or Cargo.toml."""
    if explicit_version:
        return explicit_version

    cargo_toml = repo_root / "Cargo.toml"
    if cargo_toml.exists():
        with open(cargo_toml, "r", encoding="utf-8") as f:
            for line in f:
                if line.startswith("version ="):
                    return line.split("=")[1].strip().strip("\"'")

    print(
        "❌ Error: Target version not specified and not found in Cargo.toml",
        file=sys.stderr,
    )
    sys.exit(1)


def parse_semver(ver_str):
    """Parse semver string into a tuple of ints for sorting."""
    match = re.match(r"^v?(\d+)\.(\d+)\.(\d+)", ver_str)
    if match:
        return tuple(int(x) for x in match.groups())
    return (0, 0, 0)


def resolve_prev_from_git_tags(repo_root, clean_target):
    """Find the highest git tag strictly less than clean_target."""
    try:
        cmd = ["git", "-C", str(repo_root), "tag", "-l", "v*"]
        result = subprocess.run(cmd, capture_output=True, text=True, check=True)
        tags = [
            t.strip()
            for t in result.stdout.splitlines()
            if re.match(r"^v\d+\.\d+\.\d+$", t.strip())
        ]
        target_tuple = parse_semver(clean_target)
        valid_tags = [t for t in tags if parse_semver(t) < target_tuple]
        if valid_tags:
            valid_tags.sort(key=parse_semver)
            return valid_tags[-1].lstrip("v")
    except (subprocess.SubprocessError, OSError):
        pass
    return None


def resolve_prev_from_sql_files(sql_dir, clean_target):
    """Find highest target version from existing pg_search--*--*.sql files."""
    target_tuple = parse_semver(clean_target)
    found_versions = []

    for fpath in sql_dir.glob("pg_search--*--*.sql"):
        match = re.match(r"^pg_search--.*?--(\d+\.\d+\.\d+)\.sql$", fpath.name)
        if match:
            ver_str = match.group(1)
            if parse_semver(ver_str) < target_tuple:
                found_versions.append(ver_str)

    if found_versions:
        found_versions.sort(key=parse_semver)
        return found_versions[-1]
    return None


def resolve_prev_version(repo_root, sql_dir, clean_target, explicit_prev):
    """Resolve previous version from args, git tags, or migration files."""
    if explicit_prev:
        return explicit_prev

    prev = resolve_prev_from_git_tags(repo_root, clean_target)
    if not prev:
        prev = resolve_prev_from_sql_files(sql_dir, clean_target)

    if not prev:
        print(
            f"❌ Error: Could not determine previous version for {clean_target}.",
            file=sys.stderr,
        )
        sys.exit(1)
    return prev


def parse_fragment_order(filename):
    """Extract PR number or fallback sort key from fragment filename."""
    match = re.match(r"^(\d+)\.", filename)
    if match:
        return (0, int(match.group(1)), filename)
    return (1, 0, filename)


def collect_sql_fragments(unreleased_dir):
    """Find and sort unreleased SQL fragment files."""
    files = [
        p for p in unreleased_dir.glob("*.sql") if p.is_file() and p.name != ".gitkeep"
    ]
    files.sort(key=lambda p: parse_fragment_order(p.name))
    return files


def render_sql_content(clean_target, fragments):
    """Render SQL migration script content with banners."""
    banner = (
        f"\\echo Use \"ALTER EXTENSION pg_search UPDATE TO '{clean_target}'\" "
        "to load this file. \\quit"
    )
    lines = [
        banner,
        "",
    ]
    for fpath in fragments:
        lines.append(f"-- {fpath.name}")
        with open(fpath, "r", encoding="utf-8") as f:
            lines.append(f.read().strip())
        lines.append("")
        lines.append("")
    return "\n".join(lines)


def assemble_sql_files(repo_root, clean_target, prev_version, transient):
    """Assemble SQL upgrade script and handle fragment deletion."""
    sql_dir = repo_root / "pg_search" / "sql"
    unreleased_dir = sql_dir / "unreleased"
    output_file = sql_dir / f"pg_search--{prev_version}--{clean_target}.sql"

    print(
        f"Assembling SQL upgrade script: {output_file} "
        f"(from {prev_version} to {clean_target})"
    )

    fragments = collect_sql_fragments(unreleased_dir)
    print(f"Found {len(fragments)} SQL fragment(s):")
    for f in fragments:
        print(f"  - {f.name}")

    content = render_sql_content(clean_target, fragments)
    with open(output_file, "w", encoding="utf-8") as f:
        f.write(content)
    print(f"✅ Successfully generated: {output_file}")

    if transient:
        print(f"Transient mode: fragments preserved in {unreleased_dir}.")
    else:
        for fpath in fragments:
            print(f"Removing consumed fragment: {fpath.name}")
            fpath.unlink()


def main():
    """Main execution function for assemble_sql."""
    parser = argparse.ArgumentParser(description="Assemble SQL migration fragments.")
    parser.add_argument(
        "version", nargs="?", help="Target release version (e.g. 0.25.5)"
    )
    parser.add_argument(
        "--prev-version", default=None, help="Previous version (e.g. 0.25.4)"
    )
    parser.add_argument(
        "--transient",
        action="store_true",
        help="Preserve fragments (used by PR upgrade CI)",
    )
    parser.add_argument(
        "--repo-root", default=None, help="Root directory of repository"
    )
    args = parser.parse_args()

    default_root = Path(__file__).resolve().parent.parent.parent
    repo_root = Path(args.repo_root or default_root)
    target_version = detect_target_version(repo_root, args.version)
    clean_target = target_version.split("-")[0].lstrip("v")

    sql_dir = repo_root / "pg_search" / "sql"
    prev_version = resolve_prev_version(
        repo_root, sql_dir, clean_target, args.prev_version
    )

    assemble_sql_files(repo_root, clean_target, prev_version, args.transient)


if __name__ == "__main__":
    main()
