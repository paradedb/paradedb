window.BENCHMARK_DATA = {
  "lastUpdate": 1763941948413,
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
      }
    ]
  }
}