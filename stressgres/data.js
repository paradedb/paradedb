window.BENCHMARK_DATA = {
  "lastUpdate": 1758929049289,
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
        "date": 1758927545135,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 6.693840337874685,
            "unit": "median tps",
            "extra": "avg tps: 5.747848289719205, max tps: 8.589360505840613, count: 57497"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.764810826758839,
            "unit": "median tps",
            "extra": "avg tps: 5.174452141417371, max tps: 6.5290013697748615, count: 57497"
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
        "date": 1758927625378,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.332359126880283,
            "unit": "median tps",
            "extra": "avg tps: 7.114103095443092, max tps: 11.05155375026376, count: 57912"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.426808874350691,
            "unit": "median tps",
            "extra": "avg tps: 4.909404548734937, max tps: 6.025537974772346, count: 57912"
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
        "date": 1758927548432,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.233301,
            "unit": "median cpu",
            "extra": "avg cpu: 22.152824362030607, max cpu: 50.818092, count: 57497"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 235.3125,
            "unit": "median mem",
            "extra": "avg mem: 234.40866670217576, max mem: 241.56640625, count: 57497"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.27837,
            "unit": "median cpu",
            "extra": "avg cpu: 22.209835168114044, max cpu: 33.168808, count: 57497"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 159.6328125,
            "unit": "median mem",
            "extra": "avg mem: 159.48640309494408, max mem: 160.515625, count: 57497"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 22382,
            "unit": "median block_count",
            "extra": "avg block_count: 20713.5687253248, max block_count: 23422.0, count: 57497"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 67,
            "unit": "median segment_count",
            "extra": "avg segment_count: 68.40826477903195, max segment_count: 97.0, count: 57497"
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
        "date": 1758927628587,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.856806,
            "unit": "median cpu",
            "extra": "avg cpu: 19.66244138563773, max cpu: 42.985077, count: 57912"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 228.90625,
            "unit": "median mem",
            "extra": "avg mem: 228.0937207260585, max mem: 230.54296875, count: 57912"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.27837,
            "unit": "median cpu",
            "extra": "avg cpu: 22.276853545408525, max cpu: 33.267326, count: 57912"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 162.48828125,
            "unit": "median mem",
            "extra": "avg mem: 162.24727583715378, max mem: 163.98046875, count: 57912"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 24106,
            "unit": "median block_count",
            "extra": "avg block_count: 22989.203999171157, max block_count: 25769.0, count: 57912"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 71,
            "unit": "median segment_count",
            "extra": "avg segment_count: 72.42177786987153, max segment_count: 105.0, count: 57912"
          }
        ]
      }
    ],
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
          "id": "1c48d3c6427e9fe7bdb10861a16e04d0cf1a1f48",
          "message": "ci: fix benchmark links in slack messages (#2875)",
          "timestamp": "2025-07-17T12:13:25Z",
          "url": "https://github.com/paradedb/paradedb/commit/1c48d3c6427e9fe7bdb10861a16e04d0cf1a1f48"
        },
        "date": 1758928200169,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 27.42688899330715,
            "unit": "median tps",
            "extra": "avg tps: 27.34398149420055, max tps: 27.65432330390595, count: 57864"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 135.4936338981404,
            "unit": "median tps",
            "extra": "avg tps: 134.99008261435776, max tps: 136.99302437701462, count: 57864"
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
        "date": 1758928245815,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 33.75195014460928,
            "unit": "median tps",
            "extra": "avg tps: 33.71078229347821, max tps: 34.06785916033133, count: 57481"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 387.34561764823064,
            "unit": "median tps",
            "extra": "avg tps: 385.1243905798176, max tps: 394.73348602191703, count: 57481"
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
        "date": 1758928314612,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 27.306957492092494,
            "unit": "median tps",
            "extra": "avg tps: 27.11658460598041, max tps: 27.437736202737817, count: 57527"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 124.50611106067926,
            "unit": "median tps",
            "extra": "avg tps: 124.03082539067432, max tps: 125.91891476695866, count: 57527"
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
        "date": 1758928394940,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 35.46912051736822,
            "unit": "median tps",
            "extra": "avg tps: 35.35819189057581, max tps: 35.58042358474893, count: 57890"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 612.0717013470186,
            "unit": "median tps",
            "extra": "avg tps: 608.4677880060759, max tps: 657.2496064253012, count: 57890"
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
          "id": "1c48d3c6427e9fe7bdb10861a16e04d0cf1a1f48",
          "message": "ci: fix benchmark links in slack messages (#2875)",
          "timestamp": "2025-07-17T12:13:25Z",
          "url": "https://github.com/paradedb/paradedb/commit/1c48d3c6427e9fe7bdb10861a16e04d0cf1a1f48"
        },
        "date": 1758928203685,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.731707,
            "unit": "median cpu",
            "extra": "avg cpu: 20.72923425272983, max cpu: 57.83132, count: 57864"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 179.3046875,
            "unit": "median mem",
            "extra": "avg mem: 177.753024400426, max mem: 183.49609375, count: 57864"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 17689,
            "unit": "median block_count",
            "extra": "avg block_count: 16432.35232614406, max block_count: 21825.0, count: 57864"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 40,
            "unit": "median segment_count",
            "extra": "avg segment_count: 41.52780658094843, max segment_count: 114.0, count: 57864"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 13.832853,
            "unit": "median cpu",
            "extra": "avg cpu: 11.93617220304013, max cpu: 37.28155, count: 57864"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 164.5390625,
            "unit": "median mem",
            "extra": "avg mem: 153.90716087776943, max mem: 171.4140625, count: 57864"
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
        "date": 1758928251164,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.658894,
            "unit": "median cpu",
            "extra": "avg cpu: 20.16939666815072, max cpu: 47.33728, count: 57481"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 166.64453125,
            "unit": "median mem",
            "extra": "avg mem: 166.38060531686557, max mem: 171.46875, count: 57481"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 22200,
            "unit": "median block_count",
            "extra": "avg block_count: 20645.716775978148, max block_count: 28881.0, count: 57481"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 46,
            "unit": "median segment_count",
            "extra": "avg segment_count: 48.418277343817955, max segment_count: 128.0, count: 57481"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 5.714394622244402, max cpu: 27.612656, count: 57481"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 163.55078125,
            "unit": "median mem",
            "extra": "avg mem: 154.9014865373993, max mem: 166.92578125, count: 57481"
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
        "date": 1758928317874,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.75,
            "unit": "median cpu",
            "extra": "avg cpu: 20.915769399537954, max cpu: 112.171364, count: 57527"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 174.4453125,
            "unit": "median mem",
            "extra": "avg mem: 172.97994453962923, max mem: 179.8203125, count: 57527"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 18072,
            "unit": "median block_count",
            "extra": "avg block_count: 16510.309489457126, max block_count: 20886.0, count: 57527"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 39,
            "unit": "median segment_count",
            "extra": "avg segment_count: 41.46412988683575, max segment_count: 122.0, count: 57527"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 18.640776,
            "unit": "median cpu",
            "extra": "avg cpu: 19.181342265006062, max cpu: 148.11958, count: 57527"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 167.84375,
            "unit": "median mem",
            "extra": "avg mem: 157.1773586343152, max mem: 176.60546875, count: 57527"
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
        "date": 1758928397816,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.658894,
            "unit": "median cpu",
            "extra": "avg cpu: 19.861466009173625, max cpu: 51.41188, count: 57890"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 162.68359375,
            "unit": "median mem",
            "extra": "avg mem: 161.66134330735014, max mem: 166.48046875, count: 57890"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 22855,
            "unit": "median block_count",
            "extra": "avg block_count: 21753.855035411987, max block_count: 31163.0, count: 57890"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 47,
            "unit": "median segment_count",
            "extra": "avg segment_count: 49.22845050958715, max segment_count: 135.0, count: 57890"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.669261,
            "unit": "median cpu",
            "extra": "avg cpu: 5.432063071957556, max cpu: 28.042841, count: 57890"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 162.7421875,
            "unit": "median mem",
            "extra": "avg mem: 153.23296020739764, max mem: 163.4921875, count: 57890"
          }
        ]
      }
    ],
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
          "id": "1c48d3c6427e9fe7bdb10861a16e04d0cf1a1f48",
          "message": "ci: fix benchmark links in slack messages (#2875)",
          "timestamp": "2025-07-17T12:13:25Z",
          "url": "https://github.com/paradedb/paradedb/commit/1c48d3c6427e9fe7bdb10861a16e04d0cf1a1f48"
        },
        "date": 1758928946869,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Delete value - Primary - tps",
            "value": 253.825247805844,
            "unit": "median tps",
            "extra": "avg tps: 250.07392226259952, max tps: 606.4739799533079, count: 55467"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 140.72784958459772,
            "unit": "median tps",
            "extra": "avg tps: 137.37339358202732, max tps: 149.9403669625211, count: 55467"
          },
          {
            "name": "Monitor Segment Count - Primary - tps",
            "value": 1791.9772725274931,
            "unit": "median tps",
            "extra": "avg tps: 1762.0278245674288, max tps: 1893.0948625398678, count: 55467"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 19.26972509004595,
            "unit": "median tps",
            "extra": "avg tps: 22.97693581200605, max tps: 73.41186566673635, count: 166401"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 0.5048185422128013,
            "unit": "median tps",
            "extra": "avg tps: 0.9517277470698986, max tps: 4.86618095638412, count: 55467"
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
        "date": 1758928990207,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Delete value - Primary - tps",
            "value": 246.33793943789317,
            "unit": "median tps",
            "extra": "avg tps: 239.5724351589889, max tps: 485.39673387714015, count: 55419"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 396.884246835438,
            "unit": "median tps",
            "extra": "avg tps: 388.2203163115675, max tps: 422.8618428482659, count: 55419"
          },
          {
            "name": "Monitor Segment Count - Primary - tps",
            "value": 1766.8827858794261,
            "unit": "median tps",
            "extra": "avg tps: 1762.4798127011995, max tps: 1884.7454981861806, count: 55419"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 20.159313921215755,
            "unit": "median tps",
            "extra": "avg tps: 41.775887742905894, max tps: 175.00252483155901, count: 166257"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 1.2077920904114434,
            "unit": "median tps",
            "extra": "avg tps: 1.486700453895542, max tps: 4.852585421811577, count: 55419"
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
        "date": 1758929047731,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 33.658718342168534,
            "unit": "median tps",
            "extra": "avg tps: 34.13140808847822, max tps: 38.455020546847095, count: 55594"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 255.40234233292497,
            "unit": "median tps",
            "extra": "avg tps: 293.71908592149316, max tps: 2504.448682792581, count: 55594"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 124.976598727316,
            "unit": "median tps",
            "extra": "avg tps: 124.10081392162415, max tps: 127.59476440589938, count: 55594"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 69.8329062055666,
            "unit": "median tps",
            "extra": "avg tps: 65.99745231735514, max tps: 103.5485858092204, count: 111188"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 16.680060083263108,
            "unit": "median tps",
            "extra": "avg tps: 16.639721282676554, max tps: 18.51006661643405, count: 55594"
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
          "id": "1c48d3c6427e9fe7bdb10861a16e04d0cf1a1f48",
          "message": "ci: fix benchmark links in slack messages (#2875)",
          "timestamp": "2025-07-17T12:13:25Z",
          "url": "https://github.com/paradedb/paradedb/commit/1c48d3c6427e9fe7bdb10861a16e04d0cf1a1f48"
        },
        "date": 1758928949588,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 7.567888522333765, max cpu: 32.71665, count: 55467"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 205.37890625,
            "unit": "median mem",
            "extra": "avg mem: 203.8202517938594, max mem: 205.37890625, count: 55467"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.29332,
            "unit": "median cpu",
            "extra": "avg cpu: 10.418118271519194, max cpu: 23.255816, count: 55467"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 161.6484375,
            "unit": "median mem",
            "extra": "avg mem: 156.55326745519858, max mem: 170.76953125, count: 55467"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 40392,
            "unit": "median block_count",
            "extra": "avg block_count: 41030.39506373159, max block_count: 57834.0, count: 55467"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.94593869191595, max cpu: 9.275363, count: 55467"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 120.85546875,
            "unit": "median mem",
            "extra": "avg mem: 109.79650379110552, max mem: 137.74609375, count: 55467"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 26,
            "unit": "median segment_count",
            "extra": "avg segment_count: 26.26085780734491, max segment_count: 59.0, count: 55467"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 23.076923,
            "unit": "median cpu",
            "extra": "avg cpu: 20.46212270404014, max cpu: 32.844578, count: 166401"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 215.2421875,
            "unit": "median mem",
            "extra": "avg mem: 234.9711652639933, max mem: 457.12890625, count: 166401"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.953489,
            "unit": "median cpu",
            "extra": "avg cpu: 15.536323167815674, max cpu: 32.621357, count: 55467"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 193.78515625,
            "unit": "median mem",
            "extra": "avg mem: 192.29372313999767, max mem: 224.2578125, count: 55467"
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
        "date": 1758928993053,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 7.68312287138877, max cpu: 32.55814, count: 55419"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 203.171875,
            "unit": "median mem",
            "extra": "avg mem: 201.54928046732618, max mem: 203.171875, count: 55419"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 5.825317308870432, max cpu: 13.980582, count: 55419"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 162.37109375,
            "unit": "median mem",
            "extra": "avg mem: 153.38656967263032, max mem: 164.25390625, count: 55419"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 47484,
            "unit": "median block_count",
            "extra": "avg block_count: 47601.64919973294, max block_count: 75465.0, count: 55419"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6153846,
            "unit": "median cpu",
            "extra": "avg cpu: 4.138705233820332, max cpu: 4.628737, count: 55419"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 130.73828125,
            "unit": "median mem",
            "extra": "avg mem: 116.08220993138634, max mem: 140.51171875, count: 55419"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 35,
            "unit": "median segment_count",
            "extra": "avg segment_count: 35.93715151843231, max segment_count: 69.0, count: 55419"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 23.010548,
            "unit": "median cpu",
            "extra": "avg cpu: 18.48985509947313, max cpu: 47.058823, count: 166257"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 221.76171875,
            "unit": "median mem",
            "extra": "avg mem: 272.3318857439085, max mem: 498.57421875, count: 166257"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 23.076923,
            "unit": "median cpu",
            "extra": "avg cpu: 20.747502531786143, max cpu: 32.463768, count: 55419"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 199.86328125,
            "unit": "median mem",
            "extra": "avg mem: 197.88959555556306, max mem: 230.88671875, count: 55419"
          }
        ]
      }
    ]
  }
}