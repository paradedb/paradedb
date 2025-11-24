window.BENCHMARK_DATA = {
  "lastUpdate": 1764022612081,
  "repoUrl": "https://github.com/paradedb/paradedb",
  "entries": {
    "pg_search single-server.toml Performance - TPS": [
      {
        "commit": {
          "author": {
            "email": "stuhood@paradedb.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "committer": {
            "email": "stuhood@gmail.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "distinct": true,
          "id": "099c58ef752c337320d3e06a685cf80ba86e533a",
          "message": "chore: Prepare `0.19.9`. (#3604)\n\nPrepare `0.19.9`.",
          "timestamp": "2025-11-23T15:34:49-08:00",
          "tree_id": "108a1316f3541a472d93c2c75d3a050eb585ba61",
          "url": "https://github.com/paradedb/paradedb/commit/099c58ef752c337320d3e06a685cf80ba86e533a"
        },
        "date": 1763941879025,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 145.55320856357247,
            "unit": "median tps",
            "extra": "avg tps: 162.1834713371906, max tps: 599.9327510676216, count: 55475"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3166.568078774087,
            "unit": "median tps",
            "extra": "avg tps: 3139.6648904855997, max tps: 3175.2762284966816, count: 55475"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 144.00076971107828,
            "unit": "median tps",
            "extra": "avg tps: 160.59372935389624, max tps: 621.466521098898, count: 55475"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 124.7347684341972,
            "unit": "median tps",
            "extra": "avg tps: 139.06115491738797, max tps: 455.5622384005028, count: 55475"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 3389.887335027361,
            "unit": "median tps",
            "extra": "avg tps: 3385.243831874746, max tps: 3432.1868995077116, count: 110950"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 2119.2357741234373,
            "unit": "median tps",
            "extra": "avg tps: 2107.548878633217, max tps: 2139.8052414514104, count: 55475"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 38.557477603385635,
            "unit": "median tps",
            "extra": "avg tps: 64.02303712895271, max tps: 702.6690885327918, count: 55475"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Moe",
            "username": "mdashti",
            "email": "mdashti@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "21ae4f92dc730f588c957d8cb5c893b916d95409",
          "message": "feat: supported window aggregate pushdown for all search operators (#3582)\n\n# Ticket(s) Closed\n\n- Closes #3566\n\n## What\n\nWindow aggregate queries with `|||`, `&&&`, `===`, and `###` operators\nnow properly push down to TopN scans, just like queries using the `@@@`\noperator.\n\n## Why\n\nPreviously, queries like `SELECT *, COUNT(*) OVER () FROM table WHERE\nfield ||| 'term' ORDER BY rating LIMIT 10` would fall back to\nPostgreSQL's standard WindowAgg execution path instead of using\noptimized TopN scan. This happened because the window function\nreplacement logic only checked for the `@@@` operator when deciding\nwhether to enable pushdown.\n\n## How\n\n- Added helper functions to get OIDs for all ParadeDB search operators:\n`match_disjunction_text_opoid()` for `|||`,\n`match_conjunction_text_opoid()` for `&&&`, `term_text_opoid()` for\n`===`, and `phrase_text_opoid()` for `###`\n- Updated `query_has_search_operator()` to check for all search\noperators, not just `@@@`\n\n## Tests\n\nAdded tests in `topn-agg-facet.sql` (Tests 1a-1d) verifying that window\naggregate queries with `|||`, `&&&`, `===`, and `###` operators properly\nuse TopNScanExecState execution.",
          "timestamp": "2025-11-21T21:49:27Z",
          "url": "https://github.com/paradedb/paradedb/commit/21ae4f92dc730f588c957d8cb5c893b916d95409"
        },
        "date": 1763941942513,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 149.58352251515538,
            "unit": "median tps",
            "extra": "avg tps: 165.63954835406065, max tps: 578.4240464281928, count: 55470"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3184.4783879388183,
            "unit": "median tps",
            "extra": "avg tps: 3145.107350427964, max tps: 3205.783486273771, count: 55470"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 140.69165855730824,
            "unit": "median tps",
            "extra": "avg tps: 156.6071047933291, max tps: 627.3151205928477, count: 55470"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 119.99774655181447,
            "unit": "median tps",
            "extra": "avg tps: 134.3358910052554, max tps: 465.55346625505535, count: 55470"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 3368.619330233573,
            "unit": "median tps",
            "extra": "avg tps: 3388.568257316892, max tps: 3528.0936568938687, count: 110940"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 2202.950210204094,
            "unit": "median tps",
            "extra": "avg tps: 2175.836584300534, max tps: 2212.761035304369, count: 55470"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 56.32962927394826,
            "unit": "median tps",
            "extra": "avg tps: 75.92195832714017, max tps: 371.4813748525683, count: 55470"
          }
        ]
      },
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
        "date": 1763941984110,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 1173.4935093822635,
            "unit": "median tps",
            "extra": "avg tps: 1172.0298816838908, max tps: 1236.0801765828114, count: 54945"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2708.8403958252748,
            "unit": "median tps",
            "extra": "avg tps: 2693.5851024921385, max tps: 2720.0910658614894, count: 54945"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 1164.5661718196184,
            "unit": "median tps",
            "extra": "avg tps: 1165.1100017667732, max tps: 1199.2480391919794, count: 54945"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 1016.7883711375777,
            "unit": "median tps",
            "extra": "avg tps: 1012.056425337584, max tps: 1023.0308772849636, count: 54945"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 160.2655246892727,
            "unit": "median tps",
            "extra": "avg tps: 175.24821711724235, max tps: 196.1800953641776, count: 109890"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 139.99312742779162,
            "unit": "median tps",
            "extra": "avg tps: 139.74469230165099, max tps: 146.84119144723806, count: 54945"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 21.704564051022327,
            "unit": "median tps",
            "extra": "avg tps: 27.175835120487587, max tps: 873.688266279215, count: 54945"
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
          "id": "f4cc34211b69c210cad6133141ec34b114d4e528",
          "message": "docs: fix more-like-this (MLT) JSON syntax example (#3335)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-10-14T19:24:46Z",
          "url": "https://github.com/paradedb/paradedb/commit/f4cc34211b69c210cad6133141ec34b114d4e528"
        },
        "date": 1763942018808,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 790.3830119159277,
            "unit": "median tps",
            "extra": "avg tps: 789.3770220533647, max tps: 832.3569104153414, count: 55247"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3497.977138740746,
            "unit": "median tps",
            "extra": "avg tps: 3476.1515400901562, max tps: 3504.974426304614, count: 55247"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 782.950236836777,
            "unit": "median tps",
            "extra": "avg tps: 781.9976307415394, max tps: 784.8310959713369, count: 55247"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 688.2606844263283,
            "unit": "median tps",
            "extra": "avg tps: 685.4956359379728, max tps: 690.7594419840556, count: 55247"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1758.7407967571337,
            "unit": "median tps",
            "extra": "avg tps: 1753.57585720799, max tps: 1771.1217287748789, count: 110494"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1274.5909120288939,
            "unit": "median tps",
            "extra": "avg tps: 1264.9508144652457, max tps: 1277.4202325281817, count: 55247"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 143.61598592756357,
            "unit": "median tps",
            "extra": "avg tps: 152.16327747733683, max tps: 486.9414477256183, count: 55247"
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
        "date": 1763942022418,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 1182.635715137167,
            "unit": "median tps",
            "extra": "avg tps: 1174.6895542720797, max tps: 1187.0729672736038, count: 55240"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2779.351646471533,
            "unit": "median tps",
            "extra": "avg tps: 2741.5090743785527, max tps: 2790.815489921588, count: 55240"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 1149.1657074276918,
            "unit": "median tps",
            "extra": "avg tps: 1143.835165986798, max tps: 1151.9641330489808, count: 55240"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 923.8160518575752,
            "unit": "median tps",
            "extra": "avg tps: 919.5050336774453, max tps: 930.0439349553, count: 55240"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 165.57489418933858,
            "unit": "median tps",
            "extra": "avg tps: 170.08075040862548, max tps: 179.04574998991805, count: 110480"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 143.0287317424585,
            "unit": "median tps",
            "extra": "avg tps: 142.71607980304057, max tps: 143.65582058490432, count: 55240"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 104.83920155528979,
            "unit": "median tps",
            "extra": "avg tps: 139.4917157571142, max tps: 329.4486939665425, count: 55240"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mithun.cy@gmail.com",
            "name": "Mithun Chicklore Yogendra",
            "username": "mithuncy"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "a90bb936041bd4583034d9d1a538943756064dc4",
          "message": "fix: Skip flaky edge case numeric values in json_pushdown prop tests (#3620)\n\n## Summary\n- Skip edge case numeric values added in PR #2978 that cause\nintermittent prop test failures on main\n\n## Test plan\n- [ ] Verify prop tests pass without flaky failures",
          "timestamp": "2025-11-25T03:05:45+05:30",
          "tree_id": "95bf53a51121e3433d5f9df4eaa3649da9add90e",
          "url": "https://github.com/paradedb/paradedb/commit/a90bb936041bd4583034d9d1a538943756064dc4"
        },
        "date": 1764021144209,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 146.35801983287269,
            "unit": "median tps",
            "extra": "avg tps: 164.0075059143265, max tps: 594.0388298471055, count: 55298"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3327.6204694638973,
            "unit": "median tps",
            "extra": "avg tps: 3329.4873358993723, max tps: 3373.189972587204, count: 55298"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 141.9345123530105,
            "unit": "median tps",
            "extra": "avg tps: 159.44897608595807, max tps: 602.9903836781984, count: 55298"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 126.34193364920543,
            "unit": "median tps",
            "extra": "avg tps: 140.12255073219774, max tps: 422.4951379863085, count: 55298"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 3436.1761103842487,
            "unit": "median tps",
            "extra": "avg tps: 3429.7731496829283, max tps: 3460.078883812429, count: 110596"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 2199.676448604965,
            "unit": "median tps",
            "extra": "avg tps: 2190.1890790571956, max tps: 2211.511698235189, count: 55298"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 48.6553382487192,
            "unit": "median tps",
            "extra": "avg tps: 68.97001654598411, max tps: 997.4873294171982, count: 55298"
          }
        ]
      }
    ],
    "pg_search single-server.toml Performance - Other Metrics": [
      {
        "commit": {
          "author": {
            "email": "stuhood@paradedb.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "committer": {
            "email": "stuhood@gmail.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "distinct": true,
          "id": "099c58ef752c337320d3e06a685cf80ba86e533a",
          "message": "chore: Prepare `0.19.9`. (#3604)\n\nPrepare `0.19.9`.",
          "timestamp": "2025-11-23T15:34:49-08:00",
          "tree_id": "108a1316f3541a472d93c2c75d3a050eb585ba61",
          "url": "https://github.com/paradedb/paradedb/commit/099c58ef752c337320d3e06a685cf80ba86e533a"
        },
        "date": 1763941886022,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 11.276429,
            "unit": "median cpu",
            "extra": "avg cpu: 11.64083660368523, max cpu: 38.670696, count: 55475"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 50.74609375,
            "unit": "median mem",
            "extra": "avg mem: 50.451731142969805, max mem: 60.3359375, count: 55475"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.606670851601693, max cpu: 9.356726, count: 55475"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 26.19140625,
            "unit": "median mem",
            "extra": "avg mem: 26.259539418093738, max mem: 26.96875, count: 55475"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 9.504951,
            "unit": "median cpu",
            "extra": "avg cpu: 11.652218788580665, max cpu: 35.43624, count: 55475"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 52.62890625,
            "unit": "median mem",
            "extra": "avg mem: 50.6058532137224, max mem: 57.80859375, count: 55475"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 5.192739874855518, max cpu: 14.201183, count: 55475"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 50.32421875,
            "unit": "median mem",
            "extra": "avg mem: 49.66304536108608, max mem: 58.921875, count: 55475"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.655225759402676, max cpu: 9.302325, count: 110950"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 35.4765625,
            "unit": "median mem",
            "extra": "avg mem: 35.28190394040108, max mem: 43.609375, count: 110950"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 1329,
            "unit": "median block_count",
            "extra": "avg block_count: 1328.251554754394, max block_count: 2297.0, count: 55475"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 26,
            "unit": "median segment_count",
            "extra": "avg segment_count: 25.87232086525462, max segment_count: 49.0, count: 55475"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.535360556270854, max cpu: 4.7571855, count: 55475"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 37.69921875,
            "unit": "median mem",
            "extra": "avg mem: 38.55641561514196, max mem: 46.4453125, count: 55475"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.2204283675359955, max cpu: 9.329447, count: 55475"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 41.5,
            "unit": "median mem",
            "extra": "avg mem: 40.676338792812075, max mem: 48.73046875, count: 55475"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Moe",
            "username": "mdashti",
            "email": "mdashti@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "21ae4f92dc730f588c957d8cb5c893b916d95409",
          "message": "feat: supported window aggregate pushdown for all search operators (#3582)\n\n# Ticket(s) Closed\n\n- Closes #3566\n\n## What\n\nWindow aggregate queries with `|||`, `&&&`, `===`, and `###` operators\nnow properly push down to TopN scans, just like queries using the `@@@`\noperator.\n\n## Why\n\nPreviously, queries like `SELECT *, COUNT(*) OVER () FROM table WHERE\nfield ||| 'term' ORDER BY rating LIMIT 10` would fall back to\nPostgreSQL's standard WindowAgg execution path instead of using\noptimized TopN scan. This happened because the window function\nreplacement logic only checked for the `@@@` operator when deciding\nwhether to enable pushdown.\n\n## How\n\n- Added helper functions to get OIDs for all ParadeDB search operators:\n`match_disjunction_text_opoid()` for `|||`,\n`match_conjunction_text_opoid()` for `&&&`, `term_text_opoid()` for\n`===`, and `phrase_text_opoid()` for `###`\n- Updated `query_has_search_operator()` to check for all search\noperators, not just `@@@`\n\n## Tests\n\nAdded tests in `topn-agg-facet.sql` (Tests 1a-1d) verifying that window\naggregate queries with `|||`, `&&&`, `===`, and `###` operators properly\nuse TopNScanExecState execution.",
          "timestamp": "2025-11-21T21:49:27Z",
          "url": "https://github.com/paradedb/paradedb/commit/21ae4f92dc730f588c957d8cb5c893b916d95409"
        },
        "date": 1763941946024,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 9.4395275,
            "unit": "median cpu",
            "extra": "avg cpu: 11.429327305476951, max cpu: 33.300297, count: 55470"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 51.30078125,
            "unit": "median mem",
            "extra": "avg mem: 50.71170860938345, max mem: 61.52734375, count: 55470"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.640907438399282, max cpu: 9.542743, count: 55470"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 26.68359375,
            "unit": "median mem",
            "extra": "avg mem: 26.663514202496845, max mem: 28.28515625, count: 55470"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 13.872832,
            "unit": "median cpu",
            "extra": "avg cpu: 12.064716102010452, max cpu: 41.69884, count: 55470"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 52.73828125,
            "unit": "median mem",
            "extra": "avg mem: 50.72292863822787, max mem: 60.47265625, count: 55470"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 5.23521330024068, max cpu: 14.007783, count: 55470"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 50.2265625,
            "unit": "median mem",
            "extra": "avg mem: 49.58528026128989, max mem: 58.9375, count: 55470"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.633594313485873, max cpu: 9.320388, count: 110940"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 34.875,
            "unit": "median mem",
            "extra": "avg mem: 34.75466316956238, max mem: 43.27734375, count: 110940"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 1474,
            "unit": "median block_count",
            "extra": "avg block_count: 1454.1081124932396, max block_count: 2452.0, count: 55470"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 26,
            "unit": "median segment_count",
            "extra": "avg segment_count: 25.936920858121507, max segment_count: 49.0, count: 55470"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.59509054170598, max cpu: 4.7666335, count: 55470"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 39.3671875,
            "unit": "median mem",
            "extra": "avg mem: 38.952513605327205, max mem: 48.265625, count: 55470"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.787059674657152, max cpu: 9.302325, count: 55470"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 43.34765625,
            "unit": "median mem",
            "extra": "avg mem: 40.97753099930142, max mem: 49.81640625, count: 55470"
          }
        ]
      },
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
        "date": 1763941987913,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.761930650333238, max cpu: 9.599999, count: 54945"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 59.78515625,
            "unit": "median mem",
            "extra": "avg mem: 59.56936133576759, max mem: 83.1015625, count: 54945"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.629821708395253, max cpu: 9.580839, count: 54945"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 54.0703125,
            "unit": "median mem",
            "extra": "avg mem: 53.547654757029754, max mem: 76.0078125, count: 54945"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.758994226310964, max cpu: 9.638554, count: 54945"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 61.3984375,
            "unit": "median mem",
            "extra": "avg mem: 61.15853787025662, max mem: 84.68359375, count: 54945"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.488486551154295, max cpu: 4.6511626, count: 54945"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 61.16796875,
            "unit": "median mem",
            "extra": "avg mem: 60.38974706240331, max mem: 83.01171875, count: 54945"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.221902,
            "unit": "median cpu",
            "extra": "avg cpu: 7.506581544811747, max cpu: 23.692005, count: 109890"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 73.1953125,
            "unit": "median mem",
            "extra": "avg mem: 72.84022683905496, max mem: 103.50390625, count: 109890"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 3696,
            "unit": "median block_count",
            "extra": "avg block_count: 3679.0497952497954, max block_count: 6604.0, count: 54945"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 8,
            "unit": "median segment_count",
            "extra": "avg segment_count: 8.96960596960597, max segment_count: 28.0, count: 54945"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 6.322071970008511, max cpu: 14.414414, count: 54945"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 77.28125,
            "unit": "median mem",
            "extra": "avg mem: 77.2250195508008, max mem: 104.6796875, count: 54945"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.747219381028387, max cpu: 9.29332, count: 54945"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 57.81640625,
            "unit": "median mem",
            "extra": "avg mem: 57.32272897897898, max mem: 80.28125, count: 54945"
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
          "id": "f4cc34211b69c210cad6133141ec34b114d4e528",
          "message": "docs: fix more-like-this (MLT) JSON syntax example (#3335)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-10-14T19:24:46Z",
          "url": "https://github.com/paradedb/paradedb/commit/f4cc34211b69c210cad6133141ec34b114d4e528"
        },
        "date": 1763942022577,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.87452309195427, max cpu: 14.723927, count: 55247"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 155.40625,
            "unit": "median mem",
            "extra": "avg mem: 138.13498965441562, max mem: 155.78125, count: 55247"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.594159100589956, max cpu: 9.356726, count: 55247"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 28.99609375,
            "unit": "median mem",
            "extra": "avg mem: 28.789872852258945, max mem: 33.765625, count: 55247"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.874388012085734, max cpu: 14.723927, count: 55247"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 155.67578125,
            "unit": "median mem",
            "extra": "avg mem: 138.59961032952015, max mem: 155.67578125, count: 55247"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.6009268375490056, max cpu: 4.738401, count: 55247"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 157.1015625,
            "unit": "median mem",
            "extra": "avg mem: 139.72289629414266, max mem: 157.1015625, count: 55247"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.675699393950909, max cpu: 9.81595, count: 110494"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 155.71875,
            "unit": "median mem",
            "extra": "avg mem: 137.26223419088367, max mem: 157.21875, count: 110494"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 26871,
            "unit": "median block_count",
            "extra": "avg block_count: 26564.764005285353, max block_count: 51706.0, count: 55247"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 30,
            "unit": "median segment_count",
            "extra": "avg segment_count: 29.811826886527776, max segment_count: 58.0, count: 55247"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.56656649402207, max cpu: 9.29332, count: 55247"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 152.55078125,
            "unit": "median mem",
            "extra": "avg mem: 136.77011675973355, max mem: 160.42578125, count: 55247"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 3.7037768972413105, max cpu: 4.64666, count: 55247"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 149.97265625,
            "unit": "median mem",
            "extra": "avg mem: 129.91355632149708, max mem: 153.109375, count: 55247"
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
        "date": 1763942029244,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.756773621105735, max cpu: 9.458128, count: 55240"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 56.6484375,
            "unit": "median mem",
            "extra": "avg mem: 55.85794508904326, max mem: 75.2421875, count: 55240"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.622834960747761, max cpu: 9.221902, count: 55240"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 49.47265625,
            "unit": "median mem",
            "extra": "avg mem: 49.974101293786205, max mem: 70.203125, count: 55240"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.761348752137261, max cpu: 9.411765, count: 55240"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 56.9375,
            "unit": "median mem",
            "extra": "avg mem: 56.50328849112962, max mem: 76.0, count: 55240"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6153846,
            "unit": "median cpu",
            "extra": "avg cpu: 4.755314373843053, max cpu: 9.266409, count: 55240"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 56.6796875,
            "unit": "median mem",
            "extra": "avg mem: 56.70393573780322, max mem: 76.62890625, count: 55240"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.230769,
            "unit": "median cpu",
            "extra": "avg cpu: 8.181676952726471, max cpu: 27.799229, count: 110480"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 68.6875,
            "unit": "median mem",
            "extra": "avg mem: 68.53067477569469, max mem: 94.296875, count: 110480"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 3161,
            "unit": "median block_count",
            "extra": "avg block_count: 3149.1427226647356, max block_count: 5569.0, count: 55240"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 9,
            "unit": "median segment_count",
            "extra": "avg segment_count: 9.617197682838523, max segment_count: 26.0, count: 55240"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 6.643076724100276, max cpu: 18.58664, count: 55240"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 70.76171875,
            "unit": "median mem",
            "extra": "avg mem: 70.42644773092415, max mem: 98.03515625, count: 55240"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.619827,
            "unit": "median cpu",
            "extra": "avg cpu: 4.053266116833082, max cpu: 9.302325, count: 55240"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 54.8359375,
            "unit": "median mem",
            "extra": "avg mem: 53.922033753281134, max mem: 74.3203125, count: 55240"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mithun.cy@gmail.com",
            "name": "Mithun Chicklore Yogendra",
            "username": "mithuncy"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "a90bb936041bd4583034d9d1a538943756064dc4",
          "message": "fix: Skip flaky edge case numeric values in json_pushdown prop tests (#3620)\n\n## Summary\n- Skip edge case numeric values added in PR #2978 that cause\nintermittent prop test failures on main\n\n## Test plan\n- [ ] Verify prop tests pass without flaky failures",
          "timestamp": "2025-11-25T03:05:45+05:30",
          "tree_id": "95bf53a51121e3433d5f9df4eaa3649da9add90e",
          "url": "https://github.com/paradedb/paradedb/commit/a90bb936041bd4583034d9d1a538943756064dc4"
        },
        "date": 1764021147831,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 11.320755,
            "unit": "median cpu",
            "extra": "avg cpu: 11.647973079403217, max cpu: 33.005894, count: 55298"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 47.5625,
            "unit": "median mem",
            "extra": "avg mem: 47.354404629236136, max mem: 54.9765625, count: 55298"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.584284438880107, max cpu: 9.320388, count: 55298"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 27.2890625,
            "unit": "median mem",
            "extra": "avg mem: 27.10084840037162, max mem: 28.81640625, count: 55298"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 13.846154,
            "unit": "median cpu",
            "extra": "avg cpu: 12.02601107247161, max cpu: 33.20158, count: 55298"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 51.921875,
            "unit": "median mem",
            "extra": "avg mem: 50.4344720875981, max mem: 58.828125, count: 55298"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 5.014860977035393, max cpu: 13.967022, count: 55298"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 48.59765625,
            "unit": "median mem",
            "extra": "avg mem: 49.074702563270826, max mem: 60.0078125, count: 55298"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.655185615052189, max cpu: 9.580839, count: 110596"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 35.83984375,
            "unit": "median mem",
            "extra": "avg mem: 35.8486745960523, max mem: 45.171875, count: 110596"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 1384,
            "unit": "median block_count",
            "extra": "avg block_count: 1382.812705703642, max block_count: 2375.0, count: 55298"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 26,
            "unit": "median segment_count",
            "extra": "avg segment_count: 26.485207421606567, max segment_count: 50.0, count: 55298"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.627576289058713, max cpu: 4.7904196, count: 55298"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 37.3671875,
            "unit": "median mem",
            "extra": "avg mem: 37.796232529318424, max mem: 46.55078125, count: 55298"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.762824632602695, max cpu: 9.311348, count: 55298"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 40.875,
            "unit": "median mem",
            "extra": "avg mem: 41.5199392665648, max mem: 49.99609375, count: 55298"
          }
        ]
      }
    ],
    "pg_search bulk-updates.toml Performance - TPS": [
      {
        "commit": {
          "author": {
            "email": "stuhood@paradedb.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "committer": {
            "email": "stuhood@gmail.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "distinct": true,
          "id": "099c58ef752c337320d3e06a685cf80ba86e533a",
          "message": "chore: Prepare `0.19.9`. (#3604)\n\nPrepare `0.19.9`.",
          "timestamp": "2025-11-23T15:34:49-08:00",
          "tree_id": "108a1316f3541a472d93c2c75d3a050eb585ba61",
          "url": "https://github.com/paradedb/paradedb/commit/099c58ef752c337320d3e06a685cf80ba86e533a"
        },
        "date": 1763942600754,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 7.7744941632565965,
            "unit": "median tps",
            "extra": "avg tps: 6.663442551593652, max tps: 10.216570273575098, count: 57561"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.627144869879101,
            "unit": "median tps",
            "extra": "avg tps: 5.059930336195553, max tps: 6.247503724850888, count: 57561"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Moe",
            "username": "mdashti",
            "email": "mdashti@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "21ae4f92dc730f588c957d8cb5c893b916d95409",
          "message": "feat: supported window aggregate pushdown for all search operators (#3582)\n\n# Ticket(s) Closed\n\n- Closes #3566\n\n## What\n\nWindow aggregate queries with `|||`, `&&&`, `===`, and `###` operators\nnow properly push down to TopN scans, just like queries using the `@@@`\noperator.\n\n## Why\n\nPreviously, queries like `SELECT *, COUNT(*) OVER () FROM table WHERE\nfield ||| 'term' ORDER BY rating LIMIT 10` would fall back to\nPostgreSQL's standard WindowAgg execution path instead of using\noptimized TopN scan. This happened because the window function\nreplacement logic only checked for the `@@@` operator when deciding\nwhether to enable pushdown.\n\n## How\n\n- Added helper functions to get OIDs for all ParadeDB search operators:\n`match_disjunction_text_opoid()` for `|||`,\n`match_conjunction_text_opoid()` for `&&&`, `term_text_opoid()` for\n`===`, and `phrase_text_opoid()` for `###`\n- Updated `query_has_search_operator()` to check for all search\noperators, not just `@@@`\n\n## Tests\n\nAdded tests in `topn-agg-facet.sql` (Tests 1a-1d) verifying that window\naggregate queries with `|||`, `&&&`, `===`, and `###` operators properly\nuse TopNScanExecState execution.",
          "timestamp": "2025-11-21T21:49:27Z",
          "url": "https://github.com/paradedb/paradedb/commit/21ae4f92dc730f588c957d8cb5c893b916d95409"
        },
        "date": 1763942678494,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 7.773424411610556,
            "unit": "median tps",
            "extra": "avg tps: 6.647047810192526, max tps: 10.129055630411163, count: 57776"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.581768085809889,
            "unit": "median tps",
            "extra": "avg tps: 5.038465274852335, max tps: 6.240692151442822, count: 57776"
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
        "date": 1763942738277,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 6.555935051803763,
            "unit": "median tps",
            "extra": "avg tps: 5.624031659835579, max tps: 8.390601415795693, count: 57170"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.839083771446801,
            "unit": "median tps",
            "extra": "avg tps: 5.225632531156277, max tps: 6.6419316095954875, count: 57170"
          }
        ]
      },
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
        "date": 1763942765439,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 6.788202254420522,
            "unit": "median tps",
            "extra": "avg tps: 5.813194416370813, max tps: 8.733191774730924, count: 57452"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.777401739518307,
            "unit": "median tps",
            "extra": "avg tps: 5.180194883590186, max tps: 6.514421359546129, count: 57452"
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
          "id": "f4cc34211b69c210cad6133141ec34b114d4e528",
          "message": "docs: fix more-like-this (MLT) JSON syntax example (#3335)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-10-14T19:24:46Z",
          "url": "https://github.com/paradedb/paradedb/commit/f4cc34211b69c210cad6133141ec34b114d4e528"
        },
        "date": 1763942784581,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.002876331543701,
            "unit": "median tps",
            "extra": "avg tps: 6.80195240291271, max tps: 10.731169189036867, count: 57766"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.34495446803367,
            "unit": "median tps",
            "extra": "avg tps: 4.83736165928994, max tps: 5.896420368839919, count: 57766"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mithun.cy@gmail.com",
            "name": "Mithun Chicklore Yogendra",
            "username": "mithuncy"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "a90bb936041bd4583034d9d1a538943756064dc4",
          "message": "fix: Skip flaky edge case numeric values in json_pushdown prop tests (#3620)\n\n## Summary\n- Skip edge case numeric values added in PR #2978 that cause\nintermittent prop test failures on main\n\n## Test plan\n- [ ] Verify prop tests pass without flaky failures",
          "timestamp": "2025-11-25T03:05:45+05:30",
          "tree_id": "95bf53a51121e3433d5f9df4eaa3649da9add90e",
          "url": "https://github.com/paradedb/paradedb/commit/a90bb936041bd4583034d9d1a538943756064dc4"
        },
        "date": 1764021868101,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.015303584556252,
            "unit": "median tps",
            "extra": "avg tps: 6.830792649211418, max tps: 10.40389315114134, count: 57350"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.440383004820123,
            "unit": "median tps",
            "extra": "avg tps: 4.910683936816494, max tps: 6.070997648033656, count: 57350"
          }
        ]
      }
    ],
    "pg_search bulk-updates.toml Performance - Other Metrics": [
      {
        "commit": {
          "author": {
            "email": "stuhood@paradedb.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "committer": {
            "email": "stuhood@gmail.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "distinct": true,
          "id": "099c58ef752c337320d3e06a685cf80ba86e533a",
          "message": "chore: Prepare `0.19.9`. (#3604)\n\nPrepare `0.19.9`.",
          "timestamp": "2025-11-23T15:34:49-08:00",
          "tree_id": "108a1316f3541a472d93c2c75d3a050eb585ba61",
          "url": "https://github.com/paradedb/paradedb/commit/099c58ef752c337320d3e06a685cf80ba86e533a"
        },
        "date": 1763942603967,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.166023,
            "unit": "median cpu",
            "extra": "avg cpu: 20.555809355061797, max cpu: 42.687748, count: 57561"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 230.58203125,
            "unit": "median mem",
            "extra": "avg mem: 230.5696072698963, max mem: 232.1328125, count: 57561"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.323614,
            "unit": "median cpu",
            "extra": "avg cpu: 22.425432719808303, max cpu: 33.333336, count: 57561"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 168.20703125,
            "unit": "median mem",
            "extra": "avg mem: 168.1614101507531, max mem: 168.58203125, count: 57561"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 34524,
            "unit": "median block_count",
            "extra": "avg block_count: 33917.22570837894, max block_count: 36705.0, count: 57561"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 82,
            "unit": "median segment_count",
            "extra": "avg segment_count: 84.06394954917393, max segment_count: 133.0, count: 57561"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Moe",
            "username": "mdashti",
            "email": "mdashti@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "21ae4f92dc730f588c957d8cb5c893b916d95409",
          "message": "feat: supported window aggregate pushdown for all search operators (#3582)\n\n# Ticket(s) Closed\n\n- Closes #3566\n\n## What\n\nWindow aggregate queries with `|||`, `&&&`, `===`, and `###` operators\nnow properly push down to TopN scans, just like queries using the `@@@`\noperator.\n\n## Why\n\nPreviously, queries like `SELECT *, COUNT(*) OVER () FROM table WHERE\nfield ||| 'term' ORDER BY rating LIMIT 10` would fall back to\nPostgreSQL's standard WindowAgg execution path instead of using\noptimized TopN scan. This happened because the window function\nreplacement logic only checked for the `@@@` operator when deciding\nwhether to enable pushdown.\n\n## How\n\n- Added helper functions to get OIDs for all ParadeDB search operators:\n`match_disjunction_text_opoid()` for `|||`,\n`match_conjunction_text_opoid()` for `&&&`, `term_text_opoid()` for\n`===`, and `phrase_text_opoid()` for `###`\n- Updated `query_has_search_operator()` to check for all search\noperators, not just `@@@`\n\n## Tests\n\nAdded tests in `topn-agg-facet.sql` (Tests 1a-1d) verifying that window\naggregate queries with `|||`, `&&&`, `===`, and `###` operators properly\nuse TopNScanExecState execution.",
          "timestamp": "2025-11-21T21:49:27Z",
          "url": "https://github.com/paradedb/paradedb/commit/21ae4f92dc730f588c957d8cb5c893b916d95409"
        },
        "date": 1763942681870,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.210833,
            "unit": "median cpu",
            "extra": "avg cpu: 20.802978061160335, max cpu: 42.772278, count: 57776"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 229.9921875,
            "unit": "median mem",
            "extra": "avg mem: 229.93548480878738, max mem: 231.34765625, count: 57776"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.323614,
            "unit": "median cpu",
            "extra": "avg cpu: 22.480638469590303, max cpu: 33.267326, count: 57776"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 167.76171875,
            "unit": "median mem",
            "extra": "avg mem: 167.4524166473752, max mem: 168.13671875, count: 57776"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 34881,
            "unit": "median block_count",
            "extra": "avg block_count: 33927.377717391304, max block_count: 36677.0, count: 57776"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 81,
            "unit": "median segment_count",
            "extra": "avg segment_count: 83.66934367211299, max segment_count: 134.0, count: 57776"
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
        "date": 1763942742090,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.188406,
            "unit": "median cpu",
            "extra": "avg cpu: 22.53875761107822, max cpu: 51.06383, count: 57170"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 235.3515625,
            "unit": "median mem",
            "extra": "avg mem: 234.4864671019547, max mem: 241.76953125, count: 57170"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.188406,
            "unit": "median cpu",
            "extra": "avg cpu: 22.099043385790676, max cpu: 33.23442, count: 57170"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 160.09765625,
            "unit": "median mem",
            "extra": "avg mem: 160.16084717181215, max mem: 162.04296875, count: 57170"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 22256,
            "unit": "median block_count",
            "extra": "avg block_count: 20596.77036907469, max block_count: 23231.0, count: 57170"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 66,
            "unit": "median segment_count",
            "extra": "avg segment_count: 68.1328668882281, max segment_count: 96.0, count: 57170"
          }
        ]
      },
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
        "date": 1763942768921,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.233301,
            "unit": "median cpu",
            "extra": "avg cpu: 21.369117626791468, max cpu: 43.460762, count: 57452"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 227.1796875,
            "unit": "median mem",
            "extra": "avg mem: 226.51564308574984, max mem: 231.02734375, count: 57452"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.30097,
            "unit": "median cpu",
            "extra": "avg cpu: 22.185532962922256, max cpu: 33.20158, count: 57452"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 160.23046875,
            "unit": "median mem",
            "extra": "avg mem: 159.99562216067326, max mem: 163.75, count: 57452"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 22248,
            "unit": "median block_count",
            "extra": "avg block_count: 20611.95695537144, max block_count: 23473.0, count: 57452"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 67,
            "unit": "median segment_count",
            "extra": "avg segment_count: 68.6907679454153, max segment_count: 97.0, count: 57452"
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
          "id": "f4cc34211b69c210cad6133141ec34b114d4e528",
          "message": "docs: fix more-like-this (MLT) JSON syntax example (#3335)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-10-14T19:24:46Z",
          "url": "https://github.com/paradedb/paradedb/commit/f4cc34211b69c210cad6133141ec34b114d4e528"
        },
        "date": 1763942788253,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.934912,
            "unit": "median cpu",
            "extra": "avg cpu: 19.640873315586106, max cpu: 42.72997, count: 57766"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 227.921875,
            "unit": "median mem",
            "extra": "avg mem: 227.43168599933352, max mem: 229.50390625, count: 57766"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.27837,
            "unit": "median cpu",
            "extra": "avg cpu: 22.553256402624143, max cpu: 33.136093, count: 57766"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 162.59765625,
            "unit": "median mem",
            "extra": "avg mem: 162.12713895013937, max mem: 167.0390625, count: 57766"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 24330,
            "unit": "median block_count",
            "extra": "avg block_count: 23158.445002250457, max block_count: 25777.0, count: 57766"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 70,
            "unit": "median segment_count",
            "extra": "avg segment_count: 71.83064432365059, max segment_count: 105.0, count: 57766"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mithun.cy@gmail.com",
            "name": "Mithun Chicklore Yogendra",
            "username": "mithuncy"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "a90bb936041bd4583034d9d1a538943756064dc4",
          "message": "fix: Skip flaky edge case numeric values in json_pushdown prop tests (#3620)\n\n## Summary\n- Skip edge case numeric values added in PR #2978 that cause\nintermittent prop test failures on main\n\n## Test plan\n- [ ] Verify prop tests pass without flaky failures",
          "timestamp": "2025-11-25T03:05:45+05:30",
          "tree_id": "95bf53a51121e3433d5f9df4eaa3649da9add90e",
          "url": "https://github.com/paradedb/paradedb/commit/a90bb936041bd4583034d9d1a538943756064dc4"
        },
        "date": 1764021871417,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.121387,
            "unit": "median cpu",
            "extra": "avg cpu: 20.34631991187639, max cpu: 42.772278, count: 57350"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 235.16015625,
            "unit": "median mem",
            "extra": "avg mem: 234.21493345411943, max mem: 236.29296875, count: 57350"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.323614,
            "unit": "median cpu",
            "extra": "avg cpu: 22.388460078461065, max cpu: 33.366436, count: 57350"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 166.796875,
            "unit": "median mem",
            "extra": "avg mem: 166.76049238502617, max mem: 167.765625, count: 57350"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 35125,
            "unit": "median block_count",
            "extra": "avg block_count: 34152.36840453357, max block_count: 37115.0, count: 57350"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 83,
            "unit": "median segment_count",
            "extra": "avg segment_count: 84.91471665213601, max segment_count: 134.0, count: 57350"
          }
        ]
      }
    ],
    "pg_search wide-table.toml Performance - TPS": [
      {
        "commit": {
          "author": {
            "email": "stuhood@paradedb.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "committer": {
            "email": "stuhood@gmail.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "distinct": true,
          "id": "099c58ef752c337320d3e06a685cf80ba86e533a",
          "message": "chore: Prepare `0.19.9`. (#3604)\n\nPrepare `0.19.9`.",
          "timestamp": "2025-11-23T15:34:49-08:00",
          "tree_id": "108a1316f3541a472d93c2c75d3a050eb585ba61",
          "url": "https://github.com/paradedb/paradedb/commit/099c58ef752c337320d3e06a685cf80ba86e533a"
        },
        "date": 1763943333224,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 1169.7491869585353,
            "unit": "median tps",
            "extra": "avg tps: 1169.9433563036434, max tps: 1218.0976111116072, count: 56329"
          },
          {
            "name": "Single Insert - Primary - tps",
            "value": 1329.589556683933,
            "unit": "median tps",
            "extra": "avg tps: 1322.4368490222303, max tps: 1348.3555959472114, count: 56329"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 1978.3625629106186,
            "unit": "median tps",
            "extra": "avg tps: 1948.9199987460831, max tps: 2132.197473224693, count: 56329"
          },
          {
            "name": "Top N - Primary - tps",
            "value": 5.46320954536466,
            "unit": "median tps",
            "extra": "avg tps: 5.47455586851817, max tps: 6.528366657304772, count: 56329"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Moe",
            "username": "mdashti",
            "email": "mdashti@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "21ae4f92dc730f588c957d8cb5c893b916d95409",
          "message": "feat: supported window aggregate pushdown for all search operators (#3582)\n\n# Ticket(s) Closed\n\n- Closes #3566\n\n## What\n\nWindow aggregate queries with `|||`, `&&&`, `===`, and `###` operators\nnow properly push down to TopN scans, just like queries using the `@@@`\noperator.\n\n## Why\n\nPreviously, queries like `SELECT *, COUNT(*) OVER () FROM table WHERE\nfield ||| 'term' ORDER BY rating LIMIT 10` would fall back to\nPostgreSQL's standard WindowAgg execution path instead of using\noptimized TopN scan. This happened because the window function\nreplacement logic only checked for the `@@@` operator when deciding\nwhether to enable pushdown.\n\n## How\n\n- Added helper functions to get OIDs for all ParadeDB search operators:\n`match_disjunction_text_opoid()` for `|||`,\n`match_conjunction_text_opoid()` for `&&&`, `term_text_opoid()` for\n`===`, and `phrase_text_opoid()` for `###`\n- Updated `query_has_search_operator()` to check for all search\noperators, not just `@@@`\n\n## Tests\n\nAdded tests in `topn-agg-facet.sql` (Tests 1a-1d) verifying that window\naggregate queries with `|||`, `&&&`, `===`, and `###` operators properly\nuse TopNScanExecState execution.",
          "timestamp": "2025-11-21T21:49:27Z",
          "url": "https://github.com/paradedb/paradedb/commit/21ae4f92dc730f588c957d8cb5c893b916d95409"
        },
        "date": 1763943411243,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 1168.575776369992,
            "unit": "median tps",
            "extra": "avg tps: 1167.775034039699, max tps: 1228.466671039507, count: 55631"
          },
          {
            "name": "Single Insert - Primary - tps",
            "value": 1312.6370057481695,
            "unit": "median tps",
            "extra": "avg tps: 1305.7170090588936, max tps: 1327.0519916277499, count: 55631"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 2016.2804100444746,
            "unit": "median tps",
            "extra": "avg tps: 1978.927985969312, max tps: 2186.943141635492, count: 55631"
          },
          {
            "name": "Top N - Primary - tps",
            "value": 5.571159523427133,
            "unit": "median tps",
            "extra": "avg tps: 5.5628321764950925, max tps: 6.522935479342578, count: 55631"
          }
        ]
      },
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
        "date": 1763943453686,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 27.096076532434306,
            "unit": "median tps",
            "extra": "avg tps: 27.034471461118713, max tps: 27.337618658192152, count: 57866"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 134.31024208651993,
            "unit": "median tps",
            "extra": "avg tps: 133.9290090108341, max tps: 135.61055858585544, count: 57866"
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
        "date": 1763943472919,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 27.58222696153963,
            "unit": "median tps",
            "extra": "avg tps: 27.407999359443306, max tps: 27.821570425635244, count: 57698"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 120.87213571144387,
            "unit": "median tps",
            "extra": "avg tps: 120.1786894193387, max tps: 122.31299623913378, count: 57698"
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
          "id": "f4cc34211b69c210cad6133141ec34b114d4e528",
          "message": "docs: fix more-like-this (MLT) JSON syntax example (#3335)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-10-14T19:24:46Z",
          "url": "https://github.com/paradedb/paradedb/commit/f4cc34211b69c210cad6133141ec34b114d4e528"
        },
        "date": 1763943596093,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 1185.6149389405703,
            "unit": "median tps",
            "extra": "avg tps: 1181.1410482674623, max tps: 1243.380857531004, count: 56189"
          },
          {
            "name": "Single Insert - Primary - tps",
            "value": 1309.3585727992267,
            "unit": "median tps",
            "extra": "avg tps: 1277.1618263436917, max tps: 1331.017189969902, count: 56189"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 1857.3003406871023,
            "unit": "median tps",
            "extra": "avg tps: 1785.1648505437213, max tps: 2068.7892269916733, count: 56189"
          },
          {
            "name": "Top N - Primary - tps",
            "value": 5.405021193339859,
            "unit": "median tps",
            "extra": "avg tps: 5.424420227777654, max tps: 6.594529634556623, count: 56189"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mithun.cy@gmail.com",
            "name": "Mithun Chicklore Yogendra",
            "username": "mithuncy"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "a90bb936041bd4583034d9d1a538943756064dc4",
          "message": "fix: Skip flaky edge case numeric values in json_pushdown prop tests (#3620)\n\n## Summary\n- Skip edge case numeric values added in PR #2978 that cause\nintermittent prop test failures on main\n\n## Test plan\n- [ ] Verify prop tests pass without flaky failures",
          "timestamp": "2025-11-25T03:05:45+05:30",
          "tree_id": "95bf53a51121e3433d5f9df4eaa3649da9add90e",
          "url": "https://github.com/paradedb/paradedb/commit/a90bb936041bd4583034d9d1a538943756064dc4"
        },
        "date": 1764022606362,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 1158.580552990854,
            "unit": "median tps",
            "extra": "avg tps: 1158.111622450188, max tps: 1206.223569050177, count: 56133"
          },
          {
            "name": "Single Insert - Primary - tps",
            "value": 1345.1434267986813,
            "unit": "median tps",
            "extra": "avg tps: 1333.8072578958443, max tps: 1354.61797016451, count: 56133"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 1939.3684615256914,
            "unit": "median tps",
            "extra": "avg tps: 1907.8709283793978, max tps: 2080.097865845215, count: 56133"
          },
          {
            "name": "Top N - Primary - tps",
            "value": 5.540596829545507,
            "unit": "median tps",
            "extra": "avg tps: 5.56667233721975, max tps: 6.661181782974747, count: 56133"
          }
        ]
      }
    ],
    "pg_search wide-table.toml Performance - Other Metrics": [
      {
        "commit": {
          "author": {
            "email": "stuhood@paradedb.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "committer": {
            "email": "stuhood@gmail.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "distinct": true,
          "id": "099c58ef752c337320d3e06a685cf80ba86e533a",
          "message": "chore: Prepare `0.19.9`. (#3604)\n\nPrepare `0.19.9`.",
          "timestamp": "2025-11-23T15:34:49-08:00",
          "tree_id": "108a1316f3541a472d93c2c75d3a050eb585ba61",
          "url": "https://github.com/paradedb/paradedb/commit/099c58ef752c337320d3e06a685cf80ba86e533a"
        },
        "date": 1763943336405,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Background Merger - Primary - background_merging",
            "value": 0,
            "unit": "median background_merging",
            "extra": "avg background_merging: 0.07204104457739352, max background_merging: 2.0, count: 56329"
          },
          {
            "name": "Background Merger - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.764346784551726, max cpu: 9.628887, count: 56329"
          },
          {
            "name": "Background Merger - Primary - mem",
            "value": 23.45703125,
            "unit": "median mem",
            "extra": "avg mem: 23.507984203296704, max mem: 25.93359375, count: 56329"
          },
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.964735824683363, max cpu: 11.320755, count: 56329"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 153.5390625,
            "unit": "median mem",
            "extra": "avg mem: 152.33935311961866, max mem: 153.5390625, count: 56329"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 51412,
            "unit": "median block_count",
            "extra": "avg block_count: 51278.18086598378, max block_count: 51412.0, count: 56329"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 46,
            "unit": "median segment_count",
            "extra": "avg segment_count: 43.60120009231479, max segment_count: 56.0, count: 56329"
          },
          {
            "name": "Single Insert - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.623074064766178, max cpu: 9.476802, count: 56329"
          },
          {
            "name": "Single Insert - Primary - mem",
            "value": 121.65625,
            "unit": "median mem",
            "extra": "avg mem: 110.83304787775835, max mem: 134.74609375, count: 56329"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.696091529996364, max cpu: 9.638554, count: 56329"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 155.63671875,
            "unit": "median mem",
            "extra": "avg mem: 152.18541792415985, max mem: 156.01171875, count: 56329"
          },
          {
            "name": "Top N - Primary - cpu",
            "value": 23.30097,
            "unit": "median cpu",
            "extra": "avg cpu: 23.814502385660177, max cpu: 33.103447, count: 56329"
          },
          {
            "name": "Top N - Primary - mem",
            "value": 156.38671875,
            "unit": "median mem",
            "extra": "avg mem: 174.4673708952094, max mem: 216.26171875, count: 56329"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Moe",
            "username": "mdashti",
            "email": "mdashti@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "21ae4f92dc730f588c957d8cb5c893b916d95409",
          "message": "feat: supported window aggregate pushdown for all search operators (#3582)\n\n# Ticket(s) Closed\n\n- Closes #3566\n\n## What\n\nWindow aggregate queries with `|||`, `&&&`, `===`, and `###` operators\nnow properly push down to TopN scans, just like queries using the `@@@`\noperator.\n\n## Why\n\nPreviously, queries like `SELECT *, COUNT(*) OVER () FROM table WHERE\nfield ||| 'term' ORDER BY rating LIMIT 10` would fall back to\nPostgreSQL's standard WindowAgg execution path instead of using\noptimized TopN scan. This happened because the window function\nreplacement logic only checked for the `@@@` operator when deciding\nwhether to enable pushdown.\n\n## How\n\n- Added helper functions to get OIDs for all ParadeDB search operators:\n`match_disjunction_text_opoid()` for `|||`,\n`match_conjunction_text_opoid()` for `&&&`, `term_text_opoid()` for\n`===`, and `phrase_text_opoid()` for `###`\n- Updated `query_has_search_operator()` to check for all search\noperators, not just `@@@`\n\n## Tests\n\nAdded tests in `topn-agg-facet.sql` (Tests 1a-1d) verifying that window\naggregate queries with `|||`, `&&&`, `===`, and `###` operators properly\nuse TopNScanExecState execution.",
          "timestamp": "2025-11-21T21:49:27Z",
          "url": "https://github.com/paradedb/paradedb/commit/21ae4f92dc730f588c957d8cb5c893b916d95409"
        },
        "date": 1763943414761,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Background Merger - Primary - background_merging",
            "value": 0,
            "unit": "median background_merging",
            "extra": "avg background_merging: 0.07220794161528644, max background_merging: 2.0, count: 55631"
          },
          {
            "name": "Background Merger - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 4.811873574652832, max cpu: 9.687184, count: 55631"
          },
          {
            "name": "Background Merger - Primary - mem",
            "value": 17.95703125,
            "unit": "median mem",
            "extra": "avg mem: 17.998594322971904, max mem: 20.703125, count: 55631"
          },
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 4.678363,
            "unit": "median cpu",
            "extra": "avg cpu: 4.980828052795571, max cpu: 9.696969, count: 55631"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 155.19140625,
            "unit": "median mem",
            "extra": "avg mem: 153.86234286807266, max mem: 155.56640625, count: 55631"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 51511,
            "unit": "median block_count",
            "extra": "avg block_count: 51383.21906850497, max block_count: 51511.0, count: 55631"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 46,
            "unit": "median segment_count",
            "extra": "avg segment_count: 43.87165429346947, max segment_count: 56.0, count: 55631"
          },
          {
            "name": "Single Insert - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 4.615781780613182, max cpu: 9.523809, count: 55631"
          },
          {
            "name": "Single Insert - Primary - mem",
            "value": 119.1640625,
            "unit": "median mem",
            "extra": "avg mem: 107.85171381794773, max mem: 132.6875, count: 55631"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 4.7881110146322, max cpu: 9.628887, count: 55631"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 155.51171875,
            "unit": "median mem",
            "extra": "avg mem: 151.95916458730295, max mem: 155.51171875, count: 55631"
          },
          {
            "name": "Top N - Primary - cpu",
            "value": 23.552504,
            "unit": "median cpu",
            "extra": "avg cpu: 23.94906118363031, max cpu: 33.432835, count: 55631"
          },
          {
            "name": "Top N - Primary - mem",
            "value": 157.12890625,
            "unit": "median mem",
            "extra": "avg mem: 175.96192721066043, max mem: 217.7578125, count: 55631"
          }
        ]
      },
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
        "date": 1763943457347,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.695229,
            "unit": "median cpu",
            "extra": "avg cpu: 20.736837308839327, max cpu: 55.54484, count: 57866"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 169.84765625,
            "unit": "median mem",
            "extra": "avg mem: 167.94410409124097, max mem: 171.55078125, count: 57866"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 17724,
            "unit": "median block_count",
            "extra": "avg block_count: 16401.347025887397, max block_count: 21531.0, count: 57866"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 40,
            "unit": "median segment_count",
            "extra": "avg segment_count: 41.751926865516886, max segment_count: 112.0, count: 57866"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 13.832853,
            "unit": "median cpu",
            "extra": "avg cpu: 11.966688747874391, max cpu: 37.10145, count: 57866"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 162.23828125,
            "unit": "median mem",
            "extra": "avg mem: 152.49950194738707, max mem: 168.66015625, count: 57866"
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
        "date": 1763943476719,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.731707,
            "unit": "median cpu",
            "extra": "avg cpu: 20.676656680978006, max cpu: 114.94253, count: 57698"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 173.6484375,
            "unit": "median mem",
            "extra": "avg mem: 171.95806072134215, max mem: 178.41796875, count: 57698"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 17777,
            "unit": "median block_count",
            "extra": "avg block_count: 16385.693264931193, max block_count: 21227.0, count: 57698"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 39,
            "unit": "median segment_count",
            "extra": "avg segment_count: 41.40445076085826, max segment_count: 111.0, count: 57698"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 18.658894,
            "unit": "median cpu",
            "extra": "avg cpu: 19.712748767997123, max cpu: 161.38329, count: 57698"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 166.60546875,
            "unit": "median mem",
            "extra": "avg mem: 157.6394060994315, max mem: 175.41015625, count: 57698"
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
          "id": "f4cc34211b69c210cad6133141ec34b114d4e528",
          "message": "docs: fix more-like-this (MLT) JSON syntax example (#3335)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-10-14T19:24:46Z",
          "url": "https://github.com/paradedb/paradedb/commit/f4cc34211b69c210cad6133141ec34b114d4e528"
        },
        "date": 1763943599470,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Background Merger - Primary - background_merging",
            "value": 0,
            "unit": "median background_merging",
            "extra": "avg background_merging: 0.06116855612308459, max background_merging: 1.0, count: 56189"
          },
          {
            "name": "Background Merger - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.68406865929184, max cpu: 9.60961, count: 56189"
          },
          {
            "name": "Background Merger - Primary - mem",
            "value": 19.6015625,
            "unit": "median mem",
            "extra": "avg mem: 19.599469396812545, max mem: 19.6015625, count: 56189"
          },
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 5.002057400122718, max cpu: 27.988338, count: 56189"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 170.3671875,
            "unit": "median mem",
            "extra": "avg mem: 168.83447549787326, max mem: 170.3671875, count: 56189"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 51236,
            "unit": "median block_count",
            "extra": "avg block_count: 51086.64576696506, max block_count: 51236.0, count: 56189"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 46,
            "unit": "median segment_count",
            "extra": "avg segment_count: 44.53857516595775, max segment_count: 54.0, count: 56189"
          },
          {
            "name": "Single Insert - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.6987392839565665, max cpu: 23.369036, count: 56189"
          },
          {
            "name": "Single Insert - Primary - mem",
            "value": 140.16015625,
            "unit": "median mem",
            "extra": "avg mem: 130.27413109939224, max mem: 152.04296875, count: 56189"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.850444434096453, max cpu: 28.042841, count: 56189"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 170.64453125,
            "unit": "median mem",
            "extra": "avg mem: 167.49263346974053, max mem: 170.64453125, count: 56189"
          },
          {
            "name": "Top N - Primary - cpu",
            "value": 23.369036,
            "unit": "median cpu",
            "extra": "avg cpu: 23.977137901186893, max cpu: 33.870968, count: 56189"
          },
          {
            "name": "Top N - Primary - mem",
            "value": 157.0078125,
            "unit": "median mem",
            "extra": "avg mem: 174.71135997203635, max mem: 216.4765625, count: 56189"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mithun.cy@gmail.com",
            "name": "Mithun Chicklore Yogendra",
            "username": "mithuncy"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "a90bb936041bd4583034d9d1a538943756064dc4",
          "message": "fix: Skip flaky edge case numeric values in json_pushdown prop tests (#3620)\n\n## Summary\n- Skip edge case numeric values added in PR #2978 that cause\nintermittent prop test failures on main\n\n## Test plan\n- [ ] Verify prop tests pass without flaky failures",
          "timestamp": "2025-11-25T03:05:45+05:30",
          "tree_id": "95bf53a51121e3433d5f9df4eaa3649da9add90e",
          "url": "https://github.com/paradedb/paradedb/commit/a90bb936041bd4583034d9d1a538943756064dc4"
        },
        "date": 1764022609618,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Background Merger - Primary - background_merging",
            "value": 0,
            "unit": "median background_merging",
            "extra": "avg background_merging: 0.0731833324425917, max background_merging: 2.0, count: 56133"
          },
          {
            "name": "Background Merger - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.777261087725683, max cpu: 9.667674, count: 56133"
          },
          {
            "name": "Background Merger - Primary - mem",
            "value": 22.81640625,
            "unit": "median mem",
            "extra": "avg mem: 22.7521208697424, max mem: 24.58203125, count: 56133"
          },
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.997134165991885, max cpu: 13.953489, count: 56133"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 153.75,
            "unit": "median mem",
            "extra": "avg mem: 152.48126422403044, max mem: 154.52734375, count: 56133"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 50934,
            "unit": "median block_count",
            "extra": "avg block_count: 50793.49915379545, max block_count: 50934.0, count: 56133"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 46,
            "unit": "median segment_count",
            "extra": "avg segment_count: 44.27757290720254, max segment_count: 61.0, count: 56133"
          },
          {
            "name": "Single Insert - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.672322321383341, max cpu: 9.356726, count: 56133"
          },
          {
            "name": "Single Insert - Primary - mem",
            "value": 121.82421875,
            "unit": "median mem",
            "extra": "avg mem: 110.81845105875777, max mem: 133.34765625, count: 56133"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.752347828005903, max cpu: 9.619239, count: 56133"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 155.01953125,
            "unit": "median mem",
            "extra": "avg mem: 151.340243052772, max mem: 155.01953125, count: 56133"
          },
          {
            "name": "Top N - Primary - cpu",
            "value": 23.346306,
            "unit": "median cpu",
            "extra": "avg cpu: 23.88538601755732, max cpu: 33.103447, count: 56133"
          },
          {
            "name": "Top N - Primary - mem",
            "value": 156.96484375,
            "unit": "median mem",
            "extra": "avg mem: 175.36221068990167, max mem: 216.79296875, count: 56133"
          }
        ]
      }
    ],
    "pg_search background-merge.toml Performance - TPS": [
      {
        "commit": {
          "author": {
            "email": "stuhood@paradedb.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "committer": {
            "email": "stuhood@gmail.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "distinct": true,
          "id": "099c58ef752c337320d3e06a685cf80ba86e533a",
          "message": "chore: Prepare `0.19.9`. (#3604)\n\nPrepare `0.19.9`.",
          "timestamp": "2025-11-23T15:34:49-08:00",
          "tree_id": "108a1316f3541a472d93c2c75d3a050eb585ba61",
          "url": "https://github.com/paradedb/paradedb/commit/099c58ef752c337320d3e06a685cf80ba86e533a"
        },
        "date": 1763944023402,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 29.57508172163478,
            "unit": "median tps",
            "extra": "avg tps: 29.60324869424923, max tps: 35.24612254586184, count: 55380"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 234.6224461702493,
            "unit": "median tps",
            "extra": "avg tps: 256.45995210594816, max tps: 2726.5189230637707, count: 55380"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1924.9851226250075,
            "unit": "median tps",
            "extra": "avg tps: 1904.691252093708, max tps: 2373.333075026385, count: 55380"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 163.25454980424445,
            "unit": "median tps",
            "extra": "avg tps: 194.9292157112557, max tps: 1729.186264604488, count: 110760"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 14.590489539076804,
            "unit": "median tps",
            "extra": "avg tps: 14.795960203938376, max tps: 19.4186674310372, count: 55380"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Moe",
            "username": "mdashti",
            "email": "mdashti@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "21ae4f92dc730f588c957d8cb5c893b916d95409",
          "message": "feat: supported window aggregate pushdown for all search operators (#3582)\n\n# Ticket(s) Closed\n\n- Closes #3566\n\n## What\n\nWindow aggregate queries with `|||`, `&&&`, `===`, and `###` operators\nnow properly push down to TopN scans, just like queries using the `@@@`\noperator.\n\n## Why\n\nPreviously, queries like `SELECT *, COUNT(*) OVER () FROM table WHERE\nfield ||| 'term' ORDER BY rating LIMIT 10` would fall back to\nPostgreSQL's standard WindowAgg execution path instead of using\noptimized TopN scan. This happened because the window function\nreplacement logic only checked for the `@@@` operator when deciding\nwhether to enable pushdown.\n\n## How\n\n- Added helper functions to get OIDs for all ParadeDB search operators:\n`match_disjunction_text_opoid()` for `|||`,\n`match_conjunction_text_opoid()` for `&&&`, `term_text_opoid()` for\n`===`, and `phrase_text_opoid()` for `###`\n- Updated `query_has_search_operator()` to check for all search\noperators, not just `@@@`\n\n## Tests\n\nAdded tests in `topn-agg-facet.sql` (Tests 1a-1d) verifying that window\naggregate queries with `|||`, `&&&`, `===`, and `###` operators properly\nuse TopNScanExecState execution.",
          "timestamp": "2025-11-21T21:49:27Z",
          "url": "https://github.com/paradedb/paradedb/commit/21ae4f92dc730f588c957d8cb5c893b916d95409"
        },
        "date": 1763944101916,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 29.271351233021786,
            "unit": "median tps",
            "extra": "avg tps: 29.351243078967535, max tps: 36.07078089980251, count: 55458"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 239.8690382717458,
            "unit": "median tps",
            "extra": "avg tps: 263.774947529487, max tps: 2895.392281770891, count: 55458"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1997.3597888195973,
            "unit": "median tps",
            "extra": "avg tps: 1975.5643564423453, max tps: 2337.222897183865, count: 55458"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 176.77683605836427,
            "unit": "median tps",
            "extra": "avg tps: 206.69453382343724, max tps: 1724.780712031133, count: 110916"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 14.317411089483645,
            "unit": "median tps",
            "extra": "avg tps: 14.395735088253305, max tps: 20.86564421810599, count: 55458"
          }
        ]
      },
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
        "date": 1763944178053,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Delete value - Primary - tps",
            "value": 124.64458632208047,
            "unit": "median tps",
            "extra": "avg tps: 137.46983729367344, max tps: 664.0588387966204, count: 55439"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 141.9864342231275,
            "unit": "median tps",
            "extra": "avg tps: 139.2336890486672, max tps: 149.94167214098428, count: 55439"
          },
          {
            "name": "Monitor Segment Count - Primary - tps",
            "value": 1933.883062926228,
            "unit": "median tps",
            "extra": "avg tps: 1923.595057663097, max tps: 1969.2859546453924, count: 55439"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 19.603245117857423,
            "unit": "median tps",
            "extra": "avg tps: 18.43361460502956, max tps: 68.75371873408908, count: 166317"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 0.30658628645163494,
            "unit": "median tps",
            "extra": "avg tps: 0.6404466149178135, max tps: 4.858839883523696, count: 55439"
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
        "date": 1763944188639,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 36.11997011927111,
            "unit": "median tps",
            "extra": "avg tps: 36.222857742429845, max tps: 36.98827794306843, count: 55551"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 254.13082526895764,
            "unit": "median tps",
            "extra": "avg tps: 299.0439381396041, max tps: 2848.0909539521494, count: 55551"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 120.79601468504926,
            "unit": "median tps",
            "extra": "avg tps: 121.20281136608257, max tps: 124.12753719019457, count: 55551"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 72.60334920652082,
            "unit": "median tps",
            "extra": "avg tps: 63.8725893331841, max tps: 104.78457975687311, count: 111102"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 15.630622715821731,
            "unit": "median tps",
            "extra": "avg tps: 15.707019062744552, max tps: 18.47622063290183, count: 55551"
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
          "id": "f4cc34211b69c210cad6133141ec34b114d4e528",
          "message": "docs: fix more-like-this (MLT) JSON syntax example (#3335)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-10-14T19:24:46Z",
          "url": "https://github.com/paradedb/paradedb/commit/f4cc34211b69c210cad6133141ec34b114d4e528"
        },
        "date": 1763944289855,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 36.536449963565765,
            "unit": "median tps",
            "extra": "avg tps: 36.91153647179072, max tps: 40.99900025372846, count: 55488"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 250.36014966442104,
            "unit": "median tps",
            "extra": "avg tps: 287.0277253318459, max tps: 2951.4017260869227, count: 55488"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1011.9539307273271,
            "unit": "median tps",
            "extra": "avg tps: 1013.2133306528428, max tps: 1039.3712957286095, count: 55488"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 117.1982966183818,
            "unit": "median tps",
            "extra": "avg tps: 156.85430902363714, max tps: 865.1507751126113, count: 110976"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 18.86755266365807,
            "unit": "median tps",
            "extra": "avg tps: 19.130543626588302, max tps: 22.389859328208168, count: 55488"
          }
        ]
      }
    ],
    "pg_search background-merge.toml Performance - Other Metrics": [
      {
        "commit": {
          "author": {
            "email": "stuhood@paradedb.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "committer": {
            "email": "stuhood@gmail.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "distinct": true,
          "id": "099c58ef752c337320d3e06a685cf80ba86e533a",
          "message": "chore: Prepare `0.19.9`. (#3604)\n\nPrepare `0.19.9`.",
          "timestamp": "2025-11-23T15:34:49-08:00",
          "tree_id": "108a1316f3541a472d93c2c75d3a050eb585ba61",
          "url": "https://github.com/paradedb/paradedb/commit/099c58ef752c337320d3e06a685cf80ba86e533a"
        },
        "date": 1763944026653,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.568666,
            "unit": "median cpu",
            "extra": "avg cpu: 20.125849135147494, max cpu: 46.242775, count: 55380"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 162.1328125,
            "unit": "median mem",
            "extra": "avg mem: 150.55691789962532, max mem: 165.515625, count: 55380"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 7.702492472770086, max cpu: 28.015566, count: 55380"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 113.65625,
            "unit": "median mem",
            "extra": "avg mem: 112.65651168630372, max mem: 113.65625, count: 55380"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.750613522996114, max cpu: 9.476802, count: 55380"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 118.296875,
            "unit": "median mem",
            "extra": "avg mem: 106.19090869334597, max mem: 144.4609375, count: 55380"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 14704,
            "unit": "median block_count",
            "extra": "avg block_count: 14721.90240158902, max block_count: 25850.0, count: 55380"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 3.391431370873087, max cpu: 4.6647234, count: 55380"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 92.59375,
            "unit": "median mem",
            "extra": "avg mem: 85.6181649089247, max mem: 125.98046875, count: 55380"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 27,
            "unit": "median segment_count",
            "extra": "avg segment_count: 26.671849042975804, max segment_count: 44.0, count: 55380"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.221902,
            "unit": "median cpu",
            "extra": "avg cpu: 8.75772407272096, max cpu: 27.934044, count: 110760"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 149.45703125,
            "unit": "median mem",
            "extra": "avg mem: 129.8154641440389, max mem: 151.03515625, count: 110760"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.859479,
            "unit": "median cpu",
            "extra": "avg cpu: 12.813972619965144, max cpu: 32.43243, count: 55380"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 162.75,
            "unit": "median mem",
            "extra": "avg mem: 160.42941495124595, max mem: 162.75, count: 55380"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Moe",
            "username": "mdashti",
            "email": "mdashti@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "21ae4f92dc730f588c957d8cb5c893b916d95409",
          "message": "feat: supported window aggregate pushdown for all search operators (#3582)\n\n# Ticket(s) Closed\n\n- Closes #3566\n\n## What\n\nWindow aggregate queries with `|||`, `&&&`, `===`, and `###` operators\nnow properly push down to TopN scans, just like queries using the `@@@`\noperator.\n\n## Why\n\nPreviously, queries like `SELECT *, COUNT(*) OVER () FROM table WHERE\nfield ||| 'term' ORDER BY rating LIMIT 10` would fall back to\nPostgreSQL's standard WindowAgg execution path instead of using\noptimized TopN scan. This happened because the window function\nreplacement logic only checked for the `@@@` operator when deciding\nwhether to enable pushdown.\n\n## How\n\n- Added helper functions to get OIDs for all ParadeDB search operators:\n`match_disjunction_text_opoid()` for `|||`,\n`match_conjunction_text_opoid()` for `&&&`, `term_text_opoid()` for\n`===`, and `phrase_text_opoid()` for `###`\n- Updated `query_has_search_operator()` to check for all search\noperators, not just `@@@`\n\n## Tests\n\nAdded tests in `topn-agg-facet.sql` (Tests 1a-1d) verifying that window\naggregate queries with `|||`, `&&&`, `===`, and `###` operators properly\nuse TopNScanExecState execution.",
          "timestamp": "2025-11-21T21:49:27Z",
          "url": "https://github.com/paradedb/paradedb/commit/21ae4f92dc730f588c957d8cb5c893b916d95409"
        },
        "date": 1763944105725,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.622696,
            "unit": "median cpu",
            "extra": "avg cpu: 20.426413220022056, max cpu: 47.38401, count: 55458"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 137.12890625,
            "unit": "median mem",
            "extra": "avg mem: 126.13777761718327, max mem: 164.90625, count: 55458"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 7.75104918226412, max cpu: 28.152493, count: 55458"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 114.1015625,
            "unit": "median mem",
            "extra": "avg mem: 112.96541167696725, max mem: 114.1015625, count: 55458"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.8091153393965165, max cpu: 14.035088, count: 55458"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 118.53125,
            "unit": "median mem",
            "extra": "avg mem: 106.5834217307467, max mem: 146.83984375, count: 55458"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 14829,
            "unit": "median block_count",
            "extra": "avg block_count: 14775.727487467993, max block_count: 26064.0, count: 55458"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 3.9788065905088414, max cpu: 4.7105007, count: 55458"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 94.63671875,
            "unit": "median mem",
            "extra": "avg mem: 86.4603742922572, max mem: 128.82421875, count: 55458"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 27,
            "unit": "median segment_count",
            "extra": "avg segment_count: 26.819503047351148, max segment_count: 45.0, count: 55458"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.221902,
            "unit": "median cpu",
            "extra": "avg cpu: 8.51376112937315, max cpu: 28.042841, count: 110916"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 151.4765625,
            "unit": "median mem",
            "extra": "avg mem: 131.37006798782411, max mem: 154.36328125, count: 110916"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.913043,
            "unit": "median cpu",
            "extra": "avg cpu: 13.450458601192517, max cpu: 28.346458, count: 55458"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 162.78515625,
            "unit": "median mem",
            "extra": "avg mem: 159.96433597553553, max mem: 162.95703125, count: 55458"
          }
        ]
      },
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
        "date": 1763944181794,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 9.466997218836083, max cpu: 32.621357, count: 55439"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 205.2265625,
            "unit": "median mem",
            "extra": "avg mem: 203.52989166583993, max mem: 205.2265625, count: 55439"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.275363,
            "unit": "median cpu",
            "extra": "avg cpu: 10.535916168217371, max cpu: 23.233301, count: 55439"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 157.84765625,
            "unit": "median mem",
            "extra": "avg mem: 154.60892666094716, max mem: 163.921875, count: 55439"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 37911,
            "unit": "median block_count",
            "extra": "avg block_count: 40053.08071934919, max block_count: 57334.0, count: 55439"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.619827,
            "unit": "median cpu",
            "extra": "avg cpu: 4.321388175373762, max cpu: 4.6376815, count: 55439"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 122.8125,
            "unit": "median mem",
            "extra": "avg mem: 108.81218158978787, max mem: 137.07421875, count: 55439"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 25,
            "unit": "median segment_count",
            "extra": "avg segment_count: 26.005645844982773, max segment_count: 64.0, count: 55439"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 23.054754,
            "unit": "median cpu",
            "extra": "avg cpu: 20.708235454662255, max cpu: 32.684826, count: 166317"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 214.40234375,
            "unit": "median mem",
            "extra": "avg mem: 239.90513444863575, max mem: 460.7578125, count: 166317"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.913043,
            "unit": "median cpu",
            "extra": "avg cpu: 14.84449286019769, max cpu: 32.495163, count: 55439"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 191.12890625,
            "unit": "median mem",
            "extra": "avg mem: 189.43813442477318, max mem: 221.53515625, count: 55439"
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
        "date": 1763944192553,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.532818,
            "unit": "median cpu",
            "extra": "avg cpu: 18.853715089976276, max cpu: 41.578438, count: 55551"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 156.23046875,
            "unit": "median mem",
            "extra": "avg mem: 142.51331279477418, max mem: 157.35546875, count: 55551"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 7.65644614094502, max cpu: 28.042841, count: 55551"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 149.5625,
            "unit": "median mem",
            "extra": "avg mem: 144.22145816738222, max mem: 149.5625, count: 55551"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 18.550726,
            "unit": "median cpu",
            "extra": "avg cpu: 19.419890630873482, max cpu: 74.708176, count: 55551"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 155.64453125,
            "unit": "median mem",
            "extra": "avg mem: 130.43576180278032, max mem: 165.50390625, count: 55551"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 20648,
            "unit": "median block_count",
            "extra": "avg block_count: 20909.182949001817, max block_count: 41558.0, count: 55551"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6153846,
            "unit": "median cpu",
            "extra": "avg cpu: 3.5344483363019736, max cpu: 4.6647234, count: 55551"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 100.203125,
            "unit": "median mem",
            "extra": "avg mem: 89.2383758280679, max mem: 129.89453125, count: 55551"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 30,
            "unit": "median segment_count",
            "extra": "avg segment_count: 30.604921603571494, max segment_count: 47.0, count: 55551"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 18.58664,
            "unit": "median cpu",
            "extra": "avg cpu: 20.63102476049701, max cpu: 74.4186, count: 111102"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 162.44921875,
            "unit": "median mem",
            "extra": "avg mem: 148.85209596952575, max mem: 171.421875, count: 111102"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.967022,
            "unit": "median cpu",
            "extra": "avg cpu: 14.866745386203021, max cpu: 28.20764, count: 55551"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 156.72265625,
            "unit": "median mem",
            "extra": "avg mem: 154.38514671484313, max mem: 158.15625, count: 55551"
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
          "id": "f4cc34211b69c210cad6133141ec34b114d4e528",
          "message": "docs: fix more-like-this (MLT) JSON syntax example (#3335)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-10-14T19:24:46Z",
          "url": "https://github.com/paradedb/paradedb/commit/f4cc34211b69c210cad6133141ec34b114d4e528"
        },
        "date": 1763944293177,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.568666,
            "unit": "median cpu",
            "extra": "avg cpu: 18.834818911069252, max cpu: 47.524754, count: 55488"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 157.73828125,
            "unit": "median mem",
            "extra": "avg mem: 148.12787238344328, max mem: 157.74609375, count: 55488"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 7.658580654933613, max cpu: 27.961164, count: 55488"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 112.24609375,
            "unit": "median mem",
            "extra": "avg mem: 110.61672582923335, max mem: 112.24609375, count: 55488"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 5.000363456274918, max cpu: 13.980582, count: 55488"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 145.4609375,
            "unit": "median mem",
            "extra": "avg mem: 123.70514268006146, max mem: 146.26953125, count: 55488"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 30327,
            "unit": "median block_count",
            "extra": "avg block_count: 30625.89500432526, max block_count: 62092.0, count: 55488"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.405130339105986, max cpu: 4.655674, count: 55488"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 104.20703125,
            "unit": "median mem",
            "extra": "avg mem: 92.58485625551921, max mem: 131.48046875, count: 55488"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 32,
            "unit": "median segment_count",
            "extra": "avg segment_count: 31.790891724336795, max segment_count: 55.0, count: 55488"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.284333,
            "unit": "median cpu",
            "extra": "avg cpu: 9.985552107097652, max cpu: 42.064266, count: 110976"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 151.2109375,
            "unit": "median mem",
            "extra": "avg mem: 142.1871145703801, max mem: 157.73828125, count: 110976"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.88621,
            "unit": "median cpu",
            "extra": "avg cpu: 12.359985068964887, max cpu: 27.87996, count: 55488"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 160.75390625,
            "unit": "median mem",
            "extra": "avg mem: 158.97907740187068, max mem: 162.4140625, count: 55488"
          }
        ]
      }
    ],
    "pg_search logical-replication.toml Performance - TPS": [
      {
        "commit": {
          "author": {
            "name": "Moe",
            "username": "mdashti",
            "email": "mdashti@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "21ae4f92dc730f588c957d8cb5c893b916d95409",
          "message": "feat: supported window aggregate pushdown for all search operators (#3582)\n\n# Ticket(s) Closed\n\n- Closes #3566\n\n## What\n\nWindow aggregate queries with `|||`, `&&&`, `===`, and `###` operators\nnow properly push down to TopN scans, just like queries using the `@@@`\noperator.\n\n## Why\n\nPreviously, queries like `SELECT *, COUNT(*) OVER () FROM table WHERE\nfield ||| 'term' ORDER BY rating LIMIT 10` would fall back to\nPostgreSQL's standard WindowAgg execution path instead of using\noptimized TopN scan. This happened because the window function\nreplacement logic only checked for the `@@@` operator when deciding\nwhether to enable pushdown.\n\n## How\n\n- Added helper functions to get OIDs for all ParadeDB search operators:\n`match_disjunction_text_opoid()` for `|||`,\n`match_conjunction_text_opoid()` for `&&&`, `term_text_opoid()` for\n`===`, and `phrase_text_opoid()` for `###`\n- Updated `query_has_search_operator()` to check for all search\noperators, not just `@@@`\n\n## Tests\n\nAdded tests in `topn-agg-facet.sql` (Tests 1a-1d) verifying that window\naggregate queries with `|||`, `&&&`, `===`, and `###` operators properly\nuse TopNScanExecState execution.",
          "timestamp": "2025-11-21T21:49:27Z",
          "url": "https://github.com/paradedb/paradedb/commit/21ae4f92dc730f588c957d8cb5c893b916d95409"
        },
        "date": 1763944799420,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - tps",
            "value": 149.45212203835968,
            "unit": "median tps",
            "extra": "avg tps: 195.80947198688028, max tps: 676.663167797579, count: 53550"
          },
          {
            "name": "Index Only Scan - Subscriber - tps",
            "value": 158.7159558513585,
            "unit": "median tps",
            "extra": "avg tps: 210.54052866711078, max tps: 789.094421995804, count: 53550"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - tps",
            "value": 75.82256049359339,
            "unit": "median tps",
            "extra": "avg tps: 77.04139187464918, max tps: 94.2394952522582, count: 53550"
          },
          {
            "name": "Top N - Subscriber - tps",
            "value": 87.64276341304725,
            "unit": "median tps",
            "extra": "avg tps: 94.1144142726411, max tps: 512.0496061954491, count: 107100"
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
          "id": "f4cc34211b69c210cad6133141ec34b114d4e528",
          "message": "docs: fix more-like-this (MLT) JSON syntax example (#3335)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-10-14T19:24:46Z",
          "url": "https://github.com/paradedb/paradedb/commit/f4cc34211b69c210cad6133141ec34b114d4e528"
        },
        "date": 1763945048275,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - tps",
            "value": 703.6041232559899,
            "unit": "median tps",
            "extra": "avg tps: 704.6267639960389, max tps: 1024.1685863426608, count: 53641"
          },
          {
            "name": "Index Only Scan - Subscriber - tps",
            "value": 741.6295070703356,
            "unit": "median tps",
            "extra": "avg tps: 740.5003784168845, max tps: 1082.0490345204944, count: 53641"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - tps",
            "value": 115.19407045358635,
            "unit": "median tps",
            "extra": "avg tps: 115.23437929122811, max tps: 126.88949025108047, count: 53641"
          },
          {
            "name": "Top N - Subscriber - tps",
            "value": 116.16742390090482,
            "unit": "median tps",
            "extra": "avg tps: 114.99810431009746, max tps: 128.0043278628977, count: 107282"
          }
        ]
      }
    ],
    "pg_search logical-replication.toml Performance - Other Metrics": [
      {
        "commit": {
          "author": {
            "name": "Moe",
            "username": "mdashti",
            "email": "mdashti@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "21ae4f92dc730f588c957d8cb5c893b916d95409",
          "message": "feat: supported window aggregate pushdown for all search operators (#3582)\n\n# Ticket(s) Closed\n\n- Closes #3566\n\n## What\n\nWindow aggregate queries with `|||`, `&&&`, `===`, and `###` operators\nnow properly push down to TopN scans, just like queries using the `@@@`\noperator.\n\n## Why\n\nPreviously, queries like `SELECT *, COUNT(*) OVER () FROM table WHERE\nfield ||| 'term' ORDER BY rating LIMIT 10` would fall back to\nPostgreSQL's standard WindowAgg execution path instead of using\noptimized TopN scan. This happened because the window function\nreplacement logic only checked for the `@@@` operator when deciding\nwhether to enable pushdown.\n\n## How\n\n- Added helper functions to get OIDs for all ParadeDB search operators:\n`match_disjunction_text_opoid()` for `|||`,\n`match_conjunction_text_opoid()` for `&&&`, `term_text_opoid()` for\n`===`, and `phrase_text_opoid()` for `###`\n- Updated `query_has_search_operator()` to check for all search\noperators, not just `@@@`\n\n## Tests\n\nAdded tests in `topn-agg-facet.sql` (Tests 1a-1d) verifying that window\naggregate queries with `|||`, `&&&`, `===`, and `###` operators properly\nuse TopNScanExecState execution.",
          "timestamp": "2025-11-21T21:49:27Z",
          "url": "https://github.com/paradedb/paradedb/commit/21ae4f92dc730f588c957d8cb5c893b916d95409"
        },
        "date": 1763944802875,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - cpu",
            "value": 9.204219,
            "unit": "median cpu",
            "extra": "avg cpu: 10.496092828811221, max cpu: 23.166023, count: 53550"
          },
          {
            "name": "Custom Scan - Subscriber - mem",
            "value": 41.46484375,
            "unit": "median mem",
            "extra": "avg mem: 40.92968976132119, max mem: 45.12109375, count: 53550"
          },
          {
            "name": "Delete values - Publisher - cpu",
            "value": 0,
            "unit": "median cpu",
            "extra": "avg cpu: 1.5738725651915322, max cpu: 4.567079, count: 53550"
          },
          {
            "name": "Delete values - Publisher - mem",
            "value": 24.21484375,
            "unit": "median mem",
            "extra": "avg mem: 23.729579321311856, max mem: 24.21484375, count: 53550"
          },
          {
            "name": "Find by ctid - Subscriber - cpu",
            "value": 18.443804,
            "unit": "median cpu",
            "extra": "avg cpu: 19.541613118724488, max cpu: 32.495163, count: 53550"
          },
          {
            "name": "Find by ctid - Subscriber - mem",
            "value": 39.33984375,
            "unit": "median mem",
            "extra": "avg mem: 40.23257703081232, max mem: 45.05078125, count: 53550"
          },
          {
            "name": "Index Only Scan - Subscriber - cpu",
            "value": 9.195402,
            "unit": "median cpu",
            "extra": "avg cpu: 10.12881757736102, max cpu: 23.166023, count: 53550"
          },
          {
            "name": "Index Only Scan - Subscriber - mem",
            "value": 38.58984375,
            "unit": "median mem",
            "extra": "avg mem: 38.72163872840803, max mem: 41.640625, count: 53550"
          },
          {
            "name": "Index Size Info - Subscriber - cpu",
            "value": 4.58891,
            "unit": "median cpu",
            "extra": "avg cpu: 4.686737673593338, max cpu: 9.221902, count: 53550"
          },
          {
            "name": "Index Size Info - Subscriber - mem",
            "value": 24.89453125,
            "unit": "median mem",
            "extra": "avg mem: 25.14923698646125, max mem: 28.34375, count: 53550"
          },
          {
            "name": "Index Size Info - Subscriber - pages",
            "value": 953,
            "unit": "median pages",
            "extra": "avg pages: 933.893893557423, max pages: 1472.0, count: 53550"
          },
          {
            "name": "Index Size Info - Subscriber - relation_size:MB",
            "value": 7.4453125,
            "unit": "median relation_size:MB",
            "extra": "avg relation_size:MB: 7.296046189309057, max relation_size:MB: 11.5, count: 53550"
          },
          {
            "name": "Index Size Info - Subscriber - segment_count",
            "value": 32,
            "unit": "median segment_count",
            "extra": "avg segment_count: 32.62782446311858, max segment_count: 64.0, count: 53550"
          },
          {
            "name": "Insert value A - Publisher - cpu",
            "value": 4.5801525,
            "unit": "median cpu",
            "extra": "avg cpu: 3.2267985913357204, max cpu: 4.6021094, count: 53550"
          },
          {
            "name": "Insert value A - Publisher - mem",
            "value": 21.80078125,
            "unit": "median mem",
            "extra": "avg mem: 21.651574244281047, max mem: 21.80078125, count: 53550"
          },
          {
            "name": "Insert value B - Publisher - cpu",
            "value": 4.58891,
            "unit": "median cpu",
            "extra": "avg cpu: 4.3785394450152255, max cpu: 4.6065254, count: 53550"
          },
          {
            "name": "Insert value B - Publisher - mem",
            "value": 21.0234375,
            "unit": "median mem",
            "extra": "avg mem: 20.960595019257703, max mem: 21.0234375, count: 53550"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - cpu",
            "value": 9.195402,
            "unit": "median cpu",
            "extra": "avg cpu: 10.187223967201646, max cpu: 18.622696, count: 53550"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - mem",
            "value": 45.078125,
            "unit": "median mem",
            "extra": "avg mem: 44.38313441001401, max mem: 50.953125, count: 53550"
          },
          {
            "name": "SELECT\n  pid,\n  pg_wal_lsn_diff(sent_lsn, replay_lsn) AS replication_lag,\n  application_name::text,\n  state::text\nFROM pg_stat_replication; - Publisher - replication_lag:MB",
            "value": 0,
            "unit": "median replication_lag:MB",
            "extra": "avg replication_lag:MB: 0.000019709482914259454, max replication_lag:MB: 0.10643768310546875, count: 53550"
          },
          {
            "name": "Top N - Subscriber - cpu",
            "value": 9.169055,
            "unit": "median cpu",
            "extra": "avg cpu: 8.865380841438213, max cpu: 18.58664, count: 107100"
          },
          {
            "name": "Top N - Subscriber - mem",
            "value": 45.61328125,
            "unit": "median mem",
            "extra": "avg mem: 45.53600420897526, max mem: 52.48828125, count: 107100"
          },
          {
            "name": "Update 1..9 - Publisher - cpu",
            "value": 4.5757866,
            "unit": "median cpu",
            "extra": "avg cpu: 3.401357362473211, max cpu: 4.624277, count: 53550"
          },
          {
            "name": "Update 1..9 - Publisher - mem",
            "value": 25.29296875,
            "unit": "median mem",
            "extra": "avg mem: 25.01349899334734, max mem: 25.29296875, count: 53550"
          },
          {
            "name": "Update 10,11 - Publisher - cpu",
            "value": 4.58891,
            "unit": "median cpu",
            "extra": "avg cpu: 4.393683473214386, max cpu: 4.624277, count: 53550"
          },
          {
            "name": "Update 10,11 - Publisher - mem",
            "value": 24.2421875,
            "unit": "median mem",
            "extra": "avg mem: 24.21595121381886, max mem: 24.2421875, count: 53550"
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
          "id": "f4cc34211b69c210cad6133141ec34b114d4e528",
          "message": "docs: fix more-like-this (MLT) JSON syntax example (#3335)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-10-14T19:24:46Z",
          "url": "https://github.com/paradedb/paradedb/commit/f4cc34211b69c210cad6133141ec34b114d4e528"
        },
        "date": 1763945051719,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - cpu",
            "value": 4.5714283,
            "unit": "median cpu",
            "extra": "avg cpu: 4.751350711145635, max cpu: 9.257474, count: 53641"
          },
          {
            "name": "Custom Scan - Subscriber - mem",
            "value": 134.18359375,
            "unit": "median mem",
            "extra": "avg mem: 115.92895709438676, max mem: 149.3125, count: 53641"
          },
          {
            "name": "Delete values - Publisher - cpu",
            "value": 4.5584044,
            "unit": "median cpu",
            "extra": "avg cpu: 3.216860543376711, max cpu: 4.5933013, count: 53641"
          },
          {
            "name": "Delete values - Publisher - mem",
            "value": 23.3984375,
            "unit": "median mem",
            "extra": "avg mem: 23.173137807950077, max mem: 23.3984375, count: 53641"
          },
          {
            "name": "Find by ctid - Subscriber - cpu",
            "value": 9.116809,
            "unit": "median cpu",
            "extra": "avg cpu: 8.086632334915254, max cpu: 18.390804, count: 53641"
          },
          {
            "name": "Find by ctid - Subscriber - mem",
            "value": 134.296875,
            "unit": "median mem",
            "extra": "avg mem: 116.67506861297795, max mem: 149.83203125, count: 53641"
          },
          {
            "name": "Index Only Scan - Subscriber - cpu",
            "value": 4.5714283,
            "unit": "median cpu",
            "extra": "avg cpu: 4.698191129562506, max cpu: 9.230769, count: 53641"
          },
          {
            "name": "Index Only Scan - Subscriber - mem",
            "value": 133.5859375,
            "unit": "median mem",
            "extra": "avg mem: 116.63790414806304, max mem: 150.23046875, count: 53641"
          },
          {
            "name": "Index Size Info - Subscriber - cpu",
            "value": 4.5714283,
            "unit": "median cpu",
            "extra": "avg cpu: 4.616340863051719, max cpu: 9.213051, count: 53641"
          },
          {
            "name": "Index Size Info - Subscriber - mem",
            "value": 118.2578125,
            "unit": "median mem",
            "extra": "avg mem: 102.90799598255066, max mem: 137.2265625, count: 53641"
          },
          {
            "name": "Index Size Info - Subscriber - pages",
            "value": 14270,
            "unit": "median pages",
            "extra": "avg pages: 14379.88859268097, max pages: 27288.0, count: 53641"
          },
          {
            "name": "Index Size Info - Subscriber - relation_size:MB",
            "value": 111.484375,
            "unit": "median relation_size:MB",
            "extra": "avg relation_size:MB: 112.3428810139399, max relation_size:MB: 213.1875, count: 53641"
          },
          {
            "name": "Index Size Info - Subscriber - segment_count",
            "value": 65,
            "unit": "median segment_count",
            "extra": "avg segment_count: 63.16528401782219, max segment_count: 100.0, count: 53641"
          },
          {
            "name": "Insert value A - Publisher - cpu",
            "value": 4.562738,
            "unit": "median cpu",
            "extra": "avg cpu: 3.1045972707957907, max cpu: 4.58891, count: 53641"
          },
          {
            "name": "Insert value A - Publisher - mem",
            "value": 21.01953125,
            "unit": "median mem",
            "extra": "avg mem: 20.86587116781007, max mem: 21.01953125, count: 53641"
          },
          {
            "name": "Insert value B - Publisher - cpu",
            "value": 4.5540795,
            "unit": "median cpu",
            "extra": "avg cpu: 3.853928221550767, max cpu: 4.6065254, count: 53641"
          },
          {
            "name": "Insert value B - Publisher - mem",
            "value": 21.42578125,
            "unit": "median mem",
            "extra": "avg mem: 21.159607462691785, max mem: 21.42578125, count: 53641"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - cpu",
            "value": 9.073725,
            "unit": "median cpu",
            "extra": "avg cpu: 7.113827502887376, max cpu: 13.859479, count: 53641"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - mem",
            "value": 137.796875,
            "unit": "median mem",
            "extra": "avg mem: 119.35592738355456, max mem: 155.6796875, count: 53641"
          },
          {
            "name": "SELECT\n  pid,\n  pg_wal_lsn_diff(sent_lsn, replay_lsn) AS replication_lag,\n  application_name::text,\n  state::text\nFROM pg_stat_replication; - Publisher - replication_lag:MB",
            "value": 0,
            "unit": "median replication_lag:MB",
            "extra": "avg replication_lag:MB: 0.00003271475727376214, max replication_lag:MB: 0.18213653564453125, count: 53641"
          },
          {
            "name": "Top N - Subscriber - cpu",
            "value": 4.5757866,
            "unit": "median cpu",
            "extra": "avg cpu: 4.9092512968626005, max cpu: 9.257474, count: 107282"
          },
          {
            "name": "Top N - Subscriber - mem",
            "value": 141.7421875,
            "unit": "median mem",
            "extra": "avg mem: 123.90727416994463, max mem: 159.6875, count: 107282"
          },
          {
            "name": "Update 1..9 - Publisher - cpu",
            "value": 4.562738,
            "unit": "median cpu",
            "extra": "avg cpu: 3.807035601158808, max cpu: 4.597701, count: 53641"
          },
          {
            "name": "Update 1..9 - Publisher - mem",
            "value": 24.5390625,
            "unit": "median mem",
            "extra": "avg mem: 24.347641175826325, max mem: 24.5390625, count: 53641"
          },
          {
            "name": "Update 10,11 - Publisher - cpu",
            "value": 4.5584044,
            "unit": "median cpu",
            "extra": "avg cpu: 4.34462689584058, max cpu: 4.6021094, count: 53641"
          },
          {
            "name": "Update 10,11 - Publisher - mem",
            "value": 24.078125,
            "unit": "median mem",
            "extra": "avg mem: 24.05242462621875, max mem: 24.078125, count: 53641"
          }
        ]
      }
    ]
  }
}