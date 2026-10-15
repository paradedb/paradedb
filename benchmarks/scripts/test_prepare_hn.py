"""Regression checks for the HN preparation contract."""

# pylint: disable=import-error

import csv
import json
import tempfile
import tomllib
import unittest
from pathlib import Path

import pyarrow.parquet as pq

import prepare_hn as hn


class HNPreparationTests(unittest.TestCase):
    """Check input semantics, identity, and artifact reproducibility."""

    def test_dataset_assets_match_preparation_contract(self):
        """Keep loader paths and vector dimensions consistent with preparation."""
        config = hn.load_config(hn.DEFAULT_CONFIG)
        directory = hn.DEFAULT_CONFIG.parent
        dataset = tomllib.loads((directory / "config.toml").read_text())
        self.assertEqual(dataset["s3_base_path"], config["artifact_prefix"])
        self.assertEqual(dataset["root_table"]["name"], "hn_items")
        self.assertIn(
            f"emb vector({config['embedding']['dimensions']})",
            (directory / "create_tables.sql").read_text(),
        )

    def test_html_normalization(self):
        """Handle HN markup, entities, empty text, and literal angle brackets."""
        for raw, expected in (
            (None, ""),
            ("\\N", ""),
            ("<p> &nbsp; </p>", ""),
            ("one<p>two<br>three</p>four", "one two three four"),
            ("foo<i>bar</i> &amp; &#x1F680;", "foobar & 🚀"),
            ("a<script>hidden</script><style>hidden</style>b", "ab"),
            ("&lt;p&gt;literal&lt;/p&gt;", "<p>literal</p>"),
            ("  a\n\tb  ", "a b"),
        ):
            with self.subTest(raw=raw):
                self.assertEqual(hn.normalize_text(raw), expected)

    def test_truncation_uses_token_ids_and_accepts_special_token_text(self):
        """Hash the submitted token prefix and treat special spellings as text."""
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "profile.parquet"
            hn.profile_batch(
                [
                    (1, "hello world alpha"),
                    (2, "hello world beta"),
                    (3, "<|endoftext|>"),
                ],
                path,
                2,
            )
            rows = pq.read_table(path).to_pylist()
        self.assertEqual(rows[0]["input_tokens"], 2)
        self.assertGreater(rows[0]["original_tokens"], 2)
        self.assertNotEqual(rows[0]["normalized_sha256"], rows[1]["normalized_sha256"])
        self.assertEqual(rows[0]["input_sha256"], rows[1]["input_sha256"])
        self.assertEqual(
            rows[0]["input_sha256"],
            hn.input_digest(hn.tokenizer().encode_ordinary("hello world")),
        )
        self.assertGreater(rows[2]["input_tokens"], 0)

    def test_holdout_and_manifest_are_order_and_worker_independent(self):
        """Exclude duplicates and preserve query identity across execution orders."""
        rows = [
            (1, ""),
            (2, "<p>duplicate</p>"),
            (3, "duplicate"),
            (4, "a unique input"),
            (5, "another unique input"),
            (6, "third unique input"),
            (7, "<br> &nbsp;"),
        ]
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            config = root / "prepare.toml"
            config.write_text(
                hn.DEFAULT_CONFIG.read_text().replace("count = 1000", "count = 2")
            )
            results = []
            for order, workers in ((rows, 1), (list(reversed(rows)), 2)):
                source = root / f"source-{workers}.csv"
                with source.open("w", newline="") as stream:
                    writer = csv.writer(stream)
                    writer.writerow(["id", "text"])
                    writer.writerows(order)
                output = root / f"out-{workers}"
                manifest = hn.prepare(source, output, config, workers)
                holdout = pq.read_table(output / "queries/holdout.parquet").to_pylist()
                results.append(holdout)
                self.assertEqual(manifest["counts"]["source_rows"], 7)
                self.assertEqual(manifest["counts"]["empty_rows"], 2)
                self.assertEqual(manifest["counts"]["corpus_rows"], 5)
                self.assertEqual(manifest["counts"]["corpus_vector_rows"], 3)
                self.assertEqual(manifest["counts"]["unique_inputs"], 4)
                self.assertEqual(
                    manifest["counts"]["rows_in_duplicate_normalized_texts"], 2
                )
                self.assertEqual(
                    manifest["source"]["csv_sha256"], hn.file_digest(source)
                )
                self.assertFalse(manifest["source"]["archive_verified"])
                self.assertEqual(
                    manifest, json.loads((output / "manifest.json").read_text())
                )
                for query in holdout:
                    self.assertIn(query["hn_id"], {4, 5, 6})
                    self.assertEqual(
                        query["input_sha256"], hn.input_digest(query["token_ids"]).hex()
                    )
                with self.assertRaisesRegex(ValueError, "must be empty"):
                    hn.prepare(source, output, config, workers)
            self.assertEqual(results[0], results[1])

    def test_duplicate_ids_and_insufficient_holdout_fail(self):
        """Reject invalid corpora before producing publishable artifacts."""
        config = hn.load_config(hn.DEFAULT_CONFIG)
        config["queries"]["count"] = 2
        for rows, error in (
            ([(1, "first"), (1, "second")], "duplicate HN IDs"),
            ([(1, "same"), (2, "same")], "unique inputs"),
        ):
            with self.subTest(error=error), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                hn.profile_batch(rows, root / "rows.parquet", 8192)
                with self.assertRaisesRegex(ValueError, error):
                    hn.summarize(root, config, "128MB")
                self.assertFalse((root / "manifest.json").exists())


if __name__ == "__main__":
    unittest.main()
