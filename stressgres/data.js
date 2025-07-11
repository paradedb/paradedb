window.BENCHMARK_DATA = {
  "lastUpdate": 1752239195413,
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
      },
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
        "date": 1752238150078,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 36.76214151689536,
            "unit": "median tps",
            "extra": "avg tps: 37.02609090387134, max tps: 38.15774112652048, count: 59180"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 202.66915644497536,
            "unit": "median tps",
            "extra": "avg tps: 206.94870979720113, max tps: 252.9890260121063, count: 59180"
          }
        ]
      },
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
        "date": 1752239187807,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 37.82234014574883,
            "unit": "median tps",
            "extra": "avg tps: 38.186195096009584, max tps: 39.58086860017298, count: 59153"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 201.74464204799588,
            "unit": "median tps",
            "extra": "avg tps: 205.23224918969623, max tps: 233.7369525946125, count: 59153"
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
      },
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
        "date": 1752238143945,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 424.8040573830263,
            "unit": "median tps",
            "extra": "avg tps: 430.26262891612987, max tps: 625.4019075870007, count: 58429"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2669.4671215626186,
            "unit": "median tps",
            "extra": "avg tps: 2585.349611022355, max tps: 3103.1823057924116, count: 58429"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 425.5931240671708,
            "unit": "median tps",
            "extra": "avg tps: 431.9001976013556, max tps: 636.9263162649956, count: 58429"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 428.99259209280535,
            "unit": "median tps",
            "extra": "avg tps: 430.5479095073583, max tps: 560.229864554346, count: 58429"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 293.9350646225956,
            "unit": "median tps",
            "extra": "avg tps: 292.02900262617015, max tps: 303.6400647409563, count: 116858"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 269.3806150354518,
            "unit": "median tps",
            "extra": "avg tps: 266.1746204223828, max tps: 275.8882036791982, count: 58429"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 14.308464214984275,
            "unit": "median tps",
            "extra": "avg tps: 20.10877557684736, max tps: 1580.3180864244357, count: 58429"
          }
        ]
      },
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
        "date": 1752239192222,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 419.9507849883561,
            "unit": "median tps",
            "extra": "avg tps: 425.04714770413767, max tps: 630.6280458892587, count: 58487"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2504.301465772778,
            "unit": "median tps",
            "extra": "avg tps: 2359.1891961968727, max tps: 3045.2768244477948, count: 58487"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 415.51375897220925,
            "unit": "median tps",
            "extra": "avg tps: 421.2410175397073, max tps: 608.7548401627973, count: 58487"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 418.02264178386986,
            "unit": "median tps",
            "extra": "avg tps: 420.50438026908574, max tps: 523.1861176539895, count: 58487"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 287.0577202647603,
            "unit": "median tps",
            "extra": "avg tps: 291.91354072135937, max tps: 321.6306651395404, count: 116974"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 259.018805817646,
            "unit": "median tps",
            "extra": "avg tps: 253.54578824946495, max tps: 281.59551880155203, count: 58487"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 14.454366628858304,
            "unit": "median tps",
            "extra": "avg tps: 20.92424510888961, max tps: 1789.8276932879673, count: 58487"
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
      },
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
        "date": 1752238182794,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.740741,
            "unit": "median cpu",
            "extra": "avg cpu: 6.606133071552365, max cpu: 28.152493, count: 58429"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 82.921875,
            "unit": "median mem",
            "extra": "avg mem: 81.84320072224409, max mem: 87.34375, count: 58429"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.7197638,
            "unit": "median cpu",
            "extra": "avg cpu: 4.759502174083395, max cpu: 9.595202, count: 58429"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 70.9296875,
            "unit": "median mem",
            "extra": "avg mem: 69.13551115456366, max mem: 73.1796875, count: 58429"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.740741,
            "unit": "median cpu",
            "extra": "avg cpu: 6.606299078939965, max cpu: 23.952095, count: 58429"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 82.3984375,
            "unit": "median mem",
            "extra": "avg mem: 81.69680937548135, max mem: 87.35546875, count: 58429"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.7058825,
            "unit": "median cpu",
            "extra": "avg cpu: 4.259818066665957, max cpu: 4.797601, count: 58429"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 77.6796875,
            "unit": "median mem",
            "extra": "avg mem: 77.43897762134385, max mem: 83.1796875, count: 58429"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.733728,
            "unit": "median cpu",
            "extra": "avg cpu: 6.396690470046422, max cpu: 24.615385, count: 116858"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 88.1796875,
            "unit": "median mem",
            "extra": "avg mem: 87.42538590506213, max mem: 96.4296875, count: 116858"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 5727,
            "unit": "median block_count",
            "extra": "avg block_count: 5675.860754077598, max block_count: 6064.0, count: 58429"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 117,
            "unit": "median segment_count",
            "extra": "avg segment_count: 116.52431155761694, max segment_count: 265.0, count: 58429"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.7197638,
            "unit": "median cpu",
            "extra": "avg cpu: 5.324219863529958, max cpu: 14.180207, count: 58429"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 89.73828125,
            "unit": "median mem",
            "extra": "avg mem: 89.05469044160434, max mem: 96.11328125, count: 58429"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 14.138439,
            "unit": "median cpu",
            "extra": "avg cpu: 14.647324940570195, max cpu: 28.828829, count: 58429"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 70.91796875,
            "unit": "median mem",
            "extra": "avg mem: 68.47138748256431, max mem: 72.2734375, count: 58429"
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
      },
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
        "date": 1752238165895,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.879055,
            "unit": "median cpu",
            "extra": "avg cpu: 19.4761867534322, max cpu: 47.83259, count: 59180"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 178.55078125,
            "unit": "median mem",
            "extra": "avg mem: 177.66108792824858, max mem: 178.55078125, count: 59180"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 23773,
            "unit": "median block_count",
            "extra": "avg block_count: 21655.293612706995, max block_count: 27284.0, count: 59180"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 72,
            "unit": "median segment_count",
            "extra": "avg segment_count: 74.97571814802298, max segment_count: 186.0, count: 59180"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 9.4395275,
            "unit": "median cpu",
            "extra": "avg cpu: 9.702628536523862, max cpu: 33.48281, count: 59180"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 166.51171875,
            "unit": "median mem",
            "extra": "avg mem: 157.74049443963332, max mem: 173.6640625, count: 59180"
          }
        ]
      },
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
        "date": 1752239194558,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.906942,
            "unit": "median cpu",
            "extra": "avg cpu: 19.369188323011837, max cpu: 48.1203, count: 59153"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 170.265625,
            "unit": "median mem",
            "extra": "avg mem: 168.6761738362805, max mem: 176.1796875, count: 59153"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 24145,
            "unit": "median block_count",
            "extra": "avg block_count: 21959.312731391477, max block_count: 27383.0, count: 59153"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 72,
            "unit": "median segment_count",
            "extra": "avg segment_count: 75.63308707926902, max segment_count: 170.0, count: 59153"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 9.453471,
            "unit": "median cpu",
            "extra": "avg cpu: 9.671659018677241, max cpu: 34.408604, count: 59153"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 164.81640625,
            "unit": "median mem",
            "extra": "avg mem: 157.25052690427788, max mem: 174.2734375, count: 59153"
          }
        ]
      }
    ],
    "pg_search bulk-updates.toml Performance - TPS": [
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
        "date": 1752237116603,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 7.933620044579112,
            "unit": "median tps",
            "extra": "avg tps: 6.8816419883614675, max tps: 10.173375863220695, count: 59115"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 6.89333235698411,
            "unit": "median tps",
            "extra": "avg tps: 6.281018090359702, max tps: 7.689324986980393, count: 59115"
          }
        ]
      },
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
        "date": 1752238177595,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.42721053621521,
            "unit": "median tps",
            "extra": "avg tps: 7.294552663247997, max tps: 10.752947595238485, count: 59181"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 7.218829222460416,
            "unit": "median tps",
            "extra": "avg tps: 6.61514504053846, max tps: 8.057049558883858, count: 59181"
          }
        ]
      },
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
        "date": 1752239184790,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.237446921617035,
            "unit": "median tps",
            "extra": "avg tps: 7.1568245283804215, max tps: 10.65030554696505, count: 59183"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 7.313414407949339,
            "unit": "median tps",
            "extra": "avg tps: 6.652802067942109, max tps: 8.091755012063107, count: 59183"
          }
        ]
      }
    ],
    "pg_search bulk-updates.toml Performance - Other Metrics": [
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
        "date": 1752237161360,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 22.829966,
            "unit": "median cpu",
            "extra": "avg cpu: 20.096259615936148, max cpu: 43.11377, count: 59115"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 228.07421875,
            "unit": "median mem",
            "extra": "avg mem: 227.26424243265245, max mem: 230.93359375, count: 59115"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.460411,
            "unit": "median cpu",
            "extra": "avg cpu: 21.55626161246322, max cpu: 33.532936, count: 59115"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 159.1953125,
            "unit": "median mem",
            "extra": "avg mem: 159.38718434143195, max mem: 166.140625, count: 59115"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 23229,
            "unit": "median block_count",
            "extra": "avg block_count: 21557.03264822803, max block_count: 24786.0, count: 59115"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 53,
            "unit": "median segment_count",
            "extra": "avg segment_count: 55.01409117821196, max segment_count: 85.0, count: 59115"
          }
        ]
      },
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
        "date": 1752238191382,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 19.047619,
            "unit": "median cpu",
            "extra": "avg cpu: 19.708163573241134, max cpu: 43.90244, count: 59181"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 228.2109375,
            "unit": "median mem",
            "extra": "avg mem: 227.3669952930417, max mem: 231.79296875, count: 59181"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.529411,
            "unit": "median cpu",
            "extra": "avg cpu: 21.31500893965218, max cpu: 33.633633, count: 59181"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 159.5,
            "unit": "median mem",
            "extra": "avg mem: 159.75227803528583, max mem: 165.20703125, count: 59181"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 23627,
            "unit": "median block_count",
            "extra": "avg block_count: 21966.33490478363, max block_count: 25082.0, count: 59181"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 54,
            "unit": "median segment_count",
            "extra": "avg segment_count: 56.20224396343421, max segment_count: 88.0, count: 59181"
          }
        ]
      }
    ]
  }
}