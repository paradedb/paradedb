window.BENCHMARK_DATA = {
  "lastUpdate": 1758927503635,
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
          "id": "ec43ca34644c8c72e65ed2b7e9570066e839bab7",
          "message": "chore: Upgrade to `0.17.12` (#3120)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nNote that `0.17.11` was skipped because it was a hotfix in enterprise.\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-09-03T20:56:05Z",
          "url": "https://github.com/paradedb/paradedb/commit/ec43ca34644c8c72e65ed2b7e9570066e839bab7"
        },
        "date": 1758926755482,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 1175.7123989447757,
            "unit": "median tps",
            "extra": "avg tps: 1177.2191280806442, max tps: 1217.3902007267247, count: 55385"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2893.023192764439,
            "unit": "median tps",
            "extra": "avg tps: 2884.0830335208675, max tps: 2913.9276298725504, count: 55385"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 1246.6544184760915,
            "unit": "median tps",
            "extra": "avg tps: 1244.0518264590225, max tps: 1255.4963148839531, count: 55385"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 1074.4494819039098,
            "unit": "median tps",
            "extra": "avg tps: 1065.1544895685122, max tps: 1085.5080202922306, count: 55385"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 533.8214077578291,
            "unit": "median tps",
            "extra": "avg tps: 572.8793727311004, max tps: 643.6570231676928, count: 110770"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 439.7860669227833,
            "unit": "median tps",
            "extra": "avg tps: 436.1504577062103, max tps: 442.31759402288253, count: 55385"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 161.38121830011784,
            "unit": "median tps",
            "extra": "avg tps: 162.2701014174685, max tps: 732.1818066000333, count: 55385"
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
          "id": "60c4cf138fc2eeb08d4326b839ba91c1e8e0fbba",
          "message": "chore: upgrade to `0.18.0` (#2980)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nNote: `cargo.toml` is already on `0.18.0` so this is docs-only\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-08-18T19:09:25Z",
          "url": "https://github.com/paradedb/paradedb/commit/60c4cf138fc2eeb08d4326b839ba91c1e8e0fbba"
        },
        "date": 1758926781812,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 1090.417940406629,
            "unit": "median tps",
            "extra": "avg tps: 1091.1794556255547, max tps: 1124.6272108214114, count: 55274"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2819.929221121462,
            "unit": "median tps",
            "extra": "avg tps: 2807.0671039043814, max tps: 2846.5750046602084, count: 55274"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 1198.9501826161302,
            "unit": "median tps",
            "extra": "avg tps: 1194.4086640872251, max tps: 1219.8253622347354, count: 55274"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 1023.6613015495665,
            "unit": "median tps",
            "extra": "avg tps: 1020.2356994190909, max tps: 1047.2306687495836, count: 55274"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 156.00074572883042,
            "unit": "median tps",
            "extra": "avg tps: 156.18433941685367, max tps: 164.96316973583657, count: 110548"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 136.898723716591,
            "unit": "median tps",
            "extra": "avg tps: 136.55065586345788, max tps: 142.4529456061504, count: 55274"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 107.63976917779861,
            "unit": "median tps",
            "extra": "avg tps: 132.36819022674166, max tps: 690.1511292942929, count: 55274"
          }
        ]
      },
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
          "id": "7caf32d33552263a7408c43c2c057dad25c77222",
          "message": "chore: Upgrade to `0.18.9` (#3234)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\nSigned-off-by: Ming <ming.ying.nyc@gmail.com>\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>\nCo-authored-by: Philippe Noël <21990816+philippemnoel@users.noreply.github.com>",
          "timestamp": "2025-09-26T20:26:43Z",
          "url": "https://github.com/paradedb/paradedb/commit/7caf32d33552263a7408c43c2c057dad25c77222"
        },
        "date": 1758926787149,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 1215.4157723297692,
            "unit": "median tps",
            "extra": "avg tps: 1207.1075178811052, max tps: 1218.7716553191967, count: 55239"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3298.8688192065847,
            "unit": "median tps",
            "extra": "avg tps: 3281.874524653807, max tps: 3440.623285205804, count: 55239"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 1213.77348441877,
            "unit": "median tps",
            "extra": "avg tps: 1207.8464947161615, max tps: 1217.069840005966, count: 55239"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 982.0721971674753,
            "unit": "median tps",
            "extra": "avg tps: 979.1904088453832, max tps: 1009.0700972036216, count: 55239"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1608.0544237111842,
            "unit": "median tps",
            "extra": "avg tps: 1598.1479346699955, max tps: 1618.755007307581, count: 110478"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1224.7941189964981,
            "unit": "median tps",
            "extra": "avg tps: 1212.5792777700347, max tps: 1227.5328639391864, count: 55239"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 243.88039353462614,
            "unit": "median tps",
            "extra": "avg tps: 308.0704189830976, max tps: 721.4965570184299, count: 55239"
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
          "id": "ec43ca34644c8c72e65ed2b7e9570066e839bab7",
          "message": "chore: Upgrade to `0.17.12` (#3120)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nNote that `0.17.11` was skipped because it was a hotfix in enterprise.\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-09-03T20:56:05Z",
          "url": "https://github.com/paradedb/paradedb/commit/ec43ca34644c8c72e65ed2b7e9570066e839bab7"
        },
        "date": 1758926761974,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.778957909366249, max cpu: 9.638554, count: 55385"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 65.89453125,
            "unit": "median mem",
            "extra": "avg mem: 65.73131418141193, max mem: 95.17578125, count: 55385"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.7109030086330925, max cpu: 9.284333, count: 55385"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 59.01171875,
            "unit": "median mem",
            "extra": "avg mem: 59.487267254220455, max mem: 89.48046875, count: 55385"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.771553872947026, max cpu: 11.474104, count: 55385"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 66.98828125,
            "unit": "median mem",
            "extra": "avg mem: 66.61230117868105, max mem: 96.49609375, count: 55385"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.460984434444453, max cpu: 4.7244096, count: 55385"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 65.09765625,
            "unit": "median mem",
            "extra": "avg mem: 64.52748748815112, max mem: 94.9140625, count: 55385"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.998376805736201, max cpu: 14.159292, count: 110770"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 74,
            "unit": "median mem",
            "extra": "avg mem: 73.63470623956171, max mem: 104.55078125, count: 110770"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 4485,
            "unit": "median block_count",
            "extra": "avg block_count: 4525.574090457705, max block_count: 8336.0, count: 55385"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 9,
            "unit": "median segment_count",
            "extra": "avg segment_count: 9.973891847973277, max segment_count: 29.0, count: 55385"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.861498495953062, max cpu: 9.628887, count: 55385"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 79.36328125,
            "unit": "median mem",
            "extra": "avg mem: 78.9771708128329, max mem: 109.28515625, count: 55385"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.610951,
            "unit": "median cpu",
            "extra": "avg cpu: 4.27627439127794, max cpu: 4.6511626, count: 55385"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 62.68359375,
            "unit": "median mem",
            "extra": "avg mem: 62.84976027184707, max mem: 93.3359375, count: 55385"
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
          "id": "60c4cf138fc2eeb08d4326b839ba91c1e8e0fbba",
          "message": "chore: upgrade to `0.18.0` (#2980)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nNote: `cargo.toml` is already on `0.18.0` so this is docs-only\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-08-18T19:09:25Z",
          "url": "https://github.com/paradedb/paradedb/commit/60c4cf138fc2eeb08d4326b839ba91c1e8e0fbba"
        },
        "date": 1758926784974,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.747316523925985, max cpu: 9.657948, count: 55274"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 57.125,
            "unit": "median mem",
            "extra": "avg mem: 56.81248734995206, max mem: 75.42578125, count: 55274"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.640531916671164, max cpu: 9.421001, count: 55274"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 49.4765625,
            "unit": "median mem",
            "extra": "avg mem: 49.61481134043583, max mem: 68.6484375, count: 55274"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.783464236029219, max cpu: 9.504951, count: 55274"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 56.92578125,
            "unit": "median mem",
            "extra": "avg mem: 56.713734574008576, max mem: 76.01953125, count: 55274"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.45471535513518, max cpu: 4.7244096, count: 55274"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 56.58984375,
            "unit": "median mem",
            "extra": "avg mem: 55.93715039225042, max mem: 74.98046875, count: 55274"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.239654,
            "unit": "median cpu",
            "extra": "avg cpu: 8.643228979844183, max cpu: 28.973843, count: 110548"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 62.24609375,
            "unit": "median mem",
            "extra": "avg mem: 63.208048448128864, max mem: 89.80859375, count: 110548"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 3116,
            "unit": "median block_count",
            "extra": "avg block_count: 3123.118030176937, max block_count: 5547.0, count: 55274"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 9,
            "unit": "median segment_count",
            "extra": "avg segment_count: 9.501175959764085, max segment_count: 26.0, count: 55274"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 6.784653458472426, max cpu: 23.27837, count: 55274"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 70.35546875,
            "unit": "median mem",
            "extra": "avg mem: 69.5513994062534, max mem: 95.21875, count: 55274"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.886298139560314, max cpu: 9.239654, count: 55274"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 53.69140625,
            "unit": "median mem",
            "extra": "avg mem: 53.745236938931505, max mem: 75.78125, count: 55274"
          }
        ]
      },
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
          "id": "7caf32d33552263a7408c43c2c057dad25c77222",
          "message": "chore: Upgrade to `0.18.9` (#3234)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\nSigned-off-by: Ming <ming.ying.nyc@gmail.com>\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>\nCo-authored-by: Philippe Noël <21990816+philippemnoel@users.noreply.github.com>",
          "timestamp": "2025-09-26T20:26:43Z",
          "url": "https://github.com/paradedb/paradedb/commit/7caf32d33552263a7408c43c2c057dad25c77222"
        },
        "date": 1758926791716,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.736927556807703, max cpu: 11.4832535, count: 55239"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 92.05859375,
            "unit": "median mem",
            "extra": "avg mem: 89.78751977203606, max mem: 138.79296875, count: 55239"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.6222427115245175, max cpu: 9.657948, count: 55239"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 26.15625,
            "unit": "median mem",
            "extra": "avg mem: 26.038796397812234, max mem: 28.91796875, count: 55239"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.72787282123497, max cpu: 9.657948, count: 55239"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 93.57421875,
            "unit": "median mem",
            "extra": "avg mem: 90.62241966443545, max mem: 139.7109375, count: 55239"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.493264069641446, max cpu: 4.8144436, count: 55239"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 91.72265625,
            "unit": "median mem",
            "extra": "avg mem: 90.25749541198248, max mem: 138.96875, count: 55239"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.619784614213563, max cpu: 9.638554, count: 110478"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 92.81640625,
            "unit": "median mem",
            "extra": "avg mem: 90.11533818526087, max mem: 138.7578125, count: 110478"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 7707,
            "unit": "median block_count",
            "extra": "avg block_count: 7485.743206792303, max block_count: 13648.0, count: 55239"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 9,
            "unit": "median segment_count",
            "extra": "avg segment_count: 9.867593548036714, max segment_count: 31.0, count: 55239"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.554422350840255, max cpu: 9.448819, count: 55239"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 94.2890625,
            "unit": "median mem",
            "extra": "avg mem: 92.08739628874075, max mem: 140.65625, count: 55239"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.610951,
            "unit": "median cpu",
            "extra": "avg cpu: 3.510993425742616, max cpu: 4.729064, count: 55239"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 79.03125,
            "unit": "median mem",
            "extra": "avg mem: 76.25251414591592, max mem: 123.6640625, count: 55239"
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
          "id": "1c48d3c6427e9fe7bdb10861a16e04d0cf1a1f48",
          "message": "ci: fix benchmark links in slack messages (#2875)",
          "timestamp": "2025-07-17T12:13:25Z",
          "url": "https://github.com/paradedb/paradedb/commit/1c48d3c6427e9fe7bdb10861a16e04d0cf1a1f48"
        },
        "date": 1758927490007,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 6.53457751229432,
            "unit": "median tps",
            "extra": "avg tps: 5.65951014753639, max tps: 8.521944505981049, count: 57240"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.761190839170479,
            "unit": "median tps",
            "extra": "avg tps: 5.140821086292144, max tps: 6.497739400783565, count: 57240"
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
          "id": "ec43ca34644c8c72e65ed2b7e9570066e839bab7",
          "message": "chore: Upgrade to `0.17.12` (#3120)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nNote that `0.17.11` was skipped because it was a hotfix in enterprise.\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-09-03T20:56:05Z",
          "url": "https://github.com/paradedb/paradedb/commit/ec43ca34644c8c72e65ed2b7e9570066e839bab7"
        },
        "date": 1758927499290,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.520701431503465,
            "unit": "median tps",
            "extra": "avg tps: 7.28544953931005, max tps: 11.349717091733316, count: 57846"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.338006376394384,
            "unit": "median tps",
            "extra": "avg tps: 4.826877374779002, max tps: 5.912133168008416, count: 57846"
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
          "id": "1c48d3c6427e9fe7bdb10861a16e04d0cf1a1f48",
          "message": "ci: fix benchmark links in slack messages (#2875)",
          "timestamp": "2025-07-17T12:13:25Z",
          "url": "https://github.com/paradedb/paradedb/commit/1c48d3c6427e9fe7bdb10861a16e04d0cf1a1f48"
        },
        "date": 1758927492820,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.210833,
            "unit": "median cpu",
            "extra": "avg cpu: 21.58620553859768, max cpu: 42.687748, count: 57240"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 228.203125,
            "unit": "median mem",
            "extra": "avg mem: 226.86039066594603, max mem: 233.5, count: 57240"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.255816,
            "unit": "median cpu",
            "extra": "avg cpu: 22.212478431593546, max cpu: 33.168808, count: 57240"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 160.15234375,
            "unit": "median mem",
            "extra": "avg mem: 159.89676997346697, max mem: 161.5, count: 57240"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 21991,
            "unit": "median block_count",
            "extra": "avg block_count: 20640.8142907058, max block_count: 23594.0, count: 57240"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 67,
            "unit": "median segment_count",
            "extra": "avg segment_count: 68.29385045422781, max segment_count: 96.0, count: 57240"
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
          "id": "ec43ca34644c8c72e65ed2b7e9570066e839bab7",
          "message": "chore: Upgrade to `0.17.12` (#3120)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nNote that `0.17.11` was skipped because it was a hotfix in enterprise.\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-09-03T20:56:05Z",
          "url": "https://github.com/paradedb/paradedb/commit/ec43ca34644c8c72e65ed2b7e9570066e839bab7"
        },
        "date": 1758927502079,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.622696,
            "unit": "median cpu",
            "extra": "avg cpu: 19.07741798738194, max cpu: 42.561577, count: 57846"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 230.04296875,
            "unit": "median mem",
            "extra": "avg mem: 230.09491972759568, max mem: 231.1484375, count: 57846"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.255816,
            "unit": "median cpu",
            "extra": "avg cpu: 22.453191089966555, max cpu: 33.333336, count: 57846"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 160.15234375,
            "unit": "median mem",
            "extra": "avg mem: 160.7539423777141, max mem: 164.30859375, count: 57846"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 24257,
            "unit": "median block_count",
            "extra": "avg block_count: 23070.977440099574, max block_count: 26181.0, count: 57846"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 71,
            "unit": "median segment_count",
            "extra": "avg segment_count: 73.0932994502645, max segment_count: 108.0, count: 57846"
          }
        ]
      }
    ]
  }
}