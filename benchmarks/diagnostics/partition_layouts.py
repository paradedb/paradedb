"""Change only partition_by, timing byte-identical existing Stack Overflow SQL.

Correctness companions add stable ordering separately from the timed SQL. They
do not establish that every original LIMIT/tie execution chose the same rows.
"""

import csv
from decimal import Decimal
import hashlib
import io
import json
import os
from pathlib import Path
import re
import shutil
import statistics
import subprocess
import sys

import partition_pruning as paired

BASELINE = 'c3479d9a1fd78fd85676524a7ead9820513ad67a'
# Keep every original join key. Each treatment changes exactly one index option.
LAYOUTS = {
    'original': None,
    'posts_date': ('stackoverflow_posts_idx', 'id,owner_user_id', 'id,owner_user_id,creation_date'),
    'posts_type': ('stackoverflow_posts_idx', 'id,owner_user_id', 'id,owner_user_id,post_type_id'),
    'users_reputation': ('users_idx', 'id', 'id,reputation'),
    'comments_score': ('comments_idx', 'post_id', 'post_id,score'),
    'comments_name': ('comments_idx', 'post_id', 'post_id,user_display_name'),
}


def cases():
    # Value: unique ordering key for the separately labeled correctness companion.
    result = {name + '.sql': 'id' for name in (
        'filtered_highcard', 'filtered_lowcard', 'paging_string_min',
        'paging_string_median', 'paging_string_max', 'top_k_numeric_highcard',
        'top_k_numeric_lowcard')}
    for group, key in (
        ('join_permissioned_search', 'p.id'),
        ('join_foreign_filter_local_sort', 'p.id'),
        ('join_distinct_parent_sort', 'u.id'),
        ('join_semi_filter', 'p.id'),
        ('join_aggregate_count', None),
    ):
        for variant in ('hash_partitioned', 'range_partitioned'):
            result[f'{group}/{variant}.sql'] = key
    result['count_filter/aggregate_scan.sql'] = None
    result['bucket_numeric_filter/aggregate_scan.sql'] = None
    return result


def companion(parts, key):
    """Never called when writing timed fixtures; only for separate validation."""
    query = parts[-1]
    if key is not None:
        match = re.search(r'\s+LIMIT\s+\d+\s*$', query, re.I)
        assert match, query
        prefix, limit = query[:match.start()], query[match.start():]
        prefix += (', ' if re.search(r'\bORDER\s+BY\b', prefix, re.I) else ' ORDER BY ') + key + ' ASC'
        query = prefix + limit
    return [*parts[:-1], query]


def layout_ddl(original, change):
    if change is None:
        return original
    index, before, after = change
    pattern = rf'(CREATE INDEX {index}\b.*?partition_by\s*=\s*\x27){re.escape(before)}(\x27)'
    ddl, count = re.subn(pattern, lambda m: m[1] + after + m[2], original, flags=re.S)
    assert count == 1, (index, count)
    return ddl


def load_queries():
    selected, raw, keys, manifest = {}, {}, {}, {}
    for relative, key in cases().items():
        path = paired.DATASET / 'queries' / relative
        content = path.read_bytes()
        pinned = paired.run(['git', 'show', f'{BASELINE}:{path.relative_to(paired.ROOT)}'], capture=True)
        assert content == pinned.encode(), f'Original query changed: {path}'
        name = relative.removesuffix('.sql').replace('/', '__')
        parts = paired.statements(content.decode())
        assert all(p.upper().startswith('SET ') for p in parts[:-1])
        assert parts[-1].upper().startswith('SELECT')
        selected[name], raw[name], keys[name] = parts, content, key
        manifest[name] = {'source': relative, 'sha256': hashlib.sha256(content).hexdigest(),
                          'timed_sql': content.decode(), 'correctness_companion': companion(parts, key)}
    return selected, raw, keys, manifest


def canonical_rows(parts):
    setup = ';\n'.join(parts[:-1])
    # JSON retains NULL versus empty string and preserves the projected values.
    query = (setup + ';\n' if setup else '') + 'SELECT COALESCE(json_agg(q),\'[]\'::json) FROM (' + parts[-1] + ') q'
    rows = json.loads(paired.psql(query), parse_float=Decimal)
    return query, sorted(json.dumps(r, sort_keys=True, default=lambda value: {'decimal': str(value)}) for r in rows)


def evidence(label, selected, keys):
    validated, modes = {}, {}
    for name, parts in selected.items():
        setup = ';\n'.join(parts[:-1])
        setup = setup + ';\n' if setup else ''
        observed = paired.psql(setup + parts[-1], csv_output=True)
        (paired.OUT / f'{label}-{name}-original-results.csv').write_text(observed)
        plan = paired.psql(setup + 'SET track_io_timing=on; EXPLAIN (ANALYZE, VERBOSE, BUFFERS, SETTINGS) ' + parts[-1])
        (paired.OUT / f'{label}-{name}-plan.txt').write_text(plan)
        modes[name] = {
            'paradedb': 'ParadeDB' in plan,
            'distributed': 'DistributedExec' in plan,
            'workers_launched': bool(re.search(r'MPP Launch: workers=[1-9]', plan)),
            'range_assignment': bool(re.search(r'partition=\w+\[', plan)),
        }
        sql, rows = canonical_rows(companion(parts, keys[name]))
        (paired.OUT / f'{label}-{name}-companion.sql').write_text(sql + ';\n')
        wrapper = sql[len(setup):]
        companion_plan = paired.psql(setup + 'EXPLAIN (ANALYZE, VERBOSE, BUFFERS, SETTINGS) ' + wrapper)
        (paired.OUT / f'{label}-{name}-companion-plan.txt').write_text(companion_plan)
        validated[name] = rows
        # Different tied/unordered rows are valid. Counts must still agree.
        actual_rows = list(csv.reader(io.StringIO(observed)))
        assert actual_rows and len(actual_rows) - 1 == len(rows), name
    paired.save(f'{label}-companions.json', validated)
    paired.save(f'{label}-execution-modes.json', modes)
    return validated


def trace(layout, selected):
    """Trace the original SQL, separately; record paths that emit no events too."""
    paired.switch('trace')
    log = Path('/home/runner/.pgrx/18.log')
    summary = {}
    for name, parts in selected.items():
        offset = log.stat().st_size
        paired.psql(';\n'.join(parts))
        with log.open('rb') as stream:
            stream.seek(offset)
            events = stream.read().decode(errors='replace')
        (paired.OUT / f'{layout}-{name}-trace.log').write_text(events)
        decisions = re.findall(r'PDB_PRUNING_TRACE pid=(\d+) segment=(\S+) num_docs=(\d+) eligible=(true|false) candidate=(true|false)', events)
        scorers = re.findall(r'PDB_SCORER_TRACE pid=(\d+) segment=(\S+)', events)
        summary[name] = {
            'decision_calls': len(decisions), 'scorer_calls': len(scorers),
            'eligible_calls': sum(d[3] == 'true' for d in decisions),
            'rejected_nonempty_segments': sorted({s for _, s, n, _, c in decisions if c == 'false' and int(n) > 0}),
            'scorer_process_segments': sorted(set(scorers)),
            'scope': 'original SQL on trace build; is_candidate and DeferredScorer only; absence of events is not proof of no pruning',
        }
    paired.save(f'{layout}-trace-summary.json', summary)


PROFILE_CASES = (
    ('join_foreign_filter_local_sort__range_partitioned', 'users_reputation'),
    ('join_aggregate_count__range_partitioned', 'users_reputation'),
    ('filtered_lowcard', 'posts_type'),
    ('join_semi_filter__range_partitioned', 'comments_score'),
)


def profile_cases(layout, version, selected):
    """Capture DataFusion plans and system-wide perf counters for selected cases."""
    wanted = list(PROFILE_CASES) if layout == 'bm25_layout_original' else [
        (query, arm) for query, arm in PROFILE_CASES if arm in layout
    ]
    if not wanted:
        return
    for query, _arm in wanted:
        parts = selected[query]
        setup = ';\n'.join(parts[:-1])
        setup = setup + ';\n' if setup else ''
        sql = setup + parts[-1]
        stem = f'{layout}-{version}-{query}'
        plan = paired.psql(
            setup + 'SET track_io_timing=on; EXPLAIN (ANALYZE, VERBOSE, BUFFERS, SETTINGS) ' + parts[-1]
        )
        (paired.OUT / f'{stem}-operator-plan.txt').write_text(plan)
        command = [
            'sudo', 'perf', 'stat', '-a', '-x,',
            '-e', 'cycles,instructions,branches,branch-misses,cache-misses',
            '-o', str(paired.OUT / f'{stem}-perf.csv'), '--',
            'sh', '-c',
            'for i in $(seq 1 15); do psql "$1" -Xq -v ON_ERROR_STOP=1 -c "$2" >/dev/null; done',
            'profile', paired.URL, sql,
        ]
        result = subprocess.run(command, cwd=paired.ROOT, text=True, capture_output=True)
        (paired.OUT / f'{stem}-perf.stderr').write_text(result.stderr)
        if result.returncode != 0:
            raise RuntimeError(f'perf failed for {stem}: {result.stderr}')


def layout_summary(samples):
    """Compare layouts within each binary, separately from main-vs-PR effects."""
    baseline = samples['bm25_layout_original']
    rows = []
    for layout, builds in samples.items():
        for build, queries in builds.items():
            for name, values in queries.items():
                original = baseline[build][name]
                rows.append({'layout': layout, 'build': build, 'query': name,
                             'mean_ms': statistics.mean(values), 'median_ms': statistics.median(values),
                             'mean_change_from_original_pct': 100 * (statistics.mean(values) / statistics.mean(original) - 1),
                             'median_change_from_original_pct': 100 * (statistics.median(values) / statistics.median(original) - 1)})
    paired.save('layout-comparison.json', rows)


def main():
    size = sys.argv[1]
    assert size == '1m', 'Review the first pass before expanding to 20M'
    assert os.environ.get('GITHUB_ACTIONS') == 'true', 'Isolated CI cluster only'
    paired.OUT.mkdir(exist_ok=True)
    selected, raw, keys, manifest = load_queries()
    paired.save('query-manifest.json', manifest)
    original = (paired.DATASET / 'indexes/bm25.sql').read_text()
    assert original == paired.run(['git', 'show', f'{BASELINE}:benchmarks/datasets/stackoverflow/indexes/bm25.sql'], capture=True)
    paired.psql('VACUUM FULL ANALYZE')
    paired.psql('VACUUM ANALYZE')
    (paired.OUT / 'settings.csv').write_text(paired.psql('SELECT name, setting, unit FROM pg_settings ORDER BY name', csv_output=True))
    paired.save('selectivity.json', json.loads(paired.psql("""SELECT json_build_object(
        'posts', (SELECT json_build_object('rows', count(*), 'date_filter_rows', count(*) FILTER (WHERE creation_date >= '2012-01-01T00:00:00Z'), 'type_filter_rows', count(*) FILTER (WHERE post_type_id < 3)) FROM stackoverflow_posts),
        'users', (SELECT json_build_object('rows', count(*), 'reputation_filter_rows', count(*) FILTER (WHERE reputation > 100)) FROM users),
        'comments', (SELECT json_build_object('rows', count(*), 'score_filter_rows', count(*) FILTER (WHERE score > 0)) FROM comments))""")))
    samples, expected, validation_failures = {}, None, []
    try:
        for arm, change in LAYOUTS.items():
            layout = 'bm25_layout_' + arm
            directory = paired.DATASET / 'queries' / layout
            fixture = paired.DATASET / 'indexes' / (layout + '.sql')
            assert not directory.exists() and not fixture.exists()
            directory.mkdir()
            try:
                for name, content in raw.items():
                    (directory / (name + '.sql')).write_bytes(content)
                ddl = layout_ddl(original, change)
                fixture.write_text(ddl)
                (paired.OUT / (layout + '-index.sql')).write_text(ddl)
                names = re.findall(r'CREATE INDEX ([a-z_0-9]+)', ddl)
                assert len(names) == len(set(names))
                paired.switch('main')
                paired.psql(';'.join('DROP INDEX IF EXISTS ' + n for n in names))
                inventory = None
                samples[layout] = {'main': {}, 'pr': {}}
                for number, version in enumerate(paired.ROUNDS, 1):
                    paired.switch(version)
                    label = f'{layout}-r{number}-{version}'
                    for name, content in raw.items():
                        assert (directory / (name + '.sql')).read_bytes() == content
                    if inventory is not None:
                        assert paired.identity() == inventory, 'Index changed before measurement'
                    values = paired.benchmark(size, layout, label, selected, initialize=number == 1)
                    if inventory is None:
                        inventory = paired.identity()
                        paired.save(layout + '-index-identity.json', inventory)
                    assert paired.identity() == inventory, 'Index changed during measurement'
                    results = evidence(label, selected, keys)
                    if os.environ.get('PROFILE_PRUNING') == 'true':
                        profile_cases(layout, version, selected)
                    if expected is None:
                        expected = results
                    mismatches = [name for name in expected if expected[name] != results[name]]
                    paired.save(label + '-validation.json', {'companion_mismatches': mismatches})
                    if mismatches:
                        validation_failures.append({'round': label, 'queries': mismatches})
                        paired.save('validation-failures.json', validation_failures)
                        print(f'CORRECTNESS DIFFERENCE: {label}: {mismatches}', flush=True)
                    for name, item in values.items():
                        samples[layout][version].setdefault(name, []).extend(item['samples_ms'])
                    paired.save('progress.json', {'last_round': label, 'companion_results_match': not validation_failures})
                paired.write_summary(samples)
                layout_summary(samples)
                trace(layout, selected)
                assert paired.identity() == inventory, 'Trace changed index inventory'
            finally:
                shutil.rmtree(directory)
                fixture.unlink(missing_ok=True)
        text = paired.write_summary(samples)
        with Path(os.environ['GITHUB_STEP_SUMMARY']).open('a') as stream:
            stream.write(text)
        paired.save('completion.json', {'complete': True, 'size': size, 'unchanged_queries': len(selected),
                    'layouts': len(LAYOUTS), 'companion_results_match': not validation_failures,
                    'limits': 'Tie/unordered original outputs saved for audit; equality check uses separate deterministic companions. Six layouts built once each; no write lifecycle or segment-count sweep.'})
        assert not validation_failures, 'Correctness companions differ; see validation-failures.json before accepting any recommendations'
    finally:
        paired.run(['cargo', 'pgrx', 'stop', 'pg18'], cwd=paired.ROOT / 'pg_search')


if __name__ == '__main__':
    main()
