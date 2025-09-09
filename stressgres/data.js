window.BENCHMARK_DATA = {
  "lastUpdate": 1757447863250,
  "repoUrl": "https://github.com/paradedb/paradedb",
  "entries": {
    "pg_search background-merge.toml Performance - TPS": [
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
          "id": "0544c54d64a963065cefc3a922582cc501a4c90e",
          "message": "fix: zero worker threads (#2959) (#3139)\n\nWe don't use any of Tantivy's threading features, and as of\nhttps://github.com/paradedb/tantivy/pull/59 it's now possible to set the\nnumber of merge and worker threads to zero.\n\nDoing so saves overhead of making threads that we never use, and joining\non them, for every segment merge operation.\n\n\n🍒 This is a cherry pick of 98d7dcdc33169d31d80e13ef39aa7242e1a09710 from\n`main/0.18.x`",
          "timestamp": "2025-09-09T18:06:14Z",
          "url": "https://github.com/paradedb/paradedb/commit/0544c54d64a963065cefc3a922582cc501a4c90e"
        },
        "date": 1757446934636,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Delete value - Primary - tps",
            "value": 246.36464082668698,
            "unit": "median tps",
            "extra": "avg tps: 238.717335185094, max tps: 479.59450641571533, count: 55741"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 432.5390208207214,
            "unit": "median tps",
            "extra": "avg tps: 425.98768000105474, max tps: 459.4577051934103, count: 55741"
          },
          {
            "name": "Monitor Segment Count - Primary - tps",
            "value": 1799.0441739571525,
            "unit": "median tps",
            "extra": "avg tps: 1800.078378524505, max tps: 1857.8331193913716, count: 55741"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 20.709789469776986,
            "unit": "median tps",
            "extra": "avg tps: 41.89573788596861, max tps: 173.69020992454986, count: 167223"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 1.2067561981357664,
            "unit": "median tps",
            "extra": "avg tps: 1.3994737710607075, max tps: 5.254380704612167, count: 55741"
          }
        ]
      }
    ],
    "pg_search background-merge.toml Performance - Other Metrics": [
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
          "id": "0544c54d64a963065cefc3a922582cc501a4c90e",
          "message": "fix: zero worker threads (#2959) (#3139)\n\nWe don't use any of Tantivy's threading features, and as of\nhttps://github.com/paradedb/tantivy/pull/59 it's now possible to set the\nnumber of merge and worker threads to zero.\n\nDoing so saves overhead of making threads that we never use, and joining\non them, for every segment merge operation.\n\n\n🍒 This is a cherry pick of 98d7dcdc33169d31d80e13ef39aa7242e1a09710 from\n`main/0.18.x`",
          "timestamp": "2025-09-09T18:06:14Z",
          "url": "https://github.com/paradedb/paradedb/commit/0544c54d64a963065cefc3a922582cc501a4c90e"
        },
        "date": 1757446937425,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 7.671846515387898, max cpu: 32.684826, count: 55741"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 207.15625,
            "unit": "median mem",
            "extra": "avg mem: 205.52454796514235, max mem: 207.15625, count: 55741"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 5.295032567335353, max cpu: 14.007783, count: 55741"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 164.6328125,
            "unit": "median mem",
            "extra": "avg mem: 153.93712627094507, max mem: 166.8984375, count: 55741"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 48878,
            "unit": "median block_count",
            "extra": "avg block_count: 48722.42017545433, max block_count: 76654.0, count: 55741"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 2.9487562947834243, max cpu: 4.6511626, count: 55741"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 132.2890625,
            "unit": "median mem",
            "extra": "avg mem: 116.26924701128881, max mem: 141.3125, count: 55741"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 35,
            "unit": "median segment_count",
            "extra": "avg segment_count: 35.800846773470155, max segment_count: 68.0, count: 55741"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 23.076923,
            "unit": "median cpu",
            "extra": "avg cpu: 18.56918237454024, max cpu: 32.71665, count: 167223"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 227.36328125,
            "unit": "median mem",
            "extra": "avg mem: 274.2931124811629, max mem: 500.05859375, count: 167223"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 23.166023,
            "unit": "median cpu",
            "extra": "avg cpu: 20.51355914761103, max cpu: 32.589718, count: 55741"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 200.5234375,
            "unit": "median mem",
            "extra": "avg mem: 197.4519551783023, max mem: 231.1328125, count: 55741"
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
          "id": "1c48d3c6427e9fe7bdb10861a16e04d0cf1a1f48",
          "message": "ci: fix benchmark links in slack messages (#2875)",
          "timestamp": "2025-07-17T12:13:25Z",
          "url": "https://github.com/paradedb/paradedb/commit/1c48d3c6427e9fe7bdb10861a16e04d0cf1a1f48"
        },
        "date": 1757447856470,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 1259.5320384725492,
            "unit": "median tps",
            "extra": "avg tps: 1247.5969043336243, max tps: 1263.4199707943944, count: 55039"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2586.096064044068,
            "unit": "median tps",
            "extra": "avg tps: 2593.38679646088, max tps: 2641.4324289534175, count: 55039"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 1168.8658852724543,
            "unit": "median tps",
            "extra": "avg tps: 1168.454479301618, max tps: 1209.8411521199764, count: 55039"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 983.8142784805156,
            "unit": "median tps",
            "extra": "avg tps: 980.9535466043926, max tps: 995.1930174342793, count: 55039"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 157.70939915678613,
            "unit": "median tps",
            "extra": "avg tps: 157.52784547634755, max tps: 165.27334275816813, count: 110078"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 172.17911064606312,
            "unit": "median tps",
            "extra": "avg tps: 170.1576082088954, max tps: 173.21682646991584, count: 55039"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 27.577039448947644,
            "unit": "median tps",
            "extra": "avg tps: 33.67427968993795, max tps: 775.373516807384, count: 55039"
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
          "id": "8cd6a2c7cdf969cf43bd66f12beca6ddedd6889a",
          "message": "chore: Upgrade to `0.17.5` (#3005)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-08-20T17:19:49Z",
          "url": "https://github.com/paradedb/paradedb/commit/8cd6a2c7cdf969cf43bd66f12beca6ddedd6889a"
        },
        "date": 1757447856855,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 1191.5334706884983,
            "unit": "median tps",
            "extra": "avg tps: 1188.6565213097747, max tps: 1230.0197098802896, count: 55388"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2611.564012497589,
            "unit": "median tps",
            "extra": "avg tps: 2610.3277656497885, max tps: 2666.756447467065, count: 55388"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 1168.3529666692887,
            "unit": "median tps",
            "extra": "avg tps: 1167.4953517838517, max tps: 1223.4195990608114, count: 55388"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 1015.3834298829055,
            "unit": "median tps",
            "extra": "avg tps: 1014.5556087530746, max tps: 1030.9198975332492, count: 55388"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 166.93917115571108,
            "unit": "median tps",
            "extra": "avg tps: 181.59874122140928, max tps: 206.7103830442316, count: 110776"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 140.3886761938467,
            "unit": "median tps",
            "extra": "avg tps: 139.962679810926, max tps: 149.74950131300037, count: 55388"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 39.82233959216197,
            "unit": "median tps",
            "extra": "avg tps: 45.20561714558792, max tps: 757.4896792031209, count: 55388"
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
        "date": 1757447859282,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.787988353805128, max cpu: 9.448819, count: 55039"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 59.49609375,
            "unit": "median mem",
            "extra": "avg mem: 59.61352887270844, max mem: 82.75390625, count: 55039"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.6959982956668025, max cpu: 9.320388, count: 55039"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 53.94140625,
            "unit": "median mem",
            "extra": "avg mem: 53.20595992160105, max mem: 76.22265625, count: 55039"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.750694175222884, max cpu: 9.467456, count: 55039"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 59.54296875,
            "unit": "median mem",
            "extra": "avg mem: 59.80519679795236, max mem: 83.2421875, count: 55039"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.619827,
            "unit": "median cpu",
            "extra": "avg cpu: 4.458919937868095, max cpu: 4.660194, count: 55039"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 59.98828125,
            "unit": "median mem",
            "extra": "avg mem: 60.09983496077781, max mem: 83.3046875, count: 55039"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.221902,
            "unit": "median cpu",
            "extra": "avg cpu: 7.9138276548492605, max cpu: 27.799229, count: 110078"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 65.04296875,
            "unit": "median mem",
            "extra": "avg mem: 65.09999464868093, max mem: 93.74609375, count: 110078"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 3636,
            "unit": "median block_count",
            "extra": "avg block_count: 3668.2462435727393, max block_count: 6601.0, count: 55039"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 8,
            "unit": "median segment_count",
            "extra": "avg segment_count: 8.85448500154436, max segment_count: 28.0, count: 55039"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 5.964023948836481, max cpu: 14.201183, count: 55039"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 69.05859375,
            "unit": "median mem",
            "extra": "avg mem: 69.15817477152565, max mem: 95.83203125, count: 55039"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.627355512717009, max cpu: 9.239654, count: 55039"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 57.98046875,
            "unit": "median mem",
            "extra": "avg mem: 58.246948896464325, max mem: 83.94921875, count: 55039"
          }
        ]
      }
    ]
  }
}