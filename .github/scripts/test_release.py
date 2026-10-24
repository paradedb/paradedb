"""Regression tests for release and upgrade-test version selection."""

import tempfile
import unittest
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
                        release.resolve_prev_version(
                            root,
                            sql_dir,
                            "999.0.0",
                            release.latest_released_sql_version(sql_dir),
                        ),
                        "0.26.2",
                    )
                    self.assertEqual(
                        release.latest_released_sql_version(sql_dir), "0.26.2"
                    )

            # Once the release migration is in the checkout, both selections advance.
            (sql_dir / "pg_search--0.26.2--0.26.3.sql").touch()
            with patch.object(
                release, "get_git_tags", return_value=["0.26.2", "0.26.3"]
            ):
                self.assertEqual(
                    release.resolve_prev_version(
                        root,
                        sql_dir,
                        "999.0.0",
                        release.latest_released_sql_version(sql_dir),
                    ),
                    "0.26.3",
                )
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
