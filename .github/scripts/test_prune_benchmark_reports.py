import importlib.util
import json
import tempfile
import unittest
from pathlib import Path

spec = importlib.util.spec_from_file_location('prune', Path(__file__).with_name('prune-benchmark-reports.py'))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class RetentionTests(unittest.TestCase):
    def test_age_budget_short_sha_and_history_preservation(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            history = root / 'benchmarker' / 'data.js'
            history.parent.mkdir()
            now = 40 * 86400
            runs = [dict(commit=dict(id=sha), date=date * 1000) for sha, date in
                    [('aaa111', now), ('bbb222', now - 1), ('ccc333', 0)]]
            history.write_text('window.BENCHMARK_DATA = ' + json.dumps(dict(entries=dict(test=runs))))
            original = history.read_bytes()
            for name in ['aaa', 'bbb222', 'ccc333', 'unknown']:
                directory = root / 'benchmarker-runs' / name
                directory.mkdir(parents=True)
                (directory / 'report.html').write_bytes(b'x' * 8)
            module.prune(root, now=now, max_bytes=10)
            self.assertEqual([p.name for p in (root / 'benchmarker-runs').iterdir()], ['aaa'])
            self.assertEqual(history.read_bytes(), original)
            module.prune(root, now=now, max_bytes=10)
            self.assertTrue((root / 'benchmarker-runs' / 'aaa').exists())

    def test_missing_or_invalid_history_does_not_delete_reports(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            report = root / 'benchmarker-runs' / 'abc'
            report.mkdir(parents=True)
            with self.assertRaises(FileNotFoundError):
                module.prune(root)
            self.assertTrue(report.exists())


if __name__ == '__main__':
    unittest.main()
