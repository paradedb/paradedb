"""Check build measurements independently of query and vacuum timings."""

import unittest

from index_metrics import metrics


class IndexMetricsTest(unittest.TestCase):
    """Protect the build timing boundary and size units."""

    def test_excludes_other_statement_times_and_preserves_fractional_size(self):
        """Only CREATE INDEX contributes to build time, even with vacuum output."""
        result = metrics(
            "Timing is on.\nSET\nTime: 3.000 ms\n"
            "CREATE INDEX\nTime: 120000.000 ms (02:00.000)\n"
            "VACUUM\nTime: 900000.000 ms (15:00.000)\n",
            {"index_bytes": 1572864, "segments": 8, "indexed_docs": 1000000},
        )
        self.assertEqual([row["value"] for row in result], [2, 1.5])
        self.assertEqual([row["unit"] for row in result], ["min", "MiB"])
        self.assertEqual(result[1]["extra"], "segments=8, indexed_docs=1000000")

    def test_rejects_missing_failed_or_ambiguous_builds(self):
        """Do not publish zeroes or substitute another statement's duration."""
        stats = {"index_bytes": 1024, "segments": 8, "indexed_docs": 1000000}
        for log in (
            "",
            "ERROR: build failed\nTime: 60000.000 ms\n",
            "VACUUM\nTime: 60000.000 ms\n",
            "CREATE INDEX\nTime: 0.000 ms\n",
            "CREATE INDEX\nTime: 1.000 ms\n" * 2,
        ):
            with self.subTest(log=log), self.assertRaises(ValueError):
                metrics(log, stats)

    def test_rejects_missing_or_invalid_index_stats(self):
        """An absent or empty index must not look like a size improvement."""
        for key in ("index_bytes", "segments", "indexed_docs"):
            for value in (None, 0, -1, True, "1024"):
                stats = {"index_bytes": 1024, "segments": 8, "indexed_docs": 1000000}
                stats[key] = value
                with self.subTest(key=key, value=value), self.assertRaises(ValueError):
                    metrics("CREATE INDEX\nTime: 1.000 ms\n", stats)


if __name__ == "__main__":
    unittest.main()
