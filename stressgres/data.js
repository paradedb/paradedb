window.BENCHMARK_DATA = {
  "lastUpdate": 1770668107050,
  "repoUrl": "https://github.com/paradedb/paradedb",
  "entries": {
    "pg_search single-server.toml Performance - TPS": [
      {
        "commit": {
          "author": {
            "name": "Stu Hood",
            "username": "stuhood",
            "email": "stuhood@paradedb.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "05025da0ce66b0606497b092203538a732534d16",
          "message": "chore: Fix benchmark data destination. (#4135)\n\n## What\n\nSend all query benchmark data to the same directory.\n\n## Why\n\nIn #4080, we accidentally introduced subdirectories of our `benchmarks`\ndataset, which resulted in separate datasets and pages to render them,\nrather than a single dataset and page.\n\n<img width=\"145\" height=\"413\" alt=\"Screenshot 2026-02-08 at 5 04 35 PM\"\nsrc=\"https://github.com/user-attachments/assets/5afbcaf0-d823-4507-b0ab-36494b839661\"\n/>\n\nEach subdirectory has its own `data.js` and `index.html`, but we want it\nto be merged into the parent directory's data.",
          "timestamp": "2026-02-09T01:08:28Z",
          "url": "https://github.com/paradedb/paradedb/commit/05025da0ce66b0606497b092203538a732534d16"
        },
        "date": 1770610103055,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - tps",
            "value": 128.42890343575283,
            "unit": "median tps",
            "extra": "avg tps: 128.67567885195652, max tps: 145.51349215130887, count: 29932"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2944.7990361604075,
            "unit": "median tps",
            "extra": "avg tps: 2910.3640041353437, max tps: 2963.441071251417, count: 29932"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 482.40471811577345,
            "unit": "median tps",
            "extra": "avg tps: 480.7129173347383, max tps: 577.8264069144359, count: 29932"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 2988.746211238815,
            "unit": "median tps",
            "extra": "avg tps: 2967.6152251448634, max tps: 3023.9039985550585, count: 59864"
          },
          {
            "name": "Mixed Fast Field Scan - Primary - tps",
            "value": 505.5183063997111,
            "unit": "median tps",
            "extra": "avg tps: 506.94530566253354, max tps: 575.0349482203181, count: 29932"
          },
          {
            "name": "Normal Scan - Primary - tps",
            "value": 506.0880704253664,
            "unit": "median tps",
            "extra": "avg tps: 508.7508724190859, max tps: 606.6180980372613, count: 29932"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1892.5344427645787,
            "unit": "median tps",
            "extra": "avg tps: 1869.1192890268378, max tps: 1915.2173819296436, count: 29932"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 56.884004666833754,
            "unit": "median tps",
            "extra": "avg tps: 75.57458274653905, max tps: 676.1338934426507, count: 29932"
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
          "id": "b91006e07d5c6fc93a8bb38ca19094ef5f3081f7",
          "message": "fix: Early return in fieldname extraction causes pushdown to not happen (#4075)\n\n# Description\nBackport of #4071 to `0.21.x`.\n\n---------\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2026-02-03T20:28:51Z",
          "url": "https://github.com/paradedb/paradedb/commit/b91006e07d5c6fc93a8bb38ca19094ef5f3081f7"
        },
        "date": 1770610373414,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 576.9271273931454,
            "unit": "median tps",
            "extra": "avg tps: 576.9830043436718, max tps: 635.5297465679338, count: 55286"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3204.9024242604264,
            "unit": "median tps",
            "extra": "avg tps: 3194.961836768077, max tps: 3219.601327688294, count: 55286"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 550.1662553448517,
            "unit": "median tps",
            "extra": "avg tps: 550.3130535478572, max tps: 663.0059450013507, count: 55286"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 471.6853506083667,
            "unit": "median tps",
            "extra": "avg tps: 473.5374473483095, max tps: 524.6292800272076, count: 55286"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 3278.503369482497,
            "unit": "median tps",
            "extra": "avg tps: 3260.6341072329715, max tps: 3300.321925523087, count: 110572"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 2177.4200905084954,
            "unit": "median tps",
            "extra": "avg tps: 2164.3130756931096, max tps: 2187.8838007956565, count: 55286"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 135.1278900207408,
            "unit": "median tps",
            "extra": "avg tps: 131.43541511759994, max tps: 211.8584819238106, count: 55286"
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
          "id": "589fa838de3d99e5efefeee907cc9e85086e4d13",
          "message": "chore: create `SearchPredicateUDF` for lazy Tantivy query evaluation (#4122)\n\n## Ticket(s) Closed\n\n- Partially helps #4061 \n\n## What\n\nReplace `RowInSetUDF` with a new `SearchPredicateUDF` that carries the\nsearch query and defers execution, enabling future filter pushdown to\n`PgSearchTableProvider`.\n\n## Why\n\nThe previous `RowInSetUDF` eagerly pre-computed all matching CTIDs\nbefore join execution by running the Tantivy search upfront. This\napproach:\n- Cannot benefit from DataFusion's filter pushdown mechanism\n- Executes searches even when results might not be needed\n- Doesn't preserve expression context for EXPLAIN output\n\nThe new `SearchPredicateUDF` enables lazy evaluation and is designed to\nintegrate with DataFusion's filter pushdown, allowing single-table\npredicates to be pushed to individual table scans.\n\n## How\n\n- Created `SearchPredicateUDF` in `scan/search_predicate_udf.rs` that:\n  - Carries the search query, index OID, and heap OID\n- Stores raw pointers (`expr_ptr`, `planner_info_ptr`) for lazy deparse\nin EXPLAIN\n- Falls back to executing the search when not pushed down (cross-table\npredicates)\n- Added `RawPtr<T>` utility in `postgres/utils.rs` for type-safe\nserializable pointer handling\n- Updated `JoinLevelSearchPredicate` to store expression pointers\n- Removed eager `compute_predicate_matches` from scan_state\n- Updated translator to create `SearchPredicateUDF` instead of\n`RowInSetUDF`\n- Deleted `joinscan/udf.rs` (no longer needed)\n\n## Tests\n\n- Updated `join_custom_scan` regression test for new UDF name\n(`pdb_search_predicate` instead of `row_in_set`)\n- Added unit tests for `SearchPredicateUDF` (name, into_expr,\ntry_from_expr)",
          "timestamp": "2026-02-09T11:24:14-08:00",
          "tree_id": "7f721858975e5cac391d211ec704d17b33841d28",
          "url": "https://github.com/paradedb/paradedb/commit/589fa838de3d99e5efefeee907cc9e85086e4d13"
        },
        "date": 1770666224605,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - tps",
            "value": 122.74751665665711,
            "unit": "median tps",
            "extra": "avg tps: 122.78991712707466, max tps: 151.54773840025243, count: 55047"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3067.6667031168226,
            "unit": "median tps",
            "extra": "avg tps: 3048.1976684575848, max tps: 3099.1990718359057, count: 55047"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 460.0316132283007,
            "unit": "median tps",
            "extra": "avg tps: 458.38785285663283, max tps: 568.2568170117257, count: 55047"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 3121.254921368446,
            "unit": "median tps",
            "extra": "avg tps: 3117.307777261769, max tps: 3162.600890315277, count: 110094"
          },
          {
            "name": "Mixed Fast Field Scan - Primary - tps",
            "value": 516.1818404825675,
            "unit": "median tps",
            "extra": "avg tps: 512.1554992367344, max tps: 619.3854234443678, count: 55047"
          },
          {
            "name": "Normal Scan - Primary - tps",
            "value": 556.5972695549005,
            "unit": "median tps",
            "extra": "avg tps: 552.7509383534145, max tps: 675.3703590824006, count: 55047"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1924.6977945449914,
            "unit": "median tps",
            "extra": "avg tps: 1917.8354160735992, max tps: 1935.690414833309, count: 55047"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 34.22371173108364,
            "unit": "median tps",
            "extra": "avg tps: 68.08545761403, max tps: 696.6170187717388, count: 55047"
          }
        ]
      }
    ],
    "pg_search single-server.toml Performance - Other Metrics": [
      {
        "commit": {
          "author": {
            "name": "Stu Hood",
            "username": "stuhood",
            "email": "stuhood@paradedb.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "05025da0ce66b0606497b092203538a732534d16",
          "message": "chore: Fix benchmark data destination. (#4135)\n\n## What\n\nSend all query benchmark data to the same directory.\n\n## Why\n\nIn #4080, we accidentally introduced subdirectories of our `benchmarks`\ndataset, which resulted in separate datasets and pages to render them,\nrather than a single dataset and page.\n\n<img width=\"145\" height=\"413\" alt=\"Screenshot 2026-02-08 at 5 04 35 PM\"\nsrc=\"https://github.com/user-attachments/assets/5afbcaf0-d823-4507-b0ab-36494b839661\"\n/>\n\nEach subdirectory has its own `data.js` and `index.html`, but we want it\nto be merged into the parent directory's data.",
          "timestamp": "2026-02-09T01:08:28Z",
          "url": "https://github.com/paradedb/paradedb/commit/05025da0ce66b0606497b092203538a732534d16"
        },
        "date": 1770610108861,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - cpu",
            "value": 9.239654,
            "unit": "median cpu",
            "extra": "avg cpu: 7.936195449402945, max cpu: 22.89348, count: 29932"
          },
          {
            "name": "Aggregate Custom Scan - Primary - mem",
            "value": 58.06640625,
            "unit": "median mem",
            "extra": "avg mem: 57.77868115645463, max mem: 63.98828125, count: 29932"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.60582090797109, max cpu: 9.384164, count: 29932"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 34.66015625,
            "unit": "median mem",
            "extra": "avg mem: 34.48554692720166, max mem: 35.46875, count: 29932"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.625412881184036, max cpu: 4.738401, count: 29932"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 55.98828125,
            "unit": "median mem",
            "extra": "avg mem: 55.379200275833554, max mem: 62.37890625, count: 29932"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.653208168648449, max cpu: 9.467456, count: 59864"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 44.48046875,
            "unit": "median mem",
            "extra": "avg mem: 44.164418319545135, max mem: 50.3515625, count: 59864"
          },
          {
            "name": "Mixed Fast Field Scan - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 5.4355368070757395, max cpu: 15.130024, count: 29932"
          },
          {
            "name": "Mixed Fast Field Scan - Primary - mem",
            "value": 57.2734375,
            "unit": "median mem",
            "extra": "avg mem: 56.9986648121158, max mem: 63.36328125, count: 29932"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 1142,
            "unit": "median block_count",
            "extra": "avg block_count: 1144.8037885874649, max block_count: 1874.0, count: 29932"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 12,
            "unit": "median segment_count",
            "extra": "avg segment_count: 11.309835627422157, max segment_count: 17.0, count: 29932"
          },
          {
            "name": "Normal Scan - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 5.367413275353657, max cpu: 18.934912, count: 29932"
          },
          {
            "name": "Normal Scan - Primary - mem",
            "value": 57.11328125,
            "unit": "median mem",
            "extra": "avg mem: 56.86202593065114, max mem: 63.125, count: 29932"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.48981890434958, max cpu: 4.7244096, count: 29932"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 47.04296875,
            "unit": "median mem",
            "extra": "avg mem: 46.85442275929607, max mem: 52.91015625, count: 29932"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 0,
            "unit": "median cpu",
            "extra": "avg cpu: 0.0, max cpu: 0.0, count: 29932"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 48.6640625,
            "unit": "median mem",
            "extra": "avg mem: 47.811727676483365, max mem: 55.58984375, count: 29932"
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
          "id": "b91006e07d5c6fc93a8bb38ca19094ef5f3081f7",
          "message": "fix: Early return in fieldname extraction causes pushdown to not happen (#4075)\n\n# Description\nBackport of #4071 to `0.21.x`.\n\n---------\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2026-02-03T20:28:51Z",
          "url": "https://github.com/paradedb/paradedb/commit/b91006e07d5c6fc93a8bb38ca19094ef5f3081f7"
        },
        "date": 1770610383449,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 5.440482351599542, max cpu: 18.953604, count: 55286"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 57.6015625,
            "unit": "median mem",
            "extra": "avg mem: 57.35723760592646, max mem: 68.1796875, count: 55286"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.595994830631172, max cpu: 9.486166, count: 55286"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 33.328125,
            "unit": "median mem",
            "extra": "avg mem: 33.048295666511414, max mem: 35.33984375, count: 55286"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 5.480735821157791, max cpu: 15.2623205, count: 55286"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 57.58984375,
            "unit": "median mem",
            "extra": "avg mem: 57.32359578204699, max mem: 68.1015625, count: 55286"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.627676854264966, max cpu: 9.248554, count: 55286"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 56.91015625,
            "unit": "median mem",
            "extra": "avg mem: 56.23966199625583, max mem: 67.46484375, count: 55286"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.585491465516918, max cpu: 9.657948, count: 110572"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 46.3828125,
            "unit": "median mem",
            "extra": "avg mem: 46.10013755884175, max mem: 56.640625, count: 110572"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 1743,
            "unit": "median block_count",
            "extra": "avg block_count: 1738.4084216619035, max block_count: 3053.0, count: 55286"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 11,
            "unit": "median segment_count",
            "extra": "avg segment_count: 11.301577252830734, max segment_count: 25.0, count: 55286"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.491054652340373, max cpu: 7.5235105, count: 55286"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 48.33984375,
            "unit": "median mem",
            "extra": "avg mem: 47.19514193805846, max mem: 58.4609375, count: 55286"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 3.337313731064648, max cpu: 4.7197638, count: 55286"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 51.34375,
            "unit": "median mem",
            "extra": "avg mem: 50.72060774031853, max mem: 63.296875, count: 55286"
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
          "id": "589fa838de3d99e5efefeee907cc9e85086e4d13",
          "message": "chore: create `SearchPredicateUDF` for lazy Tantivy query evaluation (#4122)\n\n## Ticket(s) Closed\n\n- Partially helps #4061 \n\n## What\n\nReplace `RowInSetUDF` with a new `SearchPredicateUDF` that carries the\nsearch query and defers execution, enabling future filter pushdown to\n`PgSearchTableProvider`.\n\n## Why\n\nThe previous `RowInSetUDF` eagerly pre-computed all matching CTIDs\nbefore join execution by running the Tantivy search upfront. This\napproach:\n- Cannot benefit from DataFusion's filter pushdown mechanism\n- Executes searches even when results might not be needed\n- Doesn't preserve expression context for EXPLAIN output\n\nThe new `SearchPredicateUDF` enables lazy evaluation and is designed to\nintegrate with DataFusion's filter pushdown, allowing single-table\npredicates to be pushed to individual table scans.\n\n## How\n\n- Created `SearchPredicateUDF` in `scan/search_predicate_udf.rs` that:\n  - Carries the search query, index OID, and heap OID\n- Stores raw pointers (`expr_ptr`, `planner_info_ptr`) for lazy deparse\nin EXPLAIN\n- Falls back to executing the search when not pushed down (cross-table\npredicates)\n- Added `RawPtr<T>` utility in `postgres/utils.rs` for type-safe\nserializable pointer handling\n- Updated `JoinLevelSearchPredicate` to store expression pointers\n- Removed eager `compute_predicate_matches` from scan_state\n- Updated translator to create `SearchPredicateUDF` instead of\n`RowInSetUDF`\n- Deleted `joinscan/udf.rs` (no longer needed)\n\n## Tests\n\n- Updated `join_custom_scan` regression test for new UDF name\n(`pdb_search_predicate` instead of `row_in_set`)\n- Added unit tests for `SearchPredicateUDF` (name, into_expr,\ntry_from_expr)",
          "timestamp": "2026-02-09T11:24:14-08:00",
          "tree_id": "7f721858975e5cac391d211ec704d17b33841d28",
          "url": "https://github.com/paradedb/paradedb/commit/589fa838de3d99e5efefeee907cc9e85086e4d13"
        },
        "date": 1770666230353,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - cpu",
            "value": 9.248554,
            "unit": "median cpu",
            "extra": "avg cpu: 8.319518760844039, max cpu: 24.096386, count: 55047"
          },
          {
            "name": "Aggregate Custom Scan - Primary - mem",
            "value": 62.61328125,
            "unit": "median mem",
            "extra": "avg mem: 62.407961463612914, max mem: 73.6875, count: 55047"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.631995388024466, max cpu: 9.365853, count: 55047"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 35.87109375,
            "unit": "median mem",
            "extra": "avg mem: 35.70685320612386, max mem: 37.90234375, count: 55047"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.60912878992201, max cpu: 4.733728, count: 55047"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 60.65234375,
            "unit": "median mem",
            "extra": "avg mem: 60.24240911459753, max mem: 71.83984375, count: 55047"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.60528797247185, max cpu: 9.311348, count: 110094"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 48.7578125,
            "unit": "median mem",
            "extra": "avg mem: 48.7224540790143, max mem: 59.73828125, count: 110094"
          },
          {
            "name": "Mixed Fast Field Scan - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 5.414787770492123, max cpu: 18.497108, count: 55047"
          },
          {
            "name": "Mixed Fast Field Scan - Primary - mem",
            "value": 61.65625,
            "unit": "median mem",
            "extra": "avg mem: 61.506719683861064, max mem: 72.73046875, count: 55047"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 1738,
            "unit": "median block_count",
            "extra": "avg block_count: 1743.4673824186605, max block_count: 3099.0, count: 55047"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 11,
            "unit": "median segment_count",
            "extra": "avg segment_count: 11.230748269660472, max segment_count: 24.0, count: 55047"
          },
          {
            "name": "Normal Scan - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 5.277364600898826, max cpu: 15.058823, count: 55047"
          },
          {
            "name": "Normal Scan - Primary - mem",
            "value": 61.640625,
            "unit": "median mem",
            "extra": "avg mem: 61.48018461780388, max mem: 72.6640625, count: 55047"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.432137182285418, max cpu: 4.824121, count: 55047"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 51.39453125,
            "unit": "median mem",
            "extra": "avg mem: 51.007638713849076, max mem: 62.13671875, count: 55047"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6875,
            "unit": "median cpu",
            "extra": "avg cpu: 3.9209044071405144, max cpu: 4.7524753, count: 55047"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 53.875,
            "unit": "median mem",
            "extra": "avg mem: 53.39942337627391, max mem: 66.6328125, count: 55047"
          }
        ]
      }
    ],
    "pg_search bulk-updates.toml Performance - TPS": [
      {
        "commit": {
          "author": {
            "name": "Stu Hood",
            "username": "stuhood",
            "email": "stuhood@paradedb.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "05025da0ce66b0606497b092203538a732534d16",
          "message": "chore: Fix benchmark data destination. (#4135)\n\n## What\n\nSend all query benchmark data to the same directory.\n\n## Why\n\nIn #4080, we accidentally introduced subdirectories of our `benchmarks`\ndataset, which resulted in separate datasets and pages to render them,\nrather than a single dataset and page.\n\n<img width=\"145\" height=\"413\" alt=\"Screenshot 2026-02-08 at 5 04 35 PM\"\nsrc=\"https://github.com/user-attachments/assets/5afbcaf0-d823-4507-b0ab-36494b839661\"\n/>\n\nEach subdirectory has its own `data.js` and `index.html`, but we want it\nto be merged into the parent directory's data.",
          "timestamp": "2026-02-09T01:08:28Z",
          "url": "https://github.com/paradedb/paradedb/commit/05025da0ce66b0606497b092203538a732534d16"
        },
        "date": 1770611026125,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 7.734170460752499,
            "unit": "median tps",
            "extra": "avg tps: 6.639922827873307, max tps: 10.014262572041444, count: 57787"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.323796506604033,
            "unit": "median tps",
            "extra": "avg tps: 4.772389363788728, max tps: 5.9722371440075, count: 57787"
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
          "id": "b91006e07d5c6fc93a8bb38ca19094ef5f3081f7",
          "message": "fix: Early return in fieldname extraction causes pushdown to not happen (#4075)\n\n# Description\nBackport of #4071 to `0.21.x`.\n\n---------\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2026-02-03T20:28:51Z",
          "url": "https://github.com/paradedb/paradedb/commit/b91006e07d5c6fc93a8bb38ca19094ef5f3081f7"
        },
        "date": 1770611293449,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 7.527352454600586,
            "unit": "median tps",
            "extra": "avg tps: 6.439218337354045, max tps: 9.66083256147737, count: 57902"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.49867850514885,
            "unit": "median tps",
            "extra": "avg tps: 4.929503119331737, max tps: 6.15630524801993, count: 57902"
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
          "id": "589fa838de3d99e5efefeee907cc9e85086e4d13",
          "message": "chore: create `SearchPredicateUDF` for lazy Tantivy query evaluation (#4122)\n\n## Ticket(s) Closed\n\n- Partially helps #4061 \n\n## What\n\nReplace `RowInSetUDF` with a new `SearchPredicateUDF` that carries the\nsearch query and defers execution, enabling future filter pushdown to\n`PgSearchTableProvider`.\n\n## Why\n\nThe previous `RowInSetUDF` eagerly pre-computed all matching CTIDs\nbefore join execution by running the Tantivy search upfront. This\napproach:\n- Cannot benefit from DataFusion's filter pushdown mechanism\n- Executes searches even when results might not be needed\n- Doesn't preserve expression context for EXPLAIN output\n\nThe new `SearchPredicateUDF` enables lazy evaluation and is designed to\nintegrate with DataFusion's filter pushdown, allowing single-table\npredicates to be pushed to individual table scans.\n\n## How\n\n- Created `SearchPredicateUDF` in `scan/search_predicate_udf.rs` that:\n  - Carries the search query, index OID, and heap OID\n- Stores raw pointers (`expr_ptr`, `planner_info_ptr`) for lazy deparse\nin EXPLAIN\n- Falls back to executing the search when not pushed down (cross-table\npredicates)\n- Added `RawPtr<T>` utility in `postgres/utils.rs` for type-safe\nserializable pointer handling\n- Updated `JoinLevelSearchPredicate` to store expression pointers\n- Removed eager `compute_predicate_matches` from scan_state\n- Updated translator to create `SearchPredicateUDF` instead of\n`RowInSetUDF`\n- Deleted `joinscan/udf.rs` (no longer needed)\n\n## Tests\n\n- Updated `join_custom_scan` regression test for new UDF name\n(`pdb_search_predicate` instead of `row_in_set`)\n- Added unit tests for `SearchPredicateUDF` (name, into_expr,\ntry_from_expr)",
          "timestamp": "2026-02-09T11:24:14-08:00",
          "tree_id": "7f721858975e5cac391d211ec704d17b33841d28",
          "url": "https://github.com/paradedb/paradedb/commit/589fa838de3d99e5efefeee907cc9e85086e4d13"
        },
        "date": 1770667149929,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 7.709777136028529,
            "unit": "median tps",
            "extra": "avg tps: 6.634865245735973, max tps: 9.93812057381648, count: 57769"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.321537651660799,
            "unit": "median tps",
            "extra": "avg tps: 4.768795858740857, max tps: 5.979266466116325, count: 57769"
          }
        ]
      }
    ],
    "pg_search bulk-updates.toml Performance - Other Metrics": [
      {
        "commit": {
          "author": {
            "name": "Stu Hood",
            "username": "stuhood",
            "email": "stuhood@paradedb.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "05025da0ce66b0606497b092203538a732534d16",
          "message": "chore: Fix benchmark data destination. (#4135)\n\n## What\n\nSend all query benchmark data to the same directory.\n\n## Why\n\nIn #4080, we accidentally introduced subdirectories of our `benchmarks`\ndataset, which resulted in separate datasets and pages to render them,\nrather than a single dataset and page.\n\n<img width=\"145\" height=\"413\" alt=\"Screenshot 2026-02-08 at 5 04 35 PM\"\nsrc=\"https://github.com/user-attachments/assets/5afbcaf0-d823-4507-b0ab-36494b839661\"\n/>\n\nEach subdirectory has its own `data.js` and `index.html`, but we want it\nto be merged into the parent directory's data.",
          "timestamp": "2026-02-09T01:08:28Z",
          "url": "https://github.com/paradedb/paradedb/commit/05025da0ce66b0606497b092203538a732534d16"
        },
        "date": 1770611036166,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.210833,
            "unit": "median cpu",
            "extra": "avg cpu: 21.04075997203024, max cpu: 42.772278, count: 57787"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 236.03515625,
            "unit": "median mem",
            "extra": "avg mem: 235.90127861802827, max mem: 237.5078125, count: 57787"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.323614,
            "unit": "median cpu",
            "extra": "avg cpu: 22.470302276083196, max cpu: 33.366436, count: 57787"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 175.40234375,
            "unit": "median mem",
            "extra": "avg mem: 175.15582075661482, max mem: 175.8515625, count: 57787"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 34256,
            "unit": "median block_count",
            "extra": "avg block_count: 33530.154359977154, max block_count: 36247.0, count: 57787"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 79,
            "unit": "median segment_count",
            "extra": "avg segment_count: 81.58656791319846, max segment_count: 130.0, count: 57787"
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
          "id": "b91006e07d5c6fc93a8bb38ca19094ef5f3081f7",
          "message": "fix: Early return in fieldname extraction causes pushdown to not happen (#4075)\n\n# Description\nBackport of #4071 to `0.21.x`.\n\n---------\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2026-02-03T20:28:51Z",
          "url": "https://github.com/paradedb/paradedb/commit/b91006e07d5c6fc93a8bb38ca19094ef5f3081f7"
        },
        "date": 1770611299018,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.233301,
            "unit": "median cpu",
            "extra": "avg cpu: 21.259072140337338, max cpu: 42.72997, count: 57902"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 232.96875,
            "unit": "median mem",
            "extra": "avg mem: 232.8323612121559, max mem: 234.4453125, count: 57902"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.323614,
            "unit": "median cpu",
            "extra": "avg cpu: 22.327023764671164, max cpu: 33.267326, count: 57902"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 172.1640625,
            "unit": "median mem",
            "extra": "avg mem: 172.0855480353658, max mem: 173.09765625, count: 57902"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 33775,
            "unit": "median block_count",
            "extra": "avg block_count: 33398.72722876585, max block_count: 36088.0, count: 57902"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 78,
            "unit": "median segment_count",
            "extra": "avg segment_count: 81.0958343407827, max segment_count: 127.0, count: 57902"
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
          "id": "589fa838de3d99e5efefeee907cc9e85086e4d13",
          "message": "chore: create `SearchPredicateUDF` for lazy Tantivy query evaluation (#4122)\n\n## Ticket(s) Closed\n\n- Partially helps #4061 \n\n## What\n\nReplace `RowInSetUDF` with a new `SearchPredicateUDF` that carries the\nsearch query and defers execution, enabling future filter pushdown to\n`PgSearchTableProvider`.\n\n## Why\n\nThe previous `RowInSetUDF` eagerly pre-computed all matching CTIDs\nbefore join execution by running the Tantivy search upfront. This\napproach:\n- Cannot benefit from DataFusion's filter pushdown mechanism\n- Executes searches even when results might not be needed\n- Doesn't preserve expression context for EXPLAIN output\n\nThe new `SearchPredicateUDF` enables lazy evaluation and is designed to\nintegrate with DataFusion's filter pushdown, allowing single-table\npredicates to be pushed to individual table scans.\n\n## How\n\n- Created `SearchPredicateUDF` in `scan/search_predicate_udf.rs` that:\n  - Carries the search query, index OID, and heap OID\n- Stores raw pointers (`expr_ptr`, `planner_info_ptr`) for lazy deparse\nin EXPLAIN\n- Falls back to executing the search when not pushed down (cross-table\npredicates)\n- Added `RawPtr<T>` utility in `postgres/utils.rs` for type-safe\nserializable pointer handling\n- Updated `JoinLevelSearchPredicate` to store expression pointers\n- Removed eager `compute_predicate_matches` from scan_state\n- Updated translator to create `SearchPredicateUDF` instead of\n`RowInSetUDF`\n- Deleted `joinscan/udf.rs` (no longer needed)\n\n## Tests\n\n- Updated `join_custom_scan` regression test for new UDF name\n(`pdb_search_predicate` instead of `row_in_set`)\n- Added unit tests for `SearchPredicateUDF` (name, into_expr,\ntry_from_expr)",
          "timestamp": "2026-02-09T11:24:14-08:00",
          "tree_id": "7f721858975e5cac391d211ec704d17b33841d28",
          "url": "https://github.com/paradedb/paradedb/commit/589fa838de3d99e5efefeee907cc9e85086e4d13"
        },
        "date": 1770667155267,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.210833,
            "unit": "median cpu",
            "extra": "avg cpu: 21.14348584771333, max cpu: 42.814667, count: 57769"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 235.69140625,
            "unit": "median mem",
            "extra": "avg mem: 235.60332619192386, max mem: 237.1796875, count: 57769"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.323614,
            "unit": "median cpu",
            "extra": "avg cpu: 22.4997001703195, max cpu: 33.23442, count: 57769"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 175.64453125,
            "unit": "median mem",
            "extra": "avg mem: 175.24456395028042, max mem: 175.78515625, count: 57769"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 33916,
            "unit": "median block_count",
            "extra": "avg block_count: 33439.17106060344, max block_count: 35963.0, count: 57769"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 78,
            "unit": "median segment_count",
            "extra": "avg segment_count: 81.21018193148575, max segment_count: 128.0, count: 57769"
          }
        ]
      }
    ],
    "pg_search wide-table.toml Performance - TPS": [
      {
        "commit": {
          "author": {
            "name": "Stu Hood",
            "username": "stuhood",
            "email": "stuhood@paradedb.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "05025da0ce66b0606497b092203538a732534d16",
          "message": "chore: Fix benchmark data destination. (#4135)\n\n## What\n\nSend all query benchmark data to the same directory.\n\n## Why\n\nIn #4080, we accidentally introduced subdirectories of our `benchmarks`\ndataset, which resulted in separate datasets and pages to render them,\nrather than a single dataset and page.\n\n<img width=\"145\" height=\"413\" alt=\"Screenshot 2026-02-08 at 5 04 35 PM\"\nsrc=\"https://github.com/user-attachments/assets/5afbcaf0-d823-4507-b0ab-36494b839661\"\n/>\n\nEach subdirectory has its own `data.js` and `index.html`, but we want it\nto be merged into the parent directory's data.",
          "timestamp": "2026-02-09T01:08:28Z",
          "url": "https://github.com/paradedb/paradedb/commit/05025da0ce66b0606497b092203538a732534d16"
        },
        "date": 1770611942930,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 1135.1821210046344,
            "unit": "median tps",
            "extra": "avg tps: 1141.5176581838195, max tps: 1192.092575386586, count: 55405"
          },
          {
            "name": "Single Insert - Primary - tps",
            "value": 1186.9992237890751,
            "unit": "median tps",
            "extra": "avg tps: 1188.2297494295433, max tps: 1273.247891879775, count: 55405"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 1898.6714837924858,
            "unit": "median tps",
            "extra": "avg tps: 1884.4992034802265, max tps: 2054.067593875601, count: 55405"
          },
          {
            "name": "Top N - Primary - tps",
            "value": 5.669895362615561,
            "unit": "median tps",
            "extra": "avg tps: 5.642040403231506, max tps: 6.848778752825472, count: 55405"
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
          "id": "b91006e07d5c6fc93a8bb38ca19094ef5f3081f7",
          "message": "fix: Early return in fieldname extraction causes pushdown to not happen (#4075)\n\n# Description\nBackport of #4071 to `0.21.x`.\n\n---------\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2026-02-03T20:28:51Z",
          "url": "https://github.com/paradedb/paradedb/commit/b91006e07d5c6fc93a8bb38ca19094ef5f3081f7"
        },
        "date": 1770612208492,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 1132.4624602011081,
            "unit": "median tps",
            "extra": "avg tps: 1135.431260806755, max tps: 1183.6204968225818, count: 55927"
          },
          {
            "name": "Single Insert - Primary - tps",
            "value": 1248.92694398444,
            "unit": "median tps",
            "extra": "avg tps: 1242.1948507636223, max tps: 1261.3168782392343, count: 55927"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 1802.8410873796174,
            "unit": "median tps",
            "extra": "avg tps: 1780.8975077676332, max tps: 1930.5241292557973, count: 55927"
          },
          {
            "name": "Top N - Primary - tps",
            "value": 5.553028817504838,
            "unit": "median tps",
            "extra": "avg tps: 5.563482341576923, max tps: 7.504011973101344, count: 55927"
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
          "id": "589fa838de3d99e5efefeee907cc9e85086e4d13",
          "message": "chore: create `SearchPredicateUDF` for lazy Tantivy query evaluation (#4122)\n\n## Ticket(s) Closed\n\n- Partially helps #4061 \n\n## What\n\nReplace `RowInSetUDF` with a new `SearchPredicateUDF` that carries the\nsearch query and defers execution, enabling future filter pushdown to\n`PgSearchTableProvider`.\n\n## Why\n\nThe previous `RowInSetUDF` eagerly pre-computed all matching CTIDs\nbefore join execution by running the Tantivy search upfront. This\napproach:\n- Cannot benefit from DataFusion's filter pushdown mechanism\n- Executes searches even when results might not be needed\n- Doesn't preserve expression context for EXPLAIN output\n\nThe new `SearchPredicateUDF` enables lazy evaluation and is designed to\nintegrate with DataFusion's filter pushdown, allowing single-table\npredicates to be pushed to individual table scans.\n\n## How\n\n- Created `SearchPredicateUDF` in `scan/search_predicate_udf.rs` that:\n  - Carries the search query, index OID, and heap OID\n- Stores raw pointers (`expr_ptr`, `planner_info_ptr`) for lazy deparse\nin EXPLAIN\n- Falls back to executing the search when not pushed down (cross-table\npredicates)\n- Added `RawPtr<T>` utility in `postgres/utils.rs` for type-safe\nserializable pointer handling\n- Updated `JoinLevelSearchPredicate` to store expression pointers\n- Removed eager `compute_predicate_matches` from scan_state\n- Updated translator to create `SearchPredicateUDF` instead of\n`RowInSetUDF`\n- Deleted `joinscan/udf.rs` (no longer needed)\n\n## Tests\n\n- Updated `join_custom_scan` regression test for new UDF name\n(`pdb_search_predicate` instead of `row_in_set`)\n- Added unit tests for `SearchPredicateUDF` (name, into_expr,\ntry_from_expr)",
          "timestamp": "2026-02-09T11:24:14-08:00",
          "tree_id": "7f721858975e5cac391d211ec704d17b33841d28",
          "url": "https://github.com/paradedb/paradedb/commit/589fa838de3d99e5efefeee907cc9e85086e4d13"
        },
        "date": 1770668097284,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 1094.1597551230295,
            "unit": "median tps",
            "extra": "avg tps: 1098.0865973884493, max tps: 1138.1031059763216, count: 55997"
          },
          {
            "name": "Single Insert - Primary - tps",
            "value": 1248.7181399135563,
            "unit": "median tps",
            "extra": "avg tps: 1246.3707637062698, max tps: 1261.5043276963152, count: 55997"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 1931.7194341400732,
            "unit": "median tps",
            "extra": "avg tps: 1908.8382810698859, max tps: 2083.6750135113007, count: 55997"
          },
          {
            "name": "Top N - Primary - tps",
            "value": 5.497376984873781,
            "unit": "median tps",
            "extra": "avg tps: 5.52430015268192, max tps: 7.688268453483811, count: 55997"
          }
        ]
      }
    ],
    "pg_search wide-table.toml Performance - Other Metrics": [
      {
        "commit": {
          "author": {
            "name": "Stu Hood",
            "username": "stuhood",
            "email": "stuhood@paradedb.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "05025da0ce66b0606497b092203538a732534d16",
          "message": "chore: Fix benchmark data destination. (#4135)\n\n## What\n\nSend all query benchmark data to the same directory.\n\n## Why\n\nIn #4080, we accidentally introduced subdirectories of our `benchmarks`\ndataset, which resulted in separate datasets and pages to render them,\nrather than a single dataset and page.\n\n<img width=\"145\" height=\"413\" alt=\"Screenshot 2026-02-08 at 5 04 35 PM\"\nsrc=\"https://github.com/user-attachments/assets/5afbcaf0-d823-4507-b0ab-36494b839661\"\n/>\n\nEach subdirectory has its own `data.js` and `index.html`, but we want it\nto be merged into the parent directory's data.",
          "timestamp": "2026-02-09T01:08:28Z",
          "url": "https://github.com/paradedb/paradedb/commit/05025da0ce66b0606497b092203538a732534d16"
        },
        "date": 1770611948328,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Background Merger - Primary - background_merging",
            "value": 0,
            "unit": "median background_merging",
            "extra": "avg background_merging: 0.0790361880696688, max background_merging: 2.0, count: 55405"
          },
          {
            "name": "Background Merger - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.742215866631361, max cpu: 9.628887, count: 55405"
          },
          {
            "name": "Background Merger - Primary - mem",
            "value": 25.546875,
            "unit": "median mem",
            "extra": "avg mem: 25.53648347960473, max mem: 25.55078125, count: 55405"
          },
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 4.911382706003295, max cpu: 13.9265, count: 55405"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 168.734375,
            "unit": "median mem",
            "extra": "avg mem: 167.42637771015703, max mem: 168.91015625, count: 55405"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 53660,
            "unit": "median block_count",
            "extra": "avg block_count: 53521.79001895136, max block_count: 53660.0, count: 55405"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 45,
            "unit": "median segment_count",
            "extra": "avg segment_count: 42.29717534518545, max segment_count: 58.0, count: 55405"
          },
          {
            "name": "Single Insert - Primary - cpu",
            "value": 4.673807,
            "unit": "median cpu",
            "extra": "avg cpu: 4.633385187516015, max cpu: 9.476802, count: 55405"
          },
          {
            "name": "Single Insert - Primary - mem",
            "value": 126.21875,
            "unit": "median mem",
            "extra": "avg mem: 115.1988111688927, max mem: 141.625, count: 55405"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 4.755157485331961, max cpu: 11.464969, count: 55405"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 168.44140625,
            "unit": "median mem",
            "extra": "avg mem: 164.38026457167675, max mem: 168.70703125, count: 55405"
          },
          {
            "name": "Top N - Primary - cpu",
            "value": 23.460411,
            "unit": "median cpu",
            "extra": "avg cpu: 23.691812283434306, max cpu: 33.7011, count: 55405"
          },
          {
            "name": "Top N - Primary - mem",
            "value": 162.73046875,
            "unit": "median mem",
            "extra": "avg mem: 182.94361540869505, max mem: 223.140625, count: 55405"
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
          "id": "b91006e07d5c6fc93a8bb38ca19094ef5f3081f7",
          "message": "fix: Early return in fieldname extraction causes pushdown to not happen (#4075)\n\n# Description\nBackport of #4071 to `0.21.x`.\n\n---------\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2026-02-03T20:28:51Z",
          "url": "https://github.com/paradedb/paradedb/commit/b91006e07d5c6fc93a8bb38ca19094ef5f3081f7"
        },
        "date": 1770612213997,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Background Merger - Primary - background_merging",
            "value": 0,
            "unit": "median background_merging",
            "extra": "avg background_merging: 0.08046203086165894, max background_merging: 2.0, count: 55927"
          },
          {
            "name": "Background Merger - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.7185845136206295, max cpu: 9.648242, count: 55927"
          },
          {
            "name": "Background Merger - Primary - mem",
            "value": 25.90625,
            "unit": "median mem",
            "extra": "avg mem: 25.8910234686511, max mem: 25.90625, count: 55927"
          },
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 4.934581921882236, max cpu: 13.93998, count: 55927"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 165.84375,
            "unit": "median mem",
            "extra": "avg mem: 164.62415144686378, max mem: 166.234375, count: 55927"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 51433,
            "unit": "median block_count",
            "extra": "avg block_count: 51292.11107336349, max block_count: 51433.0, count: 55927"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 45,
            "unit": "median segment_count",
            "extra": "avg segment_count: 42.8661290610975, max segment_count: 56.0, count: 55927"
          },
          {
            "name": "Single Insert - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.630017556650915, max cpu: 9.696969, count: 55927"
          },
          {
            "name": "Single Insert - Primary - mem",
            "value": 119.46484375,
            "unit": "median mem",
            "extra": "avg mem: 110.60735325178358, max mem: 137.24609375, count: 55927"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.743908958430622, max cpu: 9.523809, count: 55927"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 165.5703125,
            "unit": "median mem",
            "extra": "avg mem: 161.4201981489263, max mem: 165.77734375, count: 55927"
          },
          {
            "name": "Top N - Primary - cpu",
            "value": 23.369036,
            "unit": "median cpu",
            "extra": "avg cpu: 23.859869383055397, max cpu: 33.7011, count: 55927"
          },
          {
            "name": "Top N - Primary - mem",
            "value": 159.9375,
            "unit": "median mem",
            "extra": "avg mem: 178.88472864180093, max mem: 220.43359375, count: 55927"
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
          "id": "589fa838de3d99e5efefeee907cc9e85086e4d13",
          "message": "chore: create `SearchPredicateUDF` for lazy Tantivy query evaluation (#4122)\n\n## Ticket(s) Closed\n\n- Partially helps #4061 \n\n## What\n\nReplace `RowInSetUDF` with a new `SearchPredicateUDF` that carries the\nsearch query and defers execution, enabling future filter pushdown to\n`PgSearchTableProvider`.\n\n## Why\n\nThe previous `RowInSetUDF` eagerly pre-computed all matching CTIDs\nbefore join execution by running the Tantivy search upfront. This\napproach:\n- Cannot benefit from DataFusion's filter pushdown mechanism\n- Executes searches even when results might not be needed\n- Doesn't preserve expression context for EXPLAIN output\n\nThe new `SearchPredicateUDF` enables lazy evaluation and is designed to\nintegrate with DataFusion's filter pushdown, allowing single-table\npredicates to be pushed to individual table scans.\n\n## How\n\n- Created `SearchPredicateUDF` in `scan/search_predicate_udf.rs` that:\n  - Carries the search query, index OID, and heap OID\n- Stores raw pointers (`expr_ptr`, `planner_info_ptr`) for lazy deparse\nin EXPLAIN\n- Falls back to executing the search when not pushed down (cross-table\npredicates)\n- Added `RawPtr<T>` utility in `postgres/utils.rs` for type-safe\nserializable pointer handling\n- Updated `JoinLevelSearchPredicate` to store expression pointers\n- Removed eager `compute_predicate_matches` from scan_state\n- Updated translator to create `SearchPredicateUDF` instead of\n`RowInSetUDF`\n- Deleted `joinscan/udf.rs` (no longer needed)\n\n## Tests\n\n- Updated `join_custom_scan` regression test for new UDF name\n(`pdb_search_predicate` instead of `row_in_set`)\n- Added unit tests for `SearchPredicateUDF` (name, into_expr,\ntry_from_expr)",
          "timestamp": "2026-02-09T11:24:14-08:00",
          "tree_id": "7f721858975e5cac391d211ec704d17b33841d28",
          "url": "https://github.com/paradedb/paradedb/commit/589fa838de3d99e5efefeee907cc9e85086e4d13"
        },
        "date": 1770668102726,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Background Merger - Primary - background_merging",
            "value": 0,
            "unit": "median background_merging",
            "extra": "avg background_merging: 0.08004000214297194, max background_merging: 2.0, count: 55997"
          },
          {
            "name": "Background Merger - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.706867159826482, max cpu: 9.638554, count: 55997"
          },
          {
            "name": "Background Merger - Primary - mem",
            "value": 29.265625,
            "unit": "median mem",
            "extra": "avg mem: 29.246110212935516, max mem: 29.26953125, count: 55997"
          },
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.947969775248089, max cpu: 9.856263, count: 55997"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 168.73046875,
            "unit": "median mem",
            "extra": "avg mem: 167.31546011951534, max mem: 168.828125, count: 55997"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 50955,
            "unit": "median block_count",
            "extra": "avg block_count: 50813.72544957766, max block_count: 50955.0, count: 55997"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 45,
            "unit": "median segment_count",
            "extra": "avg segment_count: 41.77716663392682, max segment_count: 62.0, count: 55997"
          },
          {
            "name": "Single Insert - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.661053247054649, max cpu: 9.523809, count: 55997"
          },
          {
            "name": "Single Insert - Primary - mem",
            "value": 123.5390625,
            "unit": "median mem",
            "extra": "avg mem: 113.69642367838902, max mem: 137.8125, count: 55997"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.77926552424967, max cpu: 9.648242, count: 55997"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 168.359375,
            "unit": "median mem",
            "extra": "avg mem: 164.06738363215888, max mem: 168.47265625, count: 55997"
          },
          {
            "name": "Top N - Primary - cpu",
            "value": 23.414635,
            "unit": "median cpu",
            "extra": "avg cpu: 23.937371652887563, max cpu: 33.333336, count: 55997"
          },
          {
            "name": "Top N - Primary - mem",
            "value": 162.6953125,
            "unit": "median mem",
            "extra": "avg mem: 181.08817659874637, max mem: 223.140625, count: 55997"
          }
        ]
      }
    ],
    "pg_search background-merge.toml Performance - TPS": [
      {
        "commit": {
          "author": {
            "name": "Stu Hood",
            "username": "stuhood",
            "email": "stuhood@paradedb.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "05025da0ce66b0606497b092203538a732534d16",
          "message": "chore: Fix benchmark data destination. (#4135)\n\n## What\n\nSend all query benchmark data to the same directory.\n\n## Why\n\nIn #4080, we accidentally introduced subdirectories of our `benchmarks`\ndataset, which resulted in separate datasets and pages to render them,\nrather than a single dataset and page.\n\n<img width=\"145\" height=\"413\" alt=\"Screenshot 2026-02-08 at 5 04 35 PM\"\nsrc=\"https://github.com/user-attachments/assets/5afbcaf0-d823-4507-b0ab-36494b839661\"\n/>\n\nEach subdirectory has its own `data.js` and `index.html`, but we want it\nto be merged into the parent directory's data.",
          "timestamp": "2026-02-09T01:08:28Z",
          "url": "https://github.com/paradedb/paradedb/commit/05025da0ce66b0606497b092203538a732534d16"
        },
        "date": 1770612809105,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 29.910791087875655,
            "unit": "median tps",
            "extra": "avg tps: 29.905984741433212, max tps: 34.04397231809714, count: 55416"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 239.06082615096682,
            "unit": "median tps",
            "extra": "avg tps: 261.29951871170516, max tps: 2621.411808929018, count: 55416"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1909.8247051741882,
            "unit": "median tps",
            "extra": "avg tps: 1887.2672550690818, max tps: 2216.0258968352323, count: 55416"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 153.09092612081014,
            "unit": "median tps",
            "extra": "avg tps: 190.33620462611304, max tps: 1701.1674398747061, count: 110832"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 15.1872163560583,
            "unit": "median tps",
            "extra": "avg tps: 15.210449577106186, max tps: 20.756491073671015, count: 55416"
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
          "id": "b91006e07d5c6fc93a8bb38ca19094ef5f3081f7",
          "message": "fix: Early return in fieldname extraction causes pushdown to not happen (#4075)\n\n# Description\nBackport of #4071 to `0.21.x`.\n\n---------\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2026-02-03T20:28:51Z",
          "url": "https://github.com/paradedb/paradedb/commit/b91006e07d5c6fc93a8bb38ca19094ef5f3081f7"
        },
        "date": 1770613075276,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 34.67159866149032,
            "unit": "median tps",
            "extra": "avg tps: 34.25738405871206, max tps: 35.322420015024605, count: 55777"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 241.82739759474745,
            "unit": "median tps",
            "extra": "avg tps: 267.72288815898617, max tps: 2701.038889292735, count: 55777"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1938.8981691579854,
            "unit": "median tps",
            "extra": "avg tps: 1904.6300733877886, max tps: 2413.141971175019, count: 55777"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 152.63063691673244,
            "unit": "median tps",
            "extra": "avg tps: 195.8018048323366, max tps: 1624.4782394108508, count: 111554"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 16.525596957020568,
            "unit": "median tps",
            "extra": "avg tps: 16.092056045962423, max tps: 20.83767343000243, count: 55777"
          }
        ]
      }
    ],
    "pg_search background-merge.toml Performance - Other Metrics": [
      {
        "commit": {
          "author": {
            "name": "Stu Hood",
            "username": "stuhood",
            "email": "stuhood@paradedb.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "05025da0ce66b0606497b092203538a732534d16",
          "message": "chore: Fix benchmark data destination. (#4135)\n\n## What\n\nSend all query benchmark data to the same directory.\n\n## Why\n\nIn #4080, we accidentally introduced subdirectories of our `benchmarks`\ndataset, which resulted in separate datasets and pages to render them,\nrather than a single dataset and page.\n\n<img width=\"145\" height=\"413\" alt=\"Screenshot 2026-02-08 at 5 04 35 PM\"\nsrc=\"https://github.com/user-attachments/assets/5afbcaf0-d823-4507-b0ab-36494b839661\"\n/>\n\nEach subdirectory has its own `data.js` and `index.html`, but we want it\nto be merged into the parent directory's data.",
          "timestamp": "2026-02-09T01:08:28Z",
          "url": "https://github.com/paradedb/paradedb/commit/05025da0ce66b0606497b092203538a732534d16"
        },
        "date": 1770612814691,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.58664,
            "unit": "median cpu",
            "extra": "avg cpu: 20.05581804108024, max cpu: 57.83132, count: 55416"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 176.578125,
            "unit": "median mem",
            "extra": "avg mem: 174.2470539589198, max mem: 176.78125, count: 55416"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 7.731840969845872, max cpu: 38.554214, count: 55416"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 120.80859375,
            "unit": "median mem",
            "extra": "avg mem: 119.60175958883715, max mem: 120.9609375, count: 55416"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.806808758621002, max cpu: 9.495549, count: 55416"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 125.16015625,
            "unit": "median mem",
            "extra": "avg mem: 118.53520996305218, max mem: 159.96875, count: 55416"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 13777,
            "unit": "median block_count",
            "extra": "avg block_count: 13898.909827486646, max block_count: 24701.0, count: 55416"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.522304472560878, max cpu: 4.64666, count: 55416"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 96.546875,
            "unit": "median mem",
            "extra": "avg mem: 92.53876291932835, max mem: 134.31640625, count: 55416"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 25,
            "unit": "median segment_count",
            "extra": "avg segment_count: 24.64012198642991, max segment_count: 40.0, count: 55416"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.239654,
            "unit": "median cpu",
            "extra": "avg cpu: 9.042165393843302, max cpu: 57.83132, count: 110832"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 162.51953125,
            "unit": "median mem",
            "extra": "avg mem: 141.28773852965526, max mem: 164.609375, count: 110832"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.872832,
            "unit": "median cpu",
            "extra": "avg cpu: 12.595139447435345, max cpu: 27.799229, count: 55416"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 170.8359375,
            "unit": "median mem",
            "extra": "avg mem: 167.76749988157752, max mem: 171.8046875, count: 55416"
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
          "id": "b91006e07d5c6fc93a8bb38ca19094ef5f3081f7",
          "message": "fix: Early return in fieldname extraction causes pushdown to not happen (#4075)\n\n# Description\nBackport of #4071 to `0.21.x`.\n\n---------\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2026-02-03T20:28:51Z",
          "url": "https://github.com/paradedb/paradedb/commit/b91006e07d5c6fc93a8bb38ca19094ef5f3081f7"
        },
        "date": 1770613080569,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.568666,
            "unit": "median cpu",
            "extra": "avg cpu: 19.366751820444655, max cpu: 42.687748, count: 55777"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 166.78515625,
            "unit": "median mem",
            "extra": "avg mem: 149.8679496028829, max mem: 173.5078125, count: 55777"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 7.677355631406489, max cpu: 28.042841, count: 55777"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 117.37890625,
            "unit": "median mem",
            "extra": "avg mem: 116.189483694556, max mem: 117.453125, count: 55777"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.783298092166139, max cpu: 9.619239, count: 55777"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 124.453125,
            "unit": "median mem",
            "extra": "avg mem: 114.2958995755419, max mem: 156.625, count: 55777"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 14090,
            "unit": "median block_count",
            "extra": "avg block_count: 14271.992398300375, max block_count: 25611.0, count: 55777"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.619827,
            "unit": "median cpu",
            "extra": "avg cpu: 4.302793363284725, max cpu: 4.743083, count: 55777"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 95.56640625,
            "unit": "median mem",
            "extra": "avg mem: 89.18988848730211, max mem: 132.578125, count: 55777"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 23,
            "unit": "median segment_count",
            "extra": "avg segment_count: 23.48227764132169, max segment_count: 43.0, count: 55777"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.239654,
            "unit": "median cpu",
            "extra": "avg cpu: 9.095029959597086, max cpu: 28.973843, count: 111554"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 159.37109375,
            "unit": "median mem",
            "extra": "avg mem: 138.1940955655445, max mem: 162.234375, count: 111554"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.846154,
            "unit": "median cpu",
            "extra": "avg cpu: 12.086620187114395, max cpu: 27.612656, count: 55777"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 168.0390625,
            "unit": "median mem",
            "extra": "avg mem: 164.96804126252758, max mem: 169.05859375, count: 55777"
          }
        ]
      }
    ],
    "pg_search logical-replication.toml Performance - TPS": [
      {
        "commit": {
          "author": {
            "name": "Stu Hood",
            "username": "stuhood",
            "email": "stuhood@paradedb.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "05025da0ce66b0606497b092203538a732534d16",
          "message": "chore: Fix benchmark data destination. (#4135)\n\n## What\n\nSend all query benchmark data to the same directory.\n\n## Why\n\nIn #4080, we accidentally introduced subdirectories of our `benchmarks`\ndataset, which resulted in separate datasets and pages to render them,\nrather than a single dataset and page.\n\n<img width=\"145\" height=\"413\" alt=\"Screenshot 2026-02-08 at 5 04 35 PM\"\nsrc=\"https://github.com/user-attachments/assets/5afbcaf0-d823-4507-b0ab-36494b839661\"\n/>\n\nEach subdirectory has its own `data.js` and `index.html`, but we want it\nto be merged into the parent directory's data.",
          "timestamp": "2026-02-09T01:08:28Z",
          "url": "https://github.com/paradedb/paradedb/commit/05025da0ce66b0606497b092203538a732534d16"
        },
        "date": 1770613701362,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - tps",
            "value": 650.532871805831,
            "unit": "median tps",
            "extra": "avg tps: 649.1498567388547, max tps: 675.3727127995704, count: 53870"
          },
          {
            "name": "Index Only Scan - Subscriber - tps",
            "value": 617.3248934549949,
            "unit": "median tps",
            "extra": "avg tps: 617.5275292596172, max tps: 770.9269237726517, count: 53870"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - tps",
            "value": 88.5765689931535,
            "unit": "median tps",
            "extra": "avg tps: 88.60896199247046, max tps: 89.78555907553091, count: 53870"
          },
          {
            "name": "Top N - Subscriber - tps",
            "value": 553.4145464573752,
            "unit": "median tps",
            "extra": "avg tps: 503.94402034484847, max tps: 666.0724127786973, count: 107740"
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
          "id": "b91006e07d5c6fc93a8bb38ca19094ef5f3081f7",
          "message": "fix: Early return in fieldname extraction causes pushdown to not happen (#4075)\n\n# Description\nBackport of #4071 to `0.21.x`.\n\n---------\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2026-02-03T20:28:51Z",
          "url": "https://github.com/paradedb/paradedb/commit/b91006e07d5c6fc93a8bb38ca19094ef5f3081f7"
        },
        "date": 1770613970960,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - tps",
            "value": 538.3907275399067,
            "unit": "median tps",
            "extra": "avg tps: 539.5431053215324, max tps: 715.5627016417359, count: 53774"
          },
          {
            "name": "Index Only Scan - Subscriber - tps",
            "value": 639.9845157500472,
            "unit": "median tps",
            "extra": "avg tps: 641.8743425776631, max tps: 760.1398365569922, count: 53774"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - tps",
            "value": 85.22779959761478,
            "unit": "median tps",
            "extra": "avg tps: 85.33324503663393, max tps: 92.01518833983901, count: 53774"
          },
          {
            "name": "Top N - Subscriber - tps",
            "value": 109.12036912833446,
            "unit": "median tps",
            "extra": "avg tps: 106.70566946942041, max tps: 135.19354093714767, count: 107548"
          }
        ]
      }
    ],
    "pg_search logical-replication.toml Performance - Other Metrics": [
      {
        "commit": {
          "author": {
            "name": "Stu Hood",
            "username": "stuhood",
            "email": "stuhood@paradedb.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "05025da0ce66b0606497b092203538a732534d16",
          "message": "chore: Fix benchmark data destination. (#4135)\n\n## What\n\nSend all query benchmark data to the same directory.\n\n## Why\n\nIn #4080, we accidentally introduced subdirectories of our `benchmarks`\ndataset, which resulted in separate datasets and pages to render them,\nrather than a single dataset and page.\n\n<img width=\"145\" height=\"413\" alt=\"Screenshot 2026-02-08 at 5 04 35 PM\"\nsrc=\"https://github.com/user-attachments/assets/5afbcaf0-d823-4507-b0ab-36494b839661\"\n/>\n\nEach subdirectory has its own `data.js` and `index.html`, but we want it\nto be merged into the parent directory's data.",
          "timestamp": "2026-02-09T01:08:28Z",
          "url": "https://github.com/paradedb/paradedb/commit/05025da0ce66b0606497b092203538a732534d16"
        },
        "date": 1770613706661,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - cpu",
            "value": 4.5757866,
            "unit": "median cpu",
            "extra": "avg cpu: 4.883527194140658, max cpu: 9.347614, count: 53870"
          },
          {
            "name": "Custom Scan - Subscriber - mem",
            "value": 51.01171875,
            "unit": "median mem",
            "extra": "avg mem: 50.99832887622981, max mem: 56.76171875, count: 53870"
          },
          {
            "name": "Delete values - Publisher - cpu",
            "value": 4.5757866,
            "unit": "median cpu",
            "extra": "avg cpu: 4.38883251627021, max cpu: 4.5933013, count: 53870"
          },
          {
            "name": "Delete values - Publisher - mem",
            "value": 31.54296875,
            "unit": "median mem",
            "extra": "avg mem: 30.87243660954613, max mem: 31.87109375, count: 53870"
          },
          {
            "name": "Find by ctid - Subscriber - cpu",
            "value": 9.116809,
            "unit": "median cpu",
            "extra": "avg cpu: 8.042830935630143, max cpu: 18.390804, count: 53870"
          },
          {
            "name": "Find by ctid - Subscriber - mem",
            "value": 54.41796875,
            "unit": "median mem",
            "extra": "avg mem: 54.08520302058196, max mem: 60.26953125, count: 53870"
          },
          {
            "name": "Index Only Scan - Subscriber - cpu",
            "value": 4.5757866,
            "unit": "median cpu",
            "extra": "avg cpu: 4.9574980777346225, max cpu: 9.275363, count: 53870"
          },
          {
            "name": "Index Only Scan - Subscriber - mem",
            "value": 50.5,
            "unit": "median mem",
            "extra": "avg mem: 50.53733619419436, max mem: 56.359375, count: 53870"
          },
          {
            "name": "Index Size Info - Subscriber - cpu",
            "value": 4.5714283,
            "unit": "median cpu",
            "extra": "avg cpu: 4.633579758749953, max cpu: 9.195402, count: 53870"
          },
          {
            "name": "Index Size Info - Subscriber - mem",
            "value": 33.2734375,
            "unit": "median mem",
            "extra": "avg mem: 33.260756436212176, max mem: 38.3203125, count: 53870"
          },
          {
            "name": "Index Size Info - Subscriber - pages",
            "value": 1108,
            "unit": "median pages",
            "extra": "avg pages: 1107.860293298682, max pages: 1840.0, count: 53870"
          },
          {
            "name": "Index Size Info - Subscriber - relation_size:MB",
            "value": 8.65625,
            "unit": "median relation_size:MB",
            "extra": "avg relation_size:MB: 8.655158686421014, max relation_size:MB: 14.375, count: 53870"
          },
          {
            "name": "Index Size Info - Subscriber - segment_count",
            "value": 8,
            "unit": "median segment_count",
            "extra": "avg segment_count: 7.574642658251346, max segment_count: 12.0, count: 53870"
          },
          {
            "name": "Insert value A - Publisher - cpu",
            "value": 4.5801525,
            "unit": "median cpu",
            "extra": "avg cpu: 3.4686250169408375, max cpu: 4.5845275, count: 53870"
          },
          {
            "name": "Insert value A - Publisher - mem",
            "value": 29.3046875,
            "unit": "median mem",
            "extra": "avg mem: 28.604636813741415, max mem: 29.68359375, count: 53870"
          },
          {
            "name": "Insert value B - Publisher - cpu",
            "value": 4.562738,
            "unit": "median cpu",
            "extra": "avg cpu: 3.614133183587163, max cpu: 4.58891, count: 53870"
          },
          {
            "name": "Insert value B - Publisher - mem",
            "value": 29.26171875,
            "unit": "median mem",
            "extra": "avg mem: 28.548496960274736, max mem: 29.59765625, count: 53870"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - cpu",
            "value": 4.5933013,
            "unit": "median cpu",
            "extra": "avg cpu: 6.580047241919054, max cpu: 27.376425, count: 53870"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - mem",
            "value": 48.6875,
            "unit": "median mem",
            "extra": "avg mem: 48.727425834184146, max mem: 54.60546875, count: 53870"
          },
          {
            "name": "SELECT\n  pid,\n  pg_wal_lsn_diff(sent_lsn, replay_lsn) AS replication_lag,\n  application_name::text,\n  state::text\nFROM pg_stat_replication; - Publisher - replication_lag:MB",
            "value": 0,
            "unit": "median replication_lag:MB",
            "extra": "avg replication_lag:MB: 0.000020361716746841353, max replication_lag:MB: 0.197021484375, count: 53870"
          },
          {
            "name": "Top N - Subscriber - cpu",
            "value": 4.5757866,
            "unit": "median cpu",
            "extra": "avg cpu: 5.1424688543265376, max cpu: 13.806328, count: 107740"
          },
          {
            "name": "Top N - Subscriber - mem",
            "value": 49.02734375,
            "unit": "median mem",
            "extra": "avg mem: 49.06044814919018, max mem: 55.09375, count: 107740"
          },
          {
            "name": "Update 1..9 - Publisher - cpu",
            "value": 4.5283017,
            "unit": "median cpu",
            "extra": "avg cpu: 2.5657847108391705, max cpu: 4.5757866, count: 53870"
          },
          {
            "name": "Update 1..9 - Publisher - mem",
            "value": 32.125,
            "unit": "median mem",
            "extra": "avg mem: 31.40674163495452, max mem: 32.4609375, count: 53870"
          },
          {
            "name": "Update 10,11 - Publisher - cpu",
            "value": 4.5714283,
            "unit": "median cpu",
            "extra": "avg cpu: 3.890577206585464, max cpu: 4.5933013, count: 53870"
          },
          {
            "name": "Update 10,11 - Publisher - mem",
            "value": 32.328125,
            "unit": "median mem",
            "extra": "avg mem: 31.670201555828847, max mem: 32.42578125, count: 53870"
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
          "id": "b91006e07d5c6fc93a8bb38ca19094ef5f3081f7",
          "message": "fix: Early return in fieldname extraction causes pushdown to not happen (#4075)\n\n# Description\nBackport of #4071 to `0.21.x`.\n\n---------\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2026-02-03T20:28:51Z",
          "url": "https://github.com/paradedb/paradedb/commit/b91006e07d5c6fc93a8bb38ca19094ef5f3081f7"
        },
        "date": 1770613976779,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - cpu",
            "value": 4.5801525,
            "unit": "median cpu",
            "extra": "avg cpu: 5.011362351657741, max cpu: 11.16279, count: 53774"
          },
          {
            "name": "Custom Scan - Subscriber - mem",
            "value": 47.41015625,
            "unit": "median mem",
            "extra": "avg mem: 47.43215710949995, max mem: 53.46875, count: 53774"
          },
          {
            "name": "Delete values - Publisher - cpu",
            "value": 4.5801525,
            "unit": "median cpu",
            "extra": "avg cpu: 3.6114012896450793, max cpu: 4.5933013, count: 53774"
          },
          {
            "name": "Delete values - Publisher - mem",
            "value": 29.42578125,
            "unit": "median mem",
            "extra": "avg mem: 28.727800392034254, max mem: 29.76953125, count: 53774"
          },
          {
            "name": "Find by ctid - Subscriber - cpu",
            "value": 9.134158,
            "unit": "median cpu",
            "extra": "avg cpu: 8.206760494907828, max cpu: 18.461538, count: 53774"
          },
          {
            "name": "Find by ctid - Subscriber - mem",
            "value": 50.1015625,
            "unit": "median mem",
            "extra": "avg mem: 49.7403888118561, max mem: 55.98828125, count: 53774"
          },
          {
            "name": "Index Only Scan - Subscriber - cpu",
            "value": 4.5757866,
            "unit": "median cpu",
            "extra": "avg cpu: 4.921775883927968, max cpu: 9.275363, count: 53774"
          },
          {
            "name": "Index Only Scan - Subscriber - mem",
            "value": 47.35546875,
            "unit": "median mem",
            "extra": "avg mem: 47.37020737252204, max mem: 53.4140625, count: 53774"
          },
          {
            "name": "Index Size Info - Subscriber - cpu",
            "value": 4.5757866,
            "unit": "median cpu",
            "extra": "avg cpu: 4.634611455544592, max cpu: 9.169055, count: 53774"
          },
          {
            "name": "Index Size Info - Subscriber - mem",
            "value": 30.88671875,
            "unit": "median mem",
            "extra": "avg mem: 30.916247352926135, max mem: 36.00390625, count: 53774"
          },
          {
            "name": "Index Size Info - Subscriber - pages",
            "value": 1132,
            "unit": "median pages",
            "extra": "avg pages: 1129.0550823818203, max pages: 1868.0, count: 53774"
          },
          {
            "name": "Index Size Info - Subscriber - relation_size:MB",
            "value": 8.84375,
            "unit": "median relation_size:MB",
            "extra": "avg relation_size:MB: 8.820742831107971, max relation_size:MB: 14.59375, count: 53774"
          },
          {
            "name": "Index Size Info - Subscriber - segment_count",
            "value": 11,
            "unit": "median segment_count",
            "extra": "avg segment_count: 9.906014058838844, max segment_count: 21.0, count: 53774"
          },
          {
            "name": "Insert value A - Publisher - cpu",
            "value": 4.562738,
            "unit": "median cpu",
            "extra": "avg cpu: 3.927405702387906, max cpu: 4.619827, count: 53774"
          },
          {
            "name": "Insert value A - Publisher - mem",
            "value": 27.60546875,
            "unit": "median mem",
            "extra": "avg mem: 26.970656779414774, max mem: 27.99609375, count: 53774"
          },
          {
            "name": "Insert value B - Publisher - cpu",
            "value": 4.5845275,
            "unit": "median cpu",
            "extra": "avg cpu: 3.139843684923496, max cpu: 4.619827, count: 53774"
          },
          {
            "name": "Insert value B - Publisher - mem",
            "value": 27.5546875,
            "unit": "median mem",
            "extra": "avg mem: 26.863222119425746, max mem: 27.89453125, count: 53774"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - cpu",
            "value": 4.6065254,
            "unit": "median cpu",
            "extra": "avg cpu: 6.781197016769595, max cpu: 22.878933, count: 53774"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - mem",
            "value": 45.44140625,
            "unit": "median mem",
            "extra": "avg mem: 45.410175354841556, max mem: 51.4375, count: 53774"
          },
          {
            "name": "SELECT\n  pid,\n  pg_wal_lsn_diff(sent_lsn, replay_lsn) AS replication_lag,\n  application_name::text,\n  state::text\nFROM pg_stat_replication; - Publisher - replication_lag:MB",
            "value": 0,
            "unit": "median replication_lag:MB",
            "extra": "avg replication_lag:MB: 0.00002106362128349086, max replication_lag:MB: 0.20034027099609375, count: 53774"
          },
          {
            "name": "Top N - Subscriber - cpu",
            "value": 4.5801525,
            "unit": "median cpu",
            "extra": "avg cpu: 5.075693321344458, max cpu: 13.740458, count: 107548"
          },
          {
            "name": "Top N - Subscriber - mem",
            "value": 46.09375,
            "unit": "median mem",
            "extra": "avg mem: 46.071380082904845, max mem: 52.31640625, count: 107548"
          },
          {
            "name": "Update 1..9 - Publisher - cpu",
            "value": 4.5540795,
            "unit": "median cpu",
            "extra": "avg cpu: 4.442151022254519, max cpu: 4.597701, count: 53774"
          },
          {
            "name": "Update 1..9 - Publisher - mem",
            "value": 30.26171875,
            "unit": "median mem",
            "extra": "avg mem: 29.56126457779317, max mem: 30.60546875, count: 53774"
          },
          {
            "name": "Update 10,11 - Publisher - cpu",
            "value": 4.567079,
            "unit": "median cpu",
            "extra": "avg cpu: 3.733582823109619, max cpu: 4.619827, count: 53774"
          },
          {
            "name": "Update 10,11 - Publisher - mem",
            "value": 30.3046875,
            "unit": "median mem",
            "extra": "avg mem: 29.590795432620784, max mem: 30.40234375, count: 53774"
          }
        ]
      }
    ]
  }
}