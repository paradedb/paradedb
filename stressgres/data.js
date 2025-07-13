window.BENCHMARK_DATA = {
  "lastUpdate": 1752436427051,
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
    ]
  }
}