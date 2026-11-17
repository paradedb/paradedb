"""Regression tests for publishing CREATE INDEX time instead of finalize time."""

import unittest
from urllib.parse import parse_qs, urlsplit

from vector_index_timing import statement_timings, timed_url


class IndexTimingTests(unittest.TestCase):
    """Check duration selection and PostgreSQL connection options."""

    def test_build_excludes_drop_vacuum_and_other_indexes(self):
        """A long VACUUM must not inflate the selected index's build metric."""
        log = """2026-11-17 00:00:00 UTC [10] LOG:  duration: 2.000 ms  statement:
DROP INDEX IF EXISTS search_idx;
2026-11-17 00:00:01 UTC [10] LOG:  duration: 120000.125 ms  statement: -- build
CREATE INDEX search_idx ON docs
USING paradedb (emb vector_cosine_ops);
2026-11-17 00:02:01 UTC [10] LOG:  duration: 900000.000 ms  statement: VACUUM ANALYZE docs;
2026-11-17 00:17:01 UTC [10] LOG:  duration: 400.000 ms  statement: CREATE INDEX other_idx ON docs (id);
"""
        result = statement_timings(log, "search_idx")
        self.assertEqual(result["build_seconds"], 120.000125)
        self.assertEqual(result["statements"][2]["seconds"], 900)

    def test_quoted_index_and_multiline_command(self):
        """Recognize quoted names and leading SQL comments in server logs."""
        log = """2026-11-17 00:00:00 UTC [10] LOG:  duration: 1234.567 ms  statement: /* build */
CREATE UNIQUE INDEX CONCURRENTLY "Search""Index" ON docs (id);
"""
        self.assertEqual(
            statement_timings(log, 'Search"Index')["build_seconds"], 1.234567
        )

    def test_missing_duplicate_and_zero_timing_fail(self):
        """Never silently substitute finalize duration when timing is ambiguous."""
        line = (
            "2026-11-17 00:00:00 UTC [10] LOG:  duration: 1.000 ms  statement: "
            "CREATE INDEX search_idx ON docs (id);\n"
        )
        for log in [
            "",
            line + line,
            line.replace("1.000", "0.000"),
            line.replace("search_idx", "other_idx"),
        ]:
            with self.subTest(log=log), self.assertRaises(ValueError):
                statement_timings(log, "search_idx")

    def test_connection_options_are_preserved_and_timing_is_session_only(self):
        """Keep connection settings and encode spaces for PostgreSQL URI parsing."""
        url = "postgresql://runner@localhost/db?sslmode=require&options=-c%20work_mem%3D256MB"
        parsed = urlsplit(timed_url(url))
        self.assertEqual(parsed.netloc, "runner@localhost")
        query = parse_qs(parsed.query)
        self.assertEqual(query["sslmode"], ["require"])
        self.assertEqual(
            query["options"], ["-c work_mem=256MB -c log_min_duration_statement=0"]
        )
        self.assertTrue(
            timed_url("postgresql:///postgres?host=/tmp").startswith(
                "postgresql:///postgres?"
            )
        )


if __name__ == "__main__":
    unittest.main()
