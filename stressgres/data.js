window.BENCHMARK_DATA = {
  "lastUpdate": 1763941881413,
  "repoUrl": "https://github.com/paradedb/paradedb",
  "entries": {
    "pg_search single-server.toml Performance - TPS": [
      {
        "commit": {
          "author": {
            "email": "stuhood@paradedb.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "committer": {
            "email": "stuhood@gmail.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "distinct": true,
          "id": "099c58ef752c337320d3e06a685cf80ba86e533a",
          "message": "chore: Prepare `0.19.9`. (#3604)\n\nPrepare `0.19.9`.",
          "timestamp": "2025-11-23T15:34:49-08:00",
          "tree_id": "108a1316f3541a472d93c2c75d3a050eb585ba61",
          "url": "https://github.com/paradedb/paradedb/commit/099c58ef752c337320d3e06a685cf80ba86e533a"
        },
        "date": 1763941879025,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 145.55320856357247,
            "unit": "median tps",
            "extra": "avg tps: 162.1834713371906, max tps: 599.9327510676216, count: 55475"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3166.568078774087,
            "unit": "median tps",
            "extra": "avg tps: 3139.6648904855997, max tps: 3175.2762284966816, count: 55475"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 144.00076971107828,
            "unit": "median tps",
            "extra": "avg tps: 160.59372935389624, max tps: 621.466521098898, count: 55475"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 124.7347684341972,
            "unit": "median tps",
            "extra": "avg tps: 139.06115491738797, max tps: 455.5622384005028, count: 55475"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 3389.887335027361,
            "unit": "median tps",
            "extra": "avg tps: 3385.243831874746, max tps: 3432.1868995077116, count: 110950"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 2119.2357741234373,
            "unit": "median tps",
            "extra": "avg tps: 2107.548878633217, max tps: 2139.8052414514104, count: 55475"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 38.557477603385635,
            "unit": "median tps",
            "extra": "avg tps: 64.02303712895271, max tps: 702.6690885327918, count: 55475"
          }
        ]
      }
    ]
  }
}