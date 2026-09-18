# HN read-amplification experiment checkpoint

This branch preserves the pg_search integration, diagnostics, and opt-in controls from the HN investigation. The exact COUNT proof is a separate earlier commit and is documented in [COUNT.md](COUNT.md). The remaining changes are research prototypes; they must be extracted and reviewed independently before production enablement. Every new runtime experiment defaults off.

Tantivy is pinned to [60f739c19b28d6714f8eff3f3f86c1bec9fb3fad](https://github.com/paradedb/tantivy/commit/60f739c19b28d6714f8eff3f3f86c1bec9fb3fad); no absolute local dependency path is required. Its `POSTINGS_EXPERIMENTS.md` describes algorithm changes and limits.

## Recorded measurements

The corpus contains 28,737,557 HN rows on isolated PostgreSQL 17.9, with ten immutable CTID-sorted segments. The ParadeDB base is `8325b0bab2f4b37acb5325532ae9f573cb9c3186`. Comparisons used one backend, no parallel workers, and opposite variant orders. Cold counts below include planning plus execution Shared Read Blocks after relation eviction; OS caches remained warm. Individual comparisons took seconds; paired Benchmarker timings took about 31 seconds.

| Query / change                                             | Baseline reads | Candidate reads | Semantics                   |
| ---------------------------------------------------------- | -------------: | --------------: | --------------------------- |
| Five-term OR COUNT, visibility proof                       |          8,846 |             844 | Exact count                 |
| Ten-term OR COUNT, visibility proof                        |          8,986 |             984 | Exact count                 |
| Single-term COUNT, visibility proof                        |          8,443 |             340 | Exact count                 |
| Five-term ranked OR, local exact norms                     |          3,691 |           1,140 | Unchanged ranking           |
| Ten-term ranked OR, local exact norms                      |          4,163 |           1,301 | Unchanged ranking           |
| Five-term AND, adaptive membership + lazy reads            |          4,365 |           1,091 | Unchanged ranking           |
| Ten-term AND, same                                         |          4,428 |             631 | Unchanged ranking           |
| Nonempty learning phrase, lazy positions + anchors         |         18,255 |           2,184 | Same ten results and scores |
| Public phrase 4, same                                      |         47,120 |           2,247 | Exact, zero HN results      |
| Five public OR strings, dense policy + local norms, summed |         59,568 |           9,688 | Changed ranking policy      |

Technical strings are `open source licensing business model` and `startup founder advice early stage product market fit customer discovery`. The nonempty phrase is `the best way to learn programming`. Public query 4 is `i forgot this but this is a better answer than mine and the equivalents to`. All five selected public phrase strings have zero HN results; they demonstrate early rejection, not reproduction of the original Stack Exchange corpus. No TIN backend was run.

The broad COUNT shortcut improved warm median latency about 3.7x but made selective AND counts slower; it needs a cost gate. Exact local norms saved 3.2x OR reads but only about 2–4% warm latency. A common two-term phrase remained unchanged. Dense-term scoring controls intentionally change BM25 results and are not evidence of a universal tenfold full-scoring OR gain.

## Integration and build

Use the existing pgrx workflow with a PostgreSQL17 pg_config, `--no-default-features --features pg17,io_stats,deferred_wal`, and an isolated CARGO_TARGET_DIR. The io_stats feature enables component counters and diagnostic exporters; disable `paradedb.experiment_io_stats` for timing.

The GUCs separately control lazy postings/positions, adaptive membership-first conjunctions, phrase anchors, frequency bounds, completed-prefix thresholds, dense-term scoring, and an exact norm sidecar provider. `SearchIndexReader` captures them per query. The sidecar and export functions are available only with io_stats and require privileged experimental use.

The auxiliary norm relation is disposable diagnostic storage. It does not implement a production codec or auxiliary-relation maintenance across vacuum, reindex, and merges. Segment/suffix score-bound experiments inherit a conservativeness issue when corpus-average length changes. Completed-prefix thresholds are restricted to serial execution by the PG integration and can change float addition order by one ULP. These controls remain opt-in.

## Validation

Recorded Rust filters include Block-WAND 16 passed / 2 preexisting ignored, term-query 28 passed / 1 preexisting ignored, dense-reader opening 5 passed, phrase 31 passed / 1 preexisting ignored; earlier anchor/intersection/provider checks also passed. PostgreSQL COUNT fixtures passed 110 independent SQL-oracle comparisons including old snapshots and an observed VACUUM BufferPin wait. Dense-policy fixtures passed 20 pagination/membership/visibility checks, and all 30 public-OR policy-oracle comparisons had zero score-ULP differences. Both 31-second COUNT Benchmarker runs passed exact preflight hashes and all 1,834/1,836 checks.

The measured V7 library SHA256 was `da075fc1609593a5db4ee59f8b0b1aa588afe6f18f0bc7aecfc08b6d35e38cdc`. The saved source checkpoint subsequently received formatting and reproducible git dependency pinning. No pg18 build was needed for this commit.
