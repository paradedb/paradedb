"""Exercise real PostgreSQL masks, exhaustive search, and immutable GT publication."""

# pylint: disable=import-error

import copy
import json
import os
import shutil
import tempfile
import unittest
from datetime import UTC, datetime, timedelta
from pathlib import Path
from unittest.mock import patch

import numpy as np
import psycopg
import pyarrow as pa
import pyarrow.parquet as pq
from psycopg import sql

import ground_truth_hn as gt
import prepare_hn as hn
from embed_hn import Store
from plan_hn_embeddings import write_json
from test_hn_embeddings import FakeS3


@unittest.skipUnless(
    os.environ.get("HN_GT_DATABASE_URL") and os.environ.get("HN_GT_SUPERNOVA"),
    "Set HN_GT_DATABASE_URL and HN_GT_SUPERNOVA for GT integration tests",
)
class HNGroundTruthTests(unittest.TestCase):
    """Run the production path on a small, independent, synthetic corpus."""

    @classmethod
    def setUpClass(cls):
        # pylint: disable-next=consider-using-with
        cls.root = Path(cls.enterClassContext(tempfile.TemporaryDirectory()))
        cls.dataset = Store(cls.root / "dataset")
        generator = np.random.default_rng(42)
        vectors = generator.normal(size=(128, 1536)).astype(np.float32)
        vectors /= np.linalg.norm(vectors, axis=1, keepdims=True)
        vectors[1:16] = vectors[0]
        cls.vectors = vectors
        queries = generator.normal(size=(3, 1536)).astype(np.float32)
        queries[0] = vectors[0]
        cls.queries = queries
        authors = ["o'neil", "O'Neil", "prefix o'neil suffix", "作者"]
        cls.rows = [
            {
                "id": i + 1,
                "type": "story" if i % 2 else "comment",
                "by": authors[i % 4],
                "score": i,
                "time": datetime(2020, 1, 1, tzinfo=UTC) + timedelta(days=i),
                "emb": vector.tolist(),
            }
            for i, vector in enumerate(vectors)
        ]
        cls.rows.append(
            {
                "id": 1001,
                "type": None,
                "by": None,
                "score": None,
                "time": None,
                "emb": None,
            }
        )
        schema = pa.schema(
            [
                ("id", pa.int64()),
                ("type", pa.string()),
                ("by", pa.string()),
                ("score", pa.int32()),
                ("time", pa.timestamp("us", tz="UTC")),
                ("emb", pa.list_(pa.float32())),
            ]
        )
        corpus = cls.dataset.path(gt.CORPUS_PREFIX)
        corpus.mkdir(parents=True)
        for i in range(3):
            pq.write_table(
                pa.Table.from_pylist(cls.rows[i * 43 : (i + 1) * 43], schema=schema),
                corpus / f"part-{i}.parquet",
            )
        query_dir = cls.dataset.path("queries")
        query_dir.mkdir()
        pq.write_table(
            pa.table(
                {
                    "id": pa.array(range(3), pa.int32()),
                    "emb": pa.array(queries.tolist(), pa.list_(pa.float32())),
                }
            ),
            query_dir / "hn_queries.parquet",
        )
        pq.write_table(
            pa.table(
                {
                    "id": pa.array(range(3), pa.int32()),
                    "hn_id": pa.array([9991, 9992, 9993], pa.int64()),
                }
            ),
            query_dir / "holdout.parquet",
        )
        manifest = {
            "format": 1,
            "state": "assembled",
            "dataset": "hn",
            "version": "v1",
            "source": {"archive_verified": True},
            "embedding": {"dimensions": 1536, "distance": "cosine"},
            "counts": {"query_rows": 3},
            "corpora": {"full": {"rows": 129, "vector_rows": 128}},
            "artifacts": gt.inventory(
                cls.dataset.root, cls.dataset.root.rglob("*.parquet")
            ),
        }
        write_json(cls.dataset.path("manifest.json"), manifest)
        cls.filters = cls.root / "filters.toml"
        cls.filters.write_text(
            gt.DEFAULT_FILTERS.read_text()
            .replace("[0.1, 0.01, 0.001]", "[0.5, 0.2]")
            .replace("[0.001, 0.0001, 0.00001]", "[0.3, 0.2, 0.15, 0.1]")
        )
        cls.output = cls.root / "complete"
        cls.prepared = gt.prepare(
            cls.dataset,
            cls.output,
            os.environ["HN_GT_DATABASE_URL"],
            filters=cls.filters,
            memory_limit="256MB",
        )
        with patch.dict(os.environ, {"NOVA_BF_DEVICE": "cpu"}):
            cls.completed = gt.compute(cls.output, os.environ["HN_GT_SUPERNOVA"])

    def setUp(self):
        # pylint: disable-next=consider-using-with
        self.work = Path(self.enterContext(tempfile.TemporaryDirectory()))

    def test_exhaustive_results_match_independent_cosine_oracle(self):
        """Check complete top-10 membership, including duplicate-vector ties."""
        masks = pq.read_table(
            self.output / "ground_truth/membership.parquet"
        ).to_pydict()
        normalized_queries = self.queries.astype(np.float64)
        normalized_queries /= np.linalg.norm(normalized_queries, axis=1, keepdims=True)
        normalized_vectors = self.vectors.astype(np.float64)
        normalized_vectors /= np.linalg.norm(normalized_vectors, axis=1, keepdims=True)
        scores = normalized_queries @ normalized_vectors.T
        for item in self.prepared["filters"]:
            actual = pq.read_table(
                self.output / f"queries/ground_truth_{item['name']}_full.parquet"
            )
            self.assertTrue(actual.schema.equals(gt.GT_SCHEMA))
            eligible = np.flatnonzero(masks[item["name"]])
            self.assertEqual(len(eligible), item["matched_rows"])
            for row in actual.to_pylist():
                order = sorted(
                    eligible, key=lambda i, q=row["query_id"]: (-scores[q, i], i)
                )[:10]
                self.assertEqual(row["gt_ids"], [str(i + 1) for i in order])
        self.assertEqual(self.prepared["corpus_rows"], 128)
        self.assertTrue(
            any(
                value["boundary_ties"]
                for value in self.completed["boundary_audit"].values()
            )
        )

    def test_equality_masks_preserve_case_quotes_and_unicode(self):
        """Preserve case-sensitive, whole-value equality for authors."""
        masks = pq.read_table(
            self.output / "ground_truth/membership.parquet"
        ).to_pylist()
        authors = [
            item for item in self.prepared["filters"] if "author" in item["name"]
        ]
        self.assertEqual(len(authors), 4)
        for item in authors:
            selected = [
                self.rows[row["id"] - 1]["by"] for row in masks if row[item["name"]]
            ]
            self.assertEqual(len(set(selected)), 1)
            self.assertEqual(len(selected), 32)
            self.assertEqual(item["selectivity"], 0.25)
            self.assertIn(" = ", item["predicate"])
        with psycopg.connect(os.environ["HN_GT_DATABASE_URL"]) as db:
            self.assertIsNone(
                db.execute("SELECT to_regclass('hn_gt_items')").fetchone()[0]
            )

    def test_matches_pgvector_sequential_scan_when_available(self):
        """Cross-check filtered top-k against native PostgreSQL vector distance."""
        with psycopg.connect(os.environ["HN_GT_DATABASE_URL"]) as db:
            if not db.execute(
                "SELECT EXISTS (SELECT FROM pg_extension WHERE extname = 'vector')"
            ).fetchone()[0]:
                self.skipTest("pgvector is not installed in the scratch database")
            db.execute("""
                CREATE TEMP TABLE hn_gt_oracle (
                    id bigint, type text, "by" text, score integer,
                    time timestamptz, emb vector(1536)
                ) ON COMMIT DROP
            """)
            with db.cursor().copy("COPY hn_gt_oracle FROM STDIN") as copy_rows:
                for row in self.rows:
                    copy_rows.write_row(
                        (
                            row["id"],
                            row["type"],
                            row["by"],
                            row["score"],
                            row["time"],
                            json.dumps(row["emb"]) if row["emb"] else None,
                        )
                    )
            for item in self.prepared["filters"]:
                fixture = pq.read_table(
                    self.output / f"queries/ground_truth_{item['name']}_full.parquet"
                )
                for result in fixture.to_pylist():
                    expected = db.execute(
                        sql.SQL("""
                        SELECT id::text FROM hn_gt_oracle WHERE ({})
                        ORDER BY emb <=> %s::vector, hn_gt_oracle.id LIMIT 10
                    """).format(sql.SQL(item["predicate"])),
                        (json.dumps(self.queries[result["query_id"]].tolist()),),
                    ).fetchall()
                    self.assertEqual(result["gt_ids"], [row[0] for row in expected])

    def test_rejects_tampered_inputs_and_holdout_overlap(self):
        """Reject corrupted data and query rows leaking into the corpus."""
        dataset = Store(self.work / "dataset")
        shutil.copytree(self.dataset.root, dataset.root, dirs_exist_ok=True)
        path = dataset.path(gt.CORPUS_PREFIX + "part-0.parquet")
        with path.open("ab") as stream:
            stream.write(b"corrupt")
        with self.assertRaisesRegex(ValueError, "Checksum mismatch"):
            gt.prepare(
                dataset,
                self.work / "corrupt",
                os.environ["HN_GT_DATABASE_URL"],
                filters=self.filters,
            )
        shutil.copyfile(self.dataset.path(gt.CORPUS_PREFIX + "part-0.parquet"), path)
        holdout = dataset.path("queries/holdout.parquet")
        pq.write_table(
            pa.table({"id": pa.array(range(3), pa.int32()), "hn_id": [1, 9992, 9993]}),
            holdout,
        )
        manifest = json.loads(dataset.path("manifest.json").read_text())
        manifest["artifacts"]["queries/holdout.parquet"]["sha256"] = hn.file_digest(
            holdout
        )
        write_json(dataset.path("manifest.json"), manifest)
        with self.assertRaisesRegex(ValueError, "overlaps"):
            gt.prepare(
                dataset,
                self.work / "overlap",
                os.environ["HN_GT_DATABASE_URL"],
                filters=self.filters,
            )

    def test_rejects_partial_scans(self):
        """A partial corpus or query scan cannot become completed GT."""
        path = next((self.output / "ground_truth/supernova").glob("*compute.json"))
        runtime = json.loads(path.read_text())
        for key, value in (
            ("partial_slice", True),
            ("num_jobs", 2),
            ("corpus_files_this_worker", 1),
        ):
            changed = copy.deepcopy(runtime)
            changed["sharding"][key] = value
            with self.assertRaisesRegex(ValueError, "full corpus/query scan"):
                gt.validate_run(changed, self.prepared, 3)
        runtime["counts"]["queries_searched"] = 2
        with self.assertRaisesRegex(ValueError, "full corpus/query scan"):
            gt.validate_run(runtime, self.prepared, 3)

    def test_rejects_hits_outside_filter_and_missing_queries(self):
        """Reject unfiltered hits and incomplete result fixtures."""
        item = next(
            item
            for item in self.prepared["filters"]
            if item["name"] == "knn_top10_story"
        )
        path = (
            self.output
            / f"ground_truth/supernova/bf_hn_queries_{item['name']}_k11.parquet"
        )
        original = pq.read_table(path).to_pylist()
        rows = copy.deepcopy(original)
        rows[0]["hit_ids"][-1] = "9999999"
        changed = self.work / "changed.parquet"
        pq.write_table(pa.Table.from_pylist(rows), changed)
        with self.assertRaisesRegex(ValueError, "outside the PostgreSQL filter"):
            gt.convert_result(self.output, self.prepared, item, changed)
        pq.write_table(pa.Table.from_pylist(original[1:]), changed)
        with self.assertRaisesRegex(ValueError, "query IDs"):
            gt.convert_result(self.output, self.prepared, item, changed)

    def test_publication_is_immutable_and_bound_to_dataset(self):
        """Publish the marker last and reject conflicting GT or datasets."""
        remote = FakeS3(self.work / "s3")
        remote.prefix = hn.load_config(hn.DEFAULT_CONFIG)["artifact_prefix"]
        remote.root.mkdir()
        shutil.copyfile(
            self.dataset.path("manifest.json"), remote.root / "manifest.json"
        )
        store = Store(self.output, remote.prefix)
        uploads = []
        interrupt = [True]

        def upload(command, **kwargs):
            uploads.append(command[4])
            result = remote.run(command, **kwargs)
            if interrupt[0] and command[4].startswith(remote.prefix + "/queries/"):
                interrupt[0] = False
                raise ConnectionError("Interrupted after uploading a fixture")
            return result

        with patch("embed_hn.subprocess.run", side_effect=upload):
            with self.assertRaisesRegex(ConnectionError, "Interrupted"):
                gt.publish(store)
            self.assertFalse((remote.root / "ground_truth/manifest.json").exists())
            gt.publish(store)
            self.assertEqual(uploads[-1], remote.prefix + "/ground_truth/manifest.json")
            gt.publish(store)
            other = Store(self.work / "other", remote.prefix)
            shutil.copytree(self.output / "ground_truth", other.path("ground_truth"))
            shutil.copytree(self.output / "queries", other.path("queries"))
            manifest = json.loads(other.path("ground_truth/manifest.json").read_text())
            manifest["script_sha256"] = "different"
            write_json(other.path("ground_truth/manifest.json"), manifest)
            with self.assertRaisesRegex(ValueError, "different ground truth"):
                gt.publish(other)
            (remote.root / "manifest.json").write_text("{}")
            with self.assertRaisesRegex(ValueError, "Checksum mismatch"):
                gt.publish(store)


if __name__ == "__main__":
    unittest.main()
