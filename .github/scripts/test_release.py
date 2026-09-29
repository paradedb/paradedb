"""Regression tests for release documentation updates."""

import importlib.util
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).with_name('release.py')
SPEC = importlib.util.spec_from_file_location('release', SCRIPT)
RELEASE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(RELEASE)
DOC = Path(__file__).resolve().parents[2] / 'docs/operate/deploy/upgrading.mdx'


class UpgradeDocumentationTests(unittest.TestCase):
    def test_latest_release_updates_all_upgrade_examples(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            target = root / 'docs/operate/deploy/upgrading.mdx'
            target.parent.mkdir(parents=True)
            original = DOC.read_text()
            self.assertNotIn('{version}', original)
            target.write_text(original + '\nHistorical version: 0.24.2\n')
            RELEASE.update_version_snippet(root, '0.26.0')
            updated = target.read_text()
            self.assertIn('`pg_search` is `0.26.0`', updated)
            self.assertIn('docker pull paradedb/paradedb:0.26.0', updated)
            self.assertIn('Docker image should be `0.26.0`', updated)
            self.assertIn("ALTER EXTENSION pg_search UPDATE TO '0.26.0';", updated)
            self.assertIn('Historical version: 0.24.2', updated)
            RELEASE.update_version_snippet(root, '0.26.0')
            self.assertEqual(updated, target.read_text())

    def test_register_only_respects_latest_release_flag(self):
        # This is the command used to sync stable-branch releases to main.
        for flag, should_update in [('--is-latest', True), ('--no-is-latest', False)]:
            with self.subTest(flag=flag), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                script = root / '.github/scripts/release.py'
                script.parent.mkdir(parents=True)
                script.write_text(SCRIPT.read_text())
                target = root / 'docs/operate/deploy/upgrading.mdx'
                target.parent.mkdir(parents=True)
                original = DOC.read_text()
                target.write_text(original)
                subprocess.run(
                    [sys.executable, str(script), 'changelog', '0.26.0', '--register-only', flag],
                    check=True, capture_output=True, text=True,
                )
                if should_update:
                    self.assertIn('docker pull paradedb/paradedb:0.26.0', target.read_text())
                else:
                    self.assertEqual(original, target.read_text())


if __name__ == '__main__':
    unittest.main()
