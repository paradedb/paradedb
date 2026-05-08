window.BENCHMARK_DATA = {
  "lastUpdate": 1778264669698,
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
    ],
    "pg_search single-server.toml Performance - Other Metrics": [
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
        "date": 1778264633264,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.7058825,
            "unit": "median cpu",
            "extra": "avg cpu: 6.21519925094482, max cpu: 23.575638, count: 55529"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 57.10546875,
            "unit": "median mem",
            "extra": "avg mem: 56.920296292252694, max mem: 67.39453125, count: 55529"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.51732564154388, max cpu: 9.248554, count: 55529"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 33.87109375,
            "unit": "median mem",
            "extra": "avg mem: 33.52545622276198, max mem: 35.87109375, count: 55529"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.6966734,
            "unit": "median cpu",
            "extra": "avg cpu: 6.138716325749014, max cpu: 19.692308, count: 55529"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 57.375,
            "unit": "median mem",
            "extra": "avg mem: 57.23053459397342, max mem: 67.68359375, count: 55529"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.59872318222867, max cpu: 9.375, count: 55529"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 56.5859375,
            "unit": "median mem",
            "extra": "avg mem: 56.036684730613736, max mem: 66.97265625, count: 55529"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.577734159712496, max cpu: 9.706775, count: 111058"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 45.33984375,
            "unit": "median mem",
            "extra": "avg mem: 45.13387092780349, max mem: 55.63671875, count: 111058"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 1662,
            "unit": "median block_count",
            "extra": "avg block_count: 1667.3625132813486, max block_count: 2961.0, count: 55529"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 8,
            "unit": "median segment_count",
            "extra": "avg segment_count: 9.201354247330224, max segment_count: 17.0, count: 55529"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.5479995600119825, max cpu: 9.257474, count: 55529"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 48.13671875,
            "unit": "median mem",
            "extra": "avg mem: 47.9234766405842, max mem: 58.25390625, count: 55529"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.833837,
            "unit": "median cpu",
            "extra": "avg cpu: 3.7113656248965303, max cpu: 4.833837, count: 55529"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 49.5625,
            "unit": "median mem",
            "extra": "avg mem: 49.8654714062697, max mem: 61.2734375, count: 55529"
          }
        ]
      }
    ]
  }
}