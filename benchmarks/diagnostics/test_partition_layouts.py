"""Offline checks for experiment isolation and comparison semantics."""

import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import partition_layouts as layouts


class ExperimentTests(unittest.TestCase):
    def test_timed_sql_is_the_pinned_original(self):
        selected, raw, keys, manifest = layouts.load_queries()
        self.assertEqual(len(selected), 19)
        for name, original in raw.items():
            self.assertEqual(original.decode(), manifest[name]['timed_sql'])
            # Generating validation SQL must not change the measured definition.
            before = selected[name].copy()
            companion = layouts.companion(selected[name], keys[name])
            self.assertEqual(selected[name], before)
            if keys[name]:
                self.assertNotEqual(companion[-1], before[-1])
                self.assertIn(keys[name] + ' ASC', companion[-1])

    def test_treatments_change_exactly_one_index_option(self):
        original = (layouts.paired.DATASET / 'indexes/bm25.sql').read_text()
        for change in layouts.LAYOUTS.values():
            result = layouts.layout_ddl(original, change)
            if change is None:
                self.assertEqual(result, original)
            else:
                _, before, after = change
                self.assertNotEqual(result, original)
                self.assertEqual(result.replace("partition_by = '" + after + "'", "partition_by = '" + before + "'"), original)
        with self.assertRaises(AssertionError):
            layouts.layout_ddl(original, ('missing_index', 'id', 'id,reputation'))

    def test_companions_handle_unordered_and_tied_limits(self):
        cases = [
            (['SELECT * FROM posts LIMIT 10'], 'SELECT * FROM posts ORDER BY id ASC LIMIT 10'),
            (['SET work_mem=\'4GB\'', 'SELECT * FROM posts ORDER BY creation_date LIMIT 10'],
             'SELECT * FROM posts ORDER BY creation_date, id ASC LIMIT 10'),
        ]
        for parts, expected in cases:
            self.assertEqual(layouts.companion(parts, 'id')[-1], expected)
        with self.assertRaises(AssertionError):
            layouts.companion(['SELECT * FROM posts'], 'id')

    def test_layout_effect_is_not_confused_with_build_effect(self):
        samples = {
            'bm25_layout_original': {'main': {'q': [10, 10]}, 'pr': {'q': [20, 20]}},
            'treatment': {'main': {'q': [5, 5]}, 'pr': {'q': [10, 10]}},
        }
        with patch.object(layouts.paired, 'save') as save:
            layouts.layout_summary(samples)
            rows = save.call_args.args[1]
        treatment = [r for r in rows if r['layout'] == 'treatment']
        self.assertEqual([r['mean_change_from_original_pct'] for r in treatment], [-50, -50])

    def test_csv_row_count_handles_newlines_in_text(self):
        # Evidence uses csv.reader, not line count: post bodies can contain newlines.
        with tempfile.TemporaryDirectory() as tmp, \
             patch.object(layouts.paired, 'OUT', Path(tmp)), \
             patch.object(layouts.paired, 'psql', side_effect=[
                 'id,body\n1,"first\nsecond"\n', 'Custom Scan (ParadeDB Base Scan)',
                 '[{"id":1,"body":"first\\nsecond"}]', 'Aggregate over companion scan',
             ]):
            results = layouts.evidence('test', {'q': ['SELECT id,body FROM posts LIMIT 1']}, {'q': 'id'})
        self.assertEqual(len(results['q']), 1)

    def test_numeric_validation_does_not_round_to_float(self):
        with patch.object(layouts.paired, 'psql', return_value='[{"n":0.1234567890123456789012345}]'):
            sql, rows = layouts.canonical_rows(['SELECT n FROM posts'])
        self.assertIn('0.1234567890123456789012345', rows[0])
        self.assertIn('json_agg', sql)


if __name__ == '__main__':
    unittest.main()
