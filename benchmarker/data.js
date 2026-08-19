window.BENCHMARK_DATA = {
  "lastUpdate": 1787153970909,
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
      }
    ]
  }
}