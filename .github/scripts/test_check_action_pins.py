"""Regression coverage for the workflow pin policy."""

import unittest

import yaml
from check_action_pins import check_document


class ActionPinTests(unittest.TestCase):
    """Cover actions, reusable workflows, and YAML syntax variants."""

    def test_mutable_inline_and_quoted_references_are_rejected(self):
        document = yaml.safe_load(
            'jobs: {build: {uses: "owner/repo/.github/workflows/build.yml@main"}}\n'
            "steps:\n  - uses: 'actions/checkout@v7'\n"
        )
        self.assertEqual(len(check_document(document)), 2)

    def test_immutable_remote_and_local_actions_are_allowed(self):
        document = {
            "steps": [
                {"uses": ref}
                for ref in (
                    "actions/checkout@" + "a" * 40,
                    "owner/repo/.github/workflows/build.yml@" + "b" * 40,
                    "./.github/actions/local",
                    "docker://alpine@sha256:" + "c" * 64,
                )
            ]
        }
        self.assertEqual(check_document(document), [])

    def test_short_sha_and_container_tag_are_rejected(self):
        self.assertEqual(
            check_document(
                {
                    "steps": [
                        {"uses": "owner/repo@abcdef0"},
                        {"uses": "docker://alpine:latest"},
                    ]
                }
            ),
            ["owner/repo@abcdef0", "docker://alpine:latest"],
        )


if __name__ == "__main__":
    unittest.main()
