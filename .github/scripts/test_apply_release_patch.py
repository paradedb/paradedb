"""Exercise the release job's artifact boundary with real Git patches."""

import subprocess
import tempfile
import unittest
from pathlib import Path

import yaml  # pylint: disable=import-error
from apply_release_patch import apply_patch


class ReleasePatchTests(unittest.TestCase):
    """Prepared data must not replace code on the credentialed runner."""

    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()  # pylint: disable=consider-using-with
        self.addCleanup(self.directory.cleanup)
        self.repo = Path(self.directory.name) / "repo"
        self.repo.mkdir()
        self.patch = self.repo.parent / "release.patch"
        self.git("init", "-q")
        self.git("config", "user.email", "test@example.com")
        self.git("config", "user.name", "Test")
        self.git("config", "commit.gpgsign", "false")
        self.write("Cargo.toml", 'version = "0.26.2"\n')
        self.write("pg_search/sql/unreleased/123.sql", "SELECT 1;\n")
        self.git("add", ".")
        self.git("commit", "-qm", "Initial")

    def git(self, *args):
        """Run a git command inside the temporary test repository."""
        return subprocess.check_output(["git", *args], cwd=self.repo)

    def write(self, path, contents):
        """Write a UTF-8 file relative to the temporary test repository."""
        target = self.repo / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(contents, encoding="utf-8")

    def export(self):
        """Export staged changes to the patch file and reset the working tree."""
        self.git("add", "-A")
        self.patch.write_bytes(self.git("diff", "--cached", "--binary", "--no-renames"))
        self.git("reset", "--hard", "-q", "HEAD")

    def test_release_metadata_migration_and_fragment_removal_round_trip(self):
        """Metadata, migration, and fragment deletion patches apply cleanly."""
        self.write("Cargo.toml", 'version = "0.26.3"\n')
        self.write("pg_search/sql/pg_search--0.26.2--0.26.3.sql", "SELECT 1;\n")
        (self.repo / "pg_search/sql/unreleased/123.sql").unlink()
        self.export()
        apply_patch(self.repo, self.patch, "0.26.3")
        self.assertEqual((self.repo / "Cargo.toml").read_text(), 'version = "0.26.3"\n')
        self.assertTrue(
            (self.repo / "pg_search/sql/pg_search--0.26.2--0.26.3.sql").is_file()
        )
        self.assertFalse((self.repo / "pg_search/sql/unreleased/123.sql").exists())
        self.assertEqual(self.git("diff"), b"")

    def test_artifact_cannot_install_a_script_on_the_signing_runner(self):
        """Unexpected paths like scripts are rejected before touching the tree."""
        self.write(".github/scripts/evil.py", "raise SystemExit('untrusted')\n")
        self.export()
        with self.assertRaisesRegex(ValueError, "Forbidden release change"):
            apply_patch(self.repo, self.patch, "0.26.3")
        self.assertFalse((self.repo / ".github/scripts/evil.py").exists())

    def test_symlinks_and_executable_modes_are_rejected(self):
        """Symlinks and executable file modes are rejected even on allowed paths."""
        for symlink in (True, False):
            with self.subTest(symlink=symlink):
                self.git("reset", "--hard", "-q", "HEAD")
                target = self.repo / "Cargo.toml"
                if symlink:
                    target.unlink()
                    target.symlink_to(".github/scripts/evil.py")
                else:
                    target.chmod(0o755)
                self.export()
                with self.assertRaisesRegex(ValueError, "Forbidden release change"):
                    apply_patch(self.repo, self.patch, "0.26.3")
                self.assertFalse(target.is_symlink())

    def test_wrong_target_migration_is_rejected(self):
        """Migration scripts targeting a different version are rejected."""
        self.write("pg_search/sql/pg_search--0.26.2--0.26.4.sql", "SELECT 1;\n")
        self.export()
        with self.assertRaisesRegex(ValueError, "Forbidden release change"):
            apply_patch(self.repo, self.patch, "0.26.3")

    def test_empty_enterprise_patch_is_supported(self):
        """An empty patch leaves the checkout untouched."""
        self.patch.write_bytes(b"")
        apply_patch(self.repo, self.patch, "0.26.3-rc.0")
        self.assertEqual(self.git("status", "--porcelain"), b"")

    def test_beta_migration_retains_rc_suffix_and_unreleased_fragments(self):
        """Beta patches allow -rc migrations while preserving unreleased fragments."""
        migration = "pg_search/sql/pg_search--0.26.2--0.26.3-rc.0.sql"
        self.write(migration, "SELECT 1;\n")
        self.export()
        apply_patch(self.repo, self.patch, "0.26.3-rc.0")
        self.assertTrue((self.repo / migration).is_file())
        self.assertTrue((self.repo / "pg_search/sql/unreleased/123.sql").is_file())

    def test_main_sync_patch_with_version_bump_and_changelog_round_trip(self):
        """Main sync patches with version bumps, docs, and fragment cleanup apply cleanly."""
        self.write("Cargo.lock", 'name = "pg_search"\nversion = "0.26.0"\n')
        self.write("nix/pg_search.nix", 'cargoHash = "sha256-old";\n')
        self.write("docs/docs.json", '{"version": "v0.26.8"}\n')
        self.write("docs/project/changelog/0.26.8.mdx", '---\ntitle: "0.26.8"\n---\n')
        self.write("docs/project/changelog/unreleased/123.bugfix.mdx", "Fix\n")
        self.git("add", ".")
        self.git("commit", "-qm", "Add docs and lockfile")

        self.write("Cargo.toml", 'version = "0.27.0"\n')
        self.write("Cargo.lock", 'name = "pg_search"\nversion = "0.27.0"\n')
        self.write("nix/pg_search.nix", 'cargoHash = "sha256-new";\n')
        self.write("docs/docs.json", '{"version": "v0.27.0"}\n')
        self.write(
            "docs/project/changelog/0.26.8.mdx",
            '---\ntitle: "0.26.8"\nnoindex: true\n---\n',
        )
        self.write("docs/project/changelog/0.27.0.mdx", '---\ntitle: "0.27.0"\n---\n')
        self.write("pg_search/sql/pg_search--0.26.8--0.27.0.sql", "SELECT 1;\n")
        (self.repo / "pg_search/sql/unreleased/123.sql").unlink()
        (self.repo / "docs/project/changelog/unreleased/123.bugfix.mdx").unlink()
        self.export()

        apply_patch(self.repo, self.patch, "0.27.0")
        self.assertEqual((self.repo / "Cargo.toml").read_text(), 'version = "0.27.0"\n')
        self.assertEqual(
            (self.repo / "nix/pg_search.nix").read_text(), 'cargoHash = "sha256-new";\n'
        )
        self.assertTrue(
            (self.repo / "pg_search/sql/pg_search--0.26.8--0.27.0.sql").is_file()
        )
        self.assertTrue((self.repo / "docs/project/changelog/0.27.0.mdx").is_file())
        self.assertFalse(
            (self.repo / "docs/project/changelog/unreleased/123.bugfix.mdx").exists()
        )
        self.assertEqual(self.git("diff"), b"")


class ReleaseWorkflowBoundaryTests(unittest.TestCase):
    """Keep build tools and secrets on distinct GitHub-hosted jobs."""

    def test_preparation_and_notification_cannot_access_publishing_credentials(self):
        """Preparation, publishing, and notification jobs respect privilege boundaries."""
        root = Path(__file__).resolve().parents[2]
        jobs = yaml.safe_load(
            (root / ".github/workflows/publish-github-release.yml").read_text()
        )["jobs"]
        for job_name in ("prepare-release", "prepare-main-sync"):
            with self.subTest(prepare_job=job_name):
                prepare = jobs[job_name]
                self.assertEqual(prepare["permissions"], {"contents": "read"})
                self.assertNotIn("secrets.", str(prepare))
                self.assertNotIn("app-token", str(prepare))
        for job_name in ("publish-github-release", "publish-main-sync"):
            with self.subTest(publish_job=job_name):
                publisher = str(jobs[job_name])
                for ref in (
                    "determinate-nix-action",
                    "setup-rust-toolchain",
                    "manual-approval",
                    "softprops",
                    "WEBHOOK",
                ):
                    self.assertNotIn(ref, publisher)
        for job_name in ("approve-release", "publish-main-sync"):
            with self.subTest(unsigned_job=job_name):
                self.assertNotIn("SIGNING_KEY", str(jobs[job_name]))
        self.assertEqual(jobs["notify-failure"]["permissions"], {})
        self.assertEqual(
            set(jobs["notify-failure"]["needs"]),
            {
                "prepare-release",
                "approve-release",
                "publish-github-release",
                "prepare-main-sync",
                "publish-main-sync",
            },
        )


if __name__ == "__main__":
    unittest.main()
