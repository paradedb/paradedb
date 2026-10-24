"""Regression tests for release and upgrade-test version selection."""

import contextlib
import io
import tempfile
import unittest
from argparse import Namespace
from pathlib import Path
from unittest.mock import patch

import release


class UpgradeVersionSelectionTests(unittest.TestCase):
    """New release tags must not disconnect an older checkout's migration chain."""

    def test_newer_tag_does_not_change_checkout_upgrade_chain(self):
        """Reproduce a PR with SQL through 0.26.2 after 0.26.3 is tagged."""
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            sql_dir = root / "pg_search" / "sql"
            sql_dir.mkdir(parents=True)
            for prior, target in [("0.25.0", "0.26.1"), ("0.26.1", "0.26.2")]:
                (sql_dir / f"pg_search--{prior}--{target}.sql").touch()

            for tags in [["0.26.1", "0.26.2"], ["0.26.1", "0.26.2", "0.26.3"]]:
                with (
                    self.subTest(tags=tags),
                    patch.object(release, "get_git_tags", return_value=tags),
                ):
                    self.assertEqual(
                        release.latest_released_sql_version(sql_dir), "0.26.2"
                    )

            # Once the release migration is in the checkout, discovery advances.
            (sql_dir / "pg_search--0.26.2--0.26.3.sql").touch()
            with patch.object(
                release, "get_git_tags", return_value=["0.26.2", "0.26.3"]
            ):
                self.assertEqual(release.latest_released_sql_version(sql_dir), "0.26.3")

    def test_unreleased_sql_target_is_not_an_upgrade_starting_release(self):
        """Only tagged versions are available for the initial release checkout."""
        with tempfile.TemporaryDirectory() as directory:
            sql_dir = Path(directory)
            (sql_dir / "pg_search--0.26.2--0.26.3.sql").touch()
            (sql_dir / "pg_search--0.26.3--0.27.0.sql").touch()
            with patch.object(
                release, "get_git_tags", return_value=["0.26.3", "0.28.0"]
            ):
                self.assertEqual(release.latest_released_sql_version(sql_dir), "0.26.3")

    def test_synthetic_upgrade_extends_untagged_migration_tip(self):
        """A missing release tag must not cause CI to bypass an existing migration."""
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            sql_dir = root / "pg_search" / "sql"
            sql_dir.mkdir(parents=True)
            (sql_dir / "pg_search--0.25.0--0.26.3.sql").touch()
            (sql_dir / "pg_search--0.26.3--0.26.4.sql").write_text(
                "SELECT 'release migration';"
            )
            with patch.object(release, "get_git_tags", return_value=["0.26.3"]):
                selected = {}
                for released, expected in [(True, "v0.26.3"), (False, "v0.26.4")]:
                    with (
                        self.subTest(released=released),
                        contextlib.redirect_stdout(io.StringIO()) as output,
                    ):
                        release.handle_latest_sql_version_command(
                            Namespace(released=released), root
                        )
                        selected[released] = output.getvalue().strip()
                        self.assertEqual(selected[released], expected)
                args = release.build_parser().parse_args(
                    [
                        "sql",
                        "999.0.0",
                        "--prev-version",
                        selected[False],
                        "--preserve-fragments",
                    ]
                )
                with contextlib.redirect_stdout(io.StringIO()):
                    release.handle_sql_command(args, root)
                self.assertTrue((sql_dir / "pg_search--0.26.4--999.0.0.sql").exists())
                self.assertFalse((sql_dir / "pg_search--0.26.3--999.0.0.sql").exists())

    def test_release_uses_newest_tag_before_migration_sync(self):
        """Real releases still extend the latest release, even before its SQL sync."""
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "pg_search--0.26.2--0.26.3.sql").touch()
            with patch.object(
                release, "get_git_tags", return_value=["0.26.3", "0.26.4"]
            ):
                self.assertEqual(
                    release.resolve_prev_version(root, root, "0.27.0", None), "0.26.4"
                )

    def test_explicit_predecessor_still_takes_priority(self):
        """Release callers can still choose a predecessor explicitly."""
        self.assertEqual(
            release.resolve_prev_version(Path("."), Path("."), "999.0.0", "v0.25.0"),
            "0.25.0",
        )

    def test_tags_remain_a_fallback_without_sql_targets(self):
        """Keep the existing bootstrap behavior when the SQL chain is empty."""
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with patch.object(
                release, "get_git_tags", return_value=["0.26.2", "0.26.3"]
            ):
                self.assertEqual(
                    release.resolve_prev_version(root, root, "0.26.3", None), "0.26.2"
                )
                self.assertIsNone(release.latest_released_sql_version(root))


if __name__ == "__main__":
    unittest.main()
