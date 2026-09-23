"""Same-index ABBA measurements using the repository's ordinary benchmark runner."""

import csv
import getpass
import hashlib
import io
import json
import math
import os
from pathlib import Path
import re
import shutil
import statistics
import subprocess
import sys
from urllib.parse import quote

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'diagnostic-output'
DATASET = ROOT / 'benchmarks/datasets/stackoverflow'
URL = f"postgresql://{quote(getpass.getuser(), safe='')}@localhost:28818/postgres"
LAYOUTS = ('bm25', 'bm25_pruning', 'bm25_pruning_date')
ROUNDS = ('main', 'pr', 'pr', 'main')
RUNS = 30


def run(args, *, cwd=ROOT, capture=False):
    if capture:
        return subprocess.check_output(args, cwd=cwd, text=True)
    subprocess.run(args, cwd=cwd, check=True)


def psql(sql, *, csv_output=False):
    mode = '--csv' if csv_output else '-At'
    return run(['psql', URL, '-Xq', mode, '-v', 'ON_ERROR_STOP=1', '-c', sql], capture=True)


def save(name, obj):
    (OUT / name).write_text(json.dumps(obj, indent=2) + '\n')


def identity():
    indexes = ('stackoverflow_posts_idx', 'users_idx', 'comments_idx', 'badges_idx')
    return {name: {
        'relation': psql(f"SELECT oid, relfilenode, reloptions FROM pg_class WHERE oid='{name}'::regclass"),
        'segments': psql(f"SELECT segno, mutable, num_docs, num_deleted FROM paradedb.index_info('{name}') ORDER BY segno")
    } for name in indexes}


def switch(version):
    run(['cargo', 'pgrx', 'stop', 'pg18'], cwd=ROOT / 'pg_search')
    destination = run(['/usr/lib/postgresql/18/bin/pg_config', '--pkglibdir'], capture=True).strip()
    run(['sudo', 'install', '-m', '755', str(OUT / f'{version}.so'), destination + '/pg_search.so'])
    run(['cargo', 'pgrx', 'start', 'pg18'], cwd=ROOT / 'pg_search')


def date_bounds():
    bounds = json.loads(psql("""SELECT json_build_object(
        'date_min', min(creation_date), 'date_max', max(creation_date),
        'date_50', percentile_disc(0.50) WITHIN GROUP (ORDER BY creation_date),
        'date_51', percentile_disc(0.51) WITHIN GROUP (ORDER BY creation_date),
        'date_60', percentile_disc(0.60) WITHIN GROUP (ORDER BY creation_date),
        'date_90', percentile_disc(0.90) WITHIN GROUP (ORDER BY creation_date),
        'rows', count(*), 'null_dates', count(*) FILTER (WHERE creation_date IS NULL))
        FROM stackoverflow_posts"""))
    assert bounds['date_50'] < bounds['date_51'] < bounds['date_60']
    save('date-distribution.json', bounds)
    (OUT / 'date-histogram.csv').write_text(psql(
        "SELECT extract(year FROM creation_date) AS year, count(*) FROM stackoverflow_posts GROUP BY 1 ORDER BY 1",
        csv_output=True))
    return bounds


def statements(sql):
    # The selected fixture SQL has no semicolons or -- inside string literals.
    return [s.strip() for s in re.sub(r'--[^\n]*', '', sql).split(';') if s.strip()]


def selected_queries(bounds):
    result = {}
    for path in sorted((DATASET / 'queries').glob('pruning_*.sql')):
        sql = path.read_text()
        for key, value in bounds.items():
            sql = sql.replace('{{ ' + key + ' }}', str(value).replace("'", "''"))
        assert '{{' not in sql
        result[path.stem] = ['SET work_mem TO \'4GB\'', 'SET timezone TO \'UTC\'', *statements(sql)]
    for group in ('join_semi_filter', 'join_aggregate_count', 'join_distinct_parent_sort'):
        for variant in ('hash_partitioned', 'range_partitioned'):
            parts = statements((DATASET / 'queries' / group / (variant + '.sql')).read_text())
            # Explicitly override the GUC in both arms: defaults change over time.
            parts.insert(-1, 'SET paradedb.enable_range_partitioned_join TO ' + ('on' if variant == 'range_partitioned' else 'off'))
            if group == 'join_semi_filter':
                parts[-1] = parts[-1].replace('p.title ASC', 'p.title ASC, p.id ASC')
            if group == 'join_distinct_parent_sort':
                parts[-1] = parts[-1].replace('u.display_name ASC', 'u.display_name ASC, u.id ASC')
            result[group + '-' + variant] = parts
    assert len(result) == 11
    for parts in result.values():
        assert all(s.upper().startswith('SET ') for s in parts[:-1])
        assert parts[-1].upper().startswith('SELECT')
    save('query-definitions.json', result)
    return result


def benchmark(size, layout, label, selected, *, initialize):
    args = [str(ROOT / 'target/release/benchmarks'), 'benchmark', '--url', URL,
            '--dataset', 'stackoverflow', '--index', layout, '--size', size,
            '--runs', str(RUNS), '--output', 'json', '--fail-on-error', 'true', '--vacuum', 'false']
    if not initialize:
        args.append('--skip-index')
    path = OUT / f'{label}-benchmark.log'
    try:
        with path.open('w') as stream:
            subprocess.run(args, cwd=ROOT / 'benchmarks', stdout=stream, stderr=subprocess.STDOUT, check=True)
    except subprocess.CalledProcessError:
        print(f'Benchmark failed: {label}; final log output:\n{path.read_text()[-12000:]}', flush=True)
        raise
    shutil.copyfile(ROOT / 'benchmarks/results.json', OUT / f'{label}-runner-results.json')
    log = path.read_text()
    observed = {}
    pattern = r'Query Type: ([^\n]+).*?Results: \[cold: ([^\]]*)\] \[([^\]]*)\] \| Rows Returned: (\d+)'
    for name, cold, samples, rows in re.findall(pattern, log, re.S):
        assert name not in observed
        values = json.loads('[' + samples + ']')
        assert len(values) == RUNS and all(math.isfinite(v) and v >= 0 for v in values)
        observed[name] = {'samples_ms': values, 'cold_ms': float(cold), 'rows': int(rows)}
        print(json.dumps({'round': label, 'query': name, **observed[name],
                          'median_ms': statistics.median(values), 'mean_ms': statistics.mean(values)}), flush=True)
    assert set(observed) == set(selected), f'Missing query measurements: {set(selected) - set(observed)}'
    assert 'EXPLAIN failed:' not in log
    save(f'{label}-samples.json', observed)
    return observed


def evidence(label, selected):
    results = {}
    plan_modes = {}
    for name, parts in selected.items():
        setup = ';\n'.join(parts[:-1]) + ';\n'
        query = parts[-1]
        output = psql(setup + query, csv_output=True)
        rows = list(csv.reader(io.StringIO(output)))
        assert rows, f'No result header: {name}'
        results[name] = rows
        plan = psql(setup + 'SET track_io_timing=on; EXPLAIN (ANALYZE, VERBOSE, BUFFERS, SETTINGS, SUMMARY ON) ' + query)
        (OUT / f'{label}-{name}-plan.txt').write_text(plan)
        assert 'ParadeDB' in plan, f'No ParadeDB execution path: {name}'
        if name.startswith('pruning_') and name != 'pruning_no_date_control':
            assert 'creation_date' in plan
        # Preserve actual modes. A filename alone does not prove a range assignment or MPP.
        plan_modes[name] = {
            'distributed': 'DistributedExec' in plan,
            'workers_launched': bool(re.search(r'MPP Launch: workers=[1-9]', plan)),
            'range_assignment': bool(re.search(r'partition=(?:owner_user_id|id)\[', plan)),
            'hash_partitioning': 'Hash([' in plan or 'mode=Partitioned' in plan,
        }
    # A count query returns one row even for zero matches: inspect the actual count.
    assert int(results['pruning_date_count'][1][0]) > 0, 'Narrow query has no matching rows'
    save(f'{label}-results.json', results)
    save(f'{label}-execution-modes.json', plan_modes)
    return results


def trace(layout, selected):
    switch('trace')
    server_log = Path('/home/runner/.pgrx/18.log')
    summaries = {}
    for name in ('pruning_date_count', 'pruning_date_control', 'pruning_no_date_control'):
        # Use a fully consumed row scan for direct scorer evidence, separate from timing.
        # Disable PG parallelism in this diagnostic only to simplify pid attribution.
        query = selected[name][-1].replace('SELECT count(*)', 'SELECT id')
        query += ' ORDER BY id'
        settings = "SET max_parallel_workers_per_gather=0; SET paradedb.enable_aggregate_custom_scan=off; SET timezone='UTC';"
        offset = server_log.stat().st_size
        output = psql(settings + query)
        with server_log.open('rb') as stream:
            stream.seek(offset)
            events = stream.read().decode(errors='replace')
        (OUT / f'{layout}-{name}-trace.log').write_text(events)
        decisions = re.findall(r'PDB_PRUNING_TRACE pid=(\d+) segment=(\S+) num_docs=(\d+) eligible=(true|false) candidate=(true|false)', events)
        scorers = re.findall(r'PDB_SCORER_TRACE pid=(\d+) segment=(\S+)', events)
        rejected = sorted({segment for _, segment, docs, _, candidate in decisions if candidate == 'false' and int(docs) > 0})
        accepted = sorted({segment for _, segment, _, _, candidate in decisions if candidate == 'true'})
        opened = sorted({segment for _, segment in scorers})
        assert decisions, 'Diagnostic did not reach candidate checks'
        assert scorers, 'Diagnostic did not reach DeferredScorer creation'
        assert not set(rejected) & set(opened), 'A rejected segment reached DeferredScorer'
        summaries[name] = {'rejected_segments': rejected, 'accepted_segments': accepted,
                           'deferred_scorer_segments': opened, 'decision_calls': len(decisions),
                           'empty_segments': sorted({seg for _, seg, docs, _, _ in decisions if int(docs) == 0}),
                           'scorer_calls': len(scorers), 'eligible_calls': sum(d[3] == 'true' for d in decisions),
                           'result_sha256': hashlib.sha256(output.encode()).hexdigest(),
                           'scope': 'separate serial projection; DeferredScorer only; not timed'}
    save(f'{layout}-trace-summary.json', summaries)
    print(json.dumps({'layout': layout, 'traces': summaries}), flush=True)
    return summaries


def write_summary(all_samples):
    summary = {}
    lines = ['# Query pruning experiment', '', '| Layout | Query | Main mean ms | PR mean ms | Mean change | Main median ms | PR median ms |',
             '| --- | --- | ---: | ---: | ---: | ---: | ---: |']
    for layout, versions in all_samples.items():
        summary[layout] = {}
        for name in sorted(versions['main']):
            main = versions['main'][name]
            pr = versions['pr'][name]
            a, b = statistics.mean(main), statistics.mean(pr)
            ma, mb = statistics.median(main), statistics.median(pr)
            item = {'main_mean_ms': a, 'pr_mean_ms': b, 'mean_change_pct': 100 * (b / a - 1),
                    'main_median_ms': ma, 'pr_median_ms': mb, 'median_change_pct': 100 * (mb / ma - 1),
                    'main_samples_ms': main, 'pr_samples_ms': pr}
            summary[layout][name] = item
            lines.append(f'| {layout} | {name} | {a:.3f} | {b:.3f} | {item["mean_change_pct"]:+.1f}% | {ma:.3f} | {mb:.3f} |')
    save('comparison.json', summary)
    text = '\n'.join(lines) + '\n'
    (OUT / 'comparison.md').write_text(text)
    return text


def main():
    size = sys.argv[1]
    assert size in ('1m', '20m')
    assert os.environ.get('GITHUB_ACTIONS') == 'true', 'This script only runs in the isolated CI cluster'
    OUT.mkdir(exist_ok=True)
    # Vacuum once before index creation, never while comparing libraries.
    psql('VACUUM FULL ANALYZE')
    psql('VACUUM ANALYZE')
    bounds = date_bounds()
    selected = selected_queries(bounds)
    (OUT / 'settings.csv').write_text(psql('SELECT name, setting, unit FROM pg_settings ORDER BY name', csv_output=True))
    all_samples = {}
    expected_results = None
    trace_results = {}
    try:
        for layout in LAYOUTS:
            directory = DATASET / 'queries' / layout
            assert not directory.exists(), f'Refuse to replace existing suite: {directory}'
            directory.mkdir()
            for name, parts in selected.items():
                (directory / f'{name}.sql').write_text(';\n'.join(parts) + ';\n')
            ddl = (DATASET / 'indexes' / (layout + '.sql')).read_text()
            (OUT / f'{layout}-index.sql').write_text(ddl)
            names = re.findall(r'CREATE INDEX ([a-z_0-9]+)', ddl)
            assert len(names) == len(set(names)) and len(names) >= 4
            switch('main')
            psql(';'.join('DROP INDEX IF EXISTS ' + name for name in names))
            expected_identity = None
            all_samples[layout] = {'main': {}, 'pr': {}}
            for number, version in enumerate(ROUNDS, 1):
                switch(version)
                label = f'{layout}-r{number}-{version}'
                if expected_identity is not None:
                    assert identity() == expected_identity, 'Index changed before measurement'
                samples = benchmark(size, layout, label, selected, initialize=number == 1)
                if expected_identity is None:
                    expected_identity = identity()
                    save(f'{layout}-index-identity.json', expected_identity)
                assert identity() == expected_identity, 'Index changed during measurement'
                results = evidence(label, selected)
                if expected_results is None:
                    expected_results = results
                assert results == expected_results, f'Results differ across builds/layouts: {label}'
                for name, item in samples.items():
                    all_samples[layout][version].setdefault(name, []).extend(item['samples_ms'])
                save('progress.json', {'layout': layout, 'completed_round': label, 'results_match': True,
                                       'physical_indexes_unchanged': True})
            write_summary(all_samples)
            trace_results[layout] = trace(layout, selected)
            assert identity() == expected_identity, 'Diagnostic changed the indexes'
            shutil.rmtree(directory)
        # Date-only layout is the mechanism check: narrow window must actually prune.
        narrow = trace_results['bm25_pruning_date']['pruning_date_count']
        assert narrow['rejected_segments'], 'No segment pruning on the date-only layout'
        if bounds['null_dates'] == 0:
            assert not trace_results['bm25_pruning_date']['pruning_date_control']['rejected_segments'], 'Full date range unexpectedly rejects segments'
        for layout in LAYOUTS:
            assert trace_results[layout]['pruning_no_date_control']['eligible_calls'] == 0
        text = write_summary(all_samples)
        with Path(os.environ['GITHUB_STEP_SUMMARY']).open('a') as stream:
            stream.write(text)
        save('completion.json', {'complete': True, 'size': size, 'same_results_all_builds_and_layouts': True,
                                 'narrow_date_pruning_observed': True})
    finally:
        run(['cargo', 'pgrx', 'stop', 'pg18'], cwd=ROOT / 'pg_search')


if __name__ == '__main__':
    main()
