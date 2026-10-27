"""Export Cohere index build measurements for the benchmark publisher."""

import json
import re
import sys
from pathlib import Path


def metrics(build_log, stats):
    """Measure CREATE INDEX alone and retain pre-vacuum size diagnostics."""
    timings = re.findall(
        r"^CREATE INDEX\nTime: ([0-9]+(?:\.[0-9]+)?) ms\b", build_log, re.MULTILINE
    )
    if len(timings) != 1 or float(timings[0]) <= 0:
        raise ValueError("Expected exactly one successful CREATE INDEX timing")
    for key in ("index_bytes", "segments", "indexed_docs"):
        value = stats.get(key)
        if not isinstance(value, int) or isinstance(value, bool) or value <= 0:
            raise ValueError(f"Expected positive {key}")
    extra = f"segments={stats['segments']}, indexed_docs={stats['indexed_docs']}"
    return [
        {
            "name": "cohere_wiki_bm25_idx build time",
            "unit": "min",
            "value": float(timings[0]) / 60_000,
            "extra": extra,
        },
        {
            "name": "cohere_wiki_bm25_idx index size",
            "unit": "MiB",
            "value": stats["index_bytes"] / (1024 * 1024),
            "extra": extra,
        },
    ]


def main():
    """Read the captured build outputs and write publisher-compatible JSON."""
    directory = Path(sys.argv[1])
    result = metrics(
        (directory / "index-build.log").read_text(),
        json.loads((directory / "index-stats.json").read_text()),
    )
    (directory / "index-metrics.json").write_text(json.dumps(result, indent=2) + "\n")


if __name__ == "__main__":
    main()
