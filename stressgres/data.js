window.BENCHMARK_DATA = {
  "lastUpdate": 1778264631351,
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
        "date": 1778264595193,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 469.5038535362138,
            "unit": "median tps",
            "extra": "avg tps: 469.57655091860175, max tps: 565.4828642755789, count: 55529"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3056.0037107653116,
            "unit": "median tps",
            "extra": "avg tps: 3046.9509948172113, max tps: 3073.463935718692, count: 55529"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 469.41730571191096,
            "unit": "median tps",
            "extra": "avg tps: 468.03376846877063, max tps: 591.5847382346509, count: 55529"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 393.4925510721222,
            "unit": "median tps",
            "extra": "avg tps: 392.79560334475957, max tps: 429.0520562602851, count: 55529"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 3025.744137907696,
            "unit": "median tps",
            "extra": "avg tps: 3032.77067236461, max tps: 3372.381008908482, count: 111058"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 2030.8355824308394,
            "unit": "median tps",
            "extra": "avg tps: 2024.786751386968, max tps: 2039.925185247479, count: 55529"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 94.36971552254222,
            "unit": "median tps",
            "extra": "avg tps: 95.96977092566719, max tps: 853.6232465511487, count: 55529"
          }
        ]
      }
    ]
  }
}