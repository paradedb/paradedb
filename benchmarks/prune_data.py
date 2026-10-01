#!/usr/bin/env python3
"""
Prune benchmark data points and empty charts from benchmarks/data.js.

Allows purging data points not associated with a specified list of commits
(e.g., releases, recent edits), or applying chart-specific rules (such as
keeping post-discontinuity runs), significantly reducing file size.
"""

from __future__ import annotations

import argparse
from datetime import datetime, timezone
import json
import os
import re
import subprocess
import sys
import tempfile
from typing import Any, Dict, List, Optional, Set, Tuple


DEFAULT_DATA_PATH = "benchmarks/data.js"
BENCHMARK_DATA_PREFIX = "window.BENCHMARK_DATA = "

# Default regex pattern matching release preparation/version bump commits
RELEASE_COMMIT_PATTERN = re.compile(
    r"chore:\s*(?:Prepare|Upgrade to|bump\b).*?(?:\d+\.\d+|\bversion\b)|"
    r"^v?\d+\.\d+\.\d+(?:-\w+)?$|"
    r"\bpost[\s-]release\b",
    re.IGNORECASE,
)



def resolve_default_input_path() -> str:
    """Find the default data.js file path."""
    if os.path.isfile(DEFAULT_DATA_PATH):
        return DEFAULT_DATA_PATH
    script_dir = os.path.dirname(os.path.abspath(__file__))
    candidate = os.path.join(script_dir, "data.js")
    if os.path.isfile(candidate):
        return candidate
    return DEFAULT_DATA_PATH


def load_benchmark_data(path: str) -> Tuple[Dict[str, Any], str, str]:
    """
    Load data.js and parse the embedded JSON.

    Returns:
        (data_dict, prefix, raw_content)
    """
    if not os.path.isfile(path):
        raise FileNotFoundError(f"Input file not found: {path}")

    with open(path, "r", encoding="utf-8") as f:
        content = f.read()

    prefix = BENCHMARK_DATA_PREFIX
    idx = content.find(prefix)
    if idx != -1:
        prefix_str = content[: idx + len(prefix)]
        json_str = content[idx + len(prefix) :].strip()
    else:
        # Fall back to checking if it starts with any JS assignment
        match = re.match(r"^(\s*window\.\w+\s*=\s*)", content)
        if match:
            prefix_str = match.group(1)
            json_str = content[len(prefix_str) :].strip()
        else:
            prefix_str = ""
            json_str = content.strip()

    if json_str.endswith(";"):
        json_str = json_str[:-1].strip()

    data = json.loads(json_str)
    if not isinstance(data, dict):
        raise ValueError(f"Expected root JSON object, got {type(data).__name__}")
    if "entries" not in data:
        raise ValueError("JSON data missing required 'entries' key")

    return data, prefix_str, content


def dump_benchmark_data(data: Dict[str, Any], prefix: str) -> str:
    """Serialize the benchmark data back to JavaScript format."""
    json_str = json.dumps(data, indent=2, ensure_ascii=False)
    if prefix:
        return f"{prefix}{json_str}\n"
    return f"{json_str}\n"


def atomic_write(target_path: str, content: str) -> None:
    """Write content to a temporary file in target's directory and atomically rename."""
    target_dir = os.path.dirname(os.path.abspath(target_path)) or "."
    with tempfile.NamedTemporaryFile("w", dir=target_dir, encoding="utf-8", delete=False) as tf:
        temp_name = tf.name
        tf.write(content)
        tf.flush()
        os.fsync(tf.fileno())

    os.replace(temp_name, target_path)


def parse_iso_datetime(ts: str) -> Optional[datetime]:
    """Parse an ISO timestamp string with timezone support."""
    if not ts:
        return None
    try:
        cleaned = ts.replace("Z", "+00:00")
        dt = datetime.fromisoformat(cleaned)
        if dt.tzinfo is None:
            dt = dt.replace(tzinfo=timezone.utc)
        return dt
    except (ValueError, TypeError):
        return None


def resolve_cutoff_datetime(val: str, unique_commits: List[Dict[str, Any]]) -> Optional[datetime]:
    """Resolve a cutoff value (commit hash, prefix, ISO date, or git rev) to a datetime."""
    val_lower = val.strip().lower()
    # 1. Match against commit in data.js
    for c in unique_commits:
        cid = c["id"].lower()
        if cid == val_lower or cid.startswith(val_lower):
            dt = parse_iso_datetime(c.get("timestamp", ""))
            if dt:
                return dt
            if c.get("date"):
                return datetime.fromtimestamp(c["date"] / 1000.0, tz=timezone.utc)

    # 2. Check ISO date pattern (e.g. YYYY-MM-DD or YYYY-MM-DDTHH:MM:SS)
    if re.match(r"^\d{4}-\d{2}-\d{2}", val):
        if len(val) == 10:
            return datetime.fromisoformat(val + "T00:00:00+00:00")
        return parse_iso_datetime(val)

    # 3. Check git rev commit date
    try:
        res = subprocess.run(
            ["git", "show", "-s", "--format=%cI", val],
            capture_output=True,
            text=True,
            check=True,
        )
        git_iso = res.stdout.strip()
        if git_iso:
            return parse_iso_datetime(git_iso)
    except (subprocess.SubprocessError, FileNotFoundError):
        pass

    return None


def read_commits_from_file(file_path: str) -> List[str]:
    """Read commit hashes from a file or stdin, skipping comments and blank lines."""
    commits = []
    if file_path == "-":
        lines = sys.stdin.read().splitlines()
    else:
        with open(file_path, "r", encoding="utf-8") as f:
            lines = f.read().splitlines()

    for line in lines:
        cleaned = line.split("#", 1)[0].strip()
        if cleaned:
            # If line has multiple whitespace-separated tokens, take the first token
            tokens = cleaned.split()
            commits.append(tokens[0])
    return commits


def resolve_git_tags() -> List[str]:
    """Resolve git tags in the current repository to commit hashes."""
    try:
        res = subprocess.run(
            ["git", "show-ref", "--tags", "-d"],
            capture_output=True,
            text=True,
            check=True,
        )
    except (subprocess.SubprocessError, FileNotFoundError):
        return []

    commits = []
    for line in res.stdout.splitlines():
        parts = line.strip().split()
        if parts:
            commits.append(parts[0])
    return commits


def resolve_git_rev(rev: str) -> List[str]:
    """Resolve a git revision or range using git rev-parse or git rev-list."""
    try:
        if ".." in rev:
            res = subprocess.run(
                ["git", "rev-list", rev],
                capture_output=True,
                text=True,
                check=True,
            )
            return [line.strip() for line in res.stdout.splitlines() if line.strip()]
        else:
            res = subprocess.run(
                ["git", "rev-parse", f"{rev}^{{commit}}"],
                capture_output=True,
                text=True,
                check=True,
            )
            val = res.stdout.strip()
            return [val] if val else []
    except (subprocess.SubprocessError, FileNotFoundError):
        return []


def collect_unique_commits(data: Dict[str, Any]) -> List[Dict[str, Any]]:
    """
    Extract all unique commit metadata from data['entries'].

    Returns a list of dicts with:
        'id': full 40-char SHA
        'timestamp': ISO timestamp or date
        'date': epoch ms
        'message': commit message
        'occurrences': count across charts
    """
    commits_map: Dict[str, Dict[str, Any]] = {}
    entries = data.get("entries", {})

    for chart_name, points in entries.items():
        for point in points:
            commit = point.get("commit") or {}
            cid = commit.get("id")
            if not cid:
                continue

            if cid not in commits_map:
                commits_map[cid] = {
                    "id": cid,
                    "timestamp": commit.get("timestamp") or "",
                    "date": point.get("date") or 0,
                    "message": commit.get("message") or "",
                    "occurrences": 1,
                }
            else:
                commits_map[cid]["occurrences"] += 1
                if not commits_map[cid]["timestamp"] and commit.get("timestamp"):
                    commits_map[cid]["timestamp"] = commit.get("timestamp")
                if not commits_map[cid]["date"] and point.get("date"):
                    commits_map[cid]["date"] = point.get("date")

    # Sort by timestamp/date descending
    commits_list = list(commits_map.values())
    commits_list.sort(key=lambda c: (c["timestamp"], c["date"]), reverse=True)
    return commits_list


class CommitMatcher:
    """Efficient matcher supporting full hashes and short prefixes."""

    def __init__(self, commit_patterns: Set[str]):
        self.exact_hashes: Set[str] = set()
        self.prefixes: List[str] = []

        for p in commit_patterns:
            cleaned = p.strip().lower()
            if not cleaned:
                continue
            if len(cleaned) == 40:
                self.exact_hashes.add(cleaned)
            else:
                self.prefixes.append(cleaned)

    def matches(self, commit_id: str) -> bool:
        cid = commit_id.strip().lower()
        if cid in self.exact_hashes:
            return True
        return any(cid.startswith(pref) for pref in self.prefixes)


class ChartRule:
    """Represents a rule targeting specific charts by regex pattern."""

    def __init__(self, pattern: str, action: str, param: Optional[str] = None):
        self.pattern_str = pattern
        if pattern == "*":
            self.pattern = re.compile(r".*")
        else:
            self.pattern = re.compile(pattern, re.IGNORECASE)
        self.action = action  # "since", "latest", "commits", "drop"
        self.param = param

    @classmethod
    def parse(cls, rule_str: str) -> "ChartRule":
        """
        Parse rule string formatted as '<PATTERN>:<ACTION>[=<PARAM>]'.

        Examples:
            'stackoverflow:since=745961ada'
            'cohere (vchord):latest=1'
            '*:drop'
        """
        parts = rule_str.split(":", 1)
        if len(parts) != 2:
            raise ValueError(
                f"Invalid chart rule format '{rule_str}'. Expected '<PATTERN>:<ACTION>[=<PARAM>]'"
            )
        pattern, action_spec = parts[0].strip(), parts[1].strip()
        if "=" in action_spec:
            action, param = action_spec.split("=", 1)
            action, param = action.strip().lower(), param.strip()
        else:
            action, param = action_spec.lower(), None

        if action not in ("since", "latest", "commits", "drop"):
            raise ValueError(
                f"Unknown action '{action}' in chart rule '{rule_str}'. "
                "Supported actions: since, latest, commits, drop."
            )
        return cls(pattern, action, param)

    def matches_chart(self, chart_name: str) -> bool:
        return bool(self.pattern.search(chart_name))

    def apply(
        self,
        points: List[Dict[str, Any]],
        unique_commits: List[Dict[str, Any]],
    ) -> List[Dict[str, Any]]:
        """Filter data points for a chart according to this rule."""
        if self.action == "drop":
            return []

        if self.action == "latest":
            n = int(self.param) if self.param else 1
            return points[-n:] if n > 0 else []

        if self.action == "since":
            if not self.param:
                return points
            cutoff_dt = resolve_cutoff_datetime(self.param, unique_commits)
            if not cutoff_dt:
                print(
                    f"Warning: Could not resolve cutoff '{self.param}' for chart rule '{self.pattern_str}'",
                    file=sys.stderr,
                )
                return points

            kept = []
            for p in points:
                pts = parse_iso_datetime(p.get("commit", {}).get("timestamp", ""))
                if pts:
                    if pts >= cutoff_dt:
                        kept.append(p)
                else:
                    pdate = p.get("date")
                    if pdate and datetime.fromtimestamp(pdate / 1000.0, tz=timezone.utc) >= cutoff_dt:
                        kept.append(p)
            return kept

        if self.action == "commits":
            hashes = [h.strip().lower() for h in (self.param or "").split(",") if h.strip()]
            matcher = CommitMatcher(set(hashes))
            return [p for p in points if matcher.matches(p.get("commit", {}).get("id", ""))]

        return points


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Prune benchmark data points and empty charts from data.js.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
examples:
  # Prune using chart-specific rules:
  %(prog)s --in-place \\
      --chart-rule "stackoverflow:since=745961ada" \\
      --chart-rule "cohere \\(pg_search\\):since=f76ae3d" \\
      --chart-rule "cohere \\(hnsw\\):since=2314882" \\
      --chart-rule "cohere \\(ivfflat\\):since=2314882" \\
      --chart-rule "cohere \\(vchord\\):latest=1" \\
      --chart-rule "cohere \\(pgvectorscale\\):latest=1" \\
      --chart-rule "*:drop"

  # Dry-run keeping only specific commits:
  %(prog)s 5ce8f7c 0b5f571 c07921a

  # Prune in-place keeping releases and the 10 most recent commits:
  %(prog)s --in-place --keep-releases --keep-recent 10

  # List all commits present in data.js:
  %(prog)s --list-commits
""",
    )

    parser.add_argument(
        "commits",
        nargs="*",
        help="Commit hashes (or prefixes) to keep globally.",
    )
    parser.add_argument(
        "-i",
        "--input",
        default=resolve_default_input_path(),
        help=f"Path to input data.js file (default: {DEFAULT_DATA_PATH}).",
    )
    parser.add_argument(
        "-o",
        "--output",
        help="Path to write output data.js file.",
    )
    parser.add_argument(
        "--in-place",
        action="store_true",
        help="Overwrite the input file in place (atomic write).",
    )
    parser.add_argument(
        "-n",
        "--dry-run",
        action="store_true",
        help="Print summary of changes without writing any files (default when neither --in-place nor --output is specified).",
    )
    parser.add_argument(
        "-r",
        "--chart-rule",
        action="append",
        dest="chart_rules",
        default=[],
        help=(
            "Apply a pruning rule to charts matching a regex pattern. Format: '<PATTERN>:<ACTION>[=<PARAM>]'. "
            "Supported actions: 'since=<COMMIT_OR_DATE>', 'latest=<N>', 'commits=<H1,H2,...>', or 'drop'. "
            "Can be repeated; evaluated in order of appearance."
        ),
    )
    parser.add_argument(
        "-c",
        "--commit",
        action="append",
        dest="flag_commits",
        default=[],
        help="Commit hash or prefix to keep (can be repeated).",
    )
    parser.add_argument(
        "--commits-file",
        help="File containing commit hashes to keep (one per line). Use '-' for stdin.",
    )
    parser.add_argument(
        "--keep-recent",
        type=int,
        metavar="N",
        help="Keep the N most recent commits by date/timestamp.",
    )
    parser.add_argument(
        "--since-commit",
        metavar="COMMIT",
        help="Keep the specified commit and all subsequent commits (by timestamp).",
    )
    parser.add_argument(
        "--keep-releases",
        action="store_true",
        help="Automatically keep commits identified as releases (via message pattern or git tags).",
    )
    parser.add_argument(
        "--message-pattern",
        help="Regular expression: keep commits whose commit message matches this regex.",
    )
    parser.add_argument(
        "--git-tags",
        action="store_true",
        help="Resolve and keep all commits associated with git tags.",
    )
    parser.add_argument(
        "--git-ref",
        action="append",
        default=[],
        help="Resolve and keep commits from a git revision or range (e.g. 'v0.25.0' or 'HEAD~10..HEAD').",
    )
    parser.add_argument(
        "--keep-empty-charts",
        action="store_true",
        help="Retain charts with 0 data points instead of pruning them.",
    )
    parser.add_argument(
        "--charts",
        help="Only include charts matching this regular expression.",
    )
    parser.add_argument(
        "--exclude-charts",
        help="Exclude charts matching this regular expression.",
    )
    parser.add_argument(
        "--update-timestamp",
        action="store_true",
        help="Update the 'lastUpdate' field to the current timestamp.",
    )
    parser.add_argument(
        "--list-commits",
        action="store_true",
        help="List all unique commits in the input file with metadata and exit.",
    )
    parser.add_argument(
        "--list-charts",
        action="store_true",
        help="List all charts and data point counts in the input file and exit.",
    )
    parser.add_argument(
        "-v",
        "--verbose",
        action="store_true",
        help="Enable verbose output.",
    )

    return parser.parse_args()


def main() -> int:
    args = parse_args()

    try:
        data, prefix, raw_content = load_benchmark_data(args.input)
    except Exception as e:
        print(f"Error loading input file: {e}", file=sys.stderr)
        return 1

    unique_commits = collect_unique_commits(data)

    if args.list_commits:
        print(f"Unique commits in {args.input} ({len(unique_commits)} total):")
        print(f"{'Short SHA':<10} {'Timestamp / Date':<22} {'Points':<8} {'Message'}")
        print("-" * 80)
        for c in unique_commits:
            short_sha = c["id"][:7]
            ts = c["timestamp"] or str(c["date"])
            msg_line = c["message"].split("\n", 1)[0][:50]
            print(f"{short_sha:<10} {ts:<22} {c['occurrences']:<8} {msg_line}")
        return 0

    if args.list_charts:
        entries = data.get("entries", {})
        print(f"Charts in {args.input} ({len(entries)} total):")
        for chart_name, points in entries.items():
            print(f"  - {chart_name}: {len(points)} points")
        return 0

    chart_rules: List[ChartRule] = []
    for r_str in args.chart_rules:
        try:
            chart_rules.append(ChartRule.parse(r_str))
        except ValueError as e:
            print(f"Error: {e}", file=sys.stderr)
            return 1

    # Collect global commits to keep
    target_commits: Set[str] = set()

    for c in args.commits:
        target_commits.add(c)
    for c in args.flag_commits:
        target_commits.add(c)

    if args.commits_file:
        try:
            for c in read_commits_from_file(args.commits_file):
                target_commits.add(c)
        except Exception as e:
            print(f"Error reading commits file: {e}", file=sys.stderr)
            return 1

    if args.keep_recent is not None and args.keep_recent > 0:
        for c in unique_commits[: args.keep_recent]:
            target_commits.add(c["id"])

    if args.since_commit:
        cutoff_dt = resolve_cutoff_datetime(args.since_commit, unique_commits)
        if cutoff_dt:
            for c in unique_commits:
                c_dt = parse_iso_datetime(c["timestamp"])
                if c_dt:
                    if c_dt >= cutoff_dt:
                        target_commits.add(c["id"])
                elif c["date"]:
                    if datetime.fromtimestamp(c["date"] / 1000.0, tz=timezone.utc) >= cutoff_dt:
                        target_commits.add(c["id"])
        else:
            # Fall back to resolving via git rev-list
            git_shas = resolve_git_rev(f"{args.since_commit}^..HEAD")
            if not git_shas:
                git_shas = resolve_git_rev(f"{args.since_commit}^..upstream/main")
            if git_shas:
                for sha in git_shas:
                    target_commits.add(sha)
            else:
                print(
                    f"Warning: --since-commit '{args.since_commit}' was not found in data.js or git.",
                    file=sys.stderr,
                )

    if args.keep_releases:
        # Match commits by message pattern
        for c in unique_commits:
            if RELEASE_COMMIT_PATTERN.search(c["message"]):
                target_commits.add(c["id"])
        # Also resolve git tags if git is present
        for sha in resolve_git_tags():
            target_commits.add(sha)

    if args.message_pattern:
        pattern = re.compile(args.message_pattern, re.IGNORECASE)
        for c in unique_commits:
            if pattern.search(c["message"]):
                target_commits.add(c["id"])

    if args.git_tags:
        for sha in resolve_git_tags():
            target_commits.add(sha)

    for rev in args.git_ref:
        for sha in resolve_git_rev(rev):
            target_commits.add(sha)

    if not target_commits and not chart_rules:
        print(
            "Error: No commits or chart rules specified to keep. "
            "Use --chart-rule, --since-commit, --commits, or --keep-releases.",
            file=sys.stderr,
        )
        return 1

    matcher = CommitMatcher(target_commits) if target_commits else None

    # Compile chart filters if requested
    chart_include_filter = re.compile(args.charts) if args.charts else None
    chart_exclude_filter = re.compile(args.exclude_charts) if args.exclude_charts else None

    # Filter data points and charts
    orig_entries = data.get("entries", {})
    total_points_orig = sum(len(pts) for pts in orig_entries.values())
    total_charts_orig = len(orig_entries)

    pruned_entries: Dict[str, List[Any]] = {}
    matched_commits: Set[str] = set()
    total_points_kept = 0

    for chart_name, points in orig_entries.items():
        if chart_include_filter and not chart_include_filter.search(chart_name):
            continue
        if chart_exclude_filter and chart_exclude_filter.search(chart_name):
            continue

        # Check chart-specific rules first
        matched_rule: Optional[ChartRule] = None
        for rule in chart_rules:
            if rule.matches_chart(chart_name):
                matched_rule = rule
                break

        if matched_rule is not None:
            kept_points = matched_rule.apply(points, unique_commits)
        elif matcher is not None:
            kept_points = [p for p in points if matcher.matches(p.get("commit", {}).get("id", ""))]
        elif chart_rules:
            # If rules are defined but no rule matched and no global filter exists, keep points as is
            kept_points = points
        else:
            kept_points = []

        for p in kept_points:
            cid = p.get("commit", {}).get("id")
            if cid:
                matched_commits.add(cid)

        if kept_points or args.keep_empty_charts:
            pruned_entries[chart_name] = kept_points
            total_points_kept += len(kept_points)

    total_charts_kept = len(pruned_entries)
    points_removed = total_points_orig - total_points_kept
    charts_removed = total_charts_orig - total_charts_kept

    new_data = dict(data)
    new_data["entries"] = pruned_entries

    if args.update_timestamp:
        import time

        new_data["lastUpdate"] = int(time.time() * 1000)

    serialized = dump_benchmark_data(new_data, prefix)
    orig_size = len(raw_content.encode("utf-8"))
    new_size = len(serialized.encode("utf-8"))
    size_reduction = (1 - (new_size / orig_size)) * 100 if orig_size > 0 else 0.0

    print("Pruning summary:")
    if chart_rules:
        print(f"  Active chart rules:        {len(chart_rules)}")
    if target_commits:
        print(f"  Global target commits:     {len(target_commits)}")
    print(f"  Unique commits matched:    {len(matched_commits)}")
    print(
        f"  Data points:               {total_points_orig} -> {total_points_kept} "
        f"({points_removed} removed, {100 * total_points_kept / total_points_orig:.1f}% retained)"
    )
    print(
        f"  Charts:                    {total_charts_orig} -> {total_charts_kept} "
        f"({charts_removed} removed, {total_charts_kept} retained)"
    )
    print(
        f"  File size:                 {orig_size / (1024 * 1024):.2f} MB -> "
        f"{new_size / (1024 * 1024):.2f} MB ({size_reduction:.1f}% reduction)"
    )

    if args.verbose:
        print("\nKept charts:")
        for name, pts in pruned_entries.items():
            commits_summary = ", ".join(p["commit"]["id"][:7] for p in pts)
            print(f"  - {name}: {len(pts)} points ({commits_summary})")

    is_dry_run = args.dry_run or (not args.in_place and not args.output)

    if is_dry_run:
        if not args.dry_run:
            print("\nNotice: Dry-run mode by default. Specify --in-place to overwrite or -o <file> to write output.")
        return 0

    target_out = args.input if args.in_place else args.output
    if not target_out:
        print("Error: Target output path could not be determined.", file=sys.stderr)
        return 1

    try:
        atomic_write(target_out, serialized)
        print(f"\nSuccessfully wrote pruned data to: {target_out}")
    except Exception as e:
        print(f"Error writing to {target_out}: {e}", file=sys.stderr)
        return 1

    return 0


if __name__ == "__main__":
    sys.exit(main())
