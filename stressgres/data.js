window.BENCHMARK_DATA = {
  "lastUpdate": 1758926755253,
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
          "id": "1c48d3c6427e9fe7bdb10861a16e04d0cf1a1f48",
          "message": "ci: fix benchmark links in slack messages (#2875)",
          "timestamp": "2025-07-17T12:13:25Z",
          "url": "https://github.com/paradedb/paradedb/commit/1c48d3c6427e9fe7bdb10861a16e04d0cf1a1f48"
        },
        "date": 1758926753644,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 1239.80390343863,
            "unit": "median tps",
            "extra": "avg tps: 1233.3363274325204, max tps: 1247.7716525054022, count: 55381"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2863.777771876975,
            "unit": "median tps",
            "extra": "avg tps: 2860.0821019974182, max tps: 2913.394843756334, count: 55381"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 1220.3642158830555,
            "unit": "median tps",
            "extra": "avg tps: 1213.5905811388322, max tps: 1224.3967766086078, count: 55381"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 945.5448260100852,
            "unit": "median tps",
            "extra": "avg tps: 941.233535501285, max tps: 1013.2680295995857, count: 55381"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 175.4789608942257,
            "unit": "median tps",
            "extra": "avg tps: 176.55313155277898, max tps: 181.09101919927284, count: 110762"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 152.4354361267668,
            "unit": "median tps",
            "extra": "avg tps: 151.0827709597468, max tps: 154.13762786977972, count: 55381"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 42.852208214931956,
            "unit": "median tps",
            "extra": "avg tps: 45.94176290254606, max tps: 762.5147737237409, count: 55381"
          }
        ]
      }
    ]
  }
}