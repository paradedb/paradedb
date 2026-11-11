"""Check that finalize preserves the benchmark profiles."""

import itertools
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
        for (name, size, table), engine in itertools.product(
            [("hn", "28m", "hn_items"), ("msmarco", "113m", "msmarco")],
            ["pg_search", "pg_vectorsearch"],
        ):
            backend = "paradedb" if engine == "pg_search" else "timescale"
            suffix = "bm25" if engine == "pg_search" else "prism"
            profile_suffix = "" if engine == "pg_search" else "-pgvectorsearch"
            with (
                self.subTest(dataset=name, engine=engine),
                tempfile.TemporaryDirectory() as tmp,
            ):
                tmp = Path(tmp)
                schema = tmp / "pdbench" / "datasets" / name / "schema.yaml"
                schema.parent.mkdir(parents=True)
                schema.write_text(f"table: {table}\ncolumns:\n  emb: vector(1024)\n")
                profile = ROOT / "benchmarks" / name / f"{size}{profile_suffix}.toml"
                assets = ROOT / "benchmarks" / name / "assets"
                store = json.loads(
                    prepare(profile, assets, tmp / "pdbench", tmp / "build").read_text()
                )["datastore"]
                self.assertEqual(store["table"], table)
                self.assertEqual(
                    store["settings"],
                    tomllib.loads(profile.read_text())["postgres"]["build"],
                )
                self.assertEqual(store["url"], "${PARADEDB_URL}")
                self.assertEqual(store["container"], "${PARADEDB_CONTAINER}")
                self.assertEqual(store["vars"]["index_name"], f"{table}_{suffix}_idx")
                self.assertEqual(
                    set(store["vars"]["with_options"].split(", ")),
                    {
                        "training_sample_ratio=0.32",
                        "target_segment_count=8",
                        "vector_router_quantization=true",
                    }
                    if engine == "pg_search"
                    else {"nlist=0"},
                )
                dataset = Path(store["dataset"])
                self.assertEqual(
                    (dataset / "schema.yaml").read_text(), schema.read_text()
                )
                self.assertEqual(store["type"], backend)
                index_assets = assets / name
                if engine == "pg_vectorsearch":
                    index_assets /= engine
                self.assertEqual(
                    (dataset / backend / "pre.sql").read_text(),
                    (assets / name / "create_table.sql").read_text(),
                )
                self.assertEqual(
                    (dataset / backend / "post.sql").read_text(),
                    "DROP INDEX IF EXISTS {{ index_name }};\n"
                    + (index_assets / "index.sql").read_text(),
                )


if __name__ == "__main__":
    unittest.main()
