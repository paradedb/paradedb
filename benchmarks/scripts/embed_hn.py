# /// script
# requires-python = ">=3.12,<3.13"
# dependencies = ["duckdb==1.4.3", "pyarrow==21.0.0", "tiktoken==0.12.0", "openai==3.31.0"]
# ///
"""Submit and collect resumable HN embedding batches, with optional S3 checkpoints."""

# pylint: disable=import-error

import argparse
import base64
import fcntl
import json
import math
import re
import struct
import subprocess
import tempfile
import time
import uuid
from contextlib import contextmanager
from pathlib import Path

import openai
import pyarrow as pa
import pyarrow.parquet as pq

import prepare_hn as hn
from plan_hn_embeddings import write_json

TERMINAL = {"completed", "failed", "expired", "cancelled"}
RETRYABLE_CODES = {
    "batch_expired",
    "rate_limit_exceeded",
    "server_error",
    "request_timeout",
}


class Store:
    """Keep local files and synchronously publish durable checkpoints to S3."""

    def __init__(self, root, remote=None):
        self.root = Path(root)
        self.root.mkdir(parents=True, exist_ok=True)
        self.remote = remote.rstrip("/") if remote else None
        if self.remote and not re.fullmatch(
            r"s3://[a-z0-9.-]+/[A-Za-z0-9_./-]+", self.remote
        ):
            raise ValueError("Expected an S3 bucket and nonempty prefix")

    def path(self, relative):
        """Resolve an artifact path without permitting paths outside the cache."""
        path = Path(relative)
        if path.is_absolute() or ".." in path.parts:
            raise ValueError(f"Invalid artifact path: {relative}")
        return self.root / path

    def fetch(self, relative, optional=False, refresh=False):
        """Download metadata or inputs; distinguish missing objects from access errors."""
        path = self.path(relative)
        if self.remote and (refresh or not path.exists()):
            path.parent.mkdir(parents=True, exist_ok=True)
            temporary = path.with_suffix(path.suffix + ".download")
            result = subprocess.run(
                [
                    "aws",
                    "s3",
                    "cp",
                    f"{self.remote}/{relative}",
                    str(temporary),
                    "--only-show-errors",
                ],
                check=False,
                capture_output=True,
                text=True,
            )
            if result.returncode:
                temporary.unlink(missing_ok=True)
                if optional and (
                    "(404)" in result.stderr or "NoSuchKey" in result.stderr
                ):
                    return None
                raise RuntimeError(
                    f"Could not restore {relative}: {result.stderr.strip()}"
                )
            temporary.replace(path)
        if optional and not path.exists():
            return None
        if not path.is_file():
            raise FileNotFoundError(path)
        return path

    def publish(self, relative):
        """Upload an artifact before a checkpoint is allowed to reference it."""
        if self.remote:
            subprocess.run(
                [
                    "aws",
                    "s3",
                    "cp",
                    str(self.path(relative)),
                    f"{self.remote}/{relative}",
                    "--only-show-errors",
                ],
                check=True,
            )

    def save(self, relative, value):
        """Commit a local JSON file, then its remote checkpoint."""
        write_json(self.path(relative), value)
        self.publish(relative)

    @contextmanager
    def lock(self):
        """Prevent two processes from advancing the same local cache."""
        with (self.root / ".lock").open("w") as lock:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
            yield


def publish_plan(store):
    """Publish request files first, preserving an existing immutable plan."""
    if not store.remote:
        raise ValueError("Publishing requires --s3-prefix")
    plan_path = store.path("plan.json")
    plan_hash = hn.file_digest(plan_path)
    plan = json.loads(plan_path.read_text())
    with tempfile.TemporaryDirectory(dir=store.root) as temporary:
        existing = Store(temporary, store.remote).fetch("plan.json", optional=True)
        if existing:
            if hn.file_digest(existing) != plan_hash:
                raise ValueError(
                    "S3 prefix already contains a different plan; use a new prefix"
                )
            return
    if hn.file_digest(store.path("profile.json")) != plan["profile_sha256"]:
        raise ValueError("Profile checksum mismatch")
    profile = json.loads(store.path("profile.json").read_text())
    if (
        hn.file_digest(store.path("queries/holdout.parquet"))
        != profile["artifacts"]["queries/holdout.parquet"]["sha256"]
    ):
        raise ValueError("Holdout checksum mismatch")
    for unit in plan["units"]:
        if hn.file_digest(store.path(unit["path"])) != unit["sha256"]:
            raise ValueError(f"Request checksum mismatch: {unit['path']}")
        store.publish(unit["path"])
    store.publish("profile.json")
    store.publish("queries/holdout.parquet")
    store.publish("plan.json")


def load_requests(store, unit):
    """Validate a request shard and recover the mapping used for result collection."""
    path = store.fetch(unit["path"])
    if hn.file_digest(path) != unit["sha256"]:
        raise ValueError(f"Request checksum mismatch: {unit['path']}")
    with path.open() as stream:
        rows = [json.loads(line) for line in stream]
    requests = {row["custom_id"]: row for row in rows}
    if len(requests) != len(rows) or len(requests) != unit["requests"]:
        raise ValueError("Duplicate or missing request IDs")
    return requests


def decode_embeddings(body, request, embedding):
    """Match response indices and validate finite, nonzero float32 vectors."""
    if body["model"] != embedding["model"]:
        raise ValueError("Embedding response model mismatch")
    inputs = request["body"]["input"]
    data = body["data"]
    indices = [entry["index"] for entry in data]
    if any(
        not isinstance(index, int) or isinstance(index, bool) for index in indices
    ) or sorted(indices) != list(range(len(inputs))):
        raise ValueError("Embedding response indices are incomplete or duplicated")
    records = []
    for entry in sorted(data, key=lambda item: item["index"]):
        raw = base64.b64decode(entry["embedding"], validate=True)
        if len(raw) != embedding["dimensions"] * 4:
            raise ValueError("Embedding response dimension mismatch")
        vector = struct.unpack(f"<{embedding['dimensions']}f", raw)
        if not all(math.isfinite(value) for value in vector) or not any(vector):
            raise ValueError("Embedding response contains a nonfinite or zero vector")
        records.append(
            {
                "input_sha256": hn.input_digest(inputs[entry["index"]]).hex(),
                "emb": vector,
            }
        )
    return records


def batch_results(client, batch):
    """Read successful and failed responses without buffering entire output files."""
    for file_id in (batch.output_file_id, batch.error_file_id):
        if file_id:
            with client.files.with_streaming_response.content(file_id) as response:
                for line in response.iter_lines():
                    if line:
                        yield json.loads(line)


def parse_result(result, request, embedding):
    """Distinguish validated vectors from retryable and permanent API failures."""
    reply = result.get("response") or {}
    if reply.get("status_code") == 200 and not result.get("error"):
        return decode_embeddings(reply["body"], request, embedding), None
    error = result.get("error") or reply.get("body", {}).get("error", {})
    status = reply.get("status_code", 0)
    return [], {
        "custom_id": result["custom_id"],
        "status_code": status,
        "code": error.get("code"),
        "retryable": status in {408, 409, 429}
        or status >= 500
        or error.get("code") in RETRYABLE_CODES,
    }


def collect_results(client, batch, requests, embedding, output):
    """Stream unordered output/error files, preserving successes for partial retries."""
    schema = pa.schema(
        [
            ("input_sha256", pa.string()),
            ("emb", pa.list_(pa.float32(), embedding["dimensions"])),
        ]
    )
    seen = set()
    stats = {"completed": [], "errors": [], "rows": 0}
    temporary = output.with_suffix(".parquet.tmp")
    output.parent.mkdir(parents=True, exist_ok=True)
    with pq.ParquetWriter(temporary, schema) as writer:
        for result in batch_results(client, batch):
            custom_id = result["custom_id"]
            if custom_id not in requests or custom_id in seen:
                raise ValueError("Unknown or duplicated batch result ID")
            seen.add(custom_id)
            records, error = parse_result(result, requests[custom_id], embedding)
            if error:
                stats["errors"].append(error)
            else:
                writer.write_table(pa.Table.from_pylist(records, schema=schema))
                stats["rows"] += len(records)
                stats["completed"].append(custom_id)
    if seen != requests.keys():
        raise ValueError(
            "Batch results are incomplete; refusing to resubmit unknown outcomes"
        )
    temporary.replace(output)
    stats["completed"].sort()
    return stats


class BatchRunner:
    """Advance a durable plan without resubmitting completed or uncertain requests."""

    def __init__(self, store, client):
        self.store = store
        self.client = client
        plan_path = store.fetch("plan.json", refresh=True)
        self.plan_hash = hn.file_digest(plan_path)
        self.plan = json.loads(plan_path.read_text())
        if (
            self.plan["format"] != 1
            or self.plan["embedding"] != hn.load_config(hn.DEFAULT_CONFIG)["embedding"]
        ):
            raise ValueError("Unsupported embedding plan")
        path = store.fetch("state.json", optional=True, refresh=True)
        self.state = (
            json.loads(path.read_text())
            if path
            else {
                "plan_sha256": self.plan_hash,
                "units": {},
            }
        )
        if self.state["plan_sha256"] != self.plan_hash:
            raise ValueError("Checkpoint belongs to a different embedding plan")

    def checkpoint(self):
        """Persist progress before performing the next externally visible operation."""
        self.store.save("state.json", self.state)

    def recover_submission(self, attempt):
        """Find an accepted batch after an interrupted create call, without creating another."""
        matches = [
            batch
            for batch in self.client.batches.list(limit=100)
            if (batch.metadata or {}).get("hn_attempt") == attempt["token"]
            and (batch.metadata or {}).get("hn_plan") == self.plan_hash
        ]
        if len(matches) != 1:
            raise RuntimeError(
                f"Uncertain submission {attempt['token']}: found {len(matches)} matching batches. "
                "Rerun to reconcile; do not submit another copy."
            )
        attempt["batch_id"] = matches[0].id
        attempt["status"] = matches[0].status
        self.checkpoint()

    def submit(self, unit, progress):
        """Upload only remaining requests and durably record intent before batch creation."""
        requests = load_requests(self.store, unit)
        remaining = [
            row for key, row in requests.items() if key not in progress["completed"]
        ]
        attempt = {"token": uuid.uuid4().hex, "status": "uploading"}
        relative = f"submissions/{unit['sha256']}-{len(progress['attempts']):02}.jsonl"
        path = self.store.path(relative)
        path.parent.mkdir(parents=True, exist_ok=True)
        with path.open("w") as stream:
            for row in remaining:
                stream.write(json.dumps(row, separators=(",", ":")) + "\n")
        self.store.publish(relative)
        attempt["path"] = relative
        progress["attempts"].append(attempt)
        self.checkpoint()
        self.start_batch(unit, attempt)

    def start_batch(self, unit, attempt):
        """Resume uploads safely; disable automatic retries of paid batch creation."""
        if "file_id" not in attempt:
            with self.store.fetch(attempt["path"]).open("rb") as stream:
                attempt["file_id"] = self.client.files.create(
                    file=stream, purpose="batch"
                ).id
        attempt["status"] = "submitting"
        self.checkpoint()
        try:
            batch = self.client.with_options(max_retries=0).batches.create(
                input_file_id=attempt["file_id"],
                endpoint="/v1/embeddings",
                completion_window="24h",
                metadata={
                    "hn_plan": self.plan_hash,
                    "hn_unit": unit["sha256"],
                    "hn_attempt": attempt["token"],
                },
            )
        except openai.APIStatusError as error:
            if error.status_code < 500 and error.status_code != 408:
                attempt["status"] = "uploading"
                self.checkpoint()
            raise
        attempt["batch_id"] = batch.id
        attempt["status"] = batch.status
        self.checkpoint()

    def collect(self, unit, progress, attempt):
        """Publish validated successes before recording them as complete."""
        batch = self.client.batches.retrieve(attempt["batch_id"])
        attempt["status"] = batch.status
        if batch.status not in TERMINAL:
            return
        if batch.status == "failed":
            progress["blocked"] = (
                "Batch validation failed; inspect the batch in OpenAI before changing the plan"
            )
            self.checkpoint()
            return
        requests = load_requests(self.store, unit)
        requests = {
            key: row
            for key, row in requests.items()
            if key not in progress["completed"]
        }
        relative = (
            f"vectors/{unit['sha256']}-{len(progress['attempts']) - 1:02}.parquet"
        )
        output = self.store.path(relative)
        result = collect_results(
            self.client, batch, requests, self.plan["embedding"], output
        )
        if result["rows"]:
            self.store.publish(relative)
            progress["parts"].append(
                {
                    "path": relative,
                    "sha256": hn.file_digest(output),
                    "rows": result["rows"],
                }
            )
        progress["completed"].extend(result["completed"])
        attempt["errors"] = result["errors"]
        attempt["status"] = "collected"
        if any(not error["retryable"] for error in result["errors"]):
            progress["blocked"] = "Permanent request failures; inspect state.json"
        self.checkpoint()
        if self.store.remote:
            output.unlink(missing_ok=True)

    def refresh(self):
        """Reconcile uploads, uncertain submissions, and completed remote batches."""
        for unit in self.plan["units"]:
            progress = self.state["units"].get(unit["sha256"])
            if not progress or not progress["attempts"] or progress.get("blocked"):
                continue
            attempt = progress["attempts"][-1]
            if attempt["status"] == "collected":
                continue
            if attempt["status"] == "uploading":
                self.start_batch(unit, attempt)
            elif "batch_id" not in attempt:
                self.recover_submission(attempt)
            self.collect(unit, progress, attempt)
        self.checkpoint()

    def advance(self, max_active=4, max_pending_tokens=20000000, max_attempts=3):
        """Collect results, then fill the bounded batch queue with unfinished work."""
        if min(max_active, max_pending_tokens, max_attempts) < 1:
            raise ValueError("Batch queue limits must be positive")
        self.refresh()
        self.raise_for_failures()
        active, tokens = 0, 0
        for unit in self.plan["units"]:
            progress = self.state["units"].get(unit["sha256"], {})
            attempts = progress.get("attempts", [])
            if (
                attempts
                and attempts[-1]["status"] != "collected"
                and not progress.get("blocked")
            ):
                active += 1
                tokens += unit["tokens"]
        for unit in self.plan["units"]:
            progress = self.state["units"].setdefault(
                unit["sha256"], {"completed": [], "parts": [], "attempts": []}
            )
            if len(progress["completed"]) == unit["requests"] or progress.get(
                "blocked"
            ):
                continue
            if (
                progress["attempts"]
                and progress["attempts"][-1]["status"] != "collected"
            ):
                continue
            if len(progress["attempts"]) >= max_attempts:
                progress["blocked"] = "Retry limit reached"
                self.checkpoint()
                self.raise_for_failures()
            if unit["tokens"] > max_pending_tokens:
                raise ValueError("Queue token limit is smaller than a planned batch")
            if active >= max_active or tokens + unit["tokens"] > max_pending_tokens:
                continue
            self.submit(unit, progress)
            active += 1
            tokens += unit["tokens"]
        self.checkpoint()
        return self.finish()

    def raise_for_failures(self):
        """Stop new paid submissions when existing work requires intervention."""
        blocked = [
            key
            for key, progress in self.state["units"].items()
            if progress.get("blocked")
        ]
        if blocked:
            raise RuntimeError(
                f"Embedding batches require attention: {', '.join(blocked)}; inspect state.json"
            )

    def finish(self):
        """Publish a cache manifest only when every unique input has a vector."""
        self.raise_for_failures()
        if any(
            len(self.state["units"][unit["sha256"]]["completed"]) != unit["requests"]
            for unit in self.plan["units"]
        ):
            return False
        parts = [
            part
            for unit in self.plan["units"]
            for part in self.state["units"][unit["sha256"]]["parts"]
        ]
        if sum(part["rows"] for part in parts) != self.plan["counts"]["unique_inputs"]:
            raise ValueError("Completed cache row count does not match the plan")
        self.store.save(
            "embeddings.json",
            {
                "state": "embedded",
                "plan_sha256": self.plan_hash,
                "profile_sha256": self.plan["profile_sha256"],
                "embedding": self.plan["embedding"],
                "parts": parts,
                "rows": self.plan["counts"]["unique_inputs"],
            },
        )
        return True


def main():
    """Publish a request plan or advance its asynchronous embedding jobs."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("publish", "advance"))
    parser.add_argument("--directory", type=Path, required=True)
    parser.add_argument("--s3-prefix", help="Single-writer S3 checkpoint prefix")
    parser.add_argument("--max-active", type=int, default=4)
    parser.add_argument("--max-pending-tokens", type=int, default=20000000)
    parser.add_argument("--max-attempts", type=int, default=3)
    parser.add_argument(
        "--max-seconds",
        type=int,
        default=0,
        help="Zero performs one submit/collect pass",
    )
    parser.add_argument("--poll-seconds", type=int, default=60)
    args = parser.parse_args()
    if args.max_seconds < 0 or args.poll_seconds < 1:
        parser.error("Invalid polling limits")
    store = Store(args.directory, args.s3_prefix)
    with store.lock():
        if args.action == "publish":
            publish_plan(store)
            return
        with openai.OpenAI(max_retries=5, timeout=180) as client:
            runner = BatchRunner(store, client)
            deadline = time.monotonic() + args.max_seconds
            while True:
                try:
                    complete = runner.advance(
                        args.max_active, args.max_pending_tokens, args.max_attempts
                    )
                except openai.RateLimitError:
                    print(
                        "Batch queue rate limited; retrying from the saved checkpoint",
                        flush=True,
                    )
                    if time.monotonic() >= deadline:
                        raise
                    complete = False
                if complete:
                    print("Embedding cache complete; see embeddings.json", flush=True)
                    break
                print("Embedding jobs are pending; checkpoints saved", flush=True)
                remaining = deadline - time.monotonic()
                if remaining <= 0:
                    break
                time.sleep(min(args.poll_seconds, remaining))


if __name__ == "__main__":
    main()
