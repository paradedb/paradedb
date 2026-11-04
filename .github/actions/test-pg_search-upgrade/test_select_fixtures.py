import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("selector", Path(__file__).with_name("select_fixtures.py"))
selector = importlib.util.module_from_spec(spec)
spec.loader.exec_module(selector)


class FixtureSelectionTests(unittest.TestCase):
    def test_release_boundaries(self):
        root = Path(__file__).parent
        old = selector.select(root, selector.version("v0.25.0"))
        new = selector.select(root, selector.version("0.26.0"))
        self.assertIn("tokenizer_compatibility", old)
        self.assertNotIn("tokenizer_recent_options", old)
        self.assertIn("tokenizer_recent_options", new)
        self.assertIn("regex_tokenizers", old)
        self.assertNotIn("regex_tokenizers", new)
        earlier = selector.select(root, selector.version("0.24.0"))
        self.assertNotIn("tokenizer_compatibility", earlier)
        self.assertNotIn("vector_storage", earlier)
        self.assertIn("regex_tokenizers", earlier)
        self.assertIn("unicode_words_tokenizers", earlier)
        self.assertNotIn("unicode_words_tokenizers", old)

    def test_invalid_interval_is_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "case").mkdir()
            (root / "case" / "setup.sql").touch()
            (root / "fixture-versions.json").write_text(json.dumps({
                "case": {"min_version": "0.26.0", "max_version_exclusive": "0.25.0"}
            }))
            with self.assertRaises(ValueError):
                selector.select(root, (0, 26, 0))

    def test_new_fixture_requires_metadata(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "new_case").mkdir()
            (root / "new_case" / "setup.sql").touch()
            (root / "fixture-versions.json").write_text(json.dumps({}))
            with self.assertRaises(ValueError):
                selector.select(root, (0, 26, 0))

    def test_rejects_ambiguous_versions(self):
        for value in ["latest", "0.26", "v0.26.0-rc.1"]:
            with self.assertRaises(ValueError):
                selector.version(value)


if __name__ == "__main__":
    unittest.main()
