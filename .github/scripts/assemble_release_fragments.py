#!/usr/bin/env python3
"""scripts/assemble_release_fragments.py.

Unified release artifact assembler for ParadeDB:
- Assembles unreleased SQL migration fragments into pg_search--<prev>--<target>.sql
- Assembles unreleased changelog fragments into docs/changelog/<version>.mdx
- Registers new versions in docs/docs.json and docs/snippets/version.mdx
"""

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path
from textwrap import dedent


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


def clean_version(version_str):
    """Strip leading v and prerelease suffix (-rc.*)."""
    return version_str.split("-")[0].lstrip("v")


# ==============================================================================
# SQL Migration Assembly
# ==============================================================================


def get_git_tags():
    """Retrieve all semver git tags in the repo."""
    try:
        res = subprocess.run(
            ["git", "tag", "-l", "v*"],
            capture_output=True,
            text=True,
            check=True,
        )
        tags = []
        for line in res.stdout.strip().splitlines():
            line = line.strip()
            if re.match(r"^v\d+\.\d+\.\d+$", line):
                tags.append(line.lstrip("v"))
        return tags
    except (subprocess.SubprocessError, FileNotFoundError):
        return []


def get_existing_sql_targets(sql_dir):
    """Find all target versions from existing pg_search--*--<target>.sql files."""
    targets = []
    for file in sql_dir.glob("pg_search--*--*.sql"):
        match = re.match(r"^pg_search--.+--(\d+\.\d+\.\d+)\.sql$", file.name)
        if match:
            targets.append(match.group(1))
    return targets


def resolve_prev_version(repo_root, sql_dir, target_version, explicit_prev):
    """Determine the predecessor version to upgrade from."""
    if explicit_prev:
        return clean_version(explicit_prev)

    target_tuple = parse_semver(target_version)
    candidates = set(get_git_tags())
    candidates.update(get_existing_sql_targets(sql_dir))

    valid = [v for v in candidates if parse_semver(v) < target_tuple]
    if valid:
        valid.sort(key=parse_semver)
        return valid[-1]

    cargo_toml = repo_root / "Cargo.toml"
    if cargo_toml.exists():
        with open(cargo_toml, "r", encoding="utf-8") as f:
            for line in f:
                if line.startswith("version ="):
                    return clean_version(line.split("=")[1].strip().strip("\"'"))

    print(
        f"❌ Error: Could not determine previous version for target {target_version}",
        file=sys.stderr,
    )
    sys.exit(1)


def parse_pr_number(filename):
    """Extract PR number integer from fragment filename prefix."""
    match = re.match(r"^(\d+)\.", filename)
    if match:
        return int(match.group(1))
    return 0


def collect_sql_fragments(unreleased_dir):
    """Collect all *.sql fragments in unreleased dir sorted by PR number."""
    if not unreleased_dir.exists():
        return []
    fragments = [
        f for f in unreleased_dir.glob("*.sql") if f.is_file() and f.name != ".gitkeep"
    ]
    fragments.sort(key=lambda f: (parse_pr_number(f.name), f.name))
    return fragments


def format_sql_banner(filename):
    """Generate section banner for an assembled SQL fragment."""
    sep = "-- " + "=" * 76
    return f"\n{sep}\n-- Fragment: {filename}\n{sep}\n"


def assemble_sql_files(repo_root, clean_target, prev_version, preserve_fragments):
    """Assemble SQL fragments into pg_search--<prev>--<target>.sql."""
    sql_dir = repo_root / "pg_search" / "sql"
    unreleased_dir = sql_dir / "unreleased"
    output_file = sql_dir / f"pg_search--{prev_version}--{clean_target}.sql"

    print(
        f"Assembling SQL upgrade script: {output_file} "
        f"(from {prev_version} to {clean_target})"
    )

    fragments = collect_sql_fragments(unreleased_dir)
    print(f"Found {len(fragments)} SQL fragment(s):")
    for fragment in fragments:
        print(f"  - {fragment.name}")

    with open(output_file, "w", encoding="utf-8") as out:
        echo_header = (
            f"\\echo Use \"ALTER EXTENSION pg_search UPDATE TO '{clean_target}'\" "
            f"to load this file. \\quit\n"
        )
        out.write(echo_header)
        for fragment in fragments:
            out.write(format_sql_banner(fragment.name))
            with open(fragment, "r", encoding="utf-8") as fin:
                out.write(fin.read())
            out.write("\n")

    print(f"✅ Successfully generated: {output_file}")

    if not preserve_fragments:
        for fragment in fragments:
            print(f"Removing consumed fragment: {fragment.name}")
            fragment.unlink()
    else:
        print(f"Preserved unreleased SQL fragment(s) in {unreleased_dir}.")


# ==============================================================================
# Changelog Assembly
# ==============================================================================


def load_headers_map(json_path):
    """Load category header mappings from .changelog_headers.json."""
    if not json_path.exists():
        print(
            f"❌ Error: Required headers config file {json_path} not found.",
            file=sys.stderr,
        )
        sys.exit(1)
    try:
        with open(json_path, "r", encoding="utf-8") as f:
            return json.load(f)
    except (json.JSONDecodeError, OSError) as exc:
        print(
            f"❌ Error: Failed to parse {json_path}: {exc}",
            file=sys.stderr,
        )
        sys.exit(1)


def parse_frontmatter(content):
    """Parse YAML frontmatter header key and return (header, body)."""
    match = re.match(r"^---\s*\n(.*?)\n---\s*\n?(.*)$", content, re.DOTALL)
    if not match:
        return None, content.strip()

    frontmatter, body = match.group(1), match.group(2).strip()
    for line in frontmatter.splitlines():
        line = line.strip()
        if line.startswith("header:"):
            val = line.split(":", 1)[1].strip().strip("\"'")
            return val, body
    return None, body


def collect_changelog_fragments(unreleased_dir, headers_map):
    """Read all unreleased mdx fragments and group them by header."""
    if not unreleased_dir.exists():
        return [], {}, []

    files = [
        f for f in unreleased_dir.glob("*.mdx") if f.is_file() and f.name != ".gitkeep"
    ]
    files.sort(key=lambda f: (parse_pr_number(f.name), f.name))

    grouped = {h: [] for h in headers_map}
    extras = []

    for fpath in files:
        with open(fpath, "r", encoding="utf-8") as f:
            header, body = parse_frontmatter(f.read())
        if header and header in grouped:
            grouped[header].append(body)
        elif header:
            extras.append((f"## {header.title()}", body))
        else:
            fallback_key = next(iter(headers_map))
            grouped[fallback_key].append(body)

    return files, grouped, extras


def render_changelog(version, headers_map, grouped, extras):
    """Render the full MDX changelog document."""
    release_url = f"https://github.com/paradedb/paradedb/releases/tag/v{version}"
    lines = [
        "---",
        f'title: "{version}"',
        f'description: "ParadeDB release notes for {version}"',
        "---",
        "",
        f"See GitHub release: [v{version}]({release_url})",
        "",
    ]

    has_content = False
    for key, header_title in headers_map.items():
        items = grouped.get(key, [])
        if items:
            has_content = True
            lines.append(header_title)
            lines.append("")
            for item in items:
                lines.append(format_changelog_item(item))
                lines.append("")

    for title, body in extras:
        has_content = True
        lines.append(title)
        lines.append("")
        lines.append(format_changelog_item(body))
        lines.append("")

    if not has_content:
        lines.append("## Changes")
        lines.append("")
        lines.append(f"- Maintenance and internal updates for {version}.")
        lines.append("")

    return "\n".join(lines)


def format_changelog_item(item_str):
    """Ensure changelog item starts with a bullet point."""
    formatted_lines = []
    for i, line in enumerate(item_str.strip().splitlines()):
        if i == 0 and not line.strip().startswith("-"):
            formatted_lines.append(f"- {line}")
        else:
            formatted_lines.append(line)
    return "\n".join(formatted_lines)


def semver_key(page_str):
    """Extract a semver tuple from a changelog page path for sorting."""
    ver_part = page_str.split("/")[-1]
    match = re.match(r"^v?(\d+)\.(\d+)\.(\d+)", ver_part)
    if match:
        return tuple(int(x) for x in match.groups())
    return (0, 0, 0)


def insert_changelog_into_pages(docs_data, target_page):
    """Find the Changelog group, insert target_page if absent, and sort descending."""
    versions = docs_data.get("navigation", {}).get("versions", [])
    for ver_obj in versions:
        for anchor in ver_obj.get("anchors", []):
            if anchor.get("anchor") != "Changelog":
                continue
            for group in anchor.get("groups", []):
                if group.get("group") == "Changelog":
                    pages = group.get("pages", [])
                    if target_page not in pages:
                        pages.append(target_page)
                        pages.sort(key=semver_key, reverse=True)
                        return True
    return False


def update_navigation_version(docs_data, new_version):
    """Update top-level active version in navigation.versions[0]."""
    versions = docs_data.get("navigation", {}).get("versions", [])
    if versions and isinstance(versions[0], dict):
        versions[0]["version"] = f"v{new_version}"


def update_docs_json(docs_json_path, new_version, is_latest=True):
    """Insert changelog/<new_version> and optionally update active version in docs/docs.json."""
    try:
        with open(docs_json_path, "r", encoding="utf-8") as f:
            docs_data = json.load(f)

        target_page = f"changelog/{new_version}"
        updated_page = insert_changelog_into_pages(docs_data, target_page)
        if is_latest:
            update_navigation_version(docs_data, new_version)

        with open(docs_json_path, "w", encoding="utf-8") as f:
            json.dump(docs_data, f, indent=2)
            f.write("\n")

        if updated_page:
            if is_latest:
                print(
                    f"✅ Added '{target_page}' and set version to "
                    f"'v{new_version}' in {docs_json_path}"
                )
            else:
                print(f"✅ Added '{target_page}' to {docs_json_path}")
        elif is_latest:
            print(f"ℹ️ Set version to 'v{new_version}' in {docs_json_path}")
        return True
    except (json.JSONDecodeError, KeyError, OSError, TypeError) as exc:
        print(
            f"⚠️ Warning: Could not update docs.json automatically: {exc}",
            file=sys.stderr,
        )
        return False


def update_version_snippet(repo_root, clean_ver):
    """Update exported version variable in docs/snippets/version.mdx."""
    snippet_file = repo_root / "docs" / "snippets" / "version.mdx"
    snippet_file.parent.mkdir(parents=True, exist_ok=True)
    content = dedent(
        f"""\
        // This snippet exports the latest released version of ParadeDB for the documentation site.
        // Do not edit manually during development: Cargo.toml tracks the unreleased development version,
        // while this file is updated automatically by assemble_release_fragments.py upon release.
        export const version = "{clean_ver}";
        """
    )
    with open(snippet_file, "w", encoding="utf-8") as f:
        f.write(content)
    print(f"✅ Updated {snippet_file} with version '{clean_ver}'")


def assemble_changelog_files(
    repo_root, clean_ver, preserve_fragments=False, is_latest=True
):
    """Assemble changelog content, write output file, and clean fragments."""
    unreleased_dir = repo_root / "docs" / "changelog" / "unreleased"
    headers_map = load_headers_map(repo_root / ".changelog_headers.json")
    docs_json = repo_root / "docs" / "docs.json"
    output_path = repo_root / "docs" / "changelog" / f"{clean_ver}.mdx"

    files, grouped, extras = collect_changelog_fragments(unreleased_dir, headers_map)
    print(f"Assembling changelog for v{clean_ver} from {len(files)} fragment(s)...")

    content = render_changelog(clean_ver, headers_map, grouped, extras)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    with open(output_path, "w", encoding="utf-8") as f:
        f.write(content)
    print(f"✅ Generated changelog: {output_path}")

    if docs_json.exists():
        update_docs_json(docs_json, clean_ver, is_latest)

    if not preserve_fragments:
        if is_latest:
            update_version_snippet(repo_root, clean_ver)
        for fpath in files:
            print(f"Removing consumed fragment: {fpath.name}")
            fpath.unlink()
    else:
        print(f"Preserved unreleased changelog fragment(s) in {unreleased_dir}.")


# ==============================================================================
# Version Mutation and Calculation
# ==============================================================================


def set_cargo_version(repo_root, version, skip_nix=False):
    """Update workspace package version in Cargo.toml, sync Cargo.lock, and update Nix hash."""
    cargo_toml = repo_root / "Cargo.toml"
    if not cargo_toml.exists():
        print(f"❌ Error: {cargo_toml} not found", file=sys.stderr)
        sys.exit(1)

    with open(cargo_toml, "r", encoding="utf-8") as f:
        content = f.read()

    new_content = re.sub(
        r'(?m)^version = "[^"]+"',
        f'version = "{version}"',
        content,
        count=1,
    )
    with open(cargo_toml, "w", encoding="utf-8") as f:
        f.write(new_content)
    print(f"✅ Updated {cargo_toml} version to '{version}'")

    print("Syncing Cargo.lock via cargo update --workspace...")
    subprocess.run(["cargo", "update", "--workspace"], cwd=repo_root, check=True)

    if not skip_nix:
        nix_script = repo_root / "scripts" / "update-nix-cargo-hash.sh"
        if nix_script.exists():
            print("Updating Nix cargo hash...")
            subprocess.run(["bash", str(nix_script), "18"], cwd=repo_root, check=True)


def compute_next_dev_version(version, branch, is_beta=False):
    """Compute the next development version string."""
    clean = clean_version(version)
    if is_beta:
        return clean
    major, minor, patch = parse_semver(clean)
    if branch == "main":
        return f"{major}.{minor + 1}.0"
    return f"{major}.{minor}.{patch + 1}"


def check_is_latest(version, is_beta=False):
    """Determine whether a version should be marked as latest release."""
    if is_beta:
        return False
    target = parse_semver(clean_version(version))
    tags = [parse_semver(t) for t in get_git_tags()]
    if not tags:
        return True
    return target >= max(tags)


# ==============================================================================
# CLI Dispatcher
# ==============================================================================


def add_common_args(parser):
    """Add target version and repo root arguments to subparser."""
    parser.add_argument(
        "version", nargs="?", help="Target release version (e.g. 0.26.0)"
    )
    parser.add_argument(
        "--repo-root", default=None, help="Root directory of repository"
    )


def handle_sql_command(args, repo_root):
    """Handle sql subcommand."""
    target_version = detect_target_version(repo_root, args.version)
    clean_target = clean_version(target_version)
    sql_dir = repo_root / "pg_search" / "sql"
    prev_version = resolve_prev_version(
        repo_root, sql_dir, clean_target, args.prev_version
    )
    assemble_sql_files(repo_root, clean_target, prev_version, args.preserve_fragments)


def handle_changelog_command(args, repo_root):
    """Handle changelog subcommand."""
    target_version = detect_target_version(repo_root, args.version)
    clean_ver = clean_version(target_version)

    if args.register_only:
        docs_json = repo_root / "docs" / "docs.json"
        if docs_json.exists():
            update_docs_json(docs_json, clean_ver, args.is_latest)
        if args.is_latest:
            update_version_snippet(repo_root, clean_ver)
        return

    assemble_changelog_files(
        repo_root,
        clean_ver,
        preserve_fragments=args.preserve_fragments,
        is_latest=args.is_latest,
    )


def handle_all_command(args, repo_root):
    """Handle all subcommand to assemble SQL and Changelog."""
    target_version = detect_target_version(repo_root, args.version)
    clean_target = clean_version(target_version)

    sql_dir = repo_root / "pg_search" / "sql"
    prev_version = resolve_prev_version(
        repo_root, sql_dir, clean_target, args.prev_version
    )
    assemble_sql_files(repo_root, clean_target, prev_version, args.preserve_fragments)
    assemble_changelog_files(
        repo_root,
        clean_target,
        preserve_fragments=args.preserve_fragments,
        is_latest=args.is_latest,
    )


def handle_set_version_command(args, repo_root):
    """Handle set-version subcommand."""
    target_version = detect_target_version(repo_root, args.version)
    set_cargo_version(repo_root, target_version, skip_nix=args.skip_nix)


def handle_next_dev_version_command(args, repo_root):
    """Handle next-dev-version subcommand."""
    target_version = detect_target_version(repo_root, args.version)
    next_ver = compute_next_dev_version(target_version, args.branch, is_beta=args.beta)
    print(next_ver)


def handle_is_latest_command(args, repo_root):
    """Handle is-latest subcommand."""
    target_version = detect_target_version(repo_root, args.version)
    is_latest = check_is_latest(target_version, is_beta=args.beta)
    print("true" if is_latest else "false")


def main():
    """Main CLI entry point."""
    parser = argparse.ArgumentParser(
        description="Unified ParadeDB release artifact assembler."
    )
    parser.add_argument(
        "--repo-root", default=None, help="Root directory of repository"
    )
    subparsers = parser.add_subparsers(
        dest="command", required=True, help="Subcommand to execute"
    )

    sql_parser = subparsers.add_parser("sql", help="Assemble SQL migration scripts")
    add_common_args(sql_parser)
    sql_parser.add_argument(
        "--prev-version", default=None, help="Previous version (e.g. 0.25.4)"
    )
    sql_parser.add_argument(
        "--preserve-fragments",
        action="store_true",
        help="Preserve unreleased fragment files instead of deleting them",
    )

    cl_parser = subparsers.add_parser(
        "changelog", help="Assemble changelog page and update docs"
    )
    add_common_args(cl_parser)
    cl_parser.add_argument(
        "--preserve-fragments",
        action="store_true",
        help="Preserve unreleased fragment files instead of deleting them",
    )
    cl_parser.add_argument(
        "--register-only",
        action="store_true",
        help="Only update docs.json and version.mdx",
    )
    cl_parser.add_argument(
        "--is-latest",
        dest="is_latest",
        action=argparse.BooleanOptionalAction,
        default=True,
        help="Whether this version is the latest release",
    )

    all_parser = subparsers.add_parser(
        "all", help="Assemble all release artifacts (SQL + Changelog + Docs)"
    )
    add_common_args(all_parser)
    all_parser.add_argument(
        "--prev-version", default=None, help="Previous version (e.g. 0.25.4)"
    )
    all_parser.add_argument(
        "--preserve-fragments",
        action="store_true",
        help="Preserve unreleased fragment files instead of deleting them",
    )
    all_parser.add_argument(
        "--is-latest",
        dest="is_latest",
        action=argparse.BooleanOptionalAction,
        default=True,
        help="Whether this version is the latest release",
    )

    set_ver_parser = subparsers.add_parser(
        "set-version",
        help="Update version in Cargo.toml, Cargo.lock, and nix/pg_search.nix",
    )
    add_common_args(set_ver_parser)
    set_ver_parser.add_argument(
        "--skip-nix",
        action="store_true",
        help="Skip updating Nix cargo hash",
    )

    next_dev_parser = subparsers.add_parser(
        "next-dev-version",
        help="Compute post-release development version",
    )
    add_common_args(next_dev_parser)
    next_dev_parser.add_argument(
        "--branch",
        required=True,
        help="Release branch name (e.g. main or 0.25.x)",
    )
    next_dev_parser.add_argument(
        "--beta",
        action="store_true",
        help="Whether this was a beta release",
    )

    is_latest_parser = subparsers.add_parser(
        "is-latest",
        help="Determine if version is latest release",
    )
    add_common_args(is_latest_parser)
    is_latest_parser.add_argument(
        "--beta",
        action="store_true",
        help="Whether this is a beta release",
    )

    args = parser.parse_args()
    default_root = Path(__file__).resolve().parent.parent.parent
    repo_root = Path(args.repo_root or default_root)

    if args.command == "sql":
        handle_sql_command(args, repo_root)
    elif args.command == "changelog":
        handle_changelog_command(args, repo_root)
    elif args.command == "all":
        handle_all_command(args, repo_root)
    elif args.command == "set-version":
        handle_set_version_command(args, repo_root)
    elif args.command == "next-dev-version":
        handle_next_dev_version_command(args, repo_root)
    elif args.command == "is-latest":
        handle_is_latest_command(args, repo_root)


if __name__ == "__main__":
    main()
