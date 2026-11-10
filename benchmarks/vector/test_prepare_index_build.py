"""Check that finalize preserves the benchmark profiles."""

import json
import tempfile
import unittest
from pathlib import Path

import tomllib
from prepare_index_build import prepare

ROOT = Path(__file__).resolve().parents[2]


class PrepareIndexBuildTest(unittest.TestCase):
    """Exercise both restored-heap configurations."""

    def test_profiles_preserve_index_sql_and_settings(self):
        """Keep schema, index expressions, reloptions, and memory settings intact."""
        for name, size, table in [
            ("hn", "28m", "hn_items"),
            ("msmarco", "113m", "msmarco"),
        ]:
            with self.subTest(dataset=name), tempfile.TemporaryDirectory() as tmp:
                tmp = Path(tmp)
                schema = tmp / "pdbench" / "datasets" / name / "schema.yaml"
                schema.parent.mkdir(parents=True)
                schema.write_text(f"table: {table}\ncolumns:\n  emb: vector(1024)\n")
                profile = ROOT / "benchmarks" / name / f"{size}.toml"
                assets = ROOT / "benchmarks" / name / "assets"
                path = prepare(profile, assets, tmp / "pdbench", tmp / "build")
                load = json.loads(path.read_text())
                search = tomllib.loads(profile.read_text())
                store = load["datastore"]
                self.assertEqual(store["table"], table)
                self.assertEqual(store["settings"], search["postgres"]["build"])
                self.assertEqual(store["url"], "${PARADEDB_URL}")
                self.assertEqual(store["container"], "${PARADEDB_CONTAINER}")
                self.assertEqual(store["vars"]["index_name"], f"{table}_bm25_idx")
                self.assertEqual(
                    set(store["vars"]["with_options"].split(", ")),
                    {
                        "training_sample_ratio=0.32",
                        "target_segment_count=8",
                        "vector_router_quantization=true",
                    },
                )
                dataset = Path(store["dataset"])
                self.assertEqual(
                    (dataset / "schema.yaml").read_text(), schema.read_text()
                )
                self.assertEqual(
                    (dataset / "paradedb/post.sql").read_text(),
                    "DROP INDEX IF EXISTS {{ index_name }};\n"
                    + (assets / name / "index.sql").read_text(),
                )


if __name__ == "__main__":
    unittest.main()
