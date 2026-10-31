"""Protect same-index router experiments and strict recall comparisons."""

import copy
import json
import os
import subprocess
import tempfile
import tomllib
import unittest
from pathlib import Path
from unittest.mock import patch

from router_tuning import QUERY, VARIANTS, render_profile, run_tuning, selected_points, settings, validate_experiment


class RouterTuningTest(unittest.TestCase):
    def setUp(self):
        self.source = (Path(__file__).parent / "1m.toml").read_text()
        config = tomllib.loads(self.source)
        config["search"]["query"] = QUERY
        self.report = {
            "kind": "search", "profile": "1m", "fingerprint": "original",
            "config": config,
            "data": {
                "build": {"run_filename": "one-build.json"},
                "variants": {QUERY: {
                    "query": "SELECT id FROM cohere_wiki ORDER BY emb <=> current_setting('cohere.qvec')::vector LIMIT 10",
                    "points": [
                        {"value": str(value), "metrics": {"recall": {"recall": recall}, "latency": {"p50_ms": ms}}}
                        for value, recall, ms in [(0.1, 0.9, 4.0), (0.2, 0.95, 5.0), (0.3, 0.99, 7.0)]
                    ],
                    "targets": {str(recall): {"value": str(value), "recall": recall}
                                for value, recall in [(0.1, 0.9), (0.2, 0.95), (0.3, 0.99)]},
                }},
            },
        }

    def candidate(self, variant):
        result = copy.deepcopy(self.report)
        result["config"]["search"]["session"].update(settings(variant))
        return result

    def test_profiles_preserve_build_and_only_override_router_session_settings(self):
        base = tomllib.loads(self.source)
        for variant in VARIANTS:
            profile = tomllib.loads(render_profile(self.source, variant))
            self.assertEqual(profile["dataset"], base["dataset"])
            self.assertEqual(profile["index"], base["index"])
            self.assertEqual(profile["search"]["session"], base["search"]["session"] | settings(variant))
        with self.assertRaisesRegex(ValueError, "already overrides"):
            render_profile(render_profile(self.source, VARIANTS[0]), VARIANTS[1])

    def test_build_or_unrelated_config_drift_cannot_pass(self):
        variant = VARIANTS[1]
        validate_experiment(self.report, self.candidate(variant), variant)
        for mutate in (
            lambda r: r["data"]["build"].update(run_filename="rebuilt.json"),
            lambda r: r["config"]["index"].update(target_segment_count=1),
            lambda r: r["config"]["search"]["session"].update(work_mem="1GB"),
            lambda r: r["config"]["search"]["session"].update({next(iter(settings(variant))): "0.1"}),
        ):
            candidate = self.candidate(variant)
            mutate(candidate)
            with self.assertRaises(ValueError):
                validate_experiment(self.report, candidate, variant)

    def test_strict_target_uses_selected_point_and_preserves_unreached_targets(self):
        result = self.report["data"]["variants"][QUERY]
        result["points"].append({"value": "0.15", "metrics": {"recall": {"recall": 0.949}, "latency": {"p50_ms": 1.0}}})
        result["targets"]["0.99"] = None
        rows = selected_points(self.report)
        self.assertEqual(rows[1]["probe"], "0.2")
        self.assertEqual(rows[1]["p50_ms"], 5.0)
        self.assertIsNone(rows[2]["p50_ms"])
        result["targets"]["0.95"] = {"value": "0.15", "recall": 0.949}
        with self.assertRaisesRegex(ValueError, "does not meet"):
            selected_points(self.report)

    def test_runner_searches_one_index_and_verifies_each_requested_mode(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            profile = root / "1m.toml"
            profile.write_text(self.source)
            directory = root / "artifacts"
            initial = directory / "search" / "1m"
            initial.mkdir(parents=True)
            (initial / "default_search.json").write_text(json.dumps(self.report))
            (directory / "query_vectors.json").write_text(json.dumps(["[0]"]))
            commands = []

            def run(command, **kwargs):
                commands.append(command)
                self.assertNotIn("build", command)
                if command[1] == "search":
                    generated = Path(command[command.index("--config") + 1])
                    report = copy.deepcopy(self.report)
                    report["config"] = tomllib.loads(generated.read_text())
                    report["config"]["search"]["query"] = QUERY
                    report["profile"] = generated.stem
                    output = directory / "search" / generated.stem
                    output.mkdir()
                    (output / "one_search.json").write_text(json.dumps(report))
                return subprocess.CompletedProcess(command, 0)

            with (
                patch.dict(os.environ, {
                    "PARADEDB_URL": "postgresql://test",
                    "GITHUB_STEP_SUMMARY": str(root / "summary.md"),
                }),
                patch("router_tuning.subprocess.run", side_effect=run),
                patch("router_tuning.verify_point", return_value={"routed_segments": 8}) as verify,
            ):
                records = run_tuning(profile, directory, root / "assets", variants=(VARIANTS[0], VARIANTS[4]))
            self.assertEqual([c[1] for c in commands], ["search", "search", "compare"])
            self.assertEqual([c.kwargs["expected_mode"] for c in verify.call_args_list], ["hierarchical", "flat"])
            self.assertEqual(records["flat-c2"]["targets"][1]["p50_ms"], 5.0)
            self.assertIn("Router compositions", (directory / "operating_points.md").read_text())
