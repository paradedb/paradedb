window.BENCHMARK_DATA = {
  "lastUpdate": 1763942771248,
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
      }
    ]
  }
}