"""Regression tests for release and upgrade-test version selection."""

import contextlib
import io
import subprocess
import tempfile
import unittest
from argparse import Namespace
from pathlib import Path
from unittest.mock import patch

import release


class ChangelogSectionOrderTests(unittest.TestCase):
    """Release notes put compatibility changes before improvements."""

    def test_rendered_sections_follow_release_order(self):
        """Sections render in the canonical compatibility-first order."""
        root = Path(__file__).resolve().parents[2]
        headers = release.load_headers_map(root / ".changelog_headers.json")
        order = ["breaking", "features", "performance", "stability", "docs", "other"]
        grouped = {key: [f"Change in {key}"] for key in reversed(order)}
        rendered = release.render_changelog("0.26.5", headers, grouped, [])
        positions = [rendered.index(headers[key]) for key in order]
        self.assertEqual(positions, sorted(positions))

    def test_empty_sections_are_omitted(self):
        """Sections without entries are excluded from rendered notes."""
        root = Path(__file__).resolve().parents[2]
        headers = release.load_headers_map(root / ".changelog_headers.json")
        rendered = release.render_changelog(
            "0.26.5", headers, {"stability": ["Fix a crash"]}, []
        )
        self.assertIn(headers["stability"], rendered)
        for key in ["breaking", "features", "performance", "docs", "other"]:
            self.assertNotIn(headers[key], rendered)


def migration_project(root):
    """A minimal opted-in crate lets these tests run the actual pgrx assembler."""
    (root / "Cargo.toml").write_text(
        '[workspace]\nmembers = ["pg_search"]\nresolver = "2"\n'
    )
    crate = root / "pg_search"
    crate.mkdir(exist_ok=True)
    (crate / "Cargo.toml").write_text(
        '[package]\nname = "pg_search"\nversion = "0.26.6"\nedition = "2024"\n'
        '[lib]\npath = "lib.rs"\n'
        '[package.metadata.pgrx]\nunreleased-sql-dir = "sql/unreleased"\n'
    )
    (crate / "lib.rs").touch()
    (crate / "pg_search.control").write_text("default_version = '@CARGO_VERSION@'\n")
    (crate / "sql/unreleased").mkdir(parents=True, exist_ok=True)


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
                migration_project(root)
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


class ChangelogIndexingTests(unittest.TestCase):
    """Latest promotions and maintenance syncs preserve indexing policy."""

    def test_latest_promotion_archives_older_pages_in_both_layouts(self):
        """Promoting a release archives its predecessor without changing page bodies."""
        for layout in ["project/changelog", "changelog"]:
            with (
                self.subTest(layout=layout),
                tempfile.TemporaryDirectory() as directory,
            ):
                root = Path(directory)
                pages = root / "docs" / layout
                pages.mkdir(parents=True)
                for version in ["0.25.11", "0.26.3", "0.26.4"]:
                    (pages / f"{version}.mdx").write_text(
                        f'---\ntitle: "{version}"\nnoindex: true\n---\n\nRelease body.\n'
                    )
                release.update_changelog_indexing(root, "0.26.3", True)
                self.assertNotIn("noindex:", (pages / "0.26.3.mdx").read_text())
                release.update_changelog_indexing(root, "0.26.4", True)
                for version in ["0.25.11", "0.26.3"]:
                    self.assertIn(
                        "noindex: true", (pages / f"{version}.mdx").read_text()
                    )
                latest = (pages / "0.26.4.mdx").read_text()
                self.assertNotIn("noindex:", latest)
                self.assertTrue(latest.endswith("\n\nRelease body.\n"))
                release.update_changelog_indexing(root, "0.26.4", True)
                self.assertEqual(latest, (pages / "0.26.4.mdx").read_text())

    def test_maintenance_registration_preserves_latest(self):
        """Registering a maintenance release leaves the current latest indexable."""
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            pages = root / "docs" / "project" / "changelog"
            pages.mkdir(parents=True)
            content = '---\ntitle: "Release"\n---\n\nBody.\n'
            for version in ["0.25.12", "0.26.3"]:
                (pages / f"{version}.mdx").write_text(content)
            args = Namespace(version="0.25.12", register_only=True, is_latest=False)
            release.handle_changelog_command(args, root)
            self.assertIn("noindex: true", (pages / "0.25.12.mdx").read_text())
            self.assertEqual(content, (pages / "0.26.3.mdx").read_text())

    def test_assembly_promotes_latest_and_archives_previous(self):
        """Fresh changelog assembly applies the same indexing policy as registration."""
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            pages = root / "docs" / "project" / "changelog"
            pages.mkdir(parents=True)
            previous = pages / "0.26.3.mdx"
            previous.write_text('---\ntitle: "Previous"\n---\n')
            with (
                patch.object(release, "load_headers_map", return_value={}),
                patch.object(
                    release, "collect_changelog_fragments", return_value=([], {}, [])
                ),
                contextlib.redirect_stdout(io.StringIO()),
            ):
                release.assemble_changelog_files(
                    root, "0.26.4", preserve_fragments=True
                )
            self.assertIn("noindex: true", previous.read_text())
            self.assertNotIn("noindex:", (pages / "0.26.4.mdx").read_text())

    def test_missing_latest_page_does_not_archive_existing_latest(self):
        """A missing new page must not remove the existing indexable release."""
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            pages = root / "docs" / "project" / "changelog"
            pages.mkdir(parents=True)
            latest = pages / "0.26.3.mdx"
            content = '---\ntitle: "Release"\n---\n'
            latest.write_text(content)
            with self.assertRaises(FileNotFoundError):
                release.update_changelog_indexing(root, "0.26.4", True)
            self.assertEqual(content, latest.read_text())


class NativeMigrationTests(unittest.TestCase):
    """Preserve the release chain and fragment lifecycle when delegating to pgrx."""

    def setUp(self):
        # pylint: disable=consider-using-with
        self.root = Path(self.enterContext(tempfile.TemporaryDirectory()))
        migration_project(self.root)
        self.sql = self.root / "pg_search/sql"
        self.fragments = self.sql / "unreleased"
        (self.sql / "pg_search--0.26.5--0.26.6.sql").write_text("SELECT 1;\n")

    def test_rcs_preserve_fragments_and_final_consumes_them(self):
        """RC builds keep fragments in tree until the final release consumes them."""
        fragment = self.fragments / "123.example.sql"
        fragment.write_text("SELECT 'new schema';\n")
        for target in ["0.27.0-rc.1", "0.27.0-rc.2"]:
            release.assemble_sql_files(self.root, target, "0.26.6", True)
            self.assertTrue(fragment.exists())
            self.assertIn(
                "new schema",
                (self.sql / f"pg_search--0.26.6--{target}.sql").read_text(),
            )
        release.assemble_sql_files(self.root, "0.27.0", "0.26.6", False)
        self.assertFalse(fragment.exists())
        self.assertTrue((self.sql / "pg_search--0.26.6--0.27.0.sql").exists())
        self.assertFalse((self.sql / "pg_search--0.27.0-rc.2--0.27.0.sql").exists())

    def test_rerun_cannot_overwrite_an_existing_migration(self):
        """Re-running assembly fails rather than clobbering an existing script."""
        fragment = self.fragments / "123.example.sql"
        fragment.write_text("SELECT 'original';\n")
        release.assemble_sql_files(self.root, "0.27.0", "0.26.6", True)
        script = self.sql / "pg_search--0.26.6--0.27.0.sql"
        original = script.read_bytes()
        fragment.write_text("SELECT 'changed';\n")
        with self.assertRaises(subprocess.CalledProcessError):
            release.assemble_sql_files(self.root, "0.27.0", "0.26.6", False)
        self.assertEqual(script.read_bytes(), original)
        self.assertTrue(fragment.exists())

    def test_dependency_order_overrides_fragment_numbers(self):
        """Explicit depends-on headers take precedence over numeric PR order."""
        (self.fragments / "100.table.sql").write_text(
            "CREATE TABLE example (id integer);\n"
        )
        (self.fragments / "99.index.sql").write_text(
            "-- depends-on: 100\nCREATE INDEX example_idx ON example (id);\n"
        )
        release.assemble_sql_files(self.root, "0.27.0", "0.26.6", False)
        script = (self.sql / "pg_search--0.26.6--0.27.0.sql").read_text()
        self.assertLess(script.index("CREATE TABLE"), script.index("CREATE INDEX"))

    def test_native_lint_enforces_fragment_and_immutability_rules(self):
        """Fragment lint delegates dependency, mixed-object, base, and upstream checks to pgrx."""
        cases = [
            (
                "undeclared_dependency",
                {
                    "100.table.sql": "CREATE TABLE example (id integer);\n",
                    "99.index.sql": "CREATE INDEX example_idx ON example (id);\n",
                },
                1,
            ),
            (
                "unresolved_depends_on",
                {
                    "100.table.sql": "-- depends-on: 999\nCREATE TABLE example (id integer);\n",
                },
                1,
            ),
            (
                "multi_prerequisite_objects",
                {
                    "100.a.sql": "CREATE TABLE a (id integer);\n",
                    "101.b.sql": "CREATE TABLE b (id integer);\n",
                    "102.both.sql": (
                        "-- depends-on: 100, 101\n"
                        "ALTER TABLE a ADD COLUMN x integer;\n"
                        "ALTER TABLE b ADD COLUMN y integer;\n"
                    ),
                },
                1,
            ),
            (
                "clean_single_prerequisite",
                {
                    "100.table.sql": "CREATE TABLE example (id integer);\n",
                    "101.index.sql": (
                        "-- depends-on: 100\nCREATE INDEX example_idx ON example (id);\n"
                    ),
                },
                0,
            ),
        ]
        for name, files, expected in cases:
            with self.subTest(case=name):
                for existing in self.fragments.glob("*.sql"):
                    existing.unlink()
                for filename, sql in files.items():
                    (self.fragments / filename).write_text(sql)
                self.assertEqual(release.lint_sql_fragments(self.root), expected)

        # Initialize a git repo in the fixture to exercise `--base` and `--upstream`.
        for existing in self.fragments.glob("*.sql"):
            existing.unlink()
        (self.fragments / "100.table.sql").write_text(
            "CREATE TABLE example (id integer);\n"
        )
        for cmd in [
            ["git", "init", "-b", "main"],
            ["git", "config", "user.email", "test@example.com"],
            ["git", "config", "user.name", "Test"],
            ["git", "config", "commit.gpgsign", "false"],
            ["git", "add", "."],
            ["git", "commit", "-m", "initial"],
        ]:
            subprocess.run(cmd, cwd=self.root, check=True, capture_output=True)
        main_sha = subprocess.run(
            ["git", "rev-parse", "HEAD"],
            cwd=self.root,
            check=True,
            capture_output=True,
            text=True,
        ).stdout.strip()

        self.assertEqual(
            release.lint_sql_fragments(self.root, main_sha, upstream_ref="main"), 0
        )

        # Modifying an already-shipped upgrade script is rejected by `--base`.
        subprocess.run(
            ["git", "checkout", "-b", "0.26.x"],
            cwd=self.root,
            check=True,
            capture_output=True,
        )
        (self.sql / "pg_search--0.26.5--0.26.6.sql").write_text("SELECT 2;\n")
        subprocess.run(
            ["git", "commit", "-am", "edit released script"],
            cwd=self.root,
            check=True,
            capture_output=True,
        )
        self.assertEqual(release.lint_sql_fragments(self.root, main_sha), 1)

        # Diverging fragment bytes relative to `--upstream` are rejected.
        subprocess.run(
            ["git", "reset", "--hard", main_sha],
            cwd=self.root,
            check=True,
            capture_output=True,
        )
        (self.fragments / "100.table.sql").write_text(
            "CREATE TABLE example (id bigint);\n"
        )
        self.assertEqual(release.lint_sql_fragments(self.root, upstream_ref="main"), 1)


if __name__ == "__main__":
    unittest.main()
