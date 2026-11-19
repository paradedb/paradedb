"""Checks the Benchmarker PR comment: visible sections, the regressions filter and the collapsed
tables."""

import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).with_name("benchmarker_comment.sh")
HEADER = "| Benchmark | Current | Previous | Ratio |\n|---|---:|---:|---:|\n"
FOOTNOTE = "\nRatio: current/previous for latency"


def bench(name, value, unit="ms"):
    """Build a published metric."""
    return {"name": name, "unit": unit, "value": value}


# q p50 is 1.2x and q QPS 1.25x (previous / current); q p99 sits exactly at 1.15x and r p50 has no
# baseline, so neither is a regression.
LATENCY = [
    bench("q p50 latency", 12),
    bench("q p99 latency", 11.5),
    bench("r p50 latency", 5),
]
THROUGHPUT = [bench("q QPS", 80, "queries/s")]
LATENCY_BASELINE = [bench("q p50 latency", 10), bench("q p99 latency", 10)]
THROUGHPUT_BASELINE = [bench("q QPS", 100, "queries/s")]
ROWS = {
    "q p50": "| `q p50 latency` | 12 ms | 10 ms | 1.2x ⚠️ |",
    "q p99": "| `q p99 latency` | 11.5 ms | 10 ms | 1.15x |",
    "q QPS": "| `q QPS` | 80 queries/s | 100 queries/s | 1.25x ⚠️ |",
    "r p50": "| `r p50 latency` | 5 ms | n/a | n/a |",
}


def comment(
    latency=None,
    throughput=None,
    latency_baseline=None,
    dataset="cohere-1m",
):
    """Runs the script on fixture files the way the publisher lays them out."""
    latency = LATENCY if latency is None else latency
    throughput = THROUGHPUT if throughput is None else throughput
    latency_baseline = (
        LATENCY_BASELINE if latency_baseline is None else latency_baseline
    )
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        files = {
            "latency.json": latency,
            "throughput.json": throughput,
            "_latency_baseline.json": {
                "commit": {"id": "a" * 40},
                "benches": latency_baseline,
            },
            "_throughput_baseline.json": {
                "commit": {"id": "b" * 40},
                "benches": THROUGHPUT_BASELINE,
            },
            "_buffer_baseline.json": {},
            "_buffer_metrics.json": [],
            "_index_baseline.json": {},
            "_index_metrics.json": [],
        }
        for name, content in files.items():
            (root / name).write_text(json.dumps(content))
        context = "Context line.\n\n<details><summary>Operating points</summary>\n\n</details>\n"
        (root / "context.md").write_text(context)
        (root / "_ghpages_html").mkdir()
        (root / "_ghpages_html" / "search.html").write_text("")
        env = os.environ | {
            "DATASET": dataset,
            "COMMIT_SHA": "c" * 40,
            "GITHUB_REPOSITORY_OWNER": "paradedb",
            "EVENT_REPOSITORY_NAME": "paradedb",
            "METRICS_FILE": "latency.json",
            "THROUGHPUT_METRICS_FILE": "throughput.json",
            "CONTEXT_FILE": "context.md",
            "SWEEP_REPORTS_DIR": "",
        }
        result = subprocess.run(
            ["bash", str(SCRIPT)],
            cwd=root,
            env=env,
            check=True,
            capture_output=True,
            text=True,
        )
        return result.stdout


class CommentTests(unittest.TestCase):
    """Validate visible comparisons and detailed historical results."""

    def test_sections_in_order_with_tables_collapsed(self):
        """Keep the main findings visible and details collapsed."""
        text = comment()
        order = [
            "### 📊 Benchmarker results (cohere-1m): `ccccccc`\n\nContext line.",
            "<details><summary>Operating points</summary>",
            "Baselines: latency `aaaaaaa`, QPS `bbbbbbb`.",
            "**⚠️ 2 regressions above 1.15x**",
            FOOTNOTE,
            "<details><summary>All results (4 rows)</summary>\n\n" + HEADER,
            "</details>\n\n<details><summary>Reports</summary>\n\n##### Search",
            "_gh-pages may take ~a minute to deploy after this run._\n\n</details>\n",
        ]
        positions = [text.index(part) for part in order]
        self.assertEqual(positions, sorted(positions))
        self.assertTrue(text.endswith(order[-1]))
        self.assertEqual(text.count("<details>"), text.count("</details>"))

    def test_regressions_hold_only_rows_marked_above_the_threshold(self):
        """Only regressions appear in the visible historical comparison."""
        text = comment()
        visible = text.split(FOOTNOTE)[0].split("**⚠️ 2 regressions above 1.15x**\n\n")[
            1
        ]
        self.assertEqual(visible, HEADER + ROWS["q p50"] + "\n" + ROWS["q QPS"] + "\n")
        after = text.split("<details><summary>All results (4 rows)</summary>\n\n")[1]
        collapsed = after.split("\n\n</details>")[0]
        self.assertEqual(
            collapsed,
            HEADER + "\n".join(ROWS[k] for k in ("q p50", "q p99", "q QPS", "r p50")),
        )

    def test_one_regression_and_none(self):
        """Render singular and empty regression summaries."""
        one = comment(throughput=[bench("q QPS", 100, "queries/s")])
        regression = "**⚠️ 1 regression above 1.15x**\n\n"
        self.assertIn(regression + HEADER + ROWS["q p50"] + "\n" + FOOTNOTE, one)
        none = comment(
            throughput=[bench("q QPS", 100, "queries/s")],
            latency_baseline=[bench("q p50 latency", 12)],
        )
        baselines = "Baselines: latency `aaaaaaa`, QPS `bbbbbbb`.\n\n"
        self.assertIn(baselines + "✅ No regressions above 1.15x.\n" + FOOTNOTE, none)
        self.assertEqual(none.count(HEADER), 1)

    def test_stackexchange_pairs_styles_and_reports_achieved_updates(self):
        """Pair shuffled metrics by query style and report measured writes."""
        latency = []
        throughput = []
        expected = []
        for offset, style in enumerate(
            ["conjunction", "disjunction", "phrase", "mixed"]
        ):
            static = f"paradedb (topk_{style}) (topk-{style})"
            concurrent = f"paradedb (topk_{style}) (topk-{style}-concurrent-updates)"
            writer = f"updates (random_updates) (topk-{style}-concurrent-updates)"
            latency.extend(
                [
                    bench(f"{static} p50 latency", 10 + offset),
                    bench(f"{concurrent} p50 latency", 20 + offset),
                    bench(f"{static} p95 latency", 30 + offset),
                    bench(f"{concurrent} p95 latency", 40 + offset),
                ]
            )
            throughput.extend(
                [
                    bench(f"{static} QPS", 90 + offset, "queries/s"),
                    bench(f"{concurrent} QPS", 50 + offset, "queries/s"),
                    bench(f"{writer} updates/sec", 700 + offset, "updates/s"),
                ]
            )
            expected.append(
                f"| {style} | {90 + offset} | {50 + offset} | {10 + offset} | "
                f"{20 + offset} | {30 + offset} | {40 + offset} | {700 + offset} |"
            )
        text = comment(latency[::-1], throughput[::-1], dataset="stackexchange")
        self.assertIn("\n".join(expected), text)
        self.assertLess(
            text.index(expected[0]), text.index("<details><summary>All results")
        )
        self.assertNotIn("Static vs. concurrent updates", comment(latency, throughput))

    def test_stackexchange_without_concurrent_results_omits_comparison(self):
        """Historical static-only results do not imply concurrent measurements."""
        self.assertNotIn(
            "Static vs. concurrent updates", comment(dataset="stackexchange")
        )

    def test_stackexchange_missing_metrics_are_not_zero(self):
        """Leave missing read measurements unavailable and round writer throughput."""
        throughput = [
            bench(
                "updates (random_updates) (topk-disjunction-concurrent-updates) updates/sec",
                987.654321,
                "updates/s",
            )
        ]
        text = comment(latency=[], throughput=throughput, dataset="stackexchange")
        self.assertIn(
            "| disjunction | n/a | n/a | n/a | n/a | n/a | n/a | 987.654 |", text
        )


if __name__ == "__main__":
    unittest.main()
