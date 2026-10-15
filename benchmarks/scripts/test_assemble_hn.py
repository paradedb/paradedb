"""Check corpus identity, holdout separation, and immutable dataset publication."""

# pylint: disable=import-error

import csv
import json
import shutil
import tempfile
import unittest
from datetime import UTC
from pathlib import Path
from unittest.mock import patch

import duckdb
import pyarrow as pa
import pyarrow.parquet as pq

import assemble_hn as assembly
import embed_hn as embed
import plan_hn_embeddings as plan
import prepare_hn as hn
from test_hn_embeddings import FakeClient, FakeS3


class HNAssemblyTests(unittest.TestCase):
    """Exercise the real assembly pipeline using a completed simulated Batch cache."""

    def setUp(self):
        # pylint: disable-next=consider-using-with
        self.root = Path(self.enterContext(tempfile.TemporaryDirectory()))
        self.source = self.root / "data.csv"
        self.rows = []
        for hn_id, text in enumerate(
            [
                "",
                "<p>duplicate</p>",
                "duplicate",
                "first unique",
                "second unique",
                "third unique",
                "&nbsp;",
                "fourth unique",
                "\\N",
                'Unicode 🦆\n<p>quote " &amp; comma,</p>',
            ],
            1,
        ):
            self.rows.append(
                dict(
                    zip(
                        assembly.COLUMNS,
                        [
                            hn_id,
                            "0",
                            "comment",
                            "user",
                            "2008-10-26 05:06:58",
                            text,
                            "1",
                            "0",
                            "0",
                            "[1,2]",
                            "",
                            -1,
                            "original title",
                            "[]",
                            "0",
                        ],
                        strict=True,
                    )
                )
            )
        with self.source.open("w", newline="") as stream:
            writer = csv.DictWriter(stream, fieldnames=assembly.COLUMNS)
            writer.writeheader()
            writer.writerows(self.rows)
        config = self.root / "prepare.toml"
        config.write_text(
            hn.DEFAULT_CONFIG.read_text().replace("count = 1000", "count = 2")
        )
        profile = self.root / "profile/manifest.json"
        hn.prepare(self.source, profile.parent, config, workers=1)
        self.cache = embed.Store(self.root / "cache")
        plan.build_plan(
            self.source,
            profile,
            self.cache.root,
            plan.Limits(request_inputs=1, batch_inputs=3),
        )
        client = FakeClient()
        runner = embed.BatchRunner(self.cache, client)
        runner.advance()
        for batch_id in client.jobs:
            client.complete(batch_id)
        self.assertTrue(runner.advance())
        self.output = self.root / "output"

    def build(self, output=None, **kwargs):
        """Run a small assembly with several nested corpus sizes."""
        return assembly.assemble(
            self.source,
            self.cache,
            output or self.output,
            labels=("full", "3", "5"),
            workers=1,
            memory_limit="256MB",
            **kwargs,
        )

    def corpus(self, label="full", output=None):
        """Read emitted Parquet through the same engine used by pdbench."""
        with duckdb.connect() as conn:
            return (
                conn.read_parquet(
                    str(
                        (output or self.output)
                        / f"sampled/{label}/parquet/hn_items/*.parquet"
                    )
                )
                .order("id")
                .fetch_arrow_table()
            )

    def rewrite_part(self, transform):
        """Change shard contents and checksum to exercise semantic validation."""
        path = self.cache.path("embeddings.json")
        manifest = json.loads(path.read_text())
        part = manifest["parts"][0]
        parquet = self.cache.path(part["path"])
        table = transform(pq.read_table(parquet))
        pq.write_table(table, parquet)
        part["sha256"] = hn.file_digest(parquet)
        plan.write_json(path, manifest)

    def test_preserves_columns_vectors_and_disjoint_nested_samples(self):
        """Duplicates reuse vectors, null inputs remain null, and queries never leak."""
        manifest = self.build()
        holdout = pq.read_table(self.cache.path("queries/holdout.parquet")).to_pylist()
        excluded = {row["hn_id"] for row in holdout}
        corpus = self.corpus()
        self.assertEqual(corpus.column_names, [*assembly.COLUMNS, "emb"])
        records = {row["id"]: row for row in corpus.to_pylist()}
        self.assertEqual(set(records), set(range(1, 11)) - excluded)
        for original in self.rows:
            if original["id"] in excluded:
                continue
            row = records[original["id"]]
            self.assertEqual(
                row["text"], None if original["text"] == "\\N" else original["text"]
            )
            self.assertEqual(row["title"], original["title"])
            self.assertEqual(row["url"], "")
            self.assertEqual(row["kids"], [1, 2])
            self.assertEqual(row["parts"], [])
            self.assertEqual(row["deleted"], False)
            self.assertEqual(row["dead"], True)
            self.assertEqual(row["score"], -1)
            self.assertEqual(
                str(row["time"].astimezone(UTC)), "2008-10-26 05:06:58+00:00"
            )
            tokens = hn.tokenizer().encode_ordinary(hn.normalize_text(original["text"]))
            if tokens:
                self.assertEqual(len(row["emb"]), 1536)
                self.assertEqual(row["emb"][0], float(tokens[0] + 1))
            else:
                self.assertIsNone(row["emb"])
        self.assertEqual(records[2]["emb"], records[3]["emb"])
        self.assertEqual(manifest["corpora"]["full"], {"rows": 8, "vector_rows": 5})
        smaller = set(self.corpus("3").column("id").to_pylist())
        larger = set(self.corpus("5").column("id").to_pylist())
        self.assertTrue(smaller < larger < set(records))
        query_table = pq.read_table(self.output / "queries/hn_queries.parquet")
        self.assertEqual(query_table.column_names, ["id", "emb"])
        self.assertEqual(query_table.schema.field("id").type, pa.int32())
        self.assertEqual(query_table.column("id").to_pylist(), [0, 1])
        for row, selected in zip(query_table.to_pylist(), holdout, strict=True):
            self.assertEqual(row["emb"][0], selected["token_ids"][0] + 1)
        for relative, entry in manifest["artifacts"].items():
            self.assertEqual(hn.file_digest(self.output / relative), entry["sha256"])
        with self.assertRaisesRegex(ValueError, "must be empty"):
            self.build()

    def test_reproducible_across_cache_shard_order(self):
        """Cache response ordering cannot change corpus membership or artifact bytes."""
        first = self.build()
        path = self.cache.path("embeddings.json")
        embedded = json.loads(path.read_text())
        embedded["parts"].reverse()
        plan.write_json(path, embedded)
        second = self.build(self.root / "second")
        self.assertEqual(first["corpora"], second["corpora"])
        for relative, entry in first["artifacts"].items():
            if relative.endswith(".parquet"):
                self.assertEqual(entry, second["artifacts"][relative])

    def test_source_and_checksum_mismatches_never_complete(self):
        """Wrong source bytes and untrusted cache bytes cannot produce a manifest."""
        data = self.source.read_bytes()
        self.source.write_bytes(data.replace(b"original title", b"modified title"))
        with self.assertRaisesRegex(ValueError, "checksum"):
            self.build()
        self.assertFalse((self.output / "manifest.json").exists())
        self.source.write_bytes(data)
        self.cache.path("profile.json").write_text("{}")
        with self.assertRaisesRegex(ValueError, "Checksum"):
            self.build()

    def test_invalid_vectors_are_rejected_even_with_matching_checksums(self):
        """Validate dimensions, values, and unique keys beyond file integrity."""
        backup = self.root / "backup"
        shutil.copytree(self.cache.root, backup)
        for kind in (
            "zero",
            "nan",
            "null_element",
            "dimensions",
            "duplicate",
            "missing",
        ):
            with self.subTest(kind=kind):
                shutil.copytree(backup, self.cache.root, dirs_exist_ok=True)

                def change(table, kind=kind):
                    rows = table.to_pylist()
                    schema = table.schema
                    if kind == "duplicate":
                        rows[1]["input_sha256"] = rows[0]["input_sha256"]
                    elif kind == "missing":
                        rows[0]["input_sha256"] = "0" * 64
                    elif kind == "dimensions":
                        for row in rows:
                            row["emb"] = row["emb"][:2]
                        schema = pa.schema(
                            [
                                ("input_sha256", pa.string()),
                                ("emb", pa.list_(pa.float32(), 2)),
                            ]
                        )
                    elif kind == "zero":
                        rows[0]["emb"] = [0.0] * 1536
                    else:
                        rows[0]["emb"][0] = float("nan") if kind == "nan" else None
                    return pa.Table.from_pylist(rows, schema=schema)

                self.rewrite_part(change)
                with self.assertRaises(ValueError):
                    self.build(self.root / kind)
                self.assertFalse((self.root / kind / "manifest.json").exists())

    def test_remote_cache_restores_and_releases_downloaded_vectors(self):
        """A fresh runner downloads and verifies cache files without any API client."""
        remote = FakeS3(self.root / "remote")
        shutil.copytree(self.cache.root, remote.root)
        self.cache = embed.Store(self.root / "fresh", remote.prefix)
        with patch("embed_hn.subprocess.run", side_effect=remote.run):
            result = self.build()
        self.assertEqual(result["corpora"]["full"]["rows"], 8)
        self.assertEqual(list((self.cache.root / "vectors").glob("*.parquet")), [])

    def test_publication_rejects_unverified_corrupt_and_conflicting_artifacts(self):
        """Publish the manifest last and make interruption/retry safe for one writer."""
        manifest = self.build()
        remote = FakeS3(self.root / "published")
        remote.prefix = hn.load_config(hn.DEFAULT_CONFIG)["artifact_prefix"]
        store = embed.Store(self.output, remote.prefix)
        with patch("embed_hn.subprocess.run", side_effect=remote.run):
            with self.assertRaisesRegex(ValueError, "archive-verified"):
                assembly.publish_dataset(store)
            self.assertFalse(remote.root.exists())
            manifest["source"]["archive_verified"] = True
            plan.write_json(self.output / "manifest.json", manifest)
            original = (self.output / "queries/hn_queries.parquet").read_bytes()
            (self.output / "queries/hn_queries.parquet").write_bytes(b"broken")
            with self.assertRaisesRegex(ValueError, "checksum"):
                assembly.publish_dataset(store)
            self.assertFalse(remote.root.exists())
            (self.output / "queries/hn_queries.parquet").write_bytes(original)
            publish = store.publish

            def interrupted(relative):
                if relative == "manifest.json":
                    raise ConnectionError("Interrupted publication")
                publish(relative)

            with (
                patch.object(store, "publish", side_effect=interrupted),
                self.assertRaises(ConnectionError),
            ):
                assembly.publish_dataset(store)
            self.assertFalse((remote.root / "manifest.json").exists())
            self.assertTrue((remote.root / "assembly-intent.json").exists())
            assembly.publish_dataset(store)
            assembly.publish_dataset(store)
            self.assertEqual(
                hn.file_digest(remote.root / "manifest.json"),
                hn.file_digest(self.output / "manifest.json"),
            )
            manifest["assembly"]["sampling_seed"] = 43
            plan.write_json(self.output / "manifest.json", manifest)
            with self.assertRaisesRegex(ValueError, "different manifest"):
                assembly.publish_dataset(store)
            (remote.root / "manifest.json").unlink()
            with self.assertRaisesRegex(ValueError, "already owns"):
                assembly.publish_dataset(store)

    def test_sample_sizes_are_exact_and_bounded(self):
        """Reject ambiguous names and corpora smaller than the requested sample."""
        self.assertEqual(
            assembly.sample_sizes(["1k", "1m", "full"], 2000000),
            {"1k": 1000, "1m": 1000000, "full": 2000000},
        )
        for labels in ([], ["0"], ["../full"], ["01"], ["1m"], ["full", "full"]):
            with self.subTest(labels=labels), self.assertRaises(ValueError):
                assembly.sample_sizes(labels, 10)


if __name__ == "__main__":
    unittest.main()
