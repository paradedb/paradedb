window.BENCHMARK_DATA = {
  "lastUpdate": 1778259543853,
  "repoUrl": "https://github.com/paradedb/paradedb",
  "entries": {
    "pg_search single-server.toml Performance - TPS": [
      {
        "commit": {
          "author": {
            "name": "paradedb[bot]",
            "username": "paradedb-bot",
            "email": "developers@paradedb.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "5ce8f7cabc2743985d08edbeaffb38b3c62f6826",
          "message": "chore: Prepare `0.21.16`. (#4436)\n\n# Description\nBackport of #4434 to `0.21.x`.\n\n---------\n\nCo-authored-by: Stu Hood <stuhood@paradedb.com>\nCo-authored-by: Stu Hood <stuhood@gmail.com>",
          "timestamp": "2026-03-20T02:44:33Z",
          "url": "https://github.com/paradedb/paradedb/commit/5ce8f7cabc2743985d08edbeaffb38b3c62f6826"
        },
        "date": 1778259506168,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 490.5710756471139,
            "unit": "median tps",
            "extra": "avg tps: 498.8295845084129, max tps: 625.6157497335017, count: 55379"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2969.6033192669024,
            "unit": "median tps",
            "extra": "avg tps: 2951.5922725321684, max tps: 2976.380240088397, count: 55379"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 438.82063428541204,
            "unit": "median tps",
            "extra": "avg tps: 447.33313110783365, max tps: 611.7368234776981, count: 55379"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 379.45235210619603,
            "unit": "median tps",
            "extra": "avg tps: 387.566257802478, max tps: 449.17098756679735, count: 55379"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 3302.463252812455,
            "unit": "median tps",
            "extra": "avg tps: 3307.209908929097, max tps: 3354.7733782047308, count: 110758"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 2084.163445507118,
            "unit": "median tps",
            "extra": "avg tps: 2081.85881173375, max tps: 2097.9385046883867, count: 55379"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 80.3057006941676,
            "unit": "median tps",
            "extra": "avg tps: 136.99731061066166, max tps: 486.00166065091577, count: 55379"
          }
        ]
      }
    ]
  }
}