# /// script
# requires-python = ">=3.12,<3.13"
# dependencies = ["duckdb==1.4.3", "pyarrow==21.0.0", "tiktoken==0.12.0"]
# ///
"""Profile HN and select a reproducible query holdout without calling an API."""

# pylint: disable=import-error

import argparse
import csv
import hashlib
import itertools
import json
import multiprocessing
import os
import platform
import struct
import subprocess
import tempfile
import time
import tomllib
from collections import deque
from concurrent.futures import ProcessPoolExecutor, ThreadPoolExecutor
from functools import lru_cache
from html.parser import HTMLParser
from pathlib import Path

import duckdb
import pyarrow as pa
import pyarrow.parquet as pq
import tiktoken

DEFAULT_CONFIG = Path(__file__).resolve().parents[1] / "datasets/hn/prepare.toml"
PROFILE_SCHEMA = pa.schema(
    [
        ("hn_id", pa.int64()),
        ("normalized_sha256", pa.binary(32)),
        ("input_sha256", pa.binary(32)),
        ("input_tokens", pa.int32()),
        ("original_tokens", pa.int32()),
    ]
)
HOLDOUT_SCHEMA = pa.schema(
    [
        ("id", pa.int32()),
        ("hn_id", pa.int64()),
        ("normalized_sha256", pa.string()),
        ("input_sha256", pa.string()),
        ("token_ids", pa.list_(pa.int32())),
    ]
)


class HNText(HTMLParser):
    """Extract visible text, separating block elements and retaining inline text."""

    BLOCKS = frozenset({"p", "br", "div", "pre", "blockquote", "li", "ul", "ol", "hr"})

    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.parts = []
        self.hidden = 0

    def handle_starttag(self, tag, attrs):
        if tag in {"script", "style"}:
            self.hidden += 1
        if tag in self.BLOCKS:
            self.parts.append(" ")

    def handle_endtag(self, tag):
        if tag in {"script", "style"}:
            self.hidden = max(0, self.hidden - 1)
        if tag in self.BLOCKS:
            self.parts.append(" ")

    def handle_data(self, data):
        if not self.hidden:
            self.parts.append(data)


def normalize_text(text):
    """Apply the hn-html-text-v1 input policy without modifying source rows."""
    if text is None or text == "\\N":
        return ""
    parser = HNText()
    parser.feed(text)
    parser.close()
    return " ".join("".join(parser.parts).split())


@lru_cache(maxsize=1)
def tokenizer():
    """Reuse one tokenizer per preparation worker."""
    return tiktoken.get_encoding("cl100k_base")


def input_digest(tokens):
    """Hash the exact token-array input with platform-independent byte order."""
    return hashlib.sha256(struct.pack(f"<{len(tokens)}I", *tokens)).digest()


def file_digest(path):
    """Hash a file without materializing it in memory."""
    with Path(path).open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def load_config(path):
    """Reject settings that diverge from the implemented HN v1 contract."""
    with Path(path).open("rb") as source:
        config = tomllib.load(source)
    expected = {
        "model": "text-embedding-3-small",
        "dimensions": 1536,
        "input_column": "text",
        "preprocessing": "hn-html-text-v1",
        "encoding": "cl100k_base",
        "max_tokens": 8192,
        "distance": "cosine",
    }
    if config["embedding"] != expected:
        raise ValueError(
            "Unsupported embedding contract; update the preparer and schema together"
        )
    queries = config["queries"]
    if not 0 < queries["count"] <= 2**31 - 1:
        raise ValueError("queries.count must fit a positive int32")
    if queries["require_unique_normalized_text"] is not True:
        raise ValueError("The v1 holdout requires unique normalized text")
    if config["dataset"] != "hn" or config["version"] != "v1":
        raise ValueError("This preparer implements the hn/v1 contract")
    for key in ("artifact_prefix", "snapshot_prefix"):
        if not config[key].startswith("s3://"):
            raise ValueError(f"{key} must be an S3 URI")
    return config


def profile_batch(rows, destination, max_tokens):
    """Write bounded per-row metadata for duplicate detection and token accounting."""
    records = []
    for hn_id, raw_text in rows:
        text = normalize_text(raw_text)
        tokens = tokenizer().encode_ordinary(text)
        clipped = tokens[:max_tokens]
        records.append(
            {
                "hn_id": hn_id,
                "normalized_sha256": hashlib.sha256(text.encode()).digest()
                if text
                else None,
                "input_sha256": input_digest(clipped) if clipped else None,
                "input_tokens": len(clipped),
                "original_tokens": len(tokens),
            }
        )
    pq.write_table(pa.Table.from_pylist(records, schema=PROFILE_SCHEMA), destination)
    return len(records)


def profile_csv(source, work, max_tokens, workers):
    """Stream CSV batches through workers with bounded pending work."""
    pending = deque()
    completed = 0
    started = time.monotonic()
    with ProcessPoolExecutor(
        max_workers=workers, mp_context=multiprocessing.get_context("spawn")
    ) as pool:
        reader = read_rows(source)
        for shard in itertools.count():
            rows = list(itertools.islice(reader, 10000))
            if not rows:
                break
            pending.append(
                pool.submit(
                    profile_batch, rows, work / f"{shard:06}.parquet", max_tokens
                )
            )
            if len(pending) >= workers * 2:
                completed += pending.popleft().result()
                if completed % 100000 == 0:
                    print(
                        f"Profiled {completed:,} rows "
                        f"({completed / (time.monotonic() - started):,.0f}/s)",
                        flush=True,
                    )
        for future in pending:
            completed += future.result()
    if not completed:
        raise ValueError("Source CSV has no rows")


def read_rows(source):
    """Stream validated HN IDs and original text for each preparation stage."""
    csv.field_size_limit(2**31 - 1)
    with Path(source).open(newline="", encoding="utf-8") as stream:
        reader = csv.DictReader(stream, strict=True)
        if not reader.fieldnames or not {"id", "text"}.issubset(reader.fieldnames):
            raise ValueError("Source CSV must contain id and text columns")
        if len(set(reader.fieldnames)) != len(reader.fieldnames):
            raise ValueError("Source CSV contains duplicate column names")
        for row in reader:
            if None in row or any(value is None for value in row.values()):
                raise ValueError(
                    f"Malformed CSV record ending on line {reader.line_num}"
                )
            hn_id = int(row["id"])
            if not 0 < hn_id <= 2**63 - 1:
                raise ValueError(f"Invalid HN id: {hn_id}")
            yield hn_id, row["text"]


def summarize(work, config, memory_limit):
    """Validate identities, aggregate counts, and select globally unique inputs."""
    with duckdb.connect() as conn:
        conn.execute("SET memory_limit = ?", [memory_limit])
        conn.execute("SET threads = 2")
        conn.execute("SET temp_directory = ?", [str(work / "spill")])
        conn.read_parquet(str(work / "*.parquet")).create_view("rows")
        if conn.execute(
            "SELECT count(*) != count(DISTINCT hn_id) FROM rows"
        ).fetchone()[0]:
            raise ValueError("Source contains duplicate HN IDs")
        conn.execute("""
            CREATE TABLE inputs AS
            SELECT input_sha256, min(input_tokens) AS tokens, count(*) AS copies
            FROM rows WHERE input_tokens > 0 GROUP BY input_sha256
        """)
        columns = [
            "source_rows",
            "empty_rows",
            "truncated_rows",
            "tokens_before_truncation",
            "tokens",
        ]
        stats = dict(
            zip(
                columns,
                conn.execute("""
                    SELECT count(*), count(*) FILTER (WHERE input_tokens = 0),
                           count(*) FILTER (WHERE original_tokens > input_tokens),
                           sum(original_tokens), sum(input_tokens)
                    FROM rows
                """).fetchone(),
                strict=True,
            )
        )
        stats["unique_inputs"], stats["tokens_with_deduplication"] = conn.execute(
            "SELECT count(*), coalesce(sum(tokens), 0) FROM inputs"
        ).fetchone()
        (
            stats["duplicate_normalized_texts"],
            stats["rows_in_duplicate_normalized_texts"],
        ) = conn.execute("""
                SELECT count(*), coalesce(sum(copies), 0) FROM (
                    SELECT count(*) AS copies FROM rows WHERE input_tokens > 0
                    GROUP BY normalized_sha256 HAVING count(*) > 1
                )
            """).fetchone()
        candidates = conn.execute(
            """
            SELECT hn_id, normalized_sha256, input_sha256 FROM rows
            JOIN inputs USING (input_sha256) WHERE copies = 1
            ORDER BY sha256(? || ':' || hn_id::VARCHAR), hn_id LIMIT ?
            """,
            [str(config["queries"]["seed"]), config["queries"]["count"]],
        ).fetchall()
    if len(candidates) != config["queries"]["count"]:
        raise ValueError(
            f"Only {len(candidates)} unique inputs are available for the query holdout"
        )
    stats["query_rows"] = len(candidates)
    stats["corpus_rows"] = stats["source_rows"] - len(candidates)
    stats["corpus_vector_rows"] = stats["corpus_rows"] - stats["empty_rows"]
    stats["raw_corpus_vector_bytes"] = (
        stats["corpus_vector_rows"] * config["embedding"]["dimensions"] * 4
    )
    return stats, candidates


def write_holdout(source, output, candidates, max_tokens):
    """Recover selected inputs from the source and verify their profiled hashes."""
    selected = {row[0]: (query_id, *row[1:]) for query_id, row in enumerate(candidates)}
    records = []
    with source.open(newline="", encoding="utf-8") as stream:
        for row in csv.DictReader(stream, strict=True):
            hn_id = int(row["id"])
            if hn_id not in selected:
                continue
            query_id, normalized_hash, input_hash = selected.pop(hn_id)
            text = normalize_text(row["text"])
            tokens = tokenizer().encode_ordinary(text)[:max_tokens]
            if (
                hashlib.sha256(text.encode()).digest() != normalized_hash
                or input_digest(tokens) != input_hash
            ):
                raise ValueError(f"Source text changed for held-out HN ID {hn_id}")
            records.append(
                {
                    "id": query_id,
                    "hn_id": hn_id,
                    "normalized_sha256": normalized_hash.hex(),
                    "input_sha256": input_hash.hex(),
                    "token_ids": tokens,
                }
            )
            if not selected:
                break
    if selected:
        raise ValueError("Held-out HN IDs disappeared from the source")
    records.sort(key=lambda record: record["id"])
    queries_dir = output / "queries"
    queries_dir.mkdir()
    pq.write_table(
        pa.Table.from_pylist(records, schema=HOLDOUT_SCHEMA),
        queries_dir / "holdout.parquet",
    )


def prepare(source, output, config_path=DEFAULT_CONFIG, workers=4, memory_limit="2GB"):
    """Write the profile manifest and holdout into an empty local directory."""
    source, output, config_path = map(Path, (source, output, config_path))
    config = load_config(config_path)
    if workers < 1:
        raise ValueError("workers must be positive")
    before = source.stat()
    output.mkdir(parents=True, exist_ok=True)
    if any(output.iterdir()):
        raise ValueError(f"Output directory must be empty: {output}")
    provenance = {
        "git_revision": subprocess.run(
            ["git", "rev-parse", "HEAD"],
            cwd=Path(__file__).parent,
            capture_output=True,
            text=True,
            check=False,
        ).stdout.strip()
        or None,
        "script_sha256": file_digest(__file__),
        "config_sha256": file_digest(config_path),
        "python": platform.python_version(),
        "duckdb": duckdb.__version__,
        "pyarrow": pa.__version__,
        "tiktoken": tiktoken.__version__,
        "holdout_order": "sha256(decimal_seed + ':' + decimal_hn_id), hn_id",
        "holdout_eligibility": "nonempty, unique truncated token sequence",
        "input_sha256_encoding": "little-endian uint32 token IDs",
    }
    with tempfile.TemporaryDirectory(prefix=".profile-", dir=output) as scratch:
        with ThreadPoolExecutor(max_workers=1) as hashing:
            checksum = hashing.submit(file_digest, source)
            profile_csv(
                source, Path(scratch), config["embedding"]["max_tokens"], workers
            )
            checksum = checksum.result()
        print("Counting duplicates and selecting the query holdout...", flush=True)
        stats, candidates = summarize(Path(scratch), config, memory_limit)
    print("Reading the held-out inputs...", flush=True)
    write_holdout(source, output, candidates, config["embedding"]["max_tokens"])
    after = source.stat()
    if (after.st_size, after.st_mtime_ns, after.st_ino) != (
        before.st_size,
        before.st_mtime_ns,
        before.st_ino,
    ):
        raise ValueError("Source CSV changed during profiling")
    manifest = {
        "state": "profiled",
        "config": config,
        "source": {
            "archive_uri": config["source"],
            "archive_verified": False,
            "csv_bytes": before.st_size,
            "csv_sha256": checksum,
        },
        "preparation": provenance,
        "counts": stats,
        "estimated_embedding_cost_usd": {
            mode: round(
                stats["tokens_with_deduplication"]
                * config["pricing"][f"{mode}_usd_per_million_tokens"]
                / 1_000_000,
                6,
            )
            for mode in ("standard", "batch")
        },
        "artifacts": {
            "queries/holdout.parquet": {
                "sha256": file_digest(output / "queries/holdout.parquet"),
                "rows": stats["query_rows"],
            }
        },
    }
    (output / "manifest.json").write_text(
        json.dumps(manifest, indent=2) + "\n", encoding="utf-8"
    )
    return manifest


def main():
    """Run the local preparation dry run from the command line."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--source", type=Path, required=True, help="Extracted HN data.csv"
    )
    parser.add_argument(
        "--output", type=Path, required=True, help="Empty local artifact directory"
    )
    parser.add_argument("--config", type=Path, default=DEFAULT_CONFIG)
    parser.add_argument("--workers", type=int, default=min(4, os.cpu_count() or 1))
    parser.add_argument(
        "--memory-limit",
        default="2GB",
        help="DuckDB memory limit; more memory reduces aggregation spill",
    )
    args = parser.parse_args()
    manifest = prepare(
        args.source, args.output, args.config, args.workers, args.memory_limit
    )
    print(
        json.dumps(
            {
                "counts": manifest["counts"],
                "cost_usd": manifest["estimated_embedding_cost_usd"],
            },
            indent=2,
        )
    )


if __name__ == "__main__":
    main()
