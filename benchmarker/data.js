window.BENCHMARK_DATA = {
  "lastUpdate": 1787159356744,
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
      }
    ]
  }
}