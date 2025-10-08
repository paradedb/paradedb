window.BENCHMARK_DATA = {
  "lastUpdate": 1759946329900,
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
          "id": "52dd41fee48d3a635a315610631235ff09ac53a5",
          "message": "chore: Upgrade to `0.18.10` (#3251)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-10-01T10:56:42-04:00",
          "tree_id": "fca37808df8505e2c564f422c635f688f82a0e1d",
          "url": "https://github.com/paradedb/paradedb/commit/52dd41fee48d3a635a315610631235ff09ac53a5"
        },
        "date": 1759331649281,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 816.923813838722,
            "unit": "median tps",
            "extra": "avg tps: 817.0877983742098, max tps: 875.7408611302866, count: 55277"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3283.103289574748,
            "unit": "median tps",
            "extra": "avg tps: 3265.1830977964773, max tps: 3414.7729651912127, count: 55277"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 795.1778573532961,
            "unit": "median tps",
            "extra": "avg tps: 794.8497158957564, max tps: 843.4987696751513, count: 55277"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 700.0235442717767,
            "unit": "median tps",
            "extra": "avg tps: 696.6446630708938, max tps: 703.8712445481704, count: 55277"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1701.6969733218239,
            "unit": "median tps",
            "extra": "avg tps: 1696.8196491213662, max tps: 1722.1616162733344, count: 110554"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1288.7289539699802,
            "unit": "median tps",
            "extra": "avg tps: 1278.359861247354, max tps: 1291.609781991485, count: 55277"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 149.2175303166966,
            "unit": "median tps",
            "extra": "avg tps: 154.80606824583134, max tps: 566.275144032083, count: 55277"
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
        "date": 1759332601576,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 826.495227847459,
            "unit": "median tps",
            "extra": "avg tps: 825.9696688100971, max tps: 831.8274305586292, count: 55151"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3341.7720452123353,
            "unit": "median tps",
            "extra": "avg tps: 3326.365544234124, max tps: 3366.6196055531855, count: 55151"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 774.11309424682,
            "unit": "median tps",
            "extra": "avg tps: 774.5871512349001, max tps: 858.8357301862585, count: 55151"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 677.1327566772102,
            "unit": "median tps",
            "extra": "avg tps: 676.2949286728219, max tps: 702.1093515129337, count: 55151"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1674.2368944016855,
            "unit": "median tps",
            "extra": "avg tps: 1666.8907588028026, max tps: 1684.4116768991817, count: 110302"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1260.163488656551,
            "unit": "median tps",
            "extra": "avg tps: 1248.5105830116818, max tps: 1265.862434979998, count: 55151"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 123.5534617185738,
            "unit": "median tps",
            "extra": "avg tps: 170.55780929007673, max tps: 1077.5885292856235, count: 55151"
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
          "id": "bb494d330cfac7ee1db3b904ab5266bd671abadb",
          "message": "fix: `paradedb.snippet` projects correctly when called multiple times over the same field (#3257)\n\n# Ticket(s) Closed\n\n- Closes #3256 \n\n## What\n\nSee GH issue\n\n## Why\n\n## How\n\nWhen mapping a const node to our list of snippet generators, we were\nmatching only on the field name and function OID, which is not unique\nenough -- it doesn't account for the function arguments.\n\n## Tests\n\nAdded regression test",
          "timestamp": "2025-10-01T19:08:56-04:00",
          "tree_id": "a78e472aad70351c0c799699330da58256d3cbc4",
          "url": "https://github.com/paradedb/paradedb/commit/bb494d330cfac7ee1db3b904ab5266bd671abadb"
        },
        "date": 1759361097786,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 798.502285319091,
            "unit": "median tps",
            "extra": "avg tps: 797.8717625767025, max tps: 847.2926688052166, count: 55297"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3454.2239962987833,
            "unit": "median tps",
            "extra": "avg tps: 3427.1079101637924, max tps: 3462.9405642262855, count: 55297"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 811.8913314400511,
            "unit": "median tps",
            "extra": "avg tps: 810.8586161000968, max tps: 840.3607250908574, count: 55297"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 695.7194955442135,
            "unit": "median tps",
            "extra": "avg tps: 692.7814951013194, max tps: 699.1933511703924, count: 55297"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1728.4269811606755,
            "unit": "median tps",
            "extra": "avg tps: 1725.2105157833832, max tps: 1747.9388479667814, count: 110594"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1281.4089751223698,
            "unit": "median tps",
            "extra": "avg tps: 1273.3831981322523, max tps: 1287.8610795502361, count: 55297"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 131.09748869344446,
            "unit": "median tps",
            "extra": "avg tps: 160.95614817876333, max tps: 594.8369343769946, count: 55297"
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
          "id": "762c487685bb5635c789b31781155839d2c65cf0",
          "message": "feat: Configure a limit/offset for snippets (#3254)",
          "timestamp": "2025-10-01T20:00:17-04:00",
          "tree_id": "5dcb534f0e2f513864b19abddc44396ed24760ff",
          "url": "https://github.com/paradedb/paradedb/commit/762c487685bb5635c789b31781155839d2c65cf0"
        },
        "date": 1759364265633,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 770.6704495780041,
            "unit": "median tps",
            "extra": "avg tps: 773.1372156756638, max tps: 833.3059036806706, count: 55454"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3351.8992576987057,
            "unit": "median tps",
            "extra": "avg tps: 3340.0276351825487, max tps: 3392.0673370880227, count: 55454"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 767.7310193032201,
            "unit": "median tps",
            "extra": "avg tps: 768.7258425099952, max tps: 817.3048959199474, count: 55454"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 656.3724695496356,
            "unit": "median tps",
            "extra": "avg tps: 655.2319679617839, max tps: 678.6791519198171, count: 55454"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1673.4704448517416,
            "unit": "median tps",
            "extra": "avg tps: 1689.3598121186742, max tps: 1726.599219738657, count: 110908"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1248.8247692245359,
            "unit": "median tps",
            "extra": "avg tps: 1239.6276884585172, max tps: 1260.6609341549224, count: 55454"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 84.00063923722817,
            "unit": "median tps",
            "extra": "avg tps: 114.6168365150626, max tps: 551.6008836646156, count: 55454"
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
          "id": "850e3a9f88033d64151d6ecfa0d37c1b1f210b27",
          "message": "fix: `paradedb.snippet` projects correctly when called multiple times over the same field (#3258)\n\n# Ticket(s) Closed\n\n- Closes #3256 \n\n## What\n\nSee GH issue\n\n## Why\n\n## How\n\nWhen mapping a const node to our list of snippet generators, we were\nmatching only on the field name and function OID, which is not unique\nenough -- it doesn't account for the function arguments.\n\n## Tests\n\nAdded regression test\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-10-01T21:33:38-04:00",
          "tree_id": "1f119fb29d59884bd2105114833ca2d34c2acfd9",
          "url": "https://github.com/paradedb/paradedb/commit/850e3a9f88033d64151d6ecfa0d37c1b1f210b27"
        },
        "date": 1759369781526,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 781.9302213409401,
            "unit": "median tps",
            "extra": "avg tps: 780.1643018539881, max tps: 806.911561885554, count: 55198"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3321.381415245117,
            "unit": "median tps",
            "extra": "avg tps: 3310.4051543058818, max tps: 3459.320221032682, count: 55198"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 793.8685634087634,
            "unit": "median tps",
            "extra": "avg tps: 794.4432286873612, max tps: 849.4838606123444, count: 55198"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 650.2472529755773,
            "unit": "median tps",
            "extra": "avg tps: 650.9663926909482, max tps: 663.5091249162726, count: 55198"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1688.4380955597098,
            "unit": "median tps",
            "extra": "avg tps: 1682.2790156988226, max tps: 1703.6859645183781, count: 110396"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1281.570702932479,
            "unit": "median tps",
            "extra": "avg tps: 1269.4776061522255, max tps: 1285.233998818768, count: 55198"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 78.29028931229742,
            "unit": "median tps",
            "extra": "avg tps: 94.8064962896591, max tps: 944.5858699399715, count: 55198"
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
          "id": "2ce078f8496ff0152f7373fe94348a9cfcacd5af",
          "message": "feat: Configure a limit/offset for snippets (#3259)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n`paradedb.snippet` and `paradedb.snippet_positions` now take a limit and\noffset. For instance, if 5 snippets are found in a doc and offset is 1,\nthen the first snippet will be skipped.\n\n## Why\n\n## How\n\n## Tests\n\nSee regression test\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-10-01T22:10:01-04:00",
          "tree_id": "904048e17b5e988830cd9302f007cb5d18411d22",
          "url": "https://github.com/paradedb/paradedb/commit/2ce078f8496ff0152f7373fe94348a9cfcacd5af"
        },
        "date": 1759371964603,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 805.1580098707456,
            "unit": "median tps",
            "extra": "avg tps: 806.1997189182468, max tps: 855.8665754312897, count: 55355"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3334.354320322837,
            "unit": "median tps",
            "extra": "avg tps: 3316.436075855813, max tps: 3406.941311562263, count: 55355"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 790.462141427006,
            "unit": "median tps",
            "extra": "avg tps: 790.781279772559, max tps: 860.121315480126, count: 55355"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 682.5437879866138,
            "unit": "median tps",
            "extra": "avg tps: 681.7680351613977, max tps: 695.9504729828546, count: 55355"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1732.9638293633147,
            "unit": "median tps",
            "extra": "avg tps: 1719.651286140296, max tps: 1742.6861434370778, count: 110710"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1285.113219071312,
            "unit": "median tps",
            "extra": "avg tps: 1275.705682351796, max tps: 1291.478870896015, count: 55355"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 101.65473844640965,
            "unit": "median tps",
            "extra": "avg tps: 109.3375306636458, max tps: 882.4154888625928, count: 55355"
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
          "id": "6335204e0cc58bdeb1ea12f236ab2258a44cd192",
          "message": "chore: Upgrade to `0.18.11` (#3261)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-10-02T10:00:31-04:00",
          "tree_id": "05d18415099467cbf114e4a87c3b536bfeafb509",
          "url": "https://github.com/paradedb/paradedb/commit/6335204e0cc58bdeb1ea12f236ab2258a44cd192"
        },
        "date": 1759414691013,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 760.3498761979048,
            "unit": "median tps",
            "extra": "avg tps: 763.0748895671371, max tps: 851.922248102683, count: 54467"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2978.194193329264,
            "unit": "median tps",
            "extra": "avg tps: 2968.066493987471, max tps: 3101.798351359343, count: 54467"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 754.9824815067418,
            "unit": "median tps",
            "extra": "avg tps: 757.0835782548046, max tps: 810.8755958797922, count: 54467"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 649.6245544275155,
            "unit": "median tps",
            "extra": "avg tps: 647.0672068631002, max tps: 665.736006889976, count: 54467"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1705.204532821377,
            "unit": "median tps",
            "extra": "avg tps: 1692.3652434466057, max tps: 1726.859547459734, count: 108934"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1234.4099634668426,
            "unit": "median tps",
            "extra": "avg tps: 1221.8011909478896, max tps: 1239.3007765789987, count: 54467"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 78.61397424037189,
            "unit": "median tps",
            "extra": "avg tps: 93.09598805427576, max tps: 541.5956273735428, count: 54467"
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
          "id": "7f394c9cc9b86839fb39f22123893552f6e5a291",
          "message": "chore: Upgrade to `0.18.11` (#3262)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-10-02T10:04:40-04:00",
          "tree_id": "fd395f6aeafa74dd786657eebbf6f9cba6e6bf28",
          "url": "https://github.com/paradedb/paradedb/commit/7f394c9cc9b86839fb39f22123893552f6e5a291"
        },
        "date": 1759414941067,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 773.5982995517772,
            "unit": "median tps",
            "extra": "avg tps: 775.3732647056972, max tps: 811.8456099131637, count: 55246"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3373.561834291452,
            "unit": "median tps",
            "extra": "avg tps: 3340.791907870608, max tps: 3429.95887889408, count: 55246"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 786.1770567522344,
            "unit": "median tps",
            "extra": "avg tps: 785.9971496528207, max tps: 813.5272210879916, count: 55246"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 683.3400608921594,
            "unit": "median tps",
            "extra": "avg tps: 680.8885171519521, max tps: 687.8823786818218, count: 55246"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1720.7033735504851,
            "unit": "median tps",
            "extra": "avg tps: 1709.107963461, max tps: 1729.7663401234838, count: 110492"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1283.0094811273943,
            "unit": "median tps",
            "extra": "avg tps: 1275.8801937947812, max tps: 1287.5979922858546, count: 55246"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 76.60800880808242,
            "unit": "median tps",
            "extra": "avg tps: 88.3326496101523, max tps: 559.11181733146, count: 55246"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "stuhood@paradedb.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "6b15d1a8e21e76267b06b7e54dbbb5194fda55b9",
          "message": "chore: Improve determinism of property tests. (#3220)\n\n## What\n\nSet a seed to control what is generated by `random()` in the property\ntests, and render it in the reproduction script after a failure.\n\n## Why\n\nTo make it easier to reproduce property test failures by running over\nreproducible data.\n\n`proptest` failures are only directly reproducible via their reported\nseed if they are not dependent on the randomly generated data in the\ntable that we test against. We can't re-generate the table and index for\nevery `proptest` query that we run, because it would take way too long\nto run a reasonable number of iterations. And we don't want to run on\nstatic data, because then we might never catch data-dependent bugs like\n#3266.\n\nSetting a seed allows us to run on random data, but still reproduce\nfailures later. And for cases where failures aren't data-dependent, the\n`proptest` repro seed (e.g. `cc 08176a8c0ae10938a...`) can still be used\ndirectly.",
          "timestamp": "2025-10-03T13:48:13-07:00",
          "tree_id": "ac9934e860da0bdb5b4b083df652ebc3d85309d4",
          "url": "https://github.com/paradedb/paradedb/commit/6b15d1a8e21e76267b06b7e54dbbb5194fda55b9"
        },
        "date": 1759525452850,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 768.5874505857715,
            "unit": "median tps",
            "extra": "avg tps: 768.5228476722157, max tps: 803.6304072530261, count: 55423"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3400.1127057988742,
            "unit": "median tps",
            "extra": "avg tps: 3348.915814900233, max tps: 3423.0094423935584, count: 55423"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 794.9116295281475,
            "unit": "median tps",
            "extra": "avg tps: 793.3972305743581, max tps: 879.4035855072799, count: 55423"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 655.0460026657626,
            "unit": "median tps",
            "extra": "avg tps: 649.8201421443703, max tps: 658.4829448783231, count: 55423"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1681.5912782537043,
            "unit": "median tps",
            "extra": "avg tps: 1687.0978933534927, max tps: 1736.3162382392334, count: 110846"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1257.3669599140237,
            "unit": "median tps",
            "extra": "avg tps: 1242.5163376843584, max tps: 1267.713655570639, count: 55423"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 99.88595014347582,
            "unit": "median tps",
            "extra": "avg tps: 111.01591148239066, max tps: 1049.3047306854478, count: 55423"
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
          "id": "34e2d538b7327fc30b32f6ee33b55cbc9ccb2749",
          "message": "chore: Remove deprecated tokenizers: `en_stem`, `stem` and `lowercase` (#3279)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nFor better maintainability:\n\n1. Removes three extremely deprecated tokenizers: `en_stem`, `stem`, and\n`lowercase`\n2. Wraps the filter builders in a macro, guaranteeing that all the\nfilters are applied to all tokenizers\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-10-07T11:16:19-04:00",
          "tree_id": "7ef65d52251d2f4fb83439b6887924fa19564416",
          "url": "https://github.com/paradedb/paradedb/commit/34e2d538b7327fc30b32f6ee33b55cbc9ccb2749"
        },
        "date": 1759851153013,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 791.6090556418156,
            "unit": "median tps",
            "extra": "avg tps: 791.323160284694, max tps: 869.83608842205, count: 54847"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3315.706661836092,
            "unit": "median tps",
            "extra": "avg tps: 3286.3407745102454, max tps: 3346.6667511514097, count: 54847"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 792.3096184383185,
            "unit": "median tps",
            "extra": "avg tps: 792.9046232923478, max tps: 864.3081467863012, count: 54847"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 661.5144620608378,
            "unit": "median tps",
            "extra": "avg tps: 660.9464703895552, max tps: 677.2281143576424, count: 54847"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1729.2075479941393,
            "unit": "median tps",
            "extra": "avg tps: 1719.3495006059477, max tps: 1741.4385317722954, count: 109694"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1288.5253804918996,
            "unit": "median tps",
            "extra": "avg tps: 1278.2278329730339, max tps: 1294.8478240750944, count: 54847"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 119.64742403797058,
            "unit": "median tps",
            "extra": "avg tps: 136.43161437024673, max tps: 510.73801131897585, count: 54847"
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
          "id": "7f394c9cc9b86839fb39f22123893552f6e5a291",
          "message": "chore: Upgrade to `0.18.11` (#3262)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-10-02T10:04:40-04:00",
          "tree_id": "fd395f6aeafa74dd786657eebbf6f9cba6e6bf28",
          "url": "https://github.com/paradedb/paradedb/commit/7f394c9cc9b86839fb39f22123893552f6e5a291"
        },
        "date": 1759857188791,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 784.2344780367984,
            "unit": "median tps",
            "extra": "avg tps: 783.7157096077271, max tps: 873.4845268253393, count: 54959"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3299.3121534643014,
            "unit": "median tps",
            "extra": "avg tps: 3279.636716084482, max tps: 3318.3820686686, count: 54959"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 783.4134684295888,
            "unit": "median tps",
            "extra": "avg tps: 784.9866711493004, max tps: 884.1438362768969, count: 54959"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 648.4293264003605,
            "unit": "median tps",
            "extra": "avg tps: 647.6264191779142, max tps: 711.4451967412762, count: 54959"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1711.416127644729,
            "unit": "median tps",
            "extra": "avg tps: 1705.3789691542872, max tps: 1719.719832626081, count: 109918"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1295.2679921867182,
            "unit": "median tps",
            "extra": "avg tps: 1286.075071209951, max tps: 1297.946891951824, count: 54959"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 135.6284263453009,
            "unit": "median tps",
            "extra": "avg tps: 143.4002794259883, max tps: 897.5299079403574, count: 54959"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "b8ebd1ad157e946d6fff8f775aec3189bc469325",
          "message": "fix: possible naming collisions with builder functions (#3275)\n\n## What\n\nFix an issue where functions tagged with our `#[builder_fn]` macro could\nend up with the same name.\n\n## Why\n\nIt's come up in CI once or twice and I've seen it locally as well\n\n## How\n\n## Tests",
          "timestamp": "2025-10-07T15:35:22-04:00",
          "tree_id": "bd88762bb48211807276741ce46d155ea36600b3",
          "url": "https://github.com/paradedb/paradedb/commit/b8ebd1ad157e946d6fff8f775aec3189bc469325"
        },
        "date": 1759866689340,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 795.7061625551728,
            "unit": "median tps",
            "extra": "avg tps: 796.882949558859, max tps: 827.9853193332381, count: 54755"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3227.9197321323186,
            "unit": "median tps",
            "extra": "avg tps: 3222.5233494174768, max tps: 3241.3921822123575, count: 54755"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 795.2933441114268,
            "unit": "median tps",
            "extra": "avg tps: 795.8240769968279, max tps: 923.6423651942963, count: 54755"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 680.640258782533,
            "unit": "median tps",
            "extra": "avg tps: 677.1622977262657, max tps: 685.2249551495538, count: 54755"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1702.6337906697468,
            "unit": "median tps",
            "extra": "avg tps: 1710.1771127367472, max tps: 1748.4029646032518, count: 109510"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1299.211805777736,
            "unit": "median tps",
            "extra": "avg tps: 1289.9174004119634, max tps: 1306.8195612850177, count: 54755"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 125.28967551546148,
            "unit": "median tps",
            "extra": "avg tps: 155.6537703222638, max tps: 1028.810818151515, count: 54755"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "2f61e5c2d8fb03377c37dcd10c558e9967781b97",
          "message": "feat: A Free Space Manager fronted by an AVL tree (#3252)\n\n## What\n\nThis implements a new `v2` FSM that's fronted by an AVL tree which\nallows for minimal locking during extension and draining. It also\nprovides efficient continuation during drain as xid blocklists are\nexhausted or found to be unavailable to the current transaction. And it\nimplements a (simple) transparent conversion of the current `v1` FSM to\nthe new format.\n\nAdditionally, this fixes a problem with background merging where more\nthan one background merger process could be spawned at once -- I've seen\nup to 8 concurrently. It does this by introducing some a new page on\ndisk to track the process and coordinate locking.\n\n## Why\n\nOur current FSM is very heavyweight in terms of lock contention. This\nshould get us to something that isn't.\n\n## How\n\n## Tests\n\nA number of new tests for the array-backed AVL tree and the FSM itself.\nAll existing tests also pass and, at least, the `wide-table.toml`\nstressgres shows a slight performance improvement for the update jobs.\n\n---------\n\nSigned-off-by: Eric Ridge <eebbrr@gmail.com>\nCo-authored-by: Stu Hood <stuhood@paradedb.com>",
          "timestamp": "2025-10-07T15:36:47-04:00",
          "tree_id": "31410dd4c4d2be73d287e97485f4d0faaf1b2932",
          "url": "https://github.com/paradedb/paradedb/commit/2f61e5c2d8fb03377c37dcd10c558e9967781b97"
        },
        "date": 1759866772625,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 792.4110887355787,
            "unit": "median tps",
            "extra": "avg tps: 791.1855589419616, max tps: 854.8155085269984, count: 55311"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3345.272329571123,
            "unit": "median tps",
            "extra": "avg tps: 3316.348359930314, max tps: 3364.341848934668, count: 55311"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 801.6707022349822,
            "unit": "median tps",
            "extra": "avg tps: 799.5498140579039, max tps: 811.2379933903844, count: 55311"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 695.5113521163062,
            "unit": "median tps",
            "extra": "avg tps: 690.8731817982662, max tps: 701.5322973047848, count: 55311"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1756.0244111062666,
            "unit": "median tps",
            "extra": "avg tps: 1745.2348087698683, max tps: 1773.9746922129575, count: 110622"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1308.1923849603909,
            "unit": "median tps",
            "extra": "avg tps: 1294.9791702915386, max tps: 1316.9201368218687, count: 55311"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 144.66869144484983,
            "unit": "median tps",
            "extra": "avg tps: 158.7255452432347, max tps: 1082.4092264562464, count: 55311"
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
          "id": "96f6d1fa999bb11ba37313786636681219629534",
          "message": "chore: revert \"chore: Remove deprecated tokenizers: `en_stem`, `stem` and `lowercase` (#3279)\"\n\nThis reverts commit 34e2d538b7327fc30b32f6ee33b55cbc9ccb2749.\n\n\nRequested by @eeeebbbbrrrr due to conflicts with the SQL UX work, will\nre-open later",
          "timestamp": "2025-10-08T13:42:37-04:00",
          "tree_id": "9db0fcd9402217db0c7f51702ef447664a0831f4",
          "url": "https://github.com/paradedb/paradedb/commit/96f6d1fa999bb11ba37313786636681219629534"
        },
        "date": 1759946324904,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 813.3584955045874,
            "unit": "median tps",
            "extra": "avg tps: 813.1268119334272, max tps: 861.9742521449818, count: 55184"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3367.790255746837,
            "unit": "median tps",
            "extra": "avg tps: 3353.0600383416286, max tps: 3420.460442530339, count: 55184"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 807.8804075513674,
            "unit": "median tps",
            "extra": "avg tps: 806.8990558277161, max tps: 839.0962712032594, count: 55184"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 701.6658202707715,
            "unit": "median tps",
            "extra": "avg tps: 699.6054818212547, max tps: 723.8243086260512, count: 55184"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1741.0880384798359,
            "unit": "median tps",
            "extra": "avg tps: 1728.0503272134438, max tps: 1762.5820796246173, count: 110368"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1298.2819086568877,
            "unit": "median tps",
            "extra": "avg tps: 1286.5038862267893, max tps: 1304.3707892225336, count: 55184"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 116.42924779268425,
            "unit": "median tps",
            "extra": "avg tps: 121.25762406559934, max tps: 655.7222255736914, count: 55184"
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
        "date": 1759331283327,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.900950940489428, max cpu: 14.754097, count: 55344"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 153.8515625,
            "unit": "median mem",
            "extra": "avg mem: 137.77684198772675, max mem: 153.8515625, count: 55344"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.630076699255669, max cpu: 9.230769, count: 55344"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 27.27734375,
            "unit": "median mem",
            "extra": "avg mem: 27.504993201611736, max mem: 31.53125, count: 55344"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.841831530415032, max cpu: 13.93998, count: 55344"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 154.15234375,
            "unit": "median mem",
            "extra": "avg mem: 138.15642680609008, max mem: 154.15234375, count: 55344"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.461360845891656, max cpu: 9.29332, count: 55344"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 155.2109375,
            "unit": "median mem",
            "extra": "avg mem: 138.8830164092991, max mem: 155.5859375, count: 55344"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.687616540501086, max cpu: 9.599999, count: 110688"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 152.46484375,
            "unit": "median mem",
            "extra": "avg mem: 135.94690308428872, max mem: 156.70703125, count: 110688"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 27852,
            "unit": "median block_count",
            "extra": "avg block_count: 27897.099812084416, max block_count: 53680.0, count: 55344"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 30,
            "unit": "median segment_count",
            "extra": "avg segment_count: 29.99862677074299, max segment_count: 75.0, count: 55344"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.6039604541158345, max cpu: 9.467456, count: 55344"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 149.98828125,
            "unit": "median mem",
            "extra": "avg mem: 135.10520696405663, max mem: 157.12890625, count: 55344"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 5.205977453108888, max cpu: 9.257474, count: 55344"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 148.01171875,
            "unit": "median mem",
            "extra": "avg mem: 129.0966062123943, max mem: 151.859375, count: 55344"
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
          "id": "52dd41fee48d3a635a315610631235ff09ac53a5",
          "message": "chore: Upgrade to `0.18.10` (#3251)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-10-01T10:56:42-04:00",
          "tree_id": "fca37808df8505e2c564f422c635f688f82a0e1d",
          "url": "https://github.com/paradedb/paradedb/commit/52dd41fee48d3a635a315610631235ff09ac53a5"
        },
        "date": 1759331652066,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.839337598241216, max cpu: 9.657948, count: 55277"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 153.953125,
            "unit": "median mem",
            "extra": "avg mem: 139.03825788754816, max mem: 153.953125, count: 55277"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.6239721126173405, max cpu: 9.284333, count: 55277"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 26.83984375,
            "unit": "median mem",
            "extra": "avg mem: 27.046738330363443, max mem: 32.65234375, count: 55277"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.8585294244547965, max cpu: 14.428859, count: 55277"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 154.3046875,
            "unit": "median mem",
            "extra": "avg mem: 139.61424104679614, max mem: 154.6796875, count: 55277"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.490861859321355, max cpu: 4.738401, count: 55277"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 153.37890625,
            "unit": "median mem",
            "extra": "avg mem: 138.3879954168325, max mem: 153.37890625, count: 55277"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.676418412523148, max cpu: 9.60961, count: 110554"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 153.30078125,
            "unit": "median mem",
            "extra": "avg mem: 136.91835907819933, max mem: 155.73828125, count: 110554"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 29121,
            "unit": "median block_count",
            "extra": "avg block_count: 28914.243935090544, max block_count: 55340.0, count: 55277"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 30,
            "unit": "median segment_count",
            "extra": "avg segment_count: 30.05184796570002, max segment_count: 74.0, count: 55277"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.621426713595528, max cpu: 9.421001, count: 55277"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 152.34375,
            "unit": "median mem",
            "extra": "avg mem: 136.74350345758634, max mem: 157.25390625, count: 55277"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.295782848794089, max cpu: 4.7477746, count: 55277"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 149.03515625,
            "unit": "median mem",
            "extra": "avg mem: 128.89663273999585, max mem: 151.34375, count: 55277"
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
        "date": 1759332605206,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.857225067297755, max cpu: 9.886715, count: 55151"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 153.5078125,
            "unit": "median mem",
            "extra": "avg mem: 136.88948551476855, max mem: 153.5078125, count: 55151"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.664553510349637, max cpu: 9.619239, count: 55151"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 25.63671875,
            "unit": "median mem",
            "extra": "avg mem: 26.056295868388606, max mem: 28.77734375, count: 55151"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.899501070022115, max cpu: 14.830072, count: 55151"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 153.640625,
            "unit": "median mem",
            "extra": "avg mem: 137.3409362052592, max mem: 153.640625, count: 55151"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.453104884099258, max cpu: 4.743083, count: 55151"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 154.30859375,
            "unit": "median mem",
            "extra": "avg mem: 137.43015241677395, max mem: 154.30859375, count: 55151"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.732303595196305, max cpu: 9.486166, count: 110302"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 153.26953125,
            "unit": "median mem",
            "extra": "avg mem: 135.8541170632672, max mem: 157.4453125, count: 110302"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 26648,
            "unit": "median block_count",
            "extra": "avg block_count: 26916.922304219326, max block_count: 52985.0, count: 55151"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 30,
            "unit": "median segment_count",
            "extra": "avg segment_count: 29.968559046980108, max segment_count: 74.0, count: 55151"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.6917812705446975, max cpu: 9.448819, count: 55151"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 152.44921875,
            "unit": "median mem",
            "extra": "avg mem: 135.56749070166452, max mem: 157.32421875, count: 55151"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.468066280928109, max cpu: 4.729064, count: 55151"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 144.47265625,
            "unit": "median mem",
            "extra": "avg mem: 125.5854796660532, max mem: 150.73828125, count: 55151"
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
          "id": "bb494d330cfac7ee1db3b904ab5266bd671abadb",
          "message": "fix: `paradedb.snippet` projects correctly when called multiple times over the same field (#3257)\n\n# Ticket(s) Closed\n\n- Closes #3256 \n\n## What\n\nSee GH issue\n\n## Why\n\n## How\n\nWhen mapping a const node to our list of snippet generators, we were\nmatching only on the field name and function OID, which is not unique\nenough -- it doesn't account for the function arguments.\n\n## Tests\n\nAdded regression test",
          "timestamp": "2025-10-01T19:08:56-04:00",
          "tree_id": "a78e472aad70351c0c799699330da58256d3cbc4",
          "url": "https://github.com/paradedb/paradedb/commit/bb494d330cfac7ee1db3b904ab5266bd671abadb"
        },
        "date": 1759361100408,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.862614133529323, max cpu: 14.145383, count: 55297"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 153.43359375,
            "unit": "median mem",
            "extra": "avg mem: 137.53543047938857, max mem: 153.43359375, count: 55297"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.5862235039030015, max cpu: 4.824121, count: 55297"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 25.0078125,
            "unit": "median mem",
            "extra": "avg mem: 24.583337760185906, max mem: 25.0078125, count: 55297"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.879964400576345, max cpu: 15.763547, count: 55297"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 153.984375,
            "unit": "median mem",
            "extra": "avg mem: 138.49356147259346, max mem: 153.984375, count: 55297"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.64108873563932, max cpu: 9.257474, count: 55297"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 153.65625,
            "unit": "median mem",
            "extra": "avg mem: 137.94369290773008, max mem: 154.03125, count: 55297"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.684815150352333, max cpu: 9.495549, count: 110594"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 152.078125,
            "unit": "median mem",
            "extra": "avg mem: 136.07863729044976, max mem: 157.14453125, count: 110594"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 26677,
            "unit": "median block_count",
            "extra": "avg block_count: 27270.871638606073, max block_count: 53277.0, count: 55297"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 30,
            "unit": "median segment_count",
            "extra": "avg segment_count: 30.073114273830406, max segment_count: 74.0, count: 55297"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.562129631589592, max cpu: 9.302325, count: 55297"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 151.1171875,
            "unit": "median mem",
            "extra": "avg mem: 135.14603202366314, max mem: 157.1484375, count: 55297"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6153846,
            "unit": "median cpu",
            "extra": "avg cpu: 4.063849950356665, max cpu: 4.738401, count: 55297"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 145.80078125,
            "unit": "median mem",
            "extra": "avg mem: 128.28531766528926, max mem: 150.5234375, count: 55297"
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
          "id": "762c487685bb5635c789b31781155839d2c65cf0",
          "message": "feat: Configure a limit/offset for snippets (#3254)",
          "timestamp": "2025-10-01T20:00:17-04:00",
          "tree_id": "5dcb534f0e2f513864b19abddc44396ed24760ff",
          "url": "https://github.com/paradedb/paradedb/commit/762c487685bb5635c789b31781155839d2c65cf0"
        },
        "date": 1759364268850,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.937760697626918, max cpu: 14.530776, count: 55454"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 154.03515625,
            "unit": "median mem",
            "extra": "avg mem: 136.6970304216107, max mem: 154.03515625, count: 55454"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.6119810877753, max cpu: 9.495549, count: 55454"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 25.171875,
            "unit": "median mem",
            "extra": "avg mem: 25.251939600276806, max mem: 28.68359375, count: 55454"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.924890563169668, max cpu: 15.311005, count: 55454"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 154.09765625,
            "unit": "median mem",
            "extra": "avg mem: 136.9783883376312, max mem: 154.47265625, count: 55454"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.646763041178429, max cpu: 9.4395275, count: 55454"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 153.109375,
            "unit": "median mem",
            "extra": "avg mem: 135.51198755387347, max mem: 153.109375, count: 55454"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.721224296949073, max cpu: 9.504951, count: 110908"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 153.72265625,
            "unit": "median mem",
            "extra": "avg mem: 134.8429223855583, max mem: 156.6875, count: 110908"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 26616,
            "unit": "median block_count",
            "extra": "avg block_count: 26960.232661304864, max block_count: 53943.0, count: 55454"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 30,
            "unit": "median segment_count",
            "extra": "avg segment_count: 30.070725285822483, max segment_count: 75.0, count: 55454"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.620179900645621, max cpu: 9.619239, count: 55454"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 153.2578125,
            "unit": "median mem",
            "extra": "avg mem: 135.17115410407726, max mem: 157.3984375, count: 55454"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.540374865305191, max cpu: 9.29332, count: 55454"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 144.6171875,
            "unit": "median mem",
            "extra": "avg mem: 125.4867170312331, max mem: 149.66796875, count: 55454"
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
          "id": "850e3a9f88033d64151d6ecfa0d37c1b1f210b27",
          "message": "fix: `paradedb.snippet` projects correctly when called multiple times over the same field (#3258)\n\n# Ticket(s) Closed\n\n- Closes #3256 \n\n## What\n\nSee GH issue\n\n## Why\n\n## How\n\nWhen mapping a const node to our list of snippet generators, we were\nmatching only on the field name and function OID, which is not unique\nenough -- it doesn't account for the function arguments.\n\n## Tests\n\nAdded regression test\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-10-01T21:33:38-04:00",
          "tree_id": "1f119fb29d59884bd2105114833ca2d34c2acfd9",
          "url": "https://github.com/paradedb/paradedb/commit/850e3a9f88033d64151d6ecfa0d37c1b1f210b27"
        },
        "date": 1759369784414,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.910612391197031, max cpu: 14.486921, count: 55198"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 152.0625,
            "unit": "median mem",
            "extra": "avg mem: 137.6461953586407, max mem: 152.81640625, count: 55198"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.52538296915615, max cpu: 9.514371, count: 55198"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 26.90625,
            "unit": "median mem",
            "extra": "avg mem: 26.976610622214572, max mem: 30.45703125, count: 55198"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.865554398327846, max cpu: 13.953489, count: 55198"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 155.03515625,
            "unit": "median mem",
            "extra": "avg mem: 140.07587139423077, max mem: 155.03515625, count: 55198"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.579764220051909, max cpu: 4.824121, count: 55198"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 154.94140625,
            "unit": "median mem",
            "extra": "avg mem: 139.98689186259014, max mem: 154.94140625, count: 55198"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.619320524452236, max cpu: 9.448819, count: 110396"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 152.9765625,
            "unit": "median mem",
            "extra": "avg mem: 137.3022341516563, max mem: 157.37109375, count: 110396"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 28480,
            "unit": "median block_count",
            "extra": "avg block_count: 28433.412696112176, max block_count: 55789.0, count: 55198"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 31,
            "unit": "median segment_count",
            "extra": "avg segment_count: 30.32138845610348, max segment_count: 75.0, count: 55198"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.527287625983633, max cpu: 9.657948, count: 55198"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 152.77734375,
            "unit": "median mem",
            "extra": "avg mem: 136.33916211298416, max mem: 155.78125, count: 55198"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 3.8946081058548776, max cpu: 4.7571855, count: 55198"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 146.6953125,
            "unit": "median mem",
            "extra": "avg mem: 129.48592241227038, max mem: 149.41796875, count: 55198"
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
          "id": "2ce078f8496ff0152f7373fe94348a9cfcacd5af",
          "message": "feat: Configure a limit/offset for snippets (#3259)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n`paradedb.snippet` and `paradedb.snippet_positions` now take a limit and\noffset. For instance, if 5 snippets are found in a doc and offset is 1,\nthen the first snippet will be skipped.\n\n## Why\n\n## How\n\n## Tests\n\nSee regression test\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-10-01T22:10:01-04:00",
          "tree_id": "904048e17b5e988830cd9302f007cb5d18411d22",
          "url": "https://github.com/paradedb/paradedb/commit/2ce078f8496ff0152f7373fe94348a9cfcacd5af"
        },
        "date": 1759371967405,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.835440623739381, max cpu: 11.669368, count: 55355"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 154.65234375,
            "unit": "median mem",
            "extra": "avg mem: 138.87341400110648, max mem: 154.65234375, count: 55355"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.614410079266092, max cpu: 9.60961, count: 55355"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 24.9375,
            "unit": "median mem",
            "extra": "avg mem: 25.87046591319664, max mem: 30.0234375, count: 55355"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.853850303045765, max cpu: 13.980582, count: 55355"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 154.2109375,
            "unit": "median mem",
            "extra": "avg mem: 138.8126879205808, max mem: 154.2109375, count: 55355"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.53763524068165, max cpu: 4.743083, count: 55355"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 154.20703125,
            "unit": "median mem",
            "extra": "avg mem: 138.6006171105817, max mem: 154.20703125, count: 55355"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.658624757490824, max cpu: 9.486166, count: 110710"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 153.375,
            "unit": "median mem",
            "extra": "avg mem: 136.91404612839852, max mem: 155.296875, count: 110710"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 26806,
            "unit": "median block_count",
            "extra": "avg block_count: 27307.338993767502, max block_count: 53591.0, count: 55355"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 30,
            "unit": "median segment_count",
            "extra": "avg segment_count: 30.050456146689548, max segment_count: 75.0, count: 55355"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.5389921257745485, max cpu: 9.311348, count: 55355"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 151.9140625,
            "unit": "median mem",
            "extra": "avg mem: 136.74633706587028, max mem: 157.1796875, count: 55355"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6153846,
            "unit": "median cpu",
            "extra": "avg cpu: 3.0051855579173683, max cpu: 9.4395275, count: 55355"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 146.609375,
            "unit": "median mem",
            "extra": "avg mem: 129.39908036762714, max mem: 149.42578125, count: 55355"
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
          "id": "6335204e0cc58bdeb1ea12f236ab2258a44cd192",
          "message": "chore: Upgrade to `0.18.11` (#3261)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-10-02T10:00:31-04:00",
          "tree_id": "05d18415099467cbf114e4a87c3b536bfeafb509",
          "url": "https://github.com/paradedb/paradedb/commit/6335204e0cc58bdeb1ea12f236ab2258a44cd192"
        },
        "date": 1759414694436,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.983261693540908, max cpu: 14.443329, count: 54467"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 154.58203125,
            "unit": "median mem",
            "extra": "avg mem: 138.0621124373474, max mem: 154.58203125, count: 54467"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.595895604989292, max cpu: 9.302325, count: 54467"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 27.01953125,
            "unit": "median mem",
            "extra": "avg mem: 26.828495995855288, max mem: 30.140625, count: 54467"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.985953414254323, max cpu: 14.501511, count: 54467"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 154.99609375,
            "unit": "median mem",
            "extra": "avg mem: 138.43000607018928, max mem: 155.37109375, count: 54467"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.6051222123834314, max cpu: 4.843592, count: 54467"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 155.140625,
            "unit": "median mem",
            "extra": "avg mem: 138.48249986803935, max mem: 155.140625, count: 54467"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.65120001062419, max cpu: 9.514371, count: 108934"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 153.05078125,
            "unit": "median mem",
            "extra": "avg mem: 135.48152368205749, max mem: 157.03125, count: 108934"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 26267,
            "unit": "median block_count",
            "extra": "avg block_count: 26738.5094828061, max block_count: 52020.0, count: 54467"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 30,
            "unit": "median segment_count",
            "extra": "avg segment_count: 29.89077790221602, max segment_count: 74.0, count: 54467"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.553464534837353, max cpu: 9.266409, count: 54467"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 153.01953125,
            "unit": "median mem",
            "extra": "avg mem: 136.81929403641195, max mem: 158.3203125, count: 54467"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.668938975632673, max cpu: 9.347614, count: 54467"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 146.18359375,
            "unit": "median mem",
            "extra": "avg mem: 128.09540158777332, max mem: 151.35546875, count: 54467"
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
          "id": "7f394c9cc9b86839fb39f22123893552f6e5a291",
          "message": "chore: Upgrade to `0.18.11` (#3262)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-10-02T10:04:40-04:00",
          "tree_id": "fd395f6aeafa74dd786657eebbf6f9cba6e6bf28",
          "url": "https://github.com/paradedb/paradedb/commit/7f394c9cc9b86839fb39f22123893552f6e5a291"
        },
        "date": 1759414943722,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.876336890889598, max cpu: 14.604463, count: 55246"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 154.37890625,
            "unit": "median mem",
            "extra": "avg mem: 139.78342768876934, max mem: 154.37890625, count: 55246"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.6026744143356115, max cpu: 9.476802, count: 55246"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 26.91015625,
            "unit": "median mem",
            "extra": "avg mem: 26.873664849604495, max mem: 30.92578125, count: 55246"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.8920933377601115, max cpu: 14.604463, count: 55246"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 154.38671875,
            "unit": "median mem",
            "extra": "avg mem: 139.75908224690474, max mem: 154.38671875, count: 55246"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.452501379422018, max cpu: 4.7477746, count: 55246"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 154.59765625,
            "unit": "median mem",
            "extra": "avg mem: 139.71267765709283, max mem: 154.59765625, count: 55246"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.677079396472786, max cpu: 9.430255, count: 110492"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 151.0234375,
            "unit": "median mem",
            "extra": "avg mem: 136.62156546825787, max mem: 155.65234375, count: 110492"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 27568,
            "unit": "median block_count",
            "extra": "avg block_count: 27754.709535531983, max block_count: 53733.0, count: 55246"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 30,
            "unit": "median segment_count",
            "extra": "avg segment_count: 30.068023024291353, max segment_count: 75.0, count: 55246"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.520191919455395, max cpu: 9.266409, count: 55246"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 153.359375,
            "unit": "median mem",
            "extra": "avg mem: 137.8082098138327, max mem: 158.24609375, count: 55246"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.372664663808989, max cpu: 7.482463, count: 55246"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 147.14453125,
            "unit": "median mem",
            "extra": "avg mem: 130.30077389652644, max mem: 151.1015625, count: 55246"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "stuhood@paradedb.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "6b15d1a8e21e76267b06b7e54dbbb5194fda55b9",
          "message": "chore: Improve determinism of property tests. (#3220)\n\n## What\n\nSet a seed to control what is generated by `random()` in the property\ntests, and render it in the reproduction script after a failure.\n\n## Why\n\nTo make it easier to reproduce property test failures by running over\nreproducible data.\n\n`proptest` failures are only directly reproducible via their reported\nseed if they are not dependent on the randomly generated data in the\ntable that we test against. We can't re-generate the table and index for\nevery `proptest` query that we run, because it would take way too long\nto run a reasonable number of iterations. And we don't want to run on\nstatic data, because then we might never catch data-dependent bugs like\n#3266.\n\nSetting a seed allows us to run on random data, but still reproduce\nfailures later. And for cases where failures aren't data-dependent, the\n`proptest` repro seed (e.g. `cc 08176a8c0ae10938a...`) can still be used\ndirectly.",
          "timestamp": "2025-10-03T13:48:13-07:00",
          "tree_id": "ac9934e860da0bdb5b4b083df652ebc3d85309d4",
          "url": "https://github.com/paradedb/paradedb/commit/6b15d1a8e21e76267b06b7e54dbbb5194fda55b9"
        },
        "date": 1759525455522,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.883741975897486, max cpu: 14.215202, count: 55423"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 157.03515625,
            "unit": "median mem",
            "extra": "avg mem: 139.91145429244176, max mem: 157.03515625, count: 55423"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.70526564301935, max cpu: 9.467456, count: 55423"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 25.9453125,
            "unit": "median mem",
            "extra": "avg mem: 25.93730420470743, max mem: 29.0546875, count: 55423"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.878599103571746, max cpu: 13.714285, count: 55423"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 154.84765625,
            "unit": "median mem",
            "extra": "avg mem: 138.68505904306426, max mem: 155.22265625, count: 55423"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.565455860668511, max cpu: 9.311348, count: 55423"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 154.73046875,
            "unit": "median mem",
            "extra": "avg mem: 138.02148581985367, max mem: 154.73046875, count: 55423"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.681163230775522, max cpu: 9.60961, count: 110846"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 153.9140625,
            "unit": "median mem",
            "extra": "avg mem: 136.4545951914706, max mem: 156.9375, count: 110846"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 27670,
            "unit": "median block_count",
            "extra": "avg block_count: 27758.5408043592, max block_count: 54838.0, count: 55423"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 30,
            "unit": "median segment_count",
            "extra": "avg segment_count: 30.082546957039497, max segment_count: 74.0, count: 55423"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.527089636117885, max cpu: 4.824121, count: 55423"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 153.0078125,
            "unit": "median mem",
            "extra": "avg mem: 135.6210263000018, max mem: 156.7578125, count: 55423"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 4.9139202383793075, max cpu: 9.430255, count: 55423"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 147.41015625,
            "unit": "median mem",
            "extra": "avg mem: 127.63577353377659, max mem: 151.50390625, count: 55423"
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
          "id": "34e2d538b7327fc30b32f6ee33b55cbc9ccb2749",
          "message": "chore: Remove deprecated tokenizers: `en_stem`, `stem` and `lowercase` (#3279)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nFor better maintainability:\n\n1. Removes three extremely deprecated tokenizers: `en_stem`, `stem`, and\n`lowercase`\n2. Wraps the filter builders in a macro, guaranteeing that all the\nfilters are applied to all tokenizers\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-10-07T11:16:19-04:00",
          "tree_id": "7ef65d52251d2f4fb83439b6887924fa19564416",
          "url": "https://github.com/paradedb/paradedb/commit/34e2d538b7327fc30b32f6ee33b55cbc9ccb2749"
        },
        "date": 1759851155679,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.970788867517273, max cpu: 14.754097, count: 54847"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 154.62890625,
            "unit": "median mem",
            "extra": "avg mem: 138.4863636285941, max mem: 154.62890625, count: 54847"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.543853335286027, max cpu: 7.5471697, count: 54847"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 27.6796875,
            "unit": "median mem",
            "extra": "avg mem: 28.217191830227723, max mem: 32.85546875, count: 54847"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.933392367336915, max cpu: 14.545454, count: 54847"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 154.6171875,
            "unit": "median mem",
            "extra": "avg mem: 138.588753999763, max mem: 154.9921875, count: 54847"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.592494835113612, max cpu: 4.83871, count: 54847"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 154.34765625,
            "unit": "median mem",
            "extra": "avg mem: 137.6309772811184, max mem: 154.34765625, count: 54847"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.6803774916065635, max cpu: 9.81595, count: 109694"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 153.00390625,
            "unit": "median mem",
            "extra": "avg mem: 136.06860334202418, max mem: 156.69921875, count: 109694"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 25906,
            "unit": "median block_count",
            "extra": "avg block_count: 26324.677958685068, max block_count: 52555.0, count: 54847"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 30,
            "unit": "median segment_count",
            "extra": "avg segment_count: 29.7609531970755, max segment_count: 75.0, count: 54847"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.554875690814143, max cpu: 9.320388, count: 54847"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 151.69140625,
            "unit": "median mem",
            "extra": "avg mem: 136.5021863376757, max mem: 158.4765625, count: 54847"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6875,
            "unit": "median cpu",
            "extra": "avg cpu: 4.122481198090026, max cpu: 4.7571855, count: 54847"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 147.91015625,
            "unit": "median mem",
            "extra": "avg mem: 128.80161175634038, max mem: 151.78125, count: 54847"
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
          "id": "7f394c9cc9b86839fb39f22123893552f6e5a291",
          "message": "chore: Upgrade to `0.18.11` (#3262)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-10-02T10:04:40-04:00",
          "tree_id": "fd395f6aeafa74dd786657eebbf6f9cba6e6bf28",
          "url": "https://github.com/paradedb/paradedb/commit/7f394c9cc9b86839fb39f22123893552f6e5a291"
        },
        "date": 1759857191471,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.954215389342905, max cpu: 14.814815, count: 54959"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 154.51171875,
            "unit": "median mem",
            "extra": "avg mem: 137.58608597716025, max mem: 154.51171875, count: 54959"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.556090039560432, max cpu: 9.4395275, count: 54959"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 25.609375,
            "unit": "median mem",
            "extra": "avg mem: 26.47992160917684, max mem: 30.3671875, count: 54959"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.943424880785832, max cpu: 14.784394, count: 54959"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 155.546875,
            "unit": "median mem",
            "extra": "avg mem: 138.8704635216707, max mem: 155.921875, count: 54959"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.518882943319716, max cpu: 4.7999997, count: 54959"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 155.1640625,
            "unit": "median mem",
            "extra": "avg mem: 138.05739974913118, max mem: 155.55859375, count: 54959"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.7216724562892365, max cpu: 9.775968, count: 109918"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 156.36328125,
            "unit": "median mem",
            "extra": "avg mem: 138.2011130243113, max mem: 157.86328125, count: 109918"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 27199,
            "unit": "median block_count",
            "extra": "avg block_count: 26748.198820939244, max block_count: 52135.0, count: 54959"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 30,
            "unit": "median segment_count",
            "extra": "avg segment_count: 29.466165687148603, max segment_count: 73.0, count: 54959"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.582981539305938, max cpu: 9.275363, count: 54959"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 153.28515625,
            "unit": "median mem",
            "extra": "avg mem: 136.4299489875407, max mem: 158.19921875, count: 54959"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.610951,
            "unit": "median cpu",
            "extra": "avg cpu: 4.379820710697388, max cpu: 9.495549, count: 54959"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 148.6484375,
            "unit": "median mem",
            "extra": "avg mem: 128.65352559692226, max mem: 151.41796875, count: 54959"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "b8ebd1ad157e946d6fff8f775aec3189bc469325",
          "message": "fix: possible naming collisions with builder functions (#3275)\n\n## What\n\nFix an issue where functions tagged with our `#[builder_fn]` macro could\nend up with the same name.\n\n## Why\n\nIt's come up in CI once or twice and I've seen it locally as well\n\n## How\n\n## Tests",
          "timestamp": "2025-10-07T15:35:22-04:00",
          "tree_id": "bd88762bb48211807276741ce46d155ea36600b3",
          "url": "https://github.com/paradedb/paradedb/commit/b8ebd1ad157e946d6fff8f775aec3189bc469325"
        },
        "date": 1759866691981,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.94115208266753, max cpu: 14.738997, count: 54755"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 154.25390625,
            "unit": "median mem",
            "extra": "avg mem: 138.39035240788513, max mem: 154.62890625, count: 54755"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.5376596085853, max cpu: 9.421001, count: 54755"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 26.44140625,
            "unit": "median mem",
            "extra": "avg mem: 26.896404366610355, max mem: 30.42578125, count: 54755"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.943217884942461, max cpu: 14.738997, count: 54755"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 155.08984375,
            "unit": "median mem",
            "extra": "avg mem: 139.5029623436216, max mem: 155.08984375, count: 54755"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.499795745154522, max cpu: 4.7151275, count: 54755"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 154.58984375,
            "unit": "median mem",
            "extra": "avg mem: 138.6024847902018, max mem: 154.58984375, count: 54755"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.643398563083058, max cpu: 9.458128, count: 109510"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 152,
            "unit": "median mem",
            "extra": "avg mem: 135.4169001047279, max mem: 153.6875, count: 109510"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 28034,
            "unit": "median block_count",
            "extra": "avg block_count: 27399.81636380239, max block_count: 51882.0, count: 54755"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 30,
            "unit": "median segment_count",
            "extra": "avg segment_count: 29.732170578029404, max segment_count: 74.0, count: 54755"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.669678548110146, max cpu: 9.284333, count: 54755"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 151.0390625,
            "unit": "median mem",
            "extra": "avg mem: 135.9701396419277, max mem: 157.796875, count: 54755"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 3.626759368135436, max cpu: 4.7151275, count: 54755"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 146.7578125,
            "unit": "median mem",
            "extra": "avg mem: 128.4076317231303, max mem: 151.08203125, count: 54755"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "2f61e5c2d8fb03377c37dcd10c558e9967781b97",
          "message": "feat: A Free Space Manager fronted by an AVL tree (#3252)\n\n## What\n\nThis implements a new `v2` FSM that's fronted by an AVL tree which\nallows for minimal locking during extension and draining. It also\nprovides efficient continuation during drain as xid blocklists are\nexhausted or found to be unavailable to the current transaction. And it\nimplements a (simple) transparent conversion of the current `v1` FSM to\nthe new format.\n\nAdditionally, this fixes a problem with background merging where more\nthan one background merger process could be spawned at once -- I've seen\nup to 8 concurrently. It does this by introducing some a new page on\ndisk to track the process and coordinate locking.\n\n## Why\n\nOur current FSM is very heavyweight in terms of lock contention. This\nshould get us to something that isn't.\n\n## How\n\n## Tests\n\nA number of new tests for the array-backed AVL tree and the FSM itself.\nAll existing tests also pass and, at least, the `wide-table.toml`\nstressgres shows a slight performance improvement for the update jobs.\n\n---------\n\nSigned-off-by: Eric Ridge <eebbrr@gmail.com>\nCo-authored-by: Stu Hood <stuhood@paradedb.com>",
          "timestamp": "2025-10-07T15:36:47-04:00",
          "tree_id": "31410dd4c4d2be73d287e97485f4d0faaf1b2932",
          "url": "https://github.com/paradedb/paradedb/commit/2f61e5c2d8fb03377c37dcd10c558e9967781b97"
        },
        "date": 1759866775410,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.872772744043536, max cpu: 14.428859, count: 55311"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 154.03125,
            "unit": "median mem",
            "extra": "avg mem: 139.3950802055649, max mem: 154.03125, count: 55311"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.613343799480716, max cpu: 9.4395275, count: 55311"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 27.11328125,
            "unit": "median mem",
            "extra": "avg mem: 26.773867455162627, max mem: 29.5, count: 55311"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.882358797635232, max cpu: 13.859479, count: 55311"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 154.27734375,
            "unit": "median mem",
            "extra": "avg mem: 139.74128853494332, max mem: 154.65234375, count: 55311"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.565808638180557, max cpu: 9.284333, count: 55311"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 155.6328125,
            "unit": "median mem",
            "extra": "avg mem: 140.6187693225579, max mem: 155.6328125, count: 55311"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.672709809326492, max cpu: 9.458128, count: 110622"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 153.68359375,
            "unit": "median mem",
            "extra": "avg mem: 138.36430913804443, max mem: 156.734375, count: 110622"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 29178,
            "unit": "median block_count",
            "extra": "avg block_count: 29368.341107555458, max block_count: 57131.0, count: 55311"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 30,
            "unit": "median segment_count",
            "extra": "avg segment_count: 29.41380557212851, max segment_count: 58.0, count: 55311"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.579670965996234, max cpu: 7.494145, count: 55311"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 155.5859375,
            "unit": "median mem",
            "extra": "avg mem: 140.35238151938583, max mem: 160.08984375, count: 55311"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 3.9707340937774007, max cpu: 4.729064, count: 55311"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 147.6171875,
            "unit": "median mem",
            "extra": "avg mem: 130.14050543461065, max mem: 151.58984375, count: 55311"
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
          "id": "96f6d1fa999bb11ba37313786636681219629534",
          "message": "chore: revert \"chore: Remove deprecated tokenizers: `en_stem`, `stem` and `lowercase` (#3279)\"\n\nThis reverts commit 34e2d538b7327fc30b32f6ee33b55cbc9ccb2749.\n\n\nRequested by @eeeebbbbrrrr due to conflicts with the SQL UX work, will\nre-open later",
          "timestamp": "2025-10-08T13:42:37-04:00",
          "tree_id": "9db0fcd9402217db0c7f51702ef447664a0831f4",
          "url": "https://github.com/paradedb/paradedb/commit/96f6d1fa999bb11ba37313786636681219629534"
        },
        "date": 1759946328256,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.884470566941477, max cpu: 14.769231, count: 55184"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 154.7265625,
            "unit": "median mem",
            "extra": "avg mem: 138.8274995356444, max mem: 155.10546875, count: 55184"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.638021837483804, max cpu: 9.365853, count: 55184"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 27.11328125,
            "unit": "median mem",
            "extra": "avg mem: 26.844725146781677, max mem: 29.82421875, count: 55184"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.881373218217083, max cpu: 14.501511, count: 55184"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 155.4453125,
            "unit": "median mem",
            "extra": "avg mem: 139.46506807056846, max mem: 155.4453125, count: 55184"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.821467170199378, max cpu: 9.29332, count: 55184"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 155.140625,
            "unit": "median mem",
            "extra": "avg mem: 138.92238628667278, max mem: 155.140625, count: 55184"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.684929383756361, max cpu: 9.67742, count: 110368"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 153.0234375,
            "unit": "median mem",
            "extra": "avg mem: 136.64020984060144, max mem: 155.34765625, count: 110368"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 25958,
            "unit": "median block_count",
            "extra": "avg block_count: 26365.248242244128, max block_count: 50886.0, count: 55184"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 30,
            "unit": "median segment_count",
            "extra": "avg segment_count: 29.73387213685126, max segment_count: 58.0, count: 55184"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.408051354949315, max cpu: 4.8582993, count: 55184"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 154.5,
            "unit": "median mem",
            "extra": "avg mem: 137.99052884441141, max mem: 157.5625, count: 55184"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 3.841871172127477, max cpu: 4.7477746, count: 55184"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 147.24609375,
            "unit": "median mem",
            "extra": "avg mem: 128.95334408241067, max mem: 150.7890625, count: 55184"
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
          "id": "52dd41fee48d3a635a315610631235ff09ac53a5",
          "message": "chore: Upgrade to `0.18.10` (#3251)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-10-01T10:56:42-04:00",
          "tree_id": "fca37808df8505e2c564f422c635f688f82a0e1d",
          "url": "https://github.com/paradedb/paradedb/commit/52dd41fee48d3a635a315610631235ff09ac53a5"
        },
        "date": 1759332391159,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.370363909860528,
            "unit": "median tps",
            "extra": "avg tps: 7.1232802674229925, max tps: 11.050902962110202, count: 57328"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.351661018509944,
            "unit": "median tps",
            "extra": "avg tps: 4.843096926321027, max tps: 5.916017998463535, count: 57328"
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
        "date": 1759333346468,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.306119040440684,
            "unit": "median tps",
            "extra": "avg tps: 7.0985265686502315, max tps: 11.025198388395902, count: 57941"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.3155937432343645,
            "unit": "median tps",
            "extra": "avg tps: 4.804001324755545, max tps: 5.886228070436705, count: 57941"
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
          "id": "bb494d330cfac7ee1db3b904ab5266bd671abadb",
          "message": "fix: `paradedb.snippet` projects correctly when called multiple times over the same field (#3257)\n\n# Ticket(s) Closed\n\n- Closes #3256 \n\n## What\n\nSee GH issue\n\n## Why\n\n## How\n\nWhen mapping a const node to our list of snippet generators, we were\nmatching only on the field name and function OID, which is not unique\nenough -- it doesn't account for the function arguments.\n\n## Tests\n\nAdded regression test",
          "timestamp": "2025-10-01T19:08:56-04:00",
          "tree_id": "a78e472aad70351c0c799699330da58256d3cbc4",
          "url": "https://github.com/paradedb/paradedb/commit/bb494d330cfac7ee1db3b904ab5266bd671abadb"
        },
        "date": 1759361842671,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.207168957892966,
            "unit": "median tps",
            "extra": "avg tps: 6.982292698956268, max tps: 10.890658412573357, count: 57561"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.319765317823646,
            "unit": "median tps",
            "extra": "avg tps: 4.81587766553116, max tps: 5.909307779517827, count: 57561"
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
          "id": "762c487685bb5635c789b31781155839d2c65cf0",
          "message": "feat: Configure a limit/offset for snippets (#3254)",
          "timestamp": "2025-10-01T20:00:17-04:00",
          "tree_id": "5dcb534f0e2f513864b19abddc44396ed24760ff",
          "url": "https://github.com/paradedb/paradedb/commit/762c487685bb5635c789b31781155839d2c65cf0"
        },
        "date": 1759365009469,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.50559564723408,
            "unit": "median tps",
            "extra": "avg tps: 7.261227638261076, max tps: 11.308153628914628, count: 57820"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.226284888106244,
            "unit": "median tps",
            "extra": "avg tps: 4.731818993367936, max tps: 5.788977788072201, count: 57820"
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
          "id": "850e3a9f88033d64151d6ecfa0d37c1b1f210b27",
          "message": "fix: `paradedb.snippet` projects correctly when called multiple times over the same field (#3258)\n\n# Ticket(s) Closed\n\n- Closes #3256 \n\n## What\n\nSee GH issue\n\n## Why\n\n## How\n\nWhen mapping a const node to our list of snippet generators, we were\nmatching only on the field name and function OID, which is not unique\nenough -- it doesn't account for the function arguments.\n\n## Tests\n\nAdded regression test\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-10-01T21:33:38-04:00",
          "tree_id": "1f119fb29d59884bd2105114833ca2d34c2acfd9",
          "url": "https://github.com/paradedb/paradedb/commit/850e3a9f88033d64151d6ecfa0d37c1b1f210b27"
        },
        "date": 1759370543244,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.326343651033197,
            "unit": "median tps",
            "extra": "avg tps: 7.12002340612955, max tps: 11.001834258205852, count: 57556"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.2678133444051465,
            "unit": "median tps",
            "extra": "avg tps: 4.761399021573558, max tps: 5.851000648920031, count: 57556"
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
          "id": "2ce078f8496ff0152f7373fe94348a9cfcacd5af",
          "message": "feat: Configure a limit/offset for snippets (#3259)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n`paradedb.snippet` and `paradedb.snippet_positions` now take a limit and\noffset. For instance, if 5 snippets are found in a doc and offset is 1,\nthen the first snippet will be skipped.\n\n## Why\n\n## How\n\n## Tests\n\nSee regression test\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-10-01T22:10:01-04:00",
          "tree_id": "904048e17b5e988830cd9302f007cb5d18411d22",
          "url": "https://github.com/paradedb/paradedb/commit/2ce078f8496ff0152f7373fe94348a9cfcacd5af"
        },
        "date": 1759372709659,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.30800843330055,
            "unit": "median tps",
            "extra": "avg tps: 7.057381056450715, max tps: 10.974817479029749, count: 57297"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.37552312537329,
            "unit": "median tps",
            "extra": "avg tps: 4.867103373411656, max tps: 5.9761176264785245, count: 57297"
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
          "id": "6335204e0cc58bdeb1ea12f236ab2258a44cd192",
          "message": "chore: Upgrade to `0.18.11` (#3261)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-10-02T10:00:31-04:00",
          "tree_id": "05d18415099467cbf114e4a87c3b536bfeafb509",
          "url": "https://github.com/paradedb/paradedb/commit/6335204e0cc58bdeb1ea12f236ab2258a44cd192"
        },
        "date": 1759415441367,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.390563307640603,
            "unit": "median tps",
            "extra": "avg tps: 7.180379986020035, max tps: 11.171829853448706, count: 57913"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.2701091145217935,
            "unit": "median tps",
            "extra": "avg tps: 4.764271087657846, max tps: 5.83880971116397, count: 57913"
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
          "id": "7f394c9cc9b86839fb39f22123893552f6e5a291",
          "message": "chore: Upgrade to `0.18.11` (#3262)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-10-02T10:04:40-04:00",
          "tree_id": "fd395f6aeafa74dd786657eebbf6f9cba6e6bf28",
          "url": "https://github.com/paradedb/paradedb/commit/7f394c9cc9b86839fb39f22123893552f6e5a291"
        },
        "date": 1759415688370,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.35407262623278,
            "unit": "median tps",
            "extra": "avg tps: 7.132735746466516, max tps: 11.109148288815218, count: 57907"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 4.894140149982829,
            "unit": "median tps",
            "extra": "avg tps: 4.4484504512602, max tps: 5.398103495114528, count: 57907"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "stuhood@paradedb.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "6b15d1a8e21e76267b06b7e54dbbb5194fda55b9",
          "message": "chore: Improve determinism of property tests. (#3220)\n\n## What\n\nSet a seed to control what is generated by `random()` in the property\ntests, and render it in the reproduction script after a failure.\n\n## Why\n\nTo make it easier to reproduce property test failures by running over\nreproducible data.\n\n`proptest` failures are only directly reproducible via their reported\nseed if they are not dependent on the randomly generated data in the\ntable that we test against. We can't re-generate the table and index for\nevery `proptest` query that we run, because it would take way too long\nto run a reasonable number of iterations. And we don't want to run on\nstatic data, because then we might never catch data-dependent bugs like\n#3266.\n\nSetting a seed allows us to run on random data, but still reproduce\nfailures later. And for cases where failures aren't data-dependent, the\n`proptest` repro seed (e.g. `cc 08176a8c0ae10938a...`) can still be used\ndirectly.",
          "timestamp": "2025-10-03T13:48:13-07:00",
          "tree_id": "ac9934e860da0bdb5b4b083df652ebc3d85309d4",
          "url": "https://github.com/paradedb/paradedb/commit/6b15d1a8e21e76267b06b7e54dbbb5194fda55b9"
        },
        "date": 1759526209538,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.192483188009284,
            "unit": "median tps",
            "extra": "avg tps: 7.000851897446091, max tps: 10.893622740294543, count: 57791"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.372552392224057,
            "unit": "median tps",
            "extra": "avg tps: 4.853079900099778, max tps: 5.965724945619175, count: 57791"
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
          "id": "34e2d538b7327fc30b32f6ee33b55cbc9ccb2749",
          "message": "chore: Remove deprecated tokenizers: `en_stem`, `stem` and `lowercase` (#3279)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nFor better maintainability:\n\n1. Removes three extremely deprecated tokenizers: `en_stem`, `stem`, and\n`lowercase`\n2. Wraps the filter builders in a macro, guaranteeing that all the\nfilters are applied to all tokenizers\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-10-07T11:16:19-04:00",
          "tree_id": "7ef65d52251d2f4fb83439b6887924fa19564416",
          "url": "https://github.com/paradedb/paradedb/commit/34e2d538b7327fc30b32f6ee33b55cbc9ccb2749"
        },
        "date": 1759851829592,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.314033886629874,
            "unit": "median tps",
            "extra": "avg tps: 7.069765968465518, max tps: 10.951814817826302, count: 57270"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.368144580827352,
            "unit": "median tps",
            "extra": "avg tps: 4.862333386963252, max tps: 5.965499086673725, count: 57270"
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
          "id": "7f394c9cc9b86839fb39f22123893552f6e5a291",
          "message": "chore: Upgrade to `0.18.11` (#3262)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-10-02T10:04:40-04:00",
          "tree_id": "fd395f6aeafa74dd786657eebbf6f9cba6e6bf28",
          "url": "https://github.com/paradedb/paradedb/commit/7f394c9cc9b86839fb39f22123893552f6e5a291"
        },
        "date": 1759857864513,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.362984380837235,
            "unit": "median tps",
            "extra": "avg tps: 7.114086065040758, max tps: 11.010146149232142, count: 57600"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.268253529168869,
            "unit": "median tps",
            "extra": "avg tps: 4.7704017844729005, max tps: 5.868632347972625, count: 57600"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "b8ebd1ad157e946d6fff8f775aec3189bc469325",
          "message": "fix: possible naming collisions with builder functions (#3275)\n\n## What\n\nFix an issue where functions tagged with our `#[builder_fn]` macro could\nend up with the same name.\n\n## Why\n\nIt's come up in CI once or twice and I've seen it locally as well\n\n## How\n\n## Tests",
          "timestamp": "2025-10-07T15:35:22-04:00",
          "tree_id": "bd88762bb48211807276741ce46d155ea36600b3",
          "url": "https://github.com/paradedb/paradedb/commit/b8ebd1ad157e946d6fff8f775aec3189bc469325"
        },
        "date": 1759867373729,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.203869197847144,
            "unit": "median tps",
            "extra": "avg tps: 7.015916937034822, max tps: 10.94386167340485, count: 57906"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.427723030781504,
            "unit": "median tps",
            "extra": "avg tps: 4.901482306450147, max tps: 6.005452071881193, count: 57906"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "2f61e5c2d8fb03377c37dcd10c558e9967781b97",
          "message": "feat: A Free Space Manager fronted by an AVL tree (#3252)\n\n## What\n\nThis implements a new `v2` FSM that's fronted by an AVL tree which\nallows for minimal locking during extension and draining. It also\nprovides efficient continuation during drain as xid blocklists are\nexhausted or found to be unavailable to the current transaction. And it\nimplements a (simple) transparent conversion of the current `v1` FSM to\nthe new format.\n\nAdditionally, this fixes a problem with background merging where more\nthan one background merger process could be spawned at once -- I've seen\nup to 8 concurrently. It does this by introducing some a new page on\ndisk to track the process and coordinate locking.\n\n## Why\n\nOur current FSM is very heavyweight in terms of lock contention. This\nshould get us to something that isn't.\n\n## How\n\n## Tests\n\nA number of new tests for the array-backed AVL tree and the FSM itself.\nAll existing tests also pass and, at least, the `wide-table.toml`\nstressgres shows a slight performance improvement for the update jobs.\n\n---------\n\nSigned-off-by: Eric Ridge <eebbrr@gmail.com>\nCo-authored-by: Stu Hood <stuhood@paradedb.com>",
          "timestamp": "2025-10-07T15:36:47-04:00",
          "tree_id": "31410dd4c4d2be73d287e97485f4d0faaf1b2932",
          "url": "https://github.com/paradedb/paradedb/commit/2f61e5c2d8fb03377c37dcd10c558e9967781b97"
        },
        "date": 1759867455234,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.330392432014783,
            "unit": "median tps",
            "extra": "avg tps: 7.110430403077494, max tps: 11.072605854132549, count: 57792"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.331208929229581,
            "unit": "median tps",
            "extra": "avg tps: 4.825737764123826, max tps: 5.907494547064158, count: 57792"
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
          "id": "52dd41fee48d3a635a315610631235ff09ac53a5",
          "message": "chore: Upgrade to `0.18.10` (#3251)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-10-01T10:56:42-04:00",
          "tree_id": "fca37808df8505e2c564f422c635f688f82a0e1d",
          "url": "https://github.com/paradedb/paradedb/commit/52dd41fee48d3a635a315610631235ff09ac53a5"
        },
        "date": 1759332393722,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.897638,
            "unit": "median cpu",
            "extra": "avg cpu: 19.681886825617088, max cpu: 42.64561, count: 57328"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 225.4765625,
            "unit": "median mem",
            "extra": "avg mem: 224.98986404167684, max mem: 227.34765625, count: 57328"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.27837,
            "unit": "median cpu",
            "extra": "avg cpu: 22.389412650148248, max cpu: 33.20158, count: 57328"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 159.98046875,
            "unit": "median mem",
            "extra": "avg mem: 160.05665636501797, max mem: 162.1328125, count: 57328"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 24091,
            "unit": "median block_count",
            "extra": "avg block_count: 22971.739010605637, max block_count: 25917.0, count: 57328"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 71,
            "unit": "median segment_count",
            "extra": "avg segment_count: 72.67858638012838, max segment_count: 104.0, count: 57328"
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
        "date": 1759333349126,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.879055,
            "unit": "median cpu",
            "extra": "avg cpu: 19.59229883259347, max cpu: 42.857143, count: 57941"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 226.28515625,
            "unit": "median mem",
            "extra": "avg mem: 225.97973868353154, max mem: 228.0390625, count: 57941"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.30097,
            "unit": "median cpu",
            "extra": "avg cpu: 22.354403355612305, max cpu: 33.136093, count: 57941"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 159.3046875,
            "unit": "median mem",
            "extra": "avg mem: 159.28357449549972, max mem: 162.5859375, count: 57941"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 24135,
            "unit": "median block_count",
            "extra": "avg block_count: 23045.325072056057, max block_count: 25810.0, count: 57941"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 71,
            "unit": "median segment_count",
            "extra": "avg segment_count: 72.48673650782693, max segment_count: 105.0, count: 57941"
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
          "id": "bb494d330cfac7ee1db3b904ab5266bd671abadb",
          "message": "fix: `paradedb.snippet` projects correctly when called multiple times over the same field (#3257)\n\n# Ticket(s) Closed\n\n- Closes #3256 \n\n## What\n\nSee GH issue\n\n## Why\n\n## How\n\nWhen mapping a const node to our list of snippet generators, we were\nmatching only on the field name and function OID, which is not unique\nenough -- it doesn't account for the function arguments.\n\n## Tests\n\nAdded regression test",
          "timestamp": "2025-10-01T19:08:56-04:00",
          "tree_id": "a78e472aad70351c0c799699330da58256d3cbc4",
          "url": "https://github.com/paradedb/paradedb/commit/bb494d330cfac7ee1db3b904ab5266bd671abadb"
        },
        "date": 1759361845382,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.897638,
            "unit": "median cpu",
            "extra": "avg cpu: 19.507490959435085, max cpu: 42.814667, count: 57561"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 226.12109375,
            "unit": "median mem",
            "extra": "avg mem: 225.5802559594387, max mem: 227.7109375, count: 57561"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.30097,
            "unit": "median cpu",
            "extra": "avg cpu: 22.51641006441581, max cpu: 33.300297, count: 57561"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 160.53125,
            "unit": "median mem",
            "extra": "avg mem: 160.6853151575068, max mem: 165.171875, count: 57561"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 24110,
            "unit": "median block_count",
            "extra": "avg block_count: 22879.516304442244, max block_count: 25744.0, count: 57561"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 70,
            "unit": "median segment_count",
            "extra": "avg segment_count: 72.17838467017599, max segment_count: 105.0, count: 57561"
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
          "id": "762c487685bb5635c789b31781155839d2c65cf0",
          "message": "feat: Configure a limit/offset for snippets (#3254)",
          "timestamp": "2025-10-01T20:00:17-04:00",
          "tree_id": "5dcb534f0e2f513864b19abddc44396ed24760ff",
          "url": "https://github.com/paradedb/paradedb/commit/762c487685bb5635c789b31781155839d2c65cf0"
        },
        "date": 1759365012082,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.658894,
            "unit": "median cpu",
            "extra": "avg cpu: 19.13005222622509, max cpu: 42.72997, count: 57820"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 226.00390625,
            "unit": "median mem",
            "extra": "avg mem: 225.59660165708232, max mem: 228.25390625, count: 57820"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.323614,
            "unit": "median cpu",
            "extra": "avg cpu: 22.479349153457594, max cpu: 33.267326, count: 57820"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 159.12890625,
            "unit": "median mem",
            "extra": "avg mem: 158.88956648867173, max mem: 162.34765625, count: 57820"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 24403,
            "unit": "median block_count",
            "extra": "avg block_count: 23176.512279488066, max block_count: 26099.0, count: 57820"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 71,
            "unit": "median segment_count",
            "extra": "avg segment_count: 73.08400207540643, max segment_count: 107.0, count: 57820"
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
          "id": "850e3a9f88033d64151d6ecfa0d37c1b1f210b27",
          "message": "fix: `paradedb.snippet` projects correctly when called multiple times over the same field (#3258)\n\n# Ticket(s) Closed\n\n- Closes #3256 \n\n## What\n\nSee GH issue\n\n## Why\n\n## How\n\nWhen mapping a const node to our list of snippet generators, we were\nmatching only on the field name and function OID, which is not unique\nenough -- it doesn't account for the function arguments.\n\n## Tests\n\nAdded regression test\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-10-01T21:33:38-04:00",
          "tree_id": "1f119fb29d59884bd2105114833ca2d34c2acfd9",
          "url": "https://github.com/paradedb/paradedb/commit/850e3a9f88033d64151d6ecfa0d37c1b1f210b27"
        },
        "date": 1759370545887,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.82353,
            "unit": "median cpu",
            "extra": "avg cpu: 19.346501366981403, max cpu: 42.857143, count: 57556"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 232.40625,
            "unit": "median mem",
            "extra": "avg mem: 231.05119565618702, max mem: 233.69140625, count: 57556"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.323614,
            "unit": "median cpu",
            "extra": "avg cpu: 22.48824531686938, max cpu: 33.300297, count: 57556"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 159.4453125,
            "unit": "median mem",
            "extra": "avg mem: 159.2889048410461, max mem: 162.6875, count: 57556"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 23934,
            "unit": "median block_count",
            "extra": "avg block_count: 22932.463114184447, max block_count: 25894.0, count: 57556"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 71,
            "unit": "median segment_count",
            "extra": "avg segment_count: 72.59130238376538, max segment_count: 106.0, count: 57556"
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
          "id": "2ce078f8496ff0152f7373fe94348a9cfcacd5af",
          "message": "feat: Configure a limit/offset for snippets (#3259)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n`paradedb.snippet` and `paradedb.snippet_positions` now take a limit and\noffset. For instance, if 5 snippets are found in a doc and offset is 1,\nthen the first snippet will be skipped.\n\n## Why\n\n## How\n\n## Tests\n\nSee regression test\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-10-01T22:10:01-04:00",
          "tree_id": "904048e17b5e988830cd9302f007cb5d18411d22",
          "url": "https://github.com/paradedb/paradedb/commit/2ce078f8496ff0152f7373fe94348a9cfcacd5af"
        },
        "date": 1759372712322,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.916256,
            "unit": "median cpu",
            "extra": "avg cpu: 19.762827207775338, max cpu: 42.942345, count: 57297"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 225.125,
            "unit": "median mem",
            "extra": "avg mem: 224.75157219455207, max mem: 227.13671875, count: 57297"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.30097,
            "unit": "median cpu",
            "extra": "avg cpu: 22.3868793725517, max cpu: 33.267326, count: 57297"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 164.953125,
            "unit": "median mem",
            "extra": "avg mem: 163.98616147005953, max mem: 166.6953125, count: 57297"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 24130,
            "unit": "median block_count",
            "extra": "avg block_count: 23007.82084576854, max block_count: 25771.0, count: 57297"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 71,
            "unit": "median segment_count",
            "extra": "avg segment_count: 72.4714382951987, max segment_count: 106.0, count: 57297"
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
          "id": "6335204e0cc58bdeb1ea12f236ab2258a44cd192",
          "message": "chore: Upgrade to `0.18.11` (#3261)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-10-02T10:00:31-04:00",
          "tree_id": "05d18415099467cbf114e4a87c3b536bfeafb509",
          "url": "https://github.com/paradedb/paradedb/commit/6335204e0cc58bdeb1ea12f236ab2258a44cd192"
        },
        "date": 1759415444325,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.75,
            "unit": "median cpu",
            "extra": "avg cpu: 19.472555471183618, max cpu: 42.814667, count: 57913"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 224.49609375,
            "unit": "median mem",
            "extra": "avg mem: 224.01895610117762, max mem: 226.109375, count: 57913"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.323614,
            "unit": "median cpu",
            "extra": "avg cpu: 22.371819222874127, max cpu: 33.267326, count: 57913"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 161.1328125,
            "unit": "median mem",
            "extra": "avg mem: 161.2393975526868, max mem: 162.6328125, count: 57913"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 24139,
            "unit": "median block_count",
            "extra": "avg block_count: 23088.687617633346, max block_count: 26030.0, count: 57913"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 71,
            "unit": "median segment_count",
            "extra": "avg segment_count: 72.70690518536425, max segment_count: 107.0, count: 57913"
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
          "id": "7f394c9cc9b86839fb39f22123893552f6e5a291",
          "message": "chore: Upgrade to `0.18.11` (#3262)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-10-02T10:04:40-04:00",
          "tree_id": "fd395f6aeafa74dd786657eebbf6f9cba6e6bf28",
          "url": "https://github.com/paradedb/paradedb/commit/7f394c9cc9b86839fb39f22123893552f6e5a291"
        },
        "date": 1759415691141,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.731707,
            "unit": "median cpu",
            "extra": "avg cpu: 19.407969564893257, max cpu: 42.899704, count: 57907"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 225.578125,
            "unit": "median mem",
            "extra": "avg mem: 225.09080231826462, max mem: 227.11328125, count: 57907"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.30097,
            "unit": "median cpu",
            "extra": "avg cpu: 22.63932346315827, max cpu: 33.23442, count: 57907"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 161.11328125,
            "unit": "median mem",
            "extra": "avg mem: 161.27410944218747, max mem: 163.0859375, count: 57907"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 24232,
            "unit": "median block_count",
            "extra": "avg block_count: 23060.316144852954, max block_count: 25868.0, count: 57907"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 71,
            "unit": "median segment_count",
            "extra": "avg segment_count: 72.5740065967845, max segment_count: 107.0, count: 57907"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "stuhood@paradedb.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "6b15d1a8e21e76267b06b7e54dbbb5194fda55b9",
          "message": "chore: Improve determinism of property tests. (#3220)\n\n## What\n\nSet a seed to control what is generated by `random()` in the property\ntests, and render it in the reproduction script after a failure.\n\n## Why\n\nTo make it easier to reproduce property test failures by running over\nreproducible data.\n\n`proptest` failures are only directly reproducible via their reported\nseed if they are not dependent on the randomly generated data in the\ntable that we test against. We can't re-generate the table and index for\nevery `proptest` query that we run, because it would take way too long\nto run a reasonable number of iterations. And we don't want to run on\nstatic data, because then we might never catch data-dependent bugs like\n#3266.\n\nSetting a seed allows us to run on random data, but still reproduce\nfailures later. And for cases where failures aren't data-dependent, the\n`proptest` repro seed (e.g. `cc 08176a8c0ae10938a...`) can still be used\ndirectly.",
          "timestamp": "2025-10-03T13:48:13-07:00",
          "tree_id": "ac9934e860da0bdb5b4b083df652ebc3d85309d4",
          "url": "https://github.com/paradedb/paradedb/commit/6b15d1a8e21e76267b06b7e54dbbb5194fda55b9"
        },
        "date": 1759526212221,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.897638,
            "unit": "median cpu",
            "extra": "avg cpu: 19.542610002123396, max cpu: 42.857143, count: 57791"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 226.18359375,
            "unit": "median mem",
            "extra": "avg mem: 225.67838782747313, max mem: 227.72265625, count: 57791"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.30097,
            "unit": "median cpu",
            "extra": "avg cpu: 22.464474546541393, max cpu: 33.333336, count: 57791"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 158.89453125,
            "unit": "median mem",
            "extra": "avg mem: 158.76004136943902, max mem: 162.3515625, count: 57791"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 23922,
            "unit": "median block_count",
            "extra": "avg block_count: 22827.58988423803, max block_count: 25571.0, count: 57791"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 70,
            "unit": "median segment_count",
            "extra": "avg segment_count: 72.22133204132132, max segment_count: 104.0, count: 57791"
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
          "id": "34e2d538b7327fc30b32f6ee33b55cbc9ccb2749",
          "message": "chore: Remove deprecated tokenizers: `en_stem`, `stem` and `lowercase` (#3279)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nFor better maintainability:\n\n1. Removes three extremely deprecated tokenizers: `en_stem`, `stem`, and\n`lowercase`\n2. Wraps the filter builders in a macro, guaranteeing that all the\nfilters are applied to all tokenizers\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-10-07T11:16:19-04:00",
          "tree_id": "7ef65d52251d2f4fb83439b6887924fa19564416",
          "url": "https://github.com/paradedb/paradedb/commit/34e2d538b7327fc30b32f6ee33b55cbc9ccb2749"
        },
        "date": 1759851832876,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.86051,
            "unit": "median cpu",
            "extra": "avg cpu: 19.697592672663987, max cpu: 42.814667, count: 57270"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 224.9453125,
            "unit": "median mem",
            "extra": "avg mem: 224.448218212524, max mem: 226.49609375, count: 57270"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.27837,
            "unit": "median cpu",
            "extra": "avg cpu: 22.369360729755225, max cpu: 33.20158, count: 57270"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 164.09375,
            "unit": "median mem",
            "extra": "avg mem: 163.7209971680199, max mem: 165.8515625, count: 57270"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 24057,
            "unit": "median block_count",
            "extra": "avg block_count: 22976.39666492055, max block_count: 25681.0, count: 57270"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 71,
            "unit": "median segment_count",
            "extra": "avg segment_count: 72.36265060240964, max segment_count: 106.0, count: 57270"
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
          "id": "7f394c9cc9b86839fb39f22123893552f6e5a291",
          "message": "chore: Upgrade to `0.18.11` (#3262)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-10-02T10:04:40-04:00",
          "tree_id": "fd395f6aeafa74dd786657eebbf6f9cba6e6bf28",
          "url": "https://github.com/paradedb/paradedb/commit/7f394c9cc9b86839fb39f22123893552f6e5a291"
        },
        "date": 1759857867189,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.882769,
            "unit": "median cpu",
            "extra": "avg cpu: 19.532447473684854, max cpu: 42.814667, count: 57600"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 225.48828125,
            "unit": "median mem",
            "extra": "avg mem: 225.19161526150174, max mem: 227.40234375, count: 57600"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.346306,
            "unit": "median cpu",
            "extra": "avg cpu: 22.546178316575343, max cpu: 33.366436, count: 57600"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 158.0390625,
            "unit": "median mem",
            "extra": "avg mem: 157.74408135308158, max mem: 161.27734375, count: 57600"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 24158,
            "unit": "median block_count",
            "extra": "avg block_count: 22980.35265625, max block_count: 25599.0, count: 57600"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 71,
            "unit": "median segment_count",
            "extra": "avg segment_count: 72.48104166666667, max segment_count: 105.0, count: 57600"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "b8ebd1ad157e946d6fff8f775aec3189bc469325",
          "message": "fix: possible naming collisions with builder functions (#3275)\n\n## What\n\nFix an issue where functions tagged with our `#[builder_fn]` macro could\nend up with the same name.\n\n## Why\n\nIt's come up in CI once or twice and I've seen it locally as well\n\n## How\n\n## Tests",
          "timestamp": "2025-10-07T15:35:22-04:00",
          "tree_id": "bd88762bb48211807276741ce46d155ea36600b3",
          "url": "https://github.com/paradedb/paradedb/commit/b8ebd1ad157e946d6fff8f775aec3189bc469325"
        },
        "date": 1759867376417,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.897638,
            "unit": "median cpu",
            "extra": "avg cpu: 19.681863738491487, max cpu: 42.899704, count: 57906"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 226.046875,
            "unit": "median mem",
            "extra": "avg mem: 225.52845000140314, max mem: 227.59765625, count: 57906"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.30097,
            "unit": "median cpu",
            "extra": "avg cpu: 22.403078382261636, max cpu: 33.267326, count: 57906"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 161.3046875,
            "unit": "median mem",
            "extra": "avg mem: 161.4103487764653, max mem: 166.08203125, count: 57906"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 24044,
            "unit": "median block_count",
            "extra": "avg block_count: 22857.8116084689, max block_count: 25779.0, count: 57906"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 70,
            "unit": "median segment_count",
            "extra": "avg segment_count: 72.37795737920078, max segment_count: 107.0, count: 57906"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "2f61e5c2d8fb03377c37dcd10c558e9967781b97",
          "message": "feat: A Free Space Manager fronted by an AVL tree (#3252)\n\n## What\n\nThis implements a new `v2` FSM that's fronted by an AVL tree which\nallows for minimal locking during extension and draining. It also\nprovides efficient continuation during drain as xid blocklists are\nexhausted or found to be unavailable to the current transaction. And it\nimplements a (simple) transparent conversion of the current `v1` FSM to\nthe new format.\n\nAdditionally, this fixes a problem with background merging where more\nthan one background merger process could be spawned at once -- I've seen\nup to 8 concurrently. It does this by introducing some a new page on\ndisk to track the process and coordinate locking.\n\n## Why\n\nOur current FSM is very heavyweight in terms of lock contention. This\nshould get us to something that isn't.\n\n## How\n\n## Tests\n\nA number of new tests for the array-backed AVL tree and the FSM itself.\nAll existing tests also pass and, at least, the `wide-table.toml`\nstressgres shows a slight performance improvement for the update jobs.\n\n---------\n\nSigned-off-by: Eric Ridge <eebbrr@gmail.com>\nCo-authored-by: Stu Hood <stuhood@paradedb.com>",
          "timestamp": "2025-10-07T15:36:47-04:00",
          "tree_id": "31410dd4c4d2be73d287e97485f4d0faaf1b2932",
          "url": "https://github.com/paradedb/paradedb/commit/2f61e5c2d8fb03377c37dcd10c558e9967781b97"
        },
        "date": 1759867457845,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.82353,
            "unit": "median cpu",
            "extra": "avg cpu: 19.383037680653114, max cpu: 42.72997, count: 57792"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 225.60546875,
            "unit": "median mem",
            "extra": "avg mem: 225.11588352410368, max mem: 228.375, count: 57792"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.30097,
            "unit": "median cpu",
            "extra": "avg cpu: 22.46368728387064, max cpu: 33.300297, count: 57792"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 161.20703125,
            "unit": "median mem",
            "extra": "avg mem: 161.10615933271472, max mem: 162.33203125, count: 57792"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 24466,
            "unit": "median block_count",
            "extra": "avg block_count: 23308.52495155039, max block_count: 26004.0, count: 57792"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 71,
            "unit": "median segment_count",
            "extra": "avg segment_count: 72.5797169158361, max segment_count: 104.0, count: 57792"
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
          "id": "52dd41fee48d3a635a315610631235ff09ac53a5",
          "message": "chore: Upgrade to `0.18.10` (#3251)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-10-01T10:56:42-04:00",
          "tree_id": "fca37808df8505e2c564f422c635f688f82a0e1d",
          "url": "https://github.com/paradedb/paradedb/commit/52dd41fee48d3a635a315610631235ff09ac53a5"
        },
        "date": 1759333154454,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 451.14462495258783,
            "unit": "median tps",
            "extra": "avg tps: 439.65601202920504, max tps: 510.7593663162637, count: 57102"
          },
          {
            "name": "Single Insert - Primary - tps",
            "value": 499.83324739873854,
            "unit": "median tps",
            "extra": "avg tps: 501.7867147855015, max tps: 536.402776056529, count: 57102"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 964.1998410778413,
            "unit": "median tps",
            "extra": "avg tps: 943.7411174832705, max tps: 1239.3440279077256, count: 57102"
          },
          {
            "name": "Top N - Primary - tps",
            "value": 5.873007151551244,
            "unit": "median tps",
            "extra": "avg tps: 5.897980535706201, max tps: 7.146974642573277, count: 57102"
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
        "date": 1759334122491,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 448.23820431858815,
            "unit": "median tps",
            "extra": "avg tps: 437.72777071002815, max tps: 507.49553433499983, count: 57107"
          },
          {
            "name": "Single Insert - Primary - tps",
            "value": 486.66161517003707,
            "unit": "median tps",
            "extra": "avg tps: 484.5736038984534, max tps: 515.8676152809124, count: 57107"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 946.6064185693256,
            "unit": "median tps",
            "extra": "avg tps: 932.9140879939814, max tps: 1231.7222991776098, count: 57107"
          },
          {
            "name": "Top N - Primary - tps",
            "value": 5.860973907455079,
            "unit": "median tps",
            "extra": "avg tps: 5.888969856061533, max tps: 7.319570429636191, count: 57107"
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
          "id": "bb494d330cfac7ee1db3b904ab5266bd671abadb",
          "message": "fix: `paradedb.snippet` projects correctly when called multiple times over the same field (#3257)\n\n# Ticket(s) Closed\n\n- Closes #3256 \n\n## What\n\nSee GH issue\n\n## Why\n\n## How\n\nWhen mapping a const node to our list of snippet generators, we were\nmatching only on the field name and function OID, which is not unique\nenough -- it doesn't account for the function arguments.\n\n## Tests\n\nAdded regression test",
          "timestamp": "2025-10-01T19:08:56-04:00",
          "tree_id": "a78e472aad70351c0c799699330da58256d3cbc4",
          "url": "https://github.com/paradedb/paradedb/commit/bb494d330cfac7ee1db3b904ab5266bd671abadb"
        },
        "date": 1759362608175,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 465.99491444247565,
            "unit": "median tps",
            "extra": "avg tps: 454.1125464835233, max tps: 515.1148613523089, count: 56700"
          },
          {
            "name": "Single Insert - Primary - tps",
            "value": 390.83823567172107,
            "unit": "median tps",
            "extra": "avg tps: 350.99340444511523, max tps: 427.03528317053696, count: 56700"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 942.8521707916904,
            "unit": "median tps",
            "extra": "avg tps: 924.1881743129244, max tps: 1194.536310584637, count: 56700"
          },
          {
            "name": "Top N - Primary - tps",
            "value": 6.0620743791141045,
            "unit": "median tps",
            "extra": "avg tps: 6.09312666914546, max tps: 7.37246516277978, count: 56700"
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
          "id": "762c487685bb5635c789b31781155839d2c65cf0",
          "message": "feat: Configure a limit/offset for snippets (#3254)",
          "timestamp": "2025-10-01T20:00:17-04:00",
          "tree_id": "5dcb534f0e2f513864b19abddc44396ed24760ff",
          "url": "https://github.com/paradedb/paradedb/commit/762c487685bb5635c789b31781155839d2c65cf0"
        },
        "date": 1759365777541,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 452.97417484595957,
            "unit": "median tps",
            "extra": "avg tps: 440.23338484063817, max tps: 515.2424168385602, count: 57096"
          },
          {
            "name": "Single Insert - Primary - tps",
            "value": 502.4138778773149,
            "unit": "median tps",
            "extra": "avg tps: 501.99138754637687, max tps: 534.5039593502582, count: 57096"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 978.9649094298092,
            "unit": "median tps",
            "extra": "avg tps: 960.6570650852783, max tps: 1266.531957841478, count: 57096"
          },
          {
            "name": "Top N - Primary - tps",
            "value": 5.869478032869537,
            "unit": "median tps",
            "extra": "avg tps: 5.882111106658495, max tps: 7.043532492701113, count: 57096"
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
          "id": "850e3a9f88033d64151d6ecfa0d37c1b1f210b27",
          "message": "fix: `paradedb.snippet` projects correctly when called multiple times over the same field (#3258)\n\n# Ticket(s) Closed\n\n- Closes #3256 \n\n## What\n\nSee GH issue\n\n## Why\n\n## How\n\nWhen mapping a const node to our list of snippet generators, we were\nmatching only on the field name and function OID, which is not unique\nenough -- it doesn't account for the function arguments.\n\n## Tests\n\nAdded regression test\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-10-01T21:33:38-04:00",
          "tree_id": "1f119fb29d59884bd2105114833ca2d34c2acfd9",
          "url": "https://github.com/paradedb/paradedb/commit/850e3a9f88033d64151d6ecfa0d37c1b1f210b27"
        },
        "date": 1759371311300,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 450.6330607871356,
            "unit": "median tps",
            "extra": "avg tps: 437.92953303045334, max tps: 508.17085362895347, count: 57037"
          },
          {
            "name": "Single Insert - Primary - tps",
            "value": 502.9967793187095,
            "unit": "median tps",
            "extra": "avg tps: 502.60137571582976, max tps: 534.088715415594, count: 57037"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 971.5029706708457,
            "unit": "median tps",
            "extra": "avg tps: 949.3906354182697, max tps: 1247.0562415699956, count: 57037"
          },
          {
            "name": "Top N - Primary - tps",
            "value": 6.379619516782557,
            "unit": "median tps",
            "extra": "avg tps: 6.373184134658928, max tps: 7.0498076879435905, count: 57037"
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
          "id": "2ce078f8496ff0152f7373fe94348a9cfcacd5af",
          "message": "feat: Configure a limit/offset for snippets (#3259)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n`paradedb.snippet` and `paradedb.snippet_positions` now take a limit and\noffset. For instance, if 5 snippets are found in a doc and offset is 1,\nthen the first snippet will be skipped.\n\n## Why\n\n## How\n\n## Tests\n\nSee regression test\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-10-01T22:10:01-04:00",
          "tree_id": "904048e17b5e988830cd9302f007cb5d18411d22",
          "url": "https://github.com/paradedb/paradedb/commit/2ce078f8496ff0152f7373fe94348a9cfcacd5af"
        },
        "date": 1759373490057,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 404.5899296907769,
            "unit": "median tps",
            "extra": "avg tps: 378.97480145627213, max tps: 484.0743624183959, count: 57120"
          },
          {
            "name": "Single Insert - Primary - tps",
            "value": 501.9313719464902,
            "unit": "median tps",
            "extra": "avg tps: 501.16647209711397, max tps: 533.0533049843856, count: 57120"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 970.977826023425,
            "unit": "median tps",
            "extra": "avg tps: 952.9959571406434, max tps: 1254.4417286873459, count: 57120"
          },
          {
            "name": "Top N - Primary - tps",
            "value": 5.98386080385794,
            "unit": "median tps",
            "extra": "avg tps: 5.999623625883399, max tps: 7.215781557914875, count: 57120"
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
          "id": "6335204e0cc58bdeb1ea12f236ab2258a44cd192",
          "message": "chore: Upgrade to `0.18.11` (#3261)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-10-02T10:00:31-04:00",
          "tree_id": "05d18415099467cbf114e4a87c3b536bfeafb509",
          "url": "https://github.com/paradedb/paradedb/commit/6335204e0cc58bdeb1ea12f236ab2258a44cd192"
        },
        "date": 1759416211821,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 457.11955674244126,
            "unit": "median tps",
            "extra": "avg tps: 444.87379414574224, max tps: 514.9519432844517, count: 56381"
          },
          {
            "name": "Single Insert - Primary - tps",
            "value": 507.5527631329132,
            "unit": "median tps",
            "extra": "avg tps: 506.42137576398835, max tps: 537.3872633085547, count: 56381"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 975.3009574311006,
            "unit": "median tps",
            "extra": "avg tps: 958.7675099363324, max tps: 1246.0643239213605, count: 56381"
          },
          {
            "name": "Top N - Primary - tps",
            "value": 5.982618439347828,
            "unit": "median tps",
            "extra": "avg tps: 6.008147439081345, max tps: 7.138219186915004, count: 56381"
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
          "id": "7f394c9cc9b86839fb39f22123893552f6e5a291",
          "message": "chore: Upgrade to `0.18.11` (#3262)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-10-02T10:04:40-04:00",
          "tree_id": "fd395f6aeafa74dd786657eebbf6f9cba6e6bf28",
          "url": "https://github.com/paradedb/paradedb/commit/7f394c9cc9b86839fb39f22123893552f6e5a291"
        },
        "date": 1759416459803,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 405.9889496463904,
            "unit": "median tps",
            "extra": "avg tps: 375.89032222426835, max tps: 475.0407969505801, count: 56448"
          },
          {
            "name": "Single Insert - Primary - tps",
            "value": 519.056006057699,
            "unit": "median tps",
            "extra": "avg tps: 518.073033844842, max tps: 549.9751735983199, count: 56448"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 943.2992203356341,
            "unit": "median tps",
            "extra": "avg tps: 923.4051824078888, max tps: 1199.1639943808082, count: 56448"
          },
          {
            "name": "Top N - Primary - tps",
            "value": 6.000190866473261,
            "unit": "median tps",
            "extra": "avg tps: 6.0554910372203405, max tps: 7.556326539306647, count: 56448"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "stuhood@paradedb.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "6b15d1a8e21e76267b06b7e54dbbb5194fda55b9",
          "message": "chore: Improve determinism of property tests. (#3220)\n\n## What\n\nSet a seed to control what is generated by `random()` in the property\ntests, and render it in the reproduction script after a failure.\n\n## Why\n\nTo make it easier to reproduce property test failures by running over\nreproducible data.\n\n`proptest` failures are only directly reproducible via their reported\nseed if they are not dependent on the randomly generated data in the\ntable that we test against. We can't re-generate the table and index for\nevery `proptest` query that we run, because it would take way too long\nto run a reasonable number of iterations. And we don't want to run on\nstatic data, because then we might never catch data-dependent bugs like\n#3266.\n\nSetting a seed allows us to run on random data, but still reproduce\nfailures later. And for cases where failures aren't data-dependent, the\n`proptest` repro seed (e.g. `cc 08176a8c0ae10938a...`) can still be used\ndirectly.",
          "timestamp": "2025-10-03T13:48:13-07:00",
          "tree_id": "ac9934e860da0bdb5b4b083df652ebc3d85309d4",
          "url": "https://github.com/paradedb/paradedb/commit/6b15d1a8e21e76267b06b7e54dbbb5194fda55b9"
        },
        "date": 1759526987266,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 456.17472604680006,
            "unit": "median tps",
            "extra": "avg tps: 441.72593886267634, max tps: 504.0899940061115, count: 56461"
          },
          {
            "name": "Single Insert - Primary - tps",
            "value": 513.3033503464135,
            "unit": "median tps",
            "extra": "avg tps: 511.9603922337503, max tps: 543.9104528557318, count: 56461"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 977.4268365395536,
            "unit": "median tps",
            "extra": "avg tps: 954.2194117641016, max tps: 1234.243899141542, count: 56461"
          },
          {
            "name": "Top N - Primary - tps",
            "value": 5.977217617978894,
            "unit": "median tps",
            "extra": "avg tps: 6.00778102063647, max tps: 7.565659598186911, count: 56461"
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
          "id": "34e2d538b7327fc30b32f6ee33b55cbc9ccb2749",
          "message": "chore: Remove deprecated tokenizers: `en_stem`, `stem` and `lowercase` (#3279)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nFor better maintainability:\n\n1. Removes three extremely deprecated tokenizers: `en_stem`, `stem`, and\n`lowercase`\n2. Wraps the filter builders in a macro, guaranteeing that all the\nfilters are applied to all tokenizers\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-10-07T11:16:19-04:00",
          "tree_id": "7ef65d52251d2f4fb83439b6887924fa19564416",
          "url": "https://github.com/paradedb/paradedb/commit/34e2d538b7327fc30b32f6ee33b55cbc9ccb2749"
        },
        "date": 1759852525149,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 457.1626847800514,
            "unit": "median tps",
            "extra": "avg tps: 444.22932937345996, max tps: 509.1388876800426, count: 56630"
          },
          {
            "name": "Single Insert - Primary - tps",
            "value": 508.48697402696064,
            "unit": "median tps",
            "extra": "avg tps: 507.03183207574784, max tps: 536.3836806004434, count: 56630"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 949.7006506378768,
            "unit": "median tps",
            "extra": "avg tps: 928.7037930333101, max tps: 1193.2626766735834, count: 56630"
          },
          {
            "name": "Top N - Primary - tps",
            "value": 5.944770572476202,
            "unit": "median tps",
            "extra": "avg tps: 5.962564920162041, max tps: 7.3148957046925105, count: 56630"
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
          "id": "7f394c9cc9b86839fb39f22123893552f6e5a291",
          "message": "chore: Upgrade to `0.18.11` (#3262)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-10-02T10:04:40-04:00",
          "tree_id": "fd395f6aeafa74dd786657eebbf6f9cba6e6bf28",
          "url": "https://github.com/paradedb/paradedb/commit/7f394c9cc9b86839fb39f22123893552f6e5a291"
        },
        "date": 1759858565371,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 458.45264483309603,
            "unit": "median tps",
            "extra": "avg tps: 443.9968345128843, max tps: 507.74877695546724, count: 56525"
          },
          {
            "name": "Single Insert - Primary - tps",
            "value": 498.88403209398143,
            "unit": "median tps",
            "extra": "avg tps: 495.23917484029454, max tps: 520.9263135234536, count: 56525"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 957.9535070236408,
            "unit": "median tps",
            "extra": "avg tps: 938.7031718485273, max tps: 1206.1208523502705, count: 56525"
          },
          {
            "name": "Top N - Primary - tps",
            "value": 6.0875398022061304,
            "unit": "median tps",
            "extra": "avg tps: 6.091249464804023, max tps: 7.364780657057596, count: 56525"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "b8ebd1ad157e946d6fff8f775aec3189bc469325",
          "message": "fix: possible naming collisions with builder functions (#3275)\n\n## What\n\nFix an issue where functions tagged with our `#[builder_fn]` macro could\nend up with the same name.\n\n## Why\n\nIt's come up in CI once or twice and I've seen it locally as well\n\n## How\n\n## Tests",
          "timestamp": "2025-10-07T15:35:22-04:00",
          "tree_id": "bd88762bb48211807276741ce46d155ea36600b3",
          "url": "https://github.com/paradedb/paradedb/commit/b8ebd1ad157e946d6fff8f775aec3189bc469325"
        },
        "date": 1759868079014,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 452.9128472748808,
            "unit": "median tps",
            "extra": "avg tps: 437.4356677940625, max tps: 503.2888481666922, count: 56481"
          },
          {
            "name": "Single Insert - Primary - tps",
            "value": 518.0573955314501,
            "unit": "median tps",
            "extra": "avg tps: 517.6987526583404, max tps: 549.7518771917648, count: 56481"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 972.0537521371298,
            "unit": "median tps",
            "extra": "avg tps: 955.0845846418384, max tps: 1228.737581871722, count: 56481"
          },
          {
            "name": "Top N - Primary - tps",
            "value": 6.159990478722072,
            "unit": "median tps",
            "extra": "avg tps: 6.160401121771515, max tps: 7.4841527929309, count: 56481"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "2f61e5c2d8fb03377c37dcd10c558e9967781b97",
          "message": "feat: A Free Space Manager fronted by an AVL tree (#3252)\n\n## What\n\nThis implements a new `v2` FSM that's fronted by an AVL tree which\nallows for minimal locking during extension and draining. It also\nprovides efficient continuation during drain as xid blocklists are\nexhausted or found to be unavailable to the current transaction. And it\nimplements a (simple) transparent conversion of the current `v1` FSM to\nthe new format.\n\nAdditionally, this fixes a problem with background merging where more\nthan one background merger process could be spawned at once -- I've seen\nup to 8 concurrently. It does this by introducing some a new page on\ndisk to track the process and coordinate locking.\n\n## Why\n\nOur current FSM is very heavyweight in terms of lock contention. This\nshould get us to something that isn't.\n\n## How\n\n## Tests\n\nA number of new tests for the array-backed AVL tree and the FSM itself.\nAll existing tests also pass and, at least, the `wide-table.toml`\nstressgres shows a slight performance improvement for the update jobs.\n\n---------\n\nSigned-off-by: Eric Ridge <eebbrr@gmail.com>\nCo-authored-by: Stu Hood <stuhood@paradedb.com>",
          "timestamp": "2025-10-07T15:36:47-04:00",
          "tree_id": "31410dd4c4d2be73d287e97485f4d0faaf1b2932",
          "url": "https://github.com/paradedb/paradedb/commit/2f61e5c2d8fb03377c37dcd10c558e9967781b97"
        },
        "date": 1759868162278,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 453.24507569604236,
            "unit": "median tps",
            "extra": "avg tps: 438.73981396874933, max tps: 508.5884758666332, count: 55944"
          },
          {
            "name": "Single Insert - Primary - tps",
            "value": 506.47688632965884,
            "unit": "median tps",
            "extra": "avg tps: 504.49809619214574, max tps: 534.4519386367546, count: 55944"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 946.1226363153595,
            "unit": "median tps",
            "extra": "avg tps: 929.2839017156452, max tps: 1186.721889912047, count: 55944"
          },
          {
            "name": "Top N - Primary - tps",
            "value": 5.9344534115167775,
            "unit": "median tps",
            "extra": "avg tps: 5.966892496993967, max tps: 7.364427517389623, count: 55944"
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
          "id": "52dd41fee48d3a635a315610631235ff09ac53a5",
          "message": "chore: Upgrade to `0.18.10` (#3251)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-10-01T10:56:42-04:00",
          "tree_id": "fca37808df8505e2c564f422c635f688f82a0e1d",
          "url": "https://github.com/paradedb/paradedb/commit/52dd41fee48d3a635a315610631235ff09ac53a5"
        },
        "date": 1759333157129,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 5.833025434486139, max cpu: 28.514853, count: 57102"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 166.3046875,
            "unit": "median mem",
            "extra": "avg mem: 165.1969626446972, max mem: 166.3046875, count: 57102"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 59204,
            "unit": "median block_count",
            "extra": "avg block_count: 61277.135196665615, max block_count: 76046.0, count: 57102"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 86,
            "unit": "median segment_count",
            "extra": "avg segment_count: 92.54327344050996, max segment_count: 193.0, count: 57102"
          },
          {
            "name": "Single Insert - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.967767957564054, max cpu: 23.346306, count: 57102"
          },
          {
            "name": "Single Insert - Primary - mem",
            "value": 166.6875,
            "unit": "median mem",
            "extra": "avg mem: 158.66886353039297, max mem: 166.6875, count: 57102"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.958579014995527, max cpu: 28.514853, count: 57102"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 167.69140625,
            "unit": "median mem",
            "extra": "avg mem: 165.0767070330286, max mem: 167.69140625, count: 57102"
          },
          {
            "name": "Top N - Primary - cpu",
            "value": 23.552504,
            "unit": "median cpu",
            "extra": "avg cpu: 23.880393062058584, max cpu: 33.532936, count: 57102"
          },
          {
            "name": "Top N - Primary - mem",
            "value": 155.30859375,
            "unit": "median mem",
            "extra": "avg mem: 171.18441129743704, max mem: 215.171875, count: 57102"
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
        "date": 1759334125061,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 4.669261,
            "unit": "median cpu",
            "extra": "avg cpu: 5.813386603675458, max cpu: 28.514853, count: 57107"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 167.59375,
            "unit": "median mem",
            "extra": "avg mem: 166.65431790870647, max mem: 167.96875, count: 57107"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 59399,
            "unit": "median block_count",
            "extra": "avg block_count: 61352.55408268689, max block_count: 76406.0, count: 57107"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 87,
            "unit": "median segment_count",
            "extra": "avg segment_count: 93.7032587948938, max segment_count: 194.0, count: 57107"
          },
          {
            "name": "Single Insert - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.93289943193927, max cpu: 27.934044, count: 57107"
          },
          {
            "name": "Single Insert - Primary - mem",
            "value": 167.51953125,
            "unit": "median mem",
            "extra": "avg mem: 159.0954152539312, max mem: 167.51953125, count: 57107"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.929653901282263, max cpu: 27.87996, count: 57107"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 164.49609375,
            "unit": "median mem",
            "extra": "avg mem: 161.928203512157, max mem: 164.49609375, count: 57107"
          },
          {
            "name": "Top N - Primary - cpu",
            "value": 23.460411,
            "unit": "median cpu",
            "extra": "avg cpu: 23.885440635974618, max cpu: 33.870968, count: 57107"
          },
          {
            "name": "Top N - Primary - mem",
            "value": 155.30859375,
            "unit": "median mem",
            "extra": "avg mem: 170.98234324655908, max mem: 214.80859375, count: 57107"
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
          "id": "bb494d330cfac7ee1db3b904ab5266bd671abadb",
          "message": "fix: `paradedb.snippet` projects correctly when called multiple times over the same field (#3257)\n\n# Ticket(s) Closed\n\n- Closes #3256 \n\n## What\n\nSee GH issue\n\n## Why\n\n## How\n\nWhen mapping a const node to our list of snippet generators, we were\nmatching only on the field name and function OID, which is not unique\nenough -- it doesn't account for the function arguments.\n\n## Tests\n\nAdded regression test",
          "timestamp": "2025-10-01T19:08:56-04:00",
          "tree_id": "a78e472aad70351c0c799699330da58256d3cbc4",
          "url": "https://github.com/paradedb/paradedb/commit/bb494d330cfac7ee1db3b904ab5266bd671abadb"
        },
        "date": 1759362611355,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 5.685763765306842, max cpu: 33.136093, count: 56700"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 167.44921875,
            "unit": "median mem",
            "extra": "avg mem: 166.55093240189595, max mem: 167.828125, count: 56700"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 60498,
            "unit": "median block_count",
            "extra": "avg block_count: 63764.84451499118, max block_count: 74423.0, count: 56700"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 84,
            "unit": "median segment_count",
            "extra": "avg segment_count: 90.69245149911816, max segment_count: 191.0, count: 56700"
          },
          {
            "name": "Single Insert - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 5.261027735125707, max cpu: 32.55814, count: 56700"
          },
          {
            "name": "Single Insert - Primary - mem",
            "value": 174.5546875,
            "unit": "median mem",
            "extra": "avg mem: 181.77213941247794, max mem: 237.41015625, count: 56700"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.9870123782918006, max cpu: 28.402367, count: 56700"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 161.5,
            "unit": "median mem",
            "extra": "avg mem: 158.94260726686508, max mem: 163.421875, count: 56700"
          },
          {
            "name": "Top N - Primary - cpu",
            "value": 23.369036,
            "unit": "median cpu",
            "extra": "avg cpu: 23.87447530563146, max cpu: 33.768845, count: 56700"
          },
          {
            "name": "Top N - Primary - mem",
            "value": 156.1875,
            "unit": "median mem",
            "extra": "avg mem: 169.59656684027777, max mem: 216.43359375, count: 56700"
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
          "id": "762c487685bb5635c789b31781155839d2c65cf0",
          "message": "feat: Configure a limit/offset for snippets (#3254)",
          "timestamp": "2025-10-01T20:00:17-04:00",
          "tree_id": "5dcb534f0e2f513864b19abddc44396ed24760ff",
          "url": "https://github.com/paradedb/paradedb/commit/762c487685bb5635c789b31781155839d2c65cf0"
        },
        "date": 1759365780196,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 5.790994297125587, max cpu: 28.430405, count: 57096"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 166.56640625,
            "unit": "median mem",
            "extra": "avg mem: 165.27705531035448, max mem: 166.56640625, count: 57096"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 59102,
            "unit": "median block_count",
            "extra": "avg block_count: 61275.06634440241, max block_count: 76078.0, count: 57096"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 86,
            "unit": "median segment_count",
            "extra": "avg segment_count: 92.43116855821773, max segment_count: 194.0, count: 57096"
          },
          {
            "name": "Single Insert - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.896174081577064, max cpu: 28.346458, count: 57096"
          },
          {
            "name": "Single Insert - Primary - mem",
            "value": 168.28125,
            "unit": "median mem",
            "extra": "avg mem: 159.78889686613334, max mem: 168.65625, count: 57096"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.943227550161554, max cpu: 27.961164, count: 57096"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 164.4453125,
            "unit": "median mem",
            "extra": "avg mem: 162.01539799745166, max mem: 164.4453125, count: 57096"
          },
          {
            "name": "Top N - Primary - cpu",
            "value": 23.483368,
            "unit": "median cpu",
            "extra": "avg cpu: 23.858555315819224, max cpu: 33.136093, count: 57096"
          },
          {
            "name": "Top N - Primary - mem",
            "value": 155.49609375,
            "unit": "median mem",
            "extra": "avg mem: 171.4515115578368, max mem: 215.73046875, count: 57096"
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
          "id": "850e3a9f88033d64151d6ecfa0d37c1b1f210b27",
          "message": "fix: `paradedb.snippet` projects correctly when called multiple times over the same field (#3258)\n\n# Ticket(s) Closed\n\n- Closes #3256 \n\n## What\n\nSee GH issue\n\n## Why\n\n## How\n\nWhen mapping a const node to our list of snippet generators, we were\nmatching only on the field name and function OID, which is not unique\nenough -- it doesn't account for the function arguments.\n\n## Tests\n\nAdded regression test\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-10-01T21:33:38-04:00",
          "tree_id": "1f119fb29d59884bd2105114833ca2d34c2acfd9",
          "url": "https://github.com/paradedb/paradedb/commit/850e3a9f88033d64151d6ecfa0d37c1b1f210b27"
        },
        "date": 1759371313863,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 4.678363,
            "unit": "median cpu",
            "extra": "avg cpu: 5.84339857377726, max cpu: 33.07087, count: 57037"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 167.08984375,
            "unit": "median mem",
            "extra": "avg mem: 166.02756167323403, max mem: 167.08984375, count: 57037"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 58668,
            "unit": "median block_count",
            "extra": "avg block_count: 60939.896873959005, max block_count: 75818.0, count: 57037"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 84,
            "unit": "median segment_count",
            "extra": "avg segment_count: 91.06802601819871, max segment_count: 192.0, count: 57037"
          },
          {
            "name": "Single Insert - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 4.891460301968861, max cpu: 23.575638, count: 57037"
          },
          {
            "name": "Single Insert - Primary - mem",
            "value": 166.31640625,
            "unit": "median mem",
            "extra": "avg mem: 159.75464008012344, max mem: 168.578125, count: 57037"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 4.994446527791004, max cpu: 28.042841, count: 57037"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 166.875,
            "unit": "median mem",
            "extra": "avg mem: 164.5544844382813, max mem: 166.875, count: 57037"
          },
          {
            "name": "Top N - Primary - cpu",
            "value": 23.575638,
            "unit": "median cpu",
            "extra": "avg cpu: 23.84966015220797, max cpu: 33.768845, count: 57037"
          },
          {
            "name": "Top N - Primary - mem",
            "value": 155.0703125,
            "unit": "median mem",
            "extra": "avg mem: 170.84002338942267, max mem: 214.8515625, count: 57037"
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
          "id": "2ce078f8496ff0152f7373fe94348a9cfcacd5af",
          "message": "feat: Configure a limit/offset for snippets (#3259)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n`paradedb.snippet` and `paradedb.snippet_positions` now take a limit and\noffset. For instance, if 5 snippets are found in a doc and offset is 1,\nthen the first snippet will be skipped.\n\n## Why\n\n## How\n\n## Tests\n\nSee regression test\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-10-01T22:10:01-04:00",
          "tree_id": "904048e17b5e988830cd9302f007cb5d18411d22",
          "url": "https://github.com/paradedb/paradedb/commit/2ce078f8496ff0152f7373fe94348a9cfcacd5af"
        },
        "date": 1759373493175,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 4.669261,
            "unit": "median cpu",
            "extra": "avg cpu: 6.128667938356879, max cpu: 32.526623, count: 57120"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 175.0703125,
            "unit": "median mem",
            "extra": "avg mem: 175.34706299511993, max mem: 235.15625, count: 57120"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 64098,
            "unit": "median block_count",
            "extra": "avg block_count: 66218.59070378152, max block_count: 75078.0, count: 57120"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 85,
            "unit": "median segment_count",
            "extra": "avg segment_count: 92.31698179271709, max segment_count: 191.0, count: 57120"
          },
          {
            "name": "Single Insert - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.88874412048881, max cpu: 27.87996, count: 57120"
          },
          {
            "name": "Single Insert - Primary - mem",
            "value": 171.8515625,
            "unit": "median mem",
            "extra": "avg mem: 164.4590315618435, max mem: 172.2265625, count: 57120"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.989683258940214, max cpu: 27.853, count: 57120"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 168.27734375,
            "unit": "median mem",
            "extra": "avg mem: 164.75765637309613, max mem: 168.66015625, count: 57120"
          },
          {
            "name": "Top N - Primary - cpu",
            "value": 23.391813,
            "unit": "median cpu",
            "extra": "avg cpu: 23.845119490817744, max cpu: 33.870968, count: 57120"
          },
          {
            "name": "Top N - Primary - mem",
            "value": 154.8671875,
            "unit": "median mem",
            "extra": "avg mem: 169.87174014136906, max mem: 215.0859375, count: 57120"
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
          "id": "6335204e0cc58bdeb1ea12f236ab2258a44cd192",
          "message": "chore: Upgrade to `0.18.11` (#3261)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-10-02T10:00:31-04:00",
          "tree_id": "05d18415099467cbf114e4a87c3b536bfeafb509",
          "url": "https://github.com/paradedb/paradedb/commit/6335204e0cc58bdeb1ea12f236ab2258a44cd192"
        },
        "date": 1759416214461,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 5.780563196967673, max cpu: 27.988338, count: 56381"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 166.45703125,
            "unit": "median mem",
            "extra": "avg mem: 165.26688075603926, max mem: 166.45703125, count: 56381"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 59046,
            "unit": "median block_count",
            "extra": "avg block_count: 60989.94228552172, max block_count: 76028.0, count: 56381"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 85,
            "unit": "median segment_count",
            "extra": "avg segment_count: 92.52980614036643, max segment_count: 193.0, count: 56381"
          },
          {
            "name": "Single Insert - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.899223336615784, max cpu: 27.906979, count: 56381"
          },
          {
            "name": "Single Insert - Primary - mem",
            "value": 166.93359375,
            "unit": "median mem",
            "extra": "avg mem: 160.18236224304286, max mem: 169.5859375, count: 56381"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.978765018575738, max cpu: 23.210833, count: 56381"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 166.25,
            "unit": "median mem",
            "extra": "avg mem: 162.66700322083238, max mem: 167.75, count: 56381"
          },
          {
            "name": "Top N - Primary - cpu",
            "value": 23.323614,
            "unit": "median cpu",
            "extra": "avg cpu: 23.806739468704205, max cpu: 33.633633, count: 56381"
          },
          {
            "name": "Top N - Primary - mem",
            "value": 154.93359375,
            "unit": "median mem",
            "extra": "avg mem: 169.0490554081827, max mem: 214.78515625, count: 56381"
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
          "id": "7f394c9cc9b86839fb39f22123893552f6e5a291",
          "message": "chore: Upgrade to `0.18.11` (#3262)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-10-02T10:04:40-04:00",
          "tree_id": "fd395f6aeafa74dd786657eebbf6f9cba6e6bf28",
          "url": "https://github.com/paradedb/paradedb/commit/7f394c9cc9b86839fb39f22123893552f6e5a291"
        },
        "date": 1759416463019,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 5.950615007699571, max cpu: 32.589718, count: 56448"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 238.56640625,
            "unit": "median mem",
            "extra": "avg mem: 236.87049045692495, max mem: 238.56640625, count: 56448"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 57927,
            "unit": "median block_count",
            "extra": "avg block_count: 62244.07043650794, max block_count: 74449.0, count: 56448"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 84,
            "unit": "median segment_count",
            "extra": "avg segment_count: 90.48641227324264, max segment_count: 191.0, count: 56448"
          },
          {
            "name": "Single Insert - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.8919689572244405, max cpu: 27.87996, count: 56448"
          },
          {
            "name": "Single Insert - Primary - mem",
            "value": 167.140625,
            "unit": "median mem",
            "extra": "avg mem: 159.48393052289276, max mem: 167.92578125, count: 56448"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.8701998376465525, max cpu: 27.826086, count: 56448"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 165.3203125,
            "unit": "median mem",
            "extra": "avg mem: 162.64243065797282, max mem: 166.07421875, count: 56448"
          },
          {
            "name": "Top N - Primary - cpu",
            "value": 23.346306,
            "unit": "median cpu",
            "extra": "avg cpu: 23.887511843036997, max cpu: 33.633633, count: 56448"
          },
          {
            "name": "Top N - Primary - mem",
            "value": 155.265625,
            "unit": "median mem",
            "extra": "avg mem: 168.0730752750319, max mem: 214.76171875, count: 56448"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "stuhood@paradedb.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "6b15d1a8e21e76267b06b7e54dbbb5194fda55b9",
          "message": "chore: Improve determinism of property tests. (#3220)\n\n## What\n\nSet a seed to control what is generated by `random()` in the property\ntests, and render it in the reproduction script after a failure.\n\n## Why\n\nTo make it easier to reproduce property test failures by running over\nreproducible data.\n\n`proptest` failures are only directly reproducible via their reported\nseed if they are not dependent on the randomly generated data in the\ntable that we test against. We can't re-generate the table and index for\nevery `proptest` query that we run, because it would take way too long\nto run a reasonable number of iterations. And we don't want to run on\nstatic data, because then we might never catch data-dependent bugs like\n#3266.\n\nSetting a seed allows us to run on random data, but still reproduce\nfailures later. And for cases where failures aren't data-dependent, the\n`proptest` repro seed (e.g. `cc 08176a8c0ae10938a...`) can still be used\ndirectly.",
          "timestamp": "2025-10-03T13:48:13-07:00",
          "tree_id": "ac9934e860da0bdb5b4b083df652ebc3d85309d4",
          "url": "https://github.com/paradedb/paradedb/commit/6b15d1a8e21e76267b06b7e54dbbb5194fda55b9"
        },
        "date": 1759526989841,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 4.669261,
            "unit": "median cpu",
            "extra": "avg cpu: 5.79059596526156, max cpu: 28.180038, count: 56461"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 166.92578125,
            "unit": "median mem",
            "extra": "avg mem: 165.6646127277014, max mem: 166.92578125, count: 56461"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 58111,
            "unit": "median block_count",
            "extra": "avg block_count: 60665.90315438976, max block_count: 74840.0, count: 56461"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 85,
            "unit": "median segment_count",
            "extra": "avg segment_count: 91.33911903791999, max segment_count: 192.0, count: 56461"
          },
          {
            "name": "Single Insert - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.891851284049834, max cpu: 24.120604, count: 56461"
          },
          {
            "name": "Single Insert - Primary - mem",
            "value": 167.85546875,
            "unit": "median mem",
            "extra": "avg mem: 159.9467916044925, max mem: 167.85546875, count: 56461"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.97135991851184, max cpu: 27.665707, count: 56461"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 166.04296875,
            "unit": "median mem",
            "extra": "avg mem: 163.3556297433184, max mem: 166.04296875, count: 56461"
          },
          {
            "name": "Top N - Primary - cpu",
            "value": 23.414635,
            "unit": "median cpu",
            "extra": "avg cpu: 23.852369082473125, max cpu: 33.83686, count: 56461"
          },
          {
            "name": "Top N - Primary - mem",
            "value": 155.6328125,
            "unit": "median mem",
            "extra": "avg mem: 169.16894467253945, max mem: 215.828125, count: 56461"
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
          "id": "34e2d538b7327fc30b32f6ee33b55cbc9ccb2749",
          "message": "chore: Remove deprecated tokenizers: `en_stem`, `stem` and `lowercase` (#3279)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nFor better maintainability:\n\n1. Removes three extremely deprecated tokenizers: `en_stem`, `stem`, and\n`lowercase`\n2. Wraps the filter builders in a macro, guaranteeing that all the\nfilters are applied to all tokenizers\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-10-07T11:16:19-04:00",
          "tree_id": "7ef65d52251d2f4fb83439b6887924fa19564416",
          "url": "https://github.com/paradedb/paradedb/commit/34e2d538b7327fc30b32f6ee33b55cbc9ccb2749"
        },
        "date": 1759852527911,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 5.7643147753721165, max cpu: 27.961164, count: 56630"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 165.640625,
            "unit": "median mem",
            "extra": "avg mem: 164.5050857816087, max mem: 165.640625, count: 56630"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 58557,
            "unit": "median block_count",
            "extra": "avg block_count: 60744.7790570369, max block_count: 75485.0, count: 56630"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 86,
            "unit": "median segment_count",
            "extra": "avg segment_count: 92.51506268762141, max segment_count: 194.0, count: 56630"
          },
          {
            "name": "Single Insert - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.891922270471396, max cpu: 23.27837, count: 56630"
          },
          {
            "name": "Single Insert - Primary - mem",
            "value": 165.23828125,
            "unit": "median mem",
            "extra": "avg mem: 156.79035474240686, max mem: 166.36328125, count: 56630"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.980064874757746, max cpu: 32.526623, count: 56630"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 166.7734375,
            "unit": "median mem",
            "extra": "avg mem: 163.94517095620697, max mem: 166.7734375, count: 56630"
          },
          {
            "name": "Top N - Primary - cpu",
            "value": 23.369036,
            "unit": "median cpu",
            "extra": "avg cpu: 24.0330640500717, max cpu: 33.20158, count: 56630"
          },
          {
            "name": "Top N - Primary - mem",
            "value": 154.71875,
            "unit": "median mem",
            "extra": "avg mem: 168.22612903937843, max mem: 214.546875, count: 56630"
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
          "id": "7f394c9cc9b86839fb39f22123893552f6e5a291",
          "message": "chore: Upgrade to `0.18.11` (#3262)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-10-02T10:04:40-04:00",
          "tree_id": "fd395f6aeafa74dd786657eebbf6f9cba6e6bf28",
          "url": "https://github.com/paradedb/paradedb/commit/7f394c9cc9b86839fb39f22123893552f6e5a291"
        },
        "date": 1759858568111,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 5.701844592289937, max cpu: 28.263002, count: 56525"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 165.88671875,
            "unit": "median mem",
            "extra": "avg mem: 165.0164318194383, max mem: 166.64453125, count: 56525"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 58258,
            "unit": "median block_count",
            "extra": "avg block_count: 60750.227226890755, max block_count: 75069.0, count: 56525"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 85,
            "unit": "median segment_count",
            "extra": "avg segment_count: 91.47713401149933, max segment_count: 191.0, count: 56525"
          },
          {
            "name": "Single Insert - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.966230885609883, max cpu: 28.402367, count: 56525"
          },
          {
            "name": "Single Insert - Primary - mem",
            "value": 169.2109375,
            "unit": "median mem",
            "extra": "avg mem: 162.14075263987175, max mem: 169.5859375, count: 56525"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.955365272973902, max cpu: 32.876713, count: 56525"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 164.03515625,
            "unit": "median mem",
            "extra": "avg mem: 161.42377985404687, max mem: 166.28515625, count: 56525"
          },
          {
            "name": "Top N - Primary - cpu",
            "value": 23.460411,
            "unit": "median cpu",
            "extra": "avg cpu: 23.917853167736375, max cpu: 33.532936, count: 56525"
          },
          {
            "name": "Top N - Primary - mem",
            "value": 154.6484375,
            "unit": "median mem",
            "extra": "avg mem: 168.1407914777753, max mem: 215.22265625, count: 56525"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "b8ebd1ad157e946d6fff8f775aec3189bc469325",
          "message": "fix: possible naming collisions with builder functions (#3275)\n\n## What\n\nFix an issue where functions tagged with our `#[builder_fn]` macro could\nend up with the same name.\n\n## Why\n\nIt's come up in CI once or twice and I've seen it locally as well\n\n## How\n\n## Tests",
          "timestamp": "2025-10-07T15:35:22-04:00",
          "tree_id": "bd88762bb48211807276741ce46d155ea36600b3",
          "url": "https://github.com/paradedb/paradedb/commit/b8ebd1ad157e946d6fff8f775aec3189bc469325"
        },
        "date": 1759868081641,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 4.678363,
            "unit": "median cpu",
            "extra": "avg cpu: 5.6889826600721545, max cpu: 28.514853, count: 56481"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 168,
            "unit": "median mem",
            "extra": "avg mem: 166.63894218852357, max mem: 168.0, count: 56481"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 58106,
            "unit": "median block_count",
            "extra": "avg block_count: 60559.49862785715, max block_count: 75375.0, count: 56481"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 85,
            "unit": "median segment_count",
            "extra": "avg segment_count: 91.63725854712204, max segment_count: 193.0, count: 56481"
          },
          {
            "name": "Single Insert - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 4.883383372486818, max cpu: 14.799589, count: 56481"
          },
          {
            "name": "Single Insert - Primary - mem",
            "value": 162.5,
            "unit": "median mem",
            "extra": "avg mem: 154.77251904954764, max mem: 162.87890625, count: 56481"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 4.94514130479182, max cpu: 15.2019005, count: 56481"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 162.0625,
            "unit": "median mem",
            "extra": "avg mem: 159.50877099433882, max mem: 162.0625, count: 56481"
          },
          {
            "name": "Top N - Primary - cpu",
            "value": 23.414635,
            "unit": "median cpu",
            "extra": "avg cpu: 23.726033835275384, max cpu: 33.20158, count: 56481"
          },
          {
            "name": "Top N - Primary - mem",
            "value": 154.16796875,
            "unit": "median mem",
            "extra": "avg mem: 167.65112113199572, max mem: 214.05078125, count: 56481"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "2f61e5c2d8fb03377c37dcd10c558e9967781b97",
          "message": "feat: A Free Space Manager fronted by an AVL tree (#3252)\n\n## What\n\nThis implements a new `v2` FSM that's fronted by an AVL tree which\nallows for minimal locking during extension and draining. It also\nprovides efficient continuation during drain as xid blocklists are\nexhausted or found to be unavailable to the current transaction. And it\nimplements a (simple) transparent conversion of the current `v1` FSM to\nthe new format.\n\nAdditionally, this fixes a problem with background merging where more\nthan one background merger process could be spawned at once -- I've seen\nup to 8 concurrently. It does this by introducing some a new page on\ndisk to track the process and coordinate locking.\n\n## Why\n\nOur current FSM is very heavyweight in terms of lock contention. This\nshould get us to something that isn't.\n\n## How\n\n## Tests\n\nA number of new tests for the array-backed AVL tree and the FSM itself.\nAll existing tests also pass and, at least, the `wide-table.toml`\nstressgres shows a slight performance improvement for the update jobs.\n\n---------\n\nSigned-off-by: Eric Ridge <eebbrr@gmail.com>\nCo-authored-by: Stu Hood <stuhood@paradedb.com>",
          "timestamp": "2025-10-07T15:36:47-04:00",
          "tree_id": "31410dd4c4d2be73d287e97485f4d0faaf1b2932",
          "url": "https://github.com/paradedb/paradedb/commit/2f61e5c2d8fb03377c37dcd10c558e9967781b97"
        },
        "date": 1759868165038,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Background Merger - Primary - background_merging",
            "value": 0,
            "unit": "median background_merging",
            "extra": "avg background_merging: 0.3087909337909338, max background_merging: 1.0, count: 55944"
          },
          {
            "name": "Background Merger - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.794780149452842, max cpu: 9.619239, count: 55944"
          },
          {
            "name": "Background Merger - Primary - mem",
            "value": 17.5,
            "unit": "median mem",
            "extra": "avg mem: 17.497240683764122, max mem: 20.08984375, count: 55944"
          },
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 5.640985972112031, max cpu: 28.290766, count: 55944"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 167.03515625,
            "unit": "median mem",
            "extra": "avg mem: 165.8242957661903, max mem: 167.03515625, count: 55944"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 59666,
            "unit": "median block_count",
            "extra": "avg block_count: 61294.17101029601, max block_count: 76751.0, count: 55944"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 86,
            "unit": "median segment_count",
            "extra": "avg segment_count: 93.36817174317174, max segment_count: 194.0, count: 55944"
          },
          {
            "name": "Single Insert - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.897072404854357, max cpu: 27.906979, count: 55944"
          },
          {
            "name": "Single Insert - Primary - mem",
            "value": 165.734375,
            "unit": "median mem",
            "extra": "avg mem: 157.93393267709675, max mem: 165.734375, count: 55944"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.9370576021651615, max cpu: 27.745665, count: 55944"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 164.09375,
            "unit": "median mem",
            "extra": "avg mem: 160.24550184950576, max mem: 164.09375, count: 55944"
          },
          {
            "name": "Top N - Primary - cpu",
            "value": 23.255816,
            "unit": "median cpu",
            "extra": "avg cpu: 23.97464882515391, max cpu: 33.20158, count: 55944"
          },
          {
            "name": "Top N - Primary - mem",
            "value": 155.37890625,
            "unit": "median mem",
            "extra": "avg mem: 169.2743889955804, max mem: 215.2578125, count: 55944"
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
          "id": "52dd41fee48d3a635a315610631235ff09ac53a5",
          "message": "chore: Upgrade to `0.18.10` (#3251)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-10-01T10:56:42-04:00",
          "tree_id": "fca37808df8505e2c564f422c635f688f82a0e1d",
          "url": "https://github.com/paradedb/paradedb/commit/52dd41fee48d3a635a315610631235ff09ac53a5"
        },
        "date": 1759333872932,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 38.51051542762401,
            "unit": "median tps",
            "extra": "avg tps: 38.59414646030631, max tps: 39.348823423923356, count: 55643"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 238.51469524985734,
            "unit": "median tps",
            "extra": "avg tps: 264.70617808451505, max tps: 2800.1350478807535, count: 55643"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1017.9169829364137,
            "unit": "median tps",
            "extra": "avg tps: 1011.7833915720009, max tps: 1024.05877331169, count: 55643"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 116.69004128202747,
            "unit": "median tps",
            "extra": "avg tps: 153.31438633339243, max tps: 785.892492234513, count: 111286"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 18.036441543533286,
            "unit": "median tps",
            "extra": "avg tps: 18.06980697687859, max tps: 19.234798551165728, count: 55643"
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
        "date": 1759334841965,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 36.335743145241665,
            "unit": "median tps",
            "extra": "avg tps: 36.6089036449859, max tps: 39.510514308815, count: 55507"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 242.6456306276674,
            "unit": "median tps",
            "extra": "avg tps: 272.3255644203203, max tps: 2807.0448353194834, count: 55507"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1016.7473666811792,
            "unit": "median tps",
            "extra": "avg tps: 1005.470974509613, max tps: 1030.4523994336917, count: 55507"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 118.01556385772976,
            "unit": "median tps",
            "extra": "avg tps: 153.29558006726768, max tps: 793.5812786883989, count: 111014"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 18.802898591530987,
            "unit": "median tps",
            "extra": "avg tps: 18.919164549321746, max tps: 20.35295276568134, count: 55507"
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
          "id": "bb494d330cfac7ee1db3b904ab5266bd671abadb",
          "message": "fix: `paradedb.snippet` projects correctly when called multiple times over the same field (#3257)\n\n# Ticket(s) Closed\n\n- Closes #3256 \n\n## What\n\nSee GH issue\n\n## Why\n\n## How\n\nWhen mapping a const node to our list of snippet generators, we were\nmatching only on the field name and function OID, which is not unique\nenough -- it doesn't account for the function arguments.\n\n## Tests\n\nAdded regression test",
          "timestamp": "2025-10-01T19:08:56-04:00",
          "tree_id": "a78e472aad70351c0c799699330da58256d3cbc4",
          "url": "https://github.com/paradedb/paradedb/commit/bb494d330cfac7ee1db3b904ab5266bd671abadb"
        },
        "date": 1759363329191,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 38.81906941009015,
            "unit": "median tps",
            "extra": "avg tps: 38.81619200187246, max tps: 39.21839231446637, count: 55611"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 245.91615169414666,
            "unit": "median tps",
            "extra": "avg tps: 279.43823677923, max tps: 2827.2205188356374, count: 55611"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1054.2932613047262,
            "unit": "median tps",
            "extra": "avg tps: 1049.4788553906, max tps: 1064.0412491350373, count: 55611"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 119.12511859254494,
            "unit": "median tps",
            "extra": "avg tps: 157.83216348820667, max tps: 817.8467025264599, count: 111222"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 19.695908758939026,
            "unit": "median tps",
            "extra": "avg tps: 19.760813032863865, max tps: 20.705045975441084, count: 55611"
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
          "id": "762c487685bb5635c789b31781155839d2c65cf0",
          "message": "feat: Configure a limit/offset for snippets (#3254)",
          "timestamp": "2025-10-01T20:00:17-04:00",
          "tree_id": "5dcb534f0e2f513864b19abddc44396ed24760ff",
          "url": "https://github.com/paradedb/paradedb/commit/762c487685bb5635c789b31781155839d2c65cf0"
        },
        "date": 1759366501206,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 38.11308129107326,
            "unit": "median tps",
            "extra": "avg tps: 38.16157335914206, max tps: 39.08795291061339, count: 55525"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 243.48232063260363,
            "unit": "median tps",
            "extra": "avg tps: 271.97139214409447, max tps: 2937.624840555652, count: 55525"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1030.8752814353998,
            "unit": "median tps",
            "extra": "avg tps: 1022.5981998566448, max tps: 1043.0670535216332, count: 55525"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 122.46557395450975,
            "unit": "median tps",
            "extra": "avg tps: 157.85555048905596, max tps: 814.5742132363985, count: 111050"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 18.130932352280173,
            "unit": "median tps",
            "extra": "avg tps: 18.22146254061954, max tps: 20.024534861093205, count: 55525"
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
          "id": "850e3a9f88033d64151d6ecfa0d37c1b1f210b27",
          "message": "fix: `paradedb.snippet` projects correctly when called multiple times over the same field (#3258)\n\n# Ticket(s) Closed\n\n- Closes #3256 \n\n## What\n\nSee GH issue\n\n## Why\n\n## How\n\nWhen mapping a const node to our list of snippet generators, we were\nmatching only on the field name and function OID, which is not unique\nenough -- it doesn't account for the function arguments.\n\n## Tests\n\nAdded regression test\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-10-01T21:33:38-04:00",
          "tree_id": "1f119fb29d59884bd2105114833ca2d34c2acfd9",
          "url": "https://github.com/paradedb/paradedb/commit/850e3a9f88033d64151d6ecfa0d37c1b1f210b27"
        },
        "date": 1759372034036,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 38.65493636033226,
            "unit": "median tps",
            "extra": "avg tps: 38.667199959500756, max tps: 39.02194254160375, count: 55523"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 132.43548488875234,
            "unit": "median tps",
            "extra": "avg tps: 175.31885504716476, max tps: 2817.7273624816057, count: 55523"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1030.1492074400728,
            "unit": "median tps",
            "extra": "avg tps: 1024.5893457191155, max tps: 1037.3972051011287, count: 55523"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 113.3563300501157,
            "unit": "median tps",
            "extra": "avg tps: 119.91063910272648, max tps: 809.445201520662, count: 111046"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 18.47717354547076,
            "unit": "median tps",
            "extra": "avg tps: 18.600112128490405, max tps: 19.86316222634254, count: 55523"
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
          "id": "2ce078f8496ff0152f7373fe94348a9cfcacd5af",
          "message": "feat: Configure a limit/offset for snippets (#3259)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n`paradedb.snippet` and `paradedb.snippet_positions` now take a limit and\noffset. For instance, if 5 snippets are found in a doc and offset is 1,\nthen the first snippet will be skipped.\n\n## Why\n\n## How\n\n## Tests\n\nSee regression test\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-10-01T22:10:01-04:00",
          "tree_id": "904048e17b5e988830cd9302f007cb5d18411d22",
          "url": "https://github.com/paradedb/paradedb/commit/2ce078f8496ff0152f7373fe94348a9cfcacd5af"
        },
        "date": 1759374213924,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 37.13681629546143,
            "unit": "median tps",
            "extra": "avg tps: 37.18892774074989, max tps: 40.03025403778961, count: 55459"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 247.41464934604485,
            "unit": "median tps",
            "extra": "avg tps: 274.964521636083, max tps: 2941.6753502996908, count: 55459"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1069.4928462316363,
            "unit": "median tps",
            "extra": "avg tps: 1063.0871817262541, max tps: 1077.5917474698838, count: 55459"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 119.93583814808854,
            "unit": "median tps",
            "extra": "avg tps: 156.3718751181389, max tps: 820.4627713690927, count: 110918"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 18.24210111639352,
            "unit": "median tps",
            "extra": "avg tps: 18.2249158904113, max tps: 21.850165136446797, count: 55459"
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
          "id": "6335204e0cc58bdeb1ea12f236ab2258a44cd192",
          "message": "chore: Upgrade to `0.18.11` (#3261)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-10-02T10:00:31-04:00",
          "tree_id": "05d18415099467cbf114e4a87c3b536bfeafb509",
          "url": "https://github.com/paradedb/paradedb/commit/6335204e0cc58bdeb1ea12f236ab2258a44cd192"
        },
        "date": 1759416937639,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 37.50410886679256,
            "unit": "median tps",
            "extra": "avg tps: 37.65350590000786, max tps: 40.100113060063265, count: 55433"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 246.09971555652146,
            "unit": "median tps",
            "extra": "avg tps: 278.7393530004236, max tps: 2949.013570872487, count: 55433"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1058.6330722545526,
            "unit": "median tps",
            "extra": "avg tps: 1052.6453100279814, max tps: 1063.4897959207706, count: 55433"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 115.91914343367864,
            "unit": "median tps",
            "extra": "avg tps: 154.72648070324746, max tps: 814.5604316987312, count: 110866"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 18.119045330668122,
            "unit": "median tps",
            "extra": "avg tps: 18.111644455286363, max tps: 18.659940361942287, count: 55433"
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
          "id": "7f394c9cc9b86839fb39f22123893552f6e5a291",
          "message": "chore: Upgrade to `0.18.11` (#3262)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-10-02T10:04:40-04:00",
          "tree_id": "fd395f6aeafa74dd786657eebbf6f9cba6e6bf28",
          "url": "https://github.com/paradedb/paradedb/commit/7f394c9cc9b86839fb39f22123893552f6e5a291"
        },
        "date": 1759417201365,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 36.92607641270735,
            "unit": "median tps",
            "extra": "avg tps: 36.971573032743095, max tps: 39.00378580774595, count: 55493"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 244.9037319630471,
            "unit": "median tps",
            "extra": "avg tps: 272.1761055763902, max tps: 3111.118255160438, count: 55493"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1059.5822407012074,
            "unit": "median tps",
            "extra": "avg tps: 1055.5987868117058, max tps: 1066.904015092675, count: 55493"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 119.05211909091155,
            "unit": "median tps",
            "extra": "avg tps: 156.17806104711224, max tps: 810.3787584169211, count: 110986"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 18.04998559209127,
            "unit": "median tps",
            "extra": "avg tps: 18.079179238797618, max tps: 19.970337259255338, count: 55493"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "stuhood@paradedb.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "6b15d1a8e21e76267b06b7e54dbbb5194fda55b9",
          "message": "chore: Improve determinism of property tests. (#3220)\n\n## What\n\nSet a seed to control what is generated by `random()` in the property\ntests, and render it in the reproduction script after a failure.\n\n## Why\n\nTo make it easier to reproduce property test failures by running over\nreproducible data.\n\n`proptest` failures are only directly reproducible via their reported\nseed if they are not dependent on the randomly generated data in the\ntable that we test against. We can't re-generate the table and index for\nevery `proptest` query that we run, because it would take way too long\nto run a reasonable number of iterations. And we don't want to run on\nstatic data, because then we might never catch data-dependent bugs like\n#3266.\n\nSetting a seed allows us to run on random data, but still reproduce\nfailures later. And for cases where failures aren't data-dependent, the\n`proptest` repro seed (e.g. `cc 08176a8c0ae10938a...`) can still be used\ndirectly.",
          "timestamp": "2025-10-03T13:48:13-07:00",
          "tree_id": "ac9934e860da0bdb5b4b083df652ebc3d85309d4",
          "url": "https://github.com/paradedb/paradedb/commit/6b15d1a8e21e76267b06b7e54dbbb5194fda55b9"
        },
        "date": 1759527722402,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 40.00685369407659,
            "unit": "median tps",
            "extra": "avg tps: 39.88863434628092, max tps: 40.184394425279585, count: 55664"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 47.7723666144937,
            "unit": "median tps",
            "extra": "avg tps: 100.41616921999658, max tps: 2842.4174042429186, count: 55664"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1037.8776672522877,
            "unit": "median tps",
            "extra": "avg tps: 1035.4045371221239, max tps: 1060.0923824485733, count: 55664"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 106.40425084406567,
            "unit": "median tps",
            "extra": "avg tps: 88.67444015854302, max tps: 803.1374840030846, count: 111328"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 20.697156641900254,
            "unit": "median tps",
            "extra": "avg tps: 20.653777079421946, max tps: 21.091285911657966, count: 55664"
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
          "id": "34e2d538b7327fc30b32f6ee33b55cbc9ccb2749",
          "message": "chore: Remove deprecated tokenizers: `en_stem`, `stem` and `lowercase` (#3279)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nFor better maintainability:\n\n1. Removes three extremely deprecated tokenizers: `en_stem`, `stem`, and\n`lowercase`\n2. Wraps the filter builders in a macro, guaranteeing that all the\nfilters are applied to all tokenizers\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-10-07T11:16:19-04:00",
          "tree_id": "7ef65d52251d2f4fb83439b6887924fa19564416",
          "url": "https://github.com/paradedb/paradedb/commit/34e2d538b7327fc30b32f6ee33b55cbc9ccb2749"
        },
        "date": 1759853177867,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 35.554784077285035,
            "unit": "median tps",
            "extra": "avg tps: 35.79819406489724, max tps: 37.79330855837124, count: 55720"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 246.47336501086178,
            "unit": "median tps",
            "extra": "avg tps: 276.6804874844353, max tps: 2902.9721933549454, count: 55720"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1045.3751246550578,
            "unit": "median tps",
            "extra": "avg tps: 1039.6201921602544, max tps: 1062.7738781332241, count: 55720"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 122.19296360141851,
            "unit": "median tps",
            "extra": "avg tps: 158.17254136270822, max tps: 822.5556855140928, count: 111440"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 18.190229228529923,
            "unit": "median tps",
            "extra": "avg tps: 18.489388234820918, max tps: 21.021197586455404, count: 55720"
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
          "id": "7f394c9cc9b86839fb39f22123893552f6e5a291",
          "message": "chore: Upgrade to `0.18.11` (#3262)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-10-02T10:04:40-04:00",
          "tree_id": "fd395f6aeafa74dd786657eebbf6f9cba6e6bf28",
          "url": "https://github.com/paradedb/paradedb/commit/7f394c9cc9b86839fb39f22123893552f6e5a291"
        },
        "date": 1759859220422,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 38.57464263100098,
            "unit": "median tps",
            "extra": "avg tps: 38.482341396119224, max tps: 38.68611908071123, count: 55555"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 244.3909405447879,
            "unit": "median tps",
            "extra": "avg tps: 275.82026952196026, max tps: 2971.6592099186164, count: 55555"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1057.34462194958,
            "unit": "median tps",
            "extra": "avg tps: 1051.4513040241382, max tps: 1068.160979920235, count: 55555"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 118.89701895459777,
            "unit": "median tps",
            "extra": "avg tps: 156.01027821382, max tps: 795.7532718730788, count: 111110"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 19.168329665041952,
            "unit": "median tps",
            "extra": "avg tps: 19.18163036322835, max tps: 21.21000644057753, count: 55555"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "b8ebd1ad157e946d6fff8f775aec3189bc469325",
          "message": "fix: possible naming collisions with builder functions (#3275)\n\n## What\n\nFix an issue where functions tagged with our `#[builder_fn]` macro could\nend up with the same name.\n\n## Why\n\nIt's come up in CI once or twice and I've seen it locally as well\n\n## How\n\n## Tests",
          "timestamp": "2025-10-07T15:35:22-04:00",
          "tree_id": "bd88762bb48211807276741ce46d155ea36600b3",
          "url": "https://github.com/paradedb/paradedb/commit/b8ebd1ad157e946d6fff8f775aec3189bc469325"
        },
        "date": 1759868744609,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 38.37340400859662,
            "unit": "median tps",
            "extra": "avg tps: 38.474991339614206, max tps: 39.60525475933167, count: 55569"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 244.1465559684618,
            "unit": "median tps",
            "extra": "avg tps: 273.31698737146684, max tps: 2817.109953942506, count: 55569"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1047.6646278296112,
            "unit": "median tps",
            "extra": "avg tps: 1037.6218788093272, max tps: 1068.264407791821, count: 55569"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 121.06217323856242,
            "unit": "median tps",
            "extra": "avg tps: 157.72111963505643, max tps: 808.4267395853834, count: 111138"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 19.571492314283386,
            "unit": "median tps",
            "extra": "avg tps: 19.493504032238675, max tps: 20.02431687348693, count: 55569"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "2f61e5c2d8fb03377c37dcd10c558e9967781b97",
          "message": "feat: A Free Space Manager fronted by an AVL tree (#3252)\n\n## What\n\nThis implements a new `v2` FSM that's fronted by an AVL tree which\nallows for minimal locking during extension and draining. It also\nprovides efficient continuation during drain as xid blocklists are\nexhausted or found to be unavailable to the current transaction. And it\nimplements a (simple) transparent conversion of the current `v1` FSM to\nthe new format.\n\nAdditionally, this fixes a problem with background merging where more\nthan one background merger process could be spawned at once -- I've seen\nup to 8 concurrently. It does this by introducing some a new page on\ndisk to track the process and coordinate locking.\n\n## Why\n\nOur current FSM is very heavyweight in terms of lock contention. This\nshould get us to something that isn't.\n\n## How\n\n## Tests\n\nA number of new tests for the array-backed AVL tree and the FSM itself.\nAll existing tests also pass and, at least, the `wide-table.toml`\nstressgres shows a slight performance improvement for the update jobs.\n\n---------\n\nSigned-off-by: Eric Ridge <eebbrr@gmail.com>\nCo-authored-by: Stu Hood <stuhood@paradedb.com>",
          "timestamp": "2025-10-07T15:36:47-04:00",
          "tree_id": "31410dd4c4d2be73d287e97485f4d0faaf1b2932",
          "url": "https://github.com/paradedb/paradedb/commit/2f61e5c2d8fb03377c37dcd10c558e9967781b97"
        },
        "date": 1759868826572,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 37.09473848527078,
            "unit": "median tps",
            "extra": "avg tps: 37.15278274585468, max tps: 37.528097155684755, count: 55588"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 132.42819812931668,
            "unit": "median tps",
            "extra": "avg tps: 175.94105389959984, max tps: 2968.586602113102, count: 55588"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1055.5063228337328,
            "unit": "median tps",
            "extra": "avg tps: 1053.3095701632244, max tps: 1061.8313330900987, count: 55588"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 116.34455505099363,
            "unit": "median tps",
            "extra": "avg tps: 121.44539578803781, max tps: 814.6131003558486, count: 111176"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 19.499929520251218,
            "unit": "median tps",
            "extra": "avg tps: 19.784655814403074, max tps: 24.654095338003025, count: 55588"
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
          "id": "52dd41fee48d3a635a315610631235ff09ac53a5",
          "message": "chore: Upgrade to `0.18.10` (#3251)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-10-01T10:56:42-04:00",
          "tree_id": "fca37808df8505e2c564f422c635f688f82a0e1d",
          "url": "https://github.com/paradedb/paradedb/commit/52dd41fee48d3a635a315610631235ff09ac53a5"
        },
        "date": 1759333875618,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.497108,
            "unit": "median cpu",
            "extra": "avg cpu: 18.322596798393192, max cpu: 46.966736, count: 55643"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 156.10546875,
            "unit": "median mem",
            "extra": "avg mem: 155.1956762867746, max mem: 156.484375, count: 55643"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 7.66000764973947, max cpu: 27.906979, count: 55643"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 111.1640625,
            "unit": "median mem",
            "extra": "avg mem: 109.75634422512715, max mem: 111.1640625, count: 55643"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.956263637366348, max cpu: 13.819577, count: 55643"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 142.734375,
            "unit": "median mem",
            "extra": "avg mem: 121.89596596831588, max mem: 143.875, count: 55643"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 30764,
            "unit": "median block_count",
            "extra": "avg block_count: 31301.143216577108, max block_count: 64083.0, count: 55643"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6021094,
            "unit": "median cpu",
            "extra": "avg cpu: 2.363857542672842, max cpu: 4.6421666, count: 55643"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 100.5703125,
            "unit": "median mem",
            "extra": "avg mem: 90.09011697619198, max mem: 129.1328125, count: 55643"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 32,
            "unit": "median segment_count",
            "extra": "avg segment_count: 31.757022446668945, max segment_count: 56.0, count: 55643"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.248554,
            "unit": "median cpu",
            "extra": "avg cpu: 10.004271750145696, max cpu: 28.290766, count: 111286"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 149.6875,
            "unit": "median mem",
            "extra": "avg mem: 139.45504246238295, max mem: 154.48828125, count: 111286"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.832853,
            "unit": "median cpu",
            "extra": "avg cpu: 12.931829262844566, max cpu: 32.43243, count: 55643"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 158.9375,
            "unit": "median mem",
            "extra": "avg mem: 156.5190651087064, max mem: 160.328125, count: 55643"
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
        "date": 1759334844765,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.532818,
            "unit": "median cpu",
            "extra": "avg cpu: 18.917487068836696, max cpu: 46.64723, count: 55507"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 155.64453125,
            "unit": "median mem",
            "extra": "avg mem: 145.7910445135298, max mem: 156.75, count: 55507"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 7.674815424549638, max cpu: 27.961164, count: 55507"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 111.65625,
            "unit": "median mem",
            "extra": "avg mem: 110.46165376664204, max mem: 111.65625, count: 55507"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.962370711445244, max cpu: 13.994169, count: 55507"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 142.19140625,
            "unit": "median mem",
            "extra": "avg mem: 121.71754301540797, max mem: 142.94140625, count: 55507"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 30432,
            "unit": "median block_count",
            "extra": "avg block_count: 31020.87068297692, max block_count: 63591.0, count: 55507"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.198818662683081, max cpu: 4.6421666, count: 55507"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 102.80859375,
            "unit": "median mem",
            "extra": "avg mem: 90.6401218962248, max mem: 129.44921875, count: 55507"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 32,
            "unit": "median segment_count",
            "extra": "avg segment_count: 31.841047075143674, max segment_count: 54.0, count: 55507"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.266409,
            "unit": "median cpu",
            "extra": "avg cpu: 9.924330505732225, max cpu: 33.07087, count: 111014"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 149.30078125,
            "unit": "median mem",
            "extra": "avg mem: 140.22735565024456, max mem: 155.28515625, count: 111014"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.832853,
            "unit": "median cpu",
            "extra": "avg cpu: 11.992793736875072, max cpu: 27.799229, count: 55507"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 158.9765625,
            "unit": "median mem",
            "extra": "avg mem: 157.0005296347758, max mem: 160.3359375, count: 55507"
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
          "id": "bb494d330cfac7ee1db3b904ab5266bd671abadb",
          "message": "fix: `paradedb.snippet` projects correctly when called multiple times over the same field (#3257)\n\n# Ticket(s) Closed\n\n- Closes #3256 \n\n## What\n\nSee GH issue\n\n## Why\n\n## How\n\nWhen mapping a const node to our list of snippet generators, we were\nmatching only on the field name and function OID, which is not unique\nenough -- it doesn't account for the function arguments.\n\n## Tests\n\nAdded regression test",
          "timestamp": "2025-10-01T19:08:56-04:00",
          "tree_id": "a78e472aad70351c0c799699330da58256d3cbc4",
          "url": "https://github.com/paradedb/paradedb/commit/bb494d330cfac7ee1db3b904ab5266bd671abadb"
        },
        "date": 1759363331960,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.550726,
            "unit": "median cpu",
            "extra": "avg cpu: 18.32315935919452, max cpu: 46.376812, count: 55611"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 156.89453125,
            "unit": "median mem",
            "extra": "avg mem: 146.1543594258555, max mem: 158.01171875, count: 55611"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 7.691836347472932, max cpu: 42.31146, count: 55611"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 110.4296875,
            "unit": "median mem",
            "extra": "avg mem: 109.14861170114726, max mem: 110.4296875, count: 55611"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 5.0587532313924966, max cpu: 13.93998, count: 55611"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 144.4453125,
            "unit": "median mem",
            "extra": "avg mem: 123.34864912629696, max mem: 145.2109375, count: 55611"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 30646,
            "unit": "median block_count",
            "extra": "avg block_count: 31143.008253762746, max block_count: 63661.0, count: 55611"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.581265291716808, max cpu: 4.6647234, count: 55611"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 101.6796875,
            "unit": "median mem",
            "extra": "avg mem: 91.18955648668879, max mem: 130.29296875, count: 55611"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 31,
            "unit": "median segment_count",
            "extra": "avg segment_count: 31.650069230907555, max segment_count: 50.0, count: 55611"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.275363,
            "unit": "median cpu",
            "extra": "avg cpu: 9.955539571991679, max cpu: 37.463413, count: 111222"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 149.47265625,
            "unit": "median mem",
            "extra": "avg mem: 140.36863572264255, max mem: 156.20703125, count: 111222"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.859479,
            "unit": "median cpu",
            "extra": "avg cpu: 11.695727703719502, max cpu: 27.718958, count: 55611"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 157.875,
            "unit": "median mem",
            "extra": "avg mem: 156.20816692909227, max mem: 159.78125, count: 55611"
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
          "id": "762c487685bb5635c789b31781155839d2c65cf0",
          "message": "feat: Configure a limit/offset for snippets (#3254)",
          "timestamp": "2025-10-01T20:00:17-04:00",
          "tree_id": "5dcb534f0e2f513864b19abddc44396ed24760ff",
          "url": "https://github.com/paradedb/paradedb/commit/762c487685bb5635c789b31781155839d2c65cf0"
        },
        "date": 1759366503814,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.532818,
            "unit": "median cpu",
            "extra": "avg cpu: 18.389923746464092, max cpu: 42.146343, count: 55525"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 156.5546875,
            "unit": "median mem",
            "extra": "avg mem: 145.53859487561908, max mem: 156.5546875, count: 55525"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 7.662753062382549, max cpu: 42.60355, count: 55525"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 112.41796875,
            "unit": "median mem",
            "extra": "avg mem: 111.15273208858622, max mem: 112.41796875, count: 55525"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.957771151820104, max cpu: 14.0214205, count: 55525"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 144.52734375,
            "unit": "median mem",
            "extra": "avg mem: 123.00724553973436, max mem: 145.33203125, count: 55525"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 30676,
            "unit": "median block_count",
            "extra": "avg block_count: 31244.278901395766, max block_count: 64047.0, count: 55525"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.053604698330007, max cpu: 4.6647234, count: 55525"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 101.4375,
            "unit": "median mem",
            "extra": "avg mem: 90.45233903647006, max mem: 128.828125, count: 55525"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 32,
            "unit": "median segment_count",
            "extra": "avg segment_count: 31.792687978388113, max segment_count: 53.0, count: 55525"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.257474,
            "unit": "median cpu",
            "extra": "avg cpu: 9.787317941641486, max cpu: 42.60355, count: 111050"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 149.50390625,
            "unit": "median mem",
            "extra": "avg mem: 140.5949548697096, max mem: 156.796875, count: 111050"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.88621,
            "unit": "median cpu",
            "extra": "avg cpu: 13.411129076581984, max cpu: 27.906979, count: 55525"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 156.3359375,
            "unit": "median mem",
            "extra": "avg mem: 154.66892742570914, max mem: 158.19921875, count: 55525"
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
          "id": "850e3a9f88033d64151d6ecfa0d37c1b1f210b27",
          "message": "fix: `paradedb.snippet` projects correctly when called multiple times over the same field (#3258)\n\n# Ticket(s) Closed\n\n- Closes #3256 \n\n## What\n\nSee GH issue\n\n## Why\n\n## How\n\nWhen mapping a const node to our list of snippet generators, we were\nmatching only on the field name and function OID, which is not unique\nenough -- it doesn't account for the function arguments.\n\n## Tests\n\nAdded regression test\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-10-01T21:33:38-04:00",
          "tree_id": "1f119fb29d59884bd2105114833ca2d34c2acfd9",
          "url": "https://github.com/paradedb/paradedb/commit/850e3a9f88033d64151d6ecfa0d37c1b1f210b27"
        },
        "date": 1759372037052,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.514948,
            "unit": "median cpu",
            "extra": "avg cpu: 18.31539076682295, max cpu: 37.944664, count: 55523"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 155.953125,
            "unit": "median mem",
            "extra": "avg mem: 140.29643958078185, max mem: 155.953125, count: 55523"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 9.324588517368293, max cpu: 37.617554, count: 55523"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 113.49609375,
            "unit": "median mem",
            "extra": "avg mem: 111.96265321634728, max mem: 113.49609375, count: 55523"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.924217043100738, max cpu: 13.967022, count: 55523"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 141.90625,
            "unit": "median mem",
            "extra": "avg mem: 114.84291778575995, max mem: 143.4609375, count: 55523"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 27836,
            "unit": "median block_count",
            "extra": "avg block_count: 28951.25272409632, max block_count: 60563.0, count: 55523"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6153846,
            "unit": "median cpu",
            "extra": "avg cpu: 4.332938884842781, max cpu: 4.7058825, count: 55523"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 89.3203125,
            "unit": "median mem",
            "extra": "avg mem: 83.60635365918178, max mem: 126.46875, count: 55523"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 31,
            "unit": "median segment_count",
            "extra": "avg segment_count: 31.39444914720026, max segment_count: 52.0, count: 55523"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.29332,
            "unit": "median cpu",
            "extra": "avg cpu: 10.874453577095444, max cpu: 32.526623, count: 111046"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 150.796875,
            "unit": "median mem",
            "extra": "avg mem: 137.01055140578003, max mem: 156.51171875, count: 111046"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.872832,
            "unit": "median cpu",
            "extra": "avg cpu: 12.565322194429427, max cpu: 28.125, count: 55523"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 156.5703125,
            "unit": "median mem",
            "extra": "avg mem: 154.55695239754246, max mem: 157.6875, count: 55523"
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
          "id": "2ce078f8496ff0152f7373fe94348a9cfcacd5af",
          "message": "feat: Configure a limit/offset for snippets (#3259)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n`paradedb.snippet` and `paradedb.snippet_positions` now take a limit and\noffset. For instance, if 5 snippets are found in a doc and offset is 1,\nthen the first snippet will be skipped.\n\n## Why\n\n## How\n\n## Tests\n\nSee regression test\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-10-01T22:10:01-04:00",
          "tree_id": "904048e17b5e988830cd9302f007cb5d18411d22",
          "url": "https://github.com/paradedb/paradedb/commit/2ce078f8496ff0152f7373fe94348a9cfcacd5af"
        },
        "date": 1759374216555,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.550726,
            "unit": "median cpu",
            "extra": "avg cpu: 18.731542569941787, max cpu: 42.39451, count: 55459"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 155.10546875,
            "unit": "median mem",
            "extra": "avg mem: 143.95912584352854, max mem: 155.48046875, count: 55459"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 7.671031967049345, max cpu: 41.65863, count: 55459"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 111.96484375,
            "unit": "median mem",
            "extra": "avg mem: 110.79773289727547, max mem: 111.96484375, count: 55459"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.996986098606978, max cpu: 11.302983, count: 55459"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 141.78125,
            "unit": "median mem",
            "extra": "avg mem: 120.98388392776646, max mem: 142.53515625, count: 55459"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 30871,
            "unit": "median block_count",
            "extra": "avg block_count: 31340.896914837987, max block_count: 64019.0, count: 55459"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.12145224166995, max cpu: 4.669261, count: 55459"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 102.390625,
            "unit": "median mem",
            "extra": "avg mem: 90.70151401260391, max mem: 128.68359375, count: 55459"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 31,
            "unit": "median segment_count",
            "extra": "avg segment_count: 31.578427306658973, max segment_count: 52.0, count: 55459"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.266409,
            "unit": "median cpu",
            "extra": "avg cpu: 9.944786961652406, max cpu: 41.65863, count: 110918"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 148.91796875,
            "unit": "median mem",
            "extra": "avg mem: 139.89154745087586, max mem: 155.3046875, count: 110918"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.859479,
            "unit": "median cpu",
            "extra": "avg cpu: 12.81537801680051, max cpu: 27.988338, count: 55459"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 156.62890625,
            "unit": "median mem",
            "extra": "avg mem: 154.52069800438161, max mem: 158.4765625, count: 55459"
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
          "id": "6335204e0cc58bdeb1ea12f236ab2258a44cd192",
          "message": "chore: Upgrade to `0.18.11` (#3261)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-10-02T10:00:31-04:00",
          "tree_id": "05d18415099467cbf114e4a87c3b536bfeafb509",
          "url": "https://github.com/paradedb/paradedb/commit/6335204e0cc58bdeb1ea12f236ab2258a44cd192"
        },
        "date": 1759416940184,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.514948,
            "unit": "median cpu",
            "extra": "avg cpu: 18.63751480305884, max cpu: 42.519684, count: 55433"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 154.73046875,
            "unit": "median mem",
            "extra": "avg mem: 143.9396773892582, max mem: 156.1796875, count: 55433"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 7.659318541998368, max cpu: 27.906979, count: 55433"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 111.68359375,
            "unit": "median mem",
            "extra": "avg mem: 110.41428785651146, max mem: 111.68359375, count: 55433"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.958292248499366, max cpu: 13.9265, count: 55433"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 140.703125,
            "unit": "median mem",
            "extra": "avg mem: 119.81938478940793, max mem: 141.48046875, count: 55433"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 30769,
            "unit": "median block_count",
            "extra": "avg block_count: 31198.480616239423, max block_count: 63864.0, count: 55433"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 3.979826655265791, max cpu: 4.6421666, count: 55433"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 102.48828125,
            "unit": "median mem",
            "extra": "avg mem: 90.28821695504031, max mem: 128.75, count: 55433"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 31,
            "unit": "median segment_count",
            "extra": "avg segment_count: 31.585337253982285, max segment_count: 52.0, count: 55433"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.257474,
            "unit": "median cpu",
            "extra": "avg cpu: 10.055074983474363, max cpu: 28.346458, count: 110866"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 149.42578125,
            "unit": "median mem",
            "extra": "avg mem: 140.4754665471718, max mem: 156.9296875, count: 110866"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.846154,
            "unit": "median cpu",
            "extra": "avg cpu: 12.71178139505388, max cpu: 27.961164, count: 55433"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 157.6953125,
            "unit": "median mem",
            "extra": "avg mem: 155.82771417183807, max mem: 159.6796875, count: 55433"
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
          "id": "7f394c9cc9b86839fb39f22123893552f6e5a291",
          "message": "chore: Upgrade to `0.18.11` (#3262)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-10-02T10:04:40-04:00",
          "tree_id": "fd395f6aeafa74dd786657eebbf6f9cba6e6bf28",
          "url": "https://github.com/paradedb/paradedb/commit/7f394c9cc9b86839fb39f22123893552f6e5a291"
        },
        "date": 1759417207450,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.550726,
            "unit": "median cpu",
            "extra": "avg cpu: 18.702460007165282, max cpu: 46.10951, count: 55493"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 154.5546875,
            "unit": "median mem",
            "extra": "avg mem: 143.86119420467446, max mem: 154.5546875, count: 55493"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 7.643293702112748, max cpu: 28.070175, count: 55493"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 110.65625,
            "unit": "median mem",
            "extra": "avg mem: 109.50227147173968, max mem: 110.65625, count: 55493"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.961260914395654, max cpu: 13.953489, count: 55493"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 142.1875,
            "unit": "median mem",
            "extra": "avg mem: 120.67764395678283, max mem: 142.1875, count: 55493"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 30466,
            "unit": "median block_count",
            "extra": "avg block_count: 30999.750455012345, max block_count: 63317.0, count: 55493"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 3.1797145533738482, max cpu: 4.6511626, count: 55493"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 101.6171875,
            "unit": "median mem",
            "extra": "avg mem: 90.03416590831276, max mem: 129.015625, count: 55493"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 32,
            "unit": "median segment_count",
            "extra": "avg segment_count: 31.890905159209268, max segment_count: 56.0, count: 55493"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.266409,
            "unit": "median cpu",
            "extra": "avg cpu: 9.93311548595774, max cpu: 33.005894, count: 110986"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 148.27734375,
            "unit": "median mem",
            "extra": "avg mem: 139.91010387852972, max mem: 156.26953125, count: 110986"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.899614,
            "unit": "median cpu",
            "extra": "avg cpu: 13.010841976572783, max cpu: 27.87996, count: 55493"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 158.53515625,
            "unit": "median mem",
            "extra": "avg mem: 157.09898714702754, max mem: 162.15625, count: 55493"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "stuhood@paradedb.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "6b15d1a8e21e76267b06b7e54dbbb5194fda55b9",
          "message": "chore: Improve determinism of property tests. (#3220)\n\n## What\n\nSet a seed to control what is generated by `random()` in the property\ntests, and render it in the reproduction script after a failure.\n\n## Why\n\nTo make it easier to reproduce property test failures by running over\nreproducible data.\n\n`proptest` failures are only directly reproducible via their reported\nseed if they are not dependent on the randomly generated data in the\ntable that we test against. We can't re-generate the table and index for\nevery `proptest` query that we run, because it would take way too long\nto run a reasonable number of iterations. And we don't want to run on\nstatic data, because then we might never catch data-dependent bugs like\n#3266.\n\nSetting a seed allows us to run on random data, but still reproduce\nfailures later. And for cases where failures aren't data-dependent, the\n`proptest` repro seed (e.g. `cc 08176a8c0ae10938a...`) can still be used\ndirectly.",
          "timestamp": "2025-10-03T13:48:13-07:00",
          "tree_id": "ac9934e860da0bdb5b4b083df652ebc3d85309d4",
          "url": "https://github.com/paradedb/paradedb/commit/6b15d1a8e21e76267b06b7e54dbbb5194fda55b9"
        },
        "date": 1759527725403,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.532818,
            "unit": "median cpu",
            "extra": "avg cpu: 17.961600735904053, max cpu: 37.944664, count: 55664"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 116.22265625,
            "unit": "median mem",
            "extra": "avg mem: 132.61740272831184, max mem: 154.82421875, count: 55664"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.7105007,
            "unit": "median cpu",
            "extra": "avg cpu: 12.617041194775732, max cpu: 42.687748, count: 55664"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 111.25,
            "unit": "median mem",
            "extra": "avg mem: 109.92830897213639, max mem: 111.25, count: 55664"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 5.040250300855126, max cpu: 9.430255, count: 55664"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 72.1875,
            "unit": "median mem",
            "extra": "avg mem: 100.37328652439189, max mem: 142.515625, count: 55664"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 22441,
            "unit": "median block_count",
            "extra": "avg block_count: 26065.359154929578, max block_count: 56732.0, count: 55664"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6153846,
            "unit": "median cpu",
            "extra": "avg cpu: 4.067214095029447, max cpu: 4.6511626, count: 55664"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 57.56640625,
            "unit": "median mem",
            "extra": "avg mem: 72.88318436006665, max mem: 122.20703125, count: 55664"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 31,
            "unit": "median segment_count",
            "extra": "avg segment_count: 30.65591764874964, max segment_count: 55.0, count: 55664"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 13.872832,
            "unit": "median cpu",
            "extra": "avg cpu: 12.790123551236233, max cpu: 42.687748, count: 111328"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 149.07421875,
            "unit": "median mem",
            "extra": "avg mem: 129.36448812854806, max mem: 154.8828125, count: 111328"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.819577,
            "unit": "median cpu",
            "extra": "avg cpu: 11.64145937745889, max cpu: 23.323614, count: 55664"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 156.33984375,
            "unit": "median mem",
            "extra": "avg mem: 154.7396807603433, max mem: 157.83203125, count: 55664"
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
          "id": "34e2d538b7327fc30b32f6ee33b55cbc9ccb2749",
          "message": "chore: Remove deprecated tokenizers: `en_stem`, `stem` and `lowercase` (#3279)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nFor better maintainability:\n\n1. Removes three extremely deprecated tokenizers: `en_stem`, `stem`, and\n`lowercase`\n2. Wraps the filter builders in a macro, guaranteeing that all the\nfilters are applied to all tokenizers\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-10-07T11:16:19-04:00",
          "tree_id": "7ef65d52251d2f4fb83439b6887924fa19564416",
          "url": "https://github.com/paradedb/paradedb/commit/34e2d538b7327fc30b32f6ee33b55cbc9ccb2749"
        },
        "date": 1759853181273,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.532818,
            "unit": "median cpu",
            "extra": "avg cpu: 19.131247428355827, max cpu: 46.198265, count: 55720"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 155.6640625,
            "unit": "median mem",
            "extra": "avg mem: 144.2773235597631, max mem: 155.6640625, count: 55720"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 7.680030603282074, max cpu: 27.961164, count: 55720"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 112.01171875,
            "unit": "median mem",
            "extra": "avg mem: 110.817454389694, max mem: 112.01171875, count: 55720"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.953895940219991, max cpu: 13.846154, count: 55720"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 140.3671875,
            "unit": "median mem",
            "extra": "avg mem: 119.79253080413227, max mem: 141.1171875, count: 55720"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 31163,
            "unit": "median block_count",
            "extra": "avg block_count: 31816.60936826992, max block_count: 65448.0, count: 55720"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.53650950162927, max cpu: 4.669261, count: 55720"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 100.6328125,
            "unit": "median mem",
            "extra": "avg mem: 90.58970823705582, max mem: 127.38671875, count: 55720"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 32,
            "unit": "median segment_count",
            "extra": "avg segment_count: 31.611934673366836, max segment_count: 54.0, count: 55720"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.248554,
            "unit": "median cpu",
            "extra": "avg cpu: 9.77577461033994, max cpu: 28.042841, count: 111440"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 150.7421875,
            "unit": "median mem",
            "extra": "avg mem: 140.82044871399407, max mem: 155.85546875, count: 111440"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.806328,
            "unit": "median cpu",
            "extra": "avg cpu: 11.787031839923129, max cpu: 27.961164, count: 55720"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 156.640625,
            "unit": "median mem",
            "extra": "avg mem: 154.80192683562905, max mem: 158.37890625, count: 55720"
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
          "id": "7f394c9cc9b86839fb39f22123893552f6e5a291",
          "message": "chore: Upgrade to `0.18.11` (#3262)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-10-02T10:04:40-04:00",
          "tree_id": "fd395f6aeafa74dd786657eebbf6f9cba6e6bf28",
          "url": "https://github.com/paradedb/paradedb/commit/7f394c9cc9b86839fb39f22123893552f6e5a291"
        },
        "date": 1759859223094,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.532818,
            "unit": "median cpu",
            "extra": "avg cpu: 18.4638607661908, max cpu: 38.057484, count: 55555"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 156.765625,
            "unit": "median mem",
            "extra": "avg mem: 145.57081418470435, max mem: 157.1875, count: 55555"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 7.676549594032776, max cpu: 47.19764, count: 55555"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 112.55078125,
            "unit": "median mem",
            "extra": "avg mem: 111.31565671125462, max mem: 112.55078125, count: 55555"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 5.041650579495534, max cpu: 13.93998, count: 55555"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 141.265625,
            "unit": "median mem",
            "extra": "avg mem: 120.45950254815048, max mem: 142.01953125, count: 55555"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 30875,
            "unit": "median block_count",
            "extra": "avg block_count: 31423.755467554674, max block_count: 64419.0, count: 55555"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.447645050615718, max cpu: 4.7477746, count: 55555"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 104.703125,
            "unit": "median mem",
            "extra": "avg mem: 92.53550823789487, max mem: 130.671875, count: 55555"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 31,
            "unit": "median segment_count",
            "extra": "avg segment_count: 31.565097650976508, max segment_count: 54.0, count: 55555"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.266409,
            "unit": "median cpu",
            "extra": "avg cpu: 9.95320358895547, max cpu: 47.244095, count: 111110"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 148.93359375,
            "unit": "median mem",
            "extra": "avg mem: 140.79030050456754, max mem: 157.3515625, count: 111110"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.832853,
            "unit": "median cpu",
            "extra": "avg cpu: 12.181035691926548, max cpu: 27.745665, count: 55555"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 158.15625,
            "unit": "median mem",
            "extra": "avg mem: 156.1782102899154, max mem: 158.84765625, count: 55555"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "b8ebd1ad157e946d6fff8f775aec3189bc469325",
          "message": "fix: possible naming collisions with builder functions (#3275)\n\n## What\n\nFix an issue where functions tagged with our `#[builder_fn]` macro could\nend up with the same name.\n\n## Why\n\nIt's come up in CI once or twice and I've seen it locally as well\n\n## How\n\n## Tests",
          "timestamp": "2025-10-07T15:35:22-04:00",
          "tree_id": "bd88762bb48211807276741ce46d155ea36600b3",
          "url": "https://github.com/paradedb/paradedb/commit/b8ebd1ad157e946d6fff8f775aec3189bc469325"
        },
        "date": 1759868747199,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.514948,
            "unit": "median cpu",
            "extra": "avg cpu: 18.437645907681365, max cpu: 41.982506, count: 55569"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 154.26171875,
            "unit": "median mem",
            "extra": "avg mem: 143.70632710009178, max mem: 154.66796875, count: 55569"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 7.68638432900271, max cpu: 42.39451, count: 55569"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 111.2421875,
            "unit": "median mem",
            "extra": "avg mem: 110.04102739949433, max mem: 111.2421875, count: 55569"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.964247140732123, max cpu: 13.859479, count: 55569"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 141.9609375,
            "unit": "median mem",
            "extra": "avg mem: 121.18351614378071, max mem: 143.46875, count: 55569"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 30865,
            "unit": "median block_count",
            "extra": "avg block_count: 31373.955388795912, max block_count: 64434.0, count: 55569"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.5796682885620905, max cpu: 4.655674, count: 55569"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 101.44140625,
            "unit": "median mem",
            "extra": "avg mem: 91.06195788119275, max mem: 129.203125, count: 55569"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 31,
            "unit": "median segment_count",
            "extra": "avg segment_count: 31.613219600856592, max segment_count: 50.0, count: 55569"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.257474,
            "unit": "median cpu",
            "extra": "avg cpu: 9.873518629578772, max cpu: 42.39451, count: 111138"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 151.5234375,
            "unit": "median mem",
            "extra": "avg mem: 140.80120193335088, max mem: 156.1640625, count: 111138"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.819577,
            "unit": "median cpu",
            "extra": "avg cpu: 12.16672934173875, max cpu: 27.826086, count: 55569"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 155.89453125,
            "unit": "median mem",
            "extra": "avg mem: 154.28846189534633, max mem: 157.68359375, count: 55569"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "2f61e5c2d8fb03377c37dcd10c558e9967781b97",
          "message": "feat: A Free Space Manager fronted by an AVL tree (#3252)\n\n## What\n\nThis implements a new `v2` FSM that's fronted by an AVL tree which\nallows for minimal locking during extension and draining. It also\nprovides efficient continuation during drain as xid blocklists are\nexhausted or found to be unavailable to the current transaction. And it\nimplements a (simple) transparent conversion of the current `v1` FSM to\nthe new format.\n\nAdditionally, this fixes a problem with background merging where more\nthan one background merger process could be spawned at once -- I've seen\nup to 8 concurrently. It does this by introducing some a new page on\ndisk to track the process and coordinate locking.\n\n## Why\n\nOur current FSM is very heavyweight in terms of lock contention. This\nshould get us to something that isn't.\n\n## How\n\n## Tests\n\nA number of new tests for the array-backed AVL tree and the FSM itself.\nAll existing tests also pass and, at least, the `wide-table.toml`\nstressgres shows a slight performance improvement for the update jobs.\n\n---------\n\nSigned-off-by: Eric Ridge <eebbrr@gmail.com>\nCo-authored-by: Stu Hood <stuhood@paradedb.com>",
          "timestamp": "2025-10-07T15:36:47-04:00",
          "tree_id": "31410dd4c4d2be73d287e97485f4d0faaf1b2932",
          "url": "https://github.com/paradedb/paradedb/commit/2f61e5c2d8fb03377c37dcd10c558e9967781b97"
        },
        "date": 1759868829799,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.532818,
            "unit": "median cpu",
            "extra": "avg cpu: 18.732559401977092, max cpu: 42.60355, count: 55588"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 156.86328125,
            "unit": "median mem",
            "extra": "avg mem: 155.7995123582203, max mem: 156.86328125, count: 55588"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 9.262682397691245, max cpu: 28.015566, count: 55588"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 111.703125,
            "unit": "median mem",
            "extra": "avg mem: 110.31838860792797, max mem: 111.703125, count: 55588"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.961334583231509, max cpu: 13.913043, count: 55588"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 145.78515625,
            "unit": "median mem",
            "extra": "avg mem: 116.61494872150914, max mem: 146.18359375, count: 55588"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 28267,
            "unit": "median block_count",
            "extra": "avg block_count: 29403.89895301144, max block_count: 61393.0, count: 55588"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.133445914597494, max cpu: 4.655674, count: 55588"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 91.015625,
            "unit": "median mem",
            "extra": "avg mem: 84.18707485765813, max mem: 126.77734375, count: 55588"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 31,
            "unit": "median segment_count",
            "extra": "avg segment_count: 31.40386414334029, max segment_count: 49.0, count: 55588"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.284333,
            "unit": "median cpu",
            "extra": "avg cpu: 10.800983235207498, max cpu: 28.374382, count: 111176"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 149.12109375,
            "unit": "median mem",
            "extra": "avg mem: 136.2149652844926, max mem: 155.015625, count: 111176"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.846154,
            "unit": "median cpu",
            "extra": "avg cpu: 12.357029139539963, max cpu: 27.906979, count: 55588"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 159.05078125,
            "unit": "median mem",
            "extra": "avg mem: 157.04901989581833, max mem: 160.06640625, count: 55588"
          }
        ]
      }
    ]
  }
}