window.BENCHMARK_DATA = {
  "lastUpdate": 1752237111493,
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
    ],
    "pg_search single-server.toml Performance - Other Metrics": [
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
        "date": 1752237097892,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.7477746,
            "unit": "median cpu",
            "extra": "avg cpu: 6.885071801192753, max cpu: 23.738873, count: 58418"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 97.9375,
            "unit": "median mem",
            "extra": "avg mem: 91.59658617367506, max mem: 100.84765625, count: 58418"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.698972,
            "unit": "median cpu",
            "extra": "avg cpu: 4.744757178684784, max cpu: 9.538003, count: 58418"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 83,
            "unit": "median mem",
            "extra": "avg mem: 76.55455082337636, max mem: 86.0, count: 58418"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.740741,
            "unit": "median cpu",
            "extra": "avg cpu: 6.880863164202392, max cpu: 24.242424, count: 58418"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 98.99609375,
            "unit": "median mem",
            "extra": "avg mem: 92.26902158795234, max mem: 101.1953125, count: 58418"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.692082,
            "unit": "median cpu",
            "extra": "avg cpu: 4.477421035393748, max cpu: 4.7832584, count: 58418"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 95.23828125,
            "unit": "median mem",
            "extra": "avg mem: 88.48916001917217, max mem: 96.58203125, count: 58418"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.7267356,
            "unit": "median cpu",
            "extra": "avg cpu: 6.611805182822304, max cpu: 19.783615, count: 116836"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 97.65625,
            "unit": "median mem",
            "extra": "avg mem: 92.39853566109761, max mem: 106.796875, count: 116836"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 7699,
            "unit": "median block_count",
            "extra": "avg block_count: 6908.741500907255, max block_count: 7699.0, count: 58418"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 118,
            "unit": "median segment_count",
            "extra": "avg segment_count: 117.43637235098771, max segment_count: 369.0, count: 58418"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.7058825,
            "unit": "median cpu",
            "extra": "avg cpu: 5.526272854948317, max cpu: 14.328358, count: 58418"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 102.63671875,
            "unit": "median mem",
            "extra": "avg mem: 97.33222670292119, max mem: 110.59765625, count: 58418"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 9.552238,
            "unit": "median cpu",
            "extra": "avg cpu: 13.272652448144251, max cpu: 32.844578, count: 58418"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 77.2421875,
            "unit": "median mem",
            "extra": "avg mem: 72.8856530200452, max mem: 83.0703125, count: 58418"
          }
        ]
      }
    ],
    "pg_search wide-table.toml Performance - Other Metrics": [
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
        "date": 1752237110637,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.879055,
            "unit": "median cpu",
            "extra": "avg cpu: 19.388316394131813, max cpu: 47.97601, count: 59161"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 173.7109375,
            "unit": "median mem",
            "extra": "avg mem: 171.8246386845853, max mem: 175.78125, count: 59161"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 23776,
            "unit": "median block_count",
            "extra": "avg block_count: 21748.436115008197, max block_count: 27509.0, count: 59161"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 72,
            "unit": "median segment_count",
            "extra": "avg segment_count: 75.45557039265732, max segment_count: 169.0, count: 59161"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 9.4395275,
            "unit": "median cpu",
            "extra": "avg cpu: 9.715757226466351, max cpu: 28.828829, count: 59161"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 165.546875,
            "unit": "median mem",
            "extra": "avg mem: 157.37672840057218, max mem: 173.796875, count: 59161"
          }
        ]
      }
    ]
  }
}