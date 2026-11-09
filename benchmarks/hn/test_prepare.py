"""Reject fixture drift, held-out neighbors, and mismatched k6 vectors."""

import io
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import duckdb
from prepare import fetch, validate_ground_truth, verify_vectors


class FixturesTest(unittest.TestCase):
    """Protect corpus/query identity through the fixture handoff."""

    def test_checksum_failure_does_not_write_a_fixture(self):
        """Changed remote content must not become an accepted local fixture."""
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            with (
                patch("prepare.urlopen", return_value=io.BytesIO(b"changed")),
                self.assertRaisesRegex(ValueError, "Checksum mismatch"),
            ):
                fetch("queries/hn_queries.parquet", "0" * 64, output)
            self.assertFalse((output / "queries/hn_queries.parquet").exists())

    def test_ground_truth_requires_complete_unique_neighbors_without_holdout_leakage(
        self,
    ):
        """Reject missing queries, duplicate neighbors, and holdout leakage."""
        rows = [(0, [str(value) for value in range(10, 20)])]
        validate_ground_truth(rows, [0], {1})
        for invalid in (
            [],
            rows + rows,
            [(1, rows[0][1])],
            [(0, ["10"] * 10)],
            [(0, rows[0][1][:-1])],
            [(0, rows[0][1][:-1] + ["1"])],
            [(0, rows[0][1][:-1] + ["invalid"])],
        ):
            with self.subTest(rows=invalid), self.assertRaises(ValueError):
                validate_ground_truth(invalid, [0], {1})

    def test_export_matches_first_100_vectors_as_float32(self):
        """Accept round-trip decimals and reject wrong ordering or dimensions."""
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            (output / "queries").mkdir()
            path = output / "queries/hn_queries.parquet"
            with duckdb.connect() as db:
                db.execute(
                    "COPY (SELECT i::INT AS id, "
                    "list_transform(range(1536), lambda x: (i / 100.0)::FLOAT) AS emb "
                    "FROM range(101) AS t(i) ORDER BY id DESC) TO ? (FORMAT parquet)",
                    [str(path)],
                )
            vectors = [json.dumps([i / 100.0] * 1536) for i in range(100)]
            exported = output / "query_vectors.json"
            exported.write_text(json.dumps(vectors))
            verify_vectors(output, exported)
            self.assertEqual(
                len((output / "query-vectors.sha256").read_text().strip()), 64
            )
            for invalid in (
                vectors[:-1],
                list(reversed(vectors)),
                ["[0]"] + vectors[1:],
            ):
                exported.write_text(json.dumps(invalid))
                with self.subTest(count=len(invalid)), self.assertRaises(ValueError):
                    verify_vectors(output, exported)


if __name__ == "__main__":
    unittest.main()
