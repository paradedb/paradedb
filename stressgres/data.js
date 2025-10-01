window.BENCHMARK_DATA = {
  "lastUpdate": 1759331282046,
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
      },
      {
        "commit": {
          "author": {
            "email": "ming.ying.nyc@gmail.com",
            "name": "Ming",
            "username": "rebasedming"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "d5d310fcc28c6c692cbbcf7f8b86f61e806434a5",
          "message": "feat: introduce ascii folding filter (#3241)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nIntroduces the ASCII filter.\n\n## Why\n\nUser request\n\n## How\n\n## Tests\n\nAdded regression test",
          "timestamp": "2025-09-30T12:53:56-04:00",
          "tree_id": "ec6a192f3de17da7459676d2429d3b2f5640c7b5",
          "url": "https://github.com/paradedb/paradedb/commit/d5d310fcc28c6c692cbbcf7f8b86f61e806434a5"
        },
        "date": 1759252188536,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 781.8211910898989,
            "unit": "median tps",
            "extra": "avg tps: 782.5003985765997, max tps: 858.983647944771, count: 54533"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3453.5653065168053,
            "unit": "median tps",
            "extra": "avg tps: 3432.8538561385917, max tps: 3479.0786964753243, count: 54533"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 774.9072466815412,
            "unit": "median tps",
            "extra": "avg tps: 775.2708357952703, max tps: 845.7064796640499, count: 54533"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 679.9388031523987,
            "unit": "median tps",
            "extra": "avg tps: 676.2631691208223, max tps: 714.9048729703404, count: 54533"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1627.0367188563941,
            "unit": "median tps",
            "extra": "avg tps: 1626.0264302317864, max tps: 1640.7268056560406, count: 109066"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1254.2601937643155,
            "unit": "median tps",
            "extra": "avg tps: 1246.6472723164866, max tps: 1258.1837282221609, count: 54533"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 118.52060387041244,
            "unit": "median tps",
            "extra": "avg tps: 134.24556558406127, max tps: 622.7991833857108, count: 54533"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "98e9eccf356ea70ff614bb8ed1fda12e1ffadfb7",
          "message": "feat: introduce ascii folding filter (#3242)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nIntroduces the ASCII filter.\n\n## Why\n\nUser request\n\n## How\n\n## Tests\n\nAdded regression test\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-09-30T14:05:24-04:00",
          "tree_id": "4e4d848be0995232f7dfbb9c2a4a681ea5a0025a",
          "url": "https://github.com/paradedb/paradedb/commit/98e9eccf356ea70ff614bb8ed1fda12e1ffadfb7"
        },
        "date": 1759256484084,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 768.8154433321026,
            "unit": "median tps",
            "extra": "avg tps: 770.4381725411639, max tps: 807.9778700678829, count: 54604"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3379.531835685177,
            "unit": "median tps",
            "extra": "avg tps: 3348.2213285074754, max tps: 3401.0158786550205, count: 54604"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 768.4533400066855,
            "unit": "median tps",
            "extra": "avg tps: 767.6183512465074, max tps: 810.2729897675023, count: 54604"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 678.7195873464157,
            "unit": "median tps",
            "extra": "avg tps: 678.138239857478, max tps: 710.2770249087937, count: 54604"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1650.5590103077675,
            "unit": "median tps",
            "extra": "avg tps: 1669.3079111138334, max tps: 1713.1413952372159, count: 109208"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1243.549970477918,
            "unit": "median tps",
            "extra": "avg tps: 1236.4703529128253, max tps: 1246.6140461379373, count: 54604"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 234.30640679940944,
            "unit": "median tps",
            "extra": "avg tps: 212.2721631683611, max tps: 564.1583434505729, count: 54604"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ming.ying.nyc@gmail.com",
            "name": "Ming",
            "username": "rebasedming"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "da4cfff239fd8e2e318591df7095f4cac4987a4b",
          "message": "fix: Correctly handle `COUNT(<column>)` (#3243)\n\n# Ticket(s) Closed\n\n- Closes #3196 \n\n## What\n\nBefore, any `COUNT(<column>)` was getting rewritten to a count of the\n\"ctid\" field, which is incorrect because it doesn't correctly handle\nnull values.\n\n## Why\n\n## How\n\n## Tests\n\nSee regression tests",
          "timestamp": "2025-09-30T16:35:47-04:00",
          "tree_id": "4e13feab234146d47e8e600f153bb9a27fe8383e",
          "url": "https://github.com/paradedb/paradedb/commit/da4cfff239fd8e2e318591df7095f4cac4987a4b"
        },
        "date": 1759265503679,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 786.9363633080294,
            "unit": "median tps",
            "extra": "avg tps: 787.0954712019123, max tps: 803.7493675058224, count: 55251"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3281.321145502584,
            "unit": "median tps",
            "extra": "avg tps: 3254.507836300575, max tps: 3347.0106026517137, count: 55251"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 811.979604323004,
            "unit": "median tps",
            "extra": "avg tps: 810.2687999637484, max tps: 846.3154007480903, count: 55251"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 712.6621419466516,
            "unit": "median tps",
            "extra": "avg tps: 707.642545556544, max tps: 716.3426038542701, count: 55251"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1692.7270889032975,
            "unit": "median tps",
            "extra": "avg tps: 1681.8299896022286, max tps: 1698.3861889057628, count: 110502"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1274.1478185309852,
            "unit": "median tps",
            "extra": "avg tps: 1266.568873394661, max tps: 1280.0445344193076, count: 55251"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 91.01049183798989,
            "unit": "median tps",
            "extra": "avg tps: 106.66843341365428, max tps: 602.3011517804925, count: 55251"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ming.ying.nyc@gmail.com",
            "name": "Ming",
            "username": "rebasedming"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "a2dc2b40d542f767bb13622b2aa510493c993338",
          "message": "chore: Upgrade to `0.18.10` (#3250)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-10-01T10:50:42-04:00",
          "tree_id": "e98a6721b157dfa24946a6e5664f8b2009f68472",
          "url": "https://github.com/paradedb/paradedb/commit/a2dc2b40d542f767bb13622b2aa510493c993338"
        },
        "date": 1759331280501,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 805.0186811717253,
            "unit": "median tps",
            "extra": "avg tps: 804.2472665756997, max tps: 875.2527057745235, count: 55344"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3315.471065772171,
            "unit": "median tps",
            "extra": "avg tps: 3302.936047869045, max tps: 3337.1436467913845, count: 55344"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 802.6951473083786,
            "unit": "median tps",
            "extra": "avg tps: 801.8036368709619, max tps: 875.9907964690599, count: 55344"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 695.4476023100821,
            "unit": "median tps",
            "extra": "avg tps: 691.8816533933693, max tps: 710.7647380799956, count: 55344"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1716.9193882396303,
            "unit": "median tps",
            "extra": "avg tps: 1705.749172420956, max tps: 1723.1397953360201, count: 110688"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1277.622654531077,
            "unit": "median tps",
            "extra": "avg tps: 1271.315237760921, max tps: 1280.8598604151434, count: 55344"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 97.06131529320746,
            "unit": "median tps",
            "extra": "avg tps: 104.87267499390796, max tps: 907.4138433241109, count: 55344"
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
      },
      {
        "commit": {
          "author": {
            "email": "ming.ying.nyc@gmail.com",
            "name": "Ming",
            "username": "rebasedming"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "d5d310fcc28c6c692cbbcf7f8b86f61e806434a5",
          "message": "feat: introduce ascii folding filter (#3241)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nIntroduces the ASCII filter.\n\n## Why\n\nUser request\n\n## How\n\n## Tests\n\nAdded regression test",
          "timestamp": "2025-09-30T12:53:56-04:00",
          "tree_id": "ec6a192f3de17da7459676d2429d3b2f5640c7b5",
          "url": "https://github.com/paradedb/paradedb/commit/d5d310fcc28c6c692cbbcf7f8b86f61e806434a5"
        },
        "date": 1759252191550,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.947375938213422, max cpu: 14.45783, count: 54533"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 153.5078125,
            "unit": "median mem",
            "extra": "avg mem: 138.42045842654906, max mem: 153.8828125, count: 54533"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.517773460115766, max cpu: 9.476802, count: 54533"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 26.44140625,
            "unit": "median mem",
            "extra": "avg mem: 26.886094486366055, max mem: 32.375, count: 54533"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 5.007696626171342, max cpu: 14.45783, count: 54533"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 153.33984375,
            "unit": "median mem",
            "extra": "avg mem: 138.0651115207764, max mem: 153.33984375, count: 54533"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.495994531843696, max cpu: 9.266409, count: 54533"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 154.11328125,
            "unit": "median mem",
            "extra": "avg mem: 138.67492855530136, max mem: 154.51171875, count: 54533"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.649813652202031, max cpu: 9.638554, count: 109066"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 152.69140625,
            "unit": "median mem",
            "extra": "avg mem: 137.28084464051582, max mem: 156.21484375, count: 109066"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 27262,
            "unit": "median block_count",
            "extra": "avg block_count: 27626.123558212457, max block_count: 54091.0, count: 54533"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 31,
            "unit": "median segment_count",
            "extra": "avg segment_count: 30.270368400784847, max segment_count: 73.0, count: 54533"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.456553937159338, max cpu: 9.356726, count: 54533"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 151.4609375,
            "unit": "median mem",
            "extra": "avg mem: 135.73561607765023, max mem: 155.96484375, count: 54533"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.669261,
            "unit": "median cpu",
            "extra": "avg cpu: 3.5781019890397725, max cpu: 4.819277, count: 54533"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 148.9375,
            "unit": "median mem",
            "extra": "avg mem: 129.8301623987998, max mem: 151.6640625, count: 54533"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "98e9eccf356ea70ff614bb8ed1fda12e1ffadfb7",
          "message": "feat: introduce ascii folding filter (#3242)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nIntroduces the ASCII filter.\n\n## Why\n\nUser request\n\n## How\n\n## Tests\n\nAdded regression test\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-09-30T14:05:24-04:00",
          "tree_id": "4e4d848be0995232f7dfbb9c2a4a681ea5a0025a",
          "url": "https://github.com/paradedb/paradedb/commit/98e9eccf356ea70ff614bb8ed1fda12e1ffadfb7"
        },
        "date": 1759256486756,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.900739843103201, max cpu: 18.768328, count: 54604"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 155.984375,
            "unit": "median mem",
            "extra": "avg mem: 140.4524506847026, max mem: 155.984375, count: 54604"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.596821306609018, max cpu: 9.476802, count: 54604"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 23.19140625,
            "unit": "median mem",
            "extra": "avg mem: 23.17963241589444, max mem: 23.609375, count: 54604"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.942342528514703, max cpu: 14.443329, count: 54604"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 154.83984375,
            "unit": "median mem",
            "extra": "avg mem: 139.30792630233134, max mem: 154.83984375, count: 54604"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.5981945139823015, max cpu: 4.7571855, count: 54604"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 153.9609375,
            "unit": "median mem",
            "extra": "avg mem: 138.7396832477932, max mem: 154.76171875, count: 54604"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.654034273680889, max cpu: 9.486166, count: 109208"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 151.4140625,
            "unit": "median mem",
            "extra": "avg mem: 135.1434050304236, max mem: 155.54296875, count: 109208"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 28124,
            "unit": "median block_count",
            "extra": "avg block_count: 28164.33543330159, max block_count: 54893.0, count: 54604"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 30,
            "unit": "median segment_count",
            "extra": "avg segment_count: 30.228682880375064, max segment_count: 73.0, count: 54604"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.572072532277714, max cpu: 9.365853, count: 54604"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 148.6640625,
            "unit": "median mem",
            "extra": "avg mem: 133.06290676392754, max mem: 152.80859375, count: 54604"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 3.027049212657107, max cpu: 9.230769, count: 54604"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 144.296875,
            "unit": "median mem",
            "extra": "avg mem: 126.8642088806452, max mem: 150.21484375, count: 54604"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ming.ying.nyc@gmail.com",
            "name": "Ming",
            "username": "rebasedming"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "da4cfff239fd8e2e318591df7095f4cac4987a4b",
          "message": "fix: Correctly handle `COUNT(<column>)` (#3243)\n\n# Ticket(s) Closed\n\n- Closes #3196 \n\n## What\n\nBefore, any `COUNT(<column>)` was getting rewritten to a count of the\n\"ctid\" field, which is incorrect because it doesn't correctly handle\nnull values.\n\n## Why\n\n## How\n\n## Tests\n\nSee regression tests",
          "timestamp": "2025-09-30T16:35:47-04:00",
          "tree_id": "4e13feab234146d47e8e600f153bb9a27fe8383e",
          "url": "https://github.com/paradedb/paradedb/commit/da4cfff239fd8e2e318591df7095f4cac4987a4b"
        },
        "date": 1759265506738,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.866917500901216, max cpu: 15.80247, count: 55251"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 153.41796875,
            "unit": "median mem",
            "extra": "avg mem: 136.69689193973863, max mem: 153.41796875, count: 55251"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.560006653948433, max cpu: 9.4395275, count: 55251"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 27.140625,
            "unit": "median mem",
            "extra": "avg mem: 27.352447594274313, max mem: 32.59375, count: 55251"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.876860827179426, max cpu: 15.80247, count: 55251"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 154.09765625,
            "unit": "median mem",
            "extra": "avg mem: 137.23127147868365, max mem: 154.47265625, count: 55251"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.614334583908361, max cpu: 4.743083, count: 55251"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 154.01953125,
            "unit": "median mem",
            "extra": "avg mem: 136.88057208523375, max mem: 154.39453125, count: 55251"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.677586082025965, max cpu: 9.495549, count: 110502"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 155.0390625,
            "unit": "median mem",
            "extra": "avg mem: 136.4324612054533, max mem: 156.93359375, count: 110502"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 26517,
            "unit": "median block_count",
            "extra": "avg block_count: 26948.619717290185, max block_count: 53684.0, count: 55251"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 30,
            "unit": "median segment_count",
            "extra": "avg segment_count: 29.871024958824275, max segment_count: 76.0, count: 55251"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.580001094450137, max cpu: 9.275363, count: 55251"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 153.0234375,
            "unit": "median mem",
            "extra": "avg mem: 135.98433625635735, max mem: 158.296875, count: 55251"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 5.2427189898868045, max cpu: 9.302325, count: 55251"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 148.75,
            "unit": "median mem",
            "extra": "avg mem: 128.7412411087582, max mem: 152.6171875, count: 55251"
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
      },
      {
        "commit": {
          "author": {
            "email": "ming.ying.nyc@gmail.com",
            "name": "Ming",
            "username": "rebasedming"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "d5d310fcc28c6c692cbbcf7f8b86f61e806434a5",
          "message": "feat: introduce ascii folding filter (#3241)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nIntroduces the ASCII filter.\n\n## Why\n\nUser request\n\n## How\n\n## Tests\n\nAdded regression test",
          "timestamp": "2025-09-30T12:53:56-04:00",
          "tree_id": "ec6a192f3de17da7459676d2429d3b2f5640c7b5",
          "url": "https://github.com/paradedb/paradedb/commit/d5d310fcc28c6c692cbbcf7f8b86f61e806434a5"
        },
        "date": 1759252926701,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.247722925487114,
            "unit": "median tps",
            "extra": "avg tps: 7.07032069877739, max tps: 11.015676810542592, count: 57763"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.4274481699527835,
            "unit": "median tps",
            "extra": "avg tps: 4.888689419717362, max tps: 6.026767799984218, count: 57763"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "98e9eccf356ea70ff614bb8ed1fda12e1ffadfb7",
          "message": "feat: introduce ascii folding filter (#3242)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nIntroduces the ASCII filter.\n\n## Why\n\nUser request\n\n## How\n\n## Tests\n\nAdded regression test\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-09-30T14:05:24-04:00",
          "tree_id": "4e4d848be0995232f7dfbb9c2a4a681ea5a0025a",
          "url": "https://github.com/paradedb/paradedb/commit/98e9eccf356ea70ff614bb8ed1fda12e1ffadfb7"
        },
        "date": 1759257223198,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.334140753473228,
            "unit": "median tps",
            "extra": "avg tps: 7.120119523211053, max tps: 11.100017268210772, count: 57330"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.525800589370457,
            "unit": "median tps",
            "extra": "avg tps: 4.987373154653468, max tps: 6.116835680646243, count: 57330"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ming.ying.nyc@gmail.com",
            "name": "Ming",
            "username": "rebasedming"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "da4cfff239fd8e2e318591df7095f4cac4987a4b",
          "message": "fix: Correctly handle `COUNT(<column>)` (#3243)\n\n# Ticket(s) Closed\n\n- Closes #3196 \n\n## What\n\nBefore, any `COUNT(<column>)` was getting rewritten to a count of the\n\"ctid\" field, which is incorrect because it doesn't correctly handle\nnull values.\n\n## Why\n\n## How\n\n## Tests\n\nSee regression tests",
          "timestamp": "2025-09-30T16:35:47-04:00",
          "tree_id": "4e13feab234146d47e8e600f153bb9a27fe8383e",
          "url": "https://github.com/paradedb/paradedb/commit/da4cfff239fd8e2e318591df7095f4cac4987a4b"
        },
        "date": 1759266245474,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.21664821679825,
            "unit": "median tps",
            "extra": "avg tps: 7.063840689341735, max tps: 11.037479747432217, count: 57347"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.344184261221868,
            "unit": "median tps",
            "extra": "avg tps: 4.828776532409075, max tps: 5.9158397916337915, count: 57347"
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
      },
      {
        "commit": {
          "author": {
            "email": "ming.ying.nyc@gmail.com",
            "name": "Ming",
            "username": "rebasedming"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "d5d310fcc28c6c692cbbcf7f8b86f61e806434a5",
          "message": "feat: introduce ascii folding filter (#3241)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nIntroduces the ASCII filter.\n\n## Why\n\nUser request\n\n## How\n\n## Tests\n\nAdded regression test",
          "timestamp": "2025-09-30T12:53:56-04:00",
          "tree_id": "ec6a192f3de17da7459676d2429d3b2f5640c7b5",
          "url": "https://github.com/paradedb/paradedb/commit/d5d310fcc28c6c692cbbcf7f8b86f61e806434a5"
        },
        "date": 1759252929372,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.879055,
            "unit": "median cpu",
            "extra": "avg cpu: 19.695865695747198, max cpu: 42.72997, count: 57763"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 226.40234375,
            "unit": "median mem",
            "extra": "avg mem: 225.91511103290603, max mem: 227.96484375, count: 57763"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.233301,
            "unit": "median cpu",
            "extra": "avg cpu: 22.406001256130214, max cpu: 33.168808, count: 57763"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 161.109375,
            "unit": "median mem",
            "extra": "avg mem: 161.1207859864879, max mem: 162.32421875, count: 57763"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 24132,
            "unit": "median block_count",
            "extra": "avg block_count: 22977.342312553017, max block_count: 25769.0, count: 57763"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 71,
            "unit": "median segment_count",
            "extra": "avg segment_count: 72.52507660613195, max segment_count: 106.0, count: 57763"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "98e9eccf356ea70ff614bb8ed1fda12e1ffadfb7",
          "message": "feat: introduce ascii folding filter (#3242)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nIntroduces the ASCII filter.\n\n## Why\n\nUser request\n\n## How\n\n## Tests\n\nAdded regression test\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-09-30T14:05:24-04:00",
          "tree_id": "4e4d848be0995232f7dfbb9c2a4a681ea5a0025a",
          "url": "https://github.com/paradedb/paradedb/commit/98e9eccf356ea70ff614bb8ed1fda12e1ffadfb7"
        },
        "date": 1759257226154,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.882769,
            "unit": "median cpu",
            "extra": "avg cpu: 19.656853136583056, max cpu: 42.899704, count: 57330"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 225.5859375,
            "unit": "median mem",
            "extra": "avg mem: 225.01884723039856, max mem: 227.09765625, count: 57330"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.346306,
            "unit": "median cpu",
            "extra": "avg cpu: 22.344177782292803, max cpu: 33.432835, count: 57330"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 161.265625,
            "unit": "median mem",
            "extra": "avg mem: 161.30085006759114, max mem: 162.96484375, count: 57330"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 24084,
            "unit": "median block_count",
            "extra": "avg block_count: 22939.47177742892, max block_count: 25921.0, count: 57330"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 71,
            "unit": "median segment_count",
            "extra": "avg segment_count: 72.58170242455957, max segment_count: 106.0, count: 57330"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ming.ying.nyc@gmail.com",
            "name": "Ming",
            "username": "rebasedming"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "da4cfff239fd8e2e318591df7095f4cac4987a4b",
          "message": "fix: Correctly handle `COUNT(<column>)` (#3243)\n\n# Ticket(s) Closed\n\n- Closes #3196 \n\n## What\n\nBefore, any `COUNT(<column>)` was getting rewritten to a count of the\n\"ctid\" field, which is incorrect because it doesn't correctly handle\nnull values.\n\n## Why\n\n## How\n\n## Tests\n\nSee regression tests",
          "timestamp": "2025-09-30T16:35:47-04:00",
          "tree_id": "4e13feab234146d47e8e600f153bb9a27fe8383e",
          "url": "https://github.com/paradedb/paradedb/commit/da4cfff239fd8e2e318591df7095f4cac4987a4b"
        },
        "date": 1759266247935,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.838305,
            "unit": "median cpu",
            "extra": "avg cpu: 19.48157683237117, max cpu: 42.772278, count: 57347"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 226.27734375,
            "unit": "median mem",
            "extra": "avg mem: 225.86857475107678, max mem: 227.828125, count: 57347"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.323614,
            "unit": "median cpu",
            "extra": "avg cpu: 22.39572079216873, max cpu: 33.20158, count: 57347"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 161.2890625,
            "unit": "median mem",
            "extra": "avg mem: 161.39671511805327, max mem: 163.61328125, count: 57347"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 23991,
            "unit": "median block_count",
            "extra": "avg block_count: 22970.98221354212, max block_count: 25942.0, count: 57347"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 71,
            "unit": "median segment_count",
            "extra": "avg segment_count: 72.59999651245924, max segment_count: 106.0, count: 57347"
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
      },
      {
        "commit": {
          "author": {
            "email": "ming.ying.nyc@gmail.com",
            "name": "Ming",
            "username": "rebasedming"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "d5d310fcc28c6c692cbbcf7f8b86f61e806434a5",
          "message": "feat: introduce ascii folding filter (#3241)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nIntroduces the ASCII filter.\n\n## Why\n\nUser request\n\n## How\n\n## Tests\n\nAdded regression test",
          "timestamp": "2025-09-30T12:53:56-04:00",
          "tree_id": "ec6a192f3de17da7459676d2429d3b2f5640c7b5",
          "url": "https://github.com/paradedb/paradedb/commit/d5d310fcc28c6c692cbbcf7f8b86f61e806434a5"
        },
        "date": 1759253687731,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 410.5703601393603,
            "unit": "median tps",
            "extra": "avg tps: 382.80732185211815, max tps: 491.91680726453734, count: 57160"
          },
          {
            "name": "Single Insert - Primary - tps",
            "value": 512.785313923952,
            "unit": "median tps",
            "extra": "avg tps: 513.362705991894, max tps: 544.6410175267536, count: 57160"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 991.0779306426416,
            "unit": "median tps",
            "extra": "avg tps: 976.2844607948318, max tps: 1281.3745894097533, count: 57160"
          },
          {
            "name": "Top N - Primary - tps",
            "value": 5.977801919503166,
            "unit": "median tps",
            "extra": "avg tps: 5.997698018636629, max tps: 7.1576367168468655, count: 57160"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "98e9eccf356ea70ff614bb8ed1fda12e1ffadfb7",
          "message": "feat: introduce ascii folding filter (#3242)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nIntroduces the ASCII filter.\n\n## Why\n\nUser request\n\n## How\n\n## Tests\n\nAdded regression test\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-09-30T14:05:24-04:00",
          "tree_id": "4e4d848be0995232f7dfbb9c2a4a681ea5a0025a",
          "url": "https://github.com/paradedb/paradedb/commit/98e9eccf356ea70ff614bb8ed1fda12e1ffadfb7"
        },
        "date": 1759257996422,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 466.33419565991863,
            "unit": "median tps",
            "extra": "avg tps: 452.259311450808, max tps: 518.6072358503676, count: 56528"
          },
          {
            "name": "Single Insert - Primary - tps",
            "value": 503.20307392316784,
            "unit": "median tps",
            "extra": "avg tps: 494.5288236337877, max tps: 525.5864575864639, count: 56528"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 950.3532376261015,
            "unit": "median tps",
            "extra": "avg tps: 932.6472991677174, max tps: 1194.8830871843895, count: 56528"
          },
          {
            "name": "Top N - Primary - tps",
            "value": 6.034422942918843,
            "unit": "median tps",
            "extra": "avg tps: 6.06004858914653, max tps: 7.244353563385049, count: 56528"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ming.ying.nyc@gmail.com",
            "name": "Ming",
            "username": "rebasedming"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "da4cfff239fd8e2e318591df7095f4cac4987a4b",
          "message": "fix: Correctly handle `COUNT(<column>)` (#3243)\n\n# Ticket(s) Closed\n\n- Closes #3196 \n\n## What\n\nBefore, any `COUNT(<column>)` was getting rewritten to a count of the\n\"ctid\" field, which is incorrect because it doesn't correctly handle\nnull values.\n\n## Why\n\n## How\n\n## Tests\n\nSee regression tests",
          "timestamp": "2025-09-30T16:35:47-04:00",
          "tree_id": "4e13feab234146d47e8e600f153bb9a27fe8383e",
          "url": "https://github.com/paradedb/paradedb/commit/da4cfff239fd8e2e318591df7095f4cac4987a4b"
        },
        "date": 1759267007258,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 456.59792844762114,
            "unit": "median tps",
            "extra": "avg tps: 443.7155305600575, max tps: 510.9366995816883, count: 56516"
          },
          {
            "name": "Single Insert - Primary - tps",
            "value": 509.392647768978,
            "unit": "median tps",
            "extra": "avg tps: 506.54431623847944, max tps: 536.8359596515688, count: 56516"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 932.2385813573624,
            "unit": "median tps",
            "extra": "avg tps: 913.5975146984975, max tps: 1174.3647022440846, count: 56516"
          },
          {
            "name": "Top N - Primary - tps",
            "value": 5.949770018607669,
            "unit": "median tps",
            "extra": "avg tps: 5.97986037358285, max tps: 7.16066356494409, count: 56516"
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
      },
      {
        "commit": {
          "author": {
            "email": "ming.ying.nyc@gmail.com",
            "name": "Ming",
            "username": "rebasedming"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "d5d310fcc28c6c692cbbcf7f8b86f61e806434a5",
          "message": "feat: introduce ascii folding filter (#3241)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nIntroduces the ASCII filter.\n\n## Why\n\nUser request\n\n## How\n\n## Tests\n\nAdded regression test",
          "timestamp": "2025-09-30T12:53:56-04:00",
          "tree_id": "ec6a192f3de17da7459676d2429d3b2f5640c7b5",
          "url": "https://github.com/paradedb/paradedb/commit/d5d310fcc28c6c692cbbcf7f8b86f61e806434a5"
        },
        "date": 1759253690264,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 4.673807,
            "unit": "median cpu",
            "extra": "avg cpu: 6.083151021384377, max cpu: 32.65306, count: 57160"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 236.5625,
            "unit": "median mem",
            "extra": "avg mem: 214.59024558257522, max mem: 240.62109375, count: 57160"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 58474,
            "unit": "median block_count",
            "extra": "avg block_count: 62611.963943317, max block_count: 75093.0, count: 57160"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 84,
            "unit": "median segment_count",
            "extra": "avg segment_count: 90.7, max segment_count: 191.0, count: 57160"
          },
          {
            "name": "Single Insert - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.880170439780515, max cpu: 27.77242, count: 57160"
          },
          {
            "name": "Single Insert - Primary - mem",
            "value": 170.9375,
            "unit": "median mem",
            "extra": "avg mem: 162.39389296492303, max mem: 171.3125, count: 57160"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.970554129921113, max cpu: 18.82353, count: 57160"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 164.87109375,
            "unit": "median mem",
            "extra": "avg mem: 162.21548620374824, max mem: 165.2578125, count: 57160"
          },
          {
            "name": "Top N - Primary - cpu",
            "value": 23.4375,
            "unit": "median cpu",
            "extra": "avg cpu: 23.78714624516332, max cpu: 33.768845, count: 57160"
          },
          {
            "name": "Top N - Primary - mem",
            "value": 154.93359375,
            "unit": "median mem",
            "extra": "avg mem: 170.50948400050297, max mem: 214.74609375, count: 57160"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "98e9eccf356ea70ff614bb8ed1fda12e1ffadfb7",
          "message": "feat: introduce ascii folding filter (#3242)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nIntroduces the ASCII filter.\n\n## Why\n\nUser request\n\n## How\n\n## Tests\n\nAdded regression test\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-09-30T14:05:24-04:00",
          "tree_id": "4e4d848be0995232f7dfbb9c2a4a681ea5a0025a",
          "url": "https://github.com/paradedb/paradedb/commit/98e9eccf356ea70ff614bb8ed1fda12e1ffadfb7"
        },
        "date": 1759257999102,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 5.634337046252619, max cpu: 28.374382, count: 56528"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 166.73828125,
            "unit": "median mem",
            "extra": "avg mem: 165.72321315045198, max mem: 166.73828125, count: 56528"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 58322,
            "unit": "median block_count",
            "extra": "avg block_count: 60630.32212354939, max block_count: 74499.0, count: 56528"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 83,
            "unit": "median segment_count",
            "extra": "avg segment_count: 90.04894919332013, max segment_count: 189.0, count: 56528"
          },
          {
            "name": "Single Insert - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.919982016370797, max cpu: 28.290766, count: 56528"
          },
          {
            "name": "Single Insert - Primary - mem",
            "value": 169.125,
            "unit": "median mem",
            "extra": "avg mem: 162.45835429455315, max mem: 169.125, count: 56528"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.860231614288085, max cpu: 14.035088, count: 56528"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 160.96875,
            "unit": "median mem",
            "extra": "avg mem: 158.52764441681555, max mem: 160.96875, count: 56528"
          },
          {
            "name": "Top N - Primary - cpu",
            "value": 23.346306,
            "unit": "median cpu",
            "extra": "avg cpu: 23.927526065814572, max cpu: 33.267326, count: 56528"
          },
          {
            "name": "Top N - Primary - mem",
            "value": 155.6171875,
            "unit": "median mem",
            "extra": "avg mem: 169.62456043631033, max mem: 215.44140625, count: 56528"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ming.ying.nyc@gmail.com",
            "name": "Ming",
            "username": "rebasedming"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "da4cfff239fd8e2e318591df7095f4cac4987a4b",
          "message": "fix: Correctly handle `COUNT(<column>)` (#3243)\n\n# Ticket(s) Closed\n\n- Closes #3196 \n\n## What\n\nBefore, any `COUNT(<column>)` was getting rewritten to a count of the\n\"ctid\" field, which is incorrect because it doesn't correctly handle\nnull values.\n\n## Why\n\n## How\n\n## Tests\n\nSee regression tests",
          "timestamp": "2025-09-30T16:35:47-04:00",
          "tree_id": "4e13feab234146d47e8e600f153bb9a27fe8383e",
          "url": "https://github.com/paradedb/paradedb/commit/da4cfff239fd8e2e318591df7095f4cac4987a4b"
        },
        "date": 1759267009796,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 5.67344915486886, max cpu: 28.458496, count: 56516"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 166.8671875,
            "unit": "median mem",
            "extra": "avg mem: 165.84130800625044, max mem: 167.24609375, count: 56516"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 59412,
            "unit": "median block_count",
            "extra": "avg block_count: 60876.11074739897, max block_count: 75624.0, count: 56516"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 86,
            "unit": "median segment_count",
            "extra": "avg segment_count: 92.90446953075235, max segment_count: 194.0, count: 56516"
          },
          {
            "name": "Single Insert - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.908608513017741, max cpu: 27.77242, count: 56516"
          },
          {
            "name": "Single Insert - Primary - mem",
            "value": 167.78515625,
            "unit": "median mem",
            "extra": "avg mem: 159.94618593904823, max mem: 168.53515625, count: 56516"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.928809569703183, max cpu: 28.042841, count: 56516"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 165.10546875,
            "unit": "median mem",
            "extra": "avg mem: 162.40720596545668, max mem: 165.10546875, count: 56516"
          },
          {
            "name": "Top N - Primary - cpu",
            "value": 23.369036,
            "unit": "median cpu",
            "extra": "avg cpu: 23.8924350015706, max cpu: 34.461536, count: 56516"
          },
          {
            "name": "Top N - Primary - mem",
            "value": 155.0625,
            "unit": "median mem",
            "extra": "avg mem: 168.50526185339106, max mem: 214.5390625, count: 56516"
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
        "date": 1758929155232,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 38.21160159350066,
            "unit": "median tps",
            "extra": "avg tps: 38.203352936279856, max tps: 39.83386115168159, count: 55435"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 75.76360004966051,
            "unit": "median tps",
            "extra": "avg tps: 127.19472166287738, max tps: 2970.747430309414, count: 55435"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1023.7088733700076,
            "unit": "median tps",
            "extra": "avg tps: 1019.2625713965293, max tps: 1038.61240014216, count: 55435"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 117.76480901939178,
            "unit": "median tps",
            "extra": "avg tps: 104.65550998519761, max tps: 819.1497654727461, count: 110870"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 18.989050045008536,
            "unit": "median tps",
            "extra": "avg tps: 19.059018510082048, max tps: 19.786499808143727, count: 55435"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ming.ying.nyc@gmail.com",
            "name": "Ming",
            "username": "rebasedming"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "d5d310fcc28c6c692cbbcf7f8b86f61e806434a5",
          "message": "feat: introduce ascii folding filter (#3241)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nIntroduces the ASCII filter.\n\n## Why\n\nUser request\n\n## How\n\n## Tests\n\nAdded regression test",
          "timestamp": "2025-09-30T12:53:56-04:00",
          "tree_id": "ec6a192f3de17da7459676d2429d3b2f5640c7b5",
          "url": "https://github.com/paradedb/paradedb/commit/d5d310fcc28c6c692cbbcf7f8b86f61e806434a5"
        },
        "date": 1759254416039,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 37.77840641645994,
            "unit": "median tps",
            "extra": "avg tps: 37.85250838089398, max tps: 39.58243009469522, count: 55468"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 131.75226156594783,
            "unit": "median tps",
            "extra": "avg tps: 174.21001285404327, max tps: 2831.49067523266, count: 55468"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1068.7627201342352,
            "unit": "median tps",
            "extra": "avg tps: 1062.7604189131312, max tps: 1075.8811998276385, count: 55468"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 118.33143183592924,
            "unit": "median tps",
            "extra": "avg tps: 122.78061207028286, max tps: 823.0572174207088, count: 110936"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 18.99500994631565,
            "unit": "median tps",
            "extra": "avg tps: 19.054227674474788, max tps: 19.93306567829608, count: 55468"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "98e9eccf356ea70ff614bb8ed1fda12e1ffadfb7",
          "message": "feat: introduce ascii folding filter (#3242)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nIntroduces the ASCII filter.\n\n## Why\n\nUser request\n\n## How\n\n## Tests\n\nAdded regression test\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-09-30T14:05:24-04:00",
          "tree_id": "4e4d848be0995232f7dfbb9c2a4a681ea5a0025a",
          "url": "https://github.com/paradedb/paradedb/commit/98e9eccf356ea70ff614bb8ed1fda12e1ffadfb7"
        },
        "date": 1759258712565,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 35.99361549202297,
            "unit": "median tps",
            "extra": "avg tps: 36.39886165982206, max tps: 38.86517735976115, count: 55373"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 131.37937034063742,
            "unit": "median tps",
            "extra": "avg tps: 173.6786117146981, max tps: 3025.2414622883966, count: 55373"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1042.2616794530556,
            "unit": "median tps",
            "extra": "avg tps: 1033.0651334369932, max tps: 1062.7910894466208, count: 55373"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 119.0469052916505,
            "unit": "median tps",
            "extra": "avg tps: 122.94964369015732, max tps: 806.7662656949658, count: 110746"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 17.89303874642869,
            "unit": "median tps",
            "extra": "avg tps: 18.227390943181693, max tps: 23.143779893627137, count: 55373"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ming.ying.nyc@gmail.com",
            "name": "Ming",
            "username": "rebasedming"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "da4cfff239fd8e2e318591df7095f4cac4987a4b",
          "message": "fix: Correctly handle `COUNT(<column>)` (#3243)\n\n# Ticket(s) Closed\n\n- Closes #3196 \n\n## What\n\nBefore, any `COUNT(<column>)` was getting rewritten to a count of the\n\"ctid\" field, which is incorrect because it doesn't correctly handle\nnull values.\n\n## Why\n\n## How\n\n## Tests\n\nSee regression tests",
          "timestamp": "2025-09-30T16:35:47-04:00",
          "tree_id": "4e13feab234146d47e8e600f153bb9a27fe8383e",
          "url": "https://github.com/paradedb/paradedb/commit/da4cfff239fd8e2e318591df7095f4cac4987a4b"
        },
        "date": 1759267724690,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 38.02357119292469,
            "unit": "median tps",
            "extra": "avg tps: 38.02140321632169, max tps: 39.31080243409729, count: 55519"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 131.2842478753356,
            "unit": "median tps",
            "extra": "avg tps: 173.78703072864633, max tps: 2913.468974737234, count: 55519"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1032.8024662274563,
            "unit": "median tps",
            "extra": "avg tps: 1024.4692669678816, max tps: 1039.0557613473575, count: 55519"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 119.36958403034747,
            "unit": "median tps",
            "extra": "avg tps: 122.28964651025113, max tps: 787.5872803642854, count: 111038"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 19.0147682149184,
            "unit": "median tps",
            "extra": "avg tps: 18.98140251610371, max tps: 21.652549596782027, count: 55519"
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
        "date": 1758929050856,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.568666,
            "unit": "median cpu",
            "extra": "avg cpu: 19.316821929443567, max cpu: 41.982506, count: 55594"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 158.15234375,
            "unit": "median mem",
            "extra": "avg mem: 143.77011458071016, max mem: 159.640625, count: 55594"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 7.656153469238252, max cpu: 28.09756, count: 55594"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 145.55078125,
            "unit": "median mem",
            "extra": "avg mem: 141.08939743385076, max mem: 146.3125, count: 55594"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 18.550726,
            "unit": "median cpu",
            "extra": "avg cpu: 19.23952412919167, max cpu: 73.63375, count: 55594"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 153.7421875,
            "unit": "median mem",
            "extra": "avg mem: 129.14320396535598, max mem: 163.06640625, count: 55594"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 21267,
            "unit": "median block_count",
            "extra": "avg block_count: 21582.900654746914, max block_count: 43151.0, count: 55594"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6153846,
            "unit": "median cpu",
            "extra": "avg cpu: 4.4544879032490865, max cpu: 4.669261, count: 55594"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 99.5625,
            "unit": "median mem",
            "extra": "avg mem: 89.03787735917994, max mem: 129.25390625, count: 55594"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 30,
            "unit": "median segment_count",
            "extra": "avg segment_count: 30.52088354858438, max segment_count: 47.0, count: 55594"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 18.60465,
            "unit": "median cpu",
            "extra": "avg cpu: 20.68858600627084, max cpu: 73.49282, count: 111188"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 166.42578125,
            "unit": "median mem",
            "extra": "avg mem: 152.0996670265114, max mem: 175.15234375, count: 111188"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.913043,
            "unit": "median cpu",
            "extra": "avg cpu: 14.132321357251415, max cpu: 28.070175, count: 55594"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 154.1640625,
            "unit": "median mem",
            "extra": "avg mem: 152.01298230530722, max mem: 155.43359375, count: 55594"
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
        "date": 1758929158481,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.532818,
            "unit": "median cpu",
            "extra": "avg cpu: 18.57447865365571, max cpu: 41.498558, count: 55435"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 154.7265625,
            "unit": "median mem",
            "extra": "avg mem: 136.8904498229909, max mem: 154.7265625, count: 55435"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 10.969566062606747, max cpu: 28.015566, count: 55435"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 112,
            "unit": "median mem",
            "extra": "avg mem: 110.75295750372058, max mem: 112.0, count: 55435"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 5.032393487480207, max cpu: 14.090019, count: 55435"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 121.953125,
            "unit": "median mem",
            "extra": "avg mem: 107.68799008692613, max mem: 142.5703125, count: 55435"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 24803,
            "unit": "median block_count",
            "extra": "avg block_count: 27031.334391629836, max block_count: 58035.0, count: 55435"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 2.6762904605081497, max cpu: 4.6511626, count: 55435"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 80.47265625,
            "unit": "median mem",
            "extra": "avg mem: 78.4853668006449, max mem: 123.62109375, count: 55435"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 31,
            "unit": "median segment_count",
            "extra": "avg segment_count: 31.362117795616488, max segment_count: 52.0, count: 55435"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.302325,
            "unit": "median cpu",
            "extra": "avg cpu: 11.516838843241773, max cpu: 28.290766, count: 110870"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 149.828125,
            "unit": "median mem",
            "extra": "avg mem: 132.9337774533237, max mem: 156.09765625, count: 110870"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.793103,
            "unit": "median cpu",
            "extra": "avg cpu: 11.815974978666805, max cpu: 27.799229, count: 55435"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 155.5625,
            "unit": "median mem",
            "extra": "avg mem: 153.92821864852982, max mem: 157.67578125, count: 55435"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ming.ying.nyc@gmail.com",
            "name": "Ming",
            "username": "rebasedming"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "d5d310fcc28c6c692cbbcf7f8b86f61e806434a5",
          "message": "feat: introduce ascii folding filter (#3241)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nIntroduces the ASCII filter.\n\n## Why\n\nUser request\n\n## How\n\n## Tests\n\nAdded regression test",
          "timestamp": "2025-09-30T12:53:56-04:00",
          "tree_id": "ec6a192f3de17da7459676d2429d3b2f5640c7b5",
          "url": "https://github.com/paradedb/paradedb/commit/d5d310fcc28c6c692cbbcf7f8b86f61e806434a5"
        },
        "date": 1759254418589,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.550726,
            "unit": "median cpu",
            "extra": "avg cpu: 18.673012684582012, max cpu: 57.427715, count: 55468"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 156.71484375,
            "unit": "median mem",
            "extra": "avg mem: 142.20857951037084, max mem: 157.46484375, count: 55468"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 9.278761714985805, max cpu: 28.042841, count: 55468"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 111.80859375,
            "unit": "median mem",
            "extra": "avg mem: 110.62235975303328, max mem: 111.80859375, count: 55468"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.991060016925942, max cpu: 9.467456, count: 55468"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 140.9765625,
            "unit": "median mem",
            "extra": "avg mem: 113.50871082200548, max mem: 142.58203125, count: 55468"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 28081,
            "unit": "median block_count",
            "extra": "avg block_count: 29366.06196365472, max block_count: 61782.0, count: 55468"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.542068597235828, max cpu: 4.7105007, count: 55468"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 88.69921875,
            "unit": "median mem",
            "extra": "avg mem: 82.55250366765523, max mem: 125.46875, count: 55468"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 31,
            "unit": "median segment_count",
            "extra": "avg segment_count: 31.365453955433765, max segment_count: 52.0, count: 55468"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.284333,
            "unit": "median cpu",
            "extra": "avg cpu: 10.789152351842965, max cpu: 33.4995, count: 110936"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 148.046875,
            "unit": "median mem",
            "extra": "avg mem: 136.66838657255084, max mem: 156.53515625, count: 110936"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.846154,
            "unit": "median cpu",
            "extra": "avg cpu: 12.432355950721771, max cpu: 27.934044, count: 55468"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 156.98828125,
            "unit": "median mem",
            "extra": "avg mem: 155.49071600123045, max mem: 159.78125, count: 55468"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "98e9eccf356ea70ff614bb8ed1fda12e1ffadfb7",
          "message": "feat: introduce ascii folding filter (#3242)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nIntroduces the ASCII filter.\n\n## Why\n\nUser request\n\n## How\n\n## Tests\n\nAdded regression test\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-09-30T14:05:24-04:00",
          "tree_id": "4e4d848be0995232f7dfbb9c2a4a681ea5a0025a",
          "url": "https://github.com/paradedb/paradedb/commit/98e9eccf356ea70ff614bb8ed1fda12e1ffadfb7"
        },
        "date": 1759258715107,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.550726,
            "unit": "median cpu",
            "extra": "avg cpu: 19.06549852672669, max cpu: 46.64723, count: 55373"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 156.41796875,
            "unit": "median mem",
            "extra": "avg mem: 142.05577028459268, max mem: 156.79296875, count: 55373"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 9.328294176273939, max cpu: 28.09756, count: 55373"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 110.38671875,
            "unit": "median mem",
            "extra": "avg mem: 109.03436439013599, max mem: 110.38671875, count: 55373"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.9544039808906275, max cpu: 13.819577, count: 55373"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 143.37109375,
            "unit": "median mem",
            "extra": "avg mem: 116.00172939372528, max mem: 144.12109375, count: 55373"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 28187,
            "unit": "median block_count",
            "extra": "avg block_count: 29536.105466563127, max block_count: 62390.0, count: 55373"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.569430355244172, max cpu: 4.7197638, count: 55373"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 90.5546875,
            "unit": "median mem",
            "extra": "avg mem: 84.06516297710527, max mem: 126.578125, count: 55373"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 31,
            "unit": "median segment_count",
            "extra": "avg segment_count: 31.486753471908692, max segment_count: 54.0, count: 55373"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.275363,
            "unit": "median cpu",
            "extra": "avg cpu: 10.661349354468912, max cpu: 28.374382, count: 110746"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 148.15234375,
            "unit": "median mem",
            "extra": "avg mem: 136.87968343664784, max mem: 157.203125, count: 110746"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.872832,
            "unit": "median cpu",
            "extra": "avg cpu: 13.143288841176217, max cpu: 27.87996, count: 55373"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 157.16015625,
            "unit": "median mem",
            "extra": "avg mem: 155.122777360469, max mem: 159.41015625, count: 55373"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ming.ying.nyc@gmail.com",
            "name": "Ming",
            "username": "rebasedming"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "da4cfff239fd8e2e318591df7095f4cac4987a4b",
          "message": "fix: Correctly handle `COUNT(<column>)` (#3243)\n\n# Ticket(s) Closed\n\n- Closes #3196 \n\n## What\n\nBefore, any `COUNT(<column>)` was getting rewritten to a count of the\n\"ctid\" field, which is incorrect because it doesn't correctly handle\nnull values.\n\n## Why\n\n## How\n\n## Tests\n\nSee regression tests",
          "timestamp": "2025-09-30T16:35:47-04:00",
          "tree_id": "4e13feab234146d47e8e600f153bb9a27fe8383e",
          "url": "https://github.com/paradedb/paradedb/commit/da4cfff239fd8e2e318591df7095f4cac4987a4b"
        },
        "date": 1759267728199,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.514948,
            "unit": "median cpu",
            "extra": "avg cpu: 18.65332389199137, max cpu: 46.198265, count: 55519"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 156.58984375,
            "unit": "median mem",
            "extra": "avg mem: 141.12554436600533, max mem: 158.078125, count: 55519"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 9.306572945947323, max cpu: 42.31146, count: 55519"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 111.65625,
            "unit": "median mem",
            "extra": "avg mem: 110.39963092589925, max mem: 111.65625, count: 55519"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.955574285650869, max cpu: 9.476802, count: 55519"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 141.046875,
            "unit": "median mem",
            "extra": "avg mem: 113.20561330918244, max mem: 141.421875, count: 55519"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 27881,
            "unit": "median block_count",
            "extra": "avg block_count: 29025.72780489562, max block_count: 60806.0, count: 55519"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.412432549664421, max cpu: 4.6647234, count: 55519"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 92.1875,
            "unit": "median mem",
            "extra": "avg mem: 85.70595848943604, max mem: 128.19921875, count: 55519"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 31,
            "unit": "median segment_count",
            "extra": "avg segment_count: 31.46259838974045, max segment_count: 56.0, count: 55519"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.266409,
            "unit": "median cpu",
            "extra": "avg cpu: 10.638069421009057, max cpu: 42.31146, count: 111038"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 147.8359375,
            "unit": "median mem",
            "extra": "avg mem: 135.99979289888373, max mem: 155.68359375, count: 111038"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.832853,
            "unit": "median cpu",
            "extra": "avg cpu: 12.606592893583443, max cpu: 27.988338, count: 55519"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 156.94921875,
            "unit": "median mem",
            "extra": "avg mem: 155.02605850587187, max mem: 158.484375, count: 55519"
          }
        ]
      }
    ]
  }
}