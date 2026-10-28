"""Exercise baseline comparisons and publication against a local Git remote."""

# pylint: disable=missing-function-docstring,consider-using-with

import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path

from benchmark_publication import PREFIX, prepare, read_history, record

SCRIPT = Path(__file__).with_name("benchmark-pages.sh")
ARCHIVED = "benchmarker-runs/old/dataset/archived.html"


class PublicationTests(unittest.TestCase):
    """Baseline updates, PR comparisons, and retries."""

    def setUp(self):
        self.root = Path(self.enterContext(tempfile.TemporaryDirectory()))
        self.history = self.root / "data.js"
        self.scratch = self.root / "publication"
        self.baseline = {
            "commit": {"id": "baseline"},
            "benches": [{"name": "latency", "value": 10}],
        }
        self.original = {
            "lastUpdate": 1,
            "repoUrl": "https://example.test",
            "entries": {
                "latency": [self.baseline],
                "throughput": [{"benches": []}],
            },
        }
        self.history.write_text(PREFIX + json.dumps(self.original))

    def append_result(self, suite="latency"):
        path = self.scratch / "history.json"
        history = json.loads(path.read_text())
        history["entries"].setdefault(suite, []).append(
            {
                "commit": {"id": "current"},
                "benches": [{"name": suite, "value": 12}],
            }
        )
        path.write_text(json.dumps(history))

    def test_baseline_updates_preserve_other_suites_and_comparison(self):
        self.assertFalse(
            prepare(self.history, self.scratch, "latency", "1-latency", True)
        )
        self.append_result()
        record(self.history, self.scratch, "latency", "1-latency", True)
        history = read_history(self.history)
        self.assertEqual(
            history["entries"]["throughput"], self.original["entries"]["throughput"]
        )
        self.assertEqual(
            history["entries"]["latency"][-1]["publication_id"], "1-latency"
        )
        self.assertEqual(
            read_history(self.scratch / "comparison.js")["entries"]["latency"][0],
            self.baseline,
        )
        self.assertEqual(history["repoUrl"], self.original["repoUrl"])

    def test_pr_comparison_does_not_change_baseline(self):
        original = self.history.read_bytes()
        prepare(self.history, self.scratch, "latency", "1-latency", False)
        self.append_result()
        record(self.history, self.scratch, "latency", "1-latency", False)
        self.assertEqual(self.history.read_bytes(), original)
        self.assertEqual(
            len(read_history(self.scratch / "comparison.js")["entries"]["latency"]), 2
        )
        prepare(self.history, self.scratch, "throughput", "1-throughput", False)
        self.assertEqual(
            json.loads((self.scratch / "history.json").read_text()), self.original
        )

    def test_retry_recovers_original_comparison_after_later_publication(self):
        prepare(self.history, self.scratch, "latency", "1-latency", True)
        self.append_result()
        record(self.history, self.scratch, "latency", "1-latency", True)
        comparison = (self.scratch / "comparison.js").read_bytes()
        prepare(self.history, self.scratch, "latency", "2-latency", True)
        self.append_result()
        record(self.history, self.scratch, "latency", "2-latency", True)
        original = self.history.read_bytes()
        self.assertTrue(
            prepare(self.history, self.scratch, "latency", "1-latency", True)
        )
        self.assertEqual((self.scratch / "comparison.js").read_bytes(), comparison)
        self.assertEqual(self.history.read_bytes(), original)

    def test_first_publication_and_retry_without_previous_baseline(self):
        self.history.unlink()
        prepare(self.history, self.scratch, "latency", "1-latency", True)
        self.append_result()
        record(self.history, self.scratch, "latency", "1-latency", True)
        self.assertTrue(
            prepare(self.history, self.scratch, "latency", "1-latency", True)
        )
        self.assertEqual(
            len(read_history(self.scratch / "comparison.js")["entries"]["latency"]), 1
        )

    def test_invalid_history_fails_instead_of_replacing_baseline(self):
        for text in ("not benchmark history", PREFIX + "invalid json"):
            with self.subTest(text=text):
                self.history.write_text(text)
                with self.assertRaises(ValueError):
                    prepare(self.history, self.scratch, "latency", "1-latency", True)


class PagesCheckoutTests(unittest.TestCase):
    """Selective downloads and atomic publication preserve unrelated pages."""

    def setUp(self):
        self.root = Path(self.enterContext(tempfile.TemporaryDirectory()))
        self.remote = self.root / "remote"
        self.remote.mkdir()
        self.workspace = self.root / "publisher"
        self.workspace.mkdir()
        self.checkout = self.workspace / "benchmark-data-repository"
        self.env = {
            **os.environ,
            "APP_TOKEN": "",
            "GIT_CONFIG_GLOBAL": os.devnull,
            "GIT_CONFIG_NOSYSTEM": "1",
        }
        self.git(self.remote, "init", "-b", "gh-pages")
        self.git(self.remote, "config", "user.name", "Test")
        self.git(self.remote, "config", "user.email", "test@example.test")
        self.git(self.remote, "config", "uploadpack.allowFilter", "true")
        self.git(self.remote, "config", "receive.denyCurrentBranch", "updateInstead")
        for directory in ("benchmarker", "benchmarks", "stressgres"):
            path = self.remote / directory
            path.mkdir()
            (path / "data.js").write_text(PREFIX + json.dumps({"entries": {}}))
            (path / "index.html").write_text(f"Existing {directory} chart")
        archived = self.remote / ARCHIVED
        archived.parent.mkdir(parents=True)
        archived.write_bytes(os.urandom(1024 * 1024))
        self.git(self.remote, "add", ".")
        self.git(self.remote, "commit", "-m", "Baseline and archived dashboards")
        self.initial_commit = self.git(self.remote, "rev-parse", "HEAD")
        self.archived_blob = self.git(self.remote, "rev-parse", f"HEAD:{ARCHIVED}")

    def git(self, directory, *args, check=True):
        result = subprocess.run(
            ["git", "-C", str(directory), *args],
            env=self.env,
            text=True,
            capture_output=True,
            check=check,
        )
        return result.stdout.strip()

    def run_script(self, operation, argument, check=True):
        return subprocess.run(
            ["bash", str(SCRIPT), operation, self.remote.as_uri(), argument],
            cwd=self.workspace,
            env=self.env,
            text=True,
            capture_output=True,
            check=check,
        )

    def assert_archive_not_downloaded(self):
        self.assertFalse((self.checkout / ARCHIVED).exists())
        objects = self.git(
            self.checkout,
            "cat-file",
            "--batch-all-objects",
            "--batch-check=%(objectname)",
        )
        self.assertNotIn(self.archived_blob, objects.splitlines())

    def test_partial_checkout_downloads_only_selected_metrics(self):
        for kind, directory in (
            ("benchmarker", "benchmarker"),
            ("queries", "benchmarks"),
            ("stressgres", "stressgres"),
        ):
            with self.subTest(kind=kind):
                self.workspace = self.root / kind
                self.workspace.mkdir()
                self.checkout = self.workspace / "benchmark-data-repository"
                self.run_script("checkout", kind)
                self.assertEqual(
                    self.git(self.checkout, "rev-parse", "--is-shallow-repository"),
                    "true",
                )
                self.assertEqual(
                    read_history(self.checkout / directory / "data.js"), {"entries": {}}
                )
                self.assertEqual(
                    (self.checkout / directory / "index.html").read_text(),
                    f"Existing {directory} chart",
                )
                self.assert_archive_not_downloaded()

    def test_one_push_preserves_archive_and_combines_metrics_and_dashboards(self):
        self.run_script("checkout", "benchmarker")
        history_path = self.checkout / "benchmarker/data.js"
        scratch = self.workspace / "_benchmark_publish"
        for suite in ("latency", "throughput", "index"):
            prepare(history_path, scratch, suite, f"1-{suite}", True)
            history = json.loads((scratch / "history.json").read_text())
            history["entries"][suite] = [{"benches": []}]
            (scratch / "history.json").write_text(json.dumps(history))
            record(history_path, scratch, suite, f"1-{suite}", True)
            self.git(self.checkout, "add", "benchmarker/data.js")
        dashboards = self.workspace / "_ghpages_html"
        dashboards.mkdir()
        (dashboards / "new.html").write_text("New dashboard")
        subprocess.run(
            ["bash", str(SCRIPT), "dashboards", "old", "dataset"],
            cwd=self.workspace,
            env=self.env,
            check=True,
            capture_output=True,
        )
        self.assertEqual(
            self.git(self.remote, "rev-parse", "HEAD"), self.initial_commit
        )
        self.run_script("push", "1")
        self.assertEqual(
            self.git(
                self.remote, "rev-list", "--count", f"{self.initial_commit}..HEAD"
            ),
            "1",
        )
        self.assertEqual(
            set(read_history(self.remote / "benchmarker/data.js")["entries"]),
            {"latency", "throughput", "index"},
        )
        self.assertEqual(
            self.git(self.remote, "rev-parse", f"HEAD:{ARCHIVED}"),
            self.archived_blob,
        )
        self.assertEqual(
            (self.remote / "benchmarker-runs/old/dataset/new.html").read_text(),
            "New dashboard",
        )
        self.assert_archive_not_downloaded()
        head = self.git(self.remote, "rev-parse", "HEAD")
        self.run_script("push", "1")
        self.assertEqual(self.git(self.remote, "rev-parse", "HEAD"), head)

    def test_push_refuses_to_overwrite_concurrent_changes(self):
        self.run_script("checkout", "benchmarker")
        (self.remote / "concurrent.txt").write_text("Another writer")
        self.git(self.remote, "add", "concurrent.txt")
        self.git(self.remote, "commit", "-m", "Concurrent publication")
        head = self.git(self.remote, "rev-parse", "HEAD")
        (self.checkout / "benchmarker/data.js").write_text(
            PREFIX + '{"entries":{"new":[]}}'
        )
        self.git(self.checkout, "add", "benchmarker/data.js")
        self.assertNotEqual(self.run_script("push", "1", check=False).returncode, 0)
        self.assertEqual(self.git(self.remote, "rev-parse", "HEAD"), head)


if __name__ == "__main__":
    unittest.main()
