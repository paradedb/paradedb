#!/usr/bin/env python3
# pylint: disable=duplicate-code
"""scripts/assemble_changelog.py.

Assembles unreleased changelog fragments from docs/changelog/unreleased/
into a versioned changelog page (docs/changelog/<version>.mdx) and registers
the new page in docs/docs.json.
"""

import argparse
import json
import re
import sys
from pathlib import Path

CATEGORY_FALLBACKS = {
    "bugfix": "stability",
    "fix": "stability",
    "stability": "stability",
    "feat": "features",
    "feature": "features",
    "features": "features",
    "perf": "performance",
    "performance": "performance",
    "breaking": "breaking",
    "doc": "docs",
    "docs": "docs",
    "documentation": "docs",
}


def parse_fragment(filepath):
    """Extract frontmatter dict and markdown content from a fragment file."""
    with open(filepath, "r", encoding="utf-8") as f:
        raw = f.read()

    frontmatter = {}
    content = raw.strip()

    match = re.match(r"^---\s*\n(.*?)\n---\s*\n(.*)$", raw, re.DOTALL)
    if match:
        fm_text, content = match.group(1), match.group(2).strip()
        for line in fm_text.splitlines():
            line = line.strip()
            if ":" in line:
                key, val = line.split(":", 1)
                frontmatter[key.strip()] = val.strip().strip("\"'")

    return frontmatter, content


def determine_category(filepath, frontmatter):
    """Determine the header category from frontmatter or filename fallback."""
    if frontmatter.get("header"):
        return frontmatter["header"]

    filename = Path(filepath).stem
    parts = filename.split(".")
    if len(parts) >= 2:
        category = parts[-1].lower()
        if category in CATEGORY_FALLBACKS:
            return CATEGORY_FALLBACKS[category]

    return "other"


def format_fragment_body(body):
    """Format body lines into markdown bullet items."""
    lines = body.splitlines()
    formatted_lines = []
    for i, line in enumerate(lines):
        if i == 0 and not line.lstrip().startswith(("-", "*")):
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


def detect_version(repo_root, explicit_version):
    """Resolve target version from args or Cargo.toml."""
    if explicit_version:
        return explicit_version

    cargo_toml = repo_root / "Cargo.toml"
    if cargo_toml.exists():
        with open(cargo_toml, "r", encoding="utf-8") as f:
            for line in f:
                if line.startswith("version ="):
                    return line.split("=")[1].strip().strip("\"'")

    print(
        "❌ Error: Version not specified and not found in Cargo.toml",
        file=sys.stderr,
    )
    sys.exit(1)


def collect_fragments(unreleased_dir, headers_map):
    """Collect and categorize all fragment contents."""
    files = sorted(
        [
            p
            for p in unreleased_dir.glob("*")
            if p.is_file() and p.name != ".gitkeep" and p.suffix in [".mdx", ".md"]
        ]
    )

    grouped = {k: [] for k in headers_map}
    extras = {}

    for fpath in files:
        fm, body = parse_fragment(fpath)
        cat = determine_category(fpath, fm)
        formatted_body = format_fragment_body(body)

        if cat in grouped:
            grouped[cat].append(formatted_body)
        else:
            extras.setdefault(cat, []).append(formatted_body)

    return files, grouped, extras


def render_changelog(clean_version, headers_map, grouped, extras):
    """Render markdown content for the changelog page."""
    lines = [
        "---",
        f"title: {clean_version}",
        "noindex: true",
        "---",
        "",
    ]
    has_entries = False

    for cat_key, header_title in headers_map.items():
        entries = grouped.get(cat_key, [])
        if entries:
            has_entries = True
            lines.append(header_title)
            lines.append("")
            lines.extend(entries)
            lines.append("")

    for cat_key, entries in extras.items():
        if entries:
            has_entries = True
            lines.append(f"## {cat_key.capitalize()}")
            lines.append("")
            lines.extend(entries)
            lines.append("")

    if not has_entries:
        lines.append("## Changes")
        lines.append("")
        lines.append(f"- Maintenance and internal updates for {clean_version}.")
        lines.append("")

    lines.append(
        f"The full changelog is available [on the GitHub Release]"
        f"(https://github.com/paradedb/paradedb/releases/tag/v{clean_version})."
    )
    lines.append("")
    return "\n".join(lines)


def load_headers_map(headers_config_path):
    """Load headers map from config file if present."""
    if headers_config_path.exists():
        with open(headers_config_path, "r", encoding="utf-8") as f:
            return json.load(f)
    return {}


def assemble_and_write(repo_root, clean_version, dry_run=False, is_latest=True):
    """Assemble changelog content, write output file, and clean fragments."""
    unreleased_dir = repo_root / "docs" / "changelog" / "unreleased"
    headers_map = load_headers_map(repo_root / ".changelog_headers.json")
    docs_json = repo_root / "docs" / "docs.json"
    output_path = repo_root / "docs" / "changelog" / f"{clean_version}.mdx"

    files, grouped, extras = collect_fragments(unreleased_dir, headers_map)
    print(f"Assembling changelog for v{clean_version} from {len(files)} fragment(s)...")

    content = render_changelog(clean_version, headers_map, grouped, extras)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    with open(output_path, "w", encoding="utf-8") as f:
        f.write(content)
    print(f"✅ Generated changelog: {output_path}")

    if docs_json.exists():
        update_docs_json(docs_json, clean_version, is_latest)

    if not dry_run:
        if is_latest:
            update_version_snippet(repo_root, clean_version)
        for fpath in files:
            print(f"Removing consumed fragment: {fpath.name}")
            fpath.unlink()


def update_version_snippet(repo_root, clean_version):
    """Update exported version variable in docs/snippets/version.mdx."""
    snippet_file = repo_root / "docs" / "snippets" / "version.mdx"
    snippet_file.parent.mkdir(parents=True, exist_ok=True)
    with open(snippet_file, "w", encoding="utf-8") as f:
        f.write(f'export const version = "{clean_version}";\n')
    print(f"✅ Updated {snippet_file} with version '{clean_version}'")


def main():
    """Main execution function for assemble_changelog."""
    parser = argparse.ArgumentParser(description="Assemble changelog fragments.")
    parser.add_argument(
        "version", nargs="?", help="Target release version (e.g. 0.25.5)"
    )
    parser.add_argument(
        "--dry-run", action="store_true", help="Do not delete consumed fragments"
    )
    parser.add_argument(
        "--register-only",
        action="store_true",
        help="Only update docs/docs.json without assembling fragments",
    )
    parser.add_argument(
        "--is-latest",
        dest="is_latest",
        action=argparse.BooleanOptionalAction,
        default=True,
        help="Whether this version is the latest release",
    )
    parser.add_argument(
        "--repo-root", default=None, help="Root directory of repository"
    )
    args = parser.parse_args()

    default_root = Path(__file__).resolve().parent.parent.parent
    repo_root = Path(args.repo_root or default_root)
    version = detect_version(repo_root, args.version)
    clean_version = version.split("-")[0].lstrip("v")

    if args.register_only:
        docs_json = repo_root / "docs" / "docs.json"
        if docs_json.exists():
            update_docs_json(docs_json, clean_version, args.is_latest)
        if args.is_latest:
            update_version_snippet(repo_root, clean_version)
        return

    assemble_and_write(repo_root, clean_version, args.dry_run, is_latest=args.is_latest)


if __name__ == "__main__":
    main()
