#!/usr/bin/env python3
""".github/scripts/release.py.

Unified release artifact assembler for ParadeDB:
- Assembles unreleased SQL migration fragments into pg_search--<prev>--<target>.sql
- Assembles unreleased changelog fragments into docs/changelog/<version>.mdx
- Registers new versions in docs/docs.json and docs/snippets/version.mdx
"""

# pylint: disable=too-many-lines

import argparse
import json
import re
import subprocess
import sys
from collections import defaultdict, deque
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


def parse_depends_on(content):
    """Parse '-- depends-on: <PR>, ...' or '/* depends-on: ... */' from content."""
    deps = set()
    for match in re.finditer(
        r"(?:--|/\*)\s*depends-on:\s*([0-9#,\s\w.-]+?)(?:\*/|\n|$)",
        content,
        re.IGNORECASE,
    ):
        raw = match.group(1)
        for num in re.findall(r"\b\d+\b", raw):
            deps.add(int(num))
        for token in raw.split(","):
            token = token.strip().lstrip("#")
            if token.endswith(".sql"):
                deps.add(token)
    return deps


def _build_fragment_dependency_graph(fragments):
    """Build mapping of fragment -> set of prerequisite fragments."""
    pr_map = defaultdict(list)
    name_map = {}
    for f in fragments:
        pr_num = parse_pr_number(f.name)
        if pr_num > 0:
            pr_map[pr_num].append(f)
        name_map[f.name] = f

    fragment_deps = defaultdict(set)
    for f in fragments:
        try:
            with open(f, "r", encoding="utf-8") as fp:
                content = fp.read()
        except OSError:
            continue
        raw_deps = parse_depends_on(content)
        for dep in raw_deps:
            if isinstance(dep, int) and dep in pr_map:
                for target_f in pr_map[dep]:
                    if target_f != f:
                        fragment_deps[f].add(target_f)
            elif isinstance(dep, str) and dep in name_map and name_map[dep] != f:
                fragment_deps[f].add(name_map[dep])

    return fragment_deps


def topological_sort_fragments(fragments):
    """Topologically sort SQL fragments by their declared dependencies.

    Tiebreaker for independent fragments is (parse_pr_number(f.name), f.name).
    """
    fragment_deps = _build_fragment_dependency_graph(fragments)

    in_degree = {f: 0 for f in fragments}
    dependents = defaultdict(list)
    for f, deps in fragment_deps.items():
        for dep in deps:
            dependents[dep].append(f)
            in_degree[f] += 1

    def tiebreaker(item):
        return (parse_pr_number(item.name), item.name)

    queue = deque(sorted([f for f in fragments if in_degree[f] == 0], key=tiebreaker))
    result = []

    while queue:
        node = queue.popleft()
        result.append(node)

        new_ready = []
        for nxt in dependents[node]:
            in_degree[nxt] -= 1
            if in_degree[nxt] == 0:
                new_ready.append(nxt)
        for item in sorted(new_ready, key=tiebreaker):
            queue.append(item)

    if len(result) != len(fragments):
        unresolved = [f.name for f in fragments if in_degree[f] > 0]
        print(
            f"❌ Error: Circular or unresolved dependency among fragments: {unresolved}",
            file=sys.stderr,
        )
        sys.exit(1)

    return result


def collect_sql_fragments(unreleased_dir):
    """Collect all *.sql fragments in unreleased dir topologically sorted by dependencies."""
    if not unreleased_dir.exists():
        return []
    fragments = [
        f for f in unreleased_dir.glob("*.sql") if f.is_file() and f.name != ".gitkeep"
    ]
    return topological_sort_fragments(fragments)


def format_sql_banner(filename):
    """Generate section banner for an assembled SQL fragment."""
    sep = "-- " + "=" * 76
    return f"\n{sep}\n-- Fragment: {filename}\n{sep}\n"


def assemble_sql_files(repo_root, target_version, prev_version, preserve_fragments):
    """Assemble SQL fragments into pg_search--<prev>--<target>.sql."""
    sql_dir = repo_root / "pg_search" / "sql"
    unreleased_dir = sql_dir / "unreleased"
    output_file = sql_dir / f"pg_search--{prev_version}--{target_version}.sql"

    print(
        f"Assembling SQL upgrade script: {output_file} "
        f"(from {prev_version} to {target_version})"
    )

    fragments = collect_sql_fragments(unreleased_dir)
    print(f"Found {len(fragments)} SQL fragment(s):")
    for fragment in fragments:
        print(f"  - {fragment.name}")

    with open(output_file, "w", encoding="utf-8") as out:
        echo_header = (
            f"\\echo Use \"ALTER EXTENSION pg_search UPDATE TO '{target_version}'\" "
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
        // Do not edit manually: this file is updated automatically by release.py upon release.
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


def check_is_latest(version, is_beta=False):
    """Determine whether a version should be marked as latest release."""
    if is_beta:
        return False
    target = parse_semver(clean_version(version))
    tags = [parse_semver(t) for t in get_git_tags()]
    if not tags:
        return True
    return target >= max(tags)


def get_rendered_changelog_body(repo_root, clean_ver):
    """Retrieve rendered changelog body from existing MDX file or assemble in-memory."""
    changelog_file = repo_root / "docs" / "changelog" / f"{clean_ver}.mdx"
    if changelog_file.exists():
        with open(changelog_file, "r", encoding="utf-8") as f:
            _, body = parse_frontmatter(f.read())
        return body

    unreleased_dir = repo_root / "docs" / "changelog" / "unreleased"
    headers_map = load_headers_map(repo_root / ".changelog_headers.json")
    _, grouped, extras = collect_changelog_fragments(unreleased_dir, headers_map)
    rendered = render_changelog(clean_ver, headers_map, grouped, extras)
    _, body = parse_frontmatter(rendered)
    return body


def generate_approval_body(
    repo_root,
    target_version,
    branch,
    prev_version=None,
):
    """Generate Markdown description for manual release approval issue."""
    clean_ver = clean_version(target_version)
    sql_dir = repo_root / "pg_search" / "sql"
    prev_ver = resolve_prev_version(repo_root, sql_dir, clean_ver, prev_version)
    is_latest = check_is_latest(target_version, is_beta=False)

    if branch == "main":
        release_type = "Minor release"
        major, minor, _ = parse_semver(clean_ver)
        branch_action = f"Create stable branch `{major}.{minor}.x`"
    else:
        release_type = "Patch release"
        branch_action = "Sync release artifacts to `main`"

    changelog_body = get_rendered_changelog_body(repo_root, clean_ver)
    template = dedent(
        """\
        Please review and approve the release of **ParadeDB `v{clean_ver}`**.

        ### 📋 Release Details

        | Property | Value |
        | --- | --- |
        | **Target Version** | `{clean_ver}` (`v{clean_ver}`) |
        | **Release Type** | {release_type} |
        | **Release Branch** | `{branch}` |
        | **Previous Version** | `{prev_ver}` |
        | **Is Latest Release** | {is_latest} |
        | **SQL Upgrade Script** | `pg_search/sql/pg_search--{prev_ver}--{clean_ver}.sql` |
        | **Changelog Document** | `docs/changelog/{clean_ver}.mdx` |
        | **Post-Release Action** | {branch_action} |

        ---

        ### 📝 Rendered Changelog

        {changelog_body}

        ---

        ### 📣 Approval Instructions

        - To **approve** this release, comment `approved` on this issue.
        - To **deny** this release, comment `denied` on this issue.
        """
    )

    return template.format(
        clean_ver=clean_ver,
        release_type=release_type,
        branch=branch,
        prev_ver=prev_ver,
        is_latest="true" if is_latest else "false",
        branch_action=branch_action,
        changelog_body=changelog_body,
    ).strip()


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
    assemble_sql_files(repo_root, target_version, prev_version, args.preserve_fragments)


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
    is_beta = args.beta or ("-rc." in target_version)

    sql_dir = repo_root / "pg_search" / "sql"
    prev_version = resolve_prev_version(
        repo_root, sql_dir, clean_target, args.prev_version
    )
    preserve = args.preserve_fragments or is_beta
    assemble_sql_files(repo_root, target_version, prev_version, preserve)

    if not is_beta:
        assemble_changelog_files(
            repo_root,
            clean_target,
            preserve_fragments=args.preserve_fragments,
            is_latest=args.is_latest,
        )
    else:
        print("ℹ️ Beta release: skipping changelog assembly and docs registration.")


def handle_set_version_command(args, repo_root):
    """Handle set-version subcommand."""
    target_version = detect_target_version(repo_root, args.version)
    set_cargo_version(repo_root, target_version, skip_nix=args.skip_nix)


def handle_is_latest_command(args, repo_root):
    """Handle is-latest subcommand."""
    target_version = detect_target_version(repo_root, args.version)
    is_latest = check_is_latest(target_version, is_beta=args.beta)
    print("true" if is_latest else "false")


def handle_approval_body_command(args, repo_root):
    """Handle approval-body subcommand."""
    target_version = detect_target_version(repo_root, args.version)
    body = generate_approval_body(
        repo_root,
        target_version,
        branch=args.branch,
        prev_version=args.prev_version,
    )
    if args.output_file:
        output_path = Path(args.output_file)
        output_path.parent.mkdir(parents=True, exist_ok=True)
        with open(output_path, "w", encoding="utf-8") as f:
            f.write(body)
            f.write("\n")
        print(f"✅ Wrote approval body to {output_path}")
    else:
        print(body)


# ==============================================================================
# Fragment Linting & Validation
# ==============================================================================


def extract_statements_and_objects(content):
    """Extract (statement_type, object_signature, raw_stmt) from SQL content."""
    no_comments = re.sub(r"--[^\n]*", "", content)
    no_comments = re.sub(r"/\*.*?\*/", "", no_comments, flags=re.DOTALL)
    no_comments = re.sub(r"\\\w+[^\n]*", "", no_comments)

    results = []
    for s in no_comments.split(";"):
        stmt = " ".join(s.split()).strip()
        if not stmt:
            continue

        m = re.match(
            r"^(CREATE(?:\s+OR\s+REPLACE)?|ALTER|DROP)\s+([A-Z\s]+?)\s+"
            r"(?:IF\s+EXISTS\s+)?([^\s(]+)(?:\s*\((.*?)\))?",
            stmt,
            re.IGNORECASE,
        )
        if m:
            verb, obj_type, name, args = m.groups()
            name = name.replace('"', "").lower()
            obj_type = " ".join(obj_type.upper().split())

            if args is not None and obj_type in (
                "FUNCTION",
                "AGGREGATE",
                "PROCEDURE",
            ):
                arg_types = []
                for a in args.split(","):
                    a = re.sub(r"(?i)\s+(DEFAULT|=)\s+.*$", "", a).strip()
                    parts = a.split()
                    if parts:
                        t = parts[-1].replace('"', "").lower()
                        arg_types.append(t)
                sig = f"{obj_type} {name}({', '.join(arg_types)})"
            else:
                sig = f"{obj_type} {name}"

            results.append((verb.upper(), sig, stmt))

    return results


def catalog_unreleased_objects(unreleased_dir, before_pr=None):
    """Map object_signature -> owning PR number from existing unreleased fragments."""
    object_map = {}
    if not unreleased_dir.exists():
        return object_map

    for f in sorted(unreleased_dir.glob("*.sql")):
        if f.name == ".gitkeep":
            continue
        pr_num = parse_pr_number(f.name)
        if pr_num == 0 or (before_pr is not None and pr_num >= before_pr):
            continue

        try:
            with open(f, "r", encoding="utf-8") as fp:
                content = fp.read()
            for verb, sig, _ in extract_statements_and_objects(content):
                if "CREATE" in verb or "REPLACE" in verb:
                    object_map[sig] = pr_num
        except OSError as e:
            print(f"⚠️ Warning: Could not read {f}: {e}", file=sys.stderr)

    return object_map


def is_release_branch(branch_name):
    """Check if branch name corresponds to a release branch (e.g. 0.25.x)."""
    return bool(re.match(r"^v?\d+\.\d+\.x$", branch_name))


def get_changed_fragment_files(repo_root, base_sha):
    """Identify fragment files added or modified in the PR."""
    unreleased_dir = repo_root / "pg_search" / "sql" / "unreleased"
    diff_ref = base_sha

    if not diff_ref:
        for ref in ["origin/main", "main", "HEAD~1"]:
            try:
                mb = (
                    subprocess.check_output(
                        ["git", "merge-base", ref, "HEAD"],
                        cwd=repo_root,
                        stderr=subprocess.DEVNULL,
                    )
                    .decode()
                    .strip()
                )
                if mb:
                    diff_ref = mb
                    break
            except subprocess.SubprocessError:
                continue

    if diff_ref:
        try:
            res = subprocess.run(
                [
                    "git",
                    "diff",
                    "--name-only",
                    "--diff-filter=AM",
                    f"{diff_ref}...HEAD",
                    "--",
                    "pg_search/sql/unreleased/*.sql",
                ],
                cwd=repo_root,
                capture_output=True,
                text=True,
                check=True,
            )
            changed = []
            for line in res.stdout.strip().splitlines():
                line = line.strip()
                if line:
                    changed.append(Path(line).name)
            if changed:
                return changed
        except subprocess.SubprocessError:
            pass

    return [f.name for f in unreleased_dir.glob("*.sql") if f.name != ".gitkeep"]


def _check_single_main_fragment(fpath, unreleased_dir):
    current_pr = parse_pr_number(fpath.name)
    object_map = catalog_unreleased_objects(unreleased_dir, before_pr=current_pr)

    content = fpath.read_text(encoding="utf-8")
    declared_deps = parse_depends_on(content)

    detected_unreleased_deps = set()
    has_released_objects = False
    touched_objects = []

    for _verb, sig, _stmt in extract_statements_and_objects(content):
        if sig in object_map and object_map[sig] != current_pr:
            detected_unreleased_deps.add(object_map[sig])
            touched_objects.append((sig, f"unreleased (PR #{object_map[sig]})"))
        elif sig not in object_map:
            has_released_objects = True
            touched_objects.append((sig, "released"))

    errors = 0
    for dep_pr in detected_unreleased_deps:
        if dep_pr not in declared_deps and str(dep_pr) not in declared_deps:
            print(
                f"::error file={fpath}::Fragment touches unreleased object(s) "
                f"from PR #{dep_pr} but does not declare '-- depends-on: {dep_pr}'.",
                file=sys.stderr,
            )
            print(
                f"❌ {fpath.name}: Must declare '-- depends-on: {dep_pr}' in a header comment.",
                file=sys.stderr,
            )
            errors += 1

    if has_released_objects and detected_unreleased_deps:
        print(
            f"::error file={fpath}::Fragment mixes modifications to already-released objects "
            f"with objects from unreleased PR(s) {sorted(detected_unreleased_deps)}.",
            file=sys.stderr,
        )
        print(
            f"❌ {fpath.name}: Contains mixed dependencies:\n"
            + "\n".join(f"  - {sig} [{status}]" for sig, status in touched_objects)
            + "\nMixing released and unreleased objects in a single fragment breaks backports "
            "to stable branches.\nPlease split this into separate fragment files:\n"
            f"  1. A fragment for released objects (no unreleased dependencies)\n"
            f"  2. A separate fragment declaring '-- depends-on: {min(detected_unreleased_deps)}' "
            "for the unreleased objects.",
            file=sys.stderr,
        )
        errors += 1

    if len(detected_unreleased_deps) > 1:
        print(
            f"::error file={fpath}::Fragment touches objects from multiple distinct "
            f"unreleased PRs: {sorted(detected_unreleased_deps)}.",
            file=sys.stderr,
        )
        print(
            f"❌ {fpath.name}: Split this into separate fragment files for each unreleased PR.",
            file=sys.stderr,
        )
        errors += 1

    return errors


def lint_main_branch_fragments(repo_root, base_sha):
    """Lint fragments on PRs targeting main."""
    unreleased_dir = repo_root / "pg_search" / "sql" / "unreleased"
    changed_files = get_changed_fragment_files(repo_root, base_sha)

    if not changed_files:
        print("✅ No unreleased SQL fragments modified in this PR.")
        return 0

    print(f"Linting {len(changed_files)} fragment(s) on main: {changed_files}")

    errors = 0
    for fname in changed_files:
        fpath = unreleased_dir / fname
        if fpath.exists():
            errors += _check_single_main_fragment(fpath, unreleased_dir)

    return errors


def _check_branch_fragment_dependencies(repo_root, unreleased_sql_dir, base_ref):
    errors = 0
    for fpath in unreleased_sql_dir.glob("*.sql"):
        if fpath.name == ".gitkeep":
            continue
        with open(fpath, "r", encoding="utf-8") as fp:
            content = fp.read()
        deps = parse_depends_on(content)
        for dep in deps:
            if isinstance(dep, int):
                check = subprocess.run(
                    ["git", "log", "-n", "1", f"--grep=#{dep}\\b", f"--grep=({dep})"],
                    cwd=repo_root,
                    capture_output=True,
                    text=True,
                    check=False,
                )
                if not check.stdout.strip():
                    print(
                        f"::error file={fpath}::Fragment declares '-- depends-on: {dep}', "
                        f"but PR #{dep} has not been backported to this branch ({base_ref}).",
                        file=sys.stderr,
                    )
                    print(
                        f"❌ {fpath.name}: Dependency PR #{dep} is missing from {base_ref}. "
                        "Drop this fragment from the backport.",
                        file=sys.stderr,
                    )
                    errors += 1
    return errors


def _check_branch_fragment_identity(repo_root, all_fragments):
    subprocess.run(
        ["git", "fetch", "origin", "main"],
        cwd=repo_root,
        capture_output=True,
        check=False,
    )

    errors = 0
    for fpath in all_fragments:
        if fpath.name == ".gitkeep":
            continue
        rel_path = fpath.relative_to(repo_root)

        show_cmd = subprocess.run(
            ["git", "show", f"origin/main:{rel_path}"],
            cwd=repo_root,
            capture_output=True,
            check=False,
        )
        if show_cmd.returncode == 0:
            main_bytes = show_cmd.stdout
            local_bytes = fpath.read_bytes()
            if main_bytes != local_bytes:
                print(
                    f"::error file={rel_path}::Fragment differs from origin/main. Fragments "
                    "sharing a filename between a release branch and main must be identical.",
                    file=sys.stderr,
                )
                print(
                    f"❌ {rel_path}: Content differs from origin/main.\n"
                    "If this fragment only partially applies to the stable branch, the main-only "
                    "portion must be placed in a separate fragment file on main first.",
                    file=sys.stderr,
                )
                errors += 1
    return errors


def lint_release_branch_fragments(repo_root, base_ref):
    """Lint fragments on PRs targeting a stable release branch (e.g. 0.25.x)."""
    unreleased_sql_dir = repo_root / "pg_search" / "sql" / "unreleased"
    unreleased_cl_dir = repo_root / "docs" / "changelog" / "unreleased"

    all_fragments = list(unreleased_sql_dir.glob("*.sql")) + list(
        unreleased_cl_dir.glob("*.mdx")
    )

    errors = _check_branch_fragment_dependencies(
        repo_root, unreleased_sql_dir, base_ref
    )
    errors += _check_branch_fragment_identity(repo_root, all_fragments)
    return errors


def handle_lint_fragments_command(args, repo_root):
    """Handle lint-fragments subcommand."""
    print(f"Linting fragments with base_ref='{args.base_ref}' at {repo_root}")

    if is_release_branch(args.base_ref):
        errors = lint_release_branch_fragments(repo_root, args.base_ref)
    else:
        errors = lint_main_branch_fragments(repo_root, args.base_sha)

    if errors > 0:
        print(f"\n❌ Fragment lint failed with {errors} error(s).", file=sys.stderr)
        sys.exit(1)

    print("✅ All fragment lint checks passed.")


def build_parser():
    """Build CLI argument parser."""
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
    all_parser.add_argument(
        "--beta",
        action="store_true",
        help="Whether this is a beta release",
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

    approval_parser = subparsers.add_parser(
        "approval-body",
        help="Generate Markdown body for manual release approval issue",
    )
    add_common_args(approval_parser)
    approval_parser.add_argument(
        "--branch",
        required=True,
        help="Release branch name (e.g. main or 0.25.x)",
    )
    approval_parser.add_argument(
        "--prev-version",
        default=None,
        help="Previous version (e.g. 0.25.4)",
    )
    approval_parser.add_argument(
        "--output-file",
        default=None,
        help="Optional file path to write Markdown output to",
    )

    lint_parser = subparsers.add_parser(
        "lint-fragments", help="Lint unreleased migration fragments"
    )
    lint_parser.add_argument(
        "--base-ref",
        default="main",
        help="Target base branch of the PR (e.g. 'main' or '0.25.x')",
    )
    lint_parser.add_argument(
        "--base-sha",
        default=None,
        help="Base commit SHA of the PR",
    )

    return parser


def main():
    """Main CLI entry point."""
    parser = build_parser()
    args = parser.parse_args()
    default_root = Path(__file__).resolve().parent.parent.parent
    repo_root = Path(args.repo_root or default_root)

    commands = {
        "sql": handle_sql_command,
        "changelog": handle_changelog_command,
        "all": handle_all_command,
        "set-version": handle_set_version_command,
        "is-latest": handle_is_latest_command,
        "approval-body": handle_approval_body_command,
        "lint-fragments": handle_lint_fragments_command,
    }
    handler = commands.get(args.command)
    if handler:
        handler(args, repo_root)


if __name__ == "__main__":
    main()
