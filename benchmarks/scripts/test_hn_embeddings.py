"""Exercise HN batch planning, response validation, and interrupted-run recovery."""

# pylint: disable=import-error

import base64
import csv
import io
import json
import shutil
import struct
import subprocess
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

import pyarrow.parquet as pq

import embed_hn as embed
import plan_hn_embeddings as plan
import prepare_hn as hn


class FakeFiles:
    """Keep Batch input and output files in memory, without API calls."""

    def __init__(self):
        self.contents = {}
        self.with_streaming_response = self

    def create(self, *, file, purpose):
        """Capture the exact submitted JSONL payload."""
        assert purpose == "batch"
        file_id = f"file-{len(self.contents)}"
        self.contents[file_id] = file.read().decode()
        return SimpleNamespace(id=file_id)

    def content(self, file_id):
        """Provide the SDK's streaming response interface."""
        response = io.StringIO(self.contents[file_id])
        response.iter_lines = lambda: iter(response)
        return response


class FakeClient:
    """Model accepted, partially completed, and interrupted OpenAI batches."""

    def __init__(self):
        self.files = FakeFiles()
        self.batches = self
        self.jobs = {}
        self.fail_after_accept = False
        self.reject_before_accept = False
        self.create_calls = 0

    def with_options(self, *, max_retries):
        """Ensure paid create requests never use implicit SDK retries."""
        assert max_retries == 0
        return self

    def create(self, *, input_file_id, endpoint, completion_window, metadata):
        """Accept a job, optionally losing the response after acceptance."""
        assert endpoint == "/v1/embeddings"
        assert completion_window == "24h"
        self.create_calls += 1
        if self.reject_before_accept:
            raise ConnectionError("Connection lost before acceptance")
        batch = SimpleNamespace(
            id=f"batch-{len(self.jobs)}",
            input_file_id=input_file_id,
            status="in_progress",
            metadata=metadata,
            output_file_id=None,
            error_file_id=None,
        )
        self.jobs[batch.id] = batch
        if self.fail_after_accept:
            self.fail_after_accept = False
            raise ConnectionError("Accepted job response was lost")
        return batch

    def retrieve(self, batch_id):
        """Return the current state of an accepted job."""
        return self.jobs[batch_id]

    def list(self, *, limit):
        """Expose metadata-based recovery of an accepted job."""
        assert limit == 100
        return iter(self.jobs.values())

    def complete(self, batch_id, failures=None, status="completed"):
        """Return shuffled successes and transient/permanent errors."""
        batch = self.jobs[batch_id]
        batch.status = status
        requests = [
            json.loads(line)
            for line in self.files.contents[batch.input_file_id].splitlines()
        ]
        successes, errors = [], []
        for request in reversed(requests):
            custom_id = request["custom_id"]
            result = {"custom_id": custom_id, "error": None}
            if custom_id in (failures or {}):
                result["response"] = {
                    "status_code": failures[custom_id],
                    "body": {"error": {"code": "test_error"}},
                }
                errors.append(json.dumps(result))
            else:
                result["response"] = {
                    "status_code": 200,
                    "body": response_body(request),
                }
                successes.append(json.dumps(result))
        batch.output_file_id = f"{batch_id}-output"
        batch.error_file_id = f"{batch_id}-errors"
        self.files.contents[batch.output_file_id] = "\n".join(successes)
        self.files.contents[batch.error_file_id] = "\n".join(errors)


class FakeS3:
    """Model durable AWS CLI copies and failures after external side effects."""

    prefix = "s3://example/hn"

    def __init__(self, root):
        self.root = root
        self.fail_checkpoint = None

    def path(self, value):
        """Resolve a local path or fake S3 object key."""
        if value.startswith(self.prefix + "/"):
            return self.root / value.removeprefix(self.prefix + "/")
        return Path(value)

    def run(self, command, **_kwargs):
        """Copy objects, optionally losing the checkpoint after batch acceptance."""
        assert command[:3] == ["aws", "s3", "cp"]
        source, destination = map(self.path, command[3:5])
        if not source.exists():
            return SimpleNamespace(returncode=1, stderr="HeadObject (404) Not Found")
        if command[4] == self.prefix + "/state.json" and self.fail_checkpoint:
            state = json.loads(source.read_text())
            attempts = [
                attempt
                for unit in state["units"].values()
                for attempt in unit["attempts"]
            ]
            if any(
                (
                    "batch_id" in attempt
                    if self.fail_checkpoint == "accepted"
                    else attempt["status"] == "collected"
                )
                for attempt in attempts
            ):
                self.fail_checkpoint = None
                raise subprocess.CalledProcessError(1, command)
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, destination)
        return SimpleNamespace(returncode=0, stderr="")


def response_body(request):
    """Make deterministic test vectors, reversing indices to exercise matching."""
    dimensions = request["body"]["dimensions"]
    entries = []
    for index, tokens in enumerate(request["body"]["input"]):
        vector = [float(tokens[0] + 1)] + [0.0] * (dimensions - 1)
        entries.append(
            {
                "index": index,
                "embedding": base64.b64encode(
                    struct.pack(f"<{dimensions}f", *vector)
                ).decode(),
            }
        )
    return {"model": request["body"]["model"], "data": list(reversed(entries))}


class HNEmbeddingTests(unittest.TestCase):
    """Run the complete preparation-to-cache path with simulated API responses."""

    def setUp(self):
        # pylint: disable-next=consider-using-with
        self.root = Path(self.enterContext(tempfile.TemporaryDirectory()))
        self.source = self.root / "data.csv"
        with self.source.open("w", newline="") as stream:
            writer = csv.writer(stream)
            writer.writerow(["id", "text"])
            writer.writerows(
                [
                    (1, ""),
                    (2, "<p>duplicate</p>"),
                    (3, "duplicate"),
                    (4, "first unique"),
                    (5, "second unique"),
                    (6, "third unique"),
                    (7, "&nbsp;"),
                    (8, "fourth unique"),
                ]
            )
        config = self.root / "prepare.toml"
        config.write_text(
            hn.DEFAULT_CONFIG.read_text().replace("count = 1000", "count = 2")
        )
        self.profile = self.root / "profile/manifest.json"
        hn.prepare(self.source, self.profile.parent, config, workers=1)
        self.directory = self.root / "embeddings"
        limits = plan.Limits(request_inputs=1, batch_inputs=3)
        self.plan = plan.build_plan(self.source, self.profile, self.directory, limits)
        self.store = embed.Store(self.directory)
        self.client = FakeClient()

    def test_plan_is_deduplicated_bounded_and_reproducible(self):
        """Verify input hashes, exact counts, and deterministic request bytes."""
        hashes = []
        for unit in self.plan["units"]:
            requests = embed.load_requests(self.store, unit)
            self.assertLessEqual(unit["inputs"], self.plan["limits"]["batch_inputs"])
            self.assertLessEqual(unit["tokens"], self.plan["limits"]["batch_tokens"])
            self.assertLessEqual(unit["bytes"], self.plan["limits"]["batch_bytes"])
            for request in requests.values():
                inputs = request["body"]["input"]
                self.assertLessEqual(
                    sum(map(len, inputs)), self.plan["limits"]["request_tokens"]
                )
                hashes.extend(hn.input_digest(tokens) for tokens in inputs)
        self.assertEqual(len(hashes), 5)
        self.assertEqual(len(set(hashes)), 5)
        other = plan.build_plan(
            self.source,
            self.profile,
            self.root / "again",
            plan.Limits(**self.plan["limits"]),
        )
        self.assertEqual(self.plan, other)
        self.assertEqual(
            self.plan["counts"]["tokens_with_deduplication"],
            json.loads(self.profile.read_text())["counts"]["tokens_with_deduplication"],
        )

    def test_source_and_holdout_checksums_are_required(self):
        """Reject different source bytes even if the byte count is unchanged."""
        self.source.write_bytes(self.source.read_bytes().replace(b"first", b"other"))
        with self.assertRaisesRegex(ValueError, "Source CSV"):
            plan.build_plan(self.source, self.profile, self.root / "wrong")
        self.assertFalse((self.root / "wrong/plan.json").exists())
        (self.profile.parent / "queries/holdout.parquet").write_bytes(b"corrupt")
        with self.assertRaisesRegex(ValueError, "holdout checksum"):
            plan.build_plan(self.source, self.profile, self.root / "wrong-holdout")

    def test_request_token_and_batch_byte_limits(self):
        """Split long token arrays and reject requests that cannot fit a batch file."""
        limits = plan.Limits(request_inputs=4, request_tokens=8192, batch_inputs=4)
        writer = plan.RequestWriter(
            self.root / "packing", self.plan["embedding"], limits
        )
        plan.pack_requests([[1] * 5000, [2] * 5000, [3] * 3], writer)
        requests = list(
            embed.load_requests(
                embed.Store(self.root / "packing"), writer.units[0]
            ).values()
        )
        self.assertEqual([len(row["body"]["input"]) for row in requests], [1, 2])
        writer = plan.RequestWriter(
            self.root / "tiny", self.plan["embedding"], plan.Limits(batch_bytes=10)
        )
        with self.assertRaisesRegex(ValueError, "byte limit"):
            writer.add([[1]])

    def test_resume_collects_unordered_results_without_duplicate_calls(self):
        """Persist exactly one vector per unique input across fresh runner instances."""
        runner = embed.BatchRunner(self.store, self.client)
        self.assertFalse(runner.advance(max_active=1))
        self.assertEqual(self.client.create_calls, 1)
        for _ in self.plan["units"]:
            for batch_id in list(self.client.jobs):
                self.client.complete(batch_id)
            runner = embed.BatchRunner(self.store, self.client)
            complete = runner.advance(max_active=1)
        self.assertTrue(complete)
        before = self.client.create_calls
        self.assertTrue(embed.BatchRunner(self.store, self.client).advance())
        self.assertEqual(self.client.create_calls, before)
        manifest = json.loads((self.directory / "embeddings.json").read_text())
        rows = [
            row
            for part in manifest["parts"]
            for row in pq.read_table(self.directory / part["path"]).to_pylist()
        ]
        self.assertEqual(len(rows), 5)
        self.assertEqual(len({row["input_sha256"] for row in rows}), 5)
        for part in manifest["parts"]:
            self.assertEqual(
                part["sha256"], hn.file_digest(self.directory / part["path"])
            )

    def test_partial_expiration_only_retries_failed_requests(self):
        """Preserve successful vectors when an expired job needs another attempt."""
        runner = embed.BatchRunner(self.store, self.client)
        runner.advance(max_active=1)
        original = self.client.jobs["batch-0"]
        requests = [
            json.loads(line)
            for line in self.client.files.contents[original.input_file_id].splitlines()
        ]
        failed_id = requests[0]["custom_id"]
        self.client.complete("batch-0", {failed_id: 500}, status="expired")
        embed.BatchRunner(self.store, self.client).advance(max_active=1)
        retried = self.client.jobs["batch-1"]
        lines = self.client.files.contents[retried.input_file_id].splitlines()
        self.assertEqual([json.loads(line)["custom_id"] for line in lines], [failed_id])
        progress = json.loads((self.directory / "state.json").read_text())["units"][
            self.plan["units"][0]["sha256"]
        ]
        self.assertEqual(len(progress["completed"]), len(requests) - 1)
        self.assertEqual(
            sum(part["rows"] for part in progress["parts"]), len(requests) - 1
        )

    def test_lost_create_response_is_recovered_by_metadata(self):
        """Recover an accepted job without making a second paid create call."""
        self.client.fail_after_accept = True
        with self.assertRaises(ConnectionError):
            embed.BatchRunner(self.store, self.client).advance(max_active=1)
        embed.BatchRunner(self.store, self.client).advance(max_active=1)
        self.assertEqual(self.client.create_calls, 1)
        state = json.loads((self.directory / "state.json").read_text())
        self.assertEqual(
            state["units"][self.plan["units"][0]["sha256"]]["attempts"][0]["batch_id"],
            "batch-0",
        )

    def test_unknown_submission_is_not_automatically_repeated(self):
        """An ambiguous network failure requires reconciliation, not a second charge."""
        self.client.reject_before_accept = True
        with self.assertRaises(ConnectionError):
            embed.BatchRunner(self.store, self.client).advance(max_active=1)
        with self.assertRaisesRegex(RuntimeError, "Uncertain submission"):
            embed.BatchRunner(self.store, self.client).advance(max_active=1)
        self.assertEqual(self.client.create_calls, 1)

    def test_checkpoint_failure_prevents_submission(self):
        """Stop before contacting OpenAI when durable intent cannot be saved."""
        with (
            patch.object(self.store, "save", side_effect=OSError("S3 unavailable")),
            self.assertRaises(OSError),
        ):
            embed.BatchRunner(self.store, self.client).advance()
        self.assertEqual(self.client.create_calls, 0)

    def test_fresh_runner_recovers_after_s3_checkpoint_failure(self):
        """Keep accepted jobs and published vectors across loss of all runner files."""
        remote = FakeS3(self.root / "bucket")
        store = embed.Store(self.directory, remote.prefix)
        with patch("embed_hn.subprocess.run", side_effect=remote.run):
            embed.publish_plan(store)
            remote.fail_checkpoint = "accepted"
            with self.assertRaises(subprocess.CalledProcessError):
                embed.BatchRunner(store, self.client).advance(max_active=1)
            restored = embed.Store(self.root / "restored", remote.prefix)
            embed.BatchRunner(restored, self.client).advance(max_active=1)
            self.assertEqual(self.client.create_calls, 1)
            self.client.complete("batch-0")
            remote.fail_checkpoint = "collected"
            with self.assertRaises(subprocess.CalledProcessError):
                embed.BatchRunner(restored, self.client).advance(max_active=1)
            restored = embed.Store(self.root / "restored-again", remote.prefix)
            embed.BatchRunner(restored, self.client).advance(max_active=1)
            self.assertEqual(self.client.create_calls, 2)
            self.client.complete("batch-1")
            self.assertTrue(
                embed.BatchRunner(restored, self.client).advance(max_active=1)
            )
            manifest = json.loads((remote.root / "embeddings.json").read_text())
            self.assertEqual(sum(part["rows"] for part in manifest["parts"]), 5)
            for part in manifest["parts"]:
                self.assertEqual(
                    part["sha256"], hn.file_digest(remote.root / part["path"])
                )
                self.assertFalse((restored.root / part["path"]).exists())

    def test_publishing_preserves_existing_plan_and_local_inputs(self):
        """Do not overwrite a different immutable plan at the same S3 prefix."""
        remote = FakeS3(self.root / "bucket")
        store = embed.Store(self.directory, remote.prefix)
        with patch("embed_hn.subprocess.run", side_effect=remote.run):
            embed.publish_plan(store)
            embed.publish_plan(store)
            original = hn.file_digest(self.directory / "plan.json")
            plan.write_json(remote.root / "plan.json", {"different": True})
            with self.assertRaisesRegex(ValueError, "different plan"):
                embed.publish_plan(store)
            self.assertEqual(original, hn.file_digest(self.directory / "plan.json"))
            self.assertEqual(
                json.loads((remote.root / "plan.json").read_text()), {"different": True}
            )

    def test_queue_budget_and_retry_limit_bound_submissions(self):
        """Respect the token budget and stop retries without losing successful rows."""
        runner = embed.BatchRunner(self.store, self.client)
        runner.advance(max_active=4, max_pending_tokens=self.plan["units"][0]["tokens"])
        self.assertEqual(self.client.create_calls, 1)
        request = json.loads(
            self.client.files.contents[
                self.client.jobs["batch-0"].input_file_id
            ].splitlines()[0]
        )
        self.client.complete("batch-0", {request["custom_id"]: 500})
        with self.assertRaisesRegex(RuntimeError, "require attention"):
            embed.BatchRunner(self.store, self.client).advance(max_attempts=1)
        self.assertEqual(self.client.create_calls, 1)
        self.assertFalse((self.directory / "embeddings.json").exists())

    def test_missing_and_duplicate_results_cannot_publish_success(self):
        """Refuse incomplete responses without automatically resubmitting paid work."""
        embed.BatchRunner(self.store, self.client).advance(max_active=1)
        self.client.complete("batch-0")
        batch = self.client.jobs["batch-0"]
        line = self.client.files.contents[batch.output_file_id].splitlines()[0]
        for content in (line, line + "\n" + line):
            self.client.files.contents[batch.output_file_id] = content
            with self.assertRaises(ValueError):
                embed.BatchRunner(self.store, self.client).advance(max_active=1)
            self.assertFalse((self.directory / "embeddings.json").exists())
            self.assertEqual(self.client.create_calls, 1)

    def test_permanent_failure_does_not_complete_cache(self):
        """Surface permanent failures instead of publishing an incomplete cache."""
        embed.BatchRunner(self.store, self.client).advance(max_active=1)
        request = json.loads(
            self.client.files.contents[
                self.client.jobs["batch-0"].input_file_id
            ].splitlines()[0]
        )
        self.client.complete("batch-0", {request["custom_id"]: 400})
        with self.assertRaisesRegex(RuntimeError, "require attention"):
            embed.BatchRunner(self.store, self.client).advance(max_active=1)
        self.assertFalse((self.directory / "embeddings.json").exists())
        self.assertEqual(self.client.create_calls, 1)

    def test_corrupt_plan_and_mismatched_checkpoint_fail(self):
        """Reject changed inputs and state from another plan before submitting."""
        unit = self.plan["units"][0]
        self.store.path(unit["path"]).write_text("{}\n")
        with self.assertRaisesRegex(ValueError, "checksum mismatch"):
            embed.BatchRunner(self.store, self.client).advance()
        self.assertEqual(self.client.create_calls, 0)
        plan.write_json(
            self.directory / "state.json", {"plan_sha256": "wrong", "units": {}}
        )
        with self.assertRaisesRegex(ValueError, "different embedding plan"):
            embed.BatchRunner(self.store, self.client)

    def test_response_validation_rejects_wrong_vectors_and_indices(self):
        """Catch malformed responses before a cache shard can be published."""
        request = {
            "body": {
                "model": "text-embedding-3-small",
                "dimensions": 1536,
                "input": [[1], [2]],
            }
        }
        for mutation in ("model", "dimension", "indices", "nonfinite", "zero"):
            with self.subTest(mutation=mutation):
                body = response_body(request)
                if mutation == "model":
                    body["model"] = "wrong"
                elif mutation == "indices":
                    body["data"][0]["index"] = body["data"][1]["index"]
                else:
                    vector = (
                        [1.0]
                        if mutation == "dimension"
                        else [float("nan") if mutation == "nonfinite" else 0.0] * 1536
                    )
                    body["data"][0]["embedding"] = base64.b64encode(
                        struct.pack(f"<{len(vector)}f", *vector)
                    ).decode()
                with self.assertRaises(ValueError):
                    embed.decode_embeddings(body, request, self.plan["embedding"])

    def test_s3_access_errors_are_not_treated_as_empty_cache(self):
        """Fail closed on access denial while permitting a genuinely missing state file."""
        store = embed.Store(self.root / "remote", "s3://example/hn")
        for stderr, missing in (
            ("HeadObject (404) Not Found", True),
            ("(403) AccessDenied", False),
        ):
            with patch(
                "embed_hn.subprocess.run",
                return_value=SimpleNamespace(returncode=1, stderr=stderr),
            ):
                if missing:
                    self.assertIsNone(store.fetch("state.json", optional=True))
                else:
                    with self.assertRaises(RuntimeError):
                        store.fetch("state.json", optional=True)


if __name__ == "__main__":
    unittest.main()
