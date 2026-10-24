"""Record publication IDs in benchmark history and recover original comparisons."""

import argparse
import json
from pathlib import Path

PREFIX = "window.BENCHMARK_DATA = "


def prepare(history_path, suite, publication_id, comparison_path, *, record=False):
    """Return whether this publication exists, optionally tagging a newly appended run."""
    if not history_path.exists():
        if record:
            raise ValueError("Publisher did not create benchmark history")
        return False
    text = history_path.read_text(encoding="utf-8")
    if not text.startswith(PREFIX):
        raise ValueError("Unexpected benchmark history prefix")
    history = json.loads(text[len(PREFIX) :].strip().removesuffix(";"))
    entries = history.get("entries", {}).get(suite, [])
    position = next(
        (
            i
            for i, entry in enumerate(entries)
            if entry.get("publication_id") == publication_id
        ),
        None,
    )
    if position is None:
        if not record:
            return False
        if not entries:
            raise ValueError(f"Publisher did not append a result for {suite}")
        position = len(entries) - 1
        entries[position]["publication_id"] = publication_id
        history_path.write_text(
            PREFIX + json.dumps(history, indent=2), encoding="utf-8"
        )

    # Later publications may have appended runs since the failed attempt. Keep the
    # predecessor of this exact publication, rather than the newest history entry.
    comparison = {"entries": {suite: entries[max(0, position - 1) : position + 1]}}
    comparison_path.parent.mkdir(parents=True, exist_ok=True)
    comparison_path.write_text(PREFIX + json.dumps(comparison), encoding="utf-8")
    return True


def main():
    """Check for a completed metric or attach its ID before pushing the local commit."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--history", required=True, type=Path)
    parser.add_argument("--suite", required=True)
    parser.add_argument("--publication-id", required=True)
    parser.add_argument("--comparison", required=True, type=Path)
    parser.add_argument("--record", action="store_true")
    args = parser.parse_args()
    exists = prepare(
        args.history,
        args.suite,
        args.publication_id,
        args.comparison,
        record=args.record,
    )
    print("true" if exists else "false")


if __name__ == "__main__":
    main()
