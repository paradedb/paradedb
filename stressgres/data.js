window.BENCHMARK_DATA = {
  "lastUpdate": 1752437065845,
  "repoUrl": "https://github.com/paradedb/paradedb",
  "entries": {
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
        "date": 1752436426199,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 323.0497150835024,
            "unit": "median tps",
            "extra": "avg tps: 322.2328542245325, max tps: 494.26042525319684, count: 55121"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2206.040990216507,
            "unit": "median tps",
            "extra": "avg tps: 2210.781894497851, max tps: 2560.3097535891598, count: 55121"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 323.6061369853133,
            "unit": "median tps",
            "extra": "avg tps: 322.98857743473684, max tps: 519.96207363089, count: 55121"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 287.1048882980393,
            "unit": "median tps",
            "extra": "avg tps: 286.4277773962134, max tps: 432.5290570857905, count: 55121"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 145.29230366947672,
            "unit": "median tps",
            "extra": "avg tps: 144.8772720431259, max tps: 155.91403706930686, count: 110242"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 131.5577804731188,
            "unit": "median tps",
            "extra": "avg tps: 131.31088663669593, max tps: 139.1583075031122, count: 55121"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 5.569104651431134,
            "unit": "median tps",
            "extra": "avg tps: 9.325647373702845, max tps: 1064.2486936347286, count: 55121"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Ming",
            "username": "rebasedming",
            "email": "ming.ying.nyc@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "c0442237441f33c1c51d6c11e29849eda05816a7",
          "message": "chore: Upgrade to `0.16.2` (#2760)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-07-01T21:30:02Z",
          "url": "https://github.com/paradedb/paradedb/commit/c0442237441f33c1c51d6c11e29849eda05816a7"
        },
        "date": 1752436713145,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 260.2611570046687,
            "unit": "median tps",
            "extra": "avg tps: 265.2234427965298, max tps: 474.1525609572508, count: 55032"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2176.875606957359,
            "unit": "median tps",
            "extra": "avg tps: 2163.040697425475, max tps: 2406.17473936573, count: 55032"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 277.8758376228425,
            "unit": "median tps",
            "extra": "avg tps: 277.83159210637314, max tps: 442.057791536294, count: 55032"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 226.49563427249925,
            "unit": "median tps",
            "extra": "avg tps: 229.10546231852712, max tps: 362.5050052878605, count: 55032"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 133.8924157856556,
            "unit": "median tps",
            "extra": "avg tps: 136.31510940884488, max tps: 168.94231870109334, count: 110064"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 131.40168871253323,
            "unit": "median tps",
            "extra": "avg tps: 131.50895152780828, max tps: 137.13316182119257, count: 55032"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 5.256893075873309,
            "unit": "median tps",
            "extra": "avg tps: 9.04475908472829, max tps: 1025.144750438762, count: 55032"
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
        "date": 1752436428125,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.701273,
            "unit": "median cpu",
            "extra": "avg cpu: 7.011529890969347, max cpu: 23.66864, count: 55121"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 94.8984375,
            "unit": "median mem",
            "extra": "avg mem: 94.9453027203788, max mem: 99.94921875, count: 55121"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.632339059149825, max cpu: 9.311348, count: 55121"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 82.9609375,
            "unit": "median mem",
            "extra": "avg mem: 82.49236452531703, max mem: 86.3359375, count: 55121"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.701273,
            "unit": "median cpu",
            "extra": "avg cpu: 7.014712614651911, max cpu: 23.622047, count: 55121"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 94.98046875,
            "unit": "median mem",
            "extra": "avg mem: 94.71171221607918, max mem: 99.10546875, count: 55121"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.550330772291987, max cpu: 4.743083, count: 55121"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 95.4453125,
            "unit": "median mem",
            "extra": "avg mem: 94.8889680628753, max mem: 97.8984375, count: 55121"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.239654,
            "unit": "median cpu",
            "extra": "avg cpu: 8.125204659718948, max cpu: 24.096386, count: 110242"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 106.3125,
            "unit": "median mem",
            "extra": "avg mem: 105.04760627471835, max mem: 115.60546875, count: 110242"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 7938,
            "unit": "median block_count",
            "extra": "avg block_count: 7972.3861504689685, max block_count: 8284.0, count: 55121"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 118,
            "unit": "median segment_count",
            "extra": "avg segment_count: 117.77547577148455, max segment_count: 326.0, count: 55121"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 6.321543438593267, max cpu: 18.86051, count: 55121"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 113.1484375,
            "unit": "median mem",
            "extra": "avg mem: 113.04849735411639, max mem: 121.3984375, count: 55121"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 18.479307,
            "unit": "median cpu",
            "extra": "avg cpu: 15.89484346862342, max cpu: 32.12237, count: 55121"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 94.13671875,
            "unit": "median mem",
            "extra": "avg mem: 93.07371616251066, max mem: 98.93359375, count: 55121"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Ming",
            "username": "rebasedming",
            "email": "ming.ying.nyc@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "c0442237441f33c1c51d6c11e29849eda05816a7",
          "message": "chore: Upgrade to `0.16.2` (#2760)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-07-01T21:30:02Z",
          "url": "https://github.com/paradedb/paradedb/commit/c0442237441f33c1c51d6c11e29849eda05816a7"
        },
        "date": 1752436715465,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 9.17782,
            "unit": "median cpu",
            "extra": "avg cpu: 7.953529783768404, max cpu: 23.622047, count: 55032"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 125.6328125,
            "unit": "median mem",
            "extra": "avg mem: 124.82901964993096, max mem: 148.1875, count: 55032"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6065254,
            "unit": "median cpu",
            "extra": "avg cpu: 4.6289520368743595, max cpu: 9.239654, count: 55032"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 92.6953125,
            "unit": "median mem",
            "extra": "avg mem: 92.85805559719799, max mem: 101.6953125, count: 55032"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 9.142857,
            "unit": "median cpu",
            "extra": "avg cpu: 7.344535443470018, max cpu: 23.622047, count: 55032"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 124.3828125,
            "unit": "median mem",
            "extra": "avg mem: 122.53173657769571, max mem: 146.1328125, count: 55032"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6065254,
            "unit": "median cpu",
            "extra": "avg cpu: 4.626773099602219, max cpu: 9.221902, count: 55032"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 106.49609375,
            "unit": "median mem",
            "extra": "avg mem: 118.34019013937346, max mem: 136.87109375, count: 55032"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.195402,
            "unit": "median cpu",
            "extra": "avg cpu: 8.643082213731935, max cpu: 23.483368, count: 110064"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 130.60546875,
            "unit": "median mem",
            "extra": "avg mem: 138.46730091412724, max mem: 170.7421875, count: 110064"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 11275,
            "unit": "median block_count",
            "extra": "avg block_count: 11725.884103794157, max block_count: 14982.0, count: 55032"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 120,
            "unit": "median segment_count",
            "extra": "avg segment_count: 119.43147623201047, max segment_count: 460.0, count: 55032"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6153846,
            "unit": "median cpu",
            "extra": "avg cpu: 6.071737901708712, max cpu: 27.586206, count: 55032"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 128.03125,
            "unit": "median mem",
            "extra": "avg mem: 129.55292134008394, max mem: 145.3203125, count: 55032"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 18.35564,
            "unit": "median cpu",
            "extra": "avg cpu: 16.26565299816358, max cpu: 32.18391, count: 55032"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 98.3125,
            "unit": "median mem",
            "extra": "avg mem: 96.87702226091183, max mem: 99.7421875, count: 55032"
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
        "date": 1752437063065,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 6.873195104936607,
            "unit": "median tps",
            "extra": "avg tps: 5.885268152593949, max tps: 8.818356240768676, count: 57641"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.637502093474087,
            "unit": "median tps",
            "extra": "avg tps: 5.051487970588481, max tps: 6.35907044369686, count: 57641"
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
        "date": 1752437064965,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.166023,
            "unit": "median cpu",
            "extra": "avg cpu: 21.292413579405228, max cpu: 42.477875, count: 57641"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 229.609375,
            "unit": "median mem",
            "extra": "avg mem: 227.65215291361616, max mem: 232.91796875, count: 57641"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.255816,
            "unit": "median cpu",
            "extra": "avg cpu: 22.474624370173345, max cpu: 33.168808, count: 57641"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 159.21484375,
            "unit": "median mem",
            "extra": "avg mem: 159.01037198456828, max mem: 160.44140625, count: 57641"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 22521,
            "unit": "median block_count",
            "extra": "avg block_count: 20765.20481948613, max block_count: 23550.0, count: 57641"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 67,
            "unit": "median segment_count",
            "extra": "avg segment_count: 68.79351503270242, max segment_count: 97.0, count: 57641"
          }
        ]
      }
    ]
  }
}