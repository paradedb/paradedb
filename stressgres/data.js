window.BENCHMARK_DATA = {
  "lastUpdate": 1764095140124,
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
          "id": "799b8f38d6953f1bb21fce531ac87cb9f4656d8c",
          "message": "fix: Skip flaky edge case numeric values in json_pushdown prop tests (#3621)\n\n## Summary\n- Skip edge case numeric values added in PR #2978 that cause\nintermittent prop test failures on main\n\n## Test plan\n- [x] Verify prop tests pass without flaky failures\n\nCo-authored-by: Mithun Chicklore Yogendra <mithun.cy@gmail.com>",
          "timestamp": "2025-11-24T14:05:46-08:00",
          "tree_id": "7b6eed5a50c508b34c97163193ded91671efa67f",
          "url": "https://github.com/paradedb/paradedb/commit/799b8f38d6953f1bb21fce531ac87cb9f4656d8c"
        },
        "date": 1764022956978,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 140.63656550155565,
            "unit": "median tps",
            "extra": "avg tps: 156.56325394927913, max tps: 597.7169218827944, count: 55559"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3326.3554169397894,
            "unit": "median tps",
            "extra": "avg tps: 3294.2806952717415, max tps: 3338.464752241852, count: 55559"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 149.24028527927456,
            "unit": "median tps",
            "extra": "avg tps: 165.4774353271746, max tps: 591.0477490013644, count: 55559"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 120.36141855011245,
            "unit": "median tps",
            "extra": "avg tps: 134.94194153557453, max tps: 405.2724087707202, count: 55559"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 3388.191411157891,
            "unit": "median tps",
            "extra": "avg tps: 3374.3125287453713, max tps: 3444.663077504389, count: 111118"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 2184.306280978499,
            "unit": "median tps",
            "extra": "avg tps: 2170.6377944916253, max tps: 2198.6733788199667, count: 55559"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 31.513872725576928,
            "unit": "median tps",
            "extra": "avg tps: 39.7859303919812, max tps: 305.45002257275667, count: 55559"
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
          "id": "6bd02ab4267eaf048ba63da91b81c4415e153ea2",
          "message": "fix: Mutable segment corruption when reading beyond number of entries (#3618)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nUnder physical replication, we've observed a rare issue where entries of\nthe mutable segment fail to deserialize.\n\nThis always seems to happen when we are reading beyond the actual length\nof the mutable segment list. For instance, the mutable segment list only\ncontains 400 entries, but we try and deserialize entry 401.\n\nI don't yet have a perfect theory for why this is happening, but\nstopping the reading of the merge segment list when we've reached the\nnumber of entries seems to be working as a stopgap.\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-11-24T20:44:24-05:00",
          "tree_id": "db68d25211a34973b28339f0956e1982636fbffe",
          "url": "https://github.com/paradedb/paradedb/commit/6bd02ab4267eaf048ba63da91b81c4415e153ea2"
        },
        "date": 1764036063499,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 137.80515657408603,
            "unit": "median tps",
            "extra": "avg tps: 152.5279095900994, max tps: 568.9819012826848, count: 55497"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3144.215151540222,
            "unit": "median tps",
            "extra": "avg tps: 3124.729303696904, max tps: 3161.8709869725776, count: 55497"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 137.91542140793308,
            "unit": "median tps",
            "extra": "avg tps: 152.43560839849562, max tps: 600.4744866569496, count: 55497"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 115.63284870394803,
            "unit": "median tps",
            "extra": "avg tps: 128.96148735573212, max tps: 421.9481743842605, count: 55497"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 3049.1683949870603,
            "unit": "median tps",
            "extra": "avg tps: 3054.8358193959652, max tps: 3376.288035536166, count: 110994"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 2084.8778382980454,
            "unit": "median tps",
            "extra": "avg tps: 2078.4656782271904, max tps: 2101.921325565055, count: 55497"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 44.86699589911744,
            "unit": "median tps",
            "extra": "avg tps: 61.56132866040048, max tps: 1020.7039591065165, count: 55497"
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
          "id": "553484d76445895df38c2d1102f1a6e9b3b6fbf8",
          "message": "fix: Mutable segment corruption when reading beyond number of entries (#3624)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nUnder physical replication, we've observed a rare issue where entries of\nthe mutable segment fail to deserialize.\n\nThis always seems to happen when we are reading beyond the actual length\nof the mutable segment list. For instance, the mutable segment list only\ncontains 400 entries, but we try and deserialize entry 401.\n\nI don't yet have a perfect theory for why this is happening, but\nstopping the reading of the merge segment list when we've reached the\nnumber of entries seems to be working as a stopgap.\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-11-24T21:19:04-05:00",
          "tree_id": "15acd09158f6c1da87843db016cc4d76c3c2a3c1",
          "url": "https://github.com/paradedb/paradedb/commit/553484d76445895df38c2d1102f1a6e9b3b6fbf8"
        },
        "date": 1764038150482,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 142.530114950353,
            "unit": "median tps",
            "extra": "avg tps: 158.49529042774208, max tps: 606.4584073334423, count: 55575"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3300.052348124444,
            "unit": "median tps",
            "extra": "avg tps: 3281.1167346163184, max tps: 3308.032626032571, count: 55575"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 144.35324718941192,
            "unit": "median tps",
            "extra": "avg tps: 160.45902083099372, max tps: 580.0331910721645, count: 55575"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 121.07177431522443,
            "unit": "median tps",
            "extra": "avg tps: 135.3779880771613, max tps: 436.54763722411144, count: 55575"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 3453.1889599980286,
            "unit": "median tps",
            "extra": "avg tps: 3440.1238023285086, max tps: 3471.633270263959, count: 111150"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 2160.555860337238,
            "unit": "median tps",
            "extra": "avg tps: 2148.6845064346307, max tps: 2176.9782113465767, count: 55575"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 40.404122499882995,
            "unit": "median tps",
            "extra": "avg tps: 53.328657322758865, max tps: 375.75155006908193, count: 55575"
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
          "id": "316302d369db580ef5701cbc6fcf1abf8cfb5da4",
          "message": "chore: Upgrade to `0.19.10` (#3626)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-11-24T22:54:13-05:00",
          "tree_id": "7bac085a424878b8a62885db4e53baf3b8ef58e1",
          "url": "https://github.com/paradedb/paradedb/commit/316302d369db580ef5701cbc6fcf1abf8cfb5da4"
        },
        "date": 1764043856835,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 132.19779682153805,
            "unit": "median tps",
            "extra": "avg tps: 149.3369139301625, max tps: 623.9357151545447, count: 55519"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3183.7039575928134,
            "unit": "median tps",
            "extra": "avg tps: 3151.956169666283, max tps: 3208.084676581313, count: 55519"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 132.279850236125,
            "unit": "median tps",
            "extra": "avg tps: 149.2021481974358, max tps: 544.5902312493498, count: 55519"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 110.60409722823948,
            "unit": "median tps",
            "extra": "avg tps: 124.2561725830974, max tps: 434.10004404668445, count: 55519"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 3237.3253333464245,
            "unit": "median tps",
            "extra": "avg tps: 3271.8310099624427, max tps: 3372.5603370725835, count: 111038"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 2065.065619749485,
            "unit": "median tps",
            "extra": "avg tps: 2057.55874503355, max tps: 2071.6159148320844, count: 55519"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 23.128074624601304,
            "unit": "median tps",
            "extra": "avg tps: 37.723998155527966, max tps: 311.76506257826276, count: 55519"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mdashti@gmail.com",
            "name": "Moe",
            "username": "mdashti"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "555dbffd8e840906924893fe14ba577711ca9a30",
          "message": "feat: enable TopN optimization for LEFT JOIN LATERAL queries (#3590)\n\n# Ticket(s) Closed\n\n- Closes #3239\n\n## What\n\nEnables TopN optimization for `LEFT JOIN LATERAL` queries, allowing\nefficient execution of queries that combine lateral joins with `ORDER\nBY` and `LIMIT` clauses.\n\n## Why\n\nPreviously, `LEFT JOIN LATERAL` queries would default to a Normal scan\neven when they could benefit from TopN optimization. This was due to:\n1. The LIMIT from joined relations not being extracted\n2. The `paradedb.score()` function being wrapped in `PlaceHolderVar`\nduring joins, preventing proper pathkey extraction\n\nThis resulted in suboptimal performance for common query patterns like\nfetching the latest comment for each article.\n\n## How\n\n- Added `is_left_join_lateral()` to detect LEFT JOIN LATERAL patterns in\nthe query tree\n- Added `where_clause_only_references_left()` to ensure WHERE clauses\nonly reference the driving (left) table\n- Added `extract_funcexpr_from_placeholder()` to unwrap score functions\nfrom PlaceHolderVar nodes\n- Used `contains_lateral_reference()` for recursive detection of LATERAL\nin nested joins\n- Modified `create_custom_path()` to extract LIMIT for LEFT JOIN LATERAL\nqueries when conditions are met\n- Updated pathkey extraction to handle PlaceHolderVar-wrapped score\nfunctions\n\nThe optimization applies when:\n- The query uses LEFT JOIN LATERAL\n- The WHERE clause only references the left table\n- ORDER BY columns are from the left table and are indexed/fast fields\n- A LIMIT clause is present\n\n## Tests\n\nAdded regression tests in `lateral-join.sql`.",
          "timestamp": "2025-11-25T01:19:32-08:00",
          "tree_id": "ae5b3b7155e76725f727456e012ee1875f64b665",
          "url": "https://github.com/paradedb/paradedb/commit/555dbffd8e840906924893fe14ba577711ca9a30"
        },
        "date": 1764063384308,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 142.94971926305652,
            "unit": "median tps",
            "extra": "avg tps: 159.22408664211852, max tps: 623.5213167616696, count: 54965"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3087.0695555926077,
            "unit": "median tps",
            "extra": "avg tps: 3074.4411212296422, max tps: 3214.5415338674416, count: 54965"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 142.32153423934048,
            "unit": "median tps",
            "extra": "avg tps: 157.777352214688, max tps: 605.856312282029, count: 54965"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 120.22711543198776,
            "unit": "median tps",
            "extra": "avg tps: 134.89950213163885, max tps: 430.5297767309481, count: 54965"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 3315.470667837368,
            "unit": "median tps",
            "extra": "avg tps: 3343.923867684852, max tps: 3468.908789963555, count: 109930"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 2152.53663627426,
            "unit": "median tps",
            "extra": "avg tps: 2154.456406288744, max tps: 2180.4962141834917, count: 54965"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 55.63930601546752,
            "unit": "median tps",
            "extra": "avg tps: 76.33501577133191, max tps: 922.185099148731, count: 54965"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mdashti@gmail.com",
            "name": "Moe",
            "username": "mdashti"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "1478d17163b3a319c78213f973161105f4fe8172",
          "message": "feat: `pdb.agg()` support for wrapped functions and CTEs (#3588)\n\n## What\n\nFixes `pdb.agg()` to work correctly when:\n- Wrapped in other functions (e.g., `jsonb_pretty(pdb.agg(...))`)\n- Used inside Common Table Expressions (CTEs)\n- Used in subqueries\n\nAlso removes the `SUBQUERY_SUPPORT` feature flag - subqueries and CTEs\nare now always supported.\n\n- Closes #3504\n\n## Why\n\nPreviously, `pdb.agg()` would fail with errors like:\n- \"pdb.agg() must be handled by ParadeDB's custom scan\"\n- \"window_agg placeholder should not be executed\"\n\nThis happened because the planner hook only checked top-level\nexpressions and didn't recursively process CTEs or nested function\ncalls.\n\n## How\n\n**Planning stage:**\n- Added recursive CTE and subquery processing to the planner hook\n- Implemented `replace_in_node()` to walk expression trees and find\n`WindowFunc` nodes even when wrapped in other functions\n- Split `pdb.agg()` detection into two helpers:\n  - `query_has_paradedb_agg()` - recursive check for feature enablement\n- `query_has_paradedb_agg_at_current_level()` - non-recursive check for\nper-level validation\n\n**Execution stage:**\n- Implemented `replace_window_agg_with_const()` to recursively find and\nreplace `window_agg()` placeholders with `Const` nodes, even when\nwrapped\n- Updated `inject_window_aggregate_placeholders()` to handle nested\nexpressions\n\n**Detection:**\n- Changed `extract_and_convert_window_functions()` to use\n`expression_tree_walker` instead of only checking top-level nodes\n\n## Tests\n\nAdded regression tests in `fn_wrapped_agg.sql` covering:\n1. Basic `pdb.agg()` in TopN queries\n2. `pdb.agg()` wrapped in `jsonb_pretty()`\n3. `pdb.agg()` inside CTEs\n4. `pdb.agg()` in CTEs with outer function wrapping",
          "timestamp": "2025-11-25T09:13:06-08:00",
          "tree_id": "2564b4cb5637090acfaaf73c71de54b14cbe6a19",
          "url": "https://github.com/paradedb/paradedb/commit/1478d17163b3a319c78213f973161105f4fe8172"
        },
        "date": 1764092225164,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 144.8873498866958,
            "unit": "median tps",
            "extra": "avg tps: 160.65617987017038, max tps: 620.9010860780012, count: 55583"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3218.224809344207,
            "unit": "median tps",
            "extra": "avg tps: 3202.08331606635, max tps: 3225.1518067912098, count: 55583"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 136.4318961656868,
            "unit": "median tps",
            "extra": "avg tps: 152.29240577406347, max tps: 607.8629328136047, count: 55583"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 116.27990312063356,
            "unit": "median tps",
            "extra": "avg tps: 130.53333025525558, max tps: 411.46993473766804, count: 55583"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 3429.123603077506,
            "unit": "median tps",
            "extra": "avg tps: 3448.930493495343, max tps: 3529.7628736132783, count: 111166"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 2201.6478458253055,
            "unit": "median tps",
            "extra": "avg tps: 2188.4227385473982, max tps: 2209.5038629374108, count: 55583"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 43.848542905447104,
            "unit": "median tps",
            "extra": "avg tps: 69.10212647088757, max tps: 398.7061586445825, count: 55583"
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
          "id": "799b8f38d6953f1bb21fce531ac87cb9f4656d8c",
          "message": "fix: Skip flaky edge case numeric values in json_pushdown prop tests (#3621)\n\n## Summary\n- Skip edge case numeric values added in PR #2978 that cause\nintermittent prop test failures on main\n\n## Test plan\n- [x] Verify prop tests pass without flaky failures\n\nCo-authored-by: Mithun Chicklore Yogendra <mithun.cy@gmail.com>",
          "timestamp": "2025-11-24T14:05:46-08:00",
          "tree_id": "7b6eed5a50c508b34c97163193ded91671efa67f",
          "url": "https://github.com/paradedb/paradedb/commit/799b8f38d6953f1bb21fce531ac87cb9f4656d8c"
        },
        "date": 1764022960760,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 13.899614,
            "unit": "median cpu",
            "extra": "avg cpu: 11.993567154243603, max cpu: 33.136093, count: 55559"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 51.5546875,
            "unit": "median mem",
            "extra": "avg mem: 50.54756036375745, max mem: 59.6328125, count: 55559"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.623683122414891, max cpu: 4.776119, count: 55559"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 26.81640625,
            "unit": "median mem",
            "extra": "avg mem: 27.049323270082255, max mem: 29.19140625, count: 55559"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 9.476802,
            "unit": "median cpu",
            "extra": "avg cpu: 11.49011721200917, max cpu: 33.333336, count: 55559"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 51.37890625,
            "unit": "median mem",
            "extra": "avg mem: 50.35769097940928, max mem: 59.05859375, count: 55559"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.669261,
            "unit": "median cpu",
            "extra": "avg cpu: 5.147202221623675, max cpu: 14.035088, count: 55559"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 48.8203125,
            "unit": "median mem",
            "extra": "avg mem: 48.88704919826221, max mem: 59.4765625, count: 55559"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 4.629291955937004, max cpu: 9.504951, count: 111118"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 34.22265625,
            "unit": "median mem",
            "extra": "avg mem: 35.501339686470686, max mem: 45.04296875, count: 111118"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 1490,
            "unit": "median block_count",
            "extra": "avg block_count: 1471.7922028834212, max block_count: 2502.0, count: 55559"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 26,
            "unit": "median segment_count",
            "extra": "avg segment_count: 26.094044169261505, max segment_count: 50.0, count: 55559"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 4.436882752465221, max cpu: 4.776119, count: 55559"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 38.78515625,
            "unit": "median mem",
            "extra": "avg mem: 39.168389122374414, max mem: 47.94921875, count: 55559"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.455263588676604, max cpu: 9.365853, count: 55559"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 43.6875,
            "unit": "median mem",
            "extra": "avg mem: 42.25248440846668, max mem: 50.328125, count: 55559"
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
          "id": "6bd02ab4267eaf048ba63da91b81c4415e153ea2",
          "message": "fix: Mutable segment corruption when reading beyond number of entries (#3618)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nUnder physical replication, we've observed a rare issue where entries of\nthe mutable segment fail to deserialize.\n\nThis always seems to happen when we are reading beyond the actual length\nof the mutable segment list. For instance, the mutable segment list only\ncontains 400 entries, but we try and deserialize entry 401.\n\nI don't yet have a perfect theory for why this is happening, but\nstopping the reading of the merge segment list when we've reached the\nnumber of entries seems to be working as a stopgap.\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-11-24T20:44:24-05:00",
          "tree_id": "db68d25211a34973b28339f0956e1982636fbffe",
          "url": "https://github.com/paradedb/paradedb/commit/6bd02ab4267eaf048ba63da91b81c4415e153ea2"
        },
        "date": 1764036068419,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 13.88621,
            "unit": "median cpu",
            "extra": "avg cpu: 12.325565465219645, max cpu: 38.63179, count: 55497"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 51.921875,
            "unit": "median mem",
            "extra": "avg mem: 50.81645854730886, max mem: 58.99609375, count: 55497"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.622668897414468, max cpu: 9.320388, count: 55497"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 25.515625,
            "unit": "median mem",
            "extra": "avg mem: 26.126266608442798, max mem: 28.20703125, count: 55497"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 13.88621,
            "unit": "median cpu",
            "extra": "avg cpu: 12.35493283521767, max cpu: 38.63179, count: 55497"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 51.390625,
            "unit": "median mem",
            "extra": "avg mem: 50.68879075107664, max mem: 58.9375, count: 55497"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 5.349929852488862, max cpu: 14.007783, count: 55497"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 49.33203125,
            "unit": "median mem",
            "extra": "avg mem: 49.19146354476368, max mem: 59.484375, count: 55497"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.609607223437717, max cpu: 9.347614, count: 110994"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 37.3046875,
            "unit": "median mem",
            "extra": "avg mem: 36.45810806073977, max mem: 46.9375, count: 110994"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 1502,
            "unit": "median block_count",
            "extra": "avg block_count: 1475.172117411752, max block_count: 2477.0, count: 55497"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 27,
            "unit": "median segment_count",
            "extra": "avg segment_count: 26.860334792871686, max segment_count: 49.0, count: 55497"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.548733933193364, max cpu: 4.819277, count: 55497"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 38.98828125,
            "unit": "median mem",
            "extra": "avg mem: 38.94495007894571, max mem: 47.83984375, count: 55497"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.801358510981222, max cpu: 9.338522, count: 55497"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 40.83203125,
            "unit": "median mem",
            "extra": "avg mem: 40.685650167464004, max mem: 49.0390625, count: 55497"
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
          "id": "553484d76445895df38c2d1102f1a6e9b3b6fbf8",
          "message": "fix: Mutable segment corruption when reading beyond number of entries (#3624)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nUnder physical replication, we've observed a rare issue where entries of\nthe mutable segment fail to deserialize.\n\nThis always seems to happen when we are reading beyond the actual length\nof the mutable segment list. For instance, the mutable segment list only\ncontains 400 entries, but we try and deserialize entry 401.\n\nI don't yet have a perfect theory for why this is happening, but\nstopping the reading of the merge segment list when we've reached the\nnumber of entries seems to be working as a stopgap.\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-11-24T21:19:04-05:00",
          "tree_id": "15acd09158f6c1da87843db016cc4d76c3c2a3c1",
          "url": "https://github.com/paradedb/paradedb/commit/553484d76445895df38c2d1102f1a6e9b3b6fbf8"
        },
        "date": 1764038154214,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 13.88621,
            "unit": "median cpu",
            "extra": "avg cpu: 12.026533351872718, max cpu: 34.615387, count: 55575"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 51.18359375,
            "unit": "median mem",
            "extra": "avg mem: 49.582463872019794, max mem: 59.2578125, count: 55575"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.600259549456041, max cpu: 9.458128, count: 55575"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 26.54296875,
            "unit": "median mem",
            "extra": "avg mem: 26.858468215811964, max mem: 29.3203125, count: 55575"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 13.872832,
            "unit": "median cpu",
            "extra": "avg cpu: 11.934466705690106, max cpu: 37.17328, count: 55575"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 51.41796875,
            "unit": "median mem",
            "extra": "avg mem: 50.81052666722897, max mem: 59.99609375, count: 55575"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 5.173944918246008, max cpu: 14.10382, count: 55575"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 50.71875,
            "unit": "median mem",
            "extra": "avg mem: 49.59115729026091, max mem: 58.6171875, count: 55575"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.685902782242397, max cpu: 9.476802, count: 111150"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 35.50390625,
            "unit": "median mem",
            "extra": "avg mem: 35.89409838197256, max mem: 44.05859375, count: 111150"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 1348,
            "unit": "median block_count",
            "extra": "avg block_count: 1343.592586594692, max block_count: 2304.0, count: 55575"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 26,
            "unit": "median segment_count",
            "extra": "avg segment_count: 26.281349527665316, max segment_count: 49.0, count: 55575"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.669261,
            "unit": "median cpu",
            "extra": "avg cpu: 4.605685469778844, max cpu: 4.7619047, count: 55575"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 38.9375,
            "unit": "median mem",
            "extra": "avg mem: 38.70551535087719, max mem: 47.44140625, count: 55575"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 6.104703261732337, max cpu: 9.476802, count: 55575"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 43.359375,
            "unit": "median mem",
            "extra": "avg mem: 41.48308317869996, max mem: 50.1015625, count: 55575"
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
          "id": "316302d369db580ef5701cbc6fcf1abf8cfb5da4",
          "message": "chore: Upgrade to `0.19.10` (#3626)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-11-24T22:54:13-05:00",
          "tree_id": "7bac085a424878b8a62885db4e53baf3b8ef58e1",
          "url": "https://github.com/paradedb/paradedb/commit/316302d369db580ef5701cbc6fcf1abf8cfb5da4"
        },
        "date": 1764043860646,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 13.9265,
            "unit": "median cpu",
            "extra": "avg cpu: 12.779951454790394, max cpu: 38.63179, count: 55519"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 50.45703125,
            "unit": "median mem",
            "extra": "avg mem: 49.95814967342261, max mem: 60.38671875, count: 55519"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.641065426794962, max cpu: 9.29332, count: 55519"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 27.64453125,
            "unit": "median mem",
            "extra": "avg mem: 27.350499167514723, max mem: 28.8203125, count: 55519"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 13.9265,
            "unit": "median cpu",
            "extra": "avg cpu: 12.804943249114505, max cpu: 37.463413, count: 55519"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 52.3359375,
            "unit": "median mem",
            "extra": "avg mem: 50.93363519133089, max mem: 59.765625, count: 55519"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 5.442621054641706, max cpu: 14.035088, count: 55519"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 49.2578125,
            "unit": "median mem",
            "extra": "avg mem: 48.733201344922456, max mem: 58.3828125, count: 55519"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.6846286919710325, max cpu: 9.628887, count: 111038"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 35.6953125,
            "unit": "median mem",
            "extra": "avg mem: 35.500418775554316, max mem: 44.44921875, count: 111038"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 1474,
            "unit": "median block_count",
            "extra": "avg block_count: 1455.6816044957582, max block_count: 2466.0, count: 55519"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 26,
            "unit": "median segment_count",
            "extra": "avg segment_count: 25.87087303445667, max segment_count: 49.0, count: 55519"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.757005241561451, max cpu: 9.514371, count: 55519"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 38.0625,
            "unit": "median mem",
            "extra": "avg mem: 37.82697702588303, max mem: 46.109375, count: 55519"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.669261,
            "unit": "median cpu",
            "extra": "avg cpu: 5.329905440625324, max cpu: 9.347614, count: 55519"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 40.47265625,
            "unit": "median mem",
            "extra": "avg mem: 38.739287521501645, max mem: 49.6875, count: 55519"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mdashti@gmail.com",
            "name": "Moe",
            "username": "mdashti"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "555dbffd8e840906924893fe14ba577711ca9a30",
          "message": "feat: enable TopN optimization for LEFT JOIN LATERAL queries (#3590)\n\n# Ticket(s) Closed\n\n- Closes #3239\n\n## What\n\nEnables TopN optimization for `LEFT JOIN LATERAL` queries, allowing\nefficient execution of queries that combine lateral joins with `ORDER\nBY` and `LIMIT` clauses.\n\n## Why\n\nPreviously, `LEFT JOIN LATERAL` queries would default to a Normal scan\neven when they could benefit from TopN optimization. This was due to:\n1. The LIMIT from joined relations not being extracted\n2. The `paradedb.score()` function being wrapped in `PlaceHolderVar`\nduring joins, preventing proper pathkey extraction\n\nThis resulted in suboptimal performance for common query patterns like\nfetching the latest comment for each article.\n\n## How\n\n- Added `is_left_join_lateral()` to detect LEFT JOIN LATERAL patterns in\nthe query tree\n- Added `where_clause_only_references_left()` to ensure WHERE clauses\nonly reference the driving (left) table\n- Added `extract_funcexpr_from_placeholder()` to unwrap score functions\nfrom PlaceHolderVar nodes\n- Used `contains_lateral_reference()` for recursive detection of LATERAL\nin nested joins\n- Modified `create_custom_path()` to extract LIMIT for LEFT JOIN LATERAL\nqueries when conditions are met\n- Updated pathkey extraction to handle PlaceHolderVar-wrapped score\nfunctions\n\nThe optimization applies when:\n- The query uses LEFT JOIN LATERAL\n- The WHERE clause only references the left table\n- ORDER BY columns are from the left table and are indexed/fast fields\n- A LIMIT clause is present\n\n## Tests\n\nAdded regression tests in `lateral-join.sql`.",
          "timestamp": "2025-11-25T01:19:32-08:00",
          "tree_id": "ae5b3b7155e76725f727456e012ee1875f64b665",
          "url": "https://github.com/paradedb/paradedb/commit/555dbffd8e840906924893fe14ba577711ca9a30"
        },
        "date": 1764063387638,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 13.846154,
            "unit": "median cpu",
            "extra": "avg cpu: 12.031809699136156, max cpu: 37.75811, count: 54965"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 50.33984375,
            "unit": "median mem",
            "extra": "avg mem: 50.2947229913354, max mem: 57.41796875, count: 54965"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.654315256425771, max cpu: 9.486166, count: 54965"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 26.28515625,
            "unit": "median mem",
            "extra": "avg mem: 26.256694032566177, max mem: 27.44140625, count: 54965"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 13.859479,
            "unit": "median cpu",
            "extra": "avg cpu: 12.06523748138105, max cpu: 37.029896, count: 54965"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 51.5078125,
            "unit": "median mem",
            "extra": "avg mem: 50.111867850563996, max mem: 57.90234375, count: 54965"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.996463523129196, max cpu: 13.93998, count: 54965"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 46.39453125,
            "unit": "median mem",
            "extra": "avg mem: 47.80973886791595, max mem: 58.890625, count: 54965"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.641232932774454, max cpu: 9.411765, count: 109930"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 36.609375,
            "unit": "median mem",
            "extra": "avg mem: 36.00239083251842, max mem: 44.70703125, count: 109930"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 1322,
            "unit": "median block_count",
            "extra": "avg block_count: 1327.0021468207042, max block_count: 2291.0, count: 54965"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 26,
            "unit": "median segment_count",
            "extra": "avg segment_count: 25.81353588647321, max segment_count: 49.0, count: 54965"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.535618249115329, max cpu: 4.8144436, count: 54965"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 38.3828125,
            "unit": "median mem",
            "extra": "avg mem: 38.27974279086691, max mem: 46.453125, count: 54965"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 5.371982947605969, max cpu: 9.467456, count: 54965"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 41.20703125,
            "unit": "median mem",
            "extra": "avg mem: 41.044863776948965, max mem: 50.21875, count: 54965"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mdashti@gmail.com",
            "name": "Moe",
            "username": "mdashti"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "1478d17163b3a319c78213f973161105f4fe8172",
          "message": "feat: `pdb.agg()` support for wrapped functions and CTEs (#3588)\n\n## What\n\nFixes `pdb.agg()` to work correctly when:\n- Wrapped in other functions (e.g., `jsonb_pretty(pdb.agg(...))`)\n- Used inside Common Table Expressions (CTEs)\n- Used in subqueries\n\nAlso removes the `SUBQUERY_SUPPORT` feature flag - subqueries and CTEs\nare now always supported.\n\n- Closes #3504\n\n## Why\n\nPreviously, `pdb.agg()` would fail with errors like:\n- \"pdb.agg() must be handled by ParadeDB's custom scan\"\n- \"window_agg placeholder should not be executed\"\n\nThis happened because the planner hook only checked top-level\nexpressions and didn't recursively process CTEs or nested function\ncalls.\n\n## How\n\n**Planning stage:**\n- Added recursive CTE and subquery processing to the planner hook\n- Implemented `replace_in_node()` to walk expression trees and find\n`WindowFunc` nodes even when wrapped in other functions\n- Split `pdb.agg()` detection into two helpers:\n  - `query_has_paradedb_agg()` - recursive check for feature enablement\n- `query_has_paradedb_agg_at_current_level()` - non-recursive check for\nper-level validation\n\n**Execution stage:**\n- Implemented `replace_window_agg_with_const()` to recursively find and\nreplace `window_agg()` placeholders with `Const` nodes, even when\nwrapped\n- Updated `inject_window_aggregate_placeholders()` to handle nested\nexpressions\n\n**Detection:**\n- Changed `extract_and_convert_window_functions()` to use\n`expression_tree_walker` instead of only checking top-level nodes\n\n## Tests\n\nAdded regression tests in `fn_wrapped_agg.sql` covering:\n1. Basic `pdb.agg()` in TopN queries\n2. `pdb.agg()` wrapped in `jsonb_pretty()`\n3. `pdb.agg()` inside CTEs\n4. `pdb.agg()` in CTEs with outer function wrapping",
          "timestamp": "2025-11-25T09:13:06-08:00",
          "tree_id": "2564b4cb5637090acfaaf73c71de54b14cbe6a19",
          "url": "https://github.com/paradedb/paradedb/commit/1478d17163b3a319c78213f973161105f4fe8172"
        },
        "date": 1764092229059,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 13.872832,
            "unit": "median cpu",
            "extra": "avg cpu: 11.805331949499783, max cpu: 33.3996, count: 55583"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 50.8828125,
            "unit": "median mem",
            "extra": "avg mem: 49.99886986297519, max mem: 59.4140625, count: 55583"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.728644401456449, max cpu: 9.504951, count: 55583"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 26.85546875,
            "unit": "median mem",
            "extra": "avg mem: 26.991311979382186, max mem: 29.27734375, count: 55583"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 13.9265,
            "unit": "median cpu",
            "extra": "avg cpu: 12.377682033107645, max cpu: 37.72102, count: 55583"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 50.37109375,
            "unit": "median mem",
            "extra": "avg mem: 49.86928880570498, max mem: 58.34375, count: 55583"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.669261,
            "unit": "median cpu",
            "extra": "avg cpu: 5.0051675504035975, max cpu: 18.713451, count: 55583"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 49.98828125,
            "unit": "median mem",
            "extra": "avg mem: 49.05575642507601, max mem: 57.7421875, count: 55583"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 4.605778839696827, max cpu: 7.5471697, count: 111166"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 35.65625,
            "unit": "median mem",
            "extra": "avg mem: 35.25474645603197, max mem: 43.74609375, count: 111166"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 1477,
            "unit": "median block_count",
            "extra": "avg block_count: 1448.8387456596442, max block_count: 2445.0, count: 55583"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 27,
            "unit": "median segment_count",
            "extra": "avg segment_count: 26.544680927621755, max segment_count: 49.0, count: 55583"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.596459577589649, max cpu: 4.7808766, count: 55583"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 39.6484375,
            "unit": "median mem",
            "extra": "avg mem: 38.682145254619215, max mem: 46.890625, count: 55583"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.678363,
            "unit": "median cpu",
            "extra": "avg cpu: 6.827866600913025, max cpu: 14.076246, count: 55583"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 41.26171875,
            "unit": "median mem",
            "extra": "avg mem: 39.967835686046094, max mem: 47.71484375, count: 55583"
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
          "id": "799b8f38d6953f1bb21fce531ac87cb9f4656d8c",
          "message": "fix: Skip flaky edge case numeric values in json_pushdown prop tests (#3621)\n\n## Summary\n- Skip edge case numeric values added in PR #2978 that cause\nintermittent prop test failures on main\n\n## Test plan\n- [x] Verify prop tests pass without flaky failures\n\nCo-authored-by: Mithun Chicklore Yogendra <mithun.cy@gmail.com>",
          "timestamp": "2025-11-24T14:05:46-08:00",
          "tree_id": "7b6eed5a50c508b34c97163193ded91671efa67f",
          "url": "https://github.com/paradedb/paradedb/commit/799b8f38d6953f1bb21fce531ac87cb9f4656d8c"
        },
        "date": 1764023681016,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.130501032476094,
            "unit": "median tps",
            "extra": "avg tps: 6.933815216801957, max tps: 10.647401957777339, count: 57746"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.448093759447688,
            "unit": "median tps",
            "extra": "avg tps: 4.918359000399434, max tps: 6.056986486635658, count: 57746"
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
          "id": "6bd02ab4267eaf048ba63da91b81c4415e153ea2",
          "message": "fix: Mutable segment corruption when reading beyond number of entries (#3618)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nUnder physical replication, we've observed a rare issue where entries of\nthe mutable segment fail to deserialize.\n\nThis always seems to happen when we are reading beyond the actual length\nof the mutable segment list. For instance, the mutable segment list only\ncontains 400 entries, but we try and deserialize entry 401.\n\nI don't yet have a perfect theory for why this is happening, but\nstopping the reading of the merge segment list when we've reached the\nnumber of entries seems to be working as a stopgap.\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-11-24T20:44:24-05:00",
          "tree_id": "db68d25211a34973b28339f0956e1982636fbffe",
          "url": "https://github.com/paradedb/paradedb/commit/6bd02ab4267eaf048ba63da91b81c4415e153ea2"
        },
        "date": 1764036798532,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 7.934330186099883,
            "unit": "median tps",
            "extra": "avg tps: 6.7763557285874105, max tps: 10.41565034618958, count: 57924"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.458278345564782,
            "unit": "median tps",
            "extra": "avg tps: 4.921704755127705, max tps: 6.0683243679191134, count: 57924"
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
          "id": "553484d76445895df38c2d1102f1a6e9b3b6fbf8",
          "message": "fix: Mutable segment corruption when reading beyond number of entries (#3624)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nUnder physical replication, we've observed a rare issue where entries of\nthe mutable segment fail to deserialize.\n\nThis always seems to happen when we are reading beyond the actual length\nof the mutable segment list. For instance, the mutable segment list only\ncontains 400 entries, but we try and deserialize entry 401.\n\nI don't yet have a perfect theory for why this is happening, but\nstopping the reading of the merge segment list when we've reached the\nnumber of entries seems to be working as a stopgap.\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-11-24T21:19:04-05:00",
          "tree_id": "15acd09158f6c1da87843db016cc4d76c3c2a3c1",
          "url": "https://github.com/paradedb/paradedb/commit/553484d76445895df38c2d1102f1a6e9b3b6fbf8"
        },
        "date": 1764038878882,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.094322885401565,
            "unit": "median tps",
            "extra": "avg tps: 6.881065186882116, max tps: 10.532150169894072, count: 57910"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.467532314756039,
            "unit": "median tps",
            "extra": "avg tps: 4.941243583027143, max tps: 6.101202133800354, count: 57910"
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
          "id": "316302d369db580ef5701cbc6fcf1abf8cfb5da4",
          "message": "chore: Upgrade to `0.19.10` (#3626)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-11-24T22:54:13-05:00",
          "tree_id": "7bac085a424878b8a62885db4e53baf3b8ef58e1",
          "url": "https://github.com/paradedb/paradedb/commit/316302d369db580ef5701cbc6fcf1abf8cfb5da4"
        },
        "date": 1764044592710,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.041565596876518,
            "unit": "median tps",
            "extra": "avg tps: 6.861407008223274, max tps: 10.556698421343912, count: 57904"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.5074474448610165,
            "unit": "median tps",
            "extra": "avg tps: 4.966816003328292, max tps: 6.1290744923864935, count: 57904"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mdashti@gmail.com",
            "name": "Moe",
            "username": "mdashti"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "555dbffd8e840906924893fe14ba577711ca9a30",
          "message": "feat: enable TopN optimization for LEFT JOIN LATERAL queries (#3590)\n\n# Ticket(s) Closed\n\n- Closes #3239\n\n## What\n\nEnables TopN optimization for `LEFT JOIN LATERAL` queries, allowing\nefficient execution of queries that combine lateral joins with `ORDER\nBY` and `LIMIT` clauses.\n\n## Why\n\nPreviously, `LEFT JOIN LATERAL` queries would default to a Normal scan\neven when they could benefit from TopN optimization. This was due to:\n1. The LIMIT from joined relations not being extracted\n2. The `paradedb.score()` function being wrapped in `PlaceHolderVar`\nduring joins, preventing proper pathkey extraction\n\nThis resulted in suboptimal performance for common query patterns like\nfetching the latest comment for each article.\n\n## How\n\n- Added `is_left_join_lateral()` to detect LEFT JOIN LATERAL patterns in\nthe query tree\n- Added `where_clause_only_references_left()` to ensure WHERE clauses\nonly reference the driving (left) table\n- Added `extract_funcexpr_from_placeholder()` to unwrap score functions\nfrom PlaceHolderVar nodes\n- Used `contains_lateral_reference()` for recursive detection of LATERAL\nin nested joins\n- Modified `create_custom_path()` to extract LIMIT for LEFT JOIN LATERAL\nqueries when conditions are met\n- Updated pathkey extraction to handle PlaceHolderVar-wrapped score\nfunctions\n\nThe optimization applies when:\n- The query uses LEFT JOIN LATERAL\n- The WHERE clause only references the left table\n- ORDER BY columns are from the left table and are indexed/fast fields\n- A LIMIT clause is present\n\n## Tests\n\nAdded regression tests in `lateral-join.sql`.",
          "timestamp": "2025-11-25T01:19:32-08:00",
          "tree_id": "ae5b3b7155e76725f727456e012ee1875f64b665",
          "url": "https://github.com/paradedb/paradedb/commit/555dbffd8e840906924893fe14ba577711ca9a30"
        },
        "date": 1764064121399,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.123841929500877,
            "unit": "median tps",
            "extra": "avg tps: 6.9406936208246615, max tps: 10.653344530362176, count: 57591"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.335605880347964,
            "unit": "median tps",
            "extra": "avg tps: 4.7827032939377405, max tps: 5.965546272990113, count: 57591"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mdashti@gmail.com",
            "name": "Moe",
            "username": "mdashti"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "1478d17163b3a319c78213f973161105f4fe8172",
          "message": "feat: `pdb.agg()` support for wrapped functions and CTEs (#3588)\n\n## What\n\nFixes `pdb.agg()` to work correctly when:\n- Wrapped in other functions (e.g., `jsonb_pretty(pdb.agg(...))`)\n- Used inside Common Table Expressions (CTEs)\n- Used in subqueries\n\nAlso removes the `SUBQUERY_SUPPORT` feature flag - subqueries and CTEs\nare now always supported.\n\n- Closes #3504\n\n## Why\n\nPreviously, `pdb.agg()` would fail with errors like:\n- \"pdb.agg() must be handled by ParadeDB's custom scan\"\n- \"window_agg placeholder should not be executed\"\n\nThis happened because the planner hook only checked top-level\nexpressions and didn't recursively process CTEs or nested function\ncalls.\n\n## How\n\n**Planning stage:**\n- Added recursive CTE and subquery processing to the planner hook\n- Implemented `replace_in_node()` to walk expression trees and find\n`WindowFunc` nodes even when wrapped in other functions\n- Split `pdb.agg()` detection into two helpers:\n  - `query_has_paradedb_agg()` - recursive check for feature enablement\n- `query_has_paradedb_agg_at_current_level()` - non-recursive check for\nper-level validation\n\n**Execution stage:**\n- Implemented `replace_window_agg_with_const()` to recursively find and\nreplace `window_agg()` placeholders with `Const` nodes, even when\nwrapped\n- Updated `inject_window_aggregate_placeholders()` to handle nested\nexpressions\n\n**Detection:**\n- Changed `extract_and_convert_window_functions()` to use\n`expression_tree_walker` instead of only checking top-level nodes\n\n## Tests\n\nAdded regression tests in `fn_wrapped_agg.sql` covering:\n1. Basic `pdb.agg()` in TopN queries\n2. `pdb.agg()` wrapped in `jsonb_pretty()`\n3. `pdb.agg()` inside CTEs\n4. `pdb.agg()` in CTEs with outer function wrapping",
          "timestamp": "2025-11-25T09:13:06-08:00",
          "tree_id": "2564b4cb5637090acfaaf73c71de54b14cbe6a19",
          "url": "https://github.com/paradedb/paradedb/commit/1478d17163b3a319c78213f973161105f4fe8172"
        },
        "date": 1764092960126,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 7.760366848876833,
            "unit": "median tps",
            "extra": "avg tps: 6.628940236894814, max tps: 10.132284115182449, count: 57312"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.608310548952761,
            "unit": "median tps",
            "extra": "avg tps: 5.045090874005062, max tps: 6.252177643632038, count: 57312"
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
          "id": "799b8f38d6953f1bb21fce531ac87cb9f4656d8c",
          "message": "fix: Skip flaky edge case numeric values in json_pushdown prop tests (#3621)\n\n## Summary\n- Skip edge case numeric values added in PR #2978 that cause\nintermittent prop test failures on main\n\n## Test plan\n- [x] Verify prop tests pass without flaky failures\n\nCo-authored-by: Mithun Chicklore Yogendra <mithun.cy@gmail.com>",
          "timestamp": "2025-11-24T14:05:46-08:00",
          "tree_id": "7b6eed5a50c508b34c97163193ded91671efa67f",
          "url": "https://github.com/paradedb/paradedb/commit/799b8f38d6953f1bb21fce531ac87cb9f4656d8c"
        },
        "date": 1764023684468,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.010548,
            "unit": "median cpu",
            "extra": "avg cpu: 19.92911921122951, max cpu: 42.687748, count: 57746"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 229.19921875,
            "unit": "median mem",
            "extra": "avg mem: 229.12106013024712, max mem: 230.35546875, count: 57746"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.27837,
            "unit": "median cpu",
            "extra": "avg cpu: 22.46257273377845, max cpu: 33.20158, count: 57746"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 168.41796875,
            "unit": "median mem",
            "extra": "avg mem: 168.32285576327797, max mem: 168.8203125, count: 57746"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 35118,
            "unit": "median block_count",
            "extra": "avg block_count: 34091.13796626606, max block_count: 37182.0, count: 57746"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 83,
            "unit": "median segment_count",
            "extra": "avg segment_count: 85.04544037682264, max segment_count: 138.0, count: 57746"
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
          "id": "6bd02ab4267eaf048ba63da91b81c4415e153ea2",
          "message": "fix: Mutable segment corruption when reading beyond number of entries (#3618)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nUnder physical replication, we've observed a rare issue where entries of\nthe mutable segment fail to deserialize.\n\nThis always seems to happen when we are reading beyond the actual length\nof the mutable segment list. For instance, the mutable segment list only\ncontains 400 entries, but we try and deserialize entry 401.\n\nI don't yet have a perfect theory for why this is happening, but\nstopping the reading of the merge segment list when we've reached the\nnumber of entries seems to be working as a stopgap.\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-11-24T20:44:24-05:00",
          "tree_id": "db68d25211a34973b28339f0956e1982636fbffe",
          "url": "https://github.com/paradedb/paradedb/commit/6bd02ab4267eaf048ba63da91b81c4415e153ea2"
        },
        "date": 1764036801768,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.076923,
            "unit": "median cpu",
            "extra": "avg cpu: 20.255076465116808, max cpu: 43.286575, count: 57924"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 228.96484375,
            "unit": "median mem",
            "extra": "avg mem: 228.94772635802948, max mem: 230.11328125, count: 57924"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.255816,
            "unit": "median cpu",
            "extra": "avg cpu: 22.31989511881141, max cpu: 33.168808, count: 57924"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 167.8359375,
            "unit": "median mem",
            "extra": "avg mem: 168.01641766042573, max mem: 169.2421875, count: 57924"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 35108,
            "unit": "median block_count",
            "extra": "avg block_count: 34064.04611214695, max block_count: 37177.0, count: 57924"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 83,
            "unit": "median segment_count",
            "extra": "avg segment_count: 84.65717146605897, max segment_count: 136.0, count: 57924"
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
          "id": "553484d76445895df38c2d1102f1a6e9b3b6fbf8",
          "message": "fix: Mutable segment corruption when reading beyond number of entries (#3624)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nUnder physical replication, we've observed a rare issue where entries of\nthe mutable segment fail to deserialize.\n\nThis always seems to happen when we are reading beyond the actual length\nof the mutable segment list. For instance, the mutable segment list only\ncontains 400 entries, but we try and deserialize entry 401.\n\nI don't yet have a perfect theory for why this is happening, but\nstopping the reading of the merge segment list when we've reached the\nnumber of entries seems to be working as a stopgap.\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-11-24T21:19:04-05:00",
          "tree_id": "15acd09158f6c1da87843db016cc4d76c3c2a3c1",
          "url": "https://github.com/paradedb/paradedb/commit/553484d76445895df38c2d1102f1a6e9b3b6fbf8"
        },
        "date": 1764038882245,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.076923,
            "unit": "median cpu",
            "extra": "avg cpu: 20.339426941447304, max cpu: 42.64561, count: 57910"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 228.70703125,
            "unit": "median mem",
            "extra": "avg mem: 228.72427588337507, max mem: 229.89453125, count: 57910"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.255816,
            "unit": "median cpu",
            "extra": "avg cpu: 22.399817252200588, max cpu: 33.168808, count: 57910"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 166.30078125,
            "unit": "median mem",
            "extra": "avg mem: 166.30182003863754, max mem: 167.34765625, count: 57910"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 35438,
            "unit": "median block_count",
            "extra": "avg block_count: 34179.83327577275, max block_count: 37185.0, count: 57910"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 83,
            "unit": "median segment_count",
            "extra": "avg segment_count: 84.88249007079952, max segment_count: 137.0, count: 57910"
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
          "id": "316302d369db580ef5701cbc6fcf1abf8cfb5da4",
          "message": "chore: Upgrade to `0.19.10` (#3626)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-11-24T22:54:13-05:00",
          "tree_id": "7bac085a424878b8a62885db4e53baf3b8ef58e1",
          "url": "https://github.com/paradedb/paradedb/commit/316302d369db580ef5701cbc6fcf1abf8cfb5da4"
        },
        "date": 1764044596029,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.099133,
            "unit": "median cpu",
            "extra": "avg cpu: 20.18963148264933, max cpu: 42.64561, count: 57904"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 228.890625,
            "unit": "median mem",
            "extra": "avg mem: 228.89561919027614, max mem: 230.265625, count: 57904"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.323614,
            "unit": "median cpu",
            "extra": "avg cpu: 22.46239196523939, max cpu: 33.333336, count: 57904"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 168.546875,
            "unit": "median mem",
            "extra": "avg mem: 168.538195696217, max mem: 169.296875, count: 57904"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 35185,
            "unit": "median block_count",
            "extra": "avg block_count: 34128.648746200604, max block_count: 37113.0, count: 57904"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 83,
            "unit": "median segment_count",
            "extra": "avg segment_count: 84.93287164962697, max segment_count: 136.0, count: 57904"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mdashti@gmail.com",
            "name": "Moe",
            "username": "mdashti"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "555dbffd8e840906924893fe14ba577711ca9a30",
          "message": "feat: enable TopN optimization for LEFT JOIN LATERAL queries (#3590)\n\n# Ticket(s) Closed\n\n- Closes #3239\n\n## What\n\nEnables TopN optimization for `LEFT JOIN LATERAL` queries, allowing\nefficient execution of queries that combine lateral joins with `ORDER\nBY` and `LIMIT` clauses.\n\n## Why\n\nPreviously, `LEFT JOIN LATERAL` queries would default to a Normal scan\neven when they could benefit from TopN optimization. This was due to:\n1. The LIMIT from joined relations not being extracted\n2. The `paradedb.score()` function being wrapped in `PlaceHolderVar`\nduring joins, preventing proper pathkey extraction\n\nThis resulted in suboptimal performance for common query patterns like\nfetching the latest comment for each article.\n\n## How\n\n- Added `is_left_join_lateral()` to detect LEFT JOIN LATERAL patterns in\nthe query tree\n- Added `where_clause_only_references_left()` to ensure WHERE clauses\nonly reference the driving (left) table\n- Added `extract_funcexpr_from_placeholder()` to unwrap score functions\nfrom PlaceHolderVar nodes\n- Used `contains_lateral_reference()` for recursive detection of LATERAL\nin nested joins\n- Modified `create_custom_path()` to extract LIMIT for LEFT JOIN LATERAL\nqueries when conditions are met\n- Updated pathkey extraction to handle PlaceHolderVar-wrapped score\nfunctions\n\nThe optimization applies when:\n- The query uses LEFT JOIN LATERAL\n- The WHERE clause only references the left table\n- ORDER BY columns are from the left table and are indexed/fast fields\n- A LIMIT clause is present\n\n## Tests\n\nAdded regression tests in `lateral-join.sql`.",
          "timestamp": "2025-11-25T01:19:32-08:00",
          "tree_id": "ae5b3b7155e76725f727456e012ee1875f64b665",
          "url": "https://github.com/paradedb/paradedb/commit/555dbffd8e840906924893fe14ba577711ca9a30"
        },
        "date": 1764064124863,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.010548,
            "unit": "median cpu",
            "extra": "avg cpu: 19.91895363189571, max cpu: 42.772278, count: 57591"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 236.5859375,
            "unit": "median mem",
            "extra": "avg mem: 235.8451861781789, max mem: 237.91015625, count: 57591"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.27837,
            "unit": "median cpu",
            "extra": "avg cpu: 22.480886727118374, max cpu: 33.20158, count: 57591"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 167.33203125,
            "unit": "median mem",
            "extra": "avg mem: 167.30460522531732, max mem: 168.6484375, count: 57591"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 35242,
            "unit": "median block_count",
            "extra": "avg block_count: 34262.187824486464, max block_count: 37380.0, count: 57591"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 83,
            "unit": "median segment_count",
            "extra": "avg segment_count: 85.11375041239083, max segment_count: 135.0, count: 57591"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mdashti@gmail.com",
            "name": "Moe",
            "username": "mdashti"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "1478d17163b3a319c78213f973161105f4fe8172",
          "message": "feat: `pdb.agg()` support for wrapped functions and CTEs (#3588)\n\n## What\n\nFixes `pdb.agg()` to work correctly when:\n- Wrapped in other functions (e.g., `jsonb_pretty(pdb.agg(...))`)\n- Used inside Common Table Expressions (CTEs)\n- Used in subqueries\n\nAlso removes the `SUBQUERY_SUPPORT` feature flag - subqueries and CTEs\nare now always supported.\n\n- Closes #3504\n\n## Why\n\nPreviously, `pdb.agg()` would fail with errors like:\n- \"pdb.agg() must be handled by ParadeDB's custom scan\"\n- \"window_agg placeholder should not be executed\"\n\nThis happened because the planner hook only checked top-level\nexpressions and didn't recursively process CTEs or nested function\ncalls.\n\n## How\n\n**Planning stage:**\n- Added recursive CTE and subquery processing to the planner hook\n- Implemented `replace_in_node()` to walk expression trees and find\n`WindowFunc` nodes even when wrapped in other functions\n- Split `pdb.agg()` detection into two helpers:\n  - `query_has_paradedb_agg()` - recursive check for feature enablement\n- `query_has_paradedb_agg_at_current_level()` - non-recursive check for\nper-level validation\n\n**Execution stage:**\n- Implemented `replace_window_agg_with_const()` to recursively find and\nreplace `window_agg()` placeholders with `Const` nodes, even when\nwrapped\n- Updated `inject_window_aggregate_placeholders()` to handle nested\nexpressions\n\n**Detection:**\n- Changed `extract_and_convert_window_functions()` to use\n`expression_tree_walker` instead of only checking top-level nodes\n\n## Tests\n\nAdded regression tests in `fn_wrapped_agg.sql` covering:\n1. Basic `pdb.agg()` in TopN queries\n2. `pdb.agg()` wrapped in `jsonb_pretty()`\n3. `pdb.agg()` inside CTEs\n4. `pdb.agg()` in CTEs with outer function wrapping",
          "timestamp": "2025-11-25T09:13:06-08:00",
          "tree_id": "2564b4cb5637090acfaaf73c71de54b14cbe6a19",
          "url": "https://github.com/paradedb/paradedb/commit/1478d17163b3a319c78213f973161105f4fe8172"
        },
        "date": 1764092963580,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.143684,
            "unit": "median cpu",
            "extra": "avg cpu: 20.75002671240772, max cpu: 42.72997, count: 57312"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 229.63671875,
            "unit": "median mem",
            "extra": "avg mem: 229.61956211177414, max mem: 230.77734375, count: 57312"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.27837,
            "unit": "median cpu",
            "extra": "avg cpu: 22.375104951478498, max cpu: 33.23442, count: 57312"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 168.08984375,
            "unit": "median mem",
            "extra": "avg mem: 167.600548518962, max mem: 168.08984375, count: 57312"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 35135,
            "unit": "median block_count",
            "extra": "avg block_count: 34014.83225153545, max block_count: 36689.0, count: 57312"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 82,
            "unit": "median segment_count",
            "extra": "avg segment_count: 84.05726549413735, max segment_count: 134.0, count: 57312"
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
          "id": "799b8f38d6953f1bb21fce531ac87cb9f4656d8c",
          "message": "fix: Skip flaky edge case numeric values in json_pushdown prop tests (#3621)\n\n## Summary\n- Skip edge case numeric values added in PR #2978 that cause\nintermittent prop test failures on main\n\n## Test plan\n- [x] Verify prop tests pass without flaky failures\n\nCo-authored-by: Mithun Chicklore Yogendra <mithun.cy@gmail.com>",
          "timestamp": "2025-11-24T14:05:46-08:00",
          "tree_id": "7b6eed5a50c508b34c97163193ded91671efa67f",
          "url": "https://github.com/paradedb/paradedb/commit/799b8f38d6953f1bb21fce531ac87cb9f4656d8c"
        },
        "date": 1764024424055,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 1185.4627485318538,
            "unit": "median tps",
            "extra": "avg tps: 1189.4504514870112, max tps: 1256.5814601009497, count: 56676"
          },
          {
            "name": "Single Insert - Primary - tps",
            "value": 1251.5329266529402,
            "unit": "median tps",
            "extra": "avg tps: 1239.2587409581574, max tps: 1280.7218773242978, count: 56676"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 1985.1362678590535,
            "unit": "median tps",
            "extra": "avg tps: 1957.130317080321, max tps: 2160.982680028903, count: 56676"
          },
          {
            "name": "Top N - Primary - tps",
            "value": 5.367287738238424,
            "unit": "median tps",
            "extra": "avg tps: 5.360477912614448, max tps: 6.464384337126435, count: 56676"
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
          "id": "6bd02ab4267eaf048ba63da91b81c4415e153ea2",
          "message": "fix: Mutable segment corruption when reading beyond number of entries (#3618)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nUnder physical replication, we've observed a rare issue where entries of\nthe mutable segment fail to deserialize.\n\nThis always seems to happen when we are reading beyond the actual length\nof the mutable segment list. For instance, the mutable segment list only\ncontains 400 entries, but we try and deserialize entry 401.\n\nI don't yet have a perfect theory for why this is happening, but\nstopping the reading of the merge segment list when we've reached the\nnumber of entries seems to be working as a stopgap.\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-11-24T20:44:24-05:00",
          "tree_id": "db68d25211a34973b28339f0956e1982636fbffe",
          "url": "https://github.com/paradedb/paradedb/commit/6bd02ab4267eaf048ba63da91b81c4415e153ea2"
        },
        "date": 1764037553234,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 1157.4098697125683,
            "unit": "median tps",
            "extra": "avg tps: 1159.0500525655448, max tps: 1208.7049233461369, count: 56112"
          },
          {
            "name": "Single Insert - Primary - tps",
            "value": 1335.0657429745286,
            "unit": "median tps",
            "extra": "avg tps: 1326.850033763123, max tps: 1347.690162199693, count: 56112"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 1885.6629284254082,
            "unit": "median tps",
            "extra": "avg tps: 1868.3758626438298, max tps: 2029.6681340150838, count: 56112"
          },
          {
            "name": "Top N - Primary - tps",
            "value": 5.428522429651481,
            "unit": "median tps",
            "extra": "avg tps: 5.442147042204411, max tps: 6.7767471535052906, count: 56112"
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
          "id": "553484d76445895df38c2d1102f1a6e9b3b6fbf8",
          "message": "fix: Mutable segment corruption when reading beyond number of entries (#3624)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nUnder physical replication, we've observed a rare issue where entries of\nthe mutable segment fail to deserialize.\n\nThis always seems to happen when we are reading beyond the actual length\nof the mutable segment list. For instance, the mutable segment list only\ncontains 400 entries, but we try and deserialize entry 401.\n\nI don't yet have a perfect theory for why this is happening, but\nstopping the reading of the merge segment list when we've reached the\nnumber of entries seems to be working as a stopgap.\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-11-24T21:19:04-05:00",
          "tree_id": "15acd09158f6c1da87843db016cc4d76c3c2a3c1",
          "url": "https://github.com/paradedb/paradedb/commit/553484d76445895df38c2d1102f1a6e9b3b6fbf8"
        },
        "date": 1764039624028,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 1158.0049122089,
            "unit": "median tps",
            "extra": "avg tps: 1159.3234649991646, max tps: 1211.9155631988974, count: 56054"
          },
          {
            "name": "Single Insert - Primary - tps",
            "value": 1323.3249004008549,
            "unit": "median tps",
            "extra": "avg tps: 1314.1997034726205, max tps: 1335.7815612588452, count: 56054"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 1896.876957502432,
            "unit": "median tps",
            "extra": "avg tps: 1874.2398210064553, max tps: 2036.0091045549245, count: 56054"
          },
          {
            "name": "Top N - Primary - tps",
            "value": 5.506037309699507,
            "unit": "median tps",
            "extra": "avg tps: 5.509015604284041, max tps: 6.522061816532353, count: 56054"
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
          "id": "316302d369db580ef5701cbc6fcf1abf8cfb5da4",
          "message": "chore: Upgrade to `0.19.10` (#3626)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-11-24T22:54:13-05:00",
          "tree_id": "7bac085a424878b8a62885db4e53baf3b8ef58e1",
          "url": "https://github.com/paradedb/paradedb/commit/316302d369db580ef5701cbc6fcf1abf8cfb5da4"
        },
        "date": 1764045341884,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 1147.4942911606977,
            "unit": "median tps",
            "extra": "avg tps: 1148.4512978959208, max tps: 1200.856477507431, count: 55913"
          },
          {
            "name": "Single Insert - Primary - tps",
            "value": 1301.323820969383,
            "unit": "median tps",
            "extra": "avg tps: 1295.3871383339097, max tps: 1312.6301150395918, count: 55913"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 1885.7174843584107,
            "unit": "median tps",
            "extra": "avg tps: 1862.1872416924318, max tps: 2019.3687159958974, count: 55913"
          },
          {
            "name": "Top N - Primary - tps",
            "value": 5.542892184426932,
            "unit": "median tps",
            "extra": "avg tps: 5.530442674210358, max tps: 6.747817039014022, count: 55913"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mdashti@gmail.com",
            "name": "Moe",
            "username": "mdashti"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "555dbffd8e840906924893fe14ba577711ca9a30",
          "message": "feat: enable TopN optimization for LEFT JOIN LATERAL queries (#3590)\n\n# Ticket(s) Closed\n\n- Closes #3239\n\n## What\n\nEnables TopN optimization for `LEFT JOIN LATERAL` queries, allowing\nefficient execution of queries that combine lateral joins with `ORDER\nBY` and `LIMIT` clauses.\n\n## Why\n\nPreviously, `LEFT JOIN LATERAL` queries would default to a Normal scan\neven when they could benefit from TopN optimization. This was due to:\n1. The LIMIT from joined relations not being extracted\n2. The `paradedb.score()` function being wrapped in `PlaceHolderVar`\nduring joins, preventing proper pathkey extraction\n\nThis resulted in suboptimal performance for common query patterns like\nfetching the latest comment for each article.\n\n## How\n\n- Added `is_left_join_lateral()` to detect LEFT JOIN LATERAL patterns in\nthe query tree\n- Added `where_clause_only_references_left()` to ensure WHERE clauses\nonly reference the driving (left) table\n- Added `extract_funcexpr_from_placeholder()` to unwrap score functions\nfrom PlaceHolderVar nodes\n- Used `contains_lateral_reference()` for recursive detection of LATERAL\nin nested joins\n- Modified `create_custom_path()` to extract LIMIT for LEFT JOIN LATERAL\nqueries when conditions are met\n- Updated pathkey extraction to handle PlaceHolderVar-wrapped score\nfunctions\n\nThe optimization applies when:\n- The query uses LEFT JOIN LATERAL\n- The WHERE clause only references the left table\n- ORDER BY columns are from the left table and are indexed/fast fields\n- A LIMIT clause is present\n\n## Tests\n\nAdded regression tests in `lateral-join.sql`.",
          "timestamp": "2025-11-25T01:19:32-08:00",
          "tree_id": "ae5b3b7155e76725f727456e012ee1875f64b665",
          "url": "https://github.com/paradedb/paradedb/commit/555dbffd8e840906924893fe14ba577711ca9a30"
        },
        "date": 1764064875564,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 1150.8631910085794,
            "unit": "median tps",
            "extra": "avg tps: 1152.4314479416123, max tps: 1206.8185255772923, count: 55953"
          },
          {
            "name": "Single Insert - Primary - tps",
            "value": 1298.2490393625387,
            "unit": "median tps",
            "extra": "avg tps: 1282.9699559555163, max tps: 1306.617437653106, count: 55953"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 1883.4704241709464,
            "unit": "median tps",
            "extra": "avg tps: 1856.9080191249732, max tps: 2015.2915685360422, count: 55953"
          },
          {
            "name": "Top N - Primary - tps",
            "value": 5.536672866711221,
            "unit": "median tps",
            "extra": "avg tps: 5.524040330537291, max tps: 6.6997259282816986, count: 55953"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mdashti@gmail.com",
            "name": "Moe",
            "username": "mdashti"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "1478d17163b3a319c78213f973161105f4fe8172",
          "message": "feat: `pdb.agg()` support for wrapped functions and CTEs (#3588)\n\n## What\n\nFixes `pdb.agg()` to work correctly when:\n- Wrapped in other functions (e.g., `jsonb_pretty(pdb.agg(...))`)\n- Used inside Common Table Expressions (CTEs)\n- Used in subqueries\n\nAlso removes the `SUBQUERY_SUPPORT` feature flag - subqueries and CTEs\nare now always supported.\n\n- Closes #3504\n\n## Why\n\nPreviously, `pdb.agg()` would fail with errors like:\n- \"pdb.agg() must be handled by ParadeDB's custom scan\"\n- \"window_agg placeholder should not be executed\"\n\nThis happened because the planner hook only checked top-level\nexpressions and didn't recursively process CTEs or nested function\ncalls.\n\n## How\n\n**Planning stage:**\n- Added recursive CTE and subquery processing to the planner hook\n- Implemented `replace_in_node()` to walk expression trees and find\n`WindowFunc` nodes even when wrapped in other functions\n- Split `pdb.agg()` detection into two helpers:\n  - `query_has_paradedb_agg()` - recursive check for feature enablement\n- `query_has_paradedb_agg_at_current_level()` - non-recursive check for\nper-level validation\n\n**Execution stage:**\n- Implemented `replace_window_agg_with_const()` to recursively find and\nreplace `window_agg()` placeholders with `Const` nodes, even when\nwrapped\n- Updated `inject_window_aggregate_placeholders()` to handle nested\nexpressions\n\n**Detection:**\n- Changed `extract_and_convert_window_functions()` to use\n`expression_tree_walker` instead of only checking top-level nodes\n\n## Tests\n\nAdded regression tests in `fn_wrapped_agg.sql` covering:\n1. Basic `pdb.agg()` in TopN queries\n2. `pdb.agg()` wrapped in `jsonb_pretty()`\n3. `pdb.agg()` inside CTEs\n4. `pdb.agg()` in CTEs with outer function wrapping",
          "timestamp": "2025-11-25T09:13:06-08:00",
          "tree_id": "2564b4cb5637090acfaaf73c71de54b14cbe6a19",
          "url": "https://github.com/paradedb/paradedb/commit/1478d17163b3a319c78213f973161105f4fe8172"
        },
        "date": 1764093713160,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 1164.216900250769,
            "unit": "median tps",
            "extra": "avg tps: 1164.2074270614821, max tps: 1219.6011450441672, count: 56496"
          },
          {
            "name": "Single Insert - Primary - tps",
            "value": 1302.3829682267135,
            "unit": "median tps",
            "extra": "avg tps: 1290.087598841767, max tps: 1308.0575225563869, count: 56496"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 1904.2494245782855,
            "unit": "median tps",
            "extra": "avg tps: 1879.1756767182028, max tps: 2032.7014283144113, count: 56496"
          },
          {
            "name": "Top N - Primary - tps",
            "value": 5.505541026043874,
            "unit": "median tps",
            "extra": "avg tps: 5.51433974199605, max tps: 6.6064774946099485, count: 56496"
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
          "id": "799b8f38d6953f1bb21fce531ac87cb9f4656d8c",
          "message": "fix: Skip flaky edge case numeric values in json_pushdown prop tests (#3621)\n\n## Summary\n- Skip edge case numeric values added in PR #2978 that cause\nintermittent prop test failures on main\n\n## Test plan\n- [x] Verify prop tests pass without flaky failures\n\nCo-authored-by: Mithun Chicklore Yogendra <mithun.cy@gmail.com>",
          "timestamp": "2025-11-24T14:05:46-08:00",
          "tree_id": "7b6eed5a50c508b34c97163193ded91671efa67f",
          "url": "https://github.com/paradedb/paradedb/commit/799b8f38d6953f1bb21fce531ac87cb9f4656d8c"
        },
        "date": 1764024427287,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Background Merger - Primary - background_merging",
            "value": 0,
            "unit": "median background_merging",
            "extra": "avg background_merging: 0.07059425506387183, max background_merging: 2.0, count: 56676"
          },
          {
            "name": "Background Merger - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.826391875047785, max cpu: 9.648242, count: 56676"
          },
          {
            "name": "Background Merger - Primary - mem",
            "value": 23.0546875,
            "unit": "median mem",
            "extra": "avg mem: 23.025857781512457, max mem: 25.01171875, count: 56676"
          },
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.987424938898483, max cpu: 14.243324, count: 56676"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 157.33203125,
            "unit": "median mem",
            "extra": "avg mem: 156.11589906331605, max mem: 157.33203125, count: 56676"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 50972,
            "unit": "median block_count",
            "extra": "avg block_count: 50845.50892794128, max block_count: 50972.0, count: 56676"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 46,
            "unit": "median segment_count",
            "extra": "avg segment_count: 44.47792716493754, max segment_count: 62.0, count: 56676"
          },
          {
            "name": "Single Insert - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.56222024841417, max cpu: 9.523809, count: 56676"
          },
          {
            "name": "Single Insert - Primary - mem",
            "value": 121.07421875,
            "unit": "median mem",
            "extra": "avg mem: 110.32651772796245, max mem: 133.8515625, count: 56676"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 4.724600637024322, max cpu: 9.533267, count: 56676"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 158.97265625,
            "unit": "median mem",
            "extra": "avg mem: 155.69093640551114, max mem: 158.97265625, count: 56676"
          },
          {
            "name": "Top N - Primary - cpu",
            "value": 23.483368,
            "unit": "median cpu",
            "extra": "avg cpu: 23.846440240436305, max cpu: 33.432835, count: 56676"
          },
          {
            "name": "Top N - Primary - mem",
            "value": 155.83984375,
            "unit": "median mem",
            "extra": "avg mem: 175.66508089844027, max mem: 215.68359375, count: 56676"
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
          "id": "6bd02ab4267eaf048ba63da91b81c4415e153ea2",
          "message": "fix: Mutable segment corruption when reading beyond number of entries (#3618)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nUnder physical replication, we've observed a rare issue where entries of\nthe mutable segment fail to deserialize.\n\nThis always seems to happen when we are reading beyond the actual length\nof the mutable segment list. For instance, the mutable segment list only\ncontains 400 entries, but we try and deserialize entry 401.\n\nI don't yet have a perfect theory for why this is happening, but\nstopping the reading of the merge segment list when we've reached the\nnumber of entries seems to be working as a stopgap.\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-11-24T20:44:24-05:00",
          "tree_id": "db68d25211a34973b28339f0956e1982636fbffe",
          "url": "https://github.com/paradedb/paradedb/commit/6bd02ab4267eaf048ba63da91b81c4415e153ea2"
        },
        "date": 1764037556460,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Background Merger - Primary - background_merging",
            "value": 0,
            "unit": "median background_merging",
            "extra": "avg background_merging: 0.07166025092671799, max background_merging: 2.0, count: 56112"
          },
          {
            "name": "Background Merger - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.760292356785995, max cpu: 9.67742, count: 56112"
          },
          {
            "name": "Background Merger - Primary - mem",
            "value": 19.2734375,
            "unit": "median mem",
            "extra": "avg mem: 19.260456277734708, max mem: 21.22265625, count: 56112"
          },
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.99396062348754, max cpu: 14.229248, count: 56112"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 155.55078125,
            "unit": "median mem",
            "extra": "avg mem: 154.44359495295123, max mem: 155.92578125, count: 56112"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 51180,
            "unit": "median block_count",
            "extra": "avg block_count: 51060.64440761334, max block_count: 51180.0, count: 56112"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 46,
            "unit": "median segment_count",
            "extra": "avg segment_count: 43.59427573424579, max segment_count: 55.0, count: 56112"
          },
          {
            "name": "Single Insert - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.65242482270483, max cpu: 9.4395275, count: 56112"
          },
          {
            "name": "Single Insert - Primary - mem",
            "value": 117.97265625,
            "unit": "median mem",
            "extra": "avg mem: 106.8687863530751, max mem: 131.64453125, count: 56112"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.764975776897282, max cpu: 9.648242, count: 56112"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 154.65625,
            "unit": "median mem",
            "extra": "avg mem: 151.35254971363076, max mem: 155.03125, count: 56112"
          },
          {
            "name": "Top N - Primary - cpu",
            "value": 23.369036,
            "unit": "median cpu",
            "extra": "avg cpu: 23.84305135026446, max cpu: 33.136093, count: 56112"
          },
          {
            "name": "Top N - Primary - mem",
            "value": 157.16015625,
            "unit": "median mem",
            "extra": "avg mem: 175.88453881856822, max mem: 217.03125, count: 56112"
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
          "id": "553484d76445895df38c2d1102f1a6e9b3b6fbf8",
          "message": "fix: Mutable segment corruption when reading beyond number of entries (#3624)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nUnder physical replication, we've observed a rare issue where entries of\nthe mutable segment fail to deserialize.\n\nThis always seems to happen when we are reading beyond the actual length\nof the mutable segment list. For instance, the mutable segment list only\ncontains 400 entries, but we try and deserialize entry 401.\n\nI don't yet have a perfect theory for why this is happening, but\nstopping the reading of the merge segment list when we've reached the\nnumber of entries seems to be working as a stopgap.\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-11-24T21:19:04-05:00",
          "tree_id": "15acd09158f6c1da87843db016cc4d76c3c2a3c1",
          "url": "https://github.com/paradedb/paradedb/commit/553484d76445895df38c2d1102f1a6e9b3b6fbf8"
        },
        "date": 1764039627327,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Background Merger - Primary - background_merging",
            "value": 0,
            "unit": "median background_merging",
            "extra": "avg background_merging: 0.07162735933207265, max background_merging: 2.0, count: 56054"
          },
          {
            "name": "Background Merger - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.767649823409917, max cpu: 9.836065, count: 56054"
          },
          {
            "name": "Background Merger - Primary - mem",
            "value": 18.80859375,
            "unit": "median mem",
            "extra": "avg mem: 18.84787409239305, max mem: 21.61328125, count: 56054"
          },
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 4.976096019135709, max cpu: 14.45783, count: 56054"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 153.77734375,
            "unit": "median mem",
            "extra": "avg mem: 152.67163262768938, max mem: 153.77734375, count: 56054"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 51043,
            "unit": "median block_count",
            "extra": "avg block_count: 50903.14764334392, max block_count: 51043.0, count: 56054"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 46,
            "unit": "median segment_count",
            "extra": "avg segment_count: 43.14450351446819, max segment_count: 61.0, count: 56054"
          },
          {
            "name": "Single Insert - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 4.659621007432103, max cpu: 9.504951, count: 56054"
          },
          {
            "name": "Single Insert - Primary - mem",
            "value": 119.1484375,
            "unit": "median mem",
            "extra": "avg mem: 108.52950914241178, max mem: 132.83203125, count: 56054"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 4.750254805164779, max cpu: 9.667674, count: 56054"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 155.80859375,
            "unit": "median mem",
            "extra": "avg mem: 152.32673446029722, max mem: 155.80859375, count: 56054"
          },
          {
            "name": "Top N - Primary - cpu",
            "value": 23.391813,
            "unit": "median cpu",
            "extra": "avg cpu: 23.840027776263025, max cpu: 33.83686, count: 56054"
          },
          {
            "name": "Top N - Primary - mem",
            "value": 156.81640625,
            "unit": "median mem",
            "extra": "avg mem: 175.1946802971242, max mem: 216.3125, count: 56054"
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
          "id": "316302d369db580ef5701cbc6fcf1abf8cfb5da4",
          "message": "chore: Upgrade to `0.19.10` (#3626)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-11-24T22:54:13-05:00",
          "tree_id": "7bac085a424878b8a62885db4e53baf3b8ef58e1",
          "url": "https://github.com/paradedb/paradedb/commit/316302d369db580ef5701cbc6fcf1abf8cfb5da4"
        },
        "date": 1764045345191,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Background Merger - Primary - background_merging",
            "value": 0,
            "unit": "median background_merging",
            "extra": "avg background_merging: 0.07223722568991112, max background_merging: 2.0, count: 55913"
          },
          {
            "name": "Background Merger - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.819824699905751, max cpu: 9.599999, count: 55913"
          },
          {
            "name": "Background Merger - Primary - mem",
            "value": 19.453125,
            "unit": "median mem",
            "extra": "avg mem: 19.502106648945684, max mem: 22.12890625, count: 55913"
          },
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 5.046311851293332, max cpu: 13.953489, count: 55913"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 154.390625,
            "unit": "median mem",
            "extra": "avg mem: 153.160301285591, max mem: 154.765625, count: 55913"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 51409,
            "unit": "median block_count",
            "extra": "avg block_count: 51273.44182927047, max block_count: 51409.0, count: 55913"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 46,
            "unit": "median segment_count",
            "extra": "avg segment_count: 43.78362813656931, max segment_count: 56.0, count: 55913"
          },
          {
            "name": "Single Insert - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.592118033926352, max cpu: 9.467456, count: 55913"
          },
          {
            "name": "Single Insert - Primary - mem",
            "value": 122.125,
            "unit": "median mem",
            "extra": "avg mem: 109.61850770113837, max mem: 134.67578125, count: 55913"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.696206870908204, max cpu: 9.599999, count: 55913"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 153.00390625,
            "unit": "median mem",
            "extra": "avg mem: 149.86248634876952, max mem: 153.75390625, count: 55913"
          },
          {
            "name": "Top N - Primary - cpu",
            "value": 23.323614,
            "unit": "median cpu",
            "extra": "avg cpu: 23.90212791359234, max cpu: 33.20158, count: 55913"
          },
          {
            "name": "Top N - Primary - mem",
            "value": 156.58984375,
            "unit": "median mem",
            "extra": "avg mem: 175.3069636365872, max mem: 216.4140625, count: 55913"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mdashti@gmail.com",
            "name": "Moe",
            "username": "mdashti"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "555dbffd8e840906924893fe14ba577711ca9a30",
          "message": "feat: enable TopN optimization for LEFT JOIN LATERAL queries (#3590)\n\n# Ticket(s) Closed\n\n- Closes #3239\n\n## What\n\nEnables TopN optimization for `LEFT JOIN LATERAL` queries, allowing\nefficient execution of queries that combine lateral joins with `ORDER\nBY` and `LIMIT` clauses.\n\n## Why\n\nPreviously, `LEFT JOIN LATERAL` queries would default to a Normal scan\neven when they could benefit from TopN optimization. This was due to:\n1. The LIMIT from joined relations not being extracted\n2. The `paradedb.score()` function being wrapped in `PlaceHolderVar`\nduring joins, preventing proper pathkey extraction\n\nThis resulted in suboptimal performance for common query patterns like\nfetching the latest comment for each article.\n\n## How\n\n- Added `is_left_join_lateral()` to detect LEFT JOIN LATERAL patterns in\nthe query tree\n- Added `where_clause_only_references_left()` to ensure WHERE clauses\nonly reference the driving (left) table\n- Added `extract_funcexpr_from_placeholder()` to unwrap score functions\nfrom PlaceHolderVar nodes\n- Used `contains_lateral_reference()` for recursive detection of LATERAL\nin nested joins\n- Modified `create_custom_path()` to extract LIMIT for LEFT JOIN LATERAL\nqueries when conditions are met\n- Updated pathkey extraction to handle PlaceHolderVar-wrapped score\nfunctions\n\nThe optimization applies when:\n- The query uses LEFT JOIN LATERAL\n- The WHERE clause only references the left table\n- ORDER BY columns are from the left table and are indexed/fast fields\n- A LIMIT clause is present\n\n## Tests\n\nAdded regression tests in `lateral-join.sql`.",
          "timestamp": "2025-11-25T01:19:32-08:00",
          "tree_id": "ae5b3b7155e76725f727456e012ee1875f64b665",
          "url": "https://github.com/paradedb/paradedb/commit/555dbffd8e840906924893fe14ba577711ca9a30"
        },
        "date": 1764064878993,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Background Merger - Primary - background_merging",
            "value": 0,
            "unit": "median background_merging",
            "extra": "avg background_merging: 0.07238217789930834, max background_merging: 2.0, count: 55953"
          },
          {
            "name": "Background Merger - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.80626257460106, max cpu: 9.514371, count: 55953"
          },
          {
            "name": "Background Merger - Primary - mem",
            "value": 21.80859375,
            "unit": "median mem",
            "extra": "avg mem: 21.851548397762407, max mem: 24.07421875, count: 55953"
          },
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 5.009727236240179, max cpu: 9.667674, count: 55953"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 153.9765625,
            "unit": "median mem",
            "extra": "avg mem: 152.55129919307277, max mem: 154.3515625, count: 55953"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 51421,
            "unit": "median block_count",
            "extra": "avg block_count: 51294.29598055511, max block_count: 51421.0, count: 55953"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 46,
            "unit": "median segment_count",
            "extra": "avg segment_count: 43.87939878111987, max segment_count: 55.0, count: 55953"
          },
          {
            "name": "Single Insert - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.623003809899985, max cpu: 9.486166, count: 55953"
          },
          {
            "name": "Single Insert - Primary - mem",
            "value": 119.71484375,
            "unit": "median mem",
            "extra": "avg mem: 109.38166414501903, max mem: 134.3515625, count: 55953"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.719382783045786, max cpu: 9.667674, count: 55953"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 155.15625,
            "unit": "median mem",
            "extra": "avg mem: 151.44400874115328, max mem: 155.15625, count: 55953"
          },
          {
            "name": "Top N - Primary - cpu",
            "value": 23.391813,
            "unit": "median cpu",
            "extra": "avg cpu: 24.003067894483664, max cpu: 33.802814, count: 55953"
          },
          {
            "name": "Top N - Primary - mem",
            "value": 157.3359375,
            "unit": "median mem",
            "extra": "avg mem: 176.19344583847158, max mem: 216.8125, count: 55953"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mdashti@gmail.com",
            "name": "Moe",
            "username": "mdashti"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "1478d17163b3a319c78213f973161105f4fe8172",
          "message": "feat: `pdb.agg()` support for wrapped functions and CTEs (#3588)\n\n## What\n\nFixes `pdb.agg()` to work correctly when:\n- Wrapped in other functions (e.g., `jsonb_pretty(pdb.agg(...))`)\n- Used inside Common Table Expressions (CTEs)\n- Used in subqueries\n\nAlso removes the `SUBQUERY_SUPPORT` feature flag - subqueries and CTEs\nare now always supported.\n\n- Closes #3504\n\n## Why\n\nPreviously, `pdb.agg()` would fail with errors like:\n- \"pdb.agg() must be handled by ParadeDB's custom scan\"\n- \"window_agg placeholder should not be executed\"\n\nThis happened because the planner hook only checked top-level\nexpressions and didn't recursively process CTEs or nested function\ncalls.\n\n## How\n\n**Planning stage:**\n- Added recursive CTE and subquery processing to the planner hook\n- Implemented `replace_in_node()` to walk expression trees and find\n`WindowFunc` nodes even when wrapped in other functions\n- Split `pdb.agg()` detection into two helpers:\n  - `query_has_paradedb_agg()` - recursive check for feature enablement\n- `query_has_paradedb_agg_at_current_level()` - non-recursive check for\nper-level validation\n\n**Execution stage:**\n- Implemented `replace_window_agg_with_const()` to recursively find and\nreplace `window_agg()` placeholders with `Const` nodes, even when\nwrapped\n- Updated `inject_window_aggregate_placeholders()` to handle nested\nexpressions\n\n**Detection:**\n- Changed `extract_and_convert_window_functions()` to use\n`expression_tree_walker` instead of only checking top-level nodes\n\n## Tests\n\nAdded regression tests in `fn_wrapped_agg.sql` covering:\n1. Basic `pdb.agg()` in TopN queries\n2. `pdb.agg()` wrapped in `jsonb_pretty()`\n3. `pdb.agg()` inside CTEs\n4. `pdb.agg()` in CTEs with outer function wrapping",
          "timestamp": "2025-11-25T09:13:06-08:00",
          "tree_id": "2564b4cb5637090acfaaf73c71de54b14cbe6a19",
          "url": "https://github.com/paradedb/paradedb/commit/1478d17163b3a319c78213f973161105f4fe8172"
        },
        "date": 1764093716460,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Background Merger - Primary - background_merging",
            "value": 0,
            "unit": "median background_merging",
            "extra": "avg background_merging: 0.07145638629283489, max background_merging: 2.0, count: 56496"
          },
          {
            "name": "Background Merger - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.819546348251376, max cpu: 9.514371, count: 56496"
          },
          {
            "name": "Background Merger - Primary - mem",
            "value": 17.52734375,
            "unit": "median mem",
            "extra": "avg mem: 17.55305609301986, max mem: 20.125, count: 56496"
          },
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 4.951857181999158, max cpu: 14.501511, count: 56496"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 153.7890625,
            "unit": "median mem",
            "extra": "avg mem: 152.4985330819881, max mem: 153.7890625, count: 56496"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 51180,
            "unit": "median block_count",
            "extra": "avg block_count: 51052.42627796658, max block_count: 51180.0, count: 56496"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 47,
            "unit": "median segment_count",
            "extra": "avg segment_count: 44.54504743698669, max segment_count: 56.0, count: 56496"
          },
          {
            "name": "Single Insert - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.622596664892166, max cpu: 9.514371, count: 56496"
          },
          {
            "name": "Single Insert - Primary - mem",
            "value": 117.40625,
            "unit": "median mem",
            "extra": "avg mem: 106.61159480593759, max mem: 131.0, count: 56496"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.783369567937312, max cpu: 9.667674, count: 56496"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 154.2265625,
            "unit": "median mem",
            "extra": "avg mem: 150.72504420943517, max mem: 154.2265625, count: 56496"
          },
          {
            "name": "Top N - Primary - cpu",
            "value": 23.414635,
            "unit": "median cpu",
            "extra": "avg cpu: 24.04709291538006, max cpu: 33.667336, count: 56496"
          },
          {
            "name": "Top N - Primary - mem",
            "value": 157.125,
            "unit": "median mem",
            "extra": "avg mem: 176.02108604757416, max mem: 217.28125, count: 56496"
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
        "date": 1764023306694,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 31.30371219500451,
            "unit": "median tps",
            "extra": "avg tps: 31.138108055388546, max tps: 33.10494105756283, count: 55653"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 238.70193114478843,
            "unit": "median tps",
            "extra": "avg tps: 263.7647367294299, max tps: 2870.8433634997273, count: 55653"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 2030.9206254928615,
            "unit": "median tps",
            "extra": "avg tps: 2009.8095088331736, max tps: 2237.6253506097382, count: 55653"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 178.36647145493453,
            "unit": "median tps",
            "extra": "avg tps: 206.29854159116036, max tps: 1799.2750121217825, count: 111306"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 14.80934253309894,
            "unit": "median tps",
            "extra": "avg tps: 14.713892382489057, max tps: 19.356459657073216, count: 55653"
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
          "id": "799b8f38d6953f1bb21fce531ac87cb9f4656d8c",
          "message": "fix: Skip flaky edge case numeric values in json_pushdown prop tests (#3621)\n\n## Summary\n- Skip edge case numeric values added in PR #2978 that cause\nintermittent prop test failures on main\n\n## Test plan\n- [x] Verify prop tests pass without flaky failures\n\nCo-authored-by: Mithun Chicklore Yogendra <mithun.cy@gmail.com>",
          "timestamp": "2025-11-24T14:05:46-08:00",
          "tree_id": "7b6eed5a50c508b34c97163193ded91671efa67f",
          "url": "https://github.com/paradedb/paradedb/commit/799b8f38d6953f1bb21fce531ac87cb9f4656d8c"
        },
        "date": 1764025135164,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 30.677477326876748,
            "unit": "median tps",
            "extra": "avg tps: 30.5576646535103, max tps: 33.61023523351582, count: 55541"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 237.56464882647802,
            "unit": "median tps",
            "extra": "avg tps: 263.56960364714695, max tps: 2776.955576085142, count: 55541"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 2006.0710677406892,
            "unit": "median tps",
            "extra": "avg tps: 1995.7165859501536, max tps: 2366.1746655584643, count: 55541"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 177.55078179565888,
            "unit": "median tps",
            "extra": "avg tps: 206.2783881160192, max tps: 1733.7533666091829, count: 111082"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 14.272906682442402,
            "unit": "median tps",
            "extra": "avg tps: 14.496933648243903, max tps: 20.088341696746113, count: 55541"
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
          "id": "6bd02ab4267eaf048ba63da91b81c4415e153ea2",
          "message": "fix: Mutable segment corruption when reading beyond number of entries (#3618)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nUnder physical replication, we've observed a rare issue where entries of\nthe mutable segment fail to deserialize.\n\nThis always seems to happen when we are reading beyond the actual length\nof the mutable segment list. For instance, the mutable segment list only\ncontains 400 entries, but we try and deserialize entry 401.\n\nI don't yet have a perfect theory for why this is happening, but\nstopping the reading of the merge segment list when we've reached the\nnumber of entries seems to be working as a stopgap.\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-11-24T20:44:24-05:00",
          "tree_id": "db68d25211a34973b28339f0956e1982636fbffe",
          "url": "https://github.com/paradedb/paradedb/commit/6bd02ab4267eaf048ba63da91b81c4415e153ea2"
        },
        "date": 1764038255734,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 31.6460039189894,
            "unit": "median tps",
            "extra": "avg tps: 31.42480477851542, max tps: 36.91924734467815, count: 55620"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 236.71617842676818,
            "unit": "median tps",
            "extra": "avg tps: 260.010773665949, max tps: 2683.3139463900707, count: 55620"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1890.53888805519,
            "unit": "median tps",
            "extra": "avg tps: 1876.0175440829776, max tps: 2253.8966524170078, count: 55620"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 176.34151444227075,
            "unit": "median tps",
            "extra": "avg tps: 202.68393611393708, max tps: 1670.856402967766, count: 111240"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 14.474215466145667,
            "unit": "median tps",
            "extra": "avg tps: 14.622138560380266, max tps: 19.64032129680171, count: 55620"
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
          "id": "553484d76445895df38c2d1102f1a6e9b3b6fbf8",
          "message": "fix: Mutable segment corruption when reading beyond number of entries (#3624)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nUnder physical replication, we've observed a rare issue where entries of\nthe mutable segment fail to deserialize.\n\nThis always seems to happen when we are reading beyond the actual length\nof the mutable segment list. For instance, the mutable segment list only\ncontains 400 entries, but we try and deserialize entry 401.\n\nI don't yet have a perfect theory for why this is happening, but\nstopping the reading of the merge segment list when we've reached the\nnumber of entries seems to be working as a stopgap.\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-11-24T21:19:04-05:00",
          "tree_id": "15acd09158f6c1da87843db016cc4d76c3c2a3c1",
          "url": "https://github.com/paradedb/paradedb/commit/553484d76445895df38c2d1102f1a6e9b3b6fbf8"
        },
        "date": 1764040330440,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 31.27173047081895,
            "unit": "median tps",
            "extra": "avg tps: 31.337581482048716, max tps: 33.003371865305176, count: 55529"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 236.78965174671202,
            "unit": "median tps",
            "extra": "avg tps: 260.06462047648887, max tps: 2895.439632223776, count: 55529"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 2021.4707140736202,
            "unit": "median tps",
            "extra": "avg tps: 2003.8778617898884, max tps: 2249.896908700963, count: 55529"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 177.3963464907654,
            "unit": "median tps",
            "extra": "avg tps: 206.67741170993511, max tps: 1827.129098315335, count: 111058"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 14.689369094596579,
            "unit": "median tps",
            "extra": "avg tps: 14.627122317783687, max tps: 18.314902186462692, count: 55529"
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
          "id": "316302d369db580ef5701cbc6fcf1abf8cfb5da4",
          "message": "chore: Upgrade to `0.19.10` (#3626)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-11-24T22:54:13-05:00",
          "tree_id": "7bac085a424878b8a62885db4e53baf3b8ef58e1",
          "url": "https://github.com/paradedb/paradedb/commit/316302d369db580ef5701cbc6fcf1abf8cfb5da4"
        },
        "date": 1764046046595,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 28.77953929703215,
            "unit": "median tps",
            "extra": "avg tps: 28.64925173548965, max tps: 30.56124085661664, count: 55627"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 129.60427181396489,
            "unit": "median tps",
            "extra": "avg tps: 170.41222347077655, max tps: 2780.2729425811535, count: 55627"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 2011.6055277418554,
            "unit": "median tps",
            "extra": "avg tps: 2003.0864650437538, max tps: 2281.304670620343, count: 55627"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 170.39604671037358,
            "unit": "median tps",
            "extra": "avg tps: 160.60701415158638, max tps: 1847.0557949285967, count: 111254"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 12.837697628089648,
            "unit": "median tps",
            "extra": "avg tps: 13.10838955119405, max tps: 20.319332740747086, count: 55627"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mdashti@gmail.com",
            "name": "Moe",
            "username": "mdashti"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "555dbffd8e840906924893fe14ba577711ca9a30",
          "message": "feat: enable TopN optimization for LEFT JOIN LATERAL queries (#3590)\n\n# Ticket(s) Closed\n\n- Closes #3239\n\n## What\n\nEnables TopN optimization for `LEFT JOIN LATERAL` queries, allowing\nefficient execution of queries that combine lateral joins with `ORDER\nBY` and `LIMIT` clauses.\n\n## Why\n\nPreviously, `LEFT JOIN LATERAL` queries would default to a Normal scan\neven when they could benefit from TopN optimization. This was due to:\n1. The LIMIT from joined relations not being extracted\n2. The `paradedb.score()` function being wrapped in `PlaceHolderVar`\nduring joins, preventing proper pathkey extraction\n\nThis resulted in suboptimal performance for common query patterns like\nfetching the latest comment for each article.\n\n## How\n\n- Added `is_left_join_lateral()` to detect LEFT JOIN LATERAL patterns in\nthe query tree\n- Added `where_clause_only_references_left()` to ensure WHERE clauses\nonly reference the driving (left) table\n- Added `extract_funcexpr_from_placeholder()` to unwrap score functions\nfrom PlaceHolderVar nodes\n- Used `contains_lateral_reference()` for recursive detection of LATERAL\nin nested joins\n- Modified `create_custom_path()` to extract LIMIT for LEFT JOIN LATERAL\nqueries when conditions are met\n- Updated pathkey extraction to handle PlaceHolderVar-wrapped score\nfunctions\n\nThe optimization applies when:\n- The query uses LEFT JOIN LATERAL\n- The WHERE clause only references the left table\n- ORDER BY columns are from the left table and are indexed/fast fields\n- A LIMIT clause is present\n\n## Tests\n\nAdded regression tests in `lateral-join.sql`.",
          "timestamp": "2025-11-25T01:19:32-08:00",
          "tree_id": "ae5b3b7155e76725f727456e012ee1875f64b665",
          "url": "https://github.com/paradedb/paradedb/commit/555dbffd8e840906924893fe14ba577711ca9a30"
        },
        "date": 1764065585013,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 32.07778367475542,
            "unit": "median tps",
            "extra": "avg tps: 31.79710859613213, max tps: 33.916185287735985, count: 55434"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 240.85399218636115,
            "unit": "median tps",
            "extra": "avg tps: 269.2333016493629, max tps: 2834.875725389368, count: 55434"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1940.851154491345,
            "unit": "median tps",
            "extra": "avg tps: 1932.4157010884721, max tps: 2341.0835606935157, count: 55434"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 172.3563718604468,
            "unit": "median tps",
            "extra": "avg tps: 203.17109957409886, max tps: 1832.9892083134441, count: 110868"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 14.619872838038624,
            "unit": "median tps",
            "extra": "avg tps: 14.629631787932324, max tps: 18.447075727127462, count: 55434"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mdashti@gmail.com",
            "name": "Moe",
            "username": "mdashti"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "1478d17163b3a319c78213f973161105f4fe8172",
          "message": "feat: `pdb.agg()` support for wrapped functions and CTEs (#3588)\n\n## What\n\nFixes `pdb.agg()` to work correctly when:\n- Wrapped in other functions (e.g., `jsonb_pretty(pdb.agg(...))`)\n- Used inside Common Table Expressions (CTEs)\n- Used in subqueries\n\nAlso removes the `SUBQUERY_SUPPORT` feature flag - subqueries and CTEs\nare now always supported.\n\n- Closes #3504\n\n## Why\n\nPreviously, `pdb.agg()` would fail with errors like:\n- \"pdb.agg() must be handled by ParadeDB's custom scan\"\n- \"window_agg placeholder should not be executed\"\n\nThis happened because the planner hook only checked top-level\nexpressions and didn't recursively process CTEs or nested function\ncalls.\n\n## How\n\n**Planning stage:**\n- Added recursive CTE and subquery processing to the planner hook\n- Implemented `replace_in_node()` to walk expression trees and find\n`WindowFunc` nodes even when wrapped in other functions\n- Split `pdb.agg()` detection into two helpers:\n  - `query_has_paradedb_agg()` - recursive check for feature enablement\n- `query_has_paradedb_agg_at_current_level()` - non-recursive check for\nper-level validation\n\n**Execution stage:**\n- Implemented `replace_window_agg_with_const()` to recursively find and\nreplace `window_agg()` placeholders with `Const` nodes, even when\nwrapped\n- Updated `inject_window_aggregate_placeholders()` to handle nested\nexpressions\n\n**Detection:**\n- Changed `extract_and_convert_window_functions()` to use\n`expression_tree_walker` instead of only checking top-level nodes\n\n## Tests\n\nAdded regression tests in `fn_wrapped_agg.sql` covering:\n1. Basic `pdb.agg()` in TopN queries\n2. `pdb.agg()` wrapped in `jsonb_pretty()`\n3. `pdb.agg()` inside CTEs\n4. `pdb.agg()` in CTEs with outer function wrapping",
          "timestamp": "2025-11-25T09:13:06-08:00",
          "tree_id": "2564b4cb5637090acfaaf73c71de54b14cbe6a19",
          "url": "https://github.com/paradedb/paradedb/commit/1478d17163b3a319c78213f973161105f4fe8172"
        },
        "date": 1764094422635,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 31.180812441390337,
            "unit": "median tps",
            "extra": "avg tps: 31.133478771314316, max tps: 32.601267760428094, count: 55457"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 239.80460761723185,
            "unit": "median tps",
            "extra": "avg tps: 263.1589923937564, max tps: 2676.3278464195246, count: 55457"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1957.3918625193608,
            "unit": "median tps",
            "extra": "avg tps: 1953.6832017553984, max tps: 2231.0808048340828, count: 55457"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 170.8857090411135,
            "unit": "median tps",
            "extra": "avg tps: 200.5474193942084, max tps: 1695.4055272366666, count: 110914"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 14.550770678250506,
            "unit": "median tps",
            "extra": "avg tps: 14.735157176628567, max tps: 20.844439076769778, count: 55457"
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
        "date": 1764023310586,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.640776,
            "unit": "median cpu",
            "extra": "avg cpu: 19.770646901756475, max cpu: 46.28737, count: 55653"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 163.26953125,
            "unit": "median mem",
            "extra": "avg mem: 161.777742285344, max mem: 164.203125, count: 55653"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 7.7469650274950395, max cpu: 33.908947, count: 55653"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 113.82421875,
            "unit": "median mem",
            "extra": "avg mem: 112.6840678091253, max mem: 113.82421875, count: 55653"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.800813772876507, max cpu: 13.994169, count: 55653"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 126.97265625,
            "unit": "median mem",
            "extra": "avg mem: 111.09376066878694, max mem: 147.2109375, count: 55653"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 14786,
            "unit": "median block_count",
            "extra": "avg block_count: 14869.540761504322, max block_count: 26140.0, count: 55653"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.3284839294563735, max cpu: 4.7197638, count: 55653"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 98.671875,
            "unit": "median mem",
            "extra": "avg mem: 88.54641603159308, max mem: 128.6796875, count: 55653"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 26,
            "unit": "median segment_count",
            "extra": "avg segment_count: 25.95586940506352, max segment_count: 46.0, count: 55653"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.239654,
            "unit": "median cpu",
            "extra": "avg cpu: 8.479283000611298, max cpu: 33.908947, count: 111306"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 149.7578125,
            "unit": "median mem",
            "extra": "avg mem: 132.0681138876161, max mem: 154.28125, count: 111306"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.913043,
            "unit": "median cpu",
            "extra": "avg cpu: 12.684523252664778, max cpu: 23.414635, count: 55653"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 160.515625,
            "unit": "median mem",
            "extra": "avg mem: 158.70159627513343, max mem: 161.48046875, count: 55653"
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
          "id": "799b8f38d6953f1bb21fce531ac87cb9f4656d8c",
          "message": "fix: Skip flaky edge case numeric values in json_pushdown prop tests (#3621)\n\n## Summary\n- Skip edge case numeric values added in PR #2978 that cause\nintermittent prop test failures on main\n\n## Test plan\n- [x] Verify prop tests pass without flaky failures\n\nCo-authored-by: Mithun Chicklore Yogendra <mithun.cy@gmail.com>",
          "timestamp": "2025-11-24T14:05:46-08:00",
          "tree_id": "7b6eed5a50c508b34c97163193ded91671efa67f",
          "url": "https://github.com/paradedb/paradedb/commit/799b8f38d6953f1bb21fce531ac87cb9f4656d8c"
        },
        "date": 1764025138466,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.58664,
            "unit": "median cpu",
            "extra": "avg cpu: 19.861564665509693, max cpu: 42.27006, count: 55541"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 164.4375,
            "unit": "median mem",
            "extra": "avg mem: 162.49779990795088, max mem: 164.4375, count: 55541"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 7.754572002261662, max cpu: 27.934044, count: 55541"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 114.96484375,
            "unit": "median mem",
            "extra": "avg mem: 113.62350272490143, max mem: 114.96484375, count: 55541"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.797117131770044, max cpu: 9.430255, count: 55541"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 119.91796875,
            "unit": "median mem",
            "extra": "avg mem: 109.08442812798204, max mem: 146.98046875, count: 55541"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 14655,
            "unit": "median block_count",
            "extra": "avg block_count: 14746.48326461533, max block_count: 26090.0, count: 55541"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6153846,
            "unit": "median cpu",
            "extra": "avg cpu: 4.467939411081139, max cpu: 4.6647234, count: 55541"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 94.7734375,
            "unit": "median mem",
            "extra": "avg mem: 87.85343470926883, max mem: 129.28125, count: 55541"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 27,
            "unit": "median segment_count",
            "extra": "avg segment_count: 26.87801804072667, max segment_count: 45.0, count: 55541"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.221902,
            "unit": "median cpu",
            "extra": "avg cpu: 8.5065803502999, max cpu: 28.070175, count: 111082"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 151.6015625,
            "unit": "median mem",
            "extra": "avg mem: 131.96458127498155, max mem: 152.98828125, count: 111082"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.88621,
            "unit": "median cpu",
            "extra": "avg cpu: 13.069503283543252, max cpu: 27.906979, count: 55541"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 162.125,
            "unit": "median mem",
            "extra": "avg mem: 160.038319805414, max mem: 162.875, count: 55541"
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
          "id": "6bd02ab4267eaf048ba63da91b81c4415e153ea2",
          "message": "fix: Mutable segment corruption when reading beyond number of entries (#3618)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nUnder physical replication, we've observed a rare issue where entries of\nthe mutable segment fail to deserialize.\n\nThis always seems to happen when we are reading beyond the actual length\nof the mutable segment list. For instance, the mutable segment list only\ncontains 400 entries, but we try and deserialize entry 401.\n\nI don't yet have a perfect theory for why this is happening, but\nstopping the reading of the merge segment list when we've reached the\nnumber of entries seems to be working as a stopgap.\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-11-24T20:44:24-05:00",
          "tree_id": "db68d25211a34973b28339f0956e1982636fbffe",
          "url": "https://github.com/paradedb/paradedb/commit/6bd02ab4267eaf048ba63da91b81c4415e153ea2"
        },
        "date": 1764038258925,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.60465,
            "unit": "median cpu",
            "extra": "avg cpu: 19.772019621124702, max cpu: 46.376812, count: 55620"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 147.72265625,
            "unit": "median mem",
            "extra": "avg mem: 131.05085049779757, max mem: 159.375, count: 55620"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 7.7604385356953465, max cpu: 37.137333, count: 55620"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 113.00390625,
            "unit": "median mem",
            "extra": "avg mem: 111.95941397822276, max mem: 113.00390625, count: 55620"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.835707739081021, max cpu: 9.448819, count: 55620"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 116.96875,
            "unit": "median mem",
            "extra": "avg mem: 103.38442430218447, max mem: 139.99609375, count: 55620"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 14844,
            "unit": "median block_count",
            "extra": "avg block_count: 14859.515749730313, max block_count: 26097.0, count: 55620"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.592446566744967, max cpu: 4.6829267, count: 55620"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 94.72265625,
            "unit": "median mem",
            "extra": "avg mem: 85.2301468109493, max mem: 124.828125, count: 55620"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 26,
            "unit": "median segment_count",
            "extra": "avg segment_count: 25.851941747572816, max segment_count: 46.0, count: 55620"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.230769,
            "unit": "median cpu",
            "extra": "avg cpu: 8.49088296930483, max cpu: 37.137333, count: 111240"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 147.5703125,
            "unit": "median mem",
            "extra": "avg mem: 129.61170877719346, max mem: 153.14453125, count: 111240"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.899614,
            "unit": "median cpu",
            "extra": "avg cpu: 13.061892887457255, max cpu: 27.988338, count: 55620"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 162.12109375,
            "unit": "median mem",
            "extra": "avg mem: 159.61532448141406, max mem: 162.87109375, count: 55620"
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
          "id": "553484d76445895df38c2d1102f1a6e9b3b6fbf8",
          "message": "fix: Mutable segment corruption when reading beyond number of entries (#3624)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nUnder physical replication, we've observed a rare issue where entries of\nthe mutable segment fail to deserialize.\n\nThis always seems to happen when we are reading beyond the actual length\nof the mutable segment list. For instance, the mutable segment list only\ncontains 400 entries, but we try and deserialize entry 401.\n\nI don't yet have a perfect theory for why this is happening, but\nstopping the reading of the merge segment list when we've reached the\nnumber of entries seems to be working as a stopgap.\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-11-24T21:19:04-05:00",
          "tree_id": "15acd09158f6c1da87843db016cc4d76c3c2a3c1",
          "url": "https://github.com/paradedb/paradedb/commit/553484d76445895df38c2d1102f1a6e9b3b6fbf8"
        },
        "date": 1764040333750,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.60465,
            "unit": "median cpu",
            "extra": "avg cpu: 20.12123041057781, max cpu: 48.192772, count: 55529"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 156.19921875,
            "unit": "median mem",
            "extra": "avg mem: 140.5241208422626, max mem: 159.9765625, count: 55529"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 7.6642217252293126, max cpu: 41.941746, count: 55529"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 115.62109375,
            "unit": "median mem",
            "extra": "avg mem: 114.41576684592735, max mem: 115.62109375, count: 55529"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.80446394797804, max cpu: 14.035088, count: 55529"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 119.7265625,
            "unit": "median mem",
            "extra": "avg mem: 105.25551323688974, max mem: 142.27734375, count: 55529"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 14866,
            "unit": "median block_count",
            "extra": "avg block_count: 14840.43968016712, max block_count: 26112.0, count: 55529"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 3.958784975582378, max cpu: 4.7105007, count: 55529"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 93.8359375,
            "unit": "median mem",
            "extra": "avg mem: 83.75103830881162, max mem: 123.84375, count: 55529"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 26,
            "unit": "median segment_count",
            "extra": "avg segment_count: 25.692953231644726, max segment_count: 44.0, count: 55529"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.221902,
            "unit": "median cpu",
            "extra": "avg cpu: 8.489682469492195, max cpu: 41.941746, count: 111058"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 148.4375,
            "unit": "median mem",
            "extra": "avg mem: 129.94817133136513, max mem: 151.859375, count: 111058"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.899614,
            "unit": "median cpu",
            "extra": "avg cpu: 12.970354667236709, max cpu: 27.988338, count: 55529"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 162.2734375,
            "unit": "median mem",
            "extra": "avg mem: 160.35683163583442, max mem: 163.55078125, count: 55529"
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
          "id": "316302d369db580ef5701cbc6fcf1abf8cfb5da4",
          "message": "chore: Upgrade to `0.19.10` (#3626)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-11-24T22:54:13-05:00",
          "tree_id": "7bac085a424878b8a62885db4e53baf3b8ef58e1",
          "url": "https://github.com/paradedb/paradedb/commit/316302d369db580ef5701cbc6fcf1abf8cfb5da4"
        },
        "date": 1764046050231,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.622696,
            "unit": "median cpu",
            "extra": "avg cpu: 20.199622447765936, max cpu: 46.242775, count: 55627"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 164.60546875,
            "unit": "median mem",
            "extra": "avg mem: 162.55929969371888, max mem: 164.60546875, count: 55627"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 9.242586164405044, max cpu: 37.354088, count: 55627"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 119.40625,
            "unit": "median mem",
            "extra": "avg mem: 117.29575516666816, max mem: 119.81640625, count: 55627"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.815212951859044, max cpu: 13.9265, count: 55627"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 100.01953125,
            "unit": "median mem",
            "extra": "avg mem: 101.59119952990454, max mem: 148.48828125, count: 55627"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 14661,
            "unit": "median block_count",
            "extra": "avg block_count: 14689.946644615025, max block_count: 25827.0, count: 55627"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.5604556833139425, max cpu: 4.673807, count: 55627"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 81.8203125,
            "unit": "median mem",
            "extra": "avg mem: 82.63168178222806, max mem: 127.234375, count: 55627"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 27,
            "unit": "median segment_count",
            "extra": "avg segment_count: 27.098729034461684, max segment_count: 46.0, count: 55627"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.257474,
            "unit": "median cpu",
            "extra": "avg cpu: 9.41171903712285, max cpu: 37.354088, count: 111254"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 149.609375,
            "unit": "median mem",
            "extra": "avg mem: 127.93244705667887, max mem: 154.19921875, count: 111254"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.9265,
            "unit": "median cpu",
            "extra": "avg cpu: 13.584723663297183, max cpu: 27.934044, count: 55627"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 161.2421875,
            "unit": "median mem",
            "extra": "avg mem: 158.65280749737087, max mem: 161.38671875, count: 55627"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mdashti@gmail.com",
            "name": "Moe",
            "username": "mdashti"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "555dbffd8e840906924893fe14ba577711ca9a30",
          "message": "feat: enable TopN optimization for LEFT JOIN LATERAL queries (#3590)\n\n# Ticket(s) Closed\n\n- Closes #3239\n\n## What\n\nEnables TopN optimization for `LEFT JOIN LATERAL` queries, allowing\nefficient execution of queries that combine lateral joins with `ORDER\nBY` and `LIMIT` clauses.\n\n## Why\n\nPreviously, `LEFT JOIN LATERAL` queries would default to a Normal scan\neven when they could benefit from TopN optimization. This was due to:\n1. The LIMIT from joined relations not being extracted\n2. The `paradedb.score()` function being wrapped in `PlaceHolderVar`\nduring joins, preventing proper pathkey extraction\n\nThis resulted in suboptimal performance for common query patterns like\nfetching the latest comment for each article.\n\n## How\n\n- Added `is_left_join_lateral()` to detect LEFT JOIN LATERAL patterns in\nthe query tree\n- Added `where_clause_only_references_left()` to ensure WHERE clauses\nonly reference the driving (left) table\n- Added `extract_funcexpr_from_placeholder()` to unwrap score functions\nfrom PlaceHolderVar nodes\n- Used `contains_lateral_reference()` for recursive detection of LATERAL\nin nested joins\n- Modified `create_custom_path()` to extract LIMIT for LEFT JOIN LATERAL\nqueries when conditions are met\n- Updated pathkey extraction to handle PlaceHolderVar-wrapped score\nfunctions\n\nThe optimization applies when:\n- The query uses LEFT JOIN LATERAL\n- The WHERE clause only references the left table\n- ORDER BY columns are from the left table and are indexed/fast fields\n- A LIMIT clause is present\n\n## Tests\n\nAdded regression tests in `lateral-join.sql`.",
          "timestamp": "2025-11-25T01:19:32-08:00",
          "tree_id": "ae5b3b7155e76725f727456e012ee1875f64b665",
          "url": "https://github.com/paradedb/paradedb/commit/555dbffd8e840906924893fe14ba577711ca9a30"
        },
        "date": 1764065588422,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.550726,
            "unit": "median cpu",
            "extra": "avg cpu: 19.65754991702714, max cpu: 48.0, count: 55434"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 165.55078125,
            "unit": "median mem",
            "extra": "avg mem: 163.25503463296442, max mem: 165.55078125, count: 55434"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 7.709063974953644, max cpu: 27.87996, count: 55434"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 113.97265625,
            "unit": "median mem",
            "extra": "avg mem: 113.05049275940759, max mem: 114.3671875, count: 55434"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.855781397239122, max cpu: 9.4395275, count: 55434"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 119.48046875,
            "unit": "median mem",
            "extra": "avg mem: 106.03777958360392, max mem: 144.796875, count: 55434"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 14815,
            "unit": "median block_count",
            "extra": "avg block_count: 14783.427481329149, max block_count: 26184.0, count: 55434"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.408572942385849, max cpu: 4.7105007, count: 55434"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 90.23828125,
            "unit": "median mem",
            "extra": "avg mem: 83.12755977265758, max mem: 124.8828125, count: 55434"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 27,
            "unit": "median segment_count",
            "extra": "avg segment_count: 26.980733845654292, max segment_count: 47.0, count: 55434"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.213051,
            "unit": "median cpu",
            "extra": "avg cpu: 8.614378240776508, max cpu: 32.40116, count: 110868"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 149.26171875,
            "unit": "median mem",
            "extra": "avg mem: 129.77133741673657, max mem: 152.8515625, count: 110868"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.832853,
            "unit": "median cpu",
            "extra": "avg cpu: 12.472076241933154, max cpu: 27.692308, count: 55434"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 162.1015625,
            "unit": "median mem",
            "extra": "avg mem: 159.72473952695998, max mem: 163.09375, count: 55434"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mdashti@gmail.com",
            "name": "Moe",
            "username": "mdashti"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "1478d17163b3a319c78213f973161105f4fe8172",
          "message": "feat: `pdb.agg()` support for wrapped functions and CTEs (#3588)\n\n## What\n\nFixes `pdb.agg()` to work correctly when:\n- Wrapped in other functions (e.g., `jsonb_pretty(pdb.agg(...))`)\n- Used inside Common Table Expressions (CTEs)\n- Used in subqueries\n\nAlso removes the `SUBQUERY_SUPPORT` feature flag - subqueries and CTEs\nare now always supported.\n\n- Closes #3504\n\n## Why\n\nPreviously, `pdb.agg()` would fail with errors like:\n- \"pdb.agg() must be handled by ParadeDB's custom scan\"\n- \"window_agg placeholder should not be executed\"\n\nThis happened because the planner hook only checked top-level\nexpressions and didn't recursively process CTEs or nested function\ncalls.\n\n## How\n\n**Planning stage:**\n- Added recursive CTE and subquery processing to the planner hook\n- Implemented `replace_in_node()` to walk expression trees and find\n`WindowFunc` nodes even when wrapped in other functions\n- Split `pdb.agg()` detection into two helpers:\n  - `query_has_paradedb_agg()` - recursive check for feature enablement\n- `query_has_paradedb_agg_at_current_level()` - non-recursive check for\nper-level validation\n\n**Execution stage:**\n- Implemented `replace_window_agg_with_const()` to recursively find and\nreplace `window_agg()` placeholders with `Const` nodes, even when\nwrapped\n- Updated `inject_window_aggregate_placeholders()` to handle nested\nexpressions\n\n**Detection:**\n- Changed `extract_and_convert_window_functions()` to use\n`expression_tree_walker` instead of only checking top-level nodes\n\n## Tests\n\nAdded regression tests in `fn_wrapped_agg.sql` covering:\n1. Basic `pdb.agg()` in TopN queries\n2. `pdb.agg()` wrapped in `jsonb_pretty()`\n3. `pdb.agg()` inside CTEs\n4. `pdb.agg()` in CTEs with outer function wrapping",
          "timestamp": "2025-11-25T09:13:06-08:00",
          "tree_id": "2564b4cb5637090acfaaf73c71de54b14cbe6a19",
          "url": "https://github.com/paradedb/paradedb/commit/1478d17163b3a319c78213f973161105f4fe8172"
        },
        "date": 1764094426003,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.58664,
            "unit": "median cpu",
            "extra": "avg cpu: 19.936353767044466, max cpu: 46.466602, count: 55457"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 164.53515625,
            "unit": "median mem",
            "extra": "avg mem: 162.7578849097048, max mem: 165.01953125, count: 55457"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 7.726163069820268, max cpu: 27.961164, count: 55457"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 114.19921875,
            "unit": "median mem",
            "extra": "avg mem: 113.1233791636989, max mem: 114.19921875, count: 55457"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.876507330228836, max cpu: 9.448819, count: 55457"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 118.7734375,
            "unit": "median mem",
            "extra": "avg mem: 105.26263338036676, max mem: 141.59765625, count: 55457"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 14753,
            "unit": "median block_count",
            "extra": "avg block_count: 14694.964242566313, max block_count: 25843.0, count: 55457"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.545789723031009, max cpu: 4.660194, count: 55457"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 92.21484375,
            "unit": "median mem",
            "extra": "avg mem: 83.35891243722614, max mem: 122.6015625, count: 55457"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 26,
            "unit": "median segment_count",
            "extra": "avg segment_count: 25.76884793623889, max segment_count: 48.0, count: 55457"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.230769,
            "unit": "median cpu",
            "extra": "avg cpu: 8.698498627232343, max cpu: 28.290766, count: 110914"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 149.78515625,
            "unit": "median mem",
            "extra": "avg mem: 131.43787634732766, max mem: 152.90234375, count: 110914"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.872832,
            "unit": "median cpu",
            "extra": "avg cpu: 12.75029588525113, max cpu: 27.77242, count: 55457"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 161.59765625,
            "unit": "median mem",
            "extra": "avg mem: 159.13081461199667, max mem: 161.97265625, count: 55457"
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
        "date": 1764024010279,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - tps",
            "value": 152.5022687348075,
            "unit": "median tps",
            "extra": "avg tps: 198.0852514689509, max tps: 619.6428921294965, count: 53598"
          },
          {
            "name": "Index Only Scan - Subscriber - tps",
            "value": 161.0148123565702,
            "unit": "median tps",
            "extra": "avg tps: 212.40525216719817, max tps: 787.1667057451205, count: 53598"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - tps",
            "value": 76.50339722098897,
            "unit": "median tps",
            "extra": "avg tps: 77.82234103752951, max tps: 95.82919932225624, count: 53598"
          },
          {
            "name": "Top N - Subscriber - tps",
            "value": 89.29155620933024,
            "unit": "median tps",
            "extra": "avg tps: 96.6488292661769, max tps: 485.578107646273, count: 107196"
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
          "id": "799b8f38d6953f1bb21fce531ac87cb9f4656d8c",
          "message": "fix: Skip flaky edge case numeric values in json_pushdown prop tests (#3621)\n\n## Summary\n- Skip edge case numeric values added in PR #2978 that cause\nintermittent prop test failures on main\n\n## Test plan\n- [x] Verify prop tests pass without flaky failures\n\nCo-authored-by: Mithun Chicklore Yogendra <mithun.cy@gmail.com>",
          "timestamp": "2025-11-24T14:05:46-08:00",
          "tree_id": "7b6eed5a50c508b34c97163193ded91671efa67f",
          "url": "https://github.com/paradedb/paradedb/commit/799b8f38d6953f1bb21fce531ac87cb9f4656d8c"
        },
        "date": 1764025879355,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - tps",
            "value": 151.63581951123584,
            "unit": "median tps",
            "extra": "avg tps: 197.15215699735924, max tps: 706.090983181886, count: 53629"
          },
          {
            "name": "Index Only Scan - Subscriber - tps",
            "value": 158.35866828994656,
            "unit": "median tps",
            "extra": "avg tps: 208.68251512307967, max tps: 739.586231087156, count: 53629"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - tps",
            "value": 76.57906060626692,
            "unit": "median tps",
            "extra": "avg tps: 77.90901514530802, max tps: 93.38330398440611, count: 53629"
          },
          {
            "name": "Top N - Subscriber - tps",
            "value": 91.11828219781762,
            "unit": "median tps",
            "extra": "avg tps: 97.506113918912, max tps: 557.037685841524, count: 107258"
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
          "id": "6bd02ab4267eaf048ba63da91b81c4415e153ea2",
          "message": "fix: Mutable segment corruption when reading beyond number of entries (#3618)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nUnder physical replication, we've observed a rare issue where entries of\nthe mutable segment fail to deserialize.\n\nThis always seems to happen when we are reading beyond the actual length\nof the mutable segment list. For instance, the mutable segment list only\ncontains 400 entries, but we try and deserialize entry 401.\n\nI don't yet have a perfect theory for why this is happening, but\nstopping the reading of the merge segment list when we've reached the\nnumber of entries seems to be working as a stopgap.\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-11-24T20:44:24-05:00",
          "tree_id": "db68d25211a34973b28339f0956e1982636fbffe",
          "url": "https://github.com/paradedb/paradedb/commit/6bd02ab4267eaf048ba63da91b81c4415e153ea2"
        },
        "date": 1764038962857,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - tps",
            "value": 147.9569208383489,
            "unit": "median tps",
            "extra": "avg tps: 192.99124203151197, max tps: 679.2763913142829, count: 53637"
          },
          {
            "name": "Index Only Scan - Subscriber - tps",
            "value": 156.35205369208916,
            "unit": "median tps",
            "extra": "avg tps: 209.65447168940398, max tps: 757.4576593385489, count: 53637"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - tps",
            "value": 76.29745049492712,
            "unit": "median tps",
            "extra": "avg tps: 77.73465761999938, max tps: 96.00097223106347, count: 53637"
          },
          {
            "name": "Top N - Subscriber - tps",
            "value": 88.53789793839186,
            "unit": "median tps",
            "extra": "avg tps: 95.4033554429455, max tps: 497.43071508649876, count: 107274"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mdashti@gmail.com",
            "name": "Moe",
            "username": "mdashti"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "555dbffd8e840906924893fe14ba577711ca9a30",
          "message": "feat: enable TopN optimization for LEFT JOIN LATERAL queries (#3590)\n\n# Ticket(s) Closed\n\n- Closes #3239\n\n## What\n\nEnables TopN optimization for `LEFT JOIN LATERAL` queries, allowing\nefficient execution of queries that combine lateral joins with `ORDER\nBY` and `LIMIT` clauses.\n\n## Why\n\nPreviously, `LEFT JOIN LATERAL` queries would default to a Normal scan\neven when they could benefit from TopN optimization. This was due to:\n1. The LIMIT from joined relations not being extracted\n2. The `paradedb.score()` function being wrapped in `PlaceHolderVar`\nduring joins, preventing proper pathkey extraction\n\nThis resulted in suboptimal performance for common query patterns like\nfetching the latest comment for each article.\n\n## How\n\n- Added `is_left_join_lateral()` to detect LEFT JOIN LATERAL patterns in\nthe query tree\n- Added `where_clause_only_references_left()` to ensure WHERE clauses\nonly reference the driving (left) table\n- Added `extract_funcexpr_from_placeholder()` to unwrap score functions\nfrom PlaceHolderVar nodes\n- Used `contains_lateral_reference()` for recursive detection of LATERAL\nin nested joins\n- Modified `create_custom_path()` to extract LIMIT for LEFT JOIN LATERAL\nqueries when conditions are met\n- Updated pathkey extraction to handle PlaceHolderVar-wrapped score\nfunctions\n\nThe optimization applies when:\n- The query uses LEFT JOIN LATERAL\n- The WHERE clause only references the left table\n- ORDER BY columns are from the left table and are indexed/fast fields\n- A LIMIT clause is present\n\n## Tests\n\nAdded regression tests in `lateral-join.sql`.",
          "timestamp": "2025-11-25T01:19:32-08:00",
          "tree_id": "ae5b3b7155e76725f727456e012ee1875f64b665",
          "url": "https://github.com/paradedb/paradedb/commit/555dbffd8e840906924893fe14ba577711ca9a30"
        },
        "date": 1764066296302,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - tps",
            "value": 146.01731221223343,
            "unit": "median tps",
            "extra": "avg tps: 189.33309383591669, max tps: 662.4172965105126, count: 53606"
          },
          {
            "name": "Index Only Scan - Subscriber - tps",
            "value": 156.00949521166422,
            "unit": "median tps",
            "extra": "avg tps: 206.20292608754352, max tps: 749.6459205314563, count: 53606"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - tps",
            "value": 76.5822192937546,
            "unit": "median tps",
            "extra": "avg tps: 77.87136251258549, max tps: 96.73547885070845, count: 53606"
          },
          {
            "name": "Top N - Subscriber - tps",
            "value": 89.67287696762303,
            "unit": "median tps",
            "extra": "avg tps: 96.06441530360108, max tps: 535.258241559209, count: 107212"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mdashti@gmail.com",
            "name": "Moe",
            "username": "mdashti"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "1478d17163b3a319c78213f973161105f4fe8172",
          "message": "feat: `pdb.agg()` support for wrapped functions and CTEs (#3588)\n\n## What\n\nFixes `pdb.agg()` to work correctly when:\n- Wrapped in other functions (e.g., `jsonb_pretty(pdb.agg(...))`)\n- Used inside Common Table Expressions (CTEs)\n- Used in subqueries\n\nAlso removes the `SUBQUERY_SUPPORT` feature flag - subqueries and CTEs\nare now always supported.\n\n- Closes #3504\n\n## Why\n\nPreviously, `pdb.agg()` would fail with errors like:\n- \"pdb.agg() must be handled by ParadeDB's custom scan\"\n- \"window_agg placeholder should not be executed\"\n\nThis happened because the planner hook only checked top-level\nexpressions and didn't recursively process CTEs or nested function\ncalls.\n\n## How\n\n**Planning stage:**\n- Added recursive CTE and subquery processing to the planner hook\n- Implemented `replace_in_node()` to walk expression trees and find\n`WindowFunc` nodes even when wrapped in other functions\n- Split `pdb.agg()` detection into two helpers:\n  - `query_has_paradedb_agg()` - recursive check for feature enablement\n- `query_has_paradedb_agg_at_current_level()` - non-recursive check for\nper-level validation\n\n**Execution stage:**\n- Implemented `replace_window_agg_with_const()` to recursively find and\nreplace `window_agg()` placeholders with `Const` nodes, even when\nwrapped\n- Updated `inject_window_aggregate_placeholders()` to handle nested\nexpressions\n\n**Detection:**\n- Changed `extract_and_convert_window_functions()` to use\n`expression_tree_walker` instead of only checking top-level nodes\n\n## Tests\n\nAdded regression tests in `fn_wrapped_agg.sql` covering:\n1. Basic `pdb.agg()` in TopN queries\n2. `pdb.agg()` wrapped in `jsonb_pretty()`\n3. `pdb.agg()` inside CTEs\n4. `pdb.agg()` in CTEs with outer function wrapping",
          "timestamp": "2025-11-25T09:13:06-08:00",
          "tree_id": "2564b4cb5637090acfaaf73c71de54b14cbe6a19",
          "url": "https://github.com/paradedb/paradedb/commit/1478d17163b3a319c78213f973161105f4fe8172"
        },
        "date": 1764095137665,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - tps",
            "value": 142.75951763978898,
            "unit": "median tps",
            "extra": "avg tps: 187.77400805942293, max tps: 634.4614163058822, count: 53635"
          },
          {
            "name": "Index Only Scan - Subscriber - tps",
            "value": 153.27271730968286,
            "unit": "median tps",
            "extra": "avg tps: 202.53470874409797, max tps: 759.6426473865312, count: 53635"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - tps",
            "value": 77.33672143801904,
            "unit": "median tps",
            "extra": "avg tps: 78.8105113641498, max tps: 95.48565468084047, count: 53635"
          },
          {
            "name": "Top N - Subscriber - tps",
            "value": 88.83880892628119,
            "unit": "median tps",
            "extra": "avg tps: 95.9080126282879, max tps: 472.73231735410775, count: 107270"
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
        "date": 1764024013929,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - cpu",
            "value": 9.186603,
            "unit": "median cpu",
            "extra": "avg cpu: 10.404703216494823, max cpu: 23.233301, count: 53598"
          },
          {
            "name": "Custom Scan - Subscriber - mem",
            "value": 40.79296875,
            "unit": "median mem",
            "extra": "avg mem: 41.01011362947312, max mem: 45.28125, count: 53598"
          },
          {
            "name": "Delete values - Publisher - cpu",
            "value": 4.567079,
            "unit": "median cpu",
            "extra": "avg cpu: 2.699157053705018, max cpu: 4.5714283, count: 53598"
          },
          {
            "name": "Delete values - Publisher - mem",
            "value": 23.8359375,
            "unit": "median mem",
            "extra": "avg mem: 23.489283940165677, max mem: 23.8359375, count: 53598"
          },
          {
            "name": "Find by ctid - Subscriber - cpu",
            "value": 18.443804,
            "unit": "median cpu",
            "extra": "avg cpu: 19.82605851612647, max cpu: 32.43243, count: 53598"
          },
          {
            "name": "Find by ctid - Subscriber - mem",
            "value": 37.46484375,
            "unit": "median mem",
            "extra": "avg mem: 37.14508696394921, max mem: 38.23828125, count: 53598"
          },
          {
            "name": "Index Only Scan - Subscriber - cpu",
            "value": 9.17782,
            "unit": "median cpu",
            "extra": "avg cpu: 10.137796841964724, max cpu: 23.121387, count: 53598"
          },
          {
            "name": "Index Only Scan - Subscriber - mem",
            "value": 38.6484375,
            "unit": "median mem",
            "extra": "avg mem: 38.88104784401936, max mem: 41.3046875, count: 53598"
          },
          {
            "name": "Index Size Info - Subscriber - cpu",
            "value": 4.5801525,
            "unit": "median cpu",
            "extra": "avg cpu: 4.633520756810638, max cpu: 9.204219, count: 53598"
          },
          {
            "name": "Index Size Info - Subscriber - mem",
            "value": 24.09375,
            "unit": "median mem",
            "extra": "avg mem: 24.34418407636479, max mem: 26.0, count: 53598"
          },
          {
            "name": "Index Size Info - Subscriber - pages",
            "value": 835,
            "unit": "median pages",
            "extra": "avg pages: 858.5776148363744, max pages: 1395.0, count: 53598"
          },
          {
            "name": "Index Size Info - Subscriber - relation_size:MB",
            "value": 6.5234375,
            "unit": "median relation_size:MB",
            "extra": "avg relation_size:MB: 6.707637761670211, max relation_size:MB: 10.8984375, count: 53598"
          },
          {
            "name": "Index Size Info - Subscriber - segment_count",
            "value": 32,
            "unit": "median segment_count",
            "extra": "avg segment_count: 32.38020075375947, max segment_count: 63.0, count: 53598"
          },
          {
            "name": "Insert value A - Publisher - cpu",
            "value": 4.5801525,
            "unit": "median cpu",
            "extra": "avg cpu: 4.279895505717531, max cpu: 4.6065254, count: 53598"
          },
          {
            "name": "Insert value A - Publisher - mem",
            "value": 20.35546875,
            "unit": "median mem",
            "extra": "avg mem: 20.356345502630695, max mem: 20.73046875, count: 53598"
          },
          {
            "name": "Insert value B - Publisher - cpu",
            "value": 4.5540795,
            "unit": "median cpu",
            "extra": "avg cpu: 4.342595790242406, max cpu: 4.5540795, count: 53598"
          },
          {
            "name": "Insert value B - Publisher - mem",
            "value": 21.93359375,
            "unit": "median mem",
            "extra": "avg mem: 21.509201019627596, max mem: 21.93359375, count: 53598"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - cpu",
            "value": 9.186603,
            "unit": "median cpu",
            "extra": "avg cpu: 10.204555113343764, max cpu: 18.550726, count: 53598"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - mem",
            "value": 45.265625,
            "unit": "median mem",
            "extra": "avg mem: 44.88762654973133, max mem: 50.1796875, count: 53598"
          },
          {
            "name": "SELECT\n  pid,\n  pg_wal_lsn_diff(sent_lsn, replay_lsn) AS replication_lag,\n  application_name::text,\n  state::text\nFROM pg_stat_replication; - Publisher - replication_lag:MB",
            "value": 0,
            "unit": "median replication_lag:MB",
            "extra": "avg replication_lag:MB: 0.000028050174027744737, max replication_lag:MB: 0.226104736328125, count: 53598"
          },
          {
            "name": "Top N - Subscriber - cpu",
            "value": 9.151573,
            "unit": "median cpu",
            "extra": "avg cpu: 8.816039457061065, max cpu: 18.550726, count: 107196"
          },
          {
            "name": "Top N - Subscriber - mem",
            "value": 44.6875,
            "unit": "median mem",
            "extra": "avg mem: 44.467644366102746, max mem: 51.10546875, count: 107196"
          },
          {
            "name": "Update 1..9 - Publisher - cpu",
            "value": 4.5411544,
            "unit": "median cpu",
            "extra": "avg cpu: 2.8984355012595984, max cpu: 4.5933013, count: 53598"
          },
          {
            "name": "Update 1..9 - Publisher - mem",
            "value": 23.875,
            "unit": "median mem",
            "extra": "avg mem: 23.89840018517482, max mem: 24.25, count: 53598"
          },
          {
            "name": "Update 10,11 - Publisher - cpu",
            "value": 4.5757866,
            "unit": "median cpu",
            "extra": "avg cpu: 4.250219576562429, max cpu: 4.624277, count: 53598"
          },
          {
            "name": "Update 10,11 - Publisher - mem",
            "value": 24.1953125,
            "unit": "median mem",
            "extra": "avg mem: 24.136080608185008, max mem: 24.1953125, count: 53598"
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
          "id": "799b8f38d6953f1bb21fce531ac87cb9f4656d8c",
          "message": "fix: Skip flaky edge case numeric values in json_pushdown prop tests (#3621)\n\n## Summary\n- Skip edge case numeric values added in PR #2978 that cause\nintermittent prop test failures on main\n\n## Test plan\n- [x] Verify prop tests pass without flaky failures\n\nCo-authored-by: Mithun Chicklore Yogendra <mithun.cy@gmail.com>",
          "timestamp": "2025-11-24T14:05:46-08:00",
          "tree_id": "7b6eed5a50c508b34c97163193ded91671efa67f",
          "url": "https://github.com/paradedb/paradedb/commit/799b8f38d6953f1bb21fce531ac87cb9f4656d8c"
        },
        "date": 1764025882672,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - cpu",
            "value": 9.169055,
            "unit": "median cpu",
            "extra": "avg cpu: 10.613832092547725, max cpu: 23.188406, count: 53629"
          },
          {
            "name": "Custom Scan - Subscriber - mem",
            "value": 41.25390625,
            "unit": "median mem",
            "extra": "avg mem: 41.28704895555576, max mem: 46.01953125, count: 53629"
          },
          {
            "name": "Delete values - Publisher - cpu",
            "value": 0,
            "unit": "median cpu",
            "extra": "avg cpu: 2.068669645412006, max cpu: 4.5757866, count: 53629"
          },
          {
            "name": "Delete values - Publisher - mem",
            "value": 23.0546875,
            "unit": "median mem",
            "extra": "avg mem: 23.026418046905594, max mem: 23.4453125, count: 53629"
          },
          {
            "name": "Find by ctid - Subscriber - cpu",
            "value": 18.373205,
            "unit": "median cpu",
            "extra": "avg cpu: 19.39166114179703, max cpu: 32.526623, count: 53629"
          },
          {
            "name": "Find by ctid - Subscriber - mem",
            "value": 37.953125,
            "unit": "median mem",
            "extra": "avg mem: 39.21884075663354, max mem: 44.4375, count: 53629"
          },
          {
            "name": "Index Only Scan - Subscriber - cpu",
            "value": 9.169055,
            "unit": "median cpu",
            "extra": "avg cpu: 10.439647796295398, max cpu: 23.099133, count: 53629"
          },
          {
            "name": "Index Only Scan - Subscriber - mem",
            "value": 38.671875,
            "unit": "median mem",
            "extra": "avg mem: 39.186293941943724, max mem: 42.0703125, count: 53629"
          },
          {
            "name": "Index Size Info - Subscriber - cpu",
            "value": 4.5714283,
            "unit": "median cpu",
            "extra": "avg cpu: 4.654182341476778, max cpu: 9.204219, count: 53629"
          },
          {
            "name": "Index Size Info - Subscriber - mem",
            "value": 23.78515625,
            "unit": "median mem",
            "extra": "avg mem: 24.067958290407244, max mem: 26.46875, count: 53629"
          },
          {
            "name": "Index Size Info - Subscriber - pages",
            "value": 944,
            "unit": "median pages",
            "extra": "avg pages: 922.6042439724775, max pages: 1444.0, count: 53629"
          },
          {
            "name": "Index Size Info - Subscriber - relation_size:MB",
            "value": 7.375,
            "unit": "median relation_size:MB",
            "extra": "avg relation_size:MB: 7.207845656034981, max relation_size:MB: 11.28125, count: 53629"
          },
          {
            "name": "Index Size Info - Subscriber - segment_count",
            "value": 32,
            "unit": "median segment_count",
            "extra": "avg segment_count: 32.98202465084189, max segment_count: 66.0, count: 53629"
          },
          {
            "name": "Insert value A - Publisher - cpu",
            "value": 4.5714283,
            "unit": "median cpu",
            "extra": "avg cpu: 4.324914083342615, max cpu: 4.5757866, count: 53629"
          },
          {
            "name": "Insert value A - Publisher - mem",
            "value": 21.1640625,
            "unit": "median mem",
            "extra": "avg mem: 21.1056021170915, max mem: 21.5390625, count: 53629"
          },
          {
            "name": "Insert value B - Publisher - cpu",
            "value": 4.532578,
            "unit": "median cpu",
            "extra": "avg cpu: 4.219045909840106, max cpu: 4.5757866, count: 53629"
          },
          {
            "name": "Insert value B - Publisher - mem",
            "value": 21.5,
            "unit": "median mem",
            "extra": "avg mem: 21.578092077047867, max mem: 22.25, count: 53629"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - cpu",
            "value": 9.169055,
            "unit": "median cpu",
            "extra": "avg cpu: 10.410128157308042, max cpu: 18.58664, count: 53629"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - mem",
            "value": 44.69921875,
            "unit": "median mem",
            "extra": "avg mem: 44.30375473158179, max mem: 50.1171875, count: 53629"
          },
          {
            "name": "SELECT\n  pid,\n  pg_wal_lsn_diff(sent_lsn, replay_lsn) AS replication_lag,\n  application_name::text,\n  state::text\nFROM pg_stat_replication; - Publisher - replication_lag:MB",
            "value": 0,
            "unit": "median replication_lag:MB",
            "extra": "avg replication_lag:MB: 0.00002811063922861523, max replication_lag:MB: 0.24692535400390625, count: 53629"
          },
          {
            "name": "Top N - Subscriber - cpu",
            "value": 9.125476,
            "unit": "median cpu",
            "extra": "avg cpu: 8.496152587709894, max cpu: 18.568666, count: 107258"
          },
          {
            "name": "Top N - Subscriber - mem",
            "value": 45.421875,
            "unit": "median mem",
            "extra": "avg mem: 45.027803287400474, max mem: 51.8984375, count: 107258"
          },
          {
            "name": "Update 1..9 - Publisher - cpu",
            "value": 4.5845275,
            "unit": "median cpu",
            "extra": "avg cpu: 4.476100915537407, max cpu: 4.6065254, count: 53629"
          },
          {
            "name": "Update 1..9 - Publisher - mem",
            "value": 24.63671875,
            "unit": "median mem",
            "extra": "avg mem: 24.444718648608962, max mem: 24.63671875, count: 53629"
          },
          {
            "name": "Update 10,11 - Publisher - cpu",
            "value": 4.5714283,
            "unit": "median cpu",
            "extra": "avg cpu: 4.212757074816278, max cpu: 4.597701, count: 53629"
          },
          {
            "name": "Update 10,11 - Publisher - mem",
            "value": 24.59765625,
            "unit": "median mem",
            "extra": "avg mem: 24.464656992368866, max mem: 24.59765625, count: 53629"
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
          "id": "6bd02ab4267eaf048ba63da91b81c4415e153ea2",
          "message": "fix: Mutable segment corruption when reading beyond number of entries (#3618)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nUnder physical replication, we've observed a rare issue where entries of\nthe mutable segment fail to deserialize.\n\nThis always seems to happen when we are reading beyond the actual length\nof the mutable segment list. For instance, the mutable segment list only\ncontains 400 entries, but we try and deserialize entry 401.\n\nI don't yet have a perfect theory for why this is happening, but\nstopping the reading of the merge segment list when we've reached the\nnumber of entries seems to be working as a stopgap.\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-11-24T20:44:24-05:00",
          "tree_id": "db68d25211a34973b28339f0956e1982636fbffe",
          "url": "https://github.com/paradedb/paradedb/commit/6bd02ab4267eaf048ba63da91b81c4415e153ea2"
        },
        "date": 1764038966191,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - cpu",
            "value": 9.204219,
            "unit": "median cpu",
            "extra": "avg cpu: 10.554842432767396, max cpu: 23.188406, count: 53637"
          },
          {
            "name": "Custom Scan - Subscriber - mem",
            "value": 42.5390625,
            "unit": "median mem",
            "extra": "avg mem: 42.09784523742939, max mem: 45.0234375, count: 53637"
          },
          {
            "name": "Delete values - Publisher - cpu",
            "value": 4.567079,
            "unit": "median cpu",
            "extra": "avg cpu: 3.43498494849402, max cpu: 4.624277, count: 53637"
          },
          {
            "name": "Delete values - Publisher - mem",
            "value": 23.58203125,
            "unit": "median mem",
            "extra": "avg mem: 23.50625107784738, max mem: 23.58203125, count: 53637"
          },
          {
            "name": "Find by ctid - Subscriber - cpu",
            "value": 18.461538,
            "unit": "median cpu",
            "extra": "avg cpu: 19.67198660416963, max cpu: 32.43243, count: 53637"
          },
          {
            "name": "Find by ctid - Subscriber - mem",
            "value": 39.15625,
            "unit": "median mem",
            "extra": "avg mem: 39.548660730932006, max mem: 44.11328125, count: 53637"
          },
          {
            "name": "Index Only Scan - Subscriber - cpu",
            "value": 9.195402,
            "unit": "median cpu",
            "extra": "avg cpu: 10.216429004831307, max cpu: 23.121387, count: 53637"
          },
          {
            "name": "Index Only Scan - Subscriber - mem",
            "value": 38.9921875,
            "unit": "median mem",
            "extra": "avg mem: 39.21898319373753, max mem: 42.41796875, count: 53637"
          },
          {
            "name": "Index Size Info - Subscriber - cpu",
            "value": 4.58891,
            "unit": "median cpu",
            "extra": "avg cpu: 4.665443797155537, max cpu: 9.195402, count: 53637"
          },
          {
            "name": "Index Size Info - Subscriber - mem",
            "value": 24.08203125,
            "unit": "median mem",
            "extra": "avg mem: 24.29413522848034, max mem: 25.61328125, count: 53637"
          },
          {
            "name": "Index Size Info - Subscriber - pages",
            "value": 852,
            "unit": "median pages",
            "extra": "avg pages: 856.4967652926151, max pages: 1370.0, count: 53637"
          },
          {
            "name": "Index Size Info - Subscriber - relation_size:MB",
            "value": 6.65625,
            "unit": "median relation_size:MB",
            "extra": "avg relation_size:MB: 6.691381124503607, max relation_size:MB: 10.703125, count: 53637"
          },
          {
            "name": "Index Size Info - Subscriber - segment_count",
            "value": 32,
            "unit": "median segment_count",
            "extra": "avg segment_count: 32.84005444003207, max segment_count: 65.0, count: 53637"
          },
          {
            "name": "Insert value A - Publisher - cpu",
            "value": 4.5714283,
            "unit": "median cpu",
            "extra": "avg cpu: 4.459900387863766, max cpu: 4.6065254, count: 53637"
          },
          {
            "name": "Insert value A - Publisher - mem",
            "value": 21.19921875,
            "unit": "median mem",
            "extra": "avg mem: 21.099398065933965, max mem: 21.19921875, count: 53637"
          },
          {
            "name": "Insert value B - Publisher - cpu",
            "value": 4.58891,
            "unit": "median cpu",
            "extra": "avg cpu: 4.351077369729624, max cpu: 4.6021094, count: 53637"
          },
          {
            "name": "Insert value B - Publisher - mem",
            "value": 21.2109375,
            "unit": "median mem",
            "extra": "avg mem: 21.074018255821542, max mem: 21.5859375, count: 53637"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - cpu",
            "value": 9.195402,
            "unit": "median cpu",
            "extra": "avg cpu: 10.240958856994835, max cpu: 18.60465, count: 53637"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - mem",
            "value": 44.80078125,
            "unit": "median mem",
            "extra": "avg mem: 44.95828344647818, max mem: 50.78125, count: 53637"
          },
          {
            "name": "SELECT\n  pid,\n  pg_wal_lsn_diff(sent_lsn, replay_lsn) AS replication_lag,\n  application_name::text,\n  state::text\nFROM pg_stat_replication; - Publisher - replication_lag:MB",
            "value": 0,
            "unit": "median replication_lag:MB",
            "extra": "avg replication_lag:MB: 0.00001387791089799835, max replication_lag:MB: 0.0480194091796875, count: 53637"
          },
          {
            "name": "Top N - Subscriber - cpu",
            "value": 9.169055,
            "unit": "median cpu",
            "extra": "avg cpu: 8.92933515752176, max cpu: 18.58664, count: 107274"
          },
          {
            "name": "Top N - Subscriber - mem",
            "value": 46.29296875,
            "unit": "median mem",
            "extra": "avg mem: 45.68487449486828, max mem: 52.55859375, count: 107274"
          },
          {
            "name": "Update 1..9 - Publisher - cpu",
            "value": 4.58891,
            "unit": "median cpu",
            "extra": "avg cpu: 4.483451098426061, max cpu: 4.6153846, count: 53637"
          },
          {
            "name": "Update 1..9 - Publisher - mem",
            "value": 24.6484375,
            "unit": "median mem",
            "extra": "avg mem: 24.418130864072374, max mem: 24.6484375, count: 53637"
          },
          {
            "name": "Update 10,11 - Publisher - cpu",
            "value": 4.5714283,
            "unit": "median cpu",
            "extra": "avg cpu: 4.35228188634991, max cpu: 4.5801525, count: 53637"
          },
          {
            "name": "Update 10,11 - Publisher - mem",
            "value": 24.64453125,
            "unit": "median mem",
            "extra": "avg mem: 24.593989675387327, max mem: 24.64453125, count: 53637"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mdashti@gmail.com",
            "name": "Moe",
            "username": "mdashti"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "555dbffd8e840906924893fe14ba577711ca9a30",
          "message": "feat: enable TopN optimization for LEFT JOIN LATERAL queries (#3590)\n\n# Ticket(s) Closed\n\n- Closes #3239\n\n## What\n\nEnables TopN optimization for `LEFT JOIN LATERAL` queries, allowing\nefficient execution of queries that combine lateral joins with `ORDER\nBY` and `LIMIT` clauses.\n\n## Why\n\nPreviously, `LEFT JOIN LATERAL` queries would default to a Normal scan\neven when they could benefit from TopN optimization. This was due to:\n1. The LIMIT from joined relations not being extracted\n2. The `paradedb.score()` function being wrapped in `PlaceHolderVar`\nduring joins, preventing proper pathkey extraction\n\nThis resulted in suboptimal performance for common query patterns like\nfetching the latest comment for each article.\n\n## How\n\n- Added `is_left_join_lateral()` to detect LEFT JOIN LATERAL patterns in\nthe query tree\n- Added `where_clause_only_references_left()` to ensure WHERE clauses\nonly reference the driving (left) table\n- Added `extract_funcexpr_from_placeholder()` to unwrap score functions\nfrom PlaceHolderVar nodes\n- Used `contains_lateral_reference()` for recursive detection of LATERAL\nin nested joins\n- Modified `create_custom_path()` to extract LIMIT for LEFT JOIN LATERAL\nqueries when conditions are met\n- Updated pathkey extraction to handle PlaceHolderVar-wrapped score\nfunctions\n\nThe optimization applies when:\n- The query uses LEFT JOIN LATERAL\n- The WHERE clause only references the left table\n- ORDER BY columns are from the left table and are indexed/fast fields\n- A LIMIT clause is present\n\n## Tests\n\nAdded regression tests in `lateral-join.sql`.",
          "timestamp": "2025-11-25T01:19:32-08:00",
          "tree_id": "ae5b3b7155e76725f727456e012ee1875f64b665",
          "url": "https://github.com/paradedb/paradedb/commit/555dbffd8e840906924893fe14ba577711ca9a30"
        },
        "date": 1764066299652,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - cpu",
            "value": 9.239654,
            "unit": "median cpu",
            "extra": "avg cpu: 10.919600652428116, max cpu: 27.745665, count: 53606"
          },
          {
            "name": "Custom Scan - Subscriber - mem",
            "value": 41.09375,
            "unit": "median mem",
            "extra": "avg mem: 41.139555783750886, max mem: 44.3671875, count: 53606"
          },
          {
            "name": "Delete values - Publisher - cpu",
            "value": 4.5801525,
            "unit": "median cpu",
            "extra": "avg cpu: 4.52965940466658, max cpu: 4.6153846, count: 53606"
          },
          {
            "name": "Delete values - Publisher - mem",
            "value": 23.078125,
            "unit": "median mem",
            "extra": "avg mem: 22.841713730739098, max mem: 23.078125, count: 53606"
          },
          {
            "name": "Find by ctid - Subscriber - cpu",
            "value": 22.748816,
            "unit": "median cpu",
            "extra": "avg cpu: 20.25926307207682, max cpu: 32.621357, count: 53606"
          },
          {
            "name": "Find by ctid - Subscriber - mem",
            "value": 38.33984375,
            "unit": "median mem",
            "extra": "avg mem: 39.222977750858114, max mem: 44.47265625, count: 53606"
          },
          {
            "name": "Index Only Scan - Subscriber - cpu",
            "value": 9.230769,
            "unit": "median cpu",
            "extra": "avg cpu: 10.529441245522419, max cpu: 23.255816, count: 53606"
          },
          {
            "name": "Index Only Scan - Subscriber - mem",
            "value": 38.7578125,
            "unit": "median mem",
            "extra": "avg mem: 39.01009922221859, max mem: 42.19140625, count: 53606"
          },
          {
            "name": "Index Size Info - Subscriber - cpu",
            "value": 4.597701,
            "unit": "median cpu",
            "extra": "avg cpu: 4.625172264250409, max cpu: 9.320388, count: 53606"
          },
          {
            "name": "Index Size Info - Subscriber - mem",
            "value": 24.17578125,
            "unit": "median mem",
            "extra": "avg mem: 24.43694517054994, max mem: 27.2265625, count: 53606"
          },
          {
            "name": "Index Size Info - Subscriber - pages",
            "value": 910,
            "unit": "median pages",
            "extra": "avg pages: 900.3690445099429, max pages: 1411.0, count: 53606"
          },
          {
            "name": "Index Size Info - Subscriber - relation_size:MB",
            "value": 7.109375,
            "unit": "median relation_size:MB",
            "extra": "avg relation_size:MB: 7.034133524582137, max relation_size:MB: 11.0234375, count: 53606"
          },
          {
            "name": "Index Size Info - Subscriber - segment_count",
            "value": 32,
            "unit": "median segment_count",
            "extra": "avg segment_count: 32.77946498526284, max segment_count: 65.0, count: 53606"
          },
          {
            "name": "Insert value A - Publisher - cpu",
            "value": 4.5714283,
            "unit": "median cpu",
            "extra": "avg cpu: 4.044791714744159, max cpu: 4.6153846, count: 53606"
          },
          {
            "name": "Insert value A - Publisher - mem",
            "value": 21.125,
            "unit": "median mem",
            "extra": "avg mem: 21.087191732268774, max mem: 21.5, count: 53606"
          },
          {
            "name": "Insert value B - Publisher - cpu",
            "value": 4.5933013,
            "unit": "median cpu",
            "extra": "avg cpu: 4.472862606852304, max cpu: 4.6511626, count: 53606"
          },
          {
            "name": "Insert value B - Publisher - mem",
            "value": 21.9296875,
            "unit": "median mem",
            "extra": "avg mem: 21.547934451717158, max mem: 21.9296875, count: 53606"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - cpu",
            "value": 9.221902,
            "unit": "median cpu",
            "extra": "avg cpu: 10.458706138417378, max cpu: 18.640776, count: 53606"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - mem",
            "value": 45.5546875,
            "unit": "median mem",
            "extra": "avg mem: 44.800677556500204, max mem: 51.59765625, count: 53606"
          },
          {
            "name": "SELECT\n  pid,\n  pg_wal_lsn_diff(sent_lsn, replay_lsn) AS replication_lag,\n  application_name::text,\n  state::text\nFROM pg_stat_replication; - Publisher - replication_lag:MB",
            "value": 0,
            "unit": "median replication_lag:MB",
            "extra": "avg replication_lag:MB: 0.000023745540469029818, max replication_lag:MB: 0.13117218017578125, count: 53606"
          },
          {
            "name": "Top N - Subscriber - cpu",
            "value": 9.186603,
            "unit": "median cpu",
            "extra": "avg cpu: 8.831221443794764, max cpu: 18.60465, count: 107212"
          },
          {
            "name": "Top N - Subscriber - mem",
            "value": 45.55859375,
            "unit": "median mem",
            "extra": "avg mem: 45.37875533697254, max mem: 51.87109375, count: 107212"
          },
          {
            "name": "Update 1..9 - Publisher - cpu",
            "value": 4.5757866,
            "unit": "median cpu",
            "extra": "avg cpu: 3.9198114261853956, max cpu: 4.6421666, count: 53606"
          },
          {
            "name": "Update 1..9 - Publisher - mem",
            "value": 23.8515625,
            "unit": "median mem",
            "extra": "avg mem: 23.72441095096631, max mem: 24.23046875, count: 53606"
          },
          {
            "name": "Update 10,11 - Publisher - cpu",
            "value": 4.58891,
            "unit": "median cpu",
            "extra": "avg cpu: 4.259818689910014, max cpu: 4.6153846, count: 53606"
          },
          {
            "name": "Update 10,11 - Publisher - mem",
            "value": 23.0546875,
            "unit": "median mem",
            "extra": "avg mem: 23.009529454492036, max mem: 23.0546875, count: 53606"
          }
        ]
      }
    ]
  }
}