"""Exercise the release job's artifact boundary with real Git patches."""

import subprocess
import tempfile
import unittest
from pathlib import Path

import yaml
from apply_release_patch import apply_patch


class ReleasePatchTests(unittest.TestCase):
    """Prepared data must not replace code on the credentialed runner."""

    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
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
        return subprocess.check_output(["git", *args], cwd=self.repo)

    def write(self, path, contents):
        target = self.repo / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(contents, encoding="utf-8")

    def export(self):
        self.git("add", "-A")
        self.patch.write_bytes(self.git("diff", "--cached", "--binary", "--no-renames"))
        self.git("reset", "--hard", "-q", "HEAD")

    def test_release_metadata_migration_and_fragment_removal_round_trip(self):
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
        self.write(".github/scripts/evil.py", "raise SystemExit('untrusted')\n")
        self.export()
        with self.assertRaisesRegex(ValueError, "Forbidden release change"):
            apply_patch(self.repo, self.patch, "0.26.3")
        self.assertFalse((self.repo / ".github/scripts/evil.py").exists())

    def test_symlinks_and_executable_modes_are_rejected(self):
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
        self.write("pg_search/sql/pg_search--0.26.2--0.26.4.sql", "SELECT 1;\n")
        self.export()
        with self.assertRaisesRegex(ValueError, "Forbidden release change"):
            apply_patch(self.repo, self.patch, "0.26.3")

    def test_empty_enterprise_patch_is_supported(self):
        self.patch.write_bytes(b"")
        apply_patch(self.repo, self.patch, "0.26.3-rc.0")
        self.assertEqual(self.git("status", "--porcelain"), b"")

    def test_beta_migration_retains_rc_suffix_and_unreleased_fragments(self):
        migration = "pg_search/sql/pg_search--0.26.2--0.26.3-rc.0.sql"
        self.write(migration, "SELECT 1;\n")
        self.export()
        apply_patch(self.repo, self.patch, "0.26.3-rc.0")
        self.assertTrue((self.repo / migration).is_file())
        self.assertTrue((self.repo / "pg_search/sql/unreleased/123.sql").is_file())


class ReleaseWorkflowBoundaryTests(unittest.TestCase):
    """Keep build tools and secrets on distinct GitHub-hosted jobs."""

    def test_preparation_and_notification_cannot_access_publishing_credentials(self):
        root = Path(__file__).resolve().parents[2]
        jobs = yaml.safe_load(
            (root / ".github/workflows/publish-github-release.yml").read_text()
        )["jobs"]
        prepare = jobs["prepare-release"]
        self.assertEqual(prepare["permissions"], {"contents": "read"})
        self.assertNotIn("secrets.", str(prepare))
        self.assertNotIn("app-token", str(prepare))
        publisher = str(jobs["publish-github-release"])
        for ref in (
            "determinate-nix-action",
            "setup-rust-toolchain",
            "manual-approval",
            "softprops",
            "WEBHOOK",
        ):
            self.assertNotIn(ref, publisher)
        self.assertNotIn("SIGNING_KEY", str(jobs["approve-release"]))
        self.assertEqual(jobs["notify-failure"]["permissions"], {})


if __name__ == "__main__":
    unittest.main()
