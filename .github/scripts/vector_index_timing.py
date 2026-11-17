"""Capture PostgreSQL statement durations for vector index finalization."""

import argparse
import json
import os
import re
from pathlib import Path
from urllib.parse import parse_qsl, quote, urlencode, urlsplit

DURATION = re.compile(
    r"duration: ([0-9]+(?:\.[0-9]+)?) ms\s+statement:\s*(.*?)(?=^\d{4}-\d{2}-\d{2} |\Z)",
    re.MULTILINE | re.DOTALL,
)
CREATE_INDEX = re.compile(
    r"^CREATE\s+(?:UNIQUE\s+)?INDEX\s+(?:CONCURRENTLY\s+)?"
    r'(?:IF\s+NOT\s+EXISTS\s+)?("(?:[^"]|"")*"|\w+)\s+ON\b',
    re.IGNORECASE,
)


def timed_url(url):
    """Enable duration logging on the finalize connection without changing server defaults."""
    parts = urlsplit(url)
    query = parse_qsl(parts.query, keep_blank_values=True)
    options = " ".join(value for key, value in query if key == "options")
    query = [(key, value) for key, value in query if key != "options"]
    query.append(("options", f"{options} -c log_min_duration_statement=0".strip()))
    base = url.partition("?")[0].partition("#")[0]
    fragment = f"#{parts.fragment}" if parts.fragment else ""
    return f"{base}?{urlencode(query, quote_via=quote)}{fragment}"


def statement_timings(log, index):
    """Select the named index's CREATE duration and retain separate statement timings."""
    statements = []
    builds = []
    for duration, sql in DURATION.findall(log):
        sql = sql.strip()
        statement = {"sql": sql, "seconds": float(duration) / 1000}
        statements.append(statement)
        # Dataset scripts may put comments before the CREATE INDEX statement.
        command = re.sub(
            r"\A(?:\s|--[^\n]*(?:\n|$)|/\*.*?\*/)*", "", sql, flags=re.DOTALL
        )
        match = CREATE_INDEX.match(command)
        if match:
            name = match[1]
            name = (
                name[1:-1].replace('""', '"') if name.startswith('"') else name.lower()
            )
            if name == index:
                builds.append(statement)
    if len(builds) != 1 or builds[0]["seconds"] <= 0:
        raise ValueError(
            f"Expected one positive CREATE INDEX duration for {index}; found {len(builds)}"
        )
    return {
        "index": index,
        "build_seconds": builds[0]["seconds"],
        "statements": statements,
    }


def main():
    """Configure the finalize URL or extract its recorded statement durations."""
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("url")
    extract = commands.add_parser("extract")
    extract.add_argument("log", type=Path)
    extract.add_argument("index")
    args = parser.parse_args()
    if args.command == "url":
        print(timed_url(os.environ["PARADEDB_URL"]))
    else:
        print(json.dumps(statement_timings(args.log.read_text(), args.index), indent=2))


if __name__ == "__main__":
    main()
