import unittest

from generate_release_notes import credit_original_authors


class ReleaseNotesTests(unittest.TestCase):
    def setUp(self):
        self.pulls = {
            10: {"user": {"login": "alice"}, "labels": [], "body": ""},
            20: {
                "user": {"login": "release[bot]"},
                "labels": [{"name": "automated-cherry-pick"}],
                "body": "# Description\nBackport of #10 to `0.26.x`.",
            },
        }

    def render(self, number, author="release[bot]"):
        body = (
            "## What's Changed\n"
            f"* fix: issue (#99) by @{author} in https://github.com/org/repo/pull/{number}\n"
            "\n**Full Changelog**: https://github.com/org/repo/compare/v1...v2\n"
        )
        return credit_original_authors(body, "org/repo", self.pulls.__getitem__)

    def test_backport_credits_source_and_preserves_links(self):
        result = self.render(20)
        self.assertIn("issue (#99) by @alice in https://github.com/org/repo/pull/20", result)
        self.assertIn("**Full Changelog**: https://github.com/org/repo/compare/v1...v2", result)

    def test_manual_pr_keeps_author(self):
        self.assertIn("by @alice in", self.render(10, "alice"))

    def test_unlabelled_reference_is_not_followed(self):
        self.pulls[20]["labels"] = []
        self.assertIn("by @release[bot] in", self.render(20))

    def test_nested_backport(self):
        self.pulls[30] = dict(self.pulls[20], body="Backport of #20 to `0.25.x`.")
        self.assertIn("by @alice in", self.render(30))

    def test_missing_reference_fails(self):
        self.pulls[20]["body"] = None
        with self.assertRaisesRegex(ValueError, "Cannot find original PR"):
            self.render(20)

    def test_cycle_fails(self):
        self.pulls[20]["body"] = "Backport of #20 to `0.26.x`."
        with self.assertRaisesRegex(ValueError, "cycle"):
            self.render(20)


if __name__ == "__main__":
    unittest.main()
