"""Checks for HN artifact integrity and the shared-loader input layout."""

import hashlib
import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path

import duckdb  # pylint: disable=import-error
from prepare_heap import CORPUS, FIXTURES, fetch, prepare


class PrepareHnHeapTests(unittest.TestCase):
    """Exercise real Parquet files and optional PostgreSQL heap validation."""

    def setUp(self):
        # pylint: disable=consider-using-with
        self.root = Path(self.enterContext(tempfile.TemporaryDirectory()))
        self.source = self.root / "source"
        self.output = self.root / "output"
        shard = self.source / CORPUS / "part-0.parquet"
        shard.parent.mkdir(parents=True)
        (self.source / "queries").mkdir()
        with duckdb.connect() as db:
            db.execute("""
                CREATE TABLE corpus AS SELECT
                    id::BIGINT AS id, false AS deleted, 'comment' AS type,
                    'author' AS "by", '2026-01-01'::TIMESTAMPTZ AS time,
                    'hello' || chr(0) || 'world' AS text, false AS dead,
                    NULL::BIGINT AS parent, NULL::BIGINT AS poll,
                    [3, 4]::BIGINT[] AS kids, 'https://example.com' AS url,
                    42::INTEGER AS score, NULL::VARCHAR AS title,
                    NULL::BIGINT[] AS parts, 0::INTEGER AS descendants,
                    CASE WHEN id = 1 THEN list_resize([1.0::FLOAT], 1536, 0.0::FLOAT)
                         ELSE NULL END AS emb
                FROM range(1, 3) AS rows(id)
            """)
            db.execute("COPY corpus TO ? (FORMAT PARQUET)", [str(shard)])
            db.execute(
                "COPY (SELECT 0::INTEGER AS id, 3::BIGINT AS hn_id) TO ? (FORMAT PARQUET)",
                [str(self.source / FIXTURES[0])],
            )
            db.execute(
                "COPY (SELECT 0::INTEGER AS id, emb FROM corpus WHERE id = 1) "
                "TO ? (FORMAT PARQUET)",
                [str(self.source / FIXTURES[1])],
            )
        artifacts = {}
        for path in self.source.rglob("*.parquet"):
            artifacts[path.relative_to(self.source).as_posix()] = {
                "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
                "bytes": path.stat().st_size,
            }
        self.manifest = {
            "format": 1,
            "state": "assembled",
            "dataset": "hn",
            "version": "v1",
            "source": {"archive_verified": True},
            "embedding": {"dimensions": 1536, "distance": "cosine"},
            "corpora": {"full": {"rows": 2, "vector_rows": 1}},
            "counts": {"query_rows": 1},
            "artifacts": artifacts,
        }
        self.write_manifest()

    def write_manifest(self):
        """Write the fixture manifest after deliberate test mutations."""
        (self.source / "manifest.json").write_text(json.dumps(self.manifest))

    def test_stages_only_verified_shards_under_the_postgres_table_name(self):
        """Ignore unmanifested files and preserve the source artifact bytes."""
        (self.source / CORPUS / "unexpected.parquet").write_bytes(b"extra")
        prepare(str(self.source), self.output, workers=2)
        staged = self.output / "load-source/sampled/full/parquet/hn_items"
        self.assertEqual([p.name for p in staged.iterdir()], ["part-0.parquet"])
        with duckdb.connect() as db:
            self.assertEqual(
                db.execute(
                    "SELECT count(*), count(emb) FROM read_parquet(?)",
                    [str(staged / "*.parquet")],
                ).fetchone(),
                (2, 1),
            )
        digest = hashlib.sha256(
            (self.source / "manifest.json").read_bytes()
        ).hexdigest()
        sql = (self.output / "verify_heap.sql").read_text()
        self.assertIn(digest, sql)
        self.assertIn("ARRAY[3]::bigint[]", sql)
        with self.assertRaisesRegex(ValueError, "empty output"):
            prepare(str(self.source), self.output)

    def test_rejects_corrupted_corpus(self):
        """Never load an object whose bytes differ from the published manifest."""
        with (self.source / CORPUS / "part-0.parquet").open("ab") as stream:
            stream.write(b"changed")
        with self.assertRaisesRegex(ValueError, "checksum/size"):
            prepare(str(self.source), self.output)

    def test_rejects_incomplete_corpus_count(self):
        """Require the complete manifest corpus before invoking the heap loader."""
        self.manifest["corpora"]["full"]["rows"] = 3
        self.write_manifest()
        with self.assertRaisesRegex(ValueError, "row count"):
            prepare(str(self.source), self.output)

    def test_rejects_wrong_query_count(self):
        """Bind the holdout and query fixture to the same complete query set."""
        self.manifest["counts"]["query_rows"] = 2
        self.write_manifest()
        with self.assertRaisesRegex(ValueError, "identities"):
            prepare(str(self.source), self.output)

    def test_rejects_unverified_source_and_path_traversal(self):
        """Refuse unverified provenance and paths outside the staging directory."""
        self.manifest["source"]["archive_verified"] = False
        self.write_manifest()
        with self.assertRaisesRegex(ValueError, "archive-verified"):
            prepare(str(self.source), self.output)
        with self.assertRaisesRegex(ValueError, "Invalid artifact path"):
            fetch(str(self.source), "../escape", self.root)

    @unittest.skipUnless(
        os.environ.get("HN_TEST_DATABASE_URL"), "requires a scratch PostgreSQL database"
    )
    def test_postgres_load_and_holdout_validation(self):
        """Load all HN types, strip NULs, and reject vector-count/holdout errors."""
        prepare(str(self.source), self.output)
        url = os.environ["HN_TEST_DATABASE_URL"]
        schema = (
            Path(__file__).resolve().parents[2]
            / "benchmarks/datasets/hn/create_tables.sql"
        )
        subprocess.run(
            ["psql", url, "-X", "-v", "ON_ERROR_STOP=1", "-f", str(schema)], check=True
        )
        self.addCleanup(
            subprocess.run,
            [
                "psql",
                url,
                "-X",
                "-c",
                "DROP TABLE IF EXISTS hn_items, hn_dataset_identity;",
            ],
            check=True,
        )
        with duckdb.connect() as db:
            db.execute(
                "INSTALL postgres; LOAD postgres; SET pg_null_byte_replacement = '';"
            )
            db.execute("ATTACH '" + url.replace("'", "''") + "' AS pg (TYPE postgres)")
            db.execute(
                "INSERT INTO pg.public.hn_items SELECT * FROM read_parquet(?)",
                [
                    str(
                        self.output
                        / "load-source/sampled/full/parquet/hn_items/*.parquet"
                    )
                ],
            )
        verification = self.output / "verify_heap.sql"
        subprocess.run(
            ["psql", url, "-X", "-v", "ON_ERROR_STOP=1", "-f", str(verification)],
            check=True,
        )
        result = subprocess.check_output(
            [
                "psql",
                url,
                "-XAt",
                "-c",
                "SELECT text, kids, vector_dims(emb) FROM hn_items WHERE id=1;",
            ],
            text=True,
        )
        self.assertEqual(result.strip(), "helloworld|{3,4}|1536")
        subprocess.run(
            ["psql", url, "-X", "-c", "UPDATE hn_items SET emb = NULL WHERE id = 1;"],
            check=True,
        )
        result = subprocess.run(
            ["psql", url, "-X", "-v", "ON_ERROR_STOP=1", "-f", str(verification)],
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("HN heap counts differ", result.stderr)
        subprocess.run(
            ["psql", url, "-X", "-c", "UPDATE hn_items SET id = 3 WHERE id = 2;"],
            check=True,
        )
        verification.write_text(
            verification.read_text().replace(
                "actual_vectors <> 1", "actual_vectors <> 0"
            )
        )
        result = subprocess.run(
            ["psql", url, "-X", "-v", "ON_ERROR_STOP=1", "-f", str(verification)],
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Held-out HN queries overlap", result.stderr)


if __name__ == "__main__":
    unittest.main()
