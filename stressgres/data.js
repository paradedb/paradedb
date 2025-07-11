window.BENCHMARK_DATA = {
  "lastUpdate": 1752237089833,
  "repoUrl": "https://github.com/paradedb/paradedb",
  "entries": {
    "pg_search wide-table.toml Performance - TPS": [
      {
        "commit": {
          "author": {
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr",
            "email": "eebbrr@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752237074771,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 37.825549325934574,
            "unit": "median tps",
            "extra": "avg tps: 38.10476012239314, max tps: 39.32428486805184, count: 59161"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 202.7817098654163,
            "unit": "median tps",
            "extra": "avg tps: 206.11763802798248, max tps: 243.09219735024135, count: 59161"
          }
        ]
      }
    ],
    "pg_search single-server.toml Performance - TPS": [
      {
        "commit": {
          "author": {
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr",
            "email": "eebbrr@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752237088919,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 390.61062538021366,
            "unit": "median tps",
            "extra": "avg tps: 394.6734848646864, max tps: 587.3638830351085, count: 58418"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2516.447798620683,
            "unit": "median tps",
            "extra": "avg tps: 2452.522804628353, max tps: 2783.7517386232435, count: 58418"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 391.61871424174217,
            "unit": "median tps",
            "extra": "avg tps: 397.22787805247185, max tps: 592.9112646606859, count: 58418"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 386.9820676683288,
            "unit": "median tps",
            "extra": "avg tps: 387.3600215614747, max tps: 486.7151347332137, count: 58418"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 265.8198623483887,
            "unit": "median tps",
            "extra": "avg tps: 264.24797005831863, max tps: 275.9731733626332, count: 116836"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 244.3693135727048,
            "unit": "median tps",
            "extra": "avg tps: 241.1945628713284, max tps: 247.34549920140702, count: 58418"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 9.438228530430472,
            "unit": "median tps",
            "extra": "avg tps: 14.39237952297812, max tps: 1467.1490652793304, count: 58418"
          }
        ]
      }
    ]
  }
}