# /// script
# requires-python = ">=3.12,<3.13"
# dependencies = ["duckdb==1.4.3", "pyarrow==21.0.0", "tiktoken==0.12.0"]
# ///
"""Build deduplicated, bounded OpenAI Batch requests from a profiled HN CSV."""

import argparse
import json
import os
import shutil
import sqlite3
import tempfile
from concurrent.futures import ThreadPoolExecutor
from dataclasses import asdict, dataclass
from pathlib import Path

import prepare_hn as hn


def json_bytes(value):
    """Serialize checksummed metadata consistently."""
    return (
        json.dumps(value, sort_keys=True, separators=(",", ":"), allow_nan=False) + "\n"
    ).encode()


def write_json(path, value):
    """Replace metadata atomically after flushing its contents."""
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(path.suffix + ".tmp")
    with temporary.open("wb") as stream:
        stream.write(json_bytes(value))
        stream.flush()
        os.fsync(stream.fileno())
    temporary.replace(path)


@dataclass(frozen=True)
class Limits:
    """Bound both individual embedding requests and asynchronous batch files."""

    request_inputs: int = 128
    request_tokens: int = 250000
    batch_inputs: int = 50000
    batch_tokens: int = 5000000
    batch_bytes: int = 180000000

    def validate(self):
        """Reject limits that cannot fit a single input or exceed API limits."""
        if not 1 <= self.request_inputs <= min(2048, self.batch_inputs):
            raise ValueError("Invalid request input limit")
        if not 8192 <= self.request_tokens <= min(300000, self.batch_tokens):
            raise ValueError("Invalid request token limit")
        if not self.request_inputs <= self.batch_inputs <= 50000:
            raise ValueError("Invalid batch input limit")
        if not 1 <= self.batch_bytes <= 200000000:
            raise ValueError("Invalid batch byte limit")


class RequestWriter:
    """Pack bounded request files without keeping a corpus-sized token list."""

    def __init__(self, root, embedding, limits):
        self.root = root
        self.embedding = embedding
        self.limits = limits
        self.units = []
        self.lines = []
        self.counts = {"inputs": 0, "tokens": 0, "bytes": 0}
        self.request_number = 0

    def add(self, inputs):
        """Append one request, flushing the current batch before it overflows."""
        request = {
            "custom_id": f"hn-{self.request_number:09}",
            "method": "POST",
            "url": "/v1/embeddings",
            "body": {
                "model": self.embedding["model"],
                "dimensions": self.embedding["dimensions"],
                "encoding_format": "base64",
                "input": inputs,
            },
        }
        line = json_bytes(request)
        tokens = sum(map(len, inputs))
        if len(line) > self.limits.batch_bytes:
            raise ValueError("A single request exceeds the batch file byte limit")
        if self.lines and (
            self.counts["inputs"] + len(inputs) > self.limits.batch_inputs
            or self.counts["tokens"] + tokens > self.limits.batch_tokens
            or self.counts["bytes"] + len(line) > self.limits.batch_bytes
        ):
            self.flush()
        self.lines.append(line)
        self.counts["inputs"] += len(inputs)
        self.counts["tokens"] += tokens
        self.counts["bytes"] += len(line)
        self.request_number += 1

    def flush(self):
        """Write an immutable batch file and record its checksum and counts."""
        if not self.lines:
            return
        relative = f"requests/{len(self.units):06}.jsonl"
        destination = self.root / relative
        destination.parent.mkdir(parents=True, exist_ok=True)
        with destination.open("wb") as stream:
            stream.writelines(self.lines)
        self.units.append(
            {
                "path": relative,
                "sha256": hn.file_digest(destination),
                **self.counts,
                "requests": len(self.lines),
            }
        )
        self.lines = []
        self.counts = dict.fromkeys(self.counts, 0)


def unique_inputs(source, database, stats, max_tokens):
    """Deduplicate exact API inputs using a disposable on-disk hash index."""
    with sqlite3.connect(database) as conn:
        conn.execute("PRAGMA journal_mode = OFF")
        conn.execute("PRAGMA synchronous = OFF")
        conn.execute("PRAGMA cache_size = -65536")
        conn.execute("CREATE TABLE seen (hash BLOB PRIMARY KEY) WITHOUT ROWID")
        for _, text in hn.read_rows(source):
            tokens = hn.tokenizer().encode_ordinary(hn.normalize_text(text))
            clipped = tokens[:max_tokens]
            stats["source_rows"] += 1
            stats["empty_rows"] += not clipped
            stats["truncated_rows"] += len(tokens) > len(clipped)
            stats["tokens_before_truncation"] += len(tokens)
            stats["tokens"] += len(clipped)
            if (
                clipped
                and conn.execute(
                    "INSERT OR IGNORE INTO seen VALUES (?)", [hn.input_digest(clipped)]
                ).rowcount
            ):
                stats["unique_inputs"] += 1
                stats["tokens_with_deduplication"] += len(clipped)
                yield clipped
            if stats["source_rows"] % 100000 == 0:
                conn.commit()
                print(f"Planned {stats['source_rows']:,} source rows", flush=True)


def pack_requests(inputs, writer):
    """Keep each request within both the input-count and total-token limits."""
    pending = []
    tokens = 0
    for item in inputs:
        if pending and (
            len(pending) == writer.limits.request_inputs
            or tokens + len(item) > writer.limits.request_tokens
        ):
            writer.add(pending)
            pending, tokens = [], 0
        pending.append(item)
        tokens += len(item)
    if pending:
        writer.add(pending)
    writer.flush()


def validate_profile(profile_path):
    """Check the profile contract and its holdout before creating API inputs."""
    profile = json.loads(profile_path.read_text())
    expected = hn.load_config(hn.DEFAULT_CONFIG)
    if profile["state"] != "profiled" or any(
        profile["config"][key] != expected[key]
        for key in ("dataset", "version", "embedding")
    ):
        raise ValueError("Profile does not match the supported HN embedding contract")
    holdout = profile_path.parent / "queries/holdout.parquet"
    if (
        hn.file_digest(holdout)
        != profile["artifacts"]["queries/holdout.parquet"]["sha256"]
    ):
        raise ValueError("Profile holdout checksum mismatch")
    return profile


def build_plan(source, profile_path, output, limits=None):
    """Verify the source and publish a complete local request plan, without API calls."""
    source, profile_path, output = map(Path, (source, profile_path, output))
    limits = limits or Limits()
    limits.validate()
    profile = validate_profile(profile_path)
    if source.stat().st_size != profile["source"]["csv_bytes"]:
        raise ValueError("Source CSV size does not match the profile")
    output.mkdir(parents=True, exist_ok=True)
    if any(output.iterdir()):
        raise ValueError(f"Output directory must be empty: {output}")
    stats = dict.fromkeys(
        (
            "source_rows",
            "empty_rows",
            "truncated_rows",
            "tokens_before_truncation",
            "tokens",
            "unique_inputs",
            "tokens_with_deduplication",
        ),
        0,
    )
    before = source.stat()
    with tempfile.TemporaryDirectory(prefix=".plan-", dir=output) as scratch:
        work = Path(scratch)
        writer = RequestWriter(work, profile["config"]["embedding"], limits)
        with ThreadPoolExecutor(max_workers=1) as hashing:
            checksum = hashing.submit(hn.file_digest, source)
            pack_requests(
                unique_inputs(
                    source,
                    work / "seen.sqlite",
                    stats,
                    profile["config"]["embedding"]["max_tokens"],
                ),
                writer,
            )
            if checksum.result() != profile["source"]["csv_sha256"]:
                raise ValueError("Source CSV checksum does not match the profile")
        after = source.stat()
        if (before.st_size, before.st_mtime_ns, before.st_ino) != (
            after.st_size,
            after.st_mtime_ns,
            after.st_ino,
        ):
            raise ValueError("Source CSV changed during request planning")
        if any(value != profile["counts"][key] for key, value in stats.items()):
            raise ValueError("Embedding input counts do not match the profile")
        if not writer.units:
            raise ValueError("No nonempty embedding inputs")
        plan = {
            "format": 1,
            "profile_sha256": hn.file_digest(profile_path),
            "source_sha256": profile["source"]["csv_sha256"],
            "embedding": profile["config"]["embedding"],
            "limits": asdict(limits),
            "counts": stats,
            "units": writer.units,
        }
        (work / "requests").rename(output / "requests")
    shutil.copyfile(profile_path, output / "profile.json")
    (output / "queries").mkdir()
    shutil.copyfile(
        profile_path.parent / "queries/holdout.parquet",
        output / "queries/holdout.parquet",
    )
    write_json(output / "plan.json", plan)
    return plan


def main():
    """Build requests from the exact CSV and manifest produced by preparation."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument(
        "--profile", type=Path, required=True, help="Profile manifest.json"
    )
    parser.add_argument(
        "--output", type=Path, required=True, help="Empty local directory"
    )
    args = parser.parse_args()
    plan = build_plan(args.source, args.profile, args.output)
    print(
        json.dumps({"batches": len(plan["units"]), "counts": plan["counts"]}, indent=2)
    )


if __name__ == "__main__":
    main()
