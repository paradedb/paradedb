window.BENCHMARK_DATA = {
  "lastUpdate": 1752436428977,
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
      }
    ]
  }
}