window.BENCHMARK_DATA = {
  "lastUpdate": 1788365835772,
  "repoUrl": "https://github.com/paradedb/paradedb",
  "entries": {
    "benchmarker hn-ci (QPS)": [
      {
        "commit": {
          "author": {
            "email": "james.sewell@gmail.com",
            "name": "James Sewell",
            "username": "jamessewell"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "dd2424c18610b36018c75540a31ee16ecb018339",
          "message": "ci: benchmarker suite for pg_search (hn-benchmarker) (#5850)\n\nAdds a RunsOn workflow that benchmarks pg_search built from this\nbranch's source against the `hn-benchmarker` dataset using the external\nbenchmarker suite.\n\n- Builds pg_search from source and overlays it onto the\n`paradedb/paradedb:v0.25.0-pg18` base (buildkit-cache-dance warms the\ncargo/target mounts).\n- Brings up the dataset's single `paradedb` service via the `paradedb`\ncompose profile, points `PARADEDB_IMAGE` at the source build.\n- Runs every `k6/*.js` the dataset ships (currently `topk.js`), once\neach.\n- Publishes each script's dashboard HTML (with CPU/mem graphs) to\ngh-pages and links it from a PR comment; pushes QPS + p50/p90/p95/p99 to\ngh-pages for over-time tracking (main only).\n\n**Testing:** add the `benchmark-benchmarker` label to run. Requires the\n`hn-benchmarker` dataset to be present at\n`s3://paradedb-benchmarker/datasets/hn-benchmarker`.\n\n---------\n\nSigned-off-by: Philippe Noël <21990816+philippemnoel@users.noreply.github.com>\nCo-authored-by: Philippe Noël <21990816+philippemnoel@users.noreply.github.com>",
          "timestamp": "2026-08-19T11:04:28-04:00",
          "tree_id": "8c3bcacafae807e4074e533538fd64bd40655c80",
          "url": "https://github.com/paradedb/paradedb/commit/dd2424c18610b36018c75540a31ee16ecb018339"
        },
        "date": 1787153837624,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "paradedb (single_topk) QPS",
            "value": 522.4333333333333,
            "unit": "QPS"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "stuhood@paradedb.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "45b92e5c6cee1d64839d7c6d0d2c20c6397f752e",
          "message": "fix: Report dynamic filter pushdown under MPP (#5951)\n\n## What\n\nRecord dynamic filter pushdown via a metric.\n\n## Why\n\nTo allow for accurate reporting under MPP. `dynamic_filter_pushdown` is\ntriggered only after execution has started, and only metrics are\ntransferred back over the wire after MPP execution.\n\n## Tests\n\nSee changed regress tests.",
          "timestamp": "2026-08-19T09:32:46-07:00",
          "tree_id": "80143b13afb100c52f85648a131b2fb4084feec8",
          "url": "https://github.com/paradedb/paradedb/commit/45b92e5c6cee1d64839d7c6d0d2c20c6397f752e"
        },
        "date": 1787159011692,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "paradedb (single_topk) QPS",
            "value": 529.3490216992767,
            "unit": "QPS"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "stuhood@paradedb.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "802585d3398c5f6f0384021feb29b3813c7e75b5",
          "message": "perf: Use an in-memory channel for self-loops in MPP (#6002)\n\n# Ticket(s) Closed\n\n- Closes #5332\n\n## What\n\nIncorporates a new tag of `datafusion-distributed` which picks up\nhttps://github.com/datafusion-contrib/datafusion-distributed/pull/656,\nand switches to using it for self-loops.\n\n## Why\n\nTo avoid serialization costs when sending data in-process.\n\n## Tests\n\nCauses some benchmark movement on small datasets, but no change on\nlarger datasets.\n\n---------\n\nCo-authored-by: paradedb-github-app[bot] <282009505+paradedb-github-app[bot]@users.noreply.github.com>",
          "timestamp": "2026-08-19T19:03:30-07:00",
          "tree_id": "57508ac87b2e5595e113837b3c79f7cab2ddf966",
          "url": "https://github.com/paradedb/paradedb/commit/802585d3398c5f6f0384021feb29b3813c7e75b5"
        },
        "date": 1787192895476,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "paradedb (single_topk) QPS",
            "value": 511.1,
            "unit": "QPS"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "stuhood@paradedb.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "ddf7f2f06fd7e6fffd7c7a35d5623c11db7b89a0",
          "message": "chore: Render table name (or alias) in `PgSearchScan`. (#6009)\n\n## What\n\nRender the table name (or its assigned alias) in `EXPLAIN` for\n`PgSearchScan`.\n\n## Why\n\nBecause otherwise it's necessary to differentiate tables by inspecting\ntheir filters, which is error prone.\n\n## Tests\n\nTons of regress changes; no semantic changes.",
          "timestamp": "2026-08-19T20:08:21-07:00",
          "tree_id": "ff181690bd989f2fc653e2071d784668ac3b21ae",
          "url": "https://github.com/paradedb/paradedb/commit/ddf7f2f06fd7e6fffd7c7a35d5623c11db7b89a0"
        },
        "date": 1787196758216,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "paradedb (single_topk) QPS",
            "value": 509.1836394546485,
            "unit": "QPS"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mdashti@gmail.com",
            "name": "Moe",
            "username": "mdashti"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "a1158b82f1ffd9deeafa7c4633afd516ff897dbb",
          "message": "perf: allocate the per-segment EXPLAIN info slots only where used (#6004)\n\nThis PR allocates the per-segment EXPLAIN info slots only for scans that\npublish them.\n\n## Why\n\n`amestimateparallelscan` sizes the parallel index-scan DSM for\n`u16::MAX` segments sight unseen below PG18, and the layout allocated\n`SEGMENT_INFO_MAX_PER_SEG` (1024) bytes per segment unconditionally: a\n~66MB estimate per parallel index scan. Only basescan's Top-K workers\npublish that telemetry (`publish_segment_info` / `take_segment_info`);\nthe AM index scan and the MPP launches never touch it.\n\n## What\n\n- `ParallelScanPayloadLayout` takes `with_segment_info`; the info\nregions are zero-length when off.\n- `ParallelScanArgs` carries the flag: on for BaseScan (which sizes and\npopulates from the same args), off for the JoinScan and AggregateScan\nMPP launches and the AM index scan. The PG15-17 estimate drops to ~2MB,\ndominated by the 16-byte segment ids.\n- `set_segment_info` / `take_segment_info` no-op when the layout has no\nslots, so a misrouted publish can't index past a zero-length region.\n\nThe estimate below PG18 stays a blind `u16::MAX` guess; sizing it from\nthe real segment count needs the relation, which only the PG18 signature\nprovides.",
          "timestamp": "2026-08-20T01:49:03-07:00",
          "tree_id": "fd871fe38fa58ba333797e6d0eeaf72d9b56c287",
          "url": "https://github.com/paradedb/paradedb/commit/a1158b82f1ffd9deeafa7c4633afd516ff897dbb"
        },
        "date": 1787217585155,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "paradedb (single_topk) QPS",
            "value": 504.41651944935165,
            "unit": "QPS"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mdashti@gmail.com",
            "name": "Moe",
            "username": "mdashti"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "98c3378d167341dfdc736cd4a3126ef554205f84",
          "message": "feat: computed global partition boundaries for `CREATE INDEX`. (#5991)\n\n# Ticket(s) Closed\n\n- Closes #5736\n\n## What\n\nThe `CREATE INDEX` leader now computes global partition boundaries for a\n`partition_by` index and passes them to the parallel build workers. The\nboundaries are a recursive KD-tree over the `partition_by` fields, built\nonce from a heap sample. Workers deserialize the tree and log it at\n`DEBUG1`; routing on it is #5737.\n\n## Why\n\nEach parallel worker gets an arbitrary slice of the heap. If workers\npicked boundaries from their own tuples, segments wouldn't line up and\nlater merges would fix the edges. One set of boundaries fixed before any\nworker starts keeps a fresh index's segments aligned.\n\n## How\n\n- `index/kdtree.rs` (new, Postgres-free): `KdTree::from_sample` splits\nrecursively. Each cut is the quantile that gives both children a share\nof the sample proportional to their leaves; the dimension is the one\nspanning the widest slice of its global distribution (by rank, so mixed\ntypes compare), ties to the earlier field. Low-cardinality data can\nyield fewer than `target` leaves. Routing matches\n`RangePartitioning::partition_bounds` (NULL and `< split` left, `>=\nsplit` right, in-order leaves); in one dimension it *is* a\n`RangePartitioning`. `route`/`partition_bounds`/`partition_count` are\nthe #5737 API.\n- `postgres/build_partitioning.rs` (new): the leader samples the heap,\nnot `pg_statistic`. `BlockSampler` over <= 4096 blocks, reservoir to 30k\nrows (the `ANALYZE` size), converted through the writer's own path so\nexpression/aliased fields work.\n- `build_parallel.rs`: leaf count is `adjusted_target_segment_count`.\nThe tree ships as a binary Vec.\n\n## Tests\n\n- `kdtree.rs` and `pdb_owned_value.rs` unit test\n- `build_partitioning.rs` `pg_test`s",
          "timestamp": "2026-08-20T14:28:52-07:00",
          "tree_id": "3aaea0e0306b0d2236a8c0128d6e972f0038d5e6",
          "url": "https://github.com/paradedb/paradedb/commit/98c3378d167341dfdc736cd4a3126ef554205f84"
        },
        "date": 1787262751514,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "paradedb (single_topk) QPS",
            "value": 526.4666666666667,
            "unit": "QPS"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "142809952+aryanpatel-ctrl@users.noreply.github.com",
            "name": "aryanpatel-ctrl",
            "username": "aryanpatel-ctrl"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "1eadf7532c877f8420ae9080f62a968e043c7281",
          "message": "fix(mpp): align plain EXPLAIN with serial fallback when MPP cannot launch (#5822)\n\n## Summary\n- Fixes [#5784](https://github.com/paradedb/paradedb/issues/5784): when\ntask discovery finds fewer than 2 producer tasks, plain `EXPLAIN`\nrebuilds and renders the serial plan instead of a cap-sized distributed\nshape.\n- Shares the launch gate (`mpp_plan_has_data_parallelism`) across\nJoinScan and AggregateScan, matching plan-first MPP launch behavior from\n#5756.\n- Extends `mpp_worker_sizing` to assert the 1-segment plain-EXPLAIN\nserial contract.\n\n## Test plan\n- [x] `mpp_worker_sizing` regress\n- [x] 1-segment join: plain EXPLAIN has no `RoundRobinBatch` /\n`SortPreservingMergeExec`\n- [x] 1-segment join: EXPLAIN ANALYZE has no `MPP Launch` line\n- [x] 2-segment join still launches `workers=2` under an oversized cap",
          "timestamp": "2026-08-20T17:30:20-04:00",
          "tree_id": "3f7b3f45ca6495f94b1d6633bedf18da1ab80ae1",
          "url": "https://github.com/paradedb/paradedb/commit/1eadf7532c877f8420ae9080f62a968e043c7281"
        },
        "date": 1787264639043,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "paradedb (single_topk) QPS",
            "value": 546.2333333333333,
            "unit": "QPS"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "70322560+mehrdad3301@users.noreply.github.com",
            "name": "Mehrdad Mahabadi",
            "username": "mehrdad3301"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "a5a85388a4b99a39baed12c5d300ee88dc42f06a",
          "message": "feat: Push down ORDER BY range into the index (#2688) (#5791)\n\nFixes #2688\n\nPushes `ORDER BY` on Postgres range columns into the BM25/paradedb index\nTopK path. `SortByRange` reads the indexed bound sub-columns and\ncompares them using the same ordering as Postgres `range_cmp`.\n\n**Limitations**\n- Range columns must be the **leading** `ORDER BY` key; later keys fall\nback to Postgres sorting.\n- Only raw range columns are supported (not `lower(range_col)` or other\nexpressions).\n\n**Mapping**\nEmpty ranges sort first, unbounded lowers before finite lowers,\nunbounded uppers after finite uppers, and inclusive/exclusive endpoints\ntie-break like `range_cmp`.\n\n---------\n\nCo-authored-by: Cursor <cursoragent@cursor.com>\nCo-authored-by: Mohammad Dashti <mdashti@gmail.com>",
          "timestamp": "2026-08-20T16:51:32-07:00",
          "tree_id": "7c778fa3068610ee989c53feecc39c9bf87eceb7",
          "url": "https://github.com/paradedb/paradedb/commit/a5a85388a4b99a39baed12c5d300ee88dc42f06a"
        },
        "date": 1787271420889,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "paradedb (single_topk) QPS",
            "value": 466.05113162894565,
            "unit": "QPS"
          }
        ]
      }
    ],
    "benchmarker hn-ci (latency)": [
      {
        "commit": {
          "author": {
            "email": "james.sewell@gmail.com",
            "name": "James Sewell",
            "username": "jamessewell"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "1696390d10ad0772db19a7918a21d89803ff4d26",
          "message": "ci: only post the benchmarker summary comment on PRs (#6005)\n\nFollow-up to #5850, aligning the benchmarker with the benchmark-queries\n/ benchmark-stressgres conventions.\n\n- Summary comments are PR-only; nothing posts on pushes to main (main\nruns are tracked on the gh-pages charts). Compare tables come from\ngithub-action-benchmark's comment-always.\n- One tracked series instead of two: per-run mean + p50/p90/p95/p99\nlatency under a single customSmallerIsBetter publish, like queries. The\nk6 scripts are closed-loop, so mean ms carries the QPS signal (QPS = VUs\n/ mean latency); this halves the publish steps and PR comments.\n- Switched to the paradedb github-action-benchmark fork used by the\nother benchmark workflows (shallow-clones only the gh-pages branch).\n- Fixed a masked failure: with two publishes, the second one's gh-pages\nclone always failed on the non-empty ./benchmark-data-repository left by\nthe first, and continue-on-error hid it, so no latency history ever\nreached gh-pages. Now moot with a single publish, and the cleanup still\nruns before it.",
          "timestamp": "2026-08-21T13:12:54+12:00",
          "tree_id": "39bf4ea7592d99da81c362269c4af159c356ce01",
          "url": "https://github.com/paradedb/paradedb/commit/1696390d10ad0772db19a7918a21d89803ff4d26"
        },
        "date": 1787276285419,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "paradedb (single_topk) mean latency",
            "value": 2.122381672851844,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p50 latency",
            "value": 2.03,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p90 latency",
            "value": 2.62,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p95 latency",
            "value": 2.729,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p99 latency",
            "value": 2.88,
            "unit": "ms"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mithun.cy@gmail.com",
            "name": "Mithun Chicklore Yogendra",
            "username": "mithuncy"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "b13e9670f0e2e66028a22eb4930141866837b07b",
          "message": "fix: respect collation semantics in AggregateScan grouping (#5703)\n\n# Ticket(s) Closed\n\n- None. Follow-up to #5180.\n\n## What\n\nAggregateScan declines GROUP BY pushdown when it cannot preserve\nPostgreSQL's\ngrouping semantics: nondeterministic collations, `GROUPING SETS`, and\nGROUP BY\nwithout verifiable pathkeys fall back to PostgreSQL. Deterministic\ncollations\n(including ICU like `en-US`) stay eligible. Declined `pdb.agg()` queries\nemit\na planner WARNING naming the reason.\n\n## Why\n\nPostgreSQL groups with collation-aware equality; ParadeDB's backends\ngroup by\nbytes. With a case-insensitive collation:\n\n```text\nElectronics | 1        electronics | 2\nelectronics | 1   vs.  (correct)\n```\n\nWrong groups cannot be repaired above the scan. Separately, `GROUPING\nSETS`\nsilently dropped the grand-total row — a grouping-shape bug the same\neligibility gate now catches, unrelated to collations.\n\n## How\n\nGrouping needs collation *equality*, not *ordering*: deterministic\ncollations\nbreak ties bytewise, so their grouping pushes down while their ORDER BY\nstays\nwith PostgreSQL. A new shared module `collation_semantics.rs` models\nboth\n(`CollationOperation::Equality` / `::Ordering`), replacing\n`orderby.rs::is_collation_pushdown_safe()` for all callers — including\n#5148's\nDISTINCT gates after merging main. `create_custom_path` declines the\nunsafe\nshapes before building any path; `pdb.agg()` declines warn first, then\nfail on\nthe placeholder as before.\n\n## Tests\n\nExtended `order_by_collation.sql`: deterministic ICU stays pushed down\nwith\n`pdb.agg()` executing; nondeterministic falls back and merges equivalent\nvalues; GROUPING SETS, constant-equality, hash-only, and mixed keys\ndecline\nwith correct results; each declined `pdb.agg()` warns with its reason.",
          "timestamp": "2026-08-21T08:14:37+05:30",
          "tree_id": "30787eb999c72c03cca092a44e2cd387c16006d5",
          "url": "https://github.com/paradedb/paradedb/commit/b13e9670f0e2e66028a22eb4930141866837b07b"
        },
        "date": 1787281748108,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "paradedb (single_topk) mean latency",
            "value": 1.860179016598829,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p50 latency",
            "value": 1.737,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p90 latency",
            "value": 2.271,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p95 latency",
            "value": 2.394,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p99 latency",
            "value": 2.748,
            "unit": "ms"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "stuhood@paradedb.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "ef3e38bb8d216544db715e5378429b0293dfbf9f",
          "message": "perf: Use predicate tagging for disjunctions. (#6010)\n\n## What\n\nReplaces `SearchPredicateUDF` with predicate tagging for evaluating\njoin-level search predicates (e.g. cross-table disjunctions such as\n`p.description @@@ 'laptop' OR s.description @@@ 'display'`) in the join\nand aggregate scans.\n\n## Why\n\nEvaluating join-level search predicates using UDFs\n(`pdb_search_predicate`) required:\n1. Shipping canonical segment IDs across logical and physical plan\nserialization boundaries.\n2. Special handling for visibility\n3. Computing and intersecting CTID sets using a UDF\n\nPredicate tagging simplifies the intersection into a per-segment bitmap\nlookup on the `DocId`, without needing to fetch or visibility check\nctids.\n\n## How\n\nExpose matches as synthetic boolean columns for DataFusion boolean\nexpressions, which are then evaluated as vectorized boolean operations\nafter the join.\n\n## Tests\n\nExpanded tests.\n\nIn local benchmarks, predicate tagging was 22x faster for low\nselectivity queries.\n\n---------\n\nCo-authored-by: paradedb-github-app[bot] <282009505+paradedb-github-app[bot]@users.noreply.github.com>",
          "timestamp": "2026-08-21T08:11:18-07:00",
          "tree_id": "efd2c1616d7338a807bf2ec3e48e88690164ee0c",
          "url": "https://github.com/paradedb/paradedb/commit/ef3e38bb8d216544db715e5378429b0293dfbf9f"
        },
        "date": 1787326541050,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "paradedb (single_topk) mean latency",
            "value": 1.812193764870972,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p50 latency",
            "value": 1.703,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p90 latency",
            "value": 2.222,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p95 latency",
            "value": 2.392,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p99 latency",
            "value": 2.672,
            "unit": "ms"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ming.ying.nyc@gmail.com",
            "name": "Ming",
            "username": "rebasedming"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "1173b5fa60ad3d8beeb01d6d485e4da1d956d617",
          "message": "ci(benchmarks): add pgvectorscale (diskann) as a cohere index variant (#5745)\n\nReauthors #5588 against the sweep-based CI that replaced its fixed\n`[params]`. Adds pgvectorscale StreamingDiskANN as a fifth cohere arm,\nrun via the `benchmark-cohere-pgvectorscale` label or the\n`pgvectorscale` dispatch choice.\n\nEach arm sweeps whichever GUC actually binds it, and every ladder ends\nat that GUC's hard maximum — verified against the extension's GUC\nregistration in 0.9.0 (`query_rescore` max 1000,\n`query_search_list_size` max 10000; #5588 stopped search_list_size at\n4000):\n\n| arm | swept GUC | ladder |\n|---|---|---|\n| unfiltered | `diskann.query_rescore` | 50 → 1000 |\n| 10pct | `diskann.query_search_list_size` | 200 → 10000 |\n| 1pct | `diskann.query_search_list_size` | 500 → 10000 |\n\n**The filtered arms cannot reach 90% on the default SBQ build.**\nFiltering is post-filter streaming, so only the ~10%/1% of the beam\npassing the predicate survives, and at most 1000 candidates are ever\nexact-rescored — with rescore pinned at that maximum, beam width is the\nonly lever left. Ending the ladders at the ceiling lets the sweep's\nexisting unreachable-target fallback report the best achievable point\n(flagged ⚠️) rather than a fabricated one. Uncompressed `storage_layout\n= plain` would lift them but is far too slow to build in CI. This arm is\ndeliberately **not** iso-recall with the others; that is the property\nbeing surfaced.\n\nSupersedes #5588.",
          "timestamp": "2026-08-21T12:05:37-07:00",
          "tree_id": "31d1103b88b59d2e32d3804dcec27c056bb087e6",
          "url": "https://github.com/paradedb/paradedb/commit/1173b5fa60ad3d8beeb01d6d485e4da1d956d617"
        },
        "date": 1787340584343,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "paradedb (single_topk) mean latency",
            "value": 1.8296011948755917,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p50 latency",
            "value": 1.708,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p90 latency",
            "value": 2.192,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p95 latency",
            "value": 2.327,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p99 latency",
            "value": 2.568,
            "unit": "ms"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "rjhallsted@gmail.com",
            "name": "RJ Barman",
            "username": "barbarj"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "ff880e56c7a137931e7073e29c14007ca0aabe5a",
          "message": "chore: Update to df55-based df-d and fix api changes (#6015)\n\n## What\n\nUpdate to `datafusion-55`.\n\n## Tests\n\nBenchmarks are neutral.\n\nRegress tests show some lost dynamic filters due to\nhttps://github.com/apache/datafusion/pull/24045, which is necessary for\ncorrectness.\n\n---------\n\nCo-authored-by: paradedb-github-app[bot] <282009505+paradedb-github-app[bot]@users.noreply.github.com>\nCo-authored-by: Stu Hood <stuhood@gmail.com>",
          "timestamp": "2026-08-21T12:42:44-07:00",
          "tree_id": "12af687f821e5cd94415531f6c50b3cf84f92a10",
          "url": "https://github.com/paradedb/paradedb/commit/ff880e56c7a137931e7073e29c14007ca0aabe5a"
        },
        "date": 1787342918582,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "paradedb (single_topk) mean latency",
            "value": 1.9089562423697302,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p50 latency",
            "value": 1.771,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p90 latency",
            "value": 2.363,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p95 latency",
            "value": 2.487,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p99 latency",
            "value": 2.818,
            "unit": "ms"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mdashti@gmail.com",
            "name": "Moe",
            "username": "mdashti"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "12ddb8a9e2912c6ae86a797df8bd1b97383e8326",
          "message": "fix(mpp): replay the leader's segment view in every parallel reader (#5993)\n\nThis PR makes every reader a query opens for one index source replay the\nsame segment view, so packed `DocAddress`es stay valid across processes.\n\nCloses #5988.\n\n## Why\n\nStressgres hit `remote task 2.0 failed: range end index\n18446744073709551613 out of range for slice of length 8` on the JoinScan\nMPP path. Packed `(segment_ord, doc_id)` addresses are only meaningful\nagainst the reader that packed them, but the consumer side (a rebuilt\n`FFHelper` behind a network boundary, the leader-hosted\n`VisibilityFilterExec`, `SearchPredicateUDF`) opens its own reader over\nthe same segment id set. Two opens don't expose the same `DocId` space:\n\n- A mutable segment is materialized per open, bounded by the `(max_doc,\nnum_deleted_docs)` its meta entry holds at that moment. Concurrent DML\nmoves that bound between the producer's open and the consumer's, so a\n`doc_id` past the shorter view overflows tantivy's bitpacker, or\nresolves to the wrong ctid when the counts happen to match.\n- Segment ordinals come from an unstable doc-count sort, so they can\npermute between opens.\n\n## What\n\n- `MvccSatisfies::ParallelWorker` now has a `SegmentView`: the origin\nreader's segments in ordinal order, plus each mutable segment's\n`(max_doc, num_deleted_docs)` bound. `load_metas` rewinds mutable\nentries to that bound (the log is append-only, so it always is a prefix)\nand orders segments by the view.\n- `ParallelScanState` sends every source's view; the JoinScan leader's\nproviders, `SearchPredicateUDF`, the worker scans, and the rebuilt\nresolvers all replay that view.\n- `paradedb.aggregate` workers keep an id-only view; their addresses\nnever leave the process.\n\n## Tests\n\nA `pg_test` for the view replay\n(`test_segment_view_replays_origin_reader`), an `mpp_joinscan_mutable`\nregress covering the worker-hosted and leader-hosted shapes, and a\nconcurrent integration test (`mpp_joinscan_concurrent.rs`) that fails on\nthe base commit within a second and also catches the silent wrong-result\nvariant.",
          "timestamp": "2026-08-22T02:17:40-07:00",
          "tree_id": "55a004f381a03edc0e6a5e122a97bdab1fa801a3",
          "url": "https://github.com/paradedb/paradedb/commit/12ddb8a9e2912c6ae86a797df8bd1b97383e8326"
        },
        "date": 1787391697727,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "paradedb (single_topk) mean latency",
            "value": 2.269713915166297,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p50 latency",
            "value": 2.153,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p90 latency",
            "value": 2.816,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p95 latency",
            "value": 2.908,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p99 latency",
            "value": 3.058,
            "unit": "ms"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ming.ying.nyc@gmail.com",
            "name": "Ming",
            "username": "rebasedming"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "e9491e6b4cc67e7d7d1706fcd1e8bc394eadc34f",
          "message": "ci(benchmarks): stop recall sweeps once the top target is reached (#6028)\n\nRecall sweeps measure every value in the ladder even after a point\nreaches r99, the highest recall target. Since values are ordered\ncheapest first and recall rises monotonically with these knobs, every\ntarget already has its cheapest qualifying point by then — probing the\nlarger values only burns runner time.\n\nThis breaks out of the sweep loop after the first value that reaches the\ntop target, and documents the cheapest-first ordering requirement on\n`SweepConfig.values`.",
          "timestamp": "2026-08-22T19:23:24-07:00",
          "tree_id": "9f2b9a833af20e3f74f1d8bb2abb442cfcb0446e",
          "url": "https://github.com/paradedb/paradedb/commit/e9491e6b4cc67e7d7d1706fcd1e8bc394eadc34f"
        },
        "date": 1787453206319,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "paradedb (single_topk) mean latency",
            "value": 1.7539633764545914,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p50 latency",
            "value": 1.652,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p90 latency",
            "value": 2.089,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p95 latency",
            "value": 2.244,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p99 latency",
            "value": 2.382,
            "unit": "ms"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "cc3dfcab58eb6a14179d639ad1e97f5369297f66",
          "message": "docs: refresh repository guidance and fix copy (#6033)\n\n## Summary\n- correct documentation spelling, capitalization, grammar, and the 2026\ncopyright year\n- align regression-test and Stressgres commands with the current CLIs\nand repository layout\n- replace obsolete Mintlify CLI instructions\n- repair JoinScan links after source files moved and replace stale\ncommit-pinned references with durable relative links\n- refresh Docker and Antithesis guidance\n- clarify the qgen.rs link label while retaining its correct target at\ntests/tests/qgen.rs\n- align Debian, Ubuntu, and RHEL package descriptions with current\nproduct messaging from #5909\n- clarify pgvector requirements in the pg_search development README and\nintegration-test fixture\n- update executable deployment examples to the verified 0.25.3 release\ntags\n\n## Audit scope\nReviewed every tracked README, including the GitHub Actions upgrade-test\nREADME, plus CONTRIBUTING.md, SECURITY.md, and CODE_OF_CONDUCT.md\nagainst current source files, manifests, workflows, and tool help.\n\n## Testing\n- targeted prek checks for every changed file\n- cargo pgrx regress --dry-run --auto pg18 PREFIX_your_test\n- cargo run -p stressgres -- --help\n- local-link resolution check across all audited files\n- verified the v0.25.3 release and both referenced Docker Hub tags\n\nStandalone actionlint reports pre-existing findings elsewhere in the\npublish workflows; the imported sections introduce no new findings.",
          "timestamp": "2026-08-23T13:01:18-04:00",
          "tree_id": "62d8c6c26a02f4f98d9e697298d0efc33f26f505",
          "url": "https://github.com/paradedb/paradedb/commit/cc3dfcab58eb6a14179d639ad1e97f5369297f66"
        },
        "date": 1787505898227,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "paradedb (single_topk) mean latency",
            "value": 1.9869596256684505,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p50 latency",
            "value": 1.9,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p90 latency",
            "value": 2.498,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p95 latency",
            "value": 2.694,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p99 latency",
            "value": 2.993,
            "unit": "ms"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "40a0c232be1ce4855e18b720c5b4fcc1bfe2b568",
          "message": "chore: refresh documentation and version references (#6034)\n\n## Summary\n- update Antithesis manifest application labels and monitoring\npredicates from 0.25.0 to 0.25.3\n- fix a broken changelog link and documentation copy issues\n- update django-paradedb from 0.12.0 to 0.13.0 and actions/cache from v4\nto v6\n\n## Validation\n- `prek run --files` on all changed files\n- `mint validate`\n- `mint broken-links`\n- Prettier, codespell, and `git diff --check`\n\n## Audit notes\n- historical changelog/migration version references remain unchanged\n- the Antithesis manifest is rendered from ParadeDB Helm chart 0.18.3\nwith manual overlays; updating it to the current chart should be handled\nas a dedicated regeneration and review",
          "timestamp": "2026-08-23T13:42:25-04:00",
          "tree_id": "411cb3c049dd156dda3a3ad21e735705a0de5977",
          "url": "https://github.com/paradedb/paradedb/commit/40a0c232be1ce4855e18b720c5b4fcc1bfe2b568"
        },
        "date": 1787508396555,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "paradedb (single_topk) mean latency",
            "value": 1.8836379714738605,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p50 latency",
            "value": 1.75,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p90 latency",
            "value": 2.354,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p95 latency",
            "value": 2.499,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p99 latency",
            "value": 2.739,
            "unit": "ms"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "e351f6cc818a2ab88965f8997da360c9a33b600a",
          "message": "docs: refresh documentation and repository configuration (#6036)\n\n## Summary\n\n- clarify custom scan eligibility, filtered vector search, horizontal\nscaling, and Elasticsearch transaction behavior\n- improve Kubernetes and CloudNativePG production deployment guidance\n- refresh the release process, changelog wording, PostgreSQL 18 Nix\nexample, and development comments\n- organize and prune Codecov, Codespell, Docker, Git, and Prettier\nignore lists\n- clean up the root Cargo manifest and local pg_search helper scripts\n\n## Validation\n\n- `prek run --files` on the changed files\n- `cargo metadata --no-deps --format-version 1`\n- `shellcheck scripts/*.sh`\n- `mint validate`\n- `mint broken-links`",
          "timestamp": "2026-08-23T15:08:37-04:00",
          "tree_id": "ac4527a0921a0e4f7b0f5d0265d3149ed66edc01",
          "url": "https://github.com/paradedb/paradedb/commit/e351f6cc818a2ab88965f8997da360c9a33b600a"
        },
        "date": 1787513550737,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "paradedb (single_topk) mean latency",
            "value": 1.8680196177062327,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p50 latency",
            "value": 1.741,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p90 latency",
            "value": 2.332,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p95 latency",
            "value": 2.496,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p99 latency",
            "value": 2.628,
            "unit": "ms"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "d7ff580c75e761f5c405d8585b37f6d6c23e6092",
          "message": "docs: refresh architecture and development guidance (#6038)\n\n## Summary\n\n- correct the architecture guide's description of mutable-segment\nbuffering and PostgreSQL parallel execution\n- refresh integration-test setup, dependency organization, paths, and\nshell examples\n- audit Stressgres helpers, CLI guidance, package metadata, and\nAntithesis scripts\n- fix Stressgres argument handling and Cargo invocation, plus a macOS\nBash compatibility issue in the Antithesis consistency check\n- update `THANKYOU.md` with pgvector, Lindera, DataFusion Distributed,\nand Apache Arrow\n- correct packaging and `pg_regress` terminology in `CONTRIBUTING.md`\n\n## Validation\n\n- repository commit hooks\n- `cargo test -p stressgres --no-default-features`\n- `cargo test --package tests --no-run`\n- `cargo fmt --all -- --check`\n- `bash -n` and `shellcheck` on the changed shell scripts\n- verified every Stressgres suite has a matching Antithesis setup script\n- `mint validate`\n- `mint broken-links`\n\n## References\n\n- [PostgreSQL parallel\nplans](https://www.postgresql.org/docs/current/parallel-plans.html)",
          "timestamp": "2026-08-23T17:00:23-04:00",
          "tree_id": "79e7aa2472e2d7ca420dd1c77fceaf6b03a946eb",
          "url": "https://github.com/paradedb/paradedb/commit/d7ff580c75e761f5c405d8585b37f6d6c23e6092"
        },
        "date": 1787521177225,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "paradedb (single_topk) mean latency",
            "value": 2.1640804773339073,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p50 latency",
            "value": 2.092,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p90 latency",
            "value": 2.739,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p95 latency",
            "value": 2.881,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p99 latency",
            "value": 3.09,
            "unit": "ms"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mdashti@gmail.com",
            "name": "Moe",
            "username": "mdashti"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "de6e2ebfe8c3bda48a5cc1e779c9848ac95040b7",
          "message": "feat(mpp): gate MPP behind a minimum source size (#5482)\n\n## Ticket(s) Closed\n\n- Closes #5329\n\n## What\n\nThis PR gates MPP behind a minimum estimated source size:\n`paradedb.mpp_min_rows`, default 500,000, with 0 disabling the gate.\n\n## Why\n\nThe MPP launch costs 13-18ms regardless of data size: worker spawn, plan\ndispatch, and per-worker index opens. On small queries that floor\ndominates whatever the parallel split saves. Measured on a 500K x 800K\njoin (release build, 4 workers), the crossover sits near 30ms of serial\nwork, about 300K driving rows: a selective anti-join runs 4.0ms serial\nvs 16.6ms under MPP, while a full-scan group-by runs 318ms serial vs\n82ms under MPP. Postgres has `min_parallel_table_scan_size` for the same\nreason; MPP had no analog. This is also the dominant share of the 0.24.3\nto 0.25.3 anti-join regression a customer reported.\n\n## How\n\nThe gate sits at the front of the launch path, the same point as the\nshort-launch serial fallback, so both reuse one serial path. Each source\ncounts the rows its `@@@` predicate is estimated to match, not the\nindex's document count: a selective query over a large index does little\nscan work, and the floor dominates it just the same. An unanalyzed\nsource falls back to its live document count, an upper bound, so missing\nstatistics err toward launching. The largest source stands for the scan;\nthe smaller sides ride along. Below the threshold the query runs the\nplain serial plan.\n\nThis gate is the `Row-capped` analog of BaseScan's #5150 policy table,\nand it reads the same planner match estimates. #5150's cost tier\n(`cost_test_limited`) doesn't transfer: BaseScan's parallelism is PG\nGather, so `parallel_setup_cost` honestly prices it, while the MPP\nlaunch floor has no PG cost-unit representation and JoinScan's\n`Flags::Force` path cost is fabricated. A cost comparison against those\nnumbers would be fake precision; if the floor gets priced later, the\ngate can graduate to a cost test.\n\nThe gate is a pure function of the planner estimates, shared between the\nlaunch and the plain-`EXPLAIN` plan rebuilds the same way #5822 shares\nthe producer-task floor, so the rendered plan agrees with the executed\nmode. An estimate the planner substituted from the index's total\ndocument count (Postgres expressions, heap filters) is discounted by\n`PARAMETERIZED_SELECTIVITY`, mirroring BaseScan, so prepared statements\non big indexes still gate. The default is calibrated at the measured\ncrossover with headroom; the GUC exists to tune it.\n\n## Tests\n\nRegression tests.",
          "timestamp": "2026-08-23T16:55:36-07:00",
          "tree_id": "77c61572da4eb096208e1777ec471191de03152c",
          "url": "https://github.com/paradedb/paradedb/commit/de6e2ebfe8c3bda48a5cc1e779c9848ac95040b7"
        },
        "date": 1787530752128,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "paradedb (single_topk) mean latency",
            "value": 1.7490349843924897,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p50 latency",
            "value": 1.639,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p90 latency",
            "value": 2.069,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p95 latency",
            "value": 2.261,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p99 latency",
            "value": 2.451,
            "unit": "ms"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "d297e61da2c2353a748fe6caf2fe23c898eccf06",
          "message": "chore: audit pg_search, Docker, and benchmarks (#6042)\n\n## Summary\n\n- refresh pg_search and pg_regress development guidance, remove the\nobsolete banner, and clean up stale regression TODOs\n- clarify generated versus hand-maintained Docker assets, complete the\nCloudNativePG extension-image example, and pin the property-test image\ndistro\n- align benchmark setup and CLI descriptions with current behavior and\nnarrow generated-result ignore rules\n- fix the benchmark CSV output filename and reject zero-run benchmark\ninvocations\n- protect generated pgBackRest configs containing AWS credentials with\nexclusive creation, mode 0600, and automatic cleanup\n\n## Validation\n\n- repository commit hooks\n- `cargo test -p benchmarks` (39 tests)\n- `cargo clippy -p benchmarks --all-targets -- -D warnings`\n- `cargo clippy -p pg_search --lib -- -D warnings`\n- `cargo fmt --all -- --check`\n- `cargo metadata --no-deps --format-version 1`\n- `cargo pgrx regress --dry-run` command verification\n- native ShellCheck and Bash syntax checks for Docker scripts\n- Docker Compose configuration validation\n- regenerated Dockerfiles from the published 0.25.3 packages and\nverified byte-for-byte template parity\n- Markdownlint, Prettier, and `git diff --check`\n\n## Audit notes\n\n- historical extension migrations and regression references were\nretained\n- open issue-linked TODOs were left unchanged\n- Docker-backed ShellCheck was unavailable because the local Docker\ndaemon was offline; native ShellCheck passed\n- full benchmark execution and Docker image builds were not run\n- `cargo test -p pg_search --lib` does not complete: the `pg_test` path\nlaunches a nested Cargo build against the same target directory and the\nouter process remains waiting after that build; no test failure was\nreported before termination",
          "timestamp": "2026-08-23T22:40:48-04:00",
          "tree_id": "8d844c3622ce7b3697bcdbec6b52a9973ff04a4f",
          "url": "https://github.com/paradedb/paradedb/commit/d297e61da2c2353a748fe6caf2fe23c898eccf06"
        },
        "date": 1787540720259,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "paradedb (single_topk) mean latency",
            "value": 1.89207097349642,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p50 latency",
            "value": 1.767,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p90 latency",
            "value": 2.332,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p95 latency",
            "value": 2.494,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p99 latency",
            "value": 2.747,
            "unit": "ms"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "b7262a75638c36e7e9ab3ff3dbc08bbe1f88ab1d",
          "message": "ci: audit GitHub automation and templates (#6044)\n\n## Summary\n\n- restore benchmark baseline publishing to `main` only and make manual\n`fail_on_error: false` effective\n- trigger benchmark runs when their local composite actions change\n- harden benchmark composite-action shell inputs and align the GitHub\nApp client-ID input name with its actual value\n- replace deprecated global `apt-key` usage with scoped PostgreSQL\nrepository keyrings and HTTPS package sources\n- repair issue and discussion links and route users to the core, ORM,\nand benchmark repositories with active issue trackers\n- reduce `FUNDING.yml` to the active GitHub Sponsors account and remove\nnonexistent discussion labels\n- refresh the benchmark source action description to match its actual\nbehavior\n\n## Validation\n\n- repository commit hooks\n- Prettier across `.github` YAML\n- Actionlint across all workflows, ignoring only known custom runner\nlabels, Actionlint's stale `create-github-app-token@v3` metadata, and\npre-existing inline ShellCheck findings\n- Actionlint with inline ShellCheck enabled for the changed benchmark\nworkflows\n- Python syntax compilation for `.github/actions` and `.github/scripts`\n- Bash syntax and ShellCheck for standalone `.github/scripts`\n- live verification of issue destinations and the `paradedb/charts`\nIssues setting\n- live verification of the GitHub Sponsors destination and repository\nlabels used by templates\n- `git diff --check`\n\n## Audit notes\n\n- generated code-snippet verification fixtures were treated as generated\nartifacts and were not hand-edited\n- action major-version updates remain managed by Dependabot\n- full benchmark, release, and publishing jobs require CI\ncredentials/infrastructure and were not run locally",
          "timestamp": "2026-08-23T23:42:36-04:00",
          "tree_id": "69fa91159263c72992b4308ca6f62fe5e301df8c",
          "url": "https://github.com/paradedb/paradedb/commit/b7262a75638c36e7e9ab3ff3dbc08bbe1f88ab1d"
        },
        "date": 1787544382896,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "paradedb (single_topk) mean latency",
            "value": 1.8180154901120482,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p50 latency",
            "value": 1.716,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p90 latency",
            "value": 2.194,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p95 latency",
            "value": 2.301,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p99 latency",
            "value": 2.41,
            "unit": "ms"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "81e7d38632bacafad455cde5ecf09bb694416c37",
          "message": "chore: audit docs configuration (#6045)\n\n## Summary\n- remove the obsolete Mintlify promotional search placeholder override\n- update the footer social link from `twitter.com` to `x.com`\n- remove the empty `.mintignore` file\n\n## Audit scope\nReviewed the full `docs/` tree for navigation coverage, metadata,\ninternal links, stale version and Postgres support claims,\nrelease/install examples, ORM package pins, legacy API references,\ntracked artifacts, and custom configuration. Historical changelogs and\nintentional roadmap/coming-soon language were left unchanged.\n\n## Validation\n- `git diff --check`\n- `prettier --check docs/override.js docs/docs.json`\n- `mint validate`\n- `mint broken-links`\n- commit-time prek hooks",
          "timestamp": "2026-08-24T00:30:46-04:00",
          "tree_id": "2d0f812deb84ba49dff0c5211eba8bf7c857a5a7",
          "url": "https://github.com/paradedb/paradedb/commit/81e7d38632bacafad455cde5ecf09bb694416c37"
        },
        "date": 1787547315846,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "paradedb (single_topk) mean latency",
            "value": 1.928852729278901,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p50 latency",
            "value": 1.797,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p90 latency",
            "value": 2.447,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p95 latency",
            "value": 2.61,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p99 latency",
            "value": 2.786,
            "unit": "ms"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "fd1ec5841aef3738ac7339ec2a2b8c890ccdcdaf",
          "message": "docs: update documentation URLs (#6052)\n\n## Summary\n\n- update ParadeDB documentation links from docs.paradedb.com to\nparadedb.com/docs\n- preserve every existing documentation path and anchor\n\n## Verification\n\n- confirmed representative replacement URLs resolve successfully\n- git diff --check\n- verified no obsolete documentation URLs remain, excluding the\nintentional legacy-host redirect rules in the website repository",
          "timestamp": "2026-08-24T16:31:41-04:00",
          "tree_id": "51abed1b88f4c0d42118c854f804f1617ae27e7b",
          "url": "https://github.com/paradedb/paradedb/commit/fd1ec5841aef3738ac7339ec2a2b8c890ccdcdaf"
        },
        "date": 1787605153298,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "paradedb (single_topk) mean latency",
            "value": 1.9879533141595829,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p50 latency",
            "value": 1.845,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p90 latency",
            "value": 2.496,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p95 latency",
            "value": 2.598,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p99 latency",
            "value": 2.834,
            "unit": "ms"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "stuhood@paradedb.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "cd12adbf0bd58ef2e4307c19d15c55546a61f80c",
          "message": "feat: Add support for aggregate score joins. (#6024)\n\n## What\n\nAdds support for:\n* Aggregate score joins (without any Block-Max WAND dynamic filter\ncalculation, as described in #5301)\n* Calculation of scores for disjunctive joins under predicate tagging\n\n## Why\n\nAs described in #5961, we have been remiss in not tracking disjunctive\njoins in our benchmarks. But before tracking them, we need them to be\nfully supported, with accurate scores.\n\nThis change fills out support for ordering by a sum of scores, and adds\nsupport for scoring disjunctive joins.\n\n## How\n\n* Added expression matching for simple \"sum of scores\" patterns, and\nimproved planner warnings for other cases.\n* Fixed planning of `SegmentedTopK` to ensure that it is never\naccidentally pushed down through a join, or through a node with a schema\nthat it does not recognize.\n* Moved to lazily tagging and scoring blocks of rows in `BatchScanner`\nto avoid needing segment-sized score arrays.\n\n## Tests\n\nOverhauled `joinscan_sortby_score.sql` to use comprehensible per-table\nscores, and then validate that sums across those scores make sense under\nconjunction and disjunction.\n\nAdditionally, `join_distinct_expr.out` shows many changes due to the fix\nin `SegmentedTopK` planning.\n\n---------\n\nCo-authored-by: paradedb-github-app[bot] <282009505+paradedb-github-app[bot]@users.noreply.github.com>",
          "timestamp": "2026-08-24T15:14:35-07:00",
          "tree_id": "773179c2a83f9dbb9d24c2947f379b9f27648312",
          "url": "https://github.com/paradedb/paradedb/commit/cd12adbf0bd58ef2e4307c19d15c55546a61f80c"
        },
        "date": 1787613322565,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "paradedb (single_topk) mean latency",
            "value": 1.8444987576096303,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p50 latency",
            "value": 1.748,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p90 latency",
            "value": 2.218,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p95 latency",
            "value": 2.33,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p99 latency",
            "value": 2.542,
            "unit": "ms"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "stuhood@paradedb.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "4696391e4f03afde8eae256cd320a5843819fdc2",
          "message": "chore: Prepare `0.25.4`. (#6056)\n\nCo-authored-by: paradedb-github-app[bot] <282009505+paradedb-github-app[bot]@users.noreply.github.com>",
          "timestamp": "2026-08-24T16:48:04-07:00",
          "tree_id": "798879ff9d41f74fb2638d38af82b4187c792c45",
          "url": "https://github.com/paradedb/paradedb/commit/4696391e4f03afde8eae256cd320a5843819fdc2"
        },
        "date": 1787616644891,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "paradedb (single_topk) mean latency",
            "value": 2.275230616302168,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p50 latency",
            "value": 2.172,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p90 latency",
            "value": 2.857,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p95 latency",
            "value": 2.971,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p99 latency",
            "value": 3.115,
            "unit": "ms"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "stuhood@paradedb.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "4696391e4f03afde8eae256cd320a5843819fdc2",
          "message": "chore: Prepare `0.25.4`. (#6056)\n\nCo-authored-by: paradedb-github-app[bot] <282009505+paradedb-github-app[bot]@users.noreply.github.com>",
          "timestamp": "2026-08-24T16:48:04-07:00",
          "tree_id": "798879ff9d41f74fb2638d38af82b4187c792c45",
          "url": "https://github.com/paradedb/paradedb/commit/4696391e4f03afde8eae256cd320a5843819fdc2"
        },
        "date": 1787623460723,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "paradedb (single_topk) mean latency",
            "value": 1.9500562372859125,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p50 latency",
            "value": 1.815,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p90 latency",
            "value": 2.45,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p95 latency",
            "value": 2.569,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p99 latency",
            "value": 2.846,
            "unit": "ms"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "e0083a5846437586684ef5c5a8cc0898162610f6",
          "message": "test(stressgres): expand planner and topology coverage (#5889)\n\n## What\n\nOur Stressgres workloads were extremely outdated. I discovered this\nwhile adding \"plan shape assertions\" to them, which was recommended by\nCarl Sverre from Antithesis. They existed from pre-AggregateScan and\npre-JoinScan era, and even for the BaseScan they only covered some\ncases. This PR revamps them to be more up-to-date and have wider\ncoverage, which notably caught an MPP bug that had previously slipped\nthrough.\n\nThe performance alerts can be ignored for this PR, since this revamps\nthe suites altogether and previous values are now meaningless.\n\nCloses (partially) #5500. I commented future area of work on that issue,\nwhich is why I'm not closing it fully. More context:\n\n## Planner coverage\n\nThe single-node planner suite now asserts coverage for:\n\n- ParadeDB Base Scan, including normal, parallel, columnar, unordered\nTop K, key-ordered Top K, and score-ordered Top K paths\n- Aggregate Scan, including plain counts, grouped aggregates, and\n`pdb.agg`\n- JoinScan\n- PostgreSQL fallbacks, including Index Scan, Index Only Scan, Seq Scan,\nand Sort over a ParadeDB Base Scan\n\nAssertions are attached to the executor state that actually represents\neach optimized path, including Top K coverage under `TopKScanExecState`.\n\n## Suites and topologies\n\n- Renames suites around the behavior they validate:\n  - `single-node-planner-paths.toml`\n  - `bulk-update-merge-pressure.toml`\n  - `logical-replication-mixed-workload.toml`\n  - `logical-replication-fsm-merge-race.toml`\n- Adds `partitioned-table.toml` for partition pruning, parent/child scan\nplanning, aggregates, joins, and writes\n- Adds `logical-replication-multi-subscriber.toml` for one publisher\nwith two ParadeDB subscribers under mixed reads and writes\n- Narrows the FSM merge-race workload to paths relevant to that race\n- Removes the unused `vanilla-postgres.toml` suite and its references\n- Adds Antithesis entrypoints for every bundled suite, including\nindependent databases for the two logical-replication subscribers\n\nPhysical replication remains an enterprise-only topology:\n`paradedb-enterprise` already exercises its physical-replication and\ncombined physical/logical-replication suites in CI and Antithesis.\n\n## Tests\n\n`benchmark-stressgres` and `antithesis-stressgres` both pass error-free.",
          "timestamp": "2026-08-25T12:18:01-04:00",
          "tree_id": "c9794ea2e72745d63a40423e253a4bfc016ddf50",
          "url": "https://github.com/paradedb/paradedb/commit/e0083a5846437586684ef5c5a8cc0898162610f6"
        },
        "date": 1787676352917,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "paradedb (single_topk) mean latency",
            "value": 1.779005631440202,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p50 latency",
            "value": 1.683,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p90 latency",
            "value": 2.119,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p95 latency",
            "value": 2.348,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p99 latency",
            "value": 2.582,
            "unit": "ms"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "james.sewell@gmail.com",
            "name": "James Sewell",
            "username": "jamessewell"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "3540c09ec21744ca430ce76e6475201c4a292c6f",
          "message": "ci: run the benchmarker suite on the metal benchmark runner (#6069)\n\nMoves `benchmark-benchmarker` from `m8gd.2xlarge` to `m8gd.metal-24xl`,\nthe same whole-host runner the queries and stressgres benchmarks already\nuse (\"chosen for its performance consistency across hosts\").\n\n## Why\n\nThe tracked `hn-ci` latency series swings ~±15% between runs on\nidentical code: the last 12 runs on main range from 1.75 ms to 2.28 ms\nmean latency, including docs-only commits, and this tripped a false\nregression alert on 4696391e4 (a version bump). The workload itself is\nflat run-to-run on dedicated hardware, so the variance is per-run\nenvironment. A `.2xlarge` is a 1/12th slice of a shared metal host:\nvCPUs are dedicated, but memory bandwidth and the system-level cache are\nshared with whoever else is on the box that run.\n\nA whole host removes the noisy neighbours, matching what the other\nbenchmark suites concluded.\n\n## What doesn't change\n\n- Core pinning is untouched and works identically on the same Graviton4\nsilicon: DB on cores 0-3, pgbouncer on 4, k6 tasksetted onto 5-7.\n- Still no EBS data volume; the metal host's local NVMe serves the same\nrole.\n\n## Notes\n\n- The series may show a small step (likely faster) at the first metal\nrun, since the benchmark cores now get the chip's system-level cache to\nthemselves. Alerts only fire on regressions, so no false page, but the\nhistory chart will have a seam.\n- Runner cost per run goes up (~$5.40/hr vs ~$0.45/hr on-demand), the\nsame trade the other benchmark workflows accepted.\n\n**Testing:** add the `benchmark-benchmarker` label to run on this PR;\nre-dispatching on the same commit a few times afterwards will show\nwhether metal flattens the spread.\n\n🤖 Generated with [Claude Code](https://claude.com/claude-code)",
          "timestamp": "2026-08-25T12:23:36-04:00",
          "tree_id": "9854d10ba881cf37740c2db421a18e7982235e0b",
          "url": "https://github.com/paradedb/paradedb/commit/3540c09ec21744ca430ce76e6475201c4a292c6f"
        },
        "date": 1787676494676,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "paradedb (single_topk) mean latency",
            "value": 1.6012192193809085,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p50 latency",
            "value": 1.538,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p90 latency",
            "value": 1.862,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p95 latency",
            "value": 1.903,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p99 latency",
            "value": 2.065,
            "unit": "ms"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "stuhood@paradedb.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "1da2ad788cb80fb3ad83e5879d4cab8e0cd1a56f",
          "message": "chore: Post release version bump. (#6061)\n\nCo-authored-by: paradedb-github-app[bot] <282009505+paradedb-github-app[bot]@users.noreply.github.com>",
          "timestamp": "2026-08-25T09:38:27-07:00",
          "tree_id": "889fef766f10e2aa5724c8c3e655c8fd9fbdc170",
          "url": "https://github.com/paradedb/paradedb/commit/1da2ad788cb80fb3ad83e5879d4cab8e0cd1a56f"
        },
        "date": 1787677746287,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "paradedb (single_topk) mean latency",
            "value": 1.6543024742841226,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p50 latency",
            "value": 1.589,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p90 latency",
            "value": 1.921,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p95 latency",
            "value": 1.972,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p99 latency",
            "value": 2.263,
            "unit": "ms"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mdashti@gmail.com",
            "name": "Moe",
            "username": "mdashti"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "400f131982cfe708ae9b48e8fcf9f4a1839c3215",
          "message": "feat: partitioned index build execution (#6077)\n\n## Ticket(s) Closed\n\n- Closes #5737\n\n## What\n\nThis PR adds partitioned `CREATE INDEX` execution for an index that\ndeclares `partition_by`. Each worker routes its rows onto the leader's\nkd-tree boundaries and writes one segment per cell, so every segment of\na fresh index stays inside one cell's bounds in `partition_by` space,\nwith no merges across workers.\n\nIt carries @devdattatalele's commits from #6012 unchanged, plus the two\nfollow-ups from the last review round.\n\n## Why\n\nSegment pruning on `partition_by` needs segments that don't straddle\ncell boundaries. A parallel scan hands each worker an arbitrary slice of\nthe heap, so a worker has to route every row it sees, and a cross-worker\nmerge would undo the alignment.\n\n## How\n\nPhase 1: the scan callback routes each row with the shared `KdTree` and\nspills only `(pid, ctid)` to a worker-local `bytea` tuplesort. Phase 2:\nthe sorted records re-fetch rows through `HeapDocFetcher` under a reused\nbuffer pin and index them cell by cell, one `SerialIndexWriter` alive at\na time. The sort and the writer split the worker budget. A cell boundary\nfinalizes a segment; an overfull cell merges its own segments in passes\nof at most `CELL_MERGE_FANIN`. The drain walks HOT chain roots to the\nlive tail, so it indexes what the inline callback would have.\n`CONCURRENTLY` skips boundary planning and takes the regular path.\n\nThree refactors land first: the `bytea` tuplesort wrapper moves out of\n`keyset.rs`, `HeapDocFetcher` moves out of `index_memory_segment`, and\n`merge_now` splits out of `try_merge`.\n\nPersisting each cell's `partition_bounds` into segment stats is a\nfollow-up, together with the query-side pruning that reads it. Phase-2\nread amplification for a `partition_by` uncorrelated with heap order is\na known follow-up too (see the discussion on #6012).\n\n## Tests\n\n`#[pg_test]`s in `build_parallel.rs`\n\n---------\n\nCo-authored-by: Devdatta Talele <devtalele0@gmail.com>",
          "timestamp": "2026-08-25T11:45:29-07:00",
          "tree_id": "78e6bec5fab50325abb7a6bc7ac19b0b213c7bcf",
          "url": "https://github.com/paradedb/paradedb/commit/400f131982cfe708ae9b48e8fcf9f4a1839c3215"
        },
        "date": 1787684787883,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "paradedb (single_topk) mean latency",
            "value": 1.6567064751405656,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p50 latency",
            "value": 1.601,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p90 latency",
            "value": 1.905,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p95 latency",
            "value": 1.952,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p99 latency",
            "value": 2.108,
            "unit": "ms"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "stuhood@paradedb.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "39a2eec5d040dde203ab335772bea9a43e26f378",
          "message": "feat: Unify planner warnings GUCs, and allow for converting them into errors. (#6082)\n\n## What\n\nReplaces the separate boolean GUCs `paradedb.check_aggregate_scan` and\n`paradedb.check_topk_scan` with a single unified enum GUC,\n`paradedb.planner_warnings` (`off`, `warning`, `error`, defaulting to\n`warning`).\n\nWhen set to `error`, queries that cannot use an optimized ParadeDB scan\n(`basescan` / Top K, `aggregatescan`, or `joinscan`) raise an error\nduring execution instead of logging a warning.\n\n## Why\n\nIn CI and staging environments, users and tests need a strict mode where\nfallback to unoptimized execution paths immediately fails the query\nrather than logging warnings.\n\n## How\n\n- Replaced boolean GUCs with `paradedb.planner_warnings` (`off`,\n`warning`, `error`).\n- Added `ProcessUtility_hook` interception in\n`pg_search/src/postgres/planner_warnings.rs` to track `EXPLAIN` queries\nvia thread-local state, in order to allow `EXPLAIN` to be rendered\nrather than erroring.\n- Updated `emit_planner_warnings()` in\n`pg_search/src/postgres/planner_warnings.rs` to suppress messages when\n`off`, raise `pgrx::error!` when `error` (downgraded to `pgrx::warning!`\nduring `EXPLAIN`), and emit `pgrx::warning!` when `warning`.\n\n## Tests\n\nAdded regression tests in\n`pg_search/tests/pg_regress/sql/topk_validation.sql` verifying `off`,\n`warning`, and `error` modes, including `EXPLAIN` behavior under `error`\nmode.",
          "timestamp": "2026-08-25T14:59:37-07:00",
          "tree_id": "480847b82dfbf0a193e27ad23329081e9eaa55b1",
          "url": "https://github.com/paradedb/paradedb/commit/39a2eec5d040dde203ab335772bea9a43e26f378"
        },
        "date": 1787696454181,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "paradedb (single_topk) mean latency",
            "value": 1.658227498188698,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p50 latency",
            "value": 1.605,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p90 latency",
            "value": 1.944,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p95 latency",
            "value": 1.998,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p99 latency",
            "value": 2.109,
            "unit": "ms"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mithun.cy@gmail.com",
            "name": "Mithun Chicklore Yogendra",
            "username": "mithuncy"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "cd124da0cbd34938fdf65a773cdc0081c283cf67",
          "message": "perf: Initialize MPP search readers only where they execute (#6026)\n\n# Ticket(s) Closed\n\n- Related to #5999\n\n## What\n\nThe MPP leader opened every source index twice:\n\n1. At query begin, to capture and pin the segment manifest used to\npopulate the DSM.\n2. During DataFusion planning, to build the leader's\n`SearchIndexReader`.\n\nIt also opened fast fields for every segment when constructing\n`FFHelper`, including mutable segments the process never scanned.\n\nThis PR changes that:\n\n- `SearchIndexManifest` retains the components created during capture.\nThe leader builds its reader from those components using\n`SearchIndexReader::from_manifest`, avoiding the second index open while\npreserving the same searcher and segment view.\n- Tokenizers are registered into the managers already shared with the\ncaptured searcher.\n- `FFHelper` retains one `Searcher` and opens a segment's fast fields on\nfirst access. Mutable segments are materialized only in processes that\nactually scan them.\n- JoinScan injects manifests through the logical-plan codec.\nAggregateScan injects them directly into its providers. Serial scans and\nworker reader construction are unchanged.\n\n## Why\n\nThe second leader open became redundant after begin-time manifest\ncapture was added in #4311. Eager fast-field initialization also made\nevery process pay for segments assigned to other workers.\n\nReusing the manifest removes the redundant leader open. Lazy fast fields\navoid opening or materializing segments the process never reads.\n\n## Tests\n\n- `from_manifest_reuses_the_captured_open`: verifies zero additional\nindex opens, the same segment view, and tokenizer registration.\n- `decoded_provider_reuses_the_injected_manifest`: verifies manifest\nreuse through codec deserialization and provider planning.\n- `ffhelper_opens_only_the_segment_it_reads`: verifies only the accessed\nsegment opens and an unaccessed mutable segment remains cold.\n- `mpp_deferred_open_leader`: verifies leader-hosted leaves through\nJoinScan and AggregateScan, result parity, and continued MPP launch.\n- Local full regress and MPP integration results matched `main`.\n\n## Benchmark\n\nSame-session A/B against `12ddb8a9e`: release builds, PostgreSQL 17.7,\nApple Silicon, four MPP workers, one client, 20 warmups followed by five\nbatches of 100 transactions. Result hashes matched between base and\nhead.\n\n| Layout | Query | Base | Head | Change |\n| --- | --- | ---: | ---: | ---: |\n| Mixed: 5/21 segments | no text filter | 47.211 ms | 23.273 ms | −50.7%\n|\n| Mixed | `dragon` | 58.703 ms | 34.433 ms | −41.3% |\n| Mixed | `love` | 59.169 ms | 34.521 ms | −41.7% |\n| 128 immutable | all three | 13.99 / 16.48 / 16.83 ms | 13.6–14.6 /\n16.9 / 17.5 ms | within noise |\n\nThe mixed-layout improvement comes from avoiding mutable-segment\nmaterialization in processes that never scan those segments.\nImmutable-only layouts remain within noise.",
          "timestamp": "2026-08-25T15:48:45-07:00",
          "tree_id": "6319144c55e93583efe6b2261450a4e2f12e7dfc",
          "url": "https://github.com/paradedb/paradedb/commit/cd124da0cbd34938fdf65a773cdc0081c283cf67"
        },
        "date": 1787700426534,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "paradedb (single_topk) mean latency",
            "value": 1.6489413590510975,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p50 latency",
            "value": 1.562,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p90 latency",
            "value": 1.916,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p95 latency",
            "value": 2.002,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p99 latency",
            "value": 2.162,
            "unit": "ms"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "stuhood@paradedb.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "4f5f2980cd6628b4d280b7135328751299df8c06",
          "message": "chore: Revert \"feat: partitioned index build execution (#6077)\" (#6096)\n\nThis reverts commit 400f131982cfe708ae9b48e8fcf9f4a1839c3215, which\nregressed index builds.",
          "timestamp": "2026-08-25T20:43:22-07:00",
          "tree_id": "eaa24ed4310f534fcd047d4b758730e91108ca62",
          "url": "https://github.com/paradedb/paradedb/commit/4f5f2980cd6628b4d280b7135328751299df8c06"
        },
        "date": 1787717007185,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "paradedb (single_topk) mean latency",
            "value": 1.5951119034852554,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p50 latency",
            "value": 1.528,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p90 latency",
            "value": 1.81,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p95 latency",
            "value": 1.905,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p99 latency",
            "value": 2.018,
            "unit": "ms"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mdashti@gmail.com",
            "name": "Moe",
            "username": "mdashti"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "7496549a2eb2813a246fdb3ab4a55f4b88d335e0",
          "message": "fix: keep negative NUMERIC(p>18) values sortable in the index (#6053)\n\n## Ticket(s) Closed\n\n- Closes #6051\n\n## What\n\nThis PR fixes TopN ordering and range pushdown for negative values in\n`NumericBytes` fast fields (`NUMERIC` with precision > 18 or no\nprecision). Positive values were fine, and `NUMERIC(18,0)` was fine,\nsince that goes through `Numeric64`.\n\n## Why\n\n`decimal-bytes` stored a negative mantissa as bitwise-inverted BCD with\nno terminator. When a shorter mantissa is a prefix of a longer one, the\nshorter one sorts first in byte order, but it's the larger number:\n`-49990` (`B6 66`) came before `-49999` (`B6 66 6F`). TopN,\n`paradedb.range`, heap-filter pushdown, and `numrange` bounds all\ncompare those bytes directly.\n\n## How\n\nThe encoding fix is paradedb/decimal-bytes#19, released as `0.5.0`.\nNegative digits are nine's-complemented and end with a `0xFF`\nterminator. The old layout still decodes, and `Decimal::to_legacy_bytes`\nproduces it.\n\nThe two layouts don't sort together, so an index has to stay on one of\nthem. Same as #5245 did for datetimes, the choice follows the index's\n`created_by_version`. Indexes created before `0.25.5` keep writing and\nquerying the old layout, so existing rows and new rows stay comparable\n(and the ordering bug stays until `REINDEX`). Indexes built by `0.25.5`\nor later use the fixed layout. `query::numeric::decimal_to_index_bytes`\nis the one place that picks, and it's threaded through query terms,\nrange bounds, the index write path, and `numrange` bounds.\n\nPlease note that:\n- The gate is `0.25.5`, the version `main` carries after the `0.25.4`\nrelease. If that release doesn't ship this fix, the constant has to\nmove.\n- A JoinScan between a legacy index and a rebuilt one on a `NUMERIC(p >\n18)` key won't match negative values until both are rebuilt.\n\n## Tests\n\n- `issue_6051` regress test: TopN asc/desc, unbounded `numeric`,\n`numeric(30,10)` fractions with shared prefixes, `paradedb.range` (no\nrows outside the range), heap-filter pushdown, equality, rows inserted\nafter the build, and `numrange` containment and intersection. The\n`numeric(18,0)` column serves as the reference for the counts and\norderings.\n- Unit test in `query/numeric.rs` for the layout choice by version.\n- The legacy layout itself is covered in the crate. The regress test\nonly builds indexes with this version, so the legacy write path isn't\ncovered end to end here.\n\n---------\n\nCo-authored-by: paradedb-github-app[bot] <282009505+paradedb-github-app[bot]@users.noreply.github.com>",
          "timestamp": "2026-08-26T13:10:55-07:00",
          "tree_id": "da74bd02a844e8537199b293cd0cee2de98718f7",
          "url": "https://github.com/paradedb/paradedb/commit/7496549a2eb2813a246fdb3ab4a55f4b88d335e0"
        },
        "date": 1787776782146,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "paradedb (single_topk) mean latency",
            "value": 1.6325008777704861,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p50 latency",
            "value": 1.554,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p90 latency",
            "value": 1.893,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p95 latency",
            "value": 1.919,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p99 latency",
            "value": 2.077,
            "unit": "ms"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mdashti@gmail.com",
            "name": "Moe",
            "username": "mdashti"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "6966a36e4682c93c00dccd249774ab7e2e8c1702",
          "message": "feat: partitioned index build execution with one segment per partition (#6086)\n\n## Ticket(s) Closed\n\n- Closes #6081\n- Closes #5737\n\n## What\n\nThis PR re-lands the partitioned `CREATE INDEX` execution of #6077\n(reverted in #6096) with the build writing one segment per partition,\nwhichever workers scanned its rows, and prefetching heap blocks ahead of\nthe re-fetch drain. The first commit is #6077 unchanged; the two after\nit are the change.\n\n## Why\n\n#6077 drained each worker's own heap slice partition by partition, so a\nparallel build wrote `partitions x participants` segments. On the\nbenchmark the partitioned indexes went from 40 to 384 segments (48\npartitions x 8 participants), the indexes grew (+10%\n`stackoverflow_posts_idx`, +50% `users_idx` at 1M), and at 1M 48 of 126\nqueries got over 1.2x slower (joins up to 2.5x), none faster.\n\n## How\n\nThe two-pass plan from #5737. Phase 1 routes each scanned row on the\nleader's kd-tree and appends its ctid to a per-partition spill file: a\n`SharedFileSet` in the DSM for a parallel build (the leader initializes\nit in place before the workers spawn, so cleanup rides on the segment's\ndetach), plain temporary `BufFile`s for a serial one. Postgres decides\nwhich heap chunks a worker scans, so the labeling stays with the\nscanner.\n\nPhase 2 waits for every participant's spill, then gives each participant\na contiguous share of the partitions. The owner reads a partition's\nctids from all participants' files, sorts them through an `int8`\ntuplesort on a slice of the worker budget and re-fetches in ctid order,\n`maintenance_io_concurrency` blocks ahead. One writer is alive at a time\non the rest of the budget, so a partition merges only when it outgrows\nit, and then in one merge. `target_segment_count` is bounded at 1024, as\nis the GUC that overrides it.\n\nThere is no single global tuplesort because a parallel tuplesort lets\nonly the leader read the merge; per-partition files cost the same 8\nbytes per row.\n\n## Numbers\n\nFrom the benchmark runner, this head against `4f5f2980c` (the same index\ndefinitions built by the regular path). Ratios are partitioned over\nregular.\n\n| index | build 100k | build 1M | build 20M | size 20M |\n|---|---|---|---|---|\n| `stackoverflow_posts_idx` | 1.23x | 1.25x | 1.59x (4.12 vs 2.60 min) |\n1.05x |\n| `comments_idx` | 1.18x | 1.25x | 1.19x | 0.98x |\n| `users_idx` | 1.41x | 1.27x | 1.33x | 0.85x |\n| `badges_idx` (no `partition_by`) | 1.01x | 1.01x | 0.99x | 1.00x |\n\nThe segment counts are back to the target (48), so the sizes are flat.\nThe build itself is slower, and more so as the heap outgrows\n`shared_buffers`: the drain reads a heap block once per partition that\nhas a row in it, and with a key uncorrelated with heap order that is\nclose to `partitions` passes over the heap.\n\nQueries on the partitioned indexes:\n\n| queries (125 per suite) | 100k | 1M | 20M |\n|---|---|---|---|\n| median ratio | 1.00x | 1.00x | 1.00x |\n| over 1.2x | 1 | 10 | 19 |\n| under 0.8x | 2 | 4 | 7 |\n\n| slowest at 20M | 100k | 1M | 20M |\n|---|---|---|---|\n| `join_aggregate_sort - alternative 2` | 0.96x | 1.41x | 2.03x |\n| `join_aggregate_topk_count - alternative 2` | 1.00x | 1.34x | 1.94x |\n| `regex-and-heap` | 1.32x | 1.42x | 1.93x |\n| `join_aggregate_groupby - alternative 2` | 1.01x | 1.42x | 1.92x |\n| `join_aggregate_multi - alternative 2` | 1.00x | 1.41x | 1.83x |\n| `join_aggregate_count - alternative 2` | 1.00x | 1.39x | 1.78x |\n| `join_aggregate_disjunctive_count - alternative 2` | 0.91x | 1.20x |\n1.76x |\n| `join_top_k-score-desc-high-selectivity` | 1.19x | 1.30x | 1.54x |\n| `join_disjunctive_local_sort - alternative 2` | 0.88x | 1.07x | 1.36x\n|\n| `join_foreign_filter_local_sort` | 1.12x | 1.25x | 1.29x |\n\nThe `- alternative 2` rows are the `enable_range_partitioned_join`\nvariants: a key-aligned segment sends all of its rows to one range of\nthe shuffle. The others touch the heap per row, and a segment ordered by\nkey holds rows from all over the heap. Both are costs of the aligned\nlayout itself, not of the segment count, and stay until M3 uses the\nalignment for pruning and shuffle-free joins.\n\n## Follow-ups\n\n- Spilling the serialized document per partition in phase 1, instead of\nthe ctid, removes the re-fetch (one heap read, one sequential spill\nwrite and read) and the double expression evaluation. That is where most\nof the build gap is.\n\n## Tests\n\n`#[pg_test]`s in `build_parallel.rs`: one segment per partition with and\nwithout leader participation, a partition whose ctids spill through the\ntuplesort, a partition scanned by one participant only, and rows deleted\nby the building transaction (the drain used to drop them).\n\n---------\n\nCo-authored-by: Devdatta Talele <devtalele0@gmail.com>",
          "timestamp": "2026-08-26T16:55:12-07:00",
          "tree_id": "0783afbd7c75b1688130c52c5625aa657b806060",
          "url": "https://github.com/paradedb/paradedb/commit/6966a36e4682c93c00dccd249774ab7e2e8c1702"
        },
        "date": 1787789790895,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "paradedb (single_topk) mean latency",
            "value": 1.614741615109114,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p50 latency",
            "value": 1.558,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p90 latency",
            "value": 1.892,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p95 latency",
            "value": 1.94,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p99 latency",
            "value": 1.983,
            "unit": "ms"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ming.ying.nyc@gmail.com",
            "name": "Ming",
            "username": "rebasedming"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "0cc4b3cd1b329346523724d359ffde565388a9c2",
          "message": "feat: Efficiently intersect non-ParadeDB bitmap scans with ParadeDB custom scan (#6088)\n\n# Ticket(s) Closed\n\n- Closes #5702 \n\n## What\n\nToday, a query with a predicate that cannot be answered by the ParadeDB\nindex falls back to a heap filter, which evaluates the predicate against\nthe heap for every tuple emitted by the ParadeDB index. While correct,\nthis is extremely expensive over large result sets.\n\nA better path exists: if that predicate can be answered by another\nindex, and the index can produce a bitmap, we can attach the bitmap scan\nas a child of our custom scan and use the bitmap to cheaply reject\ntuples.\n\nTo illustrate:\n\n```sql\n-- SETUP\nCREATE TABLE items (id bigint, description text, location point);\nINSERT INTO items\nSELECT i, 'blue running shoes ' || i, point(i % 1000, i / 1000)\nFROM generate_series(1, 1000000) i;\nCREATE INDEX items_paradedb ON items USING paradedb (id, description) WITH (key_field = 'id');\nCREATE INDEX items_location ON items USING gist (location);\n```\n\nOn `main`, the following query which uses a GIST predicate touches 6k+\nbuffers:\n\n```sql\nEXPLAIN (ANALYZE, BUFFERS)\nSELECT count(*) FROM items\nWHERE description === 'shoes' AND location <@ circle(point(500, 500), 20);\n\n                                                                                                              QUERY PLAN                                                                                                              \n--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------\n Custom Scan (ParadeDB Aggregate Scan) on items  (cost=0.00..0.00 rows=1 width=8) (actual time=77.336..77.337 rows=1.00 loops=1)\n   Index: items_paradedb\n   Tantivy Query: {\"boolean\":{\"must\":[{\"with_index\":{\"query\":{\"term\":{\"field\":\"description\",\"value\":\"shoes\"}}}},{\"heap_filter\":{\"indexed_query\":\"all\",\"field_filters\":[{\"heap_filter\":\"(location <@ '<(500,500),20>'::circle)\"}]}}]}}\n     Applies to Aggregates: COUNT(*)\n     Aggregate Definition: {\"0\":{\"value_count\":{\"field\":\"ctid\",\"missing\":null}}}\n   Buffers: shared hit=6007\n Planning:\n   Buffers: shared hit=141 read=12\n Planning Time: 8.093 ms\n Execution Time: 77.498 ms\n(10 rows)\n```\n\nOn this branch, we drop down to~300 buffers (20x improvement):\n\n```sql\nEXPLAIN (ANALYZE, BUFFERS)\nSELECT count(*) FROM items\nWHERE description === 'shoes' AND location <@ circle(point(500, 500), 20);\n\n                                                                                                                                   QUERY PLAN                                                                                                                                    \n----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------\n Custom Scan (ParadeDB Aggregate Scan) on items  (cost=40.03..40.03 rows=1 width=8) (actual time=8.226..8.227 rows=1 loops=1)\n   Bitmap Intersection: items_location\n   Bitmap Exact Candidates: 1257\n   Bitmap Lossy Blocks: 0\n   Bitmap Recheck Blocks: 0\n   Index: items_paradedb\n   Tantivy Query: {\"boolean\":{\"must\":[{\"with_index\":{\"query\":{\"term\":{\"field\":\"description\",\"value\":\"shoes\"}}}},{\"heap_filter\":{\"indexed_query\":\"all\",\"field_filters\":[],\"recheck_filters\":[{\"heap_filter\":\"(location <@ '<(500,500),20>'::circle)\"}],\"uses_tid_bitmap\":true}}]}}\n     Applies to Aggregates: COUNT(*)\n     Aggregate Definition: {\"0\":{\"value_count\":{\"field\":\"ctid\",\"missing\":null}}}\n   Buffers: shared hit=295 read=25\n   ->  Bitmap Index Scan on items_location  (cost=0.00..39.78 rows=1000 width=0) (actual time=0.109..0.109 rows=1257 loops=1)\n         Index Cond: (location <@ '<(500,500),20>'::circle)\n         Buffers: shared read=25\n Planning:\n   Buffers: shared hit=119 read=26\n Planning Time: 2.676 ms\n Execution Time: 8.299 ms\n(17 rows)\n```\n\n## Why\n\nCustomer request\n\n## How\n\nImplemented:\n\n1. Find heap filters\nThe planner identifies AND-connected predicates that ParadeDB would\notherwise evaluate against heap rows.\n\n2. Require CTID sorting\nBitmap intersection is enabled only when the ParadeDB index is sorted by\nCTID.\n\n3. Choose a PostgreSQL index\nThe planner finds a profitable btree, GIN, or GiST index that covers a\nheap filter.\n\n4. Rewrite the query\nCovered filters are marked as bitmap-backed. Exact matches require\nrechecking only on lossy or recheck pages.\n\n5. Attach the bitmap plan\nThe PostgreSQL bitmap index scan becomes a child of the ParadeDB custom\nscan.\n\n6. Build the TIDBitmap\nThe leader executes the child index scan once and fills a native\nTIDBitmap. For parallel scans, it gets built inside DSA shared memory.\n\n7. Create per-segment cursors\nEach Tantivy segment receives its own forward-only cursor over the\nbitmap. Parallel workers attach to shared cursors.\n\n8. Stream the intersection\nTantivy matches are produced in CTID order and merged with the bitmap.\nMissing CTIDs are rejected immediately; exact matches avoid redundant\nheap-filter evaluation.\n\n## Tests\n\nSee regression test\n\n## Opens\n\n- #6089 BitmapAnd over multiple indexes\n- #6090 BitmapOr for indexable disjunctions\n- #6091 ScalarArrayOpExpr (`= ANY`) matching",
          "timestamp": "2026-08-26T18:17:42-07:00",
          "tree_id": "1e2571350f55e9e9ddeb4ad8aa41d913a74c8f26",
          "url": "https://github.com/paradedb/paradedb/commit/0cc4b3cd1b329346523724d359ffde565388a9c2"
        },
        "date": 1787795810708,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "paradedb (single_topk) mean latency",
            "value": 1.6077256809338543,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p50 latency",
            "value": 1.545,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p90 latency",
            "value": 1.847,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p95 latency",
            "value": 1.946,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p99 latency",
            "value": 2.217,
            "unit": "ms"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "stuhood@paradedb.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "a97a3589f65a9cbb3387eea9b942512eb219256f",
          "message": "chore: Switch to tracking fragments of upgrade SQL and release notes (#6079)\n\n## What\n\nTransitions ParadeDB release artifact generation (SQL extension\nmigration scripts and changelogs) from manually maintained, monolithic\nfiles to a PR-level fragment model.\n\n## Why\n\nTo remove confusion about which file to put upgrade snippets in, and to\nfurther automate release preparation.\n\n## How\n\nSee the changes to `RELEASE.md` and `CONTRIBUTING.md`\n\n## Tests\n\n- Tested `assemble_sql.py` and `assemble_changelog.py` locally.\n- CI has validated that the upgrade test properly picks up the change in\n`pg_search/sql/unreleased/5903.rename_test_table_proc.sql`\n\n---------\n\nCo-authored-by: paradedb-github-app[bot] <282009505+paradedb-github-app[bot]@users.noreply.github.com>",
          "timestamp": "2026-08-27T09:46:40-07:00",
          "tree_id": "da02ea71f79b24e45f5ea8ad7d0227faf2ea86a0",
          "url": "https://github.com/paradedb/paradedb/commit/a97a3589f65a9cbb3387eea9b942512eb219256f"
        },
        "date": 1787850443089,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "paradedb (single_topk) mean latency",
            "value": 1.6652746502518017,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p50 latency",
            "value": 1.588,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p90 latency",
            "value": 1.896,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p95 latency",
            "value": 2.047,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p99 latency",
            "value": 2.133,
            "unit": "ms"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "d7e80ccd5bad1d084256dc1035265c5877e18099",
          "message": "fix: use ParadeDB index terminology (#6127)\n\nUpdates current Rust documentation, planner diagnostics, user-facing\nerrors, and admin notices from “BM25 index” to “ParadeDB index.”\nRequired regression expectations are updated with the changed messages.\n\nAlso fixes the Antithesis monitoring query to select the `paradedb`\naccess method and describes the legacy `bm25` access method as a\nbackwards-compatible alias.\n\nDeliberately unchanged: upgrade SQL, compatibility test inputs,\nchangelogs, internal `bm25_*` identifiers, operator classes, and\nversion-pinned benchmark fixtures.\n\nValidation: all non-build commit hooks pass, including formatting,\nMarkdown, YAML, and Prettier. Rust build hooks are locally blocked by\nthe configured PostgreSQL SDK path\n`/Applications/Xcode.app/.../MacOSX26.5.sdk`, which does not exist on\nthis machine; CI remains authoritative.",
          "timestamp": "2026-08-27T15:35:13-04:00",
          "tree_id": "37b6269e7ca6417cfd98072ef0612cff8eff1860",
          "url": "https://github.com/paradedb/paradedb/commit/d7e80ccd5bad1d084256dc1035265c5877e18099"
        },
        "date": 1787860549517,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "paradedb (single_topk) mean latency",
            "value": 1.6272995679046314,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p50 latency",
            "value": 1.554,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p90 latency",
            "value": 1.858,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p95 latency",
            "value": 1.909,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p99 latency",
            "value": 2.081,
            "unit": "ms"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mdashti@gmail.com",
            "name": "Moe",
            "username": "mdashti"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "67a4c846171467072d0c68342b7f427eb464621b",
          "message": "feat: per-segment statistics component and range partition pruning (#6084)\n\n## Ticket(s) Closed\n\n- Closes #5735\n\n## What\n\nThis PR adds a per-segment `.stats` component and uses it for range\npartitioning.\n\nEvery immutable segment records the empirical `min`/`max` of each fast\nfield. A segment from a partitioned build also records its partition's\nbox. A range-partitioned join takes its split points from those boxes,\nand each partition searches only the segments its range can reach.\n\n## Why\n\nRange partitioning needs the value distribution and needs to skip\nsegments. Today both need segment reads at plan time and a scan of every\nsegment per partition. A segment already knows its extremes and, after a\npartitioned build, its partition. Query-level pruning outside range\npartitioning stays with #6078.\n\n## How\n\nA tantivy `SegmentPlugin` writes one small `.stats` file beside each\nsegment's other components, so the statistics follow the segment through\nwrites, merges, and vacuum. The file holds the empirical range of every\nfast field, taken from the segment's own fast columns, and, for a\npartitioned build, the box the kd-tree gave the partition. A merge\nrecomputes the range and keeps the box only when every source had one.\n\nStorage tracks the file as one more entry in the segment's metadata. A\nsegment without the file counts as unknown, so existing indexes keep\nworking and get the file when they merge.\n\nA partitioned build with an explicit `target_segment_count` keeps that\nmany partitions on a heap under the 15MB floor, which otherwise\ncollapses a build to one segment. A plain build keeps the floor.\n\nThe planner reads the boxes to get split points that line up with the\nsegments, and no longer samples: a join is range partitioned only when\nat least one side has split points, and a side without any is cut on the\nother side's. A segment without a box does not remove the split points,\nsince its own range places it at execution. The task count never exceeds\nwhat the split points seat, so no task is empty. At execution, each\npartition opens only the segments whose range can reach it. The range\nquery stays in place, so a kept segment costs time, not correctness.\n\n## Tests\n\n`#[pg_test]`s in `stats/tests.rs`, and `partitioned_stats_pruning.sql`\nfor the join shapes: split points on both sides, on one side, on\nneither, and more workers than the split points seat.\n\n## Benchmarks\n\nThe `alternative 2` join queries run with `enable_range_partitioned_join\n= on` over indexes built with `partition_by`. At 1M and 20M rows they\nare 0.32 to 0.83x of current `main` (0cc4b3cd1). Part of that is\nrecovering #6086, which made the same queries 1.3 to 2x slower on `main`\nbecause every task range-filtered all segments. Against `main` before\n#6086 (7496549a2) the PR is still 0.58 to 0.84x on the aggregate joins\nat 20M, and 0.14 to 0.47x on `join_conjunctive_score_sort`,\n`join_permissioned_search`, `join_semi_filter`, and `join_top_k`.",
          "timestamp": "2026-08-28T17:08:12-04:00",
          "tree_id": "80978c2022c9b675f958a09162adb0f1e6aacef1",
          "url": "https://github.com/paradedb/paradedb/commit/67a4c846171467072d0c68342b7f427eb464621b"
        },
        "date": 1787952495806,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "paradedb (single_topk) mean latency",
            "value": 1.625141405865986,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p50 latency",
            "value": 1.566,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p90 latency",
            "value": 1.854,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p95 latency",
            "value": 1.889,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p99 latency",
            "value": 2.098,
            "unit": "ms"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "stuhood@paradedb.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "85b9031af4edc412752a30594cdd3fdea30f9d22",
          "message": "chore: Remove post-release bump. (#6147)\n\nThe post-release bump of the version was an unnecessary artifact of\nusing `Cargo.toml` to drive the upgrade script testing.\n\nBy using a synthetic version to test upgrades, we can remove one commit\nand a bunch of complexity from the process.\n\n---------\n\nCo-authored-by: paradedb-github-app[bot] <282009505+paradedb-github-app[bot]@users.noreply.github.com>",
          "timestamp": "2026-08-28T15:53:58-07:00",
          "tree_id": "b1792653ee77a0fc02cdcef3457ada85dcd2560b",
          "url": "https://github.com/paradedb/paradedb/commit/85b9031af4edc412752a30594cdd3fdea30f9d22"
        },
        "date": 1787958848534,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "paradedb (single_topk) mean latency",
            "value": 1.607890913022355,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p50 latency",
            "value": 1.549,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p90 latency",
            "value": 1.832,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p95 latency",
            "value": 1.941,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p99 latency",
            "value": 1.977,
            "unit": "ms"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "2dbed3a3a873f700775168e59d6c683cd2d9a32a",
          "message": "ci: Fix benchmarker CI failure handling & cleanup duplicated actions and workflows (#6183)\n\n## Summary\n- consolidate source-building benchmark workflows on a single\n`Swatinem/rust-cache` owner for Cargo binaries and build artifacts\n- verify the cached `cargo-pgrx` version in both Benchmarker and\nStressgres, installing the pinned version only when missing or stale\n- notify `@pg_search-maintainers` in Slack when the Benchmarker workflow\nfails on a push\n\n## Root cause\n[Run\n33561565782](https://github.com/paradedb/paradedb/actions/runs/33561565782/job/100035036147)\nrestored `~/.cargo/bin/cargo-pgrx` through `Swatinem/rust-cache`, while\na second dedicated `cargo-pgrx` cache reported a miss. The subsequent\ninstall failed because the binary already existed. Removing the\noverlapping cache ownership prevents that inconsistent state.\n\nThe query benchmark workflow does not install `cargo-pgrx`; it uses a\nprepared benchmark cluster, so there is no equivalent cache path to\nchange there.\n\n## Validation\n- `git diff --check`\n- parsed all three modified YAML files with Ruby YAML",
          "timestamp": "2026-09-01T16:13:19-07:00",
          "tree_id": "9b4db32eb6476fe2a4f2e5a84707d9d6dee78521",
          "url": "https://github.com/paradedb/paradedb/commit/2dbed3a3a873f700775168e59d6c683cd2d9a32a"
        },
        "date": 1788306120019,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "paradedb (single_topk) mean latency",
            "value": 1.6256989617486215,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p50 latency",
            "value": 1.572,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p90 latency",
            "value": 1.851,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p95 latency",
            "value": 1.873,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p99 latency",
            "value": 2.007,
            "unit": "ms"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mithun.cy@gmail.com",
            "name": "Mithun Chicklore Yogendra",
            "username": "mithuncy"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "8a85104bf07d4f6510bb189dec2155c7a7ee4af7",
          "message": "fix: respect PostgreSQL parallel mode for MPP scans (#6181)\n\n# Ticket(s) Closed\n\n- Closes #6157\n\n## What\n\nGate MPP producer-worker launch on PostgreSQL's statement-wide\nparallel-mode decision (`PlannerGlobal.parallelModeOK`). A DataFusion\ncustom scan under `ModifyTable` now plans and runs serially instead of\nfailing with `cannot assign transaction IDs during a parallel\noperation`. The scan is still selected and still runs on DataFusion.\n\n## Why\n\nMPP launches its own producers via `EnterParallelMode()`, bypassing\nPostgreSQL's per-statement decision. Under `INSERT ... SELECT` that\ndecision is false, and the heap write's `AssignTransactionId` errors\nonce the backend is in parallel mode. The subplan's own query level\ncan't detect this — a SELECT under `ModifyTable` is still `CMD_SELECT`;\nonly the statement-wide flag reflects the enclosing INSERT.\n\nSide effect of adopting PG's decision: MPP is also suppressed for\ncursor-driven queries, queries containing `PARALLEL UNSAFE` functions,\nand modifying CTEs — all cases where entering parallel mode was already\nunsafe. pg_search's `@@@`, `score`, and `snippet*` are `parallel_safe`,\nso ordinary search queries are unaffected.\n\n## How\n\nReview in this order:\n\n1. `mpp/glue.rs` — `query_allows_parallel_mode(&PlannerInfo)`: reads\n`parallelModeOK`, captured once at path creation by each scan.\n2. `mpp/launch.rs` — `mpp_eligible(mpp_query_safe, &RelNode)`: the\nsingle gate (statement safety + worker budget + min-rows), used by\n`AggregateScan::prepare_mpp`, `JoinScan::begin_custom_scan`, and both\nplain-EXPLAIN rebuilds, so rendered plans match execution.\n3. `{aggregatescan,joinscan}/privdat.rs` + `scan_state.rs` — the flag is\nserialized in private data (`#[serde(default)]`, fail-closed) and copied\ninto scan state.\n4. `joinscan/mod.rs` — `bake_logical_plan` folds `!mpp_query_safe` into\n`force_serial` at the only place plan bytes are produced, so no caller\ncan bake MPP provider metadata (`mpp_source_idx`) for an unsafe\nstatement.\n\n## Tests\n\n- New `mpp_worker_sizing` regress cases: `INSERT ... SELECT` over both\nscans (serial plan shape + correct inserted rows), and a\n`force_generic_plan` prepared `INSERT` covering JoinScan's exec-time\nrebake with a runtime `Param`.\n- Full regress: 333/333. Integration (`tests` + `tokenizers`): 619\npassed, 0 failed. Unit/`#[pg_test]`: 339 passed, 0 failed.",
          "timestamp": "2026-09-02T11:38:09+05:30",
          "tree_id": "540c7951897fd1e08951af9b75d85f0a2781a674",
          "url": "https://github.com/paradedb/paradedb/commit/8a85104bf07d4f6510bb189dec2155c7a7ee4af7"
        },
        "date": 1788330512297,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "paradedb (single_topk) mean latency",
            "value": 1.6018258926167237,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p50 latency",
            "value": 1.537,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p90 latency",
            "value": 1.894,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p95 latency",
            "value": 1.974,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p99 latency",
            "value": 2.018,
            "unit": "ms"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "46780009+sahilchug@users.noreply.github.com",
            "name": "sahil",
            "username": "sahilchug"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "f583d7e93a8235920707b29abd1ec929be41f6b5",
          "message": "feat: push down GROUP BY DATE(timestamp) to the aggregate custom scan (#5936)\n\n# Ticket(s) Closed\n\n- Closes #4082 \n\n## What\nPushes `GROUP BY DATE(ts)` and `GROUP BY ts::date` into DataFusion\nbackend when `ts` is a bare timestamp without timezone.\n\nThe DataFusion path supports:\n\n- handles -infinity, infinity\n- preserves NULL date values correctly\n- handles `DATE()` combined with other grouping columns \n- supports both serial and MPP execution, producing the same grouped\nresults when work is distributed across multiple workers and index\nsegments\n\nShapes outside the safe boundary refuse pushdown with a named reason and\nfall sback to native Postgres execution :\n\n- `DATE(timestamptz)` — the result depends on the session `TimeZone`\n- `DATE()` over a cast (e.g. `DATE(text_col::timestamp)`) — the argument\n      must be a bare timestamp column  \n\n**Note**: support for Top K query via DataFusion path will be a follow\nup PR\n\n## Why\n  \nThis is a rework of #4918, which had 2 correctness bugs:\n     - NULL timestamp rows silently dropped fro results\n- `DATE(timestampz)` was pushed down with wrong timezone semantics.\n\nAlso an earlier attempt was done using Tantivy Path for predicate push\ndown but it has limitations:\n\n- Tantivy converts the stored `i64` timestamp microseconds to `f64` when\ncalculating histogram buckets. For dates far from the PostgreSQL epoch,\nthis loses microsecond precision and can move timestamps near midnight\ninto the wrong day.\n- PostgreSQL's `infinity` and `-infinity` timestamp sentinels cannot be\nrepresented correctly after the conversion to `f64`.\n- `ORDER BY ... LIMIT` / TopK queries were already routed toward\nDataFusion, so the histogram implementation did not help with that query\nshape\n  \n\n## How\n\n- Detects `Date(timestamp)` in `GROUP BY` clause and routes the query to\nDataFusion backend\n- Validates that `Date(timestamp)` is a bare timestamp column without\ntimezone\n- Adds a Grouping transform to `JoinGroupColumn` metadata\n- Applies a DataFusion scalar UDF `TimestampToDateUdf` that converts\ntimestamps to Arrow `Date32` values and preserves `NULL` and handles\n`-infinity` and `infinity` sentinels\n- Updates the Arrow `Date32` projection to map the internal\n`i32::MIN/MAX` sentinels back to PostgreSQL `-infinity` and `infinity`\ninstead of treating them as finite day counts.\n\n## Tests\n\n- Integration and `pg_regress` tests cover basic date grouping, NULL\ngroups, TopK, aggregate `FILTER`, multi-column grouping, timestamp\nboundaries, infinities, and fallback for unsupported expressions.\n- MPP regression tests verify that serial and distributed DataFusion\nexecution produce the same results as native PostgreSQL across multiple\nindex segments.",
          "timestamp": "2026-09-02T11:56:33-04:00",
          "tree_id": "39176d36a4ff5b7395132a8049b49145d589d53e",
          "url": "https://github.com/paradedb/paradedb/commit/f583d7e93a8235920707b29abd1ec929be41f6b5"
        },
        "date": 1788365829625,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "paradedb (single_topk) mean latency",
            "value": 1.6879536044467043,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p50 latency",
            "value": 1.611,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p90 latency",
            "value": 1.968,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p95 latency",
            "value": 2.063,
            "unit": "ms"
          },
          {
            "name": "paradedb (single_topk) p99 latency",
            "value": 2.151,
            "unit": "ms"
          }
        ]
      }
    ]
  }
}