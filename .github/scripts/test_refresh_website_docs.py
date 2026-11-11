"""Tests for docs deployment gating and API failures."""

# pylint: disable=missing-function-docstring

import copy
import io
import json
import os
import tempfile
import unittest
import urllib.error
from pathlib import Path
from unittest.mock import patch

import refresh_website_docs as refresh

HOOK = "https://api.vercel.com/v1/integrations/deploy/project/secret"
EVENT = {
    "action": "completed",
    "repository": {"full_name": "paradedb/paradedb"},
    "check_run": {
        "app": {"id": 222410, "slug": "mintlify"},
        "name": "Mintlify Deployment",
        "conclusion": "success",
        "head_sha": "docs-commit",
        "check_suite": {"id": 42},
        "output": {
            "summary": "Your changes are now live at https://www.paradedb.com/docs!"
        },
    },
}
SUITE = {
    "app": {"id": 222410},
    "head_branch": "main",
    "head_sha": "docs-commit",
}
QUEUED = {"job": {"id": "build-job", "state": "PENDING"}}


class DocsRefreshTests(unittest.TestCase):
    """Only published production docs should trigger a website build."""

    def setUp(self):
        env = patch.dict(
            os.environ,
            {
                "GH_TOKEN": "github-token",
                "MINTLIFY_API_KEY": "mintlify-token",
                "MINTLIFY_PROJECT_ID": "docs-project",
                "WEBSITE_DEPLOY_HOOK_URL": HOOK,
            },
            clear=True,
        )
        env.start()
        self.addCleanup(env.stop)
        output = patch("sys.stdout", new=io.StringIO())
        output.start()
        self.addCleanup(output.stop)

    def run_check(self, event):
        with tempfile.TemporaryDirectory() as directory:
            event_path = Path(directory) / "event.json"
            event_path.write_text(json.dumps(event))
            with (
                patch.dict(os.environ, {"GITHUB_EVENT_PATH": str(event_path)}),
                patch("sys.argv", ["refresh_website_docs.py", "check"]),
            ):
                refresh.main()

    @patch.object(refresh, "request_json", side_effect=[SUITE, QUEUED])
    def test_production_check_queues_website_build(self, request):
        self.run_check(EVENT)
        self.assertEqual(request.call_count, 2)
        self.assertEqual(request.call_args.args, ("Vercel", HOOK))
        self.assertEqual(request.call_args.kwargs, {"method": "POST"})

    @patch.object(refresh, "request_json")
    def test_failed_skipped_preview_and_foreign_checks_do_nothing(self, request):
        variants = []
        for conclusion in ("failure", "skipped", "cancelled"):
            event = copy.deepcopy(EVENT)
            event["check_run"]["conclusion"] = conclusion
            variants.append(event)
        for field, value in (
            ("app", {"id": 1, "slug": "mintlify"}),
            ("name", "Other Check"),
            ("output", {"summary": "Preview is live at https://preview.mintlify.app"}),
        ):
            event = copy.deepcopy(EVENT)
            event["check_run"][field] = value
            variants.append(event)
        event = copy.deepcopy(EVENT)
        event["repository"]["full_name"] = "someone/paradedb"
        variants.append(event)
        for event in variants:
            with self.subTest(event=event):
                self.run_check(event)
        request.assert_not_called()

    @patch.object(refresh, "request_json")
    def test_suite_must_match_main_commit_and_app(self, request):
        for field, value in (
            ("head_branch", "feature-branch"),
            ("head_sha", "different-commit"),
            ("app", {"id": 1}),
        ):
            with self.subTest(field=field):
                request.reset_mock()
                request.return_value = {**SUITE, field: value}
                self.run_check(EVENT)
                self.assertEqual(request.call_count, 1)

    @patch.object(refresh.time, "sleep")
    @patch.object(refresh, "request_json")
    def test_publish_waits_for_success_before_rebuilding(self, request, sleep):
        request.side_effect = [
            {"statusId": "deployment-id"},
            {"status": "queued"},
            {"status": "in_progress"},
            {"status": "success"},
            QUEUED,
        ]
        refresh.publish_docs()
        self.assertEqual(request.call_count, 5)
        self.assertEqual(sleep.call_count, 2)
        self.assertEqual(request.call_args.args, ("Vercel", HOOK))
        self.assertTrue(
            all(
                call.args[1].endswith("/update-status/deployment-id")
                for call in request.call_args_list[1:4]
            )
        )

    @patch.object(refresh, "request_json")
    def test_failed_or_unknown_status_never_rebuilds(self, request):
        for status in ("failure", "unexpected", None):
            with self.subTest(status=status):
                request.reset_mock()
                request.side_effect = [
                    {"statusId": "deployment-id"},
                    {"status": status},
                ]
                with self.assertRaises(RuntimeError):
                    refresh.publish_docs()
                self.assertEqual(request.call_count, 2)

    @patch.object(refresh.time, "monotonic", side_effect=[0, 901])
    @patch.object(refresh, "request_json", return_value={"statusId": "deployment-id"})
    def test_timeout_never_rebuilds(self, request, _monotonic):
        with self.assertRaisesRegex(RuntimeError, "timed out"):
            refresh.publish_docs()
        self.assertEqual(request.call_count, 1)

    @patch.object(refresh, "request_json", return_value={})
    def test_missing_status_id_never_rebuilds(self, request):
        with self.assertRaisesRegex(RuntimeError, "status ID"):
            refresh.publish_docs()
        self.assertEqual(request.call_count, 1)

    @patch.object(refresh, "request_json")
    def test_missing_hook_fails_before_publishing(self, request):
        os.environ.pop("WEBSITE_DEPLOY_HOOK_URL")
        with self.assertRaisesRegex(RuntimeError, "Missing WEBSITE_DEPLOY_HOOK_URL"):
            refresh.publish_docs()
        request.assert_not_called()

    @patch.object(refresh, "request_json", return_value=QUEUED)
    def test_manual_refresh_does_not_republish_docs(self, request):
        with patch("sys.argv", ["refresh_website_docs.py", "refresh"]):
            refresh.main()
        request.assert_called_once_with("Vercel", HOOK, method="POST")

    @patch.object(refresh, "request_json")
    def test_rejected_build_is_not_reported_as_queued(self, request):
        for response in (
            {},
            {"job": "invalid"},
            {"job": {"id": "x", "state": "ERROR"}},
        ):
            with self.subTest(response=response):
                request.return_value = response
                with self.assertRaisesRegex(RuntimeError, "did not queue"):
                    refresh.refresh_website()

    @patch.object(refresh.urllib.request, "urlopen")
    def test_hook_errors_do_not_expose_secret_url_or_retry_posts(self, urlopen):
        for error in (
            urllib.error.HTTPError(HOOK, 500, HOOK, {}, None),
            urllib.error.URLError(HOOK),
        ):
            with self.subTest(error=type(error).__name__):
                urlopen.reset_mock()
                urlopen.side_effect = error
                with self.assertRaises(RuntimeError) as caught:
                    refresh.request_json("Vercel", HOOK, method="POST")
                self.assertNotIn("secret", str(caught.exception))
                self.assertEqual(urlopen.call_count, 1)

    @patch.object(refresh.time, "sleep")
    @patch.object(refresh.urllib.request, "urlopen")
    def test_status_reads_retry_transient_errors(self, urlopen, sleep):
        response = unittest.mock.MagicMock()
        response.__enter__.return_value = io.StringIO('{"status":"success"}')
        urlopen.side_effect = [
            urllib.error.HTTPError(
                "https://api.mintlify.com", 429, "rate limit", {}, None
            ),
            response,
        ]
        self.assertEqual(
            refresh.request_json("Mintlify", "https://api.mintlify.com"),
            {"status": "success"},
        )
        sleep.assert_called_once_with(5)


if __name__ == "__main__":
    unittest.main()
