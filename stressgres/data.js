window.BENCHMARK_DATA = {
  "lastUpdate": 1758926758027,
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
          "id": "1c48d3c6427e9fe7bdb10861a16e04d0cf1a1f48",
          "message": "ci: fix benchmark links in slack messages (#2875)",
          "timestamp": "2025-07-17T12:13:25Z",
          "url": "https://github.com/paradedb/paradedb/commit/1c48d3c6427e9fe7bdb10861a16e04d0cf1a1f48"
        },
        "date": 1758926756462,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.746008333270744, max cpu: 9.458128, count: 55381"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 61.35546875,
            "unit": "median mem",
            "extra": "avg mem: 59.59294351289251, max mem: 83.46484375, count: 55381"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.6369741120476595, max cpu: 9.495549, count: 55381"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 53.34375,
            "unit": "median mem",
            "extra": "avg mem: 52.46040094696286, max mem: 76.1875, count: 55381"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.7709374303532215, max cpu: 14.4, count: 55381"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 60.68359375,
            "unit": "median mem",
            "extra": "avg mem: 60.1881573074475, max mem: 84.05078125, count: 55381"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.228251321703924, max cpu: 4.733728, count: 55381"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 59.7265625,
            "unit": "median mem",
            "extra": "avg mem: 59.45510975955201, max mem: 82.640625, count: 55381"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.213051,
            "unit": "median cpu",
            "extra": "avg cpu: 7.468630302092398, max cpu: 24.0, count: 110762"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 63.83203125,
            "unit": "median mem",
            "extra": "avg mem: 64.43475192022083, max mem: 93.046875, count: 110762"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 3762,
            "unit": "median block_count",
            "extra": "avg block_count: 3702.9660352828587, max block_count: 6752.0, count: 55381"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 8,
            "unit": "median segment_count",
            "extra": "avg segment_count: 8.89653491269569, max segment_count: 27.0, count: 55381"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 6.2046946756182795, max cpu: 14.257426, count: 55381"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 69.3359375,
            "unit": "median mem",
            "extra": "avg mem: 69.7826820542018, max mem: 96.4140625, count: 55381"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.585499493293692, max cpu: 9.284333, count: 55381"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 57.6171875,
            "unit": "median mem",
            "extra": "avg mem: 57.88476608347177, max mem: 81.9453125, count: 55381"
          }
        ]
      }
    ]
  }
}