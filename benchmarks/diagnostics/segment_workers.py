"""Separate segment/layout effects from worker-budget effects on one pinned build.

Timing uses the ordinary benchmark runner and unchanged SQL. Profiles use one
persistent psql connection per query, outside timing; system-wide capture includes
short-lived MPP workers. Only the isolated GitHub Actions cluster may be changed.
"""

import os
from pathlib import Path
import re
import shutil
import statistics
import subprocess
import sys

import partition_layouts as layouts
import partition_pruning as paired


WORKER_ORDER = (4, 8, 16, 16, 8, 4)
COUNTS = (16, 48, 64)
JOIN = 'join_distinct_parent_sort__range_partitioned'
DIRECT = 'filtered_lowcard'


def set_workers(workers):
    # Raise the per-query budget, not the number of CPUs or maintenance workers.
    paired.psql(f'ALTER SYSTEM SET max_parallel_workers_per_gather = {workers}')
    paired.psql('SELECT pg_reload_conf()')
    assert paired.psql('SHOW max_parallel_workers_per_gather').strip() == str(workers)


def execution(label):
    result = {}
    for query in (DIRECT, JOIN):
        plan = (paired.OUT / f'{label}-{query}-plan.txt').read_text()
        launched = re.search(r'MPP Launch: workers=(\d+)', plan)
        stages = re.findall(r'Stage (\d+) ── tasks=(\d+), partitions=(\d+)', plan)
        result[query] = {
            'launched_workers': int(launched[1]) if launched else 0,
            'stages': stages,
            'distributed': 'DistributedExec' in plan,
            'postgres_gather': bool(re.search(r'\bGather(?: Merge)?\b', plan)),
            'range_assignment': bool(re.search(r'partition=\w+\[', plan)),
        }
    # Do not infer serial or parallel execution merely from the requested GUC.
    assert not result[DIRECT]['distributed'] and not result[DIRECT]['postgres_gather'], result
    assert result[JOIN]['distributed'] and result[JOIN]['range_assignment'], result
    assert result[JOIN]['launched_workers'] > 0 and result[JOIN]['stages'], result
    paired.save(label + '-actual-workers.json', result)
    return result


def profile(label, query, parts):
    # Startup is amortized; no new client/backend for every repetition.
    repetitions = 2000 if query == DIRECT else 100
    sql = paired.OUT / f'{label}-{query}-profile.sql'
    sql.write_text('\\o /dev/null\n' + ';\n'.join(parts[:-1]) + ';\n'
                   + (parts[-1] + ';\n') * repetitions)
    command = ['psql', paired.URL, '-Xq', '-v', 'ON_ERROR_STOP=1', '-f', str(sql)]
    # Warm outside perf and ordinary timing samples.
    paired.psql(';\n'.join(parts))
    stem = paired.OUT / f'{label}-{query}'
    data = Path(str(stem) + '-perf.data')
    commands = {
        'perf-stat': ['sudo', 'perf', 'stat', '-a', '-x,',
                      '-e', 'cycles,instructions,branches,branch-misses,cache-misses',
                      '-o', str(stem) + '-perf-stat.csv', '--', *command],
        'perf-record': ['sudo', 'perf', 'record', '-a', '-e', 'cycles:u', '-F', '499',
                        '--call-graph', 'dwarf,16384', '-o', str(data), '--', *command],
        'perf-self': ['sudo', 'perf', 'report', '--stdio', '--no-children',
                      '--comms', 'postgres', '--percent-limit', '0.1', '-i', str(data)],
        'perf-children': ['sudo', 'perf', 'report', '--stdio', '--children',
                          '--comms', 'postgres', '--percent-limit', '0.1', '-i', str(data)],
        # Unfiltered report exposes missing comm filters and startup/background samples.
        'perf-all': ['sudo', 'perf', 'report', '--stdio', '--no-children',
                     '--percent-limit', '0.1', '-i', str(data)],
        'perf-header': ['sudo', 'perf', 'report', '--header-only', '-i', str(data)],
    }
    for kind, args in commands.items():
        result = subprocess.run(args, cwd=paired.ROOT, capture_output=True, text=True)
        Path(str(stem) + f'-{kind}.txt').write_text(result.stdout)
        Path(str(stem) + f'-{kind}.stderr').write_text(result.stderr)
        if result.returncode:
            raise RuntimeError(f'{kind} failed: {label}/{query}: {result.stderr}')
    # Retain raw stacks for re-analysis instead of only aggregate hardware counters.
    paired.run(['sudo', 'chmod', 'a+r', str(data), str(stem) + '-perf-stat.csv'])
    report = Path(str(stem) + '-perf-self.txt').read_text()
    paired.save(str(stem.name) + '-profile-quality.json', {
        'repetitions': repetitions,
        'postgres_report_has_pg_search': 'pg_search' in report,
        'postgres_report_has_unknown': '[unknown]' in report,
        'scope': 'system-wide userspace samples; filtered and unfiltered reports retained; '
                 'inspect symbol resolution and lost samples before attribution',
    })


def summary(samples):
    paired.save('worker-comparison.json', [
        {'layout': layout, 'worker_setting': worker, 'query': query, 'n': len(values),
         'mean_ms': statistics.mean(values), 'median_ms': statistics.median(values),
         'samples_ms': values}
        for (layout, worker, query), values in samples.items()
    ])


def main():
    assert sys.argv[1] == '1m'
    assert os.environ.get('GITHUB_ACTIONS') == 'true', 'Isolated CI cluster only'
    assert os.environ.get('PROFILE_PRUNING') == 'true', 'This experiment requires profiles'
    paired.OUT.mkdir(exist_ok=True)
    layouts.SEGMENT_SWEEP = True
    selected, raw, keys, manifest = layouts.load_queries()
    paired.save('query-manifest.json', manifest)
    original = (paired.DATASET / 'indexes/bm25.sql').read_text()
    assert original == paired.run(['git', 'show', f'{layouts.BASELINE}:benchmarks/datasets/stackoverflow/indexes/bm25.sql'], capture=True)

    # Constant headroom: a setting of 16 must not be silently capped by the old pool of 8.
    paired.psql('ALTER SYSTEM SET max_worker_processes = 32')
    paired.psql('ALTER SYSTEM SET max_parallel_workers = 32')
    paired.psql('ALTER SYSTEM SET max_parallel_maintenance_workers = 8')
    paired.switch('pr')
    assert paired.psql('SHOW max_worker_processes').strip() == '32'
    assert paired.psql('SHOW max_parallel_workers').strip() == '32'
    paired.psql('VACUUM FULL ANALYZE')
    paired.psql('VACUUM ANALYZE')
    paired.save('experiment-design.json', {
        'binary': os.environ['PR_SHA'], 'segment_targets': COUNTS,
        'layouts': ['original', 'posts_type'], 'worker_order': WORKER_ORDER,
        'samples_per_round': paired.RUNS, 'build_workers': 8,
        'segment_count_scope': 'all four indexes; fixed physical index per worker comparison',
        'profiles': 'serial filter at worker setting 8; join at settings 8 and 16',
        'limits': 'one build per layout/count; layout order not interleaved; no ongoing writes; '
                  'worker-setting effects require checking actual widths and plan changes',
    })
    samples, expected, modes = {}, None, {}
    for target in COUNTS:
        for arm in ('original', 'posts_type'):
            layout = f'bm25_workers_{arm}_s{target}'
            directory = paired.DATASET / 'queries' / layout
            fixture = paired.DATASET / 'indexes' / (layout + '.sql')
            assert not directory.exists() and not fixture.exists()
            directory.mkdir()
            try:
                for name, content in raw.items():
                    (directory / (name + '.sql')).write_bytes(content)
                ddl = layouts.layout_ddl(original, layouts.LAYOUTS[arm])
                fixture.write_text(ddl)
                (paired.OUT / (layout + '-index.sql')).write_text(ddl)
                set_workers(8)
                paired.psql(f'ALTER SYSTEM SET paradedb.global_target_segment_count = {target}')
                paired.psql('SELECT pg_reload_conf()')
                assert paired.psql('SHOW paradedb.global_target_segment_count').strip() == str(target)
                names = re.findall(r'CREATE INDEX ([a-z_0-9]+)', ddl)
                paired.psql(';'.join('DROP INDEX IF EXISTS ' + n for n in names))
                # Build and warm once, excluded from all comparison samples.
                paired.benchmark('1m', layout, layout + '-build', selected, initialize=True)
                inventory = paired.identity()
                paired.save(layout + '-index-identity.json', inventory)
                layouts.segment_inventory(layout, target)
                for round_no, workers in enumerate(WORKER_ORDER, 1):
                    set_workers(workers)
                    label = f'{layout}-r{round_no}-w{workers}'
                    assert paired.identity() == inventory
                    (paired.OUT / f'{label}-settings.csv').write_text(paired.psql(
                        'SELECT name, setting, unit FROM pg_settings ORDER BY name', csv_output=True))
                    values = paired.benchmark('1m', layout, label, selected, initialize=False)
                    results = layouts.evidence(label, selected, keys)
                    modes[label] = execution(label)
                    if expected is None:
                        expected = results
                    mismatches = [q for q in expected if expected[q] != results[q]]
                    paired.save(label + '-validation.json', {'companion_mismatches': mismatches})
                    assert not mismatches, (label, mismatches)
                    assert paired.identity() == inventory
                    for query, item in values.items():
                        samples.setdefault((layout, workers, query), []).extend(item['samples_ms'])
                    summary(samples)
                    paired.save('progress.json', {'last_round': label})

                for workers in (8, 16):
                    set_workers(workers)
                    for query in ((DIRECT, JOIN) if workers == 8 else (JOIN,)):
                        profile(f'{layout}-w{workers}', query, selected[query])
                set_workers(8)
                layouts.trace(layout, selected)
                paired.switch('pr')
                assert paired.identity() == inventory
            finally:
                shutil.rmtree(directory)
                fixture.unlink(missing_ok=True)

    assert len(samples) == 36 and all(len(v) == 60 for v in samples.values())
    paired.save('all-execution-modes.json', modes)
    paired.save('completion.json', {
        'complete': True, 'cells': 18, 'queries': 2, 'samples_per_cell_query': 60,
        'correctness_companions_match': True,
        'conclusions': 'Read actual-worker counts, plans and profile quality before attributing costs.',
    })


if __name__ == '__main__':
    main()
