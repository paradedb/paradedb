#!/usr/bin/env python3
"""Prepare benchmark-action JSON and stage its results in the shared pages checkout."""

import argparse
import json
import os
from pathlib import Path

PREFIX = "window.BENCHMARK_DATA = "


def read_history(path):
    """Load chart data without silently discarding malformed history."""
    if not path.exists():
        return {"lastUpdate": 0, "repoUrl": "", "entries": {}}
    text = path.read_text()
    if not text.startswith(PREFIX):
        raise ValueError("Unexpected benchmark history prefix")
    return json.loads(text[len(PREFIX) :].strip().removesuffix(";"))


def write_comparison(directory, suite, entries):
    """Save the baseline/result pair consumed by comparison comments."""
    comparison = {"entries": {suite: entries}}
    (directory / "comparison.js").write_text(PREFIX + json.dumps(comparison))


def prepare(history_path, directory, suite, publication_id, publish_baseline):
    """Recover a published comparison or seed the action's scratch JSON."""
    history = read_history(history_path)
    entries = history.get("entries", {}).get(suite, [])
    position = next(
        (
            i
            for i, entry in enumerate(entries)
            if entry.get("publication_id") == publication_id
        ),
        None,
    )
    published = publish_baseline and position is not None
    directory.mkdir(parents=True, exist_ok=True)
    if published:
        write_comparison(directory, suite, entries[max(0, position - 1) : position + 1])
    else:
        (directory / "history.json").write_text(json.dumps(history))
    return published


def record(history_path, directory, suite, publication_id, publish_baseline):
    """Save the action's comparison and optionally advance the shared baseline."""
    history = json.loads((directory / "history.json").read_text())
    entries = history.get("entries", {}).get(suite, [])
    if not entries:
        raise ValueError("Publisher did not append a result")
    entries[-1]["publication_id"] = publication_id
    write_comparison(directory, suite, entries[-2:])
    if publish_baseline:
        history_path.parent.mkdir(parents=True, exist_ok=True)
        history_path.write_text(PREFIX + json.dumps(history, indent=2))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("operation", choices=["prepare", "record"])
    args = parser.parse_args()
    publication_history = (
        Path("benchmark-data-repository") / os.environ["DATA_DIRECTORY"] / "data.js"
    )
    parameters = (
        publication_history,
        Path("_benchmark_publish"),
        os.environ["SUITE"],
        os.environ["PUBLICATION_ID"],
        os.environ["PUBLISH_BASELINE"] == "true",
    )
    if args.operation == "prepare":
        print(f"published={str(prepare(*parameters)).lower()}")
    else:
        record(*parameters)
