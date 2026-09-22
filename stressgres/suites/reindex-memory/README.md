# Concurrent reindex memory reproduction

These synthetic fixtures exercise a BM25 index with ngram text and JSON indexed in two ways. They check 40,000 search matches while reindexing, optionally with concurrent updates. The forced-overlap variants sort by interleaved IDs and request one segment; the control uses default sorting. This distinguishes sorted-merge amplification from general build memory.

## Run

Use a disposable PostgreSQL instance with pg_search loaded and a database named `merge_repro`. Every setup checks that database name before dropping only its `merge_memory_repro` schema. The suites connect to postgres at 127.0.0.1:5432; adjust their connection strings for your local instance. Disable `paradedb.global_enable_background_merging` at server startup for a matched comparison. The measured runs used a 4 GiB container limit and shared_buffers=256MB. Each build session sets maintenance_work_mem=32MB and max_parallel_maintenance_workers=0.

From the repository root, run each suite separately:

```sh
cargo run -p stressgres -- headless stressgres/suites/reindex-memory/read-only.toml --runtime=45000
cargo run -p stressgres -- headless stressgres/suites/reindex-memory/concurrent-writes.toml --runtime=45000
cargo run -p stressgres -- headless stressgres/suites/reindex-memory/default-sort.toml --runtime=30000
```

Stressgres cancels in-flight work when its runtime expires. A canceled concurrent reindex can leave an invalid `ledger_search_ccnew`; this is expected deadline behavior, not evidence of an OOM. For an untimed measurement with longer text (512 repetitions instead of 64), recreate the fixture and then run three reindexes in separate sessions:

```sh
psql -X -v ON_ERROR_STOP=1 -d merge_repro -f stressgres/suites/reindex-memory/scaled-setup.sql
for run in 1 2 3; do
  PGOPTIONS='-c maintenance_work_mem=32MB -c max_parallel_maintenance_workers=0' \
    psql -X -v ON_ERROR_STOP=1 -d merge_repro \
      -c 'REINDEX INDEX CONCURRENTLY merge_memory_repro.ledger_search;' \
      -c 'SELECT merge_memory_repro.verify();' \
      -c "SELECT count(*), sum(num_docs) FROM pdb.index_segments('merge_memory_repro.ledger_search'::regclass);"
done
```

Expect 40,000 matches and one segment with 40,000 documents. Inspect pg_index for indisvalid/indisready and leftover replacement indexes after untimed builds.

## Measurement and evidence

Sample PostgreSQL backend `/proc/PID/status` RssAnon from the server PID namespace while recording pg_stat_progress_create_index. Keep cgroup memory.current separate because it includes cache/shared memory. Stressgres's own process-memory display cannot measure a server in a different PID namespace. A whole setup session also performs inserts and verification; split those operations before attributing a setup peak to index creation.

Using identical core/compiler settings and baseline versus streaming-merge Tantivy builds, the larger repeated reindex peaked at 120.5–126.5 MiB baseline and 51.5 MiB patched (59.2% reduction in median peak). The repeated default-sort control was approximately 42.4 MiB in both. All timed workloads and untimed correctness checks passed. These were sampled anonymous RSS measurements, not allocator totals or a production-sized benchmark.

The separate Tantivy merge_memory example isolates allocations and supplies the memory regression check. This SQL workload deliberately forces sort overlap and does not establish the cause of a production crash using default index sorting. The separate setup spike was subsequently isolated to ANALYZE index-expression statistics; see below. Neither maintenance_work_mem nor this fix is a total process-memory cap.

## Separate ANALYZE spike

Phase-separated measurements of the same scaled fixture attributed the large setup spike to `ANALYZE`, after index creation. In fresh backends, controls measured:

| Operation                           | Sampled rows | Baseline peak RssAnon (MiB) | Patched peak (MiB) |
| ----------------------------------- | -----------: | --------------------------: | -----------------: |
| Full ANALYZE, statistics target 100 |       30,000 |                       712.9 |              728.3 |
| Full ANALYZE, repeat                |       30,000 |                       708.9 |              716.7 |
| Full ANALYZE, statistics target 10  |        3,000 |                        73.4 |               65.3 |
| Explicit-column ANALYZE, target 100 |       30,000 |                        23.0 |               23.0 |

Insertion peaked around 23.7 MiB; ten standalone verification calls used about 8 MiB and returned 40,000 matches each. Phase sampling was approximately 20 ms, with 5 ms sampling for the controls. These are sampled peaks, not exact allocation maxima.

The index expressions produce 7,188, 7,231, and 7,231-byte datums. PostgreSQL collects copies of expression results for the sample before computing their statistics. An explicit column list skips index-expression statistics, explaining why the same 30,000 sampled rows use much less memory in that control. See PostgreSQL 17's [analyze.c](https://github.com/postgres/postgres/blob/REL_17_STABLE/src/backend/commands/analyze.c), specifically `do_analyze_rel` and `compute_index_stats`.

Run `analyze-controls.sql` only against the disposable fixture left by scaled-setup.sql. Use fresh sessions for individual statements when comparing RSS, to avoid allocator high-water effects. These controls explain this synthetic spike; they are not production tuning recommendations. Lowering statistics targets or skipping expression statistics can affect plans. This does not establish that ANALYZE caused the production OOM.
