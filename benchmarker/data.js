window.BENCHMARK_DATA = {
  "lastUpdate": 1787266374008,
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
      }
    ]
  }
}