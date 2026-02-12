window.BENCHMARK_DATA = {
  "lastUpdate": 1770940505336,
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
          "id": "30c4c9cbf69783f13dbfa4ed26b331f441be585c",
          "message": "perf: Use parallel workers for the join scan (#4101)\n\n## Ticket(s) Closed\n\n- Closes #4063\n\n## What\n\nAdds support for parallel workers in the `joinscan`, by relying (for\nnow) on the fact that we only support INNER joins, and can thus do a\nbroadcast join.\n\n## Why\n\nTo get an implementation of parallel workers in place without (yet)\ntackling the problem of partitioning DataFusion plans across parallel\nworkers and introducing RPC.\n\n## How\n\n- Implemented a \"broadcast join\" strategy for `JoinScan` where the\nlargest index scan is partitioned across workers while the others are\nreplicated.\n- Introduced `ParallelSegmentPlan` and `ParallelScanStream` for dynamic\nworker-driven scanning.\n- This strategy is necessary in order to continue to use the lazy work\nclaiming strategy that we use in `ParallelScanState`, but after #4062\nthe replicated/un-partitioned indexes could begin using\n`MultiSegmentPlan` to provide sorted access.\n- In future, if/when we change our parallel worker partitioning\nstrategy, we might be able to use `MultiSegmentPlan` and assign _ranges_\nof an index to the parallel workers. TBD.\n- Centralized `RowEstimate` handling to better manage unanalyzed tables,\nand ease determining the largest index to scan.\n- Cleaned up registration of the `CustomScan`'s vtable\n(`CustomExecMethods`).\n- Before this, encountered some segfaults due to registration issues\naround having multiple parallel `CustomScan` implementations.\n- Remove \"lazy checkout\" from `MultiSegmentPlan`, as no consumer will\nactually use it lazily.\n\n## Tests\n\nExisting tests (and proptests) pass.\n\nBenchmarks show speedups across a few of our joins. Notably: we are\nfaster than Postgres for the `semi_join_filter` join for the first time.",
          "timestamp": "2026-02-09T17:53:18-08:00",
          "tree_id": "6616d18d10f8cf9e48caa5c264c26297828fd02b",
          "url": "https://github.com/paradedb/paradedb/commit/30c4c9cbf69783f13dbfa4ed26b331f441be585c"
        },
        "date": 1770689593064,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - tps",
            "value": 128.51739456173803,
            "unit": "median tps",
            "extra": "avg tps: 128.4523822433821, max tps: 144.61265155088174, count: 55138"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3035.3858943586038,
            "unit": "median tps",
            "extra": "avg tps: 3022.9268860247985, max tps: 3046.5175969857382, count: 55138"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 482.31371047876877,
            "unit": "median tps",
            "extra": "avg tps: 474.62668817258924, max tps: 526.6246579152279, count: 55138"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 3025.4825592029247,
            "unit": "median tps",
            "extra": "avg tps: 3008.559768770172, max tps: 3058.155176007422, count: 110276"
          },
          {
            "name": "Mixed Fast Field Scan - Primary - tps",
            "value": 511.8911877494761,
            "unit": "median tps",
            "extra": "avg tps: 508.33196769615944, max tps: 677.6274883310384, count: 55138"
          },
          {
            "name": "Normal Scan - Primary - tps",
            "value": 530.1265146878884,
            "unit": "median tps",
            "extra": "avg tps: 526.1773884751633, max tps: 664.8092958635114, count: 55138"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1973.3794901914819,
            "unit": "median tps",
            "extra": "avg tps: 1966.5044575959269, max tps: 1991.1095662762202, count: 55138"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 32.03653871588129,
            "unit": "median tps",
            "extra": "avg tps: 106.26783759314996, max tps: 362.8590272359083, count: 55138"
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
          "id": "ff10528e88ec12dd794b2a6e8b75996ad447a713",
          "message": "fix: Joinscan row estimation needs `ExprContext` (#4147)\n\n# Ticket(s) Closed\n\n- Closes #4146 \n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2026-02-10T14:16:35-05:00",
          "tree_id": "bba16dfdbd260ac92fb27be55e3b411b77039476",
          "url": "https://github.com/paradedb/paradedb/commit/ff10528e88ec12dd794b2a6e8b75996ad447a713"
        },
        "date": 1770752168729,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - tps",
            "value": 123.18450148520687,
            "unit": "median tps",
            "extra": "avg tps: 123.53700460515647, max tps: 136.47429568387093, count: 55178"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2961.030896064569,
            "unit": "median tps",
            "extra": "avg tps: 2944.087246136454, max tps: 2972.9608623947206, count: 55178"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 486.78143850478307,
            "unit": "median tps",
            "extra": "avg tps: 483.934473577815, max tps: 542.6322779791208, count: 55178"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 3007.7288610295345,
            "unit": "median tps",
            "extra": "avg tps: 3007.4863359780375, max tps: 3064.3312995077827, count: 110356"
          },
          {
            "name": "Mixed Fast Field Scan - Primary - tps",
            "value": 544.2135623362619,
            "unit": "median tps",
            "extra": "avg tps: 544.9029129893462, max tps: 635.1594270986917, count: 55178"
          },
          {
            "name": "Normal Scan - Primary - tps",
            "value": 534.1785888859409,
            "unit": "median tps",
            "extra": "avg tps: 533.4631826510314, max tps: 627.7998845671557, count: 55178"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1933.2779559865312,
            "unit": "median tps",
            "extra": "avg tps: 1920.8972277157334, max tps: 1939.581523384379, count: 55178"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 82.48756136183405,
            "unit": "median tps",
            "extra": "avg tps: 114.25498824530281, max tps: 271.68973901211984, count: 55178"
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
          "id": "084451f652ebc5c322fbf12b0189bc5e229dce3a",
          "message": "fix: reduce overhead for ngram match queries and add TEXT[] regression coverage (#4150)\n\n# Ticket(s) Closed\n\n- Closes #2884\n\n## What\n\nMinor optimization to `match_query` and new regression test covering\nngram search on TEXT[] columns with `conjunction_mode`.\n\n## Why\n\nA it's reported in #2884, slow ngram searches (~16 queries/s vs ~70\nwithout ngram) on a 350k-row TEXT[] column. We investigated and found\nthe N-way posting list intersection in `BooleanQuery` with many Must\nclauses is inherently expensive and can't be fundamentally improved at\nthe pg_search level. However, we identified two sources of unnecessary\noverhead in how `match_query` constructs the query.\n\n## How\n\n1. **`IndexRecordOption::WithFreqs` instead of `WithFreqsAndPositions`**\n— `match_query` creates `TermQuery` instances inside a `BooleanQuery`.\nThe BooleanQuery scorer only uses doc iteration and BM25 scores, never\npositions. `WithFreqsAndPositions` was requesting position data that was\nnever read. `WithFreqs` produces identical BM25 scores with less\nper-document overhead.\n\n2. **Deduplicate terms for conjunction mode** — For queries with\nrepeated ngram tokens (e.g., strings with repeated substrings),\nduplicate Must clauses add intersection work without changing which\ndocuments match. Dedup removes them before building the query.\n\nBoth changes preserve identical matching semantics and BM25 scoring.\n\n## Tests\n\nNew `ngram-text-array` regression test covering the exact pattern from\nthe reported issue: TEXT[] column with ICU + ngram alias fields, `match`\nwith `conjunction_mode`, `disjunction_max`, edge cases (short queries,\nsingle-token queries), and the JSON `::jsonb` query path.",
          "timestamp": "2026-02-10T15:11:24-08:00",
          "tree_id": "ce5fefd07b9871c52c5cd32b82b7f79613310334",
          "url": "https://github.com/paradedb/paradedb/commit/084451f652ebc5c322fbf12b0189bc5e229dce3a"
        },
        "date": 1770766269711,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - tps",
            "value": 127.13617328155539,
            "unit": "median tps",
            "extra": "avg tps: 126.73338506938191, max tps: 131.12043743726005, count: 55182"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2961.0720946763895,
            "unit": "median tps",
            "extra": "avg tps: 2946.7773327341456, max tps: 2989.199196190458, count: 55182"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 468.893761466032,
            "unit": "median tps",
            "extra": "avg tps: 467.0566005527532, max tps: 625.633780053231, count: 55182"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 3005.6715843811066,
            "unit": "median tps",
            "extra": "avg tps: 2997.2589259970273, max tps: 3039.5591209577588, count: 110364"
          },
          {
            "name": "Mixed Fast Field Scan - Primary - tps",
            "value": 537.6306225862224,
            "unit": "median tps",
            "extra": "avg tps: 533.7407204647441, max tps: 627.5103464254536, count: 55182"
          },
          {
            "name": "Normal Scan - Primary - tps",
            "value": 543.2376963665822,
            "unit": "median tps",
            "extra": "avg tps: 539.9546031841996, max tps: 611.3579427813658, count: 55182"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1908.7487905362907,
            "unit": "median tps",
            "extra": "avg tps: 1904.1916923149345, max tps: 1914.7304475215662, count: 55182"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 265.20542686076334,
            "unit": "median tps",
            "extra": "avg tps: 211.47180219514135, max tps: 335.355390599562, count: 55182"
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
          "id": "59354e0de782d993f3e4a260eb7c56ad4804a1ad",
          "message": "fix: add field validation for `paradedb.aggregate()` API (#4141)\n\n# Ticket(s) Closed\n\n- Closes #N/A\n\n## What\n\nAdds field validation to the `paradedb.aggregate()` direct SQL function\nso that referencing a nonexistent or unsupported field returns a clear\nerror instead of silently producing null results.\n\n## Why\n\nThe `pdb.agg()` aggregate path already validates fields at plan time via\n`AggregateType::validate_fields()`. However, the `paradedb.aggregate()`\nfunction is a plain `pg_extern` that calls `execute_aggregate()`\ndirectly — it never enters the custom scan planner, so it skipped\nvalidation entirely. An invalid field like `\"nonexistent_field\"` would\nquietly return `{\"value\": null}` instead of telling the user something\nis wrong.\n\n## How\n\n- Extracted the field validation logic from\n`AggregateType::validate_fields()` into a standalone\n`validate_agg_json_fields()` function in `aggregate_type.rs`. The\nexisting `validate_fields()` now delegates to it for custom aggregates.\n- Called `validate_agg_json_fields()` in `aggregate_impl()`\n(`api/aggregate.rs`) before executing, so the direct API gets the same\nvalidation as the planner path.\n\n## Tests\n\n- Added regression tests (tests 13–15 in `agg-validate.sql`) covering\nthe `paradedb.aggregate()` path: valid field succeeds, invalid field\nerrors, invalid nested field errors.",
          "timestamp": "2026-02-10T15:12:54-08:00",
          "tree_id": "a2a30dc05294896dfaef747d15452a4024f5d8aa",
          "url": "https://github.com/paradedb/paradedb/commit/59354e0de782d993f3e4a260eb7c56ad4804a1ad"
        },
        "date": 1770766361186,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - tps",
            "value": 124.46908984637332,
            "unit": "median tps",
            "extra": "avg tps: 124.04942838864258, max tps: 141.2146340667349, count: 55160"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3087.089790931307,
            "unit": "median tps",
            "extra": "avg tps: 3063.646407012724, max tps: 3103.168633503268, count: 55160"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 454.4472351692372,
            "unit": "median tps",
            "extra": "avg tps: 451.32415083417334, max tps: 538.7382095411183, count: 55160"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 3106.393087977146,
            "unit": "median tps",
            "extra": "avg tps: 3096.66646739803, max tps: 3126.9605887868856, count: 110320"
          },
          {
            "name": "Mixed Fast Field Scan - Primary - tps",
            "value": 520.4348201663356,
            "unit": "median tps",
            "extra": "avg tps: 516.6190201749807, max tps: 641.0854345709807, count: 55160"
          },
          {
            "name": "Normal Scan - Primary - tps",
            "value": 539.4345715187845,
            "unit": "median tps",
            "extra": "avg tps: 534.9728883260256, max tps: 615.1854025772762, count: 55160"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1959.3422364749827,
            "unit": "median tps",
            "extra": "avg tps: 1945.7876502540364, max tps: 1966.313121916722, count: 55160"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 48.34129545595227,
            "unit": "median tps",
            "extra": "avg tps: 71.2559536476258, max tps: 294.2350814722229, count: 55160"
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
          "id": "44d1f928882599cf5fd9fbc853c8eee1fb5c57ed",
          "message": "fix: Rebase against Tantivy, inherit upstream bugfix for intersection queries (#4155)\n\n# Ticket(s) Closed\n\n- Closes #4149 \n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2026-02-11T11:47:17-05:00",
          "tree_id": "b88ad04015b13dee26d44c4c9d585ea252d0323c",
          "url": "https://github.com/paradedb/paradedb/commit/44d1f928882599cf5fd9fbc853c8eee1fb5c57ed"
        },
        "date": 1770829294469,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - tps",
            "value": 126.14075458889957,
            "unit": "median tps",
            "extra": "avg tps: 126.07726606714783, max tps: 133.5404837057107, count: 8378"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2868.1868425494176,
            "unit": "median tps",
            "extra": "avg tps: 2881.5368466785917, max tps: 3026.175252016383, count: 8378"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 465.2101641805812,
            "unit": "median tps",
            "extra": "avg tps: 469.40653177643566, max tps: 555.2385040875271, count: 8378"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 2943.547815046841,
            "unit": "median tps",
            "extra": "avg tps: 2929.5123454286145, max tps: 3006.87667469181, count: 16756"
          },
          {
            "name": "Mixed Fast Field Scan - Primary - tps",
            "value": 532.2417509682589,
            "unit": "median tps",
            "extra": "avg tps: 534.4452930366144, max tps: 661.5550100153812, count: 8378"
          },
          {
            "name": "Normal Scan - Primary - tps",
            "value": 548.6533022530181,
            "unit": "median tps",
            "extra": "avg tps: 547.054619833619, max tps: 647.4375752294977, count: 8378"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1800.1068474558576,
            "unit": "median tps",
            "extra": "avg tps: 1801.5266779968372, max tps: 1834.4707212251358, count: 8378"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 33.13519010347301,
            "unit": "median tps",
            "extra": "avg tps: 42.34235592118829, max tps: 212.05156924522788, count: 8378"
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
          "id": "80daf35915e5edde9b7e091036a88ce3d6c6aea1",
          "message": "chore: Upgrade to `0.21.8` (#4168)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2026-02-12T09:45:33-05:00",
          "tree_id": "3de2b0c6e4f9a21b26370ecab28ddd8db57c65ff",
          "url": "https://github.com/paradedb/paradedb/commit/80daf35915e5edde9b7e091036a88ce3d6c6aea1"
        },
        "date": 1770908838324,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - tps",
            "value": 121.82484787757484,
            "unit": "median tps",
            "extra": "avg tps: 122.27216091008334, max tps: 136.30236504299043, count: 55136"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3023.362181800181,
            "unit": "median tps",
            "extra": "avg tps: 3011.154362646014, max tps: 3042.085627231733, count: 55136"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 462.8875625819791,
            "unit": "median tps",
            "extra": "avg tps: 464.42911081292823, max tps: 540.5766763868359, count: 55136"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 3063.7302810633187,
            "unit": "median tps",
            "extra": "avg tps: 3053.8372194197063, max tps: 3092.3547397247194, count: 110272"
          },
          {
            "name": "Mixed Fast Field Scan - Primary - tps",
            "value": 507.1006774810483,
            "unit": "median tps",
            "extra": "avg tps: 508.86647312962566, max tps: 626.0144974822144, count: 55136"
          },
          {
            "name": "Normal Scan - Primary - tps",
            "value": 532.3863015285766,
            "unit": "median tps",
            "extra": "avg tps: 533.3878291943332, max tps: 604.9356205772708, count: 55136"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1976.0771817905215,
            "unit": "median tps",
            "extra": "avg tps: 1957.440704158337, max tps: 1983.197496114219, count: 55136"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 37.43009397907358,
            "unit": "median tps",
            "extra": "avg tps: 37.15407877220718, max tps: 196.63645478629846, count: 55136"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "2aec8447addadd4def1cf10f4d11e24c1755fadb",
          "message": "chore: Remove tuned_postgres from /benchmarks (#4167)\n\n# Ticket(s) Closed\n\n- Closes #N/A\n\n## What\nWe are moving user-facing benchmarks to a much more complete project,\nwhich will be in a separate repository. This is no longer relevant to\nkeep here, so removing.\n\n## Why\n^\n\n## How\n^\n\n## Tests\n^",
          "timestamp": "2026-02-12T12:21:38-05:00",
          "tree_id": "3502c5fdf7ad1b45110e95b70c181dd5ca1eae37",
          "url": "https://github.com/paradedb/paradedb/commit/2aec8447addadd4def1cf10f4d11e24c1755fadb"
        },
        "date": 1770918054114,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - tps",
            "value": 119.10836833000177,
            "unit": "median tps",
            "extra": "avg tps: 119.95932887729525, max tps: 138.84620073735528, count: 55205"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3118.4052924634066,
            "unit": "median tps",
            "extra": "avg tps: 3105.2085314225833, max tps: 3130.8197388971103, count: 55205"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 453.5268576945254,
            "unit": "median tps",
            "extra": "avg tps: 459.5392362909417, max tps: 582.1849795256459, count: 55205"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 3095.5321423679775,
            "unit": "median tps",
            "extra": "avg tps: 3085.1613199233852, max tps: 3125.4474074020804, count: 110410"
          },
          {
            "name": "Mixed Fast Field Scan - Primary - tps",
            "value": 516.363616689627,
            "unit": "median tps",
            "extra": "avg tps: 520.041514432194, max tps: 644.3915347532587, count: 55205"
          },
          {
            "name": "Normal Scan - Primary - tps",
            "value": 495.8318007343751,
            "unit": "median tps",
            "extra": "avg tps: 502.17854339646857, max tps: 647.3441852779786, count: 55205"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1990.3128354068788,
            "unit": "median tps",
            "extra": "avg tps: 1978.5284117615913, max tps: 1996.6304219651201, count: 55205"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 58.339612822675925,
            "unit": "median tps",
            "extra": "avg tps: 85.86302310157052, max tps: 231.07441608303816, count: 55205"
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
          "id": "ba868f34636e9fc6068c68b3b0d8a098eb4971d8",
          "message": "feat: join-scan: pre-materialization dynamic filter pushdown from TopK and HashJoin (#4161)\n\n## Ticket(s) Closed\n\n- Closes #4151\n\n## What\n\nDynamic filters from DataFusion's `SortExec(TopK)` and `HashJoinExec`\nare now pushed down into `PgSearchScan` and applied *before* column\nmaterialization — at the term-ordinal level for strings and the\nfast-field level for numerics. This avoids expensive term dictionary I/O\nfor documents that will be discarded anyway.\n\n## Why\n\nPreviously, `PgSearchScan` had no awareness of dynamic filters. Every\ndocument that passed the Tantivy query and visibility checks was fully\nmaterialized (all fast-field columns loaded, string dictionaries walked)\nbefore any join-key or TopK pruning could happen upstream. For selective\njoins or tight LIMIT queries, this meant loading data for rows that were\nimmediately thrown away by HashJoin or TopK.\n\n## How\n\n- Enabled DataFusion's TopK dynamic filter pushdown in the JoinScan\nsession config.\n- `SegmentPlan` now accepts dynamic filters from parent operators (TopK\nthresholds, HashJoin key bounds) and passes them to the Scanner on each\nbatch.\n- Before column materialization, the Scanner converts these filters to\nterm-ordinal comparisons (for strings) or direct fast-field comparisons\n(for numerics) and prunes non-matching documents in-place — skipping\ndictionary I/O entirely for pruned rows.\n\n## Tests\n\n- New `topk_dynamic_filter` regression test covering. You can take a\nlook at EXPLAIN ANALYZE diff in the follow-up PR (#4162):\nhttps://github.com/paradedb/paradedb/blob/3b074a9b5516a7a0a75a948201ef32e07b0127e4/pg_search/tests/pg_regress/expected/topk_dynamic_filter.out#L170-L181\n- All existing regression tests pass.",
          "timestamp": "2026-02-12T10:25:25-08:00",
          "tree_id": "748bfdacf0d0b82f9ceb26840b3100a7ca8e2252",
          "url": "https://github.com/paradedb/paradedb/commit/ba868f34636e9fc6068c68b3b0d8a098eb4971d8"
        },
        "date": 1770921878686,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - tps",
            "value": 121.1599199120167,
            "unit": "median tps",
            "extra": "avg tps: 121.22819828697033, max tps: 130.719549613635, count: 55154"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3083.3901266908283,
            "unit": "median tps",
            "extra": "avg tps: 3063.255471226749, max tps: 3093.4965493654486, count: 55154"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 428.87000742958406,
            "unit": "median tps",
            "extra": "avg tps: 428.7736613734014, max tps: 518.7941615423854, count: 55154"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 2986.9912449001354,
            "unit": "median tps",
            "extra": "avg tps: 2970.3030568487443, max tps: 3018.7796685552516, count: 110308"
          },
          {
            "name": "Mixed Fast Field Scan - Primary - tps",
            "value": 505.0338470048913,
            "unit": "median tps",
            "extra": "avg tps: 505.65472206502676, max tps: 631.1722482083288, count: 55154"
          },
          {
            "name": "Normal Scan - Primary - tps",
            "value": 508.32042908647327,
            "unit": "median tps",
            "extra": "avg tps: 507.89457400399425, max tps: 655.332479810563, count: 55154"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1950.601435257427,
            "unit": "median tps",
            "extra": "avg tps: 1935.4472973148333, max tps: 1956.3426939632577, count: 55154"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 49.75852474812861,
            "unit": "median tps",
            "extra": "avg tps: 56.06089231239807, max tps: 256.2363770328673, count: 55154"
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
          "id": "655ee8b04cab31c056cb41a89d677b896630ed16",
          "message": "feat: join-scan: surface dynamic filter metrics in EXPLAIN ANALYZE (#4162)\n\n# Ticket(s) Closed\n\n- Closes #4151\n\n## What\n\nUsed DataFusion metrics, and made dynamic filter pruning stats visible\nthrough `EXPLAIN ANALYZE`.\n\n## Why\n\n`EXPLAIN ANALYZE` is the natural place for execution-time stats.\n\n## How\n\n- Added `ExecutionPlanMetricsSet` to `SegmentPlan` with two custom\ncounters (`rows_scanned`, `rows_pruned`), only registered when dynamic\nfilters are present.\n\n## Tests\n\n- Updated `topk_dynamic_filter` regression test to use `EXPLAIN\n(ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)` — verifying\n`Dynamic Filter` lines appear with correct pruning stats (e.g., `30\nscanned, 24 pruned (80.0%)`).\n- Updated `join_custom_scan` and `filter_pushdown_datafusion` expected\noutput.\n\n---------\n\nSigned-off-by: Moe <mdashti@gmail.com>",
          "timestamp": "2026-02-12T12:45:44-08:00",
          "tree_id": "8c73104c0b40b30047e010ebfba45fb9add3f7e8",
          "url": "https://github.com/paradedb/paradedb/commit/655ee8b04cab31c056cb41a89d677b896630ed16"
        },
        "date": 1770930317017,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - tps",
            "value": 123.32722629746671,
            "unit": "median tps",
            "extra": "avg tps: 123.4661181642466, max tps: 134.7099137353074, count: 55049"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3024.3236245766443,
            "unit": "median tps",
            "extra": "avg tps: 2999.162435511663, max tps: 3043.662070598802, count: 55049"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 472.15928554602897,
            "unit": "median tps",
            "extra": "avg tps: 471.11228165774, max tps: 560.0494187607114, count: 55049"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 2996.690628331249,
            "unit": "median tps",
            "extra": "avg tps: 3009.47013411538, max tps: 3080.865744045185, count: 110098"
          },
          {
            "name": "Mixed Fast Field Scan - Primary - tps",
            "value": 532.1948788517838,
            "unit": "median tps",
            "extra": "avg tps: 533.6494191613309, max tps: 647.4814851273893, count: 55049"
          },
          {
            "name": "Normal Scan - Primary - tps",
            "value": 552.6400657089289,
            "unit": "median tps",
            "extra": "avg tps: 555.8937120941553, max tps: 625.8261629687023, count: 55049"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1992.8577118495741,
            "unit": "median tps",
            "extra": "avg tps: 1976.7852673728444, max tps: 2005.5229986816405, count: 55049"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 45.53941274012749,
            "unit": "median tps",
            "extra": "avg tps: 47.28254017129468, max tps: 568.955079858535, count: 55049"
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
          "id": "1da48a637b80cc0ce2312ab8c4e448762e152223",
          "message": "perf: Add statistics to joinscan (#4132)\n\n## Ticket(s) Closed\n\n- Closes #4062.\n\n## What\n\n* Exposes sorting from the joinscan's `TableProvider`, but does not yet\nforce `SortMergeJoin`.\n* Adds statistics on `TableProvider` and our `ExecutionPlan`s using\nTantivy's query estimates.\n* Removes the `ParallelSegmentPlan` that was added in #4101, as it makes\nmore sense to let DataFusion coalesce for us if needed.\n\n## Why\n\nTo allow the DataFusion optimizer to re-order joins based on table\nsizes, and use sortedness in plans (although it does not yet by\ndefault).\n\n## Tests\n\nExisting tests show the impact of join reordering due to statistics.",
          "timestamp": "2026-02-12T14:34:01-08:00",
          "tree_id": "fbc185b154055782f4973f483feb5ad00a4ca2bb",
          "url": "https://github.com/paradedb/paradedb/commit/1da48a637b80cc0ce2312ab8c4e448762e152223"
        },
        "date": 1770936796218,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - tps",
            "value": 124.91451061026538,
            "unit": "median tps",
            "extra": "avg tps: 125.56931345310355, max tps: 145.30187545308075, count: 55002"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3130.929996622497,
            "unit": "median tps",
            "extra": "avg tps: 3122.7827855974338, max tps: 3146.7893308457383, count: 55002"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 460.9737542522349,
            "unit": "median tps",
            "extra": "avg tps: 463.4702341731758, max tps: 567.9766258685901, count: 55002"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 3077.1470642849836,
            "unit": "median tps",
            "extra": "avg tps: 3058.6390718197304, max tps: 3126.0869199536414, count: 110004"
          },
          {
            "name": "Mixed Fast Field Scan - Primary - tps",
            "value": 501.2495105980829,
            "unit": "median tps",
            "extra": "avg tps: 506.13861308668135, max tps: 664.7033637299874, count: 55002"
          },
          {
            "name": "Normal Scan - Primary - tps",
            "value": 549.5659231108259,
            "unit": "median tps",
            "extra": "avg tps: 552.6118713490107, max tps: 632.8632325968939, count: 55002"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1982.235439864849,
            "unit": "median tps",
            "extra": "avg tps: 1965.7833440557808, max tps: 1988.930300170519, count: 55002"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 55.09741975785236,
            "unit": "median tps",
            "extra": "avg tps: 98.66840427846833, max tps: 227.2937433343742, count: 55002"
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
          "id": "b04eae77b43a69abf89d75cd16038e7fcdd72770",
          "message": "refactor: unify static OPERATOR_LOOKUP into shared function (#4060) (#4173)\n\n## Summary\n\n- Deduplicate the `OnceLock<HashMap<PostgresOperatorOid,\nTantivyOperator>>` pattern that was copy-pasted across `pushdown.rs`,\n`planning.rs`, and `translator.rs`\n- Centralize into a single `pub(crate) fn lookup_operator(opno)` in\n`opexpr.rs`\n- Privatize `OperatorAccepts` and `initialize_equality_operator_lookup`\nsince they are no longer needed outside `opexpr.rs`\n\nCloses #4060\n\n## Test plan\n\nNo new tests added. This is a pure refactoring with no behavioral\nchange. Existing integration and regression tests provide full coverage\nof all modified call sites.",
          "timestamp": "2026-02-12T15:09:31-08:00",
          "tree_id": "4395a9fad346a49f0bcb0a75092401e526f3830c",
          "url": "https://github.com/paradedb/paradedb/commit/b04eae77b43a69abf89d75cd16038e7fcdd72770"
        },
        "date": 1770938919320,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - tps",
            "value": 125.14942756551274,
            "unit": "median tps",
            "extra": "avg tps: 125.85625076241112, max tps: 135.90092933759743, count: 55147"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2979.9328540666083,
            "unit": "median tps",
            "extra": "avg tps: 2968.7107826271854, max tps: 3038.8539873842774, count: 55147"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 452.1658673090475,
            "unit": "median tps",
            "extra": "avg tps: 457.47747372962175, max tps: 631.7582337050599, count: 55147"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 3009.56688546176,
            "unit": "median tps",
            "extra": "avg tps: 3002.932545044477, max tps: 3031.4188673642357, count: 110294"
          },
          {
            "name": "Mixed Fast Field Scan - Primary - tps",
            "value": 515.3907160660846,
            "unit": "median tps",
            "extra": "avg tps: 517.8926483020041, max tps: 670.1722549952312, count: 55147"
          },
          {
            "name": "Normal Scan - Primary - tps",
            "value": 509.25393148572107,
            "unit": "median tps",
            "extra": "avg tps: 513.7250017188857, max tps: 645.4572463508334, count: 55147"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1928.1074549508814,
            "unit": "median tps",
            "extra": "avg tps: 1923.6140607215286, max tps: 1939.9349199361036, count: 55147"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 42.65998250411346,
            "unit": "median tps",
            "extra": "avg tps: 83.39627466120139, max tps: 343.905563532254, count: 55147"
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
          "id": "30c4c9cbf69783f13dbfa4ed26b331f441be585c",
          "message": "perf: Use parallel workers for the join scan (#4101)\n\n## Ticket(s) Closed\n\n- Closes #4063\n\n## What\n\nAdds support for parallel workers in the `joinscan`, by relying (for\nnow) on the fact that we only support INNER joins, and can thus do a\nbroadcast join.\n\n## Why\n\nTo get an implementation of parallel workers in place without (yet)\ntackling the problem of partitioning DataFusion plans across parallel\nworkers and introducing RPC.\n\n## How\n\n- Implemented a \"broadcast join\" strategy for `JoinScan` where the\nlargest index scan is partitioned across workers while the others are\nreplicated.\n- Introduced `ParallelSegmentPlan` and `ParallelScanStream` for dynamic\nworker-driven scanning.\n- This strategy is necessary in order to continue to use the lazy work\nclaiming strategy that we use in `ParallelScanState`, but after #4062\nthe replicated/un-partitioned indexes could begin using\n`MultiSegmentPlan` to provide sorted access.\n- In future, if/when we change our parallel worker partitioning\nstrategy, we might be able to use `MultiSegmentPlan` and assign _ranges_\nof an index to the parallel workers. TBD.\n- Centralized `RowEstimate` handling to better manage unanalyzed tables,\nand ease determining the largest index to scan.\n- Cleaned up registration of the `CustomScan`'s vtable\n(`CustomExecMethods`).\n- Before this, encountered some segfaults due to registration issues\naround having multiple parallel `CustomScan` implementations.\n- Remove \"lazy checkout\" from `MultiSegmentPlan`, as no consumer will\nactually use it lazily.\n\n## Tests\n\nExisting tests (and proptests) pass.\n\nBenchmarks show speedups across a few of our joins. Notably: we are\nfaster than Postgres for the `semi_join_filter` join for the first time.",
          "timestamp": "2026-02-09T17:53:18-08:00",
          "tree_id": "6616d18d10f8cf9e48caa5c264c26297828fd02b",
          "url": "https://github.com/paradedb/paradedb/commit/30c4c9cbf69783f13dbfa4ed26b331f441be585c"
        },
        "date": 1770689598411,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - cpu",
            "value": 9.239654,
            "unit": "median cpu",
            "extra": "avg cpu: 8.135305058972593, max cpu: 23.59882, count: 55138"
          },
          {
            "name": "Aggregate Custom Scan - Primary - mem",
            "value": 63.13671875,
            "unit": "median mem",
            "extra": "avg mem: 62.962134567471615, max mem: 74.26953125, count: 55138"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.641103795843249, max cpu: 9.221902, count: 55138"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 35.234375,
            "unit": "median mem",
            "extra": "avg mem: 35.0771790071049, max mem: 37.27734375, count: 55138"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.627651369067488, max cpu: 9.230769, count: 55138"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 61.67578125,
            "unit": "median mem",
            "extra": "avg mem: 61.25555254543146, max mem: 72.9375, count: 55138"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.635773490505873, max cpu: 9.302325, count: 110276"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 49.4140625,
            "unit": "median mem",
            "extra": "avg mem: 49.22245193310421, max mem: 60.40234375, count: 110276"
          },
          {
            "name": "Mixed Fast Field Scan - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 5.491172226480156, max cpu: 19.009901, count: 55138"
          },
          {
            "name": "Mixed Fast Field Scan - Primary - mem",
            "value": 62.19140625,
            "unit": "median mem",
            "extra": "avg mem: 62.11503586739182, max mem: 73.44921875, count: 55138"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 1788,
            "unit": "median block_count",
            "extra": "avg block_count: 1788.4568718488158, max block_count: 3161.0, count: 55138"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 16,
            "unit": "median segment_count",
            "extra": "avg segment_count: 16.07463092604012, max segment_count: 31.0, count: 55138"
          },
          {
            "name": "Normal Scan - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 5.454125935488214, max cpu: 14.229248, count: 55138"
          },
          {
            "name": "Normal Scan - Primary - mem",
            "value": 62.203125,
            "unit": "median mem",
            "extra": "avg mem: 62.11130042461642, max mem: 73.3671875, count: 55138"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.642600667926891, max cpu: 9.421001, count: 55138"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 51.953125,
            "unit": "median mem",
            "extra": "avg mem: 51.79941968050618, max mem: 62.7265625, count: 55138"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.610951,
            "unit": "median cpu",
            "extra": "avg cpu: 3.2078166230886254, max cpu: 9.239654, count: 55138"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 54.578125,
            "unit": "median mem",
            "extra": "avg mem: 53.499304514921654, max mem: 66.70703125, count: 55138"
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
          "id": "ff10528e88ec12dd794b2a6e8b75996ad447a713",
          "message": "fix: Joinscan row estimation needs `ExprContext` (#4147)\n\n# Ticket(s) Closed\n\n- Closes #4146 \n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2026-02-10T14:16:35-05:00",
          "tree_id": "bba16dfdbd260ac92fb27be55e3b411b77039476",
          "url": "https://github.com/paradedb/paradedb/commit/ff10528e88ec12dd794b2a6e8b75996ad447a713"
        },
        "date": 1770752173489,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - cpu",
            "value": 9.239654,
            "unit": "median cpu",
            "extra": "avg cpu: 8.192878059786237, max cpu: 23.099133, count: 55178"
          },
          {
            "name": "Aggregate Custom Scan - Primary - mem",
            "value": 63.09375,
            "unit": "median mem",
            "extra": "avg mem: 62.879040970246294, max mem: 74.1796875, count: 55178"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.635553432483242, max cpu: 9.239654, count: 55178"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 35.359375,
            "unit": "median mem",
            "extra": "avg mem: 35.31506846308764, max mem: 37.390625, count: 55178"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.524043654378884, max cpu: 4.833837, count: 55178"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 61.43359375,
            "unit": "median mem",
            "extra": "avg mem: 60.85069648172732, max mem: 72.6015625, count: 55178"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.6417614453071305, max cpu: 9.311348, count: 110356"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 49.51953125,
            "unit": "median mem",
            "extra": "avg mem: 49.29646885703994, max mem: 60.33203125, count: 110356"
          },
          {
            "name": "Mixed Fast Field Scan - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 5.25950548020731, max cpu: 18.303146, count: 55178"
          },
          {
            "name": "Mixed Fast Field Scan - Primary - mem",
            "value": 62.24609375,
            "unit": "median mem",
            "extra": "avg mem: 62.05151247723278, max mem: 73.421875, count: 55178"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 1786,
            "unit": "median block_count",
            "extra": "avg block_count: 1783.4936568922396, max block_count: 3151.0, count: 55178"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 12,
            "unit": "median segment_count",
            "extra": "avg segment_count: 11.607089782159557, max segment_count: 24.0, count: 55178"
          },
          {
            "name": "Normal Scan - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 5.3040372441261345, max cpu: 18.768328, count: 55178"
          },
          {
            "name": "Normal Scan - Primary - mem",
            "value": 62.02734375,
            "unit": "median mem",
            "extra": "avg mem: 61.87604080770416, max mem: 73.2578125, count: 55178"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.446057585621564, max cpu: 4.7197638, count: 55178"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 52.171875,
            "unit": "median mem",
            "extra": "avg mem: 51.96485238682084, max mem: 62.80078125, count: 55178"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.303266726605848, max cpu: 4.669261, count: 55178"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 55.0625,
            "unit": "median mem",
            "extra": "avg mem: 54.272079041352534, max mem: 67.3671875, count: 55178"
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
          "id": "084451f652ebc5c322fbf12b0189bc5e229dce3a",
          "message": "fix: reduce overhead for ngram match queries and add TEXT[] regression coverage (#4150)\n\n# Ticket(s) Closed\n\n- Closes #2884\n\n## What\n\nMinor optimization to `match_query` and new regression test covering\nngram search on TEXT[] columns with `conjunction_mode`.\n\n## Why\n\nA it's reported in #2884, slow ngram searches (~16 queries/s vs ~70\nwithout ngram) on a 350k-row TEXT[] column. We investigated and found\nthe N-way posting list intersection in `BooleanQuery` with many Must\nclauses is inherently expensive and can't be fundamentally improved at\nthe pg_search level. However, we identified two sources of unnecessary\noverhead in how `match_query` constructs the query.\n\n## How\n\n1. **`IndexRecordOption::WithFreqs` instead of `WithFreqsAndPositions`**\n— `match_query` creates `TermQuery` instances inside a `BooleanQuery`.\nThe BooleanQuery scorer only uses doc iteration and BM25 scores, never\npositions. `WithFreqsAndPositions` was requesting position data that was\nnever read. `WithFreqs` produces identical BM25 scores with less\nper-document overhead.\n\n2. **Deduplicate terms for conjunction mode** — For queries with\nrepeated ngram tokens (e.g., strings with repeated substrings),\nduplicate Must clauses add intersection work without changing which\ndocuments match. Dedup removes them before building the query.\n\nBoth changes preserve identical matching semantics and BM25 scoring.\n\n## Tests\n\nNew `ngram-text-array` regression test covering the exact pattern from\nthe reported issue: TEXT[] column with ICU + ngram alias fields, `match`\nwith `conjunction_mode`, `disjunction_max`, edge cases (short queries,\nsingle-token queries), and the JSON `::jsonb` query path.",
          "timestamp": "2026-02-10T15:11:24-08:00",
          "tree_id": "ce5fefd07b9871c52c5cd32b82b7f79613310334",
          "url": "https://github.com/paradedb/paradedb/commit/084451f652ebc5c322fbf12b0189bc5e229dce3a"
        },
        "date": 1770766274436,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - cpu",
            "value": 9.248554,
            "unit": "median cpu",
            "extra": "avg cpu: 8.22921346606804, max cpu: 23.369036, count: 55182"
          },
          {
            "name": "Aggregate Custom Scan - Primary - mem",
            "value": 63.19921875,
            "unit": "median mem",
            "extra": "avg mem: 62.87380353308144, max mem: 74.36328125, count: 55182"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.6360412185760485, max cpu: 9.29332, count: 55182"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 34.23828125,
            "unit": "median mem",
            "extra": "avg mem: 34.191710569683046, max mem: 35.55078125, count: 55182"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.491934702549757, max cpu: 9.213051, count: 55182"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 61.67578125,
            "unit": "median mem",
            "extra": "avg mem: 61.04986899883567, max mem: 72.92578125, count: 55182"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.583658430182306, max cpu: 9.320388, count: 110364"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 49.66015625,
            "unit": "median mem",
            "extra": "avg mem: 49.307457170137, max mem: 60.390625, count: 110364"
          },
          {
            "name": "Mixed Fast Field Scan - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 5.330492885719985, max cpu: 18.550726, count: 55182"
          },
          {
            "name": "Mixed Fast Field Scan - Primary - mem",
            "value": 62.34765625,
            "unit": "median mem",
            "extra": "avg mem: 62.085424779026674, max mem: 73.57421875, count: 55182"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 1840,
            "unit": "median block_count",
            "extra": "avg block_count: 1821.7869413939327, max block_count: 3210.0, count: 55182"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 12,
            "unit": "median segment_count",
            "extra": "avg segment_count: 13.209814794679424, max segment_count: 30.0, count: 55182"
          },
          {
            "name": "Normal Scan - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 5.34014572914909, max cpu: 18.479307, count: 55182"
          },
          {
            "name": "Normal Scan - Primary - mem",
            "value": 62.41015625,
            "unit": "median mem",
            "extra": "avg mem: 62.07321673904081, max mem: 73.515625, count: 55182"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.578849176816376, max cpu: 4.7999997, count: 55182"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 52.37890625,
            "unit": "median mem",
            "extra": "avg mem: 51.78627046070277, max mem: 62.91015625, count: 55182"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 0,
            "unit": "median cpu",
            "extra": "avg cpu: 1.076284347426473, max cpu: 4.7619047, count: 55182"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 54.09765625,
            "unit": "median mem",
            "extra": "avg mem: 52.05010422896053, max mem: 66.0390625, count: 55182"
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
          "id": "59354e0de782d993f3e4a260eb7c56ad4804a1ad",
          "message": "fix: add field validation for `paradedb.aggregate()` API (#4141)\n\n# Ticket(s) Closed\n\n- Closes #N/A\n\n## What\n\nAdds field validation to the `paradedb.aggregate()` direct SQL function\nso that referencing a nonexistent or unsupported field returns a clear\nerror instead of silently producing null results.\n\n## Why\n\nThe `pdb.agg()` aggregate path already validates fields at plan time via\n`AggregateType::validate_fields()`. However, the `paradedb.aggregate()`\nfunction is a plain `pg_extern` that calls `execute_aggregate()`\ndirectly — it never enters the custom scan planner, so it skipped\nvalidation entirely. An invalid field like `\"nonexistent_field\"` would\nquietly return `{\"value\": null}` instead of telling the user something\nis wrong.\n\n## How\n\n- Extracted the field validation logic from\n`AggregateType::validate_fields()` into a standalone\n`validate_agg_json_fields()` function in `aggregate_type.rs`. The\nexisting `validate_fields()` now delegates to it for custom aggregates.\n- Called `validate_agg_json_fields()` in `aggregate_impl()`\n(`api/aggregate.rs`) before executing, so the direct API gets the same\nvalidation as the planner path.\n\n## Tests\n\n- Added regression tests (tests 13–15 in `agg-validate.sql`) covering\nthe `paradedb.aggregate()` path: valid field succeeds, invalid field\nerrors, invalid nested field errors.",
          "timestamp": "2026-02-10T15:12:54-08:00",
          "tree_id": "a2a30dc05294896dfaef747d15452a4024f5d8aa",
          "url": "https://github.com/paradedb/paradedb/commit/59354e0de782d993f3e4a260eb7c56ad4804a1ad"
        },
        "date": 1770766366508,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - cpu",
            "value": 9.266409,
            "unit": "median cpu",
            "extra": "avg cpu: 8.239145325422937, max cpu: 23.645319, count: 55160"
          },
          {
            "name": "Aggregate Custom Scan - Primary - mem",
            "value": 63.00390625,
            "unit": "median mem",
            "extra": "avg mem: 62.86292815785895, max mem: 74.16796875, count: 55160"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.629100550080108, max cpu: 9.284333, count: 55160"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 35.4609375,
            "unit": "median mem",
            "extra": "avg mem: 35.37526329654641, max mem: 37.703125, count: 55160"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.621325513382449, max cpu: 9.275363, count: 55160"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 61.29296875,
            "unit": "median mem",
            "extra": "avg mem: 60.74161133520667, max mem: 72.63671875, count: 55160"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.658163862693781, max cpu: 9.257474, count: 110320"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 49.1796875,
            "unit": "median mem",
            "extra": "avg mem: 49.09053375266271, max mem: 60.13671875, count: 110320"
          },
          {
            "name": "Mixed Fast Field Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 5.365750996486545, max cpu: 18.568666, count: 55160"
          },
          {
            "name": "Mixed Fast Field Scan - Primary - mem",
            "value": 62.0390625,
            "unit": "median mem",
            "extra": "avg mem: 61.904293688247826, max mem: 73.2421875, count: 55160"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 1775,
            "unit": "median block_count",
            "extra": "avg block_count: 1771.6580674401741, max block_count: 3147.0, count: 55160"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 16,
            "unit": "median segment_count",
            "extra": "avg segment_count: 15.892893401015229, max segment_count: 30.0, count: 55160"
          },
          {
            "name": "Normal Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 5.317422750919269, max cpu: 18.972332, count: 55160"
          },
          {
            "name": "Normal Scan - Primary - mem",
            "value": 62.03125,
            "unit": "median mem",
            "extra": "avg mem: 61.879230803004894, max mem: 73.26953125, count: 55160"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.552775604553044, max cpu: 4.733728, count: 55160"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 51.625,
            "unit": "median mem",
            "extra": "avg mem: 51.54410549990936, max mem: 62.359375, count: 55160"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 2.7356350407348518, max cpu: 4.655674, count: 55160"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 54.69140625,
            "unit": "median mem",
            "extra": "avg mem: 53.842597316329766, max mem: 66.7109375, count: 55160"
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
          "id": "44d1f928882599cf5fd9fbc853c8eee1fb5c57ed",
          "message": "fix: Rebase against Tantivy, inherit upstream bugfix for intersection queries (#4155)\n\n# Ticket(s) Closed\n\n- Closes #4149 \n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2026-02-11T11:47:17-05:00",
          "tree_id": "b88ad04015b13dee26d44c4c9d585ea252d0323c",
          "url": "https://github.com/paradedb/paradedb/commit/44d1f928882599cf5fd9fbc853c8eee1fb5c57ed"
        },
        "date": 1770829301454,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - cpu",
            "value": 9.221902,
            "unit": "median cpu",
            "extra": "avg cpu: 8.249969129863826, max cpu: 19.238478, count: 8378"
          },
          {
            "name": "Aggregate Custom Scan - Primary - mem",
            "value": 53.15625,
            "unit": "median mem",
            "extra": "avg mem: 52.362486758474574, max mem: 55.09375, count: 8378"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.619827,
            "unit": "median cpu",
            "extra": "avg cpu: 4.665560410694498, max cpu: 9.365853, count: 8378"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 33.91015625,
            "unit": "median mem",
            "extra": "avg mem: 33.12990449316663, max mem: 33.93359375, count: 8378"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.498919009404736, max cpu: 4.7058825, count: 8378"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 49.4296875,
            "unit": "median mem",
            "extra": "avg mem: 49.19290897663524, max mem: 51.93359375, count: 8378"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6153846,
            "unit": "median cpu",
            "extra": "avg cpu: 4.553586902560542, max cpu: 4.7105007, count: 16756"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 39.71484375,
            "unit": "median mem",
            "extra": "avg mem: 38.80154357021365, max mem: 41.32421875, count: 16756"
          },
          {
            "name": "Mixed Fast Field Scan - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 5.374587190642078, max cpu: 14.159292, count: 8378"
          },
          {
            "name": "Mixed Fast Field Scan - Primary - mem",
            "value": 52.41015625,
            "unit": "median mem",
            "extra": "avg mem: 51.573635936380995, max mem: 54.2578125, count: 8378"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 605,
            "unit": "median block_count",
            "extra": "avg block_count: 603.6586297445691, max block_count: 805.0, count: 8378"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 6,
            "unit": "median segment_count",
            "extra": "avg segment_count: 5.405944139412748, max segment_count: 9.0, count: 8378"
          },
          {
            "name": "Normal Scan - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 5.2469019285389935, max cpu: 14.131501, count: 8378"
          },
          {
            "name": "Normal Scan - Primary - mem",
            "value": 52.328125,
            "unit": "median mem",
            "extra": "avg mem: 51.53700586730127, max mem: 54.2109375, count: 8378"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6021094,
            "unit": "median cpu",
            "extra": "avg cpu: 3.968464364776814, max cpu: 4.7151275, count: 8378"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 42.48828125,
            "unit": "median mem",
            "extra": "avg mem: 41.579103194378135, max mem: 44.08203125, count: 8378"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 0,
            "unit": "median cpu",
            "extra": "avg cpu: 0.0, max cpu: 0.0, count: 8378"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 43.4609375,
            "unit": "median mem",
            "extra": "avg mem: 41.67340150543089, max mem: 45.6328125, count: 8378"
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
          "id": "80daf35915e5edde9b7e091036a88ce3d6c6aea1",
          "message": "chore: Upgrade to `0.21.8` (#4168)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2026-02-12T09:45:33-05:00",
          "tree_id": "3de2b0c6e4f9a21b26370ecab28ddd8db57c65ff",
          "url": "https://github.com/paradedb/paradedb/commit/80daf35915e5edde9b7e091036a88ce3d6c6aea1"
        },
        "date": 1770908843277,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - cpu",
            "value": 9.221902,
            "unit": "median cpu",
            "extra": "avg cpu: 8.30611030105937, max cpu: 23.121387, count: 55136"
          },
          {
            "name": "Aggregate Custom Scan - Primary - mem",
            "value": 62.40234375,
            "unit": "median mem",
            "extra": "avg mem: 62.4117450769461, max mem: 73.5, count: 55136"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6153846,
            "unit": "median cpu",
            "extra": "avg cpu: 4.579632875976373, max cpu: 9.266409, count: 55136"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 36.015625,
            "unit": "median mem",
            "extra": "avg mem: 35.74462139641069, max mem: 37.453125, count: 55136"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6153846,
            "unit": "median cpu",
            "extra": "avg cpu: 4.611843943687672, max cpu: 9.257474, count: 55136"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 60.828125,
            "unit": "median mem",
            "extra": "avg mem: 60.432603655619744, max mem: 71.8203125, count: 55136"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.702233697224778, max cpu: 9.467456, count: 110272"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 48.8515625,
            "unit": "median mem",
            "extra": "avg mem: 48.79865925988692, max mem: 59.75390625, count: 110272"
          },
          {
            "name": "Mixed Fast Field Scan - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 5.392766517183136, max cpu: 18.897638, count: 55136"
          },
          {
            "name": "Mixed Fast Field Scan - Primary - mem",
            "value": 61.625,
            "unit": "median mem",
            "extra": "avg mem: 61.6368223291044, max mem: 72.65625, count: 55136"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 1700,
            "unit": "median block_count",
            "extra": "avg block_count: 1721.0118615786419, max block_count: 3045.0, count: 55136"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 10,
            "unit": "median segment_count",
            "extra": "avg segment_count: 11.939912217063261, max segment_count: 29.0, count: 55136"
          },
          {
            "name": "Normal Scan - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 5.309164272272262, max cpu: 18.497108, count: 55136"
          },
          {
            "name": "Normal Scan - Primary - mem",
            "value": 61.55078125,
            "unit": "median mem",
            "extra": "avg mem: 61.536106528062426, max mem: 72.5078125, count: 55136"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.566474788358497, max cpu: 4.776119, count: 55136"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 51.48828125,
            "unit": "median mem",
            "extra": "avg mem: 51.49385298398506, max mem: 62.15234375, count: 55136"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6065254,
            "unit": "median cpu",
            "extra": "avg cpu: 4.291767177273862, max cpu: 4.64666, count: 55136"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 53.95703125,
            "unit": "median mem",
            "extra": "avg mem: 52.87793882683274, max mem: 66.015625, count: 55136"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "2aec8447addadd4def1cf10f4d11e24c1755fadb",
          "message": "chore: Remove tuned_postgres from /benchmarks (#4167)\n\n# Ticket(s) Closed\n\n- Closes #N/A\n\n## What\nWe are moving user-facing benchmarks to a much more complete project,\nwhich will be in a separate repository. This is no longer relevant to\nkeep here, so removing.\n\n## Why\n^\n\n## How\n^\n\n## Tests\n^",
          "timestamp": "2026-02-12T12:21:38-05:00",
          "tree_id": "3502c5fdf7ad1b45110e95b70c181dd5ca1eae37",
          "url": "https://github.com/paradedb/paradedb/commit/2aec8447addadd4def1cf10f4d11e24c1755fadb"
        },
        "date": 1770918059051,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - cpu",
            "value": 9.266409,
            "unit": "median cpu",
            "extra": "avg cpu: 8.401486319796641, max cpu: 23.166023, count: 55205"
          },
          {
            "name": "Aggregate Custom Scan - Primary - mem",
            "value": 62.875,
            "unit": "median mem",
            "extra": "avg mem: 62.56013643748302, max mem: 73.6484375, count: 55205"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.647892916416334, max cpu: 9.29332, count: 55205"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 35.34375,
            "unit": "median mem",
            "extra": "avg mem: 35.28663044278145, max mem: 36.8515625, count: 55205"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.890670289783641, max cpu: 9.4395275, count: 55205"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 60.9609375,
            "unit": "median mem",
            "extra": "avg mem: 60.14952506566434, max mem: 71.70703125, count: 55205"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.684811452581543, max cpu: 9.284333, count: 110410"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 49.31640625,
            "unit": "median mem",
            "extra": "avg mem: 49.005098326691424, max mem: 59.91796875, count: 110410"
          },
          {
            "name": "Mixed Fast Field Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 5.324601426864073, max cpu: 18.897638, count: 55205"
          },
          {
            "name": "Mixed Fast Field Scan - Primary - mem",
            "value": 62.2890625,
            "unit": "median mem",
            "extra": "avg mem: 61.948176116293816, max mem: 73.01953125, count: 55205"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 1732,
            "unit": "median block_count",
            "extra": "avg block_count: 1729.163517797301, max block_count: 3082.0, count: 55205"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 15,
            "unit": "median segment_count",
            "extra": "avg segment_count: 16.848165926999368, max segment_count: 30.0, count: 55205"
          },
          {
            "name": "Normal Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 5.43113373178758, max cpu: 18.916256, count: 55205"
          },
          {
            "name": "Normal Scan - Primary - mem",
            "value": 61.71484375,
            "unit": "median mem",
            "extra": "avg mem: 61.401476669346074, max mem: 72.484375, count: 55205"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.610478366150767, max cpu: 4.7952046, count: 55205"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 51.87890625,
            "unit": "median mem",
            "extra": "avg mem: 51.62946942079522, max mem: 62.46484375, count: 55205"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.477716174934031, max cpu: 4.7244096, count: 55205"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 54.78125,
            "unit": "median mem",
            "extra": "avg mem: 54.04537073464813, max mem: 66.5078125, count: 55205"
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
          "id": "ba868f34636e9fc6068c68b3b0d8a098eb4971d8",
          "message": "feat: join-scan: pre-materialization dynamic filter pushdown from TopK and HashJoin (#4161)\n\n## Ticket(s) Closed\n\n- Closes #4151\n\n## What\n\nDynamic filters from DataFusion's `SortExec(TopK)` and `HashJoinExec`\nare now pushed down into `PgSearchScan` and applied *before* column\nmaterialization — at the term-ordinal level for strings and the\nfast-field level for numerics. This avoids expensive term dictionary I/O\nfor documents that will be discarded anyway.\n\n## Why\n\nPreviously, `PgSearchScan` had no awareness of dynamic filters. Every\ndocument that passed the Tantivy query and visibility checks was fully\nmaterialized (all fast-field columns loaded, string dictionaries walked)\nbefore any join-key or TopK pruning could happen upstream. For selective\njoins or tight LIMIT queries, this meant loading data for rows that were\nimmediately thrown away by HashJoin or TopK.\n\n## How\n\n- Enabled DataFusion's TopK dynamic filter pushdown in the JoinScan\nsession config.\n- `SegmentPlan` now accepts dynamic filters from parent operators (TopK\nthresholds, HashJoin key bounds) and passes them to the Scanner on each\nbatch.\n- Before column materialization, the Scanner converts these filters to\nterm-ordinal comparisons (for strings) or direct fast-field comparisons\n(for numerics) and prunes non-matching documents in-place — skipping\ndictionary I/O entirely for pruned rows.\n\n## Tests\n\n- New `topk_dynamic_filter` regression test covering. You can take a\nlook at EXPLAIN ANALYZE diff in the follow-up PR (#4162):\nhttps://github.com/paradedb/paradedb/blob/3b074a9b5516a7a0a75a948201ef32e07b0127e4/pg_search/tests/pg_regress/expected/topk_dynamic_filter.out#L170-L181\n- All existing regression tests pass.",
          "timestamp": "2026-02-12T10:25:25-08:00",
          "tree_id": "748bfdacf0d0b82f9ceb26840b3100a7ca8e2252",
          "url": "https://github.com/paradedb/paradedb/commit/ba868f34636e9fc6068c68b3b0d8a098eb4971d8"
        },
        "date": 1770921884286,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - cpu",
            "value": 9.230769,
            "unit": "median cpu",
            "extra": "avg cpu: 8.28428214715497, max cpu: 23.529411, count: 55154"
          },
          {
            "name": "Aggregate Custom Scan - Primary - mem",
            "value": 62.87109375,
            "unit": "median mem",
            "extra": "avg mem: 62.79307165788066, max mem: 73.59375, count: 55154"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.590266037605356, max cpu: 9.4395275, count: 55154"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 35.48828125,
            "unit": "median mem",
            "extra": "avg mem: 35.341882997493386, max mem: 37.41015625, count: 55154"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.639638730586811, max cpu: 9.275363, count: 55154"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 60.85546875,
            "unit": "median mem",
            "extra": "avg mem: 60.467619217327844, max mem: 71.66796875, count: 55154"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.609014125709052, max cpu: 9.375, count: 110308"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 49.0390625,
            "unit": "median mem",
            "extra": "avg mem: 48.7748228964581, max mem: 59.4375, count: 110308"
          },
          {
            "name": "Mixed Fast Field Scan - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 5.446595887328231, max cpu: 19.335348, count: 55154"
          },
          {
            "name": "Mixed Fast Field Scan - Primary - mem",
            "value": 61.94140625,
            "unit": "median mem",
            "extra": "avg mem: 61.83980245936378, max mem: 72.6875, count: 55154"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 1755,
            "unit": "median block_count",
            "extra": "avg block_count: 1753.565797584944, max block_count: 3066.0, count: 55154"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 14,
            "unit": "median segment_count",
            "extra": "avg segment_count: 15.145284113572904, max segment_count: 30.0, count: 55154"
          },
          {
            "name": "Normal Scan - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 5.397498082791857, max cpu: 18.58664, count: 55154"
          },
          {
            "name": "Normal Scan - Primary - mem",
            "value": 61.92578125,
            "unit": "median mem",
            "extra": "avg mem: 61.7911186745295, max mem: 72.6328125, count: 55154"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.680951477590147, max cpu: 9.266409, count: 55154"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 51.8203125,
            "unit": "median mem",
            "extra": "avg mem: 51.43702172269917, max mem: 61.98828125, count: 55154"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6153846,
            "unit": "median cpu",
            "extra": "avg cpu: 3.5326106947115195, max cpu: 4.729064, count: 55154"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 53.953125,
            "unit": "median mem",
            "extra": "avg mem: 51.94206449214926, max mem: 65.515625, count: 55154"
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
          "id": "655ee8b04cab31c056cb41a89d677b896630ed16",
          "message": "feat: join-scan: surface dynamic filter metrics in EXPLAIN ANALYZE (#4162)\n\n# Ticket(s) Closed\n\n- Closes #4151\n\n## What\n\nUsed DataFusion metrics, and made dynamic filter pruning stats visible\nthrough `EXPLAIN ANALYZE`.\n\n## Why\n\n`EXPLAIN ANALYZE` is the natural place for execution-time stats.\n\n## How\n\n- Added `ExecutionPlanMetricsSet` to `SegmentPlan` with two custom\ncounters (`rows_scanned`, `rows_pruned`), only registered when dynamic\nfilters are present.\n\n## Tests\n\n- Updated `topk_dynamic_filter` regression test to use `EXPLAIN\n(ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)` — verifying\n`Dynamic Filter` lines appear with correct pruning stats (e.g., `30\nscanned, 24 pruned (80.0%)`).\n- Updated `join_custom_scan` and `filter_pushdown_datafusion` expected\noutput.\n\n---------\n\nSigned-off-by: Moe <mdashti@gmail.com>",
          "timestamp": "2026-02-12T12:45:44-08:00",
          "tree_id": "8c73104c0b40b30047e010ebfba45fb9add3f7e8",
          "url": "https://github.com/paradedb/paradedb/commit/655ee8b04cab31c056cb41a89d677b896630ed16"
        },
        "date": 1770930323479,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - cpu",
            "value": 9.239654,
            "unit": "median cpu",
            "extra": "avg cpu: 8.21521263379245, max cpu: 24.439917, count: 55049"
          },
          {
            "name": "Aggregate Custom Scan - Primary - mem",
            "value": 62.703125,
            "unit": "median mem",
            "extra": "avg mem: 62.53373556967429, max mem: 74.0390625, count: 55049"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.608623959616678, max cpu: 9.302325, count: 55049"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 35.62109375,
            "unit": "median mem",
            "extra": "avg mem: 35.33349855072299, max mem: 37.30078125, count: 55049"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.673847315387429, max cpu: 9.29332, count: 55049"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 60.89453125,
            "unit": "median mem",
            "extra": "avg mem: 60.4067821253565, max mem: 72.234375, count: 55049"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.635732689801101, max cpu: 9.67742, count: 110098"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 48.9140625,
            "unit": "median mem",
            "extra": "avg mem: 48.75996498573998, max mem: 60.00390625, count: 110098"
          },
          {
            "name": "Mixed Fast Field Scan - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 5.303350515555923, max cpu: 19.551935, count: 55049"
          },
          {
            "name": "Mixed Fast Field Scan - Primary - mem",
            "value": 61.890625,
            "unit": "median mem",
            "extra": "avg mem: 61.75799869775109, max mem: 73.1640625, count: 55049"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 1734,
            "unit": "median block_count",
            "extra": "avg block_count: 1741.582172246544, max block_count: 3125.0, count: 55049"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 10,
            "unit": "median segment_count",
            "extra": "avg segment_count: 10.707042816399934, max segment_count: 30.0, count: 55049"
          },
          {
            "name": "Normal Scan - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 5.273339356071607, max cpu: 14.51613, count: 55049"
          },
          {
            "name": "Normal Scan - Primary - mem",
            "value": 61.81640625,
            "unit": "median mem",
            "extra": "avg mem: 61.711756372731564, max mem: 73.078125, count: 55049"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.567020443913195, max cpu: 4.83871, count: 55049"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 51.87109375,
            "unit": "median mem",
            "extra": "avg mem: 51.605225075046775, max mem: 62.59765625, count: 55049"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.079759486710634, max cpu: 4.64666, count: 55049"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 53.31640625,
            "unit": "median mem",
            "extra": "avg mem: 53.138149790414, max mem: 66.08203125, count: 55049"
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
          "id": "1da48a637b80cc0ce2312ab8c4e448762e152223",
          "message": "perf: Add statistics to joinscan (#4132)\n\n## Ticket(s) Closed\n\n- Closes #4062.\n\n## What\n\n* Exposes sorting from the joinscan's `TableProvider`, but does not yet\nforce `SortMergeJoin`.\n* Adds statistics on `TableProvider` and our `ExecutionPlan`s using\nTantivy's query estimates.\n* Removes the `ParallelSegmentPlan` that was added in #4101, as it makes\nmore sense to let DataFusion coalesce for us if needed.\n\n## Why\n\nTo allow the DataFusion optimizer to re-order joins based on table\nsizes, and use sortedness in plans (although it does not yet by\ndefault).\n\n## Tests\n\nExisting tests show the impact of join reordering due to statistics.",
          "timestamp": "2026-02-12T14:34:01-08:00",
          "tree_id": "fbc185b154055782f4973f483feb5ad00a4ca2bb",
          "url": "https://github.com/paradedb/paradedb/commit/1da48a637b80cc0ce2312ab8c4e448762e152223"
        },
        "date": 1770936801290,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - cpu",
            "value": 9.230769,
            "unit": "median cpu",
            "extra": "avg cpu: 8.463952766197655, max cpu: 24.169184, count: 55002"
          },
          {
            "name": "Aggregate Custom Scan - Primary - mem",
            "value": 62.98046875,
            "unit": "median mem",
            "extra": "avg mem: 62.85885207867441, max mem: 74.10546875, count: 55002"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6153846,
            "unit": "median cpu",
            "extra": "avg cpu: 4.598767243184674, max cpu: 9.275363, count: 55002"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 35.66796875,
            "unit": "median mem",
            "extra": "avg mem: 35.03147442365732, max mem: 36.109375, count: 55002"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.619827,
            "unit": "median cpu",
            "extra": "avg cpu: 4.612408469951981, max cpu: 4.701273, count: 55002"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 60.9921875,
            "unit": "median mem",
            "extra": "avg mem: 60.6604266947111, max mem: 72.30859375, count: 55002"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6153846,
            "unit": "median cpu",
            "extra": "avg cpu: 4.598337737742236, max cpu: 9.257474, count: 110004"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 49.17578125,
            "unit": "median mem",
            "extra": "avg mem: 49.05408141408494, max mem: 60.06640625, count: 110004"
          },
          {
            "name": "Mixed Fast Field Scan - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 5.4317554528820295, max cpu: 24.169184, count: 55002"
          },
          {
            "name": "Mixed Fast Field Scan - Primary - mem",
            "value": 62.03515625,
            "unit": "median mem",
            "extra": "avg mem: 61.991123192111196, max mem: 73.2265625, count: 55002"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 1745,
            "unit": "median block_count",
            "extra": "avg block_count: 1765.4072033744228, max block_count: 3121.0, count: 55002"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 13,
            "unit": "median segment_count",
            "extra": "avg segment_count: 12.367459365113996, max segment_count: 28.0, count: 55002"
          },
          {
            "name": "Normal Scan - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 5.229733807254391, max cpu: 19.335348, count: 55002"
          },
          {
            "name": "Normal Scan - Primary - mem",
            "value": 61.88671875,
            "unit": "median mem",
            "extra": "avg mem: 61.824349071965564, max mem: 73.0625, count: 55002"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.619827,
            "unit": "median cpu",
            "extra": "avg cpu: 4.535080400354167, max cpu: 4.7151275, count: 55002"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 52.09375,
            "unit": "median mem",
            "extra": "avg mem: 52.002469228391696, max mem: 62.8359375, count: 55002"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.610951,
            "unit": "median cpu",
            "extra": "avg cpu: 4.203534901868651, max cpu: 4.6332045, count: 55002"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 54.203125,
            "unit": "median mem",
            "extra": "avg mem: 53.93595403348969, max mem: 66.33984375, count: 55002"
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
          "id": "b04eae77b43a69abf89d75cd16038e7fcdd72770",
          "message": "refactor: unify static OPERATOR_LOOKUP into shared function (#4060) (#4173)\n\n## Summary\n\n- Deduplicate the `OnceLock<HashMap<PostgresOperatorOid,\nTantivyOperator>>` pattern that was copy-pasted across `pushdown.rs`,\n`planning.rs`, and `translator.rs`\n- Centralize into a single `pub(crate) fn lookup_operator(opno)` in\n`opexpr.rs`\n- Privatize `OperatorAccepts` and `initialize_equality_operator_lookup`\nsince they are no longer needed outside `opexpr.rs`\n\nCloses #4060\n\n## Test plan\n\nNo new tests added. This is a pure refactoring with no behavioral\nchange. Existing integration and regression tests provide full coverage\nof all modified call sites.",
          "timestamp": "2026-02-12T15:09:31-08:00",
          "tree_id": "4395a9fad346a49f0bcb0a75092401e526f3830c",
          "url": "https://github.com/paradedb/paradedb/commit/b04eae77b43a69abf89d75cd16038e7fcdd72770"
        },
        "date": 1770938924914,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - cpu",
            "value": 9.248554,
            "unit": "median cpu",
            "extra": "avg cpu: 8.120644907311245, max cpu: 29.357798, count: 55147"
          },
          {
            "name": "Aggregate Custom Scan - Primary - mem",
            "value": 63.5390625,
            "unit": "median mem",
            "extra": "avg mem: 63.29998727831976, max mem: 74.7734375, count: 55147"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.619364550688875, max cpu: 9.29332, count: 55147"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 36.40625,
            "unit": "median mem",
            "extra": "avg mem: 35.8688529351098, max mem: 37.83203125, count: 55147"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.629923173253462, max cpu: 9.248554, count: 55147"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 61.64453125,
            "unit": "median mem",
            "extra": "avg mem: 61.079708480855714, max mem: 72.953125, count: 55147"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.683947637488085, max cpu: 9.628887, count: 110294"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 49.73046875,
            "unit": "median mem",
            "extra": "avg mem: 49.51079207192821, max mem: 60.7578125, count: 110294"
          },
          {
            "name": "Mixed Fast Field Scan - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 5.392260024081133, max cpu: 19.393938, count: 55147"
          },
          {
            "name": "Mixed Fast Field Scan - Primary - mem",
            "value": 62.75,
            "unit": "median mem",
            "extra": "avg mem: 62.47737779253631, max mem: 73.93359375, count: 55147"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 1815,
            "unit": "median block_count",
            "extra": "avg block_count: 1816.3493027725897, max block_count: 3216.0, count: 55147"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 14,
            "unit": "median segment_count",
            "extra": "avg segment_count: 14.228734110649718, max segment_count: 31.0, count: 55147"
          },
          {
            "name": "Normal Scan - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 5.3631501088200775, max cpu: 18.934912, count: 55147"
          },
          {
            "name": "Normal Scan - Primary - mem",
            "value": 62.71484375,
            "unit": "median mem",
            "extra": "avg mem: 62.41769674970987, max mem: 73.87109375, count: 55147"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.611983332332859, max cpu: 9.257474, count: 55147"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 52.5625,
            "unit": "median mem",
            "extra": "avg mem: 52.45661326205415, max mem: 63.51953125, count: 55147"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 3.687555585449164, max cpu: 4.7058825, count: 55147"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 55.296875,
            "unit": "median mem",
            "extra": "avg mem: 54.559063304667525, max mem: 68.03515625, count: 55147"
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
          "id": "30c4c9cbf69783f13dbfa4ed26b331f441be585c",
          "message": "perf: Use parallel workers for the join scan (#4101)\n\n## Ticket(s) Closed\n\n- Closes #4063\n\n## What\n\nAdds support for parallel workers in the `joinscan`, by relying (for\nnow) on the fact that we only support INNER joins, and can thus do a\nbroadcast join.\n\n## Why\n\nTo get an implementation of parallel workers in place without (yet)\ntackling the problem of partitioning DataFusion plans across parallel\nworkers and introducing RPC.\n\n## How\n\n- Implemented a \"broadcast join\" strategy for `JoinScan` where the\nlargest index scan is partitioned across workers while the others are\nreplicated.\n- Introduced `ParallelSegmentPlan` and `ParallelScanStream` for dynamic\nworker-driven scanning.\n- This strategy is necessary in order to continue to use the lazy work\nclaiming strategy that we use in `ParallelScanState`, but after #4062\nthe replicated/un-partitioned indexes could begin using\n`MultiSegmentPlan` to provide sorted access.\n- In future, if/when we change our parallel worker partitioning\nstrategy, we might be able to use `MultiSegmentPlan` and assign _ranges_\nof an index to the parallel workers. TBD.\n- Centralized `RowEstimate` handling to better manage unanalyzed tables,\nand ease determining the largest index to scan.\n- Cleaned up registration of the `CustomScan`'s vtable\n(`CustomExecMethods`).\n- Before this, encountered some segfaults due to registration issues\naround having multiple parallel `CustomScan` implementations.\n- Remove \"lazy checkout\" from `MultiSegmentPlan`, as no consumer will\nactually use it lazily.\n\n## Tests\n\nExisting tests (and proptests) pass.\n\nBenchmarks show speedups across a few of our joins. Notably: we are\nfaster than Postgres for the `semi_join_filter` join for the first time.",
          "timestamp": "2026-02-09T17:53:18-08:00",
          "tree_id": "6616d18d10f8cf9e48caa5c264c26297828fd02b",
          "url": "https://github.com/paradedb/paradedb/commit/30c4c9cbf69783f13dbfa4ed26b331f441be585c"
        },
        "date": 1770690561391,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 7.400730737373,
            "unit": "median tps",
            "extra": "avg tps: 6.338316671424383, max tps: 9.488274706936327, count: 57290"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.54940259819862,
            "unit": "median tps",
            "extra": "avg tps: 4.967600101198495, max tps: 6.2603639715073705, count: 57290"
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
          "id": "ff10528e88ec12dd794b2a6e8b75996ad447a713",
          "message": "fix: Joinscan row estimation needs `ExprContext` (#4147)\n\n# Ticket(s) Closed\n\n- Closes #4146 \n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2026-02-10T14:16:35-05:00",
          "tree_id": "bba16dfdbd260ac92fb27be55e3b411b77039476",
          "url": "https://github.com/paradedb/paradedb/commit/ff10528e88ec12dd794b2a6e8b75996ad447a713"
        },
        "date": 1770753084105,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 7.618990872966136,
            "unit": "median tps",
            "extra": "avg tps: 6.492983603921218, max tps: 9.742258240326677, count: 57786"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.30596871304801,
            "unit": "median tps",
            "extra": "avg tps: 4.7420553388129445, max tps: 5.997889859587869, count: 57786"
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
          "id": "084451f652ebc5c322fbf12b0189bc5e229dce3a",
          "message": "fix: reduce overhead for ngram match queries and add TEXT[] regression coverage (#4150)\n\n# Ticket(s) Closed\n\n- Closes #2884\n\n## What\n\nMinor optimization to `match_query` and new regression test covering\nngram search on TEXT[] columns with `conjunction_mode`.\n\n## Why\n\nA it's reported in #2884, slow ngram searches (~16 queries/s vs ~70\nwithout ngram) on a 350k-row TEXT[] column. We investigated and found\nthe N-way posting list intersection in `BooleanQuery` with many Must\nclauses is inherently expensive and can't be fundamentally improved at\nthe pg_search level. However, we identified two sources of unnecessary\noverhead in how `match_query` constructs the query.\n\n## How\n\n1. **`IndexRecordOption::WithFreqs` instead of `WithFreqsAndPositions`**\n— `match_query` creates `TermQuery` instances inside a `BooleanQuery`.\nThe BooleanQuery scorer only uses doc iteration and BM25 scores, never\npositions. `WithFreqsAndPositions` was requesting position data that was\nnever read. `WithFreqs` produces identical BM25 scores with less\nper-document overhead.\n\n2. **Deduplicate terms for conjunction mode** — For queries with\nrepeated ngram tokens (e.g., strings with repeated substrings),\nduplicate Must clauses add intersection work without changing which\ndocuments match. Dedup removes them before building the query.\n\nBoth changes preserve identical matching semantics and BM25 scoring.\n\n## Tests\n\nNew `ngram-text-array` regression test covering the exact pattern from\nthe reported issue: TEXT[] column with ICU + ngram alias fields, `match`\nwith `conjunction_mode`, `disjunction_max`, edge cases (short queries,\nsingle-token queries), and the JSON `::jsonb` query path.",
          "timestamp": "2026-02-10T15:11:24-08:00",
          "tree_id": "ce5fefd07b9871c52c5cd32b82b7f79613310334",
          "url": "https://github.com/paradedb/paradedb/commit/084451f652ebc5c322fbf12b0189bc5e229dce3a"
        },
        "date": 1770767186911,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 7.440028161557425,
            "unit": "median tps",
            "extra": "avg tps: 6.392933817365211, max tps: 9.718688492433413, count: 57763"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.420415459232548,
            "unit": "median tps",
            "extra": "avg tps: 4.859189884214031, max tps: 6.065683963042209, count: 57763"
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
          "id": "59354e0de782d993f3e4a260eb7c56ad4804a1ad",
          "message": "fix: add field validation for `paradedb.aggregate()` API (#4141)\n\n# Ticket(s) Closed\n\n- Closes #N/A\n\n## What\n\nAdds field validation to the `paradedb.aggregate()` direct SQL function\nso that referencing a nonexistent or unsupported field returns a clear\nerror instead of silently producing null results.\n\n## Why\n\nThe `pdb.agg()` aggregate path already validates fields at plan time via\n`AggregateType::validate_fields()`. However, the `paradedb.aggregate()`\nfunction is a plain `pg_extern` that calls `execute_aggregate()`\ndirectly — it never enters the custom scan planner, so it skipped\nvalidation entirely. An invalid field like `\"nonexistent_field\"` would\nquietly return `{\"value\": null}` instead of telling the user something\nis wrong.\n\n## How\n\n- Extracted the field validation logic from\n`AggregateType::validate_fields()` into a standalone\n`validate_agg_json_fields()` function in `aggregate_type.rs`. The\nexisting `validate_fields()` now delegates to it for custom aggregates.\n- Called `validate_agg_json_fields()` in `aggregate_impl()`\n(`api/aggregate.rs`) before executing, so the direct API gets the same\nvalidation as the planner path.\n\n## Tests\n\n- Added regression tests (tests 13–15 in `agg-validate.sql`) covering\nthe `paradedb.aggregate()` path: valid field succeeds, invalid field\nerrors, invalid nested field errors.",
          "timestamp": "2026-02-10T15:12:54-08:00",
          "tree_id": "a2a30dc05294896dfaef747d15452a4024f5d8aa",
          "url": "https://github.com/paradedb/paradedb/commit/59354e0de782d993f3e4a260eb7c56ad4804a1ad"
        },
        "date": 1770767280541,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 7.561244615250255,
            "unit": "median tps",
            "extra": "avg tps: 6.46598828845343, max tps: 9.798809739933926, count: 57786"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.418886361932809,
            "unit": "median tps",
            "extra": "avg tps: 4.862213777012628, max tps: 6.0682034087617325, count: 57786"
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
          "id": "44d1f928882599cf5fd9fbc853c8eee1fb5c57ed",
          "message": "fix: Rebase against Tantivy, inherit upstream bugfix for intersection queries (#4155)\n\n# Ticket(s) Closed\n\n- Closes #4149 \n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2026-02-11T11:47:17-05:00",
          "tree_id": "b88ad04015b13dee26d44c4c9d585ea252d0323c",
          "url": "https://github.com/paradedb/paradedb/commit/44d1f928882599cf5fd9fbc853c8eee1fb5c57ed"
        },
        "date": 1770830233059,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 7.76401206267423,
            "unit": "median tps",
            "extra": "avg tps: 6.646171767550437, max tps: 10.025221353608849, count: 57588"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.23223560772492,
            "unit": "median tps",
            "extra": "avg tps: 4.673588714027965, max tps: 5.884365888882815, count: 57588"
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
          "id": "80daf35915e5edde9b7e091036a88ce3d6c6aea1",
          "message": "chore: Upgrade to `0.21.8` (#4168)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2026-02-12T09:45:33-05:00",
          "tree_id": "3de2b0c6e4f9a21b26370ecab28ddd8db57c65ff",
          "url": "https://github.com/paradedb/paradedb/commit/80daf35915e5edde9b7e091036a88ce3d6c6aea1"
        },
        "date": 1770909797038,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 7.496016463918951,
            "unit": "median tps",
            "extra": "avg tps: 6.422217088973288, max tps: 9.688881008138374, count: 57596"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.456162929051679,
            "unit": "median tps",
            "extra": "avg tps: 4.870981517635184, max tps: 6.1230570200639605, count: 57596"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "2aec8447addadd4def1cf10f4d11e24c1755fadb",
          "message": "chore: Remove tuned_postgres from /benchmarks (#4167)\n\n# Ticket(s) Closed\n\n- Closes #N/A\n\n## What\nWe are moving user-facing benchmarks to a much more complete project,\nwhich will be in a separate repository. This is no longer relevant to\nkeep here, so removing.\n\n## Why\n^\n\n## How\n^\n\n## Tests\n^",
          "timestamp": "2026-02-12T12:21:38-05:00",
          "tree_id": "3502c5fdf7ad1b45110e95b70c181dd5ca1eae37",
          "url": "https://github.com/paradedb/paradedb/commit/2aec8447addadd4def1cf10f4d11e24c1755fadb"
        },
        "date": 1770918985701,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 7.468412823730576,
            "unit": "median tps",
            "extra": "avg tps: 6.388320116432732, max tps: 9.672816501733603, count: 57864"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.436835802304092,
            "unit": "median tps",
            "extra": "avg tps: 4.853615018333301, max tps: 6.084172654140523, count: 57864"
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
          "id": "ba868f34636e9fc6068c68b3b0d8a098eb4971d8",
          "message": "feat: join-scan: pre-materialization dynamic filter pushdown from TopK and HashJoin (#4161)\n\n## Ticket(s) Closed\n\n- Closes #4151\n\n## What\n\nDynamic filters from DataFusion's `SortExec(TopK)` and `HashJoinExec`\nare now pushed down into `PgSearchScan` and applied *before* column\nmaterialization — at the term-ordinal level for strings and the\nfast-field level for numerics. This avoids expensive term dictionary I/O\nfor documents that will be discarded anyway.\n\n## Why\n\nPreviously, `PgSearchScan` had no awareness of dynamic filters. Every\ndocument that passed the Tantivy query and visibility checks was fully\nmaterialized (all fast-field columns loaded, string dictionaries walked)\nbefore any join-key or TopK pruning could happen upstream. For selective\njoins or tight LIMIT queries, this meant loading data for rows that were\nimmediately thrown away by HashJoin or TopK.\n\n## How\n\n- Enabled DataFusion's TopK dynamic filter pushdown in the JoinScan\nsession config.\n- `SegmentPlan` now accepts dynamic filters from parent operators (TopK\nthresholds, HashJoin key bounds) and passes them to the Scanner on each\nbatch.\n- Before column materialization, the Scanner converts these filters to\nterm-ordinal comparisons (for strings) or direct fast-field comparisons\n(for numerics) and prunes non-matching documents in-place — skipping\ndictionary I/O entirely for pruned rows.\n\n## Tests\n\n- New `topk_dynamic_filter` regression test covering. You can take a\nlook at EXPLAIN ANALYZE diff in the follow-up PR (#4162):\nhttps://github.com/paradedb/paradedb/blob/3b074a9b5516a7a0a75a948201ef32e07b0127e4/pg_search/tests/pg_regress/expected/topk_dynamic_filter.out#L170-L181\n- All existing regression tests pass.",
          "timestamp": "2026-02-12T10:25:25-08:00",
          "tree_id": "748bfdacf0d0b82f9ceb26840b3100a7ca8e2252",
          "url": "https://github.com/paradedb/paradedb/commit/ba868f34636e9fc6068c68b3b0d8a098eb4971d8"
        },
        "date": 1770922816871,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 7.500777183545529,
            "unit": "median tps",
            "extra": "avg tps: 6.3926203652358335, max tps: 9.790771322725242, count: 57760"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.3562272021893484,
            "unit": "median tps",
            "extra": "avg tps: 4.794832583454915, max tps: 5.9922809285700875, count: 57760"
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
          "id": "655ee8b04cab31c056cb41a89d677b896630ed16",
          "message": "feat: join-scan: surface dynamic filter metrics in EXPLAIN ANALYZE (#4162)\n\n# Ticket(s) Closed\n\n- Closes #4151\n\n## What\n\nUsed DataFusion metrics, and made dynamic filter pruning stats visible\nthrough `EXPLAIN ANALYZE`.\n\n## Why\n\n`EXPLAIN ANALYZE` is the natural place for execution-time stats.\n\n## How\n\n- Added `ExecutionPlanMetricsSet` to `SegmentPlan` with two custom\ncounters (`rows_scanned`, `rows_pruned`), only registered when dynamic\nfilters are present.\n\n## Tests\n\n- Updated `topk_dynamic_filter` regression test to use `EXPLAIN\n(ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)` — verifying\n`Dynamic Filter` lines appear with correct pruning stats (e.g., `30\nscanned, 24 pruned (80.0%)`).\n- Updated `join_custom_scan` and `filter_pushdown_datafusion` expected\noutput.\n\n---------\n\nSigned-off-by: Moe <mdashti@gmail.com>",
          "timestamp": "2026-02-12T12:45:44-08:00",
          "tree_id": "8c73104c0b40b30047e010ebfba45fb9add3f7e8",
          "url": "https://github.com/paradedb/paradedb/commit/655ee8b04cab31c056cb41a89d677b896630ed16"
        },
        "date": 1770931281583,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 7.526545290363941,
            "unit": "median tps",
            "extra": "avg tps: 6.4544481428952105, max tps: 9.740084899846057, count: 57893"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.411635750413463,
            "unit": "median tps",
            "extra": "avg tps: 4.833142833483922, max tps: 6.06312556698496, count: 57893"
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
          "id": "1da48a637b80cc0ce2312ab8c4e448762e152223",
          "message": "perf: Add statistics to joinscan (#4132)\n\n## Ticket(s) Closed\n\n- Closes #4062.\n\n## What\n\n* Exposes sorting from the joinscan's `TableProvider`, but does not yet\nforce `SortMergeJoin`.\n* Adds statistics on `TableProvider` and our `ExecutionPlan`s using\nTantivy's query estimates.\n* Removes the `ParallelSegmentPlan` that was added in #4101, as it makes\nmore sense to let DataFusion coalesce for us if needed.\n\n## Why\n\nTo allow the DataFusion optimizer to re-order joins based on table\nsizes, and use sortedness in plans (although it does not yet by\ndefault).\n\n## Tests\n\nExisting tests show the impact of join reordering due to statistics.",
          "timestamp": "2026-02-12T14:34:01-08:00",
          "tree_id": "fbc185b154055782f4973f483feb5ad00a4ca2bb",
          "url": "https://github.com/paradedb/paradedb/commit/1da48a637b80cc0ce2312ab8c4e448762e152223"
        },
        "date": 1770937718544,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 7.7082485288053535,
            "unit": "median tps",
            "extra": "avg tps: 6.590760841304289, max tps: 9.966562038747714, count: 57829"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.397396109338542,
            "unit": "median tps",
            "extra": "avg tps: 4.834106497005306, max tps: 6.059196250691988, count: 57829"
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
          "id": "b04eae77b43a69abf89d75cd16038e7fcdd72770",
          "message": "refactor: unify static OPERATOR_LOOKUP into shared function (#4060) (#4173)\n\n## Summary\n\n- Deduplicate the `OnceLock<HashMap<PostgresOperatorOid,\nTantivyOperator>>` pattern that was copy-pasted across `pushdown.rs`,\n`planning.rs`, and `translator.rs`\n- Centralize into a single `pub(crate) fn lookup_operator(opno)` in\n`opexpr.rs`\n- Privatize `OperatorAccepts` and `initialize_equality_operator_lookup`\nsince they are no longer needed outside `opexpr.rs`\n\nCloses #4060\n\n## Test plan\n\nNo new tests added. This is a pure refactoring with no behavioral\nchange. Existing integration and regression tests provide full coverage\nof all modified call sites.",
          "timestamp": "2026-02-12T15:09:31-08:00",
          "tree_id": "4395a9fad346a49f0bcb0a75092401e526f3830c",
          "url": "https://github.com/paradedb/paradedb/commit/b04eae77b43a69abf89d75cd16038e7fcdd72770"
        },
        "date": 1770939857569,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 7.640546508364129,
            "unit": "median tps",
            "extra": "avg tps: 6.526680354027976, max tps: 9.921462242351888, count: 57332"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.398938535640128,
            "unit": "median tps",
            "extra": "avg tps: 4.834815994531836, max tps: 6.059051657625613, count: 57332"
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
          "id": "30c4c9cbf69783f13dbfa4ed26b331f441be585c",
          "message": "perf: Use parallel workers for the join scan (#4101)\n\n## Ticket(s) Closed\n\n- Closes #4063\n\n## What\n\nAdds support for parallel workers in the `joinscan`, by relying (for\nnow) on the fact that we only support INNER joins, and can thus do a\nbroadcast join.\n\n## Why\n\nTo get an implementation of parallel workers in place without (yet)\ntackling the problem of partitioning DataFusion plans across parallel\nworkers and introducing RPC.\n\n## How\n\n- Implemented a \"broadcast join\" strategy for `JoinScan` where the\nlargest index scan is partitioned across workers while the others are\nreplicated.\n- Introduced `ParallelSegmentPlan` and `ParallelScanStream` for dynamic\nworker-driven scanning.\n- This strategy is necessary in order to continue to use the lazy work\nclaiming strategy that we use in `ParallelScanState`, but after #4062\nthe replicated/un-partitioned indexes could begin using\n`MultiSegmentPlan` to provide sorted access.\n- In future, if/when we change our parallel worker partitioning\nstrategy, we might be able to use `MultiSegmentPlan` and assign _ranges_\nof an index to the parallel workers. TBD.\n- Centralized `RowEstimate` handling to better manage unanalyzed tables,\nand ease determining the largest index to scan.\n- Cleaned up registration of the `CustomScan`'s vtable\n(`CustomExecMethods`).\n- Before this, encountered some segfaults due to registration issues\naround having multiple parallel `CustomScan` implementations.\n- Remove \"lazy checkout\" from `MultiSegmentPlan`, as no consumer will\nactually use it lazily.\n\n## Tests\n\nExisting tests (and proptests) pass.\n\nBenchmarks show speedups across a few of our joins. Notably: we are\nfaster than Postgres for the `semi_join_filter` join for the first time.",
          "timestamp": "2026-02-09T17:53:18-08:00",
          "tree_id": "6616d18d10f8cf9e48caa5c264c26297828fd02b",
          "url": "https://github.com/paradedb/paradedb/commit/30c4c9cbf69783f13dbfa4ed26b331f441be585c"
        },
        "date": 1770690566648,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.27837,
            "unit": "median cpu",
            "extra": "avg cpu: 21.527461979901606, max cpu: 42.857143, count: 57290"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 235.90234375,
            "unit": "median mem",
            "extra": "avg mem: 235.81683846711906, max mem: 237.390625, count: 57290"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.346306,
            "unit": "median cpu",
            "extra": "avg cpu: 22.303379056927895, max cpu: 33.300297, count: 57290"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 175.26171875,
            "unit": "median mem",
            "extra": "avg mem: 175.13625755476522, max mem: 175.890625, count: 57290"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 33736,
            "unit": "median block_count",
            "extra": "avg block_count: 33023.90729621225, max block_count: 35430.0, count: 57290"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 78,
            "unit": "median segment_count",
            "extra": "avg segment_count: 80.3794204922325, max segment_count: 127.0, count: 57290"
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
          "id": "ff10528e88ec12dd794b2a6e8b75996ad447a713",
          "message": "fix: Joinscan row estimation needs `ExprContext` (#4147)\n\n# Ticket(s) Closed\n\n- Closes #4146 \n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2026-02-10T14:16:35-05:00",
          "tree_id": "bba16dfdbd260ac92fb27be55e3b411b77039476",
          "url": "https://github.com/paradedb/paradedb/commit/ff10528e88ec12dd794b2a6e8b75996ad447a713"
        },
        "date": 1770753088894,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.210833,
            "unit": "median cpu",
            "extra": "avg cpu: 21.14997887702312, max cpu: 42.899704, count: 57786"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 235.890625,
            "unit": "median mem",
            "extra": "avg mem: 235.76220897470841, max mem: 237.38671875, count: 57786"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.323614,
            "unit": "median cpu",
            "extra": "avg cpu: 22.50588240970629, max cpu: 33.333336, count: 57786"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 175.53125,
            "unit": "median mem",
            "extra": "avg mem: 175.31616255721542, max mem: 176.5078125, count: 57786"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 33901,
            "unit": "median block_count",
            "extra": "avg block_count: 33381.07228394421, max block_count: 35822.0, count: 57786"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 79,
            "unit": "median segment_count",
            "extra": "avg segment_count: 81.26108399958467, max segment_count: 125.0, count: 57786"
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
          "id": "084451f652ebc5c322fbf12b0189bc5e229dce3a",
          "message": "fix: reduce overhead for ngram match queries and add TEXT[] regression coverage (#4150)\n\n# Ticket(s) Closed\n\n- Closes #2884\n\n## What\n\nMinor optimization to `match_query` and new regression test covering\nngram search on TEXT[] columns with `conjunction_mode`.\n\n## Why\n\nA it's reported in #2884, slow ngram searches (~16 queries/s vs ~70\nwithout ngram) on a 350k-row TEXT[] column. We investigated and found\nthe N-way posting list intersection in `BooleanQuery` with many Must\nclauses is inherently expensive and can't be fundamentally improved at\nthe pg_search level. However, we identified two sources of unnecessary\noverhead in how `match_query` constructs the query.\n\n## How\n\n1. **`IndexRecordOption::WithFreqs` instead of `WithFreqsAndPositions`**\n— `match_query` creates `TermQuery` instances inside a `BooleanQuery`.\nThe BooleanQuery scorer only uses doc iteration and BM25 scores, never\npositions. `WithFreqsAndPositions` was requesting position data that was\nnever read. `WithFreqs` produces identical BM25 scores with less\nper-document overhead.\n\n2. **Deduplicate terms for conjunction mode** — For queries with\nrepeated ngram tokens (e.g., strings with repeated substrings),\nduplicate Must clauses add intersection work without changing which\ndocuments match. Dedup removes them before building the query.\n\nBoth changes preserve identical matching semantics and BM25 scoring.\n\n## Tests\n\nNew `ngram-text-array` regression test covering the exact pattern from\nthe reported issue: TEXT[] column with ICU + ngram alias fields, `match`\nwith `conjunction_mode`, `disjunction_max`, edge cases (short queries,\nsingle-token queries), and the JSON `::jsonb` query path.",
          "timestamp": "2026-02-10T15:11:24-08:00",
          "tree_id": "ce5fefd07b9871c52c5cd32b82b7f79613310334",
          "url": "https://github.com/paradedb/paradedb/commit/084451f652ebc5c322fbf12b0189bc5e229dce3a"
        },
        "date": 1770767191684,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.210833,
            "unit": "median cpu",
            "extra": "avg cpu: 20.9172862404699, max cpu: 42.857143, count: 57763"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 236.41015625,
            "unit": "median mem",
            "extra": "avg mem: 236.2737237586128, max mem: 237.90625, count: 57763"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.30097,
            "unit": "median cpu",
            "extra": "avg cpu: 22.451578144558916, max cpu: 33.168808, count: 57763"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 175.71484375,
            "unit": "median mem",
            "extra": "avg mem: 175.4453720780387, max mem: 176.328125, count: 57763"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 33820,
            "unit": "median block_count",
            "extra": "avg block_count: 33337.10614060904, max block_count: 35948.0, count: 57763"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 78,
            "unit": "median segment_count",
            "extra": "avg segment_count: 80.93106313730243, max segment_count: 126.0, count: 57763"
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
          "id": "59354e0de782d993f3e4a260eb7c56ad4804a1ad",
          "message": "fix: add field validation for `paradedb.aggregate()` API (#4141)\n\n# Ticket(s) Closed\n\n- Closes #N/A\n\n## What\n\nAdds field validation to the `paradedb.aggregate()` direct SQL function\nso that referencing a nonexistent or unsupported field returns a clear\nerror instead of silently producing null results.\n\n## Why\n\nThe `pdb.agg()` aggregate path already validates fields at plan time via\n`AggregateType::validate_fields()`. However, the `paradedb.aggregate()`\nfunction is a plain `pg_extern` that calls `execute_aggregate()`\ndirectly — it never enters the custom scan planner, so it skipped\nvalidation entirely. An invalid field like `\"nonexistent_field\"` would\nquietly return `{\"value\": null}` instead of telling the user something\nis wrong.\n\n## How\n\n- Extracted the field validation logic from\n`AggregateType::validate_fields()` into a standalone\n`validate_agg_json_fields()` function in `aggregate_type.rs`. The\nexisting `validate_fields()` now delegates to it for custom aggregates.\n- Called `validate_agg_json_fields()` in `aggregate_impl()`\n(`api/aggregate.rs`) before executing, so the direct API gets the same\nvalidation as the planner path.\n\n## Tests\n\n- Added regression tests (tests 13–15 in `agg-validate.sql`) covering\nthe `paradedb.aggregate()` path: valid field succeeds, invalid field\nerrors, invalid nested field errors.",
          "timestamp": "2026-02-10T15:12:54-08:00",
          "tree_id": "a2a30dc05294896dfaef747d15452a4024f5d8aa",
          "url": "https://github.com/paradedb/paradedb/commit/59354e0de782d993f3e4a260eb7c56ad4804a1ad"
        },
        "date": 1770767285228,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.233301,
            "unit": "median cpu",
            "extra": "avg cpu: 21.189849279358153, max cpu: 43.070786, count: 57786"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 236.1640625,
            "unit": "median mem",
            "extra": "avg mem: 236.05800347943273, max mem: 237.66015625, count: 57786"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.346306,
            "unit": "median cpu",
            "extra": "avg cpu: 22.40394769745995, max cpu: 33.267326, count: 57786"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 175.2890625,
            "unit": "median mem",
            "extra": "avg mem: 175.14107520636486, max mem: 176.140625, count: 57786"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 34262,
            "unit": "median block_count",
            "extra": "avg block_count: 33434.61866195964, max block_count: 35892.0, count: 57786"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 78,
            "unit": "median segment_count",
            "extra": "avg segment_count: 81.00520887412176, max segment_count: 128.0, count: 57786"
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
          "id": "44d1f928882599cf5fd9fbc853c8eee1fb5c57ed",
          "message": "fix: Rebase against Tantivy, inherit upstream bugfix for intersection queries (#4155)\n\n# Ticket(s) Closed\n\n- Closes #4149 \n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2026-02-11T11:47:17-05:00",
          "tree_id": "b88ad04015b13dee26d44c4c9d585ea252d0323c",
          "url": "https://github.com/paradedb/paradedb/commit/44d1f928882599cf5fd9fbc853c8eee1fb5c57ed"
        },
        "date": 1770830238183,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.188406,
            "unit": "median cpu",
            "extra": "avg cpu: 21.08839989961961, max cpu: 42.814667, count: 57588"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 235.91015625,
            "unit": "median mem",
            "extra": "avg mem: 235.82466372117455, max mem: 237.3828125, count: 57588"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.30097,
            "unit": "median cpu",
            "extra": "avg cpu: 22.51760637332268, max cpu: 33.23442, count: 57588"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 175.3984375,
            "unit": "median mem",
            "extra": "avg mem: 175.28135452752744, max mem: 176.515625, count: 57588"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 34222,
            "unit": "median block_count",
            "extra": "avg block_count: 33581.166458289925, max block_count: 36261.0, count: 57588"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 79,
            "unit": "median segment_count",
            "extra": "avg segment_count: 81.35278182954782, max segment_count: 127.0, count: 57588"
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
          "id": "80daf35915e5edde9b7e091036a88ce3d6c6aea1",
          "message": "chore: Upgrade to `0.21.8` (#4168)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2026-02-12T09:45:33-05:00",
          "tree_id": "3de2b0c6e4f9a21b26370ecab28ddd8db57c65ff",
          "url": "https://github.com/paradedb/paradedb/commit/80daf35915e5edde9b7e091036a88ce3d6c6aea1"
        },
        "date": 1770909802057,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.233301,
            "unit": "median cpu",
            "extra": "avg cpu: 21.184001913734374, max cpu: 42.985077, count: 57596"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 236.41015625,
            "unit": "median mem",
            "extra": "avg mem: 236.25506355693105, max mem: 237.92578125, count: 57596"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.346306,
            "unit": "median cpu",
            "extra": "avg cpu: 22.353187560645182, max cpu: 33.333336, count: 57596"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 175.65625,
            "unit": "median mem",
            "extra": "avg mem: 175.46933102721977, max mem: 176.3984375, count: 57596"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 33801,
            "unit": "median block_count",
            "extra": "avg block_count: 33278.61129245086, max block_count: 35719.0, count: 57596"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 78,
            "unit": "median segment_count",
            "extra": "avg segment_count: 80.6069865962914, max segment_count: 126.0, count: 57596"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "2aec8447addadd4def1cf10f4d11e24c1755fadb",
          "message": "chore: Remove tuned_postgres from /benchmarks (#4167)\n\n# Ticket(s) Closed\n\n- Closes #N/A\n\n## What\nWe are moving user-facing benchmarks to a much more complete project,\nwhich will be in a separate repository. This is no longer relevant to\nkeep here, so removing.\n\n## Why\n^\n\n## How\n^\n\n## Tests\n^",
          "timestamp": "2026-02-12T12:21:38-05:00",
          "tree_id": "3502c5fdf7ad1b45110e95b70c181dd5ca1eae37",
          "url": "https://github.com/paradedb/paradedb/commit/2aec8447addadd4def1cf10f4d11e24c1755fadb"
        },
        "date": 1770918990672,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.233301,
            "unit": "median cpu",
            "extra": "avg cpu: 21.395133304554204, max cpu: 42.857143, count: 57864"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 236.19921875,
            "unit": "median mem",
            "extra": "avg mem: 236.1071346983876, max mem: 237.73046875, count: 57864"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.323614,
            "unit": "median cpu",
            "extra": "avg cpu: 22.322019498110382, max cpu: 33.3996, count: 57864"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 175.76171875,
            "unit": "median mem",
            "extra": "avg mem: 175.43184537503888, max mem: 176.32421875, count: 57864"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 33836,
            "unit": "median block_count",
            "extra": "avg block_count: 33269.37232130513, max block_count: 35997.0, count: 57864"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 78,
            "unit": "median segment_count",
            "extra": "avg segment_count: 80.85258537259782, max segment_count: 128.0, count: 57864"
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
          "id": "ba868f34636e9fc6068c68b3b0d8a098eb4971d8",
          "message": "feat: join-scan: pre-materialization dynamic filter pushdown from TopK and HashJoin (#4161)\n\n## Ticket(s) Closed\n\n- Closes #4151\n\n## What\n\nDynamic filters from DataFusion's `SortExec(TopK)` and `HashJoinExec`\nare now pushed down into `PgSearchScan` and applied *before* column\nmaterialization — at the term-ordinal level for strings and the\nfast-field level for numerics. This avoids expensive term dictionary I/O\nfor documents that will be discarded anyway.\n\n## Why\n\nPreviously, `PgSearchScan` had no awareness of dynamic filters. Every\ndocument that passed the Tantivy query and visibility checks was fully\nmaterialized (all fast-field columns loaded, string dictionaries walked)\nbefore any join-key or TopK pruning could happen upstream. For selective\njoins or tight LIMIT queries, this meant loading data for rows that were\nimmediately thrown away by HashJoin or TopK.\n\n## How\n\n- Enabled DataFusion's TopK dynamic filter pushdown in the JoinScan\nsession config.\n- `SegmentPlan` now accepts dynamic filters from parent operators (TopK\nthresholds, HashJoin key bounds) and passes them to the Scanner on each\nbatch.\n- Before column materialization, the Scanner converts these filters to\nterm-ordinal comparisons (for strings) or direct fast-field comparisons\n(for numerics) and prunes non-matching documents in-place — skipping\ndictionary I/O entirely for pruned rows.\n\n## Tests\n\n- New `topk_dynamic_filter` regression test covering. You can take a\nlook at EXPLAIN ANALYZE diff in the follow-up PR (#4162):\nhttps://github.com/paradedb/paradedb/blob/3b074a9b5516a7a0a75a948201ef32e07b0127e4/pg_search/tests/pg_regress/expected/topk_dynamic_filter.out#L170-L181\n- All existing regression tests pass.",
          "timestamp": "2026-02-12T10:25:25-08:00",
          "tree_id": "748bfdacf0d0b82f9ceb26840b3100a7ca8e2252",
          "url": "https://github.com/paradedb/paradedb/commit/ba868f34636e9fc6068c68b3b0d8a098eb4971d8"
        },
        "date": 1770922821786,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.210833,
            "unit": "median cpu",
            "extra": "avg cpu: 21.004813488793843, max cpu: 42.857143, count: 57760"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 235.5625,
            "unit": "median mem",
            "extra": "avg mem: 235.53686577540685, max mem: 237.1796875, count: 57760"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.323614,
            "unit": "median cpu",
            "extra": "avg cpu: 22.45919168185927, max cpu: 33.333336, count: 57760"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 175.171875,
            "unit": "median mem",
            "extra": "avg mem: 175.28630573980695, max mem: 176.30078125, count: 57760"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 34255,
            "unit": "median block_count",
            "extra": "avg block_count: 33470.48261772853, max block_count: 35836.0, count: 57760"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 78,
            "unit": "median segment_count",
            "extra": "avg segment_count: 81.03561288088643, max segment_count: 123.0, count: 57760"
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
          "id": "655ee8b04cab31c056cb41a89d677b896630ed16",
          "message": "feat: join-scan: surface dynamic filter metrics in EXPLAIN ANALYZE (#4162)\n\n# Ticket(s) Closed\n\n- Closes #4151\n\n## What\n\nUsed DataFusion metrics, and made dynamic filter pruning stats visible\nthrough `EXPLAIN ANALYZE`.\n\n## Why\n\n`EXPLAIN ANALYZE` is the natural place for execution-time stats.\n\n## How\n\n- Added `ExecutionPlanMetricsSet` to `SegmentPlan` with two custom\ncounters (`rows_scanned`, `rows_pruned`), only registered when dynamic\nfilters are present.\n\n## Tests\n\n- Updated `topk_dynamic_filter` regression test to use `EXPLAIN\n(ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)` — verifying\n`Dynamic Filter` lines appear with correct pruning stats (e.g., `30\nscanned, 24 pruned (80.0%)`).\n- Updated `join_custom_scan` and `filter_pushdown_datafusion` expected\noutput.\n\n---------\n\nSigned-off-by: Moe <mdashti@gmail.com>",
          "timestamp": "2026-02-12T12:45:44-08:00",
          "tree_id": "8c73104c0b40b30047e010ebfba45fb9add3f7e8",
          "url": "https://github.com/paradedb/paradedb/commit/655ee8b04cab31c056cb41a89d677b896630ed16"
        },
        "date": 1770931286506,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.188406,
            "unit": "median cpu",
            "extra": "avg cpu: 21.25296884466299, max cpu: 43.373497, count: 57893"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 235.7578125,
            "unit": "median mem",
            "extra": "avg mem: 235.60848954374882, max mem: 237.23828125, count: 57893"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.27837,
            "unit": "median cpu",
            "extra": "avg cpu: 22.43228396080314, max cpu: 33.23442, count: 57893"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 174.875,
            "unit": "median mem",
            "extra": "avg mem: 175.12067413158758, max mem: 175.96484375, count: 57893"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 33941,
            "unit": "median block_count",
            "extra": "avg block_count: 33442.75928005113, max block_count: 35959.0, count: 57893"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 78,
            "unit": "median segment_count",
            "extra": "avg segment_count: 80.92233948836648, max segment_count: 127.0, count: 57893"
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
          "id": "1da48a637b80cc0ce2312ab8c4e448762e152223",
          "message": "perf: Add statistics to joinscan (#4132)\n\n## Ticket(s) Closed\n\n- Closes #4062.\n\n## What\n\n* Exposes sorting from the joinscan's `TableProvider`, but does not yet\nforce `SortMergeJoin`.\n* Adds statistics on `TableProvider` and our `ExecutionPlan`s using\nTantivy's query estimates.\n* Removes the `ParallelSegmentPlan` that was added in #4101, as it makes\nmore sense to let DataFusion coalesce for us if needed.\n\n## Why\n\nTo allow the DataFusion optimizer to re-order joins based on table\nsizes, and use sortedness in plans (although it does not yet by\ndefault).\n\n## Tests\n\nExisting tests show the impact of join reordering due to statistics.",
          "timestamp": "2026-02-12T14:34:01-08:00",
          "tree_id": "fbc185b154055782f4973f483feb5ad00a4ca2bb",
          "url": "https://github.com/paradedb/paradedb/commit/1da48a637b80cc0ce2312ab8c4e448762e152223"
        },
        "date": 1770937723421,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.210833,
            "unit": "median cpu",
            "extra": "avg cpu: 21.008528798724385, max cpu: 42.899704, count: 57829"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 236.09765625,
            "unit": "median mem",
            "extra": "avg mem: 235.94593786205883, max mem: 237.57421875, count: 57829"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.346306,
            "unit": "median cpu",
            "extra": "avg cpu: 22.453882596444164, max cpu: 33.300297, count: 57829"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 175.26953125,
            "unit": "median mem",
            "extra": "avg mem: 175.12362539232046, max mem: 176.16796875, count: 57829"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 34055,
            "unit": "median block_count",
            "extra": "avg block_count: 33487.82475920386, max block_count: 35975.0, count: 57829"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 78,
            "unit": "median segment_count",
            "extra": "avg segment_count: 81.2388939805288, max segment_count: 125.0, count: 57829"
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
          "id": "b04eae77b43a69abf89d75cd16038e7fcdd72770",
          "message": "refactor: unify static OPERATOR_LOOKUP into shared function (#4060) (#4173)\n\n## Summary\n\n- Deduplicate the `OnceLock<HashMap<PostgresOperatorOid,\nTantivyOperator>>` pattern that was copy-pasted across `pushdown.rs`,\n`planning.rs`, and `translator.rs`\n- Centralize into a single `pub(crate) fn lookup_operator(opno)` in\n`opexpr.rs`\n- Privatize `OperatorAccepts` and `initialize_equality_operator_lookup`\nsince they are no longer needed outside `opexpr.rs`\n\nCloses #4060\n\n## Test plan\n\nNo new tests added. This is a pure refactoring with no behavioral\nchange. Existing integration and regression tests provide full coverage\nof all modified call sites.",
          "timestamp": "2026-02-12T15:09:31-08:00",
          "tree_id": "4395a9fad346a49f0bcb0a75092401e526f3830c",
          "url": "https://github.com/paradedb/paradedb/commit/b04eae77b43a69abf89d75cd16038e7fcdd72770"
        },
        "date": 1770939862465,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.166023,
            "unit": "median cpu",
            "extra": "avg cpu: 21.077764847758793, max cpu: 42.942345, count: 57332"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 235.9609375,
            "unit": "median mem",
            "extra": "avg mem: 235.8667928687295, max mem: 237.44140625, count: 57332"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.30097,
            "unit": "median cpu",
            "extra": "avg cpu: 22.348473189509306, max cpu: 33.23442, count: 57332"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 175.37890625,
            "unit": "median mem",
            "extra": "avg mem: 175.30339895259192, max mem: 176.0625, count: 57332"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 34109,
            "unit": "median block_count",
            "extra": "avg block_count: 33491.55089653248, max block_count: 36087.0, count: 57332"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 79,
            "unit": "median segment_count",
            "extra": "avg segment_count: 81.3209725807577, max segment_count: 129.0, count: 57332"
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
          "id": "30c4c9cbf69783f13dbfa4ed26b331f441be585c",
          "message": "perf: Use parallel workers for the join scan (#4101)\n\n## Ticket(s) Closed\n\n- Closes #4063\n\n## What\n\nAdds support for parallel workers in the `joinscan`, by relying (for\nnow) on the fact that we only support INNER joins, and can thus do a\nbroadcast join.\n\n## Why\n\nTo get an implementation of parallel workers in place without (yet)\ntackling the problem of partitioning DataFusion plans across parallel\nworkers and introducing RPC.\n\n## How\n\n- Implemented a \"broadcast join\" strategy for `JoinScan` where the\nlargest index scan is partitioned across workers while the others are\nreplicated.\n- Introduced `ParallelSegmentPlan` and `ParallelScanStream` for dynamic\nworker-driven scanning.\n- This strategy is necessary in order to continue to use the lazy work\nclaiming strategy that we use in `ParallelScanState`, but after #4062\nthe replicated/un-partitioned indexes could begin using\n`MultiSegmentPlan` to provide sorted access.\n- In future, if/when we change our parallel worker partitioning\nstrategy, we might be able to use `MultiSegmentPlan` and assign _ranges_\nof an index to the parallel workers. TBD.\n- Centralized `RowEstimate` handling to better manage unanalyzed tables,\nand ease determining the largest index to scan.\n- Cleaned up registration of the `CustomScan`'s vtable\n(`CustomExecMethods`).\n- Before this, encountered some segfaults due to registration issues\naround having multiple parallel `CustomScan` implementations.\n- Remove \"lazy checkout\" from `MultiSegmentPlan`, as no consumer will\nactually use it lazily.\n\n## Tests\n\nExisting tests (and proptests) pass.\n\nBenchmarks show speedups across a few of our joins. Notably: we are\nfaster than Postgres for the `semi_join_filter` join for the first time.",
          "timestamp": "2026-02-09T17:53:18-08:00",
          "tree_id": "6616d18d10f8cf9e48caa5c264c26297828fd02b",
          "url": "https://github.com/paradedb/paradedb/commit/30c4c9cbf69783f13dbfa4ed26b331f441be585c"
        },
        "date": 1770691481768,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 1098.0423592839659,
            "unit": "median tps",
            "extra": "avg tps: 1100.6077733235857, max tps: 1141.5840679968992, count: 56047"
          },
          {
            "name": "Single Insert - Primary - tps",
            "value": 1230.4496943869392,
            "unit": "median tps",
            "extra": "avg tps: 1226.885902355026, max tps: 1246.477804641551, count: 56047"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 1868.5699245697276,
            "unit": "median tps",
            "extra": "avg tps: 1843.207000563869, max tps: 2014.5476335535614, count: 56047"
          },
          {
            "name": "Top N - Primary - tps",
            "value": 5.376099275101758,
            "unit": "median tps",
            "extra": "avg tps: 5.400312166640497, max tps: 7.022703180944993, count: 56047"
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
          "id": "ff10528e88ec12dd794b2a6e8b75996ad447a713",
          "message": "fix: Joinscan row estimation needs `ExprContext` (#4147)\n\n# Ticket(s) Closed\n\n- Closes #4146 \n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2026-02-10T14:16:35-05:00",
          "tree_id": "bba16dfdbd260ac92fb27be55e3b411b77039476",
          "url": "https://github.com/paradedb/paradedb/commit/ff10528e88ec12dd794b2a6e8b75996ad447a713"
        },
        "date": 1770754033835,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 1109.8890074485907,
            "unit": "median tps",
            "extra": "avg tps: 1112.1284022607467, max tps: 1165.8484013659233, count: 55373"
          },
          {
            "name": "Single Insert - Primary - tps",
            "value": 1195.3043944286387,
            "unit": "median tps",
            "extra": "avg tps: 1195.7135066676394, max tps: 1211.4208128830899, count: 55373"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 1888.2904642403307,
            "unit": "median tps",
            "extra": "avg tps: 1862.412973007843, max tps: 2048.939630510873, count: 55373"
          },
          {
            "name": "Top N - Primary - tps",
            "value": 5.601855236911998,
            "unit": "median tps",
            "extra": "avg tps: 5.5869130916079985, max tps: 6.3053123118794305, count: 55373"
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
          "id": "084451f652ebc5c322fbf12b0189bc5e229dce3a",
          "message": "fix: reduce overhead for ngram match queries and add TEXT[] regression coverage (#4150)\n\n# Ticket(s) Closed\n\n- Closes #2884\n\n## What\n\nMinor optimization to `match_query` and new regression test covering\nngram search on TEXT[] columns with `conjunction_mode`.\n\n## Why\n\nA it's reported in #2884, slow ngram searches (~16 queries/s vs ~70\nwithout ngram) on a 350k-row TEXT[] column. We investigated and found\nthe N-way posting list intersection in `BooleanQuery` with many Must\nclauses is inherently expensive and can't be fundamentally improved at\nthe pg_search level. However, we identified two sources of unnecessary\noverhead in how `match_query` constructs the query.\n\n## How\n\n1. **`IndexRecordOption::WithFreqs` instead of `WithFreqsAndPositions`**\n— `match_query` creates `TermQuery` instances inside a `BooleanQuery`.\nThe BooleanQuery scorer only uses doc iteration and BM25 scores, never\npositions. `WithFreqsAndPositions` was requesting position data that was\nnever read. `WithFreqs` produces identical BM25 scores with less\nper-document overhead.\n\n2. **Deduplicate terms for conjunction mode** — For queries with\nrepeated ngram tokens (e.g., strings with repeated substrings),\nduplicate Must clauses add intersection work without changing which\ndocuments match. Dedup removes them before building the query.\n\nBoth changes preserve identical matching semantics and BM25 scoring.\n\n## Tests\n\nNew `ngram-text-array` regression test covering the exact pattern from\nthe reported issue: TEXT[] column with ICU + ngram alias fields, `match`\nwith `conjunction_mode`, `disjunction_max`, edge cases (short queries,\nsingle-token queries), and the JSON `::jsonb` query path.",
          "timestamp": "2026-02-10T15:11:24-08:00",
          "tree_id": "ce5fefd07b9871c52c5cd32b82b7f79613310334",
          "url": "https://github.com/paradedb/paradedb/commit/084451f652ebc5c322fbf12b0189bc5e229dce3a"
        },
        "date": 1770768114198,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 1138.7621385996117,
            "unit": "median tps",
            "extra": "avg tps: 1139.5674881733623, max tps: 1187.4389777339322, count: 56262"
          },
          {
            "name": "Single Insert - Primary - tps",
            "value": 1279.2329310216578,
            "unit": "median tps",
            "extra": "avg tps: 1273.4208647784003, max tps: 1294.7039736583133, count: 56262"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 1808.9313625231289,
            "unit": "median tps",
            "extra": "avg tps: 1793.2262204644308, max tps: 1937.9017651068864, count: 56262"
          },
          {
            "name": "Top N - Primary - tps",
            "value": 5.450642681828197,
            "unit": "median tps",
            "extra": "avg tps: 5.461248320007809, max tps: 7.450437325397604, count: 56262"
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
          "id": "59354e0de782d993f3e4a260eb7c56ad4804a1ad",
          "message": "fix: add field validation for `paradedb.aggregate()` API (#4141)\n\n# Ticket(s) Closed\n\n- Closes #N/A\n\n## What\n\nAdds field validation to the `paradedb.aggregate()` direct SQL function\nso that referencing a nonexistent or unsupported field returns a clear\nerror instead of silently producing null results.\n\n## Why\n\nThe `pdb.agg()` aggregate path already validates fields at plan time via\n`AggregateType::validate_fields()`. However, the `paradedb.aggregate()`\nfunction is a plain `pg_extern` that calls `execute_aggregate()`\ndirectly — it never enters the custom scan planner, so it skipped\nvalidation entirely. An invalid field like `\"nonexistent_field\"` would\nquietly return `{\"value\": null}` instead of telling the user something\nis wrong.\n\n## How\n\n- Extracted the field validation logic from\n`AggregateType::validate_fields()` into a standalone\n`validate_agg_json_fields()` function in `aggregate_type.rs`. The\nexisting `validate_fields()` now delegates to it for custom aggregates.\n- Called `validate_agg_json_fields()` in `aggregate_impl()`\n(`api/aggregate.rs`) before executing, so the direct API gets the same\nvalidation as the planner path.\n\n## Tests\n\n- Added regression tests (tests 13–15 in `agg-validate.sql`) covering\nthe `paradedb.aggregate()` path: valid field succeeds, invalid field\nerrors, invalid nested field errors.",
          "timestamp": "2026-02-10T15:12:54-08:00",
          "tree_id": "a2a30dc05294896dfaef747d15452a4024f5d8aa",
          "url": "https://github.com/paradedb/paradedb/commit/59354e0de782d993f3e4a260eb7c56ad4804a1ad"
        },
        "date": 1770768221038,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 1106.2051196767181,
            "unit": "median tps",
            "extra": "avg tps: 1108.4661162198922, max tps: 1156.4949232463425, count: 56320"
          },
          {
            "name": "Single Insert - Primary - tps",
            "value": 1274.559800697776,
            "unit": "median tps",
            "extra": "avg tps: 1266.9594148584538, max tps: 1287.042219975652, count: 56320"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 1854.8371404435406,
            "unit": "median tps",
            "extra": "avg tps: 1832.9960958286972, max tps: 1980.613148418794, count: 56320"
          },
          {
            "name": "Top N - Primary - tps",
            "value": 5.333188707534094,
            "unit": "median tps",
            "extra": "avg tps: 5.333463298540279, max tps: 7.007627108352358, count: 56320"
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
          "id": "44d1f928882599cf5fd9fbc853c8eee1fb5c57ed",
          "message": "fix: Rebase against Tantivy, inherit upstream bugfix for intersection queries (#4155)\n\n# Ticket(s) Closed\n\n- Closes #4149 \n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2026-02-11T11:47:17-05:00",
          "tree_id": "b88ad04015b13dee26d44c4c9d585ea252d0323c",
          "url": "https://github.com/paradedb/paradedb/commit/44d1f928882599cf5fd9fbc853c8eee1fb5c57ed"
        },
        "date": 1770831174151,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 1147.0623441200182,
            "unit": "median tps",
            "extra": "avg tps: 1147.4965964019423, max tps: 1203.378845800133, count: 56346"
          },
          {
            "name": "Single Insert - Primary - tps",
            "value": 1257.4449303647664,
            "unit": "median tps",
            "extra": "avg tps: 1244.379711732735, max tps: 1266.2609393284172, count: 56346"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 1930.7629646392554,
            "unit": "median tps",
            "extra": "avg tps: 1897.902775012248, max tps: 2091.47590765199, count: 56346"
          },
          {
            "name": "Top N - Primary - tps",
            "value": 5.470943835675098,
            "unit": "median tps",
            "extra": "avg tps: 5.464849752487045, max tps: 6.51400590088161, count: 56346"
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
          "id": "80daf35915e5edde9b7e091036a88ce3d6c6aea1",
          "message": "chore: Upgrade to `0.21.8` (#4168)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2026-02-12T09:45:33-05:00",
          "tree_id": "3de2b0c6e4f9a21b26370ecab28ddd8db57c65ff",
          "url": "https://github.com/paradedb/paradedb/commit/80daf35915e5edde9b7e091036a88ce3d6c6aea1"
        },
        "date": 1770910741073,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 1140.0905264672072,
            "unit": "median tps",
            "extra": "avg tps: 1143.9183051540574, max tps: 1203.3130341060237, count: 56593"
          },
          {
            "name": "Single Insert - Primary - tps",
            "value": 1214.141213822769,
            "unit": "median tps",
            "extra": "avg tps: 1212.9905264400772, max tps: 1231.9830389961728, count: 56593"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 1950.1027403606627,
            "unit": "median tps",
            "extra": "avg tps: 1924.1331131096053, max tps: 2111.3380329515144, count: 56593"
          },
          {
            "name": "Top N - Primary - tps",
            "value": 5.348806687802424,
            "unit": "median tps",
            "extra": "avg tps: 5.3333794284988825, max tps: 6.608296639774997, count: 56593"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "2aec8447addadd4def1cf10f4d11e24c1755fadb",
          "message": "chore: Remove tuned_postgres from /benchmarks (#4167)\n\n# Ticket(s) Closed\n\n- Closes #N/A\n\n## What\nWe are moving user-facing benchmarks to a much more complete project,\nwhich will be in a separate repository. This is no longer relevant to\nkeep here, so removing.\n\n## Why\n^\n\n## How\n^\n\n## Tests\n^",
          "timestamp": "2026-02-12T12:21:38-05:00",
          "tree_id": "3502c5fdf7ad1b45110e95b70c181dd5ca1eae37",
          "url": "https://github.com/paradedb/paradedb/commit/2aec8447addadd4def1cf10f4d11e24c1755fadb"
        },
        "date": 1770919960658,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 1133.4452825371663,
            "unit": "median tps",
            "extra": "avg tps: 1134.8222817269793, max tps: 1176.9480749513218, count: 56432"
          },
          {
            "name": "Single Insert - Primary - tps",
            "value": 1244.3673930628634,
            "unit": "median tps",
            "extra": "avg tps: 1239.1522061767262, max tps: 1256.3455950232665, count: 56432"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 1888.2179610464677,
            "unit": "median tps",
            "extra": "avg tps: 1871.853329600306, max tps: 2044.8899236785803, count: 56432"
          },
          {
            "name": "Top N - Primary - tps",
            "value": 5.2956470396145825,
            "unit": "median tps",
            "extra": "avg tps: 5.319743852739979, max tps: 7.7730338175086136, count: 56432"
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
          "id": "ba868f34636e9fc6068c68b3b0d8a098eb4971d8",
          "message": "feat: join-scan: pre-materialization dynamic filter pushdown from TopK and HashJoin (#4161)\n\n## Ticket(s) Closed\n\n- Closes #4151\n\n## What\n\nDynamic filters from DataFusion's `SortExec(TopK)` and `HashJoinExec`\nare now pushed down into `PgSearchScan` and applied *before* column\nmaterialization — at the term-ordinal level for strings and the\nfast-field level for numerics. This avoids expensive term dictionary I/O\nfor documents that will be discarded anyway.\n\n## Why\n\nPreviously, `PgSearchScan` had no awareness of dynamic filters. Every\ndocument that passed the Tantivy query and visibility checks was fully\nmaterialized (all fast-field columns loaded, string dictionaries walked)\nbefore any join-key or TopK pruning could happen upstream. For selective\njoins or tight LIMIT queries, this meant loading data for rows that were\nimmediately thrown away by HashJoin or TopK.\n\n## How\n\n- Enabled DataFusion's TopK dynamic filter pushdown in the JoinScan\nsession config.\n- `SegmentPlan` now accepts dynamic filters from parent operators (TopK\nthresholds, HashJoin key bounds) and passes them to the Scanner on each\nbatch.\n- Before column materialization, the Scanner converts these filters to\nterm-ordinal comparisons (for strings) or direct fast-field comparisons\n(for numerics) and prunes non-matching documents in-place — skipping\ndictionary I/O entirely for pruned rows.\n\n## Tests\n\n- New `topk_dynamic_filter` regression test covering. You can take a\nlook at EXPLAIN ANALYZE diff in the follow-up PR (#4162):\nhttps://github.com/paradedb/paradedb/blob/3b074a9b5516a7a0a75a948201ef32e07b0127e4/pg_search/tests/pg_regress/expected/topk_dynamic_filter.out#L170-L181\n- All existing regression tests pass.",
          "timestamp": "2026-02-12T10:25:25-08:00",
          "tree_id": "748bfdacf0d0b82f9ceb26840b3100a7ca8e2252",
          "url": "https://github.com/paradedb/paradedb/commit/ba868f34636e9fc6068c68b3b0d8a098eb4971d8"
        },
        "date": 1770923766529,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 1109.83745462343,
            "unit": "median tps",
            "extra": "avg tps: 1112.109644926575, max tps: 1151.997066400359, count: 56332"
          },
          {
            "name": "Single Insert - Primary - tps",
            "value": 1289.2686572151176,
            "unit": "median tps",
            "extra": "avg tps: 1277.5422586958884, max tps: 1302.5985115877527, count: 56332"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 1831.3783752571971,
            "unit": "median tps",
            "extra": "avg tps: 1811.874001181249, max tps: 1974.3564989197944, count: 56332"
          },
          {
            "name": "Top N - Primary - tps",
            "value": 5.143667476894732,
            "unit": "median tps",
            "extra": "avg tps: 5.197070659238286, max tps: 8.074810307464674, count: 56332"
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
          "id": "655ee8b04cab31c056cb41a89d677b896630ed16",
          "message": "feat: join-scan: surface dynamic filter metrics in EXPLAIN ANALYZE (#4162)\n\n# Ticket(s) Closed\n\n- Closes #4151\n\n## What\n\nUsed DataFusion metrics, and made dynamic filter pruning stats visible\nthrough `EXPLAIN ANALYZE`.\n\n## Why\n\n`EXPLAIN ANALYZE` is the natural place for execution-time stats.\n\n## How\n\n- Added `ExecutionPlanMetricsSet` to `SegmentPlan` with two custom\ncounters (`rows_scanned`, `rows_pruned`), only registered when dynamic\nfilters are present.\n\n## Tests\n\n- Updated `topk_dynamic_filter` regression test to use `EXPLAIN\n(ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)` — verifying\n`Dynamic Filter` lines appear with correct pruning stats (e.g., `30\nscanned, 24 pruned (80.0%)`).\n- Updated `join_custom_scan` and `filter_pushdown_datafusion` expected\noutput.\n\n---------\n\nSigned-off-by: Moe <mdashti@gmail.com>",
          "timestamp": "2026-02-12T12:45:44-08:00",
          "tree_id": "8c73104c0b40b30047e010ebfba45fb9add3f7e8",
          "url": "https://github.com/paradedb/paradedb/commit/655ee8b04cab31c056cb41a89d677b896630ed16"
        },
        "date": 1770932252544,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 1076.4805752607176,
            "unit": "median tps",
            "extra": "avg tps: 1080.224438119592, max tps: 1122.7420745147424, count: 56485"
          },
          {
            "name": "Single Insert - Primary - tps",
            "value": 1230.8527593016117,
            "unit": "median tps",
            "extra": "avg tps: 1227.728526100698, max tps: 1244.7590032900057, count: 56485"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 1802.0962966938887,
            "unit": "median tps",
            "extra": "avg tps: 1778.8752315274255, max tps: 1919.7596822244384, count: 56485"
          },
          {
            "name": "Top N - Primary - tps",
            "value": 5.188874684087461,
            "unit": "median tps",
            "extra": "avg tps: 5.21858479055741, max tps: 7.024892834206081, count: 56485"
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
          "id": "1da48a637b80cc0ce2312ab8c4e448762e152223",
          "message": "perf: Add statistics to joinscan (#4132)\n\n## Ticket(s) Closed\n\n- Closes #4062.\n\n## What\n\n* Exposes sorting from the joinscan's `TableProvider`, but does not yet\nforce `SortMergeJoin`.\n* Adds statistics on `TableProvider` and our `ExecutionPlan`s using\nTantivy's query estimates.\n* Removes the `ParallelSegmentPlan` that was added in #4101, as it makes\nmore sense to let DataFusion coalesce for us if needed.\n\n## Why\n\nTo allow the DataFusion optimizer to re-order joins based on table\nsizes, and use sortedness in plans (although it does not yet by\ndefault).\n\n## Tests\n\nExisting tests show the impact of join reordering due to statistics.",
          "timestamp": "2026-02-12T14:34:01-08:00",
          "tree_id": "fbc185b154055782f4973f483feb5ad00a4ca2bb",
          "url": "https://github.com/paradedb/paradedb/commit/1da48a637b80cc0ce2312ab8c4e448762e152223"
        },
        "date": 1770938691116,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 1134.8117612948631,
            "unit": "median tps",
            "extra": "avg tps: 1134.0403135041088, max tps: 1198.898954212904, count: 56030"
          },
          {
            "name": "Single Insert - Primary - tps",
            "value": 1282.1340837310406,
            "unit": "median tps",
            "extra": "avg tps: 1261.2303483429964, max tps: 1297.1202262898923, count: 56030"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 1976.9000346681087,
            "unit": "median tps",
            "extra": "avg tps: 1936.511655218023, max tps: 2151.927423923557, count: 56030"
          },
          {
            "name": "Top N - Primary - tps",
            "value": 5.3911393620579515,
            "unit": "median tps",
            "extra": "avg tps: 5.394584724177951, max tps: 7.260330140207466, count: 56030"
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
          "id": "30c4c9cbf69783f13dbfa4ed26b331f441be585c",
          "message": "perf: Use parallel workers for the join scan (#4101)\n\n## Ticket(s) Closed\n\n- Closes #4063\n\n## What\n\nAdds support for parallel workers in the `joinscan`, by relying (for\nnow) on the fact that we only support INNER joins, and can thus do a\nbroadcast join.\n\n## Why\n\nTo get an implementation of parallel workers in place without (yet)\ntackling the problem of partitioning DataFusion plans across parallel\nworkers and introducing RPC.\n\n## How\n\n- Implemented a \"broadcast join\" strategy for `JoinScan` where the\nlargest index scan is partitioned across workers while the others are\nreplicated.\n- Introduced `ParallelSegmentPlan` and `ParallelScanStream` for dynamic\nworker-driven scanning.\n- This strategy is necessary in order to continue to use the lazy work\nclaiming strategy that we use in `ParallelScanState`, but after #4062\nthe replicated/un-partitioned indexes could begin using\n`MultiSegmentPlan` to provide sorted access.\n- In future, if/when we change our parallel worker partitioning\nstrategy, we might be able to use `MultiSegmentPlan` and assign _ranges_\nof an index to the parallel workers. TBD.\n- Centralized `RowEstimate` handling to better manage unanalyzed tables,\nand ease determining the largest index to scan.\n- Cleaned up registration of the `CustomScan`'s vtable\n(`CustomExecMethods`).\n- Before this, encountered some segfaults due to registration issues\naround having multiple parallel `CustomScan` implementations.\n- Remove \"lazy checkout\" from `MultiSegmentPlan`, as no consumer will\nactually use it lazily.\n\n## Tests\n\nExisting tests (and proptests) pass.\n\nBenchmarks show speedups across a few of our joins. Notably: we are\nfaster than Postgres for the `semi_join_filter` join for the first time.",
          "timestamp": "2026-02-09T17:53:18-08:00",
          "tree_id": "6616d18d10f8cf9e48caa5c264c26297828fd02b",
          "url": "https://github.com/paradedb/paradedb/commit/30c4c9cbf69783f13dbfa4ed26b331f441be585c"
        },
        "date": 1770691487030,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Background Merger - Primary - background_merging",
            "value": 0,
            "unit": "median background_merging",
            "extra": "avg background_merging: 0.08014701946580549, max background_merging: 2.0, count: 56047"
          },
          {
            "name": "Background Merger - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.78034936111944, max cpu: 9.667674, count: 56047"
          },
          {
            "name": "Background Merger - Primary - mem",
            "value": 24.71484375,
            "unit": "median mem",
            "extra": "avg mem: 24.760412996012274, max mem: 24.8359375, count: 56047"
          },
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.960115977688028, max cpu: 14.243324, count: 56047"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 168.48046875,
            "unit": "median mem",
            "extra": "avg mem: 167.11866317276125, max mem: 168.7734375, count: 56047"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 51594,
            "unit": "median block_count",
            "extra": "avg block_count: 51455.523685478256, max block_count: 51594.0, count: 56047"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 45,
            "unit": "median segment_count",
            "extra": "avg segment_count: 42.461184363123806, max segment_count: 62.0, count: 56047"
          },
          {
            "name": "Single Insert - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.651393938876497, max cpu: 9.458128, count: 56047"
          },
          {
            "name": "Single Insert - Primary - mem",
            "value": 124.01953125,
            "unit": "median mem",
            "extra": "avg mem: 112.9052134118463, max mem: 139.07421875, count: 56047"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.770450868992074, max cpu: 9.504951, count: 56047"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 168.1640625,
            "unit": "median mem",
            "extra": "avg mem: 163.76768430680946, max mem: 168.2421875, count: 56047"
          },
          {
            "name": "Top N - Primary - cpu",
            "value": 23.369036,
            "unit": "median cpu",
            "extra": "avg cpu: 24.18832752182028, max cpu: 33.870968, count: 56047"
          },
          {
            "name": "Top N - Primary - mem",
            "value": 162.4921875,
            "unit": "median mem",
            "extra": "avg mem: 181.17679400213214, max mem: 223.0234375, count: 56047"
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
          "id": "ff10528e88ec12dd794b2a6e8b75996ad447a713",
          "message": "fix: Joinscan row estimation needs `ExprContext` (#4147)\n\n# Ticket(s) Closed\n\n- Closes #4146 \n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2026-02-10T14:16:35-05:00",
          "tree_id": "bba16dfdbd260ac92fb27be55e3b411b77039476",
          "url": "https://github.com/paradedb/paradedb/commit/ff10528e88ec12dd794b2a6e8b75996ad447a713"
        },
        "date": 1770754038717,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Background Merger - Primary - background_merging",
            "value": 0,
            "unit": "median background_merging",
            "extra": "avg background_merging: 0.0812489841619562, max background_merging: 2.0, count: 55373"
          },
          {
            "name": "Background Merger - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.67341241624782, max cpu: 9.495549, count: 55373"
          },
          {
            "name": "Background Merger - Primary - mem",
            "value": 28.3828125,
            "unit": "median mem",
            "extra": "avg mem: 28.373747344712225, max mem: 28.38671875, count: 55373"
          },
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 5.001762531674406, max cpu: 11.311861, count: 55373"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 168.55078125,
            "unit": "median mem",
            "extra": "avg mem: 167.11395932189424, max mem: 168.625, count: 55373"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 51163,
            "unit": "median block_count",
            "extra": "avg block_count: 51012.76248352085, max block_count: 51163.0, count: 55373"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 45,
            "unit": "median segment_count",
            "extra": "avg segment_count: 42.34365123796796, max segment_count: 62.0, count: 55373"
          },
          {
            "name": "Single Insert - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.581058017132775, max cpu: 9.458128, count: 55373"
          },
          {
            "name": "Single Insert - Primary - mem",
            "value": 123.1171875,
            "unit": "median mem",
            "extra": "avg mem: 111.4075578209371, max mem: 137.01953125, count: 55373"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.7782091114385805, max cpu: 9.514371, count: 55373"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 168.14453125,
            "unit": "median mem",
            "extra": "avg mem: 163.91260202117908, max mem: 168.25, count: 55373"
          },
          {
            "name": "Top N - Primary - cpu",
            "value": 23.323614,
            "unit": "median cpu",
            "extra": "avg cpu: 23.69143616272746, max cpu: 33.333336, count: 55373"
          },
          {
            "name": "Top N - Primary - mem",
            "value": 162.44921875,
            "unit": "median mem",
            "extra": "avg mem: 182.49402510869467, max mem: 222.91015625, count: 55373"
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
          "id": "084451f652ebc5c322fbf12b0189bc5e229dce3a",
          "message": "fix: reduce overhead for ngram match queries and add TEXT[] regression coverage (#4150)\n\n# Ticket(s) Closed\n\n- Closes #2884\n\n## What\n\nMinor optimization to `match_query` and new regression test covering\nngram search on TEXT[] columns with `conjunction_mode`.\n\n## Why\n\nA it's reported in #2884, slow ngram searches (~16 queries/s vs ~70\nwithout ngram) on a 350k-row TEXT[] column. We investigated and found\nthe N-way posting list intersection in `BooleanQuery` with many Must\nclauses is inherently expensive and can't be fundamentally improved at\nthe pg_search level. However, we identified two sources of unnecessary\noverhead in how `match_query` constructs the query.\n\n## How\n\n1. **`IndexRecordOption::WithFreqs` instead of `WithFreqsAndPositions`**\n— `match_query` creates `TermQuery` instances inside a `BooleanQuery`.\nThe BooleanQuery scorer only uses doc iteration and BM25 scores, never\npositions. `WithFreqsAndPositions` was requesting position data that was\nnever read. `WithFreqs` produces identical BM25 scores with less\nper-document overhead.\n\n2. **Deduplicate terms for conjunction mode** — For queries with\nrepeated ngram tokens (e.g., strings with repeated substrings),\nduplicate Must clauses add intersection work without changing which\ndocuments match. Dedup removes them before building the query.\n\nBoth changes preserve identical matching semantics and BM25 scoring.\n\n## Tests\n\nNew `ngram-text-array` regression test covering the exact pattern from\nthe reported issue: TEXT[] column with ICU + ngram alias fields, `match`\nwith `conjunction_mode`, `disjunction_max`, edge cases (short queries,\nsingle-token queries), and the JSON `::jsonb` query path.",
          "timestamp": "2026-02-10T15:11:24-08:00",
          "tree_id": "ce5fefd07b9871c52c5cd32b82b7f79613310334",
          "url": "https://github.com/paradedb/paradedb/commit/084451f652ebc5c322fbf12b0189bc5e229dce3a"
        },
        "date": 1770768118794,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Background Merger - Primary - background_merging",
            "value": 0,
            "unit": "median background_merging",
            "extra": "avg background_merging: 0.07943194340762859, max background_merging: 2.0, count: 56262"
          },
          {
            "name": "Background Merger - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.734356746238106, max cpu: 9.657948, count: 56262"
          },
          {
            "name": "Background Merger - Primary - mem",
            "value": 27.9375,
            "unit": "median mem",
            "extra": "avg mem: 27.928240864171197, max mem: 27.94140625, count: 56262"
          },
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.988346912386232, max cpu: 13.994169, count: 56262"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 168.60546875,
            "unit": "median mem",
            "extra": "avg mem: 167.29037076257066, max mem: 168.95703125, count: 56262"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 50863,
            "unit": "median block_count",
            "extra": "avg block_count: 50725.116366286304, max block_count: 50863.0, count: 56262"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 45,
            "unit": "median segment_count",
            "extra": "avg segment_count: 41.597205929401724, max segment_count: 61.0, count: 56262"
          },
          {
            "name": "Single Insert - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.5869856425257645, max cpu: 9.504951, count: 56262"
          },
          {
            "name": "Single Insert - Primary - mem",
            "value": 122.6953125,
            "unit": "median mem",
            "extra": "avg mem: 112.86584195371654, max mem: 138.4453125, count: 56262"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.731566524062671, max cpu: 9.648242, count: 56262"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 168.3984375,
            "unit": "median mem",
            "extra": "avg mem: 163.9821762329592, max mem: 168.5625, count: 56262"
          },
          {
            "name": "Top N - Primary - cpu",
            "value": 23.323614,
            "unit": "median cpu",
            "extra": "avg cpu: 23.71756430608774, max cpu: 33.333336, count: 56262"
          },
          {
            "name": "Top N - Primary - mem",
            "value": 162.71875,
            "unit": "median mem",
            "extra": "avg mem: 181.59294246394103, max mem: 223.1484375, count: 56262"
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
          "id": "59354e0de782d993f3e4a260eb7c56ad4804a1ad",
          "message": "fix: add field validation for `paradedb.aggregate()` API (#4141)\n\n# Ticket(s) Closed\n\n- Closes #N/A\n\n## What\n\nAdds field validation to the `paradedb.aggregate()` direct SQL function\nso that referencing a nonexistent or unsupported field returns a clear\nerror instead of silently producing null results.\n\n## Why\n\nThe `pdb.agg()` aggregate path already validates fields at plan time via\n`AggregateType::validate_fields()`. However, the `paradedb.aggregate()`\nfunction is a plain `pg_extern` that calls `execute_aggregate()`\ndirectly — it never enters the custom scan planner, so it skipped\nvalidation entirely. An invalid field like `\"nonexistent_field\"` would\nquietly return `{\"value\": null}` instead of telling the user something\nis wrong.\n\n## How\n\n- Extracted the field validation logic from\n`AggregateType::validate_fields()` into a standalone\n`validate_agg_json_fields()` function in `aggregate_type.rs`. The\nexisting `validate_fields()` now delegates to it for custom aggregates.\n- Called `validate_agg_json_fields()` in `aggregate_impl()`\n(`api/aggregate.rs`) before executing, so the direct API gets the same\nvalidation as the planner path.\n\n## Tests\n\n- Added regression tests (tests 13–15 in `agg-validate.sql`) covering\nthe `paradedb.aggregate()` path: valid field succeeds, invalid field\nerrors, invalid nested field errors.",
          "timestamp": "2026-02-10T15:12:54-08:00",
          "tree_id": "a2a30dc05294896dfaef747d15452a4024f5d8aa",
          "url": "https://github.com/paradedb/paradedb/commit/59354e0de782d993f3e4a260eb7c56ad4804a1ad"
        },
        "date": 1770768225666,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Background Merger - Primary - background_merging",
            "value": 0,
            "unit": "median background_merging",
            "extra": "avg background_merging: 0.0805752840909091, max background_merging: 2.0, count: 56320"
          },
          {
            "name": "Background Merger - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.678458849755706, max cpu: 9.67742, count: 56320"
          },
          {
            "name": "Background Merger - Primary - mem",
            "value": 25.1875,
            "unit": "median mem",
            "extra": "avg mem: 25.17456970214844, max mem: 25.19140625, count: 56320"
          },
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 5.037142164670159, max cpu: 14.385615, count: 56320"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 168.55078125,
            "unit": "median mem",
            "extra": "avg mem: 167.20983158458364, max mem: 168.76953125, count: 56320"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 51423,
            "unit": "median block_count",
            "extra": "avg block_count: 51281.476171875, max block_count: 51423.0, count: 56320"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 45,
            "unit": "median segment_count",
            "extra": "avg segment_count: 42.641921164772725, max segment_count: 56.0, count: 56320"
          },
          {
            "name": "Single Insert - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 4.699388799756743, max cpu: 9.514371, count: 56320"
          },
          {
            "name": "Single Insert - Primary - mem",
            "value": 125.04296875,
            "unit": "median mem",
            "extra": "avg mem: 114.17963527332653, max mem: 139.453125, count: 56320"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.74054807758374, max cpu: 9.514371, count: 56320"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 168.09765625,
            "unit": "median mem",
            "extra": "avg mem: 164.02823222767222, max mem: 168.34375, count: 56320"
          },
          {
            "name": "Top N - Primary - cpu",
            "value": 23.369036,
            "unit": "median cpu",
            "extra": "avg cpu: 23.920687684627985, max cpu: 33.73494, count: 56320"
          },
          {
            "name": "Top N - Primary - mem",
            "value": 162.7578125,
            "unit": "median mem",
            "extra": "avg mem: 181.46491636796432, max mem: 223.10546875, count: 56320"
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
          "id": "44d1f928882599cf5fd9fbc853c8eee1fb5c57ed",
          "message": "fix: Rebase against Tantivy, inherit upstream bugfix for intersection queries (#4155)\n\n# Ticket(s) Closed\n\n- Closes #4149 \n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2026-02-11T11:47:17-05:00",
          "tree_id": "b88ad04015b13dee26d44c4c9d585ea252d0323c",
          "url": "https://github.com/paradedb/paradedb/commit/44d1f928882599cf5fd9fbc853c8eee1fb5c57ed"
        },
        "date": 1770831179035,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Background Merger - Primary - background_merging",
            "value": 0,
            "unit": "median background_merging",
            "extra": "avg background_merging: 0.07791147552621304, max background_merging: 2.0, count: 56346"
          },
          {
            "name": "Background Merger - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.646470232827884, max cpu: 9.667674, count: 56346"
          },
          {
            "name": "Background Merger - Primary - mem",
            "value": 28.5234375,
            "unit": "median mem",
            "extra": "avg mem: 28.51460146904838, max mem: 28.52734375, count: 56346"
          },
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.908642899182279, max cpu: 9.657948, count: 56346"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 168.78515625,
            "unit": "median mem",
            "extra": "avg mem: 167.45242612330512, max mem: 169.01953125, count: 56346"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 50848,
            "unit": "median block_count",
            "extra": "avg block_count: 50708.33993539914, max block_count: 50848.0, count: 56346"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 45,
            "unit": "median segment_count",
            "extra": "avg segment_count: 42.02127923898768, max segment_count: 63.0, count: 56346"
          },
          {
            "name": "Single Insert - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.5972809636694105, max cpu: 9.476802, count: 56346"
          },
          {
            "name": "Single Insert - Primary - mem",
            "value": 125.7109375,
            "unit": "median mem",
            "extra": "avg mem: 115.06756302576936, max mem: 139.31640625, count: 56346"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.702193878703535, max cpu: 9.638554, count: 56346"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 168.45703125,
            "unit": "median mem",
            "extra": "avg mem: 164.2216451977514, max mem: 168.71875, count: 56346"
          },
          {
            "name": "Top N - Primary - cpu",
            "value": 23.323614,
            "unit": "median cpu",
            "extra": "avg cpu: 23.782388545519233, max cpu: 33.905144, count: 56346"
          },
          {
            "name": "Top N - Primary - mem",
            "value": 162.671875,
            "unit": "median mem",
            "extra": "avg mem: 181.69741758787225, max mem: 223.12109375, count: 56346"
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
          "id": "80daf35915e5edde9b7e091036a88ce3d6c6aea1",
          "message": "chore: Upgrade to `0.21.8` (#4168)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2026-02-12T09:45:33-05:00",
          "tree_id": "3de2b0c6e4f9a21b26370ecab28ddd8db57c65ff",
          "url": "https://github.com/paradedb/paradedb/commit/80daf35915e5edde9b7e091036a88ce3d6c6aea1"
        },
        "date": 1770910745944,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Background Merger - Primary - background_merging",
            "value": 0,
            "unit": "median background_merging",
            "extra": "avg background_merging: 0.08020426554520878, max background_merging: 2.0, count: 56593"
          },
          {
            "name": "Background Merger - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.662352375903439, max cpu: 9.523809, count: 56593"
          },
          {
            "name": "Background Merger - Primary - mem",
            "value": 26.52734375,
            "unit": "median mem",
            "extra": "avg mem: 26.52502310908593, max mem: 26.53125, count: 56593"
          },
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 4.669261,
            "unit": "median cpu",
            "extra": "avg cpu: 4.950744701861605, max cpu: 14.10382, count: 56593"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 168.72265625,
            "unit": "median mem",
            "extra": "avg mem: 167.32609342973072, max mem: 168.8671875, count: 56593"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 51443,
            "unit": "median block_count",
            "extra": "avg block_count: 51300.159843090136, max block_count: 51443.0, count: 56593"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 46,
            "unit": "median segment_count",
            "extra": "avg segment_count: 42.96476596045447, max segment_count: 56.0, count: 56593"
          },
          {
            "name": "Single Insert - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.639035283701152, max cpu: 9.542743, count: 56593"
          },
          {
            "name": "Single Insert - Primary - mem",
            "value": 126.64453125,
            "unit": "median mem",
            "extra": "avg mem: 115.92323800812379, max mem: 141.015625, count: 56593"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.773376775942392, max cpu: 9.523809, count: 56593"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 168.203125,
            "unit": "median mem",
            "extra": "avg mem: 164.06549092864842, max mem: 168.35546875, count: 56593"
          },
          {
            "name": "Top N - Primary - cpu",
            "value": 23.391813,
            "unit": "median cpu",
            "extra": "avg cpu: 23.862268728274316, max cpu: 33.3996, count: 56593"
          },
          {
            "name": "Top N - Primary - mem",
            "value": 162.37890625,
            "unit": "median mem",
            "extra": "avg mem: 182.24336524890447, max mem: 222.8125, count: 56593"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "2aec8447addadd4def1cf10f4d11e24c1755fadb",
          "message": "chore: Remove tuned_postgres from /benchmarks (#4167)\n\n# Ticket(s) Closed\n\n- Closes #N/A\n\n## What\nWe are moving user-facing benchmarks to a much more complete project,\nwhich will be in a separate repository. This is no longer relevant to\nkeep here, so removing.\n\n## Why\n^\n\n## How\n^\n\n## Tests\n^",
          "timestamp": "2026-02-12T12:21:38-05:00",
          "tree_id": "3502c5fdf7ad1b45110e95b70c181dd5ca1eae37",
          "url": "https://github.com/paradedb/paradedb/commit/2aec8447addadd4def1cf10f4d11e24c1755fadb"
        },
        "date": 1770919965535,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Background Merger - Primary - background_merging",
            "value": 0,
            "unit": "median background_merging",
            "extra": "avg background_merging: 0.08016728097533314, max background_merging: 2.0, count: 56432"
          },
          {
            "name": "Background Merger - Primary - cpu",
            "value": 4.669261,
            "unit": "median cpu",
            "extra": "avg cpu: 4.800765774546711, max cpu: 9.7165985, count: 56432"
          },
          {
            "name": "Background Merger - Primary - mem",
            "value": 26.46484375,
            "unit": "median mem",
            "extra": "avg mem: 26.461355037921745, max mem: 26.46875, count: 56432"
          },
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 4.669261,
            "unit": "median cpu",
            "extra": "avg cpu: 5.00158299900631, max cpu: 11.688313, count: 56432"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 168.5078125,
            "unit": "median mem",
            "extra": "avg mem: 167.1146043302116, max mem: 168.8671875, count: 56432"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 53559,
            "unit": "median block_count",
            "extra": "avg block_count: 53417.54401757868, max block_count: 53559.0, count: 56432"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 45,
            "unit": "median segment_count",
            "extra": "avg segment_count: 41.711422597108026, max segment_count: 58.0, count: 56432"
          },
          {
            "name": "Single Insert - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 4.584677325076258, max cpu: 9.514371, count: 56432"
          },
          {
            "name": "Single Insert - Primary - mem",
            "value": 121.44921875,
            "unit": "median mem",
            "extra": "avg mem: 111.22657938979657, max mem: 138.390625, count: 56432"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 4.748872544601074, max cpu: 9.571285, count: 56432"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 168.30859375,
            "unit": "median mem",
            "extra": "avg mem: 163.91098931844965, max mem: 168.44140625, count: 56432"
          },
          {
            "name": "Top N - Primary - cpu",
            "value": 23.391813,
            "unit": "median cpu",
            "extra": "avg cpu: 23.96239723028422, max cpu: 33.3996, count: 56432"
          },
          {
            "name": "Top N - Primary - mem",
            "value": 162.69140625,
            "unit": "median mem",
            "extra": "avg mem: 180.63344060661504, max mem: 223.12109375, count: 56432"
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
          "id": "ba868f34636e9fc6068c68b3b0d8a098eb4971d8",
          "message": "feat: join-scan: pre-materialization dynamic filter pushdown from TopK and HashJoin (#4161)\n\n## Ticket(s) Closed\n\n- Closes #4151\n\n## What\n\nDynamic filters from DataFusion's `SortExec(TopK)` and `HashJoinExec`\nare now pushed down into `PgSearchScan` and applied *before* column\nmaterialization — at the term-ordinal level for strings and the\nfast-field level for numerics. This avoids expensive term dictionary I/O\nfor documents that will be discarded anyway.\n\n## Why\n\nPreviously, `PgSearchScan` had no awareness of dynamic filters. Every\ndocument that passed the Tantivy query and visibility checks was fully\nmaterialized (all fast-field columns loaded, string dictionaries walked)\nbefore any join-key or TopK pruning could happen upstream. For selective\njoins or tight LIMIT queries, this meant loading data for rows that were\nimmediately thrown away by HashJoin or TopK.\n\n## How\n\n- Enabled DataFusion's TopK dynamic filter pushdown in the JoinScan\nsession config.\n- `SegmentPlan` now accepts dynamic filters from parent operators (TopK\nthresholds, HashJoin key bounds) and passes them to the Scanner on each\nbatch.\n- Before column materialization, the Scanner converts these filters to\nterm-ordinal comparisons (for strings) or direct fast-field comparisons\n(for numerics) and prunes non-matching documents in-place — skipping\ndictionary I/O entirely for pruned rows.\n\n## Tests\n\n- New `topk_dynamic_filter` regression test covering. You can take a\nlook at EXPLAIN ANALYZE diff in the follow-up PR (#4162):\nhttps://github.com/paradedb/paradedb/blob/3b074a9b5516a7a0a75a948201ef32e07b0127e4/pg_search/tests/pg_regress/expected/topk_dynamic_filter.out#L170-L181\n- All existing regression tests pass.",
          "timestamp": "2026-02-12T10:25:25-08:00",
          "tree_id": "748bfdacf0d0b82f9ceb26840b3100a7ca8e2252",
          "url": "https://github.com/paradedb/paradedb/commit/ba868f34636e9fc6068c68b3b0d8a098eb4971d8"
        },
        "date": 1770923771642,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Background Merger - Primary - background_merging",
            "value": 0,
            "unit": "median background_merging",
            "extra": "avg background_merging: 0.07919122346091031, max background_merging: 2.0, count: 56332"
          },
          {
            "name": "Background Merger - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.608495488649105, max cpu: 9.648242, count: 56332"
          },
          {
            "name": "Background Merger - Primary - mem",
            "value": 25.83203125,
            "unit": "median mem",
            "extra": "avg mem: 25.82079499107967, max mem: 25.8359375, count: 56332"
          },
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.93950513744535, max cpu: 14.007783, count: 56332"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 168.6171875,
            "unit": "median mem",
            "extra": "avg mem: 167.24393405113878, max mem: 168.84375, count: 56332"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 51857,
            "unit": "median block_count",
            "extra": "avg block_count: 51719.378115458356, max block_count: 51857.0, count: 56332"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 45,
            "unit": "median segment_count",
            "extra": "avg segment_count: 42.233810267698644, max segment_count: 62.0, count: 56332"
          },
          {
            "name": "Single Insert - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.560079663541668, max cpu: 9.338522, count: 56332"
          },
          {
            "name": "Single Insert - Primary - mem",
            "value": 123.640625,
            "unit": "median mem",
            "extra": "avg mem: 112.35047512681513, max mem: 138.08203125, count: 56332"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.715386375110886, max cpu: 9.628887, count: 56332"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 168.35546875,
            "unit": "median mem",
            "extra": "avg mem: 163.93654992666248, max mem: 168.5078125, count: 56332"
          },
          {
            "name": "Top N - Primary - cpu",
            "value": 23.391813,
            "unit": "median cpu",
            "extra": "avg cpu: 23.87712884262067, max cpu: 33.768845, count: 56332"
          },
          {
            "name": "Top N - Primary - mem",
            "value": 162.859375,
            "unit": "median mem",
            "extra": "avg mem: 181.23221634128913, max mem: 223.4296875, count: 56332"
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
          "id": "655ee8b04cab31c056cb41a89d677b896630ed16",
          "message": "feat: join-scan: surface dynamic filter metrics in EXPLAIN ANALYZE (#4162)\n\n# Ticket(s) Closed\n\n- Closes #4151\n\n## What\n\nUsed DataFusion metrics, and made dynamic filter pruning stats visible\nthrough `EXPLAIN ANALYZE`.\n\n## Why\n\n`EXPLAIN ANALYZE` is the natural place for execution-time stats.\n\n## How\n\n- Added `ExecutionPlanMetricsSet` to `SegmentPlan` with two custom\ncounters (`rows_scanned`, `rows_pruned`), only registered when dynamic\nfilters are present.\n\n## Tests\n\n- Updated `topk_dynamic_filter` regression test to use `EXPLAIN\n(ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)` — verifying\n`Dynamic Filter` lines appear with correct pruning stats (e.g., `30\nscanned, 24 pruned (80.0%)`).\n- Updated `join_custom_scan` and `filter_pushdown_datafusion` expected\noutput.\n\n---------\n\nSigned-off-by: Moe <mdashti@gmail.com>",
          "timestamp": "2026-02-12T12:45:44-08:00",
          "tree_id": "8c73104c0b40b30047e010ebfba45fb9add3f7e8",
          "url": "https://github.com/paradedb/paradedb/commit/655ee8b04cab31c056cb41a89d677b896630ed16"
        },
        "date": 1770932257481,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Background Merger - Primary - background_merging",
            "value": 0,
            "unit": "median background_merging",
            "extra": "avg background_merging: 0.08023369036027264, max background_merging: 2.0, count: 56485"
          },
          {
            "name": "Background Merger - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.687304643395738, max cpu: 9.628887, count: 56485"
          },
          {
            "name": "Background Merger - Primary - mem",
            "value": 27.921875,
            "unit": "median mem",
            "extra": "avg mem: 27.912848751327786, max mem: 27.92578125, count: 56485"
          },
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 5.019276490439829, max cpu: 14.201183, count: 56485"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 168.7578125,
            "unit": "median mem",
            "extra": "avg mem: 167.2628877550456, max mem: 168.86328125, count: 56485"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 50886,
            "unit": "median block_count",
            "extra": "avg block_count: 50744.36439762769, max block_count: 50886.0, count: 56485"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 45,
            "unit": "median segment_count",
            "extra": "avg segment_count: 41.710843586792954, max segment_count: 62.0, count: 56485"
          },
          {
            "name": "Single Insert - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.601814552061664, max cpu: 9.467456, count: 56485"
          },
          {
            "name": "Single Insert - Primary - mem",
            "value": 122.40625,
            "unit": "median mem",
            "extra": "avg mem: 113.23308490362486, max mem: 138.9140625, count: 56485"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.748226113110279, max cpu: 9.667674, count: 56485"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 168.1875,
            "unit": "median mem",
            "extra": "avg mem: 163.8641573122289, max mem: 168.3671875, count: 56485"
          },
          {
            "name": "Top N - Primary - cpu",
            "value": 23.323614,
            "unit": "median cpu",
            "extra": "avg cpu: 24.01678481454792, max cpu: 33.802814, count: 56485"
          },
          {
            "name": "Top N - Primary - mem",
            "value": 162.48046875,
            "unit": "median mem",
            "extra": "avg mem: 181.49421168230504, max mem: 222.8359375, count: 56485"
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
          "id": "1da48a637b80cc0ce2312ab8c4e448762e152223",
          "message": "perf: Add statistics to joinscan (#4132)\n\n## Ticket(s) Closed\n\n- Closes #4062.\n\n## What\n\n* Exposes sorting from the joinscan's `TableProvider`, but does not yet\nforce `SortMergeJoin`.\n* Adds statistics on `TableProvider` and our `ExecutionPlan`s using\nTantivy's query estimates.\n* Removes the `ParallelSegmentPlan` that was added in #4101, as it makes\nmore sense to let DataFusion coalesce for us if needed.\n\n## Why\n\nTo allow the DataFusion optimizer to re-order joins based on table\nsizes, and use sortedness in plans (although it does not yet by\ndefault).\n\n## Tests\n\nExisting tests show the impact of join reordering due to statistics.",
          "timestamp": "2026-02-12T14:34:01-08:00",
          "tree_id": "fbc185b154055782f4973f483feb5ad00a4ca2bb",
          "url": "https://github.com/paradedb/paradedb/commit/1da48a637b80cc0ce2312ab8c4e448762e152223"
        },
        "date": 1770938695981,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Background Merger - Primary - background_merging",
            "value": 0,
            "unit": "median background_merging",
            "extra": "avg background_merging: 0.081295734427985, max background_merging: 2.0, count: 56030"
          },
          {
            "name": "Background Merger - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.680869042038446, max cpu: 9.542743, count: 56030"
          },
          {
            "name": "Background Merger - Primary - mem",
            "value": 25.48046875,
            "unit": "median mem",
            "extra": "avg mem: 25.47026502264412, max mem: 25.484375, count: 56030"
          },
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 4.9639932963579545, max cpu: 9.846154, count: 56030"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 168.56640625,
            "unit": "median mem",
            "extra": "avg mem: 167.2358802622479, max mem: 168.8046875, count: 56030"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 53199,
            "unit": "median block_count",
            "extra": "avg block_count: 53060.58393717651, max block_count: 53199.0, count: 56030"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 45,
            "unit": "median segment_count",
            "extra": "avg segment_count: 41.991147599500266, max segment_count: 58.0, count: 56030"
          },
          {
            "name": "Single Insert - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.613101323746462, max cpu: 9.561753, count: 56030"
          },
          {
            "name": "Single Insert - Primary - mem",
            "value": 118.8125,
            "unit": "median mem",
            "extra": "avg mem: 110.36239354196412, max mem: 136.609375, count: 56030"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.723484329247225, max cpu: 9.514371, count: 56030"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 168.36328125,
            "unit": "median mem",
            "extra": "avg mem: 164.0456023140505, max mem: 168.49609375, count: 56030"
          },
          {
            "name": "Top N - Primary - cpu",
            "value": 23.346306,
            "unit": "median cpu",
            "extra": "avg cpu: 23.793309239872364, max cpu: 33.267326, count: 56030"
          },
          {
            "name": "Top N - Primary - mem",
            "value": 162.7421875,
            "unit": "median mem",
            "extra": "avg mem: 182.39990121084688, max mem: 223.1875, count: 56030"
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
        "date": 1770668997407,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 31.32550047352845,
            "unit": "median tps",
            "extra": "avg tps: 30.965527172575797, max tps: 35.37378108862219, count: 55466"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 241.31147827853363,
            "unit": "median tps",
            "extra": "avg tps: 262.76296303547474, max tps: 2633.5855963108024, count: 55466"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1981.5825598839979,
            "unit": "median tps",
            "extra": "avg tps: 1966.9935774967119, max tps: 2201.669777079911, count: 55466"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 160.81310139088492,
            "unit": "median tps",
            "extra": "avg tps: 197.1529620463513, max tps: 1703.1909979015247, count: 110932"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 14.5260597612265,
            "unit": "median tps",
            "extra": "avg tps: 14.239282236418822, max tps: 19.246776626837644, count: 55466"
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
          "id": "30c4c9cbf69783f13dbfa4ed26b331f441be585c",
          "message": "perf: Use parallel workers for the join scan (#4101)\n\n## Ticket(s) Closed\n\n- Closes #4063\n\n## What\n\nAdds support for parallel workers in the `joinscan`, by relying (for\nnow) on the fact that we only support INNER joins, and can thus do a\nbroadcast join.\n\n## Why\n\nTo get an implementation of parallel workers in place without (yet)\ntackling the problem of partitioning DataFusion plans across parallel\nworkers and introducing RPC.\n\n## How\n\n- Implemented a \"broadcast join\" strategy for `JoinScan` where the\nlargest index scan is partitioned across workers while the others are\nreplicated.\n- Introduced `ParallelSegmentPlan` and `ParallelScanStream` for dynamic\nworker-driven scanning.\n- This strategy is necessary in order to continue to use the lazy work\nclaiming strategy that we use in `ParallelScanState`, but after #4062\nthe replicated/un-partitioned indexes could begin using\n`MultiSegmentPlan` to provide sorted access.\n- In future, if/when we change our parallel worker partitioning\nstrategy, we might be able to use `MultiSegmentPlan` and assign _ranges_\nof an index to the parallel workers. TBD.\n- Centralized `RowEstimate` handling to better manage unanalyzed tables,\nand ease determining the largest index to scan.\n- Cleaned up registration of the `CustomScan`'s vtable\n(`CustomExecMethods`).\n- Before this, encountered some segfaults due to registration issues\naround having multiple parallel `CustomScan` implementations.\n- Remove \"lazy checkout\" from `MultiSegmentPlan`, as no consumer will\nactually use it lazily.\n\n## Tests\n\nExisting tests (and proptests) pass.\n\nBenchmarks show speedups across a few of our joins. Notably: we are\nfaster than Postgres for the `semi_join_filter` join for the first time.",
          "timestamp": "2026-02-09T17:53:18-08:00",
          "tree_id": "6616d18d10f8cf9e48caa5c264c26297828fd02b",
          "url": "https://github.com/paradedb/paradedb/commit/30c4c9cbf69783f13dbfa4ed26b331f441be585c"
        },
        "date": 1770692363343,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 31.519618368190162,
            "unit": "median tps",
            "extra": "avg tps: 31.391583100685033, max tps: 35.91969155411928, count: 55513"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 241.39378286819556,
            "unit": "median tps",
            "extra": "avg tps: 268.57088934725334, max tps: 2674.6101597815828, count: 55513"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1905.724468821617,
            "unit": "median tps",
            "extra": "avg tps: 1890.0603994689673, max tps: 2250.3609109045515, count: 55513"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 160.9649612401569,
            "unit": "median tps",
            "extra": "avg tps: 197.1169333587316, max tps: 1769.0928293205554, count: 111026"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 14.79737993378904,
            "unit": "median tps",
            "extra": "avg tps: 14.799080552474917, max tps: 21.324158599879627, count: 55513"
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
          "id": "ff10528e88ec12dd794b2a6e8b75996ad447a713",
          "message": "fix: Joinscan row estimation needs `ExprContext` (#4147)\n\n# Ticket(s) Closed\n\n- Closes #4146 \n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2026-02-10T14:16:35-05:00",
          "tree_id": "bba16dfdbd260ac92fb27be55e3b411b77039476",
          "url": "https://github.com/paradedb/paradedb/commit/ff10528e88ec12dd794b2a6e8b75996ad447a713"
        },
        "date": 1770754960410,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 33.29685342393448,
            "unit": "median tps",
            "extra": "avg tps: 32.88885134897058, max tps: 37.01910567585181, count: 55347"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 240.71461986769242,
            "unit": "median tps",
            "extra": "avg tps: 264.0096195932289, max tps: 2668.69270722863, count: 55347"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1911.1793327388216,
            "unit": "median tps",
            "extra": "avg tps: 1889.0450014343114, max tps: 2330.3656903055853, count: 55347"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 152.72343760674494,
            "unit": "median tps",
            "extra": "avg tps: 196.22891283406386, max tps: 1668.0442704509526, count: 110694"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 17.263878782002084,
            "unit": "median tps",
            "extra": "avg tps: 16.988464101511255, max tps: 20.58684319967058, count: 55347"
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
          "id": "084451f652ebc5c322fbf12b0189bc5e229dce3a",
          "message": "fix: reduce overhead for ngram match queries and add TEXT[] regression coverage (#4150)\n\n# Ticket(s) Closed\n\n- Closes #2884\n\n## What\n\nMinor optimization to `match_query` and new regression test covering\nngram search on TEXT[] columns with `conjunction_mode`.\n\n## Why\n\nA it's reported in #2884, slow ngram searches (~16 queries/s vs ~70\nwithout ngram) on a 350k-row TEXT[] column. We investigated and found\nthe N-way posting list intersection in `BooleanQuery` with many Must\nclauses is inherently expensive and can't be fundamentally improved at\nthe pg_search level. However, we identified two sources of unnecessary\noverhead in how `match_query` constructs the query.\n\n## How\n\n1. **`IndexRecordOption::WithFreqs` instead of `WithFreqsAndPositions`**\n— `match_query` creates `TermQuery` instances inside a `BooleanQuery`.\nThe BooleanQuery scorer only uses doc iteration and BM25 scores, never\npositions. `WithFreqsAndPositions` was requesting position data that was\nnever read. `WithFreqs` produces identical BM25 scores with less\nper-document overhead.\n\n2. **Deduplicate terms for conjunction mode** — For queries with\nrepeated ngram tokens (e.g., strings with repeated substrings),\nduplicate Must clauses add intersection work without changing which\ndocuments match. Dedup removes them before building the query.\n\nBoth changes preserve identical matching semantics and BM25 scoring.\n\n## Tests\n\nNew `ngram-text-array` regression test covering the exact pattern from\nthe reported issue: TEXT[] column with ICU + ngram alias fields, `match`\nwith `conjunction_mode`, `disjunction_max`, edge cases (short queries,\nsingle-token queries), and the JSON `::jsonb` query path.",
          "timestamp": "2026-02-10T15:11:24-08:00",
          "tree_id": "ce5fefd07b9871c52c5cd32b82b7f79613310334",
          "url": "https://github.com/paradedb/paradedb/commit/084451f652ebc5c322fbf12b0189bc5e229dce3a"
        },
        "date": 1770769001410,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 32.23858846638604,
            "unit": "median tps",
            "extra": "avg tps: 31.871386403133897, max tps: 34.49862254885562, count: 55479"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 237.8196901223786,
            "unit": "median tps",
            "extra": "avg tps: 258.52857274332564, max tps: 2632.931146690919, count: 55479"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1936.7431863910028,
            "unit": "median tps",
            "extra": "avg tps: 1911.7887534833155, max tps: 2316.520787729924, count: 55479"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 156.55740289621326,
            "unit": "median tps",
            "extra": "avg tps: 192.72260925180885, max tps: 1614.6535850241303, count: 110958"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 15.384882192091693,
            "unit": "median tps",
            "extra": "avg tps: 15.327145348866317, max tps: 21.114634369384103, count: 55479"
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
          "id": "59354e0de782d993f3e4a260eb7c56ad4804a1ad",
          "message": "fix: add field validation for `paradedb.aggregate()` API (#4141)\n\n# Ticket(s) Closed\n\n- Closes #N/A\n\n## What\n\nAdds field validation to the `paradedb.aggregate()` direct SQL function\nso that referencing a nonexistent or unsupported field returns a clear\nerror instead of silently producing null results.\n\n## Why\n\nThe `pdb.agg()` aggregate path already validates fields at plan time via\n`AggregateType::validate_fields()`. However, the `paradedb.aggregate()`\nfunction is a plain `pg_extern` that calls `execute_aggregate()`\ndirectly — it never enters the custom scan planner, so it skipped\nvalidation entirely. An invalid field like `\"nonexistent_field\"` would\nquietly return `{\"value\": null}` instead of telling the user something\nis wrong.\n\n## How\n\n- Extracted the field validation logic from\n`AggregateType::validate_fields()` into a standalone\n`validate_agg_json_fields()` function in `aggregate_type.rs`. The\nexisting `validate_fields()` now delegates to it for custom aggregates.\n- Called `validate_agg_json_fields()` in `aggregate_impl()`\n(`api/aggregate.rs`) before executing, so the direct API gets the same\nvalidation as the planner path.\n\n## Tests\n\n- Added regression tests (tests 13–15 in `agg-validate.sql`) covering\nthe `paradedb.aggregate()` path: valid field succeeds, invalid field\nerrors, invalid nested field errors.",
          "timestamp": "2026-02-10T15:12:54-08:00",
          "tree_id": "a2a30dc05294896dfaef747d15452a4024f5d8aa",
          "url": "https://github.com/paradedb/paradedb/commit/59354e0de782d993f3e4a260eb7c56ad4804a1ad"
        },
        "date": 1770769123932,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 31.39992804148552,
            "unit": "median tps",
            "extra": "avg tps: 31.141226987277292, max tps: 33.99900389718382, count: 55599"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 242.62137457928753,
            "unit": "median tps",
            "extra": "avg tps: 271.1112747475618, max tps: 2960.8369081184924, count: 55599"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1992.987612810321,
            "unit": "median tps",
            "extra": "avg tps: 1976.7718868734057, max tps: 2337.165965356797, count: 55599"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 160.78369608696386,
            "unit": "median tps",
            "extra": "avg tps: 199.42224204313393, max tps: 1739.9401617740236, count: 111198"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 14.737616869949598,
            "unit": "median tps",
            "extra": "avg tps: 14.713937098030309, max tps: 19.00768805859675, count: 55599"
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
          "id": "44d1f928882599cf5fd9fbc853c8eee1fb5c57ed",
          "message": "fix: Rebase against Tantivy, inherit upstream bugfix for intersection queries (#4155)\n\n# Ticket(s) Closed\n\n- Closes #4149 \n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2026-02-11T11:47:17-05:00",
          "tree_id": "b88ad04015b13dee26d44c4c9d585ea252d0323c",
          "url": "https://github.com/paradedb/paradedb/commit/44d1f928882599cf5fd9fbc853c8eee1fb5c57ed"
        },
        "date": 1770832066930,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 30.404849540440118,
            "unit": "median tps",
            "extra": "avg tps: 30.261968372305542, max tps: 32.97311493133814, count: 55452"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 244.87291021041693,
            "unit": "median tps",
            "extra": "avg tps: 271.61045837266835, max tps: 2755.3891249498365, count: 55452"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1955.021191536359,
            "unit": "median tps",
            "extra": "avg tps: 1947.5181148225684, max tps: 2267.5138016517426, count: 55452"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 162.09442672861252,
            "unit": "median tps",
            "extra": "avg tps: 198.4801738098219, max tps: 1733.365790805639, count: 110904"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 14.752444673796095,
            "unit": "median tps",
            "extra": "avg tps: 14.828188947642174, max tps: 19.49622267509348, count: 55452"
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
          "id": "80daf35915e5edde9b7e091036a88ce3d6c6aea1",
          "message": "chore: Upgrade to `0.21.8` (#4168)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2026-02-12T09:45:33-05:00",
          "tree_id": "3de2b0c6e4f9a21b26370ecab28ddd8db57c65ff",
          "url": "https://github.com/paradedb/paradedb/commit/80daf35915e5edde9b7e091036a88ce3d6c6aea1"
        },
        "date": 1770911669850,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 31.37212837379414,
            "unit": "median tps",
            "extra": "avg tps: 31.016882175818647, max tps: 35.16030641433513, count: 55530"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 242.05914523520875,
            "unit": "median tps",
            "extra": "avg tps: 267.655451237681, max tps: 2667.996513227958, count: 55530"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 2012.3362909737739,
            "unit": "median tps",
            "extra": "avg tps: 1997.0245327322514, max tps: 2282.4746064714204, count: 55530"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 175.56615294620372,
            "unit": "median tps",
            "extra": "avg tps: 206.19705000494437, max tps: 1793.854929158024, count: 111060"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 14.562226246354046,
            "unit": "median tps",
            "extra": "avg tps: 14.370029996888375, max tps: 16.422211375899014, count: 55530"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "2aec8447addadd4def1cf10f4d11e24c1755fadb",
          "message": "chore: Remove tuned_postgres from /benchmarks (#4167)\n\n# Ticket(s) Closed\n\n- Closes #N/A\n\n## What\nWe are moving user-facing benchmarks to a much more complete project,\nwhich will be in a separate repository. This is no longer relevant to\nkeep here, so removing.\n\n## Why\n^\n\n## How\n^\n\n## Tests\n^",
          "timestamp": "2026-02-12T12:21:38-05:00",
          "tree_id": "3502c5fdf7ad1b45110e95b70c181dd5ca1eae37",
          "url": "https://github.com/paradedb/paradedb/commit/2aec8447addadd4def1cf10f4d11e24c1755fadb"
        },
        "date": 1770920861608,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 31.26333836342651,
            "unit": "median tps",
            "extra": "avg tps: 31.1311465137564, max tps: 36.135917955935824, count: 55511"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 240.10856576843207,
            "unit": "median tps",
            "extra": "avg tps: 265.45846065048494, max tps: 2732.838608572186, count: 55511"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1987.748358177142,
            "unit": "median tps",
            "extra": "avg tps: 1967.386627297054, max tps: 2245.460240758247, count: 55511"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 165.720065809579,
            "unit": "median tps",
            "extra": "avg tps: 200.4179367831058, max tps: 1723.5527680423581, count: 111022"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 14.773608433392837,
            "unit": "median tps",
            "extra": "avg tps: 14.787119246251123, max tps: 21.002163600889837, count: 55511"
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
          "id": "ba868f34636e9fc6068c68b3b0d8a098eb4971d8",
          "message": "feat: join-scan: pre-materialization dynamic filter pushdown from TopK and HashJoin (#4161)\n\n## Ticket(s) Closed\n\n- Closes #4151\n\n## What\n\nDynamic filters from DataFusion's `SortExec(TopK)` and `HashJoinExec`\nare now pushed down into `PgSearchScan` and applied *before* column\nmaterialization — at the term-ordinal level for strings and the\nfast-field level for numerics. This avoids expensive term dictionary I/O\nfor documents that will be discarded anyway.\n\n## Why\n\nPreviously, `PgSearchScan` had no awareness of dynamic filters. Every\ndocument that passed the Tantivy query and visibility checks was fully\nmaterialized (all fast-field columns loaded, string dictionaries walked)\nbefore any join-key or TopK pruning could happen upstream. For selective\njoins or tight LIMIT queries, this meant loading data for rows that were\nimmediately thrown away by HashJoin or TopK.\n\n## How\n\n- Enabled DataFusion's TopK dynamic filter pushdown in the JoinScan\nsession config.\n- `SegmentPlan` now accepts dynamic filters from parent operators (TopK\nthresholds, HashJoin key bounds) and passes them to the Scanner on each\nbatch.\n- Before column materialization, the Scanner converts these filters to\nterm-ordinal comparisons (for strings) or direct fast-field comparisons\n(for numerics) and prunes non-matching documents in-place — skipping\ndictionary I/O entirely for pruned rows.\n\n## Tests\n\n- New `topk_dynamic_filter` regression test covering. You can take a\nlook at EXPLAIN ANALYZE diff in the follow-up PR (#4162):\nhttps://github.com/paradedb/paradedb/blob/3b074a9b5516a7a0a75a948201ef32e07b0127e4/pg_search/tests/pg_regress/expected/topk_dynamic_filter.out#L170-L181\n- All existing regression tests pass.",
          "timestamp": "2026-02-12T10:25:25-08:00",
          "tree_id": "748bfdacf0d0b82f9ceb26840b3100a7ca8e2252",
          "url": "https://github.com/paradedb/paradedb/commit/ba868f34636e9fc6068c68b3b0d8a098eb4971d8"
        },
        "date": 1770924660056,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 32.05876294140814,
            "unit": "median tps",
            "extra": "avg tps: 31.74344252545587, max tps: 33.91295901857893, count: 55521"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 235.10294783952747,
            "unit": "median tps",
            "extra": "avg tps: 255.2798137678419, max tps: 2628.9476935803727, count: 55521"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1832.2885000313067,
            "unit": "median tps",
            "extra": "avg tps: 1817.6092399461693, max tps: 2247.3509572369626, count: 55521"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 166.8010860705979,
            "unit": "median tps",
            "extra": "avg tps: 200.44967894623755, max tps: 1691.379675314951, count: 111042"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 15.120556484612647,
            "unit": "median tps",
            "extra": "avg tps: 15.147494115222631, max tps: 19.659269376720214, count: 55521"
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
          "id": "655ee8b04cab31c056cb41a89d677b896630ed16",
          "message": "feat: join-scan: surface dynamic filter metrics in EXPLAIN ANALYZE (#4162)\n\n# Ticket(s) Closed\n\n- Closes #4151\n\n## What\n\nUsed DataFusion metrics, and made dynamic filter pruning stats visible\nthrough `EXPLAIN ANALYZE`.\n\n## Why\n\n`EXPLAIN ANALYZE` is the natural place for execution-time stats.\n\n## How\n\n- Added `ExecutionPlanMetricsSet` to `SegmentPlan` with two custom\ncounters (`rows_scanned`, `rows_pruned`), only registered when dynamic\nfilters are present.\n\n## Tests\n\n- Updated `topk_dynamic_filter` regression test to use `EXPLAIN\n(ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)` — verifying\n`Dynamic Filter` lines appear with correct pruning stats (e.g., `30\nscanned, 24 pruned (80.0%)`).\n- Updated `join_custom_scan` and `filter_pushdown_datafusion` expected\noutput.\n\n---------\n\nSigned-off-by: Moe <mdashti@gmail.com>",
          "timestamp": "2026-02-12T12:45:44-08:00",
          "tree_id": "8c73104c0b40b30047e010ebfba45fb9add3f7e8",
          "url": "https://github.com/paradedb/paradedb/commit/655ee8b04cab31c056cb41a89d677b896630ed16"
        },
        "date": 1770933148612,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 31.032229574095123,
            "unit": "median tps",
            "extra": "avg tps: 30.908424209519435, max tps: 32.69058869329126, count: 55521"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 244.81551263761267,
            "unit": "median tps",
            "extra": "avg tps: 264.3553776298923, max tps: 2587.181178429406, count: 55521"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1916.882176645583,
            "unit": "median tps",
            "extra": "avg tps: 1905.6549610182462, max tps: 2256.0405632577367, count: 55521"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 165.53842325340537,
            "unit": "median tps",
            "extra": "avg tps: 199.9805781864525, max tps: 1630.7987103222947, count: 111042"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 14.887416055233148,
            "unit": "median tps",
            "extra": "avg tps: 14.998501013299439, max tps: 19.614406656156746, count: 55521"
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
          "id": "1da48a637b80cc0ce2312ab8c4e448762e152223",
          "message": "perf: Add statistics to joinscan (#4132)\n\n## Ticket(s) Closed\n\n- Closes #4062.\n\n## What\n\n* Exposes sorting from the joinscan's `TableProvider`, but does not yet\nforce `SortMergeJoin`.\n* Adds statistics on `TableProvider` and our `ExecutionPlan`s using\nTantivy's query estimates.\n* Removes the `ParallelSegmentPlan` that was added in #4101, as it makes\nmore sense to let DataFusion coalesce for us if needed.\n\n## Why\n\nTo allow the DataFusion optimizer to re-order joins based on table\nsizes, and use sortedness in plans (although it does not yet by\ndefault).\n\n## Tests\n\nExisting tests show the impact of join reordering due to statistics.",
          "timestamp": "2026-02-12T14:34:01-08:00",
          "tree_id": "fbc185b154055782f4973f483feb5ad00a4ca2bb",
          "url": "https://github.com/paradedb/paradedb/commit/1da48a637b80cc0ce2312ab8c4e448762e152223"
        },
        "date": 1770939593893,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 30.462407261115107,
            "unit": "median tps",
            "extra": "avg tps: 30.384792269866086, max tps: 33.75638205015806, count: 55482"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 244.62918126584617,
            "unit": "median tps",
            "extra": "avg tps: 266.76710715839124, max tps: 2740.4990921582917, count: 55482"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1955.2030761870913,
            "unit": "median tps",
            "extra": "avg tps: 1951.3219401222932, max tps: 2178.2093736911966, count: 55482"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 159.21159116027465,
            "unit": "median tps",
            "extra": "avg tps: 198.5432998356933, max tps: 1755.5527607155605, count: 110964"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 15.65875268762498,
            "unit": "median tps",
            "extra": "avg tps: 15.583723990984824, max tps: 18.465856119607338, count: 55482"
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
        "date": 1770669003425,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.568666,
            "unit": "median cpu",
            "extra": "avg cpu: 19.820309536233538, max cpu: 46.466602, count: 55466"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 174.40625,
            "unit": "median mem",
            "extra": "avg mem: 154.74090623929524, max mem: 176.234375, count: 55466"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 7.723379151126835, max cpu: 42.60355, count: 55466"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 120.05078125,
            "unit": "median mem",
            "extra": "avg mem: 118.8472919361636, max mem: 120.2109375, count: 55466"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.757180701235227, max cpu: 9.402546, count: 55466"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 126.21875,
            "unit": "median mem",
            "extra": "avg mem: 118.14312920865936, max mem: 158.921875, count: 55466"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 13592,
            "unit": "median block_count",
            "extra": "avg block_count: 13605.410503732017, max block_count: 23844.0, count: 55466"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.510747141471511, max cpu: 4.64666, count: 55466"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 98.2265625,
            "unit": "median mem",
            "extra": "avg mem: 92.60416361487218, max mem: 134.94140625, count: 55466"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 25,
            "unit": "median segment_count",
            "extra": "avg segment_count: 24.74625896945877, max segment_count: 38.0, count: 55466"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.230769,
            "unit": "median cpu",
            "extra": "avg cpu: 8.868715748260165, max cpu: 42.60355, count: 110932"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 162.1171875,
            "unit": "median mem",
            "extra": "avg mem: 141.36243536299264, max mem: 163.8984375, count: 110932"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.872832,
            "unit": "median cpu",
            "extra": "avg cpu: 12.774380953500971, max cpu: 27.906979, count: 55466"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 170.66796875,
            "unit": "median mem",
            "extra": "avg mem: 167.9917086734441, max mem: 171.45703125, count: 55466"
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
          "id": "30c4c9cbf69783f13dbfa4ed26b331f441be585c",
          "message": "perf: Use parallel workers for the join scan (#4101)\n\n## Ticket(s) Closed\n\n- Closes #4063\n\n## What\n\nAdds support for parallel workers in the `joinscan`, by relying (for\nnow) on the fact that we only support INNER joins, and can thus do a\nbroadcast join.\n\n## Why\n\nTo get an implementation of parallel workers in place without (yet)\ntackling the problem of partitioning DataFusion plans across parallel\nworkers and introducing RPC.\n\n## How\n\n- Implemented a \"broadcast join\" strategy for `JoinScan` where the\nlargest index scan is partitioned across workers while the others are\nreplicated.\n- Introduced `ParallelSegmentPlan` and `ParallelScanStream` for dynamic\nworker-driven scanning.\n- This strategy is necessary in order to continue to use the lazy work\nclaiming strategy that we use in `ParallelScanState`, but after #4062\nthe replicated/un-partitioned indexes could begin using\n`MultiSegmentPlan` to provide sorted access.\n- In future, if/when we change our parallel worker partitioning\nstrategy, we might be able to use `MultiSegmentPlan` and assign _ranges_\nof an index to the parallel workers. TBD.\n- Centralized `RowEstimate` handling to better manage unanalyzed tables,\nand ease determining the largest index to scan.\n- Cleaned up registration of the `CustomScan`'s vtable\n(`CustomExecMethods`).\n- Before this, encountered some segfaults due to registration issues\naround having multiple parallel `CustomScan` implementations.\n- Remove \"lazy checkout\" from `MultiSegmentPlan`, as no consumer will\nactually use it lazily.\n\n## Tests\n\nExisting tests (and proptests) pass.\n\nBenchmarks show speedups across a few of our joins. Notably: we are\nfaster than Postgres for the `semi_join_filter` join for the first time.",
          "timestamp": "2026-02-09T17:53:18-08:00",
          "tree_id": "6616d18d10f8cf9e48caa5c264c26297828fd02b",
          "url": "https://github.com/paradedb/paradedb/commit/30c4c9cbf69783f13dbfa4ed26b331f441be585c"
        },
        "date": 1770692368626,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.550726,
            "unit": "median cpu",
            "extra": "avg cpu: 19.72186106286642, max cpu: 51.714005, count: 55513"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 140.0859375,
            "unit": "median mem",
            "extra": "avg mem: 133.58768814568208, max mem: 172.703125, count: 55513"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 7.662024999718967, max cpu: 37.573387, count: 55513"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 119.66015625,
            "unit": "median mem",
            "extra": "avg mem: 118.5573407355034, max mem: 120.23046875, count: 55513"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.806567492192201, max cpu: 9.402546, count: 55513"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 123.23046875,
            "unit": "median mem",
            "extra": "avg mem: 115.22312714195324, max mem: 156.49609375, count: 55513"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 13817,
            "unit": "median block_count",
            "extra": "avg block_count: 13865.550843946463, max block_count: 24335.0, count: 55513"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.189458001504795, max cpu: 4.733728, count: 55513"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 96.390625,
            "unit": "median mem",
            "extra": "avg mem: 90.83322839357447, max mem: 132.5546875, count: 55513"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 24,
            "unit": "median segment_count",
            "extra": "avg segment_count: 23.636931889827608, max segment_count: 42.0, count: 55513"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.213051,
            "unit": "median cpu",
            "extra": "avg cpu: 8.731960666778042, max cpu: 37.573387, count: 111026"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 158.08984375,
            "unit": "median mem",
            "extra": "avg mem: 139.08169387829201, max mem: 164.1796875, count: 111026"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.859479,
            "unit": "median cpu",
            "extra": "avg cpu: 12.652193684742564, max cpu: 27.77242, count: 55513"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 170.5625,
            "unit": "median mem",
            "extra": "avg mem: 167.35252412723145, max mem: 171.66015625, count: 55513"
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
          "id": "ff10528e88ec12dd794b2a6e8b75996ad447a713",
          "message": "fix: Joinscan row estimation needs `ExprContext` (#4147)\n\n# Ticket(s) Closed\n\n- Closes #4146 \n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2026-02-10T14:16:35-05:00",
          "tree_id": "bba16dfdbd260ac92fb27be55e3b411b77039476",
          "url": "https://github.com/paradedb/paradedb/commit/ff10528e88ec12dd794b2a6e8b75996ad447a713"
        },
        "date": 1770754965237,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.514948,
            "unit": "median cpu",
            "extra": "avg cpu: 19.52859560587936, max cpu: 42.60355, count: 55347"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 142.1171875,
            "unit": "median mem",
            "extra": "avg mem: 133.92111869207002, max mem: 170.98046875, count: 55347"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 7.631619524099564, max cpu: 27.906979, count: 55347"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 119.49609375,
            "unit": "median mem",
            "extra": "avg mem: 118.2001239763447, max mem: 119.65234375, count: 55347"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.619827,
            "unit": "median cpu",
            "extra": "avg cpu: 4.731459039825182, max cpu: 9.448819, count: 55347"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 124.4453125,
            "unit": "median mem",
            "extra": "avg mem: 114.98747814922218, max mem: 153.61328125, count: 55347"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 13780,
            "unit": "median block_count",
            "extra": "avg block_count: 14029.045043091766, max block_count: 25009.0, count: 55347"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6153846,
            "unit": "median cpu",
            "extra": "avg cpu: 4.451969279614765, max cpu: 4.701273, count: 55347"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 96.84765625,
            "unit": "median mem",
            "extra": "avg mem: 91.12928722706289, max mem: 130.90625, count: 55347"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 23,
            "unit": "median segment_count",
            "extra": "avg segment_count: 23.47037779825465, max segment_count: 45.0, count: 55347"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.204219,
            "unit": "median cpu",
            "extra": "avg cpu: 9.050502843574131, max cpu: 28.402367, count: 110694"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 156.375,
            "unit": "median mem",
            "extra": "avg mem: 138.99087094828988, max mem: 163.84765625, count: 110694"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 11.302983,
            "unit": "median cpu",
            "extra": "avg cpu: 11.718772046135003, max cpu: 27.799229, count: 55347"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 170.88671875,
            "unit": "median mem",
            "extra": "avg mem: 167.5897293439572, max mem: 172.1484375, count: 55347"
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
          "id": "084451f652ebc5c322fbf12b0189bc5e229dce3a",
          "message": "fix: reduce overhead for ngram match queries and add TEXT[] regression coverage (#4150)\n\n# Ticket(s) Closed\n\n- Closes #2884\n\n## What\n\nMinor optimization to `match_query` and new regression test covering\nngram search on TEXT[] columns with `conjunction_mode`.\n\n## Why\n\nA it's reported in #2884, slow ngram searches (~16 queries/s vs ~70\nwithout ngram) on a 350k-row TEXT[] column. We investigated and found\nthe N-way posting list intersection in `BooleanQuery` with many Must\nclauses is inherently expensive and can't be fundamentally improved at\nthe pg_search level. However, we identified two sources of unnecessary\noverhead in how `match_query` constructs the query.\n\n## How\n\n1. **`IndexRecordOption::WithFreqs` instead of `WithFreqsAndPositions`**\n— `match_query` creates `TermQuery` instances inside a `BooleanQuery`.\nThe BooleanQuery scorer only uses doc iteration and BM25 scores, never\npositions. `WithFreqsAndPositions` was requesting position data that was\nnever read. `WithFreqs` produces identical BM25 scores with less\nper-document overhead.\n\n2. **Deduplicate terms for conjunction mode** — For queries with\nrepeated ngram tokens (e.g., strings with repeated substrings),\nduplicate Must clauses add intersection work without changing which\ndocuments match. Dedup removes them before building the query.\n\nBoth changes preserve identical matching semantics and BM25 scoring.\n\n## Tests\n\nNew `ngram-text-array` regression test covering the exact pattern from\nthe reported issue: TEXT[] column with ICU + ngram alias fields, `match`\nwith `conjunction_mode`, `disjunction_max`, edge cases (short queries,\nsingle-token queries), and the JSON `::jsonb` query path.",
          "timestamp": "2026-02-10T15:11:24-08:00",
          "tree_id": "ce5fefd07b9871c52c5cd32b82b7f79613310334",
          "url": "https://github.com/paradedb/paradedb/commit/084451f652ebc5c322fbf12b0189bc5e229dce3a"
        },
        "date": 1770769006061,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.532818,
            "unit": "median cpu",
            "extra": "avg cpu: 19.55581444858819, max cpu: 42.436146, count: 55479"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 140.59765625,
            "unit": "median mem",
            "extra": "avg mem: 135.13516354442672, max mem: 176.57421875, count: 55479"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 7.654299941189489, max cpu: 33.005894, count: 55479"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 120.22265625,
            "unit": "median mem",
            "extra": "avg mem: 118.94135238671389, max mem: 120.3125, count: 55479"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.819493002518741, max cpu: 13.93998, count: 55479"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 122.2734375,
            "unit": "median mem",
            "extra": "avg mem: 115.97202763376683, max mem: 159.55078125, count: 55479"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 13653,
            "unit": "median block_count",
            "extra": "avg block_count: 13741.56873772058, max block_count: 24104.0, count: 55479"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.131829635994176, max cpu: 4.7058825, count: 55479"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 95.7421875,
            "unit": "median mem",
            "extra": "avg mem: 91.23034173121812, max mem: 133.87890625, count: 55479"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 23,
            "unit": "median segment_count",
            "extra": "avg segment_count: 23.46500477658213, max segment_count: 40.0, count: 55479"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.213051,
            "unit": "median cpu",
            "extra": "avg cpu: 8.828444914629326, max cpu: 42.436146, count: 110958"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 161.1640625,
            "unit": "median mem",
            "extra": "avg mem: 140.12939809573217, max mem: 164.45703125, count: 110958"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.859479,
            "unit": "median cpu",
            "extra": "avg cpu: 12.532283604645068, max cpu: 27.77242, count: 55479"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 171.34375,
            "unit": "median mem",
            "extra": "avg mem: 168.22757872066458, max mem: 172.17578125, count: 55479"
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
          "id": "59354e0de782d993f3e4a260eb7c56ad4804a1ad",
          "message": "fix: add field validation for `paradedb.aggregate()` API (#4141)\n\n# Ticket(s) Closed\n\n- Closes #N/A\n\n## What\n\nAdds field validation to the `paradedb.aggregate()` direct SQL function\nso that referencing a nonexistent or unsupported field returns a clear\nerror instead of silently producing null results.\n\n## Why\n\nThe `pdb.agg()` aggregate path already validates fields at plan time via\n`AggregateType::validate_fields()`. However, the `paradedb.aggregate()`\nfunction is a plain `pg_extern` that calls `execute_aggregate()`\ndirectly — it never enters the custom scan planner, so it skipped\nvalidation entirely. An invalid field like `\"nonexistent_field\"` would\nquietly return `{\"value\": null}` instead of telling the user something\nis wrong.\n\n## How\n\n- Extracted the field validation logic from\n`AggregateType::validate_fields()` into a standalone\n`validate_agg_json_fields()` function in `aggregate_type.rs`. The\nexisting `validate_fields()` now delegates to it for custom aggregates.\n- Called `validate_agg_json_fields()` in `aggregate_impl()`\n(`api/aggregate.rs`) before executing, so the direct API gets the same\nvalidation as the planner path.\n\n## Tests\n\n- Added regression tests (tests 13–15 in `agg-validate.sql`) covering\nthe `paradedb.aggregate()` path: valid field succeeds, invalid field\nerrors, invalid nested field errors.",
          "timestamp": "2026-02-10T15:12:54-08:00",
          "tree_id": "a2a30dc05294896dfaef747d15452a4024f5d8aa",
          "url": "https://github.com/paradedb/paradedb/commit/59354e0de782d993f3e4a260eb7c56ad4804a1ad"
        },
        "date": 1770769128636,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.568666,
            "unit": "median cpu",
            "extra": "avg cpu: 19.74282027554478, max cpu: 52.071007, count: 55599"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 162.7734375,
            "unit": "median mem",
            "extra": "avg mem: 146.43788374678502, max mem: 176.84765625, count: 55599"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 7.6416882834139255, max cpu: 27.906979, count: 55599"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 119.40625,
            "unit": "median mem",
            "extra": "avg mem: 118.13811694567349, max mem: 119.5546875, count: 55599"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.788760154192649, max cpu: 13.806328, count: 55599"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 125.265625,
            "unit": "median mem",
            "extra": "avg mem: 115.78047990690031, max mem: 159.9765625, count: 55599"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 13535,
            "unit": "median block_count",
            "extra": "avg block_count: 13728.3754563931, max block_count: 24400.0, count: 55599"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.268180628278826, max cpu: 4.6966734, count: 55599"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 98.1796875,
            "unit": "median mem",
            "extra": "avg mem: 91.26775359325258, max mem: 134.56640625, count: 55599"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 24,
            "unit": "median segment_count",
            "extra": "avg segment_count: 24.485044695048472, max segment_count: 38.0, count: 55599"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.221902,
            "unit": "median cpu",
            "extra": "avg cpu: 8.801807852318095, max cpu: 27.934044, count: 111198"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 152.91796875,
            "unit": "median mem",
            "extra": "avg mem: 139.66190489037572, max mem: 164.15625, count: 111198"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.88621,
            "unit": "median cpu",
            "extra": "avg cpu: 13.40321571824946, max cpu: 27.934044, count: 55599"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 171.28515625,
            "unit": "median mem",
            "extra": "avg mem: 168.4005037747981, max mem: 172.41015625, count: 55599"
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
          "id": "44d1f928882599cf5fd9fbc853c8eee1fb5c57ed",
          "message": "fix: Rebase against Tantivy, inherit upstream bugfix for intersection queries (#4155)\n\n# Ticket(s) Closed\n\n- Closes #4149 \n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2026-02-11T11:47:17-05:00",
          "tree_id": "b88ad04015b13dee26d44c4c9d585ea252d0323c",
          "url": "https://github.com/paradedb/paradedb/commit/44d1f928882599cf5fd9fbc853c8eee1fb5c57ed"
        },
        "date": 1770832071723,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.550726,
            "unit": "median cpu",
            "extra": "avg cpu: 19.899691501506105, max cpu: 57.54246, count: 55452"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 176.74609375,
            "unit": "median mem",
            "extra": "avg mem: 174.01098247132654, max mem: 177.12109375, count: 55452"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 7.6851475749616585, max cpu: 37.684006, count: 55452"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 119.61328125,
            "unit": "median mem",
            "extra": "avg mem: 118.4656213087445, max mem: 119.71875, count: 55452"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.782595283789067, max cpu: 9.338522, count: 55452"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 122.5390625,
            "unit": "median mem",
            "extra": "avg mem: 113.70769511243958, max mem: 153.96484375, count: 55452"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 13615,
            "unit": "median block_count",
            "extra": "avg block_count: 13635.262713698334, max block_count: 24045.0, count: 55452"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6065254,
            "unit": "median cpu",
            "extra": "avg cpu: 4.029855242939196, max cpu: 4.64666, count: 55452"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 94.203125,
            "unit": "median mem",
            "extra": "avg mem: 88.9381242025761, max mem: 129.7109375, count: 55452"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 25,
            "unit": "median segment_count",
            "extra": "avg segment_count: 24.657325254273967, max segment_count: 41.0, count: 55452"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.213051,
            "unit": "median cpu",
            "extra": "avg cpu: 8.730381742302912, max cpu: 37.684006, count: 110904"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 154.84375,
            "unit": "median mem",
            "extra": "avg mem: 138.72152122415557, max mem: 164.15625, count: 110904"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.859479,
            "unit": "median cpu",
            "extra": "avg cpu: 13.141413865494417, max cpu: 28.20764, count: 55452"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 171.83203125,
            "unit": "median mem",
            "extra": "avg mem: 168.04449583085372, max mem: 172.98828125, count: 55452"
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
          "id": "80daf35915e5edde9b7e091036a88ce3d6c6aea1",
          "message": "chore: Upgrade to `0.21.8` (#4168)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2026-02-12T09:45:33-05:00",
          "tree_id": "3de2b0c6e4f9a21b26370ecab28ddd8db57c65ff",
          "url": "https://github.com/paradedb/paradedb/commit/80daf35915e5edde9b7e091036a88ce3d6c6aea1"
        },
        "date": 1770911674685,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.532818,
            "unit": "median cpu",
            "extra": "avg cpu: 19.710249321880802, max cpu: 42.519684, count: 55530"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 173.98046875,
            "unit": "median mem",
            "extra": "avg mem: 161.36149132507202, max mem: 177.12109375, count: 55530"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 7.703595285227953, max cpu: 32.40116, count: 55530"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 121.56640625,
            "unit": "median mem",
            "extra": "avg mem: 120.25567134319287, max mem: 121.66796875, count: 55530"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.804684237132907, max cpu: 13.899614, count: 55530"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 126.9609375,
            "unit": "median mem",
            "extra": "avg mem: 118.42746713488205, max mem: 159.9921875, count: 55530"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 13867,
            "unit": "median block_count",
            "extra": "avg block_count: 13924.83884386818, max block_count: 24465.0, count: 55530"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.406794631465588, max cpu: 4.673807, count: 55530"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 99.8828125,
            "unit": "median mem",
            "extra": "avg mem: 93.23022127678732, max mem: 135.078125, count: 55530"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 24,
            "unit": "median segment_count",
            "extra": "avg segment_count: 23.628110931028274, max segment_count: 47.0, count: 55530"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.195402,
            "unit": "median cpu",
            "extra": "avg cpu: 8.424417191350546, max cpu: 41.65863, count: 111060"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 163.28125,
            "unit": "median mem",
            "extra": "avg mem: 141.95074206791375, max mem: 164.91015625, count: 111060"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.859479,
            "unit": "median cpu",
            "extra": "avg cpu: 13.239096202919065, max cpu: 27.799229, count: 55530"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 172.109375,
            "unit": "median mem",
            "extra": "avg mem: 169.0079684545516, max mem: 172.8515625, count: 55530"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "2aec8447addadd4def1cf10f4d11e24c1755fadb",
          "message": "chore: Remove tuned_postgres from /benchmarks (#4167)\n\n# Ticket(s) Closed\n\n- Closes #N/A\n\n## What\nWe are moving user-facing benchmarks to a much more complete project,\nwhich will be in a separate repository. This is no longer relevant to\nkeep here, so removing.\n\n## Why\n^\n\n## How\n^\n\n## Tests\n^",
          "timestamp": "2026-02-12T12:21:38-05:00",
          "tree_id": "3502c5fdf7ad1b45110e95b70c181dd5ca1eae37",
          "url": "https://github.com/paradedb/paradedb/commit/2aec8447addadd4def1cf10f4d11e24c1755fadb"
        },
        "date": 1770920866442,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.60465,
            "unit": "median cpu",
            "extra": "avg cpu: 19.662848689978837, max cpu: 42.519684, count: 55511"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 168.8125,
            "unit": "median mem",
            "extra": "avg mem: 156.5304826971231, max mem: 172.4609375, count: 55511"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 7.702959777974767, max cpu: 28.042841, count: 55511"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 120.6484375,
            "unit": "median mem",
            "extra": "avg mem: 119.41365133430762, max mem: 120.77734375, count: 55511"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.750164774867125, max cpu: 9.430255, count: 55511"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 119.83203125,
            "unit": "median mem",
            "extra": "avg mem: 111.86896340083497, max mem: 151.87109375, count: 55511"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 13581,
            "unit": "median block_count",
            "extra": "avg block_count: 13696.392949145215, max block_count: 24309.0, count: 55511"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.58891,
            "unit": "median cpu",
            "extra": "avg cpu: 2.4360184007596613, max cpu: 4.743083, count: 55511"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 94.734375,
            "unit": "median mem",
            "extra": "avg mem: 89.2389242109672, max mem: 129.3125, count: 55511"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 24,
            "unit": "median segment_count",
            "extra": "avg segment_count: 23.60222298283223, max segment_count: 36.0, count: 55511"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.239654,
            "unit": "median cpu",
            "extra": "avg cpu: 8.75403870739724, max cpu: 28.374382, count: 111022"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 155.96484375,
            "unit": "median mem",
            "extra": "avg mem: 138.11757783648736, max mem: 164.1015625, count: 111022"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.899614,
            "unit": "median cpu",
            "extra": "avg cpu: 13.33913198048037, max cpu: 27.961164, count: 55511"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 171.7578125,
            "unit": "median mem",
            "extra": "avg mem: 168.6513882102196, max mem: 172.6640625, count: 55511"
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
          "id": "ba868f34636e9fc6068c68b3b0d8a098eb4971d8",
          "message": "feat: join-scan: pre-materialization dynamic filter pushdown from TopK and HashJoin (#4161)\n\n## Ticket(s) Closed\n\n- Closes #4151\n\n## What\n\nDynamic filters from DataFusion's `SortExec(TopK)` and `HashJoinExec`\nare now pushed down into `PgSearchScan` and applied *before* column\nmaterialization — at the term-ordinal level for strings and the\nfast-field level for numerics. This avoids expensive term dictionary I/O\nfor documents that will be discarded anyway.\n\n## Why\n\nPreviously, `PgSearchScan` had no awareness of dynamic filters. Every\ndocument that passed the Tantivy query and visibility checks was fully\nmaterialized (all fast-field columns loaded, string dictionaries walked)\nbefore any join-key or TopK pruning could happen upstream. For selective\njoins or tight LIMIT queries, this meant loading data for rows that were\nimmediately thrown away by HashJoin or TopK.\n\n## How\n\n- Enabled DataFusion's TopK dynamic filter pushdown in the JoinScan\nsession config.\n- `SegmentPlan` now accepts dynamic filters from parent operators (TopK\nthresholds, HashJoin key bounds) and passes them to the Scanner on each\nbatch.\n- Before column materialization, the Scanner converts these filters to\nterm-ordinal comparisons (for strings) or direct fast-field comparisons\n(for numerics) and prunes non-matching documents in-place — skipping\ndictionary I/O entirely for pruned rows.\n\n## Tests\n\n- New `topk_dynamic_filter` regression test covering. You can take a\nlook at EXPLAIN ANALYZE diff in the follow-up PR (#4162):\nhttps://github.com/paradedb/paradedb/blob/3b074a9b5516a7a0a75a948201ef32e07b0127e4/pg_search/tests/pg_regress/expected/topk_dynamic_filter.out#L170-L181\n- All existing regression tests pass.",
          "timestamp": "2026-02-12T10:25:25-08:00",
          "tree_id": "748bfdacf0d0b82f9ceb26840b3100a7ca8e2252",
          "url": "https://github.com/paradedb/paradedb/commit/ba868f34636e9fc6068c68b3b0d8a098eb4971d8"
        },
        "date": 1770924664999,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.622696,
            "unit": "median cpu",
            "extra": "avg cpu: 19.756257226112375, max cpu: 47.105007, count: 55521"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 176.59765625,
            "unit": "median mem",
            "extra": "avg mem: 160.72141537661426, max mem: 176.84765625, count: 55521"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 7.695928019428969, max cpu: 28.042841, count: 55521"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 119.5625,
            "unit": "median mem",
            "extra": "avg mem: 118.53537449512346, max mem: 119.82421875, count: 55521"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.773012358539643, max cpu: 9.458128, count: 55521"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 128.88671875,
            "unit": "median mem",
            "extra": "avg mem: 118.94957791871543, max mem: 159.5703125, count: 55521"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 13505,
            "unit": "median block_count",
            "extra": "avg block_count: 13826.34878694548, max block_count: 24861.0, count: 55521"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.509820035280814, max cpu: 4.692082, count: 55521"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 100.2734375,
            "unit": "median mem",
            "extra": "avg mem: 93.94157749714073, max mem: 136.2109375, count: 55521"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 24,
            "unit": "median segment_count",
            "extra": "avg segment_count: 23.58006880279534, max segment_count: 41.0, count: 55521"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.213051,
            "unit": "median cpu",
            "extra": "avg cpu: 8.609023266817363, max cpu: 28.09756, count: 111042"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 163.15234375,
            "unit": "median mem",
            "extra": "avg mem: 141.50570729994055, max mem: 164.4140625, count: 111042"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.93998,
            "unit": "median cpu",
            "extra": "avg cpu: 14.0833036050464, max cpu: 28.09756, count: 55521"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 170.26953125,
            "unit": "median mem",
            "extra": "avg mem: 167.44190085451902, max mem: 171.46484375, count: 55521"
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
          "id": "655ee8b04cab31c056cb41a89d677b896630ed16",
          "message": "feat: join-scan: surface dynamic filter metrics in EXPLAIN ANALYZE (#4162)\n\n# Ticket(s) Closed\n\n- Closes #4151\n\n## What\n\nUsed DataFusion metrics, and made dynamic filter pruning stats visible\nthrough `EXPLAIN ANALYZE`.\n\n## Why\n\n`EXPLAIN ANALYZE` is the natural place for execution-time stats.\n\n## How\n\n- Added `ExecutionPlanMetricsSet` to `SegmentPlan` with two custom\ncounters (`rows_scanned`, `rows_pruned`), only registered when dynamic\nfilters are present.\n\n## Tests\n\n- Updated `topk_dynamic_filter` regression test to use `EXPLAIN\n(ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)` — verifying\n`Dynamic Filter` lines appear with correct pruning stats (e.g., `30\nscanned, 24 pruned (80.0%)`).\n- Updated `join_custom_scan` and `filter_pushdown_datafusion` expected\noutput.\n\n---------\n\nSigned-off-by: Moe <mdashti@gmail.com>",
          "timestamp": "2026-02-12T12:45:44-08:00",
          "tree_id": "8c73104c0b40b30047e010ebfba45fb9add3f7e8",
          "url": "https://github.com/paradedb/paradedb/commit/655ee8b04cab31c056cb41a89d677b896630ed16"
        },
        "date": 1770933153479,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.58664,
            "unit": "median cpu",
            "extra": "avg cpu: 19.754563744546058, max cpu: 47.105007, count: 55521"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 176.53515625,
            "unit": "median mem",
            "extra": "avg mem: 173.99745296261776, max mem: 176.7421875, count: 55521"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 7.671683553869707, max cpu: 32.55814, count: 55521"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 119.3671875,
            "unit": "median mem",
            "extra": "avg mem: 118.13385187305254, max mem: 119.4921875, count: 55521"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.807300762575229, max cpu: 14.0214205, count: 55521"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 119.66796875,
            "unit": "median mem",
            "extra": "avg mem: 114.47276023655463, max mem: 158.41796875, count: 55521"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 13583,
            "unit": "median block_count",
            "extra": "avg block_count: 13561.015633724177, max block_count: 23498.0, count: 55521"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 3.655513551019012, max cpu: 4.669261, count: 55521"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 93.9765625,
            "unit": "median mem",
            "extra": "avg mem: 90.43003892952666, max mem: 133.97265625, count: 55521"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 24,
            "unit": "median segment_count",
            "extra": "avg segment_count: 23.717007978962915, max segment_count: 39.0, count: 55521"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.221902,
            "unit": "median cpu",
            "extra": "avg cpu: 8.634308593222343, max cpu: 37.2093, count: 111042"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 159.2578125,
            "unit": "median mem",
            "extra": "avg mem: 139.25133508042003, max mem: 164.2578125, count: 111042"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.88621,
            "unit": "median cpu",
            "extra": "avg cpu: 12.9587559300628, max cpu: 27.826086, count: 55521"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 170.0703125,
            "unit": "median mem",
            "extra": "avg mem: 167.1376537847166, max mem: 171.07421875, count: 55521"
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
          "id": "1da48a637b80cc0ce2312ab8c4e448762e152223",
          "message": "perf: Add statistics to joinscan (#4132)\n\n## Ticket(s) Closed\n\n- Closes #4062.\n\n## What\n\n* Exposes sorting from the joinscan's `TableProvider`, but does not yet\nforce `SortMergeJoin`.\n* Adds statistics on `TableProvider` and our `ExecutionPlan`s using\nTantivy's query estimates.\n* Removes the `ParallelSegmentPlan` that was added in #4101, as it makes\nmore sense to let DataFusion coalesce for us if needed.\n\n## Why\n\nTo allow the DataFusion optimizer to re-order joins based on table\nsizes, and use sortedness in plans (although it does not yet by\ndefault).\n\n## Tests\n\nExisting tests show the impact of join reordering due to statistics.",
          "timestamp": "2026-02-12T14:34:01-08:00",
          "tree_id": "fbc185b154055782f4973f483feb5ad00a4ca2bb",
          "url": "https://github.com/paradedb/paradedb/commit/1da48a637b80cc0ce2312ab8c4e448762e152223"
        },
        "date": 1770939598727,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.58664,
            "unit": "median cpu",
            "extra": "avg cpu: 19.981931446396818, max cpu: 46.55674, count: 55482"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 176.55078125,
            "unit": "median mem",
            "extra": "avg mem: 174.26875263317382, max mem: 176.8828125, count: 55482"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 7.688765756090463, max cpu: 27.988338, count: 55482"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 119.8203125,
            "unit": "median mem",
            "extra": "avg mem: 118.55889656498955, max mem: 119.8828125, count: 55482"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.813556410590541, max cpu: 9.347614, count: 55482"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 118.859375,
            "unit": "median mem",
            "extra": "avg mem: 113.9042087270421, max mem: 158.609375, count: 55482"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 13719,
            "unit": "median block_count",
            "extra": "avg block_count: 13829.898615767275, max block_count: 24498.0, count: 55482"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.329612191131423, max cpu: 4.660194, count: 55482"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 92.90234375,
            "unit": "median mem",
            "extra": "avg mem: 89.64081312407627, max mem: 131.890625, count: 55482"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 24,
            "unit": "median segment_count",
            "extra": "avg segment_count: 23.87691503550701, max segment_count: 37.0, count: 55482"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.230769,
            "unit": "median cpu",
            "extra": "avg cpu: 8.981255744228262, max cpu: 28.374382, count: 110964"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 160.26953125,
            "unit": "median mem",
            "extra": "avg mem: 139.1740037168586, max mem: 164.15625, count: 110964"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.859479,
            "unit": "median cpu",
            "extra": "avg cpu: 12.238673808777286, max cpu: 23.323614, count: 55482"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 170.5859375,
            "unit": "median mem",
            "extra": "avg mem: 167.58823885167712, max mem: 171.92578125, count: 55482"
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
        "date": 1770669876595,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - tps",
            "value": 581.4416096999739,
            "unit": "median tps",
            "extra": "avg tps: 583.6498258461738, max tps: 669.4461112855656, count: 53920"
          },
          {
            "name": "Index Only Scan - Subscriber - tps",
            "value": 651.270067873859,
            "unit": "median tps",
            "extra": "avg tps: 655.1806731203897, max tps: 819.5111397515079, count: 53920"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - tps",
            "value": 86.18761042586127,
            "unit": "median tps",
            "extra": "avg tps: 86.20198122827064, max tps: 91.41853315123981, count: 53920"
          },
          {
            "name": "Top N - Subscriber - tps",
            "value": 534.4481799886908,
            "unit": "median tps",
            "extra": "avg tps: 502.79368507567875, max tps: 711.6868348831133, count: 107840"
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
          "id": "30c4c9cbf69783f13dbfa4ed26b331f441be585c",
          "message": "perf: Use parallel workers for the join scan (#4101)\n\n## Ticket(s) Closed\n\n- Closes #4063\n\n## What\n\nAdds support for parallel workers in the `joinscan`, by relying (for\nnow) on the fact that we only support INNER joins, and can thus do a\nbroadcast join.\n\n## Why\n\nTo get an implementation of parallel workers in place without (yet)\ntackling the problem of partitioning DataFusion plans across parallel\nworkers and introducing RPC.\n\n## How\n\n- Implemented a \"broadcast join\" strategy for `JoinScan` where the\nlargest index scan is partitioned across workers while the others are\nreplicated.\n- Introduced `ParallelSegmentPlan` and `ParallelScanStream` for dynamic\nworker-driven scanning.\n- This strategy is necessary in order to continue to use the lazy work\nclaiming strategy that we use in `ParallelScanState`, but after #4062\nthe replicated/un-partitioned indexes could begin using\n`MultiSegmentPlan` to provide sorted access.\n- In future, if/when we change our parallel worker partitioning\nstrategy, we might be able to use `MultiSegmentPlan` and assign _ranges_\nof an index to the parallel workers. TBD.\n- Centralized `RowEstimate` handling to better manage unanalyzed tables,\nand ease determining the largest index to scan.\n- Cleaned up registration of the `CustomScan`'s vtable\n(`CustomExecMethods`).\n- Before this, encountered some segfaults due to registration issues\naround having multiple parallel `CustomScan` implementations.\n- Remove \"lazy checkout\" from `MultiSegmentPlan`, as no consumer will\nactually use it lazily.\n\n## Tests\n\nExisting tests (and proptests) pass.\n\nBenchmarks show speedups across a few of our joins. Notably: we are\nfaster than Postgres for the `semi_join_filter` join for the first time.",
          "timestamp": "2026-02-09T17:53:18-08:00",
          "tree_id": "6616d18d10f8cf9e48caa5c264c26297828fd02b",
          "url": "https://github.com/paradedb/paradedb/commit/30c4c9cbf69783f13dbfa4ed26b331f441be585c"
        },
        "date": 1770693243613,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - tps",
            "value": 581.2243011903114,
            "unit": "median tps",
            "extra": "avg tps: 582.4657053427549, max tps: 741.06161408578, count: 53888"
          },
          {
            "name": "Index Only Scan - Subscriber - tps",
            "value": 636.7825048532006,
            "unit": "median tps",
            "extra": "avg tps: 639.659278978035, max tps: 866.9791901874911, count: 53888"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - tps",
            "value": 86.47825254974342,
            "unit": "median tps",
            "extra": "avg tps: 86.49516931763202, max tps: 94.05065896325486, count: 53888"
          },
          {
            "name": "Top N - Subscriber - tps",
            "value": 524.5919516178276,
            "unit": "median tps",
            "extra": "avg tps: 489.30204644567925, max tps: 695.1079001098171, count: 107776"
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
          "id": "ff10528e88ec12dd794b2a6e8b75996ad447a713",
          "message": "fix: Joinscan row estimation needs `ExprContext` (#4147)\n\n# Ticket(s) Closed\n\n- Closes #4146 \n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2026-02-10T14:16:35-05:00",
          "tree_id": "bba16dfdbd260ac92fb27be55e3b411b77039476",
          "url": "https://github.com/paradedb/paradedb/commit/ff10528e88ec12dd794b2a6e8b75996ad447a713"
        },
        "date": 1770755863029,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - tps",
            "value": 564.574902004518,
            "unit": "median tps",
            "extra": "avg tps: 565.4496270254331, max tps: 715.9758051530605, count: 53820"
          },
          {
            "name": "Index Only Scan - Subscriber - tps",
            "value": 607.8646281985295,
            "unit": "median tps",
            "extra": "avg tps: 611.5421120845169, max tps: 796.0441413656393, count: 53820"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - tps",
            "value": 88.64257155176334,
            "unit": "median tps",
            "extra": "avg tps: 88.53919660991542, max tps: 92.01446017896018, count: 53820"
          },
          {
            "name": "Top N - Subscriber - tps",
            "value": 555.3277800222002,
            "unit": "median tps",
            "extra": "avg tps: 492.54884170897407, max tps: 731.7853619295252, count: 107640"
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
          "id": "084451f652ebc5c322fbf12b0189bc5e229dce3a",
          "message": "fix: reduce overhead for ngram match queries and add TEXT[] regression coverage (#4150)\n\n# Ticket(s) Closed\n\n- Closes #2884\n\n## What\n\nMinor optimization to `match_query` and new regression test covering\nngram search on TEXT[] columns with `conjunction_mode`.\n\n## Why\n\nA it's reported in #2884, slow ngram searches (~16 queries/s vs ~70\nwithout ngram) on a 350k-row TEXT[] column. We investigated and found\nthe N-way posting list intersection in `BooleanQuery` with many Must\nclauses is inherently expensive and can't be fundamentally improved at\nthe pg_search level. However, we identified two sources of unnecessary\noverhead in how `match_query` constructs the query.\n\n## How\n\n1. **`IndexRecordOption::WithFreqs` instead of `WithFreqsAndPositions`**\n— `match_query` creates `TermQuery` instances inside a `BooleanQuery`.\nThe BooleanQuery scorer only uses doc iteration and BM25 scores, never\npositions. `WithFreqsAndPositions` was requesting position data that was\nnever read. `WithFreqs` produces identical BM25 scores with less\nper-document overhead.\n\n2. **Deduplicate terms for conjunction mode** — For queries with\nrepeated ngram tokens (e.g., strings with repeated substrings),\nduplicate Must clauses add intersection work without changing which\ndocuments match. Dedup removes them before building the query.\n\nBoth changes preserve identical matching semantics and BM25 scoring.\n\n## Tests\n\nNew `ngram-text-array` regression test covering the exact pattern from\nthe reported issue: TEXT[] column with ICU + ngram alias fields, `match`\nwith `conjunction_mode`, `disjunction_max`, edge cases (short queries,\nsingle-token queries), and the JSON `::jsonb` query path.",
          "timestamp": "2026-02-10T15:11:24-08:00",
          "tree_id": "ce5fefd07b9871c52c5cd32b82b7f79613310334",
          "url": "https://github.com/paradedb/paradedb/commit/084451f652ebc5c322fbf12b0189bc5e229dce3a"
        },
        "date": 1770769888251,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - tps",
            "value": 634.382585192046,
            "unit": "median tps",
            "extra": "avg tps: 633.3584727594874, max tps: 730.0217306337001, count: 53834"
          },
          {
            "name": "Index Only Scan - Subscriber - tps",
            "value": 692.5625237109159,
            "unit": "median tps",
            "extra": "avg tps: 694.6201342016622, max tps: 841.0484282623643, count: 53834"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - tps",
            "value": 90.54241996834016,
            "unit": "median tps",
            "extra": "avg tps: 90.51092070028638, max tps: 92.09280356705423, count: 53834"
          },
          {
            "name": "Top N - Subscriber - tps",
            "value": 499.41845306559674,
            "unit": "median tps",
            "extra": "avg tps: 461.53405870709287, max tps: 705.5485466586914, count: 107668"
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
          "id": "59354e0de782d993f3e4a260eb7c56ad4804a1ad",
          "message": "fix: add field validation for `paradedb.aggregate()` API (#4141)\n\n# Ticket(s) Closed\n\n- Closes #N/A\n\n## What\n\nAdds field validation to the `paradedb.aggregate()` direct SQL function\nso that referencing a nonexistent or unsupported field returns a clear\nerror instead of silently producing null results.\n\n## Why\n\nThe `pdb.agg()` aggregate path already validates fields at plan time via\n`AggregateType::validate_fields()`. However, the `paradedb.aggregate()`\nfunction is a plain `pg_extern` that calls `execute_aggregate()`\ndirectly — it never enters the custom scan planner, so it skipped\nvalidation entirely. An invalid field like `\"nonexistent_field\"` would\nquietly return `{\"value\": null}` instead of telling the user something\nis wrong.\n\n## How\n\n- Extracted the field validation logic from\n`AggregateType::validate_fields()` into a standalone\n`validate_agg_json_fields()` function in `aggregate_type.rs`. The\nexisting `validate_fields()` now delegates to it for custom aggregates.\n- Called `validate_agg_json_fields()` in `aggregate_impl()`\n(`api/aggregate.rs`) before executing, so the direct API gets the same\nvalidation as the planner path.\n\n## Tests\n\n- Added regression tests (tests 13–15 in `agg-validate.sql`) covering\nthe `paradedb.aggregate()` path: valid field succeeds, invalid field\nerrors, invalid nested field errors.",
          "timestamp": "2026-02-10T15:12:54-08:00",
          "tree_id": "a2a30dc05294896dfaef747d15452a4024f5d8aa",
          "url": "https://github.com/paradedb/paradedb/commit/59354e0de782d993f3e4a260eb7c56ad4804a1ad"
        },
        "date": 1770770022658,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - tps",
            "value": 605.2634126590865,
            "unit": "median tps",
            "extra": "avg tps: 606.6950661153709, max tps: 712.0222869811239, count: 53871"
          },
          {
            "name": "Index Only Scan - Subscriber - tps",
            "value": 671.3516092971279,
            "unit": "median tps",
            "extra": "avg tps: 672.8135425192027, max tps: 855.7817691537016, count: 53871"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - tps",
            "value": 84.17511266725795,
            "unit": "median tps",
            "extra": "avg tps: 84.22569549006897, max tps: 85.73210408033455, count: 53871"
          },
          {
            "name": "Top N - Subscriber - tps",
            "value": 570.6105747638405,
            "unit": "median tps",
            "extra": "avg tps: 512.3676310586563, max tps: 715.7971639916606, count: 107742"
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
          "id": "44d1f928882599cf5fd9fbc853c8eee1fb5c57ed",
          "message": "fix: Rebase against Tantivy, inherit upstream bugfix for intersection queries (#4155)\n\n# Ticket(s) Closed\n\n- Closes #4149 \n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2026-02-11T11:47:17-05:00",
          "tree_id": "b88ad04015b13dee26d44c4c9d585ea252d0323c",
          "url": "https://github.com/paradedb/paradedb/commit/44d1f928882599cf5fd9fbc853c8eee1fb5c57ed"
        },
        "date": 1770832962657,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - tps",
            "value": 577.2938743384723,
            "unit": "median tps",
            "extra": "avg tps: 581.4478235675998, max tps: 751.3493946146743, count: 53890"
          },
          {
            "name": "Index Only Scan - Subscriber - tps",
            "value": 615.981318507171,
            "unit": "median tps",
            "extra": "avg tps: 623.8089452765196, max tps: 855.653984285548, count: 53890"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - tps",
            "value": 83.59479146501404,
            "unit": "median tps",
            "extra": "avg tps: 83.79869165168371, max tps: 92.84956122910421, count: 53890"
          },
          {
            "name": "Top N - Subscriber - tps",
            "value": 543.2024408588065,
            "unit": "median tps",
            "extra": "avg tps: 487.86591102311644, max tps: 728.3732462752495, count: 107780"
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
          "id": "80daf35915e5edde9b7e091036a88ce3d6c6aea1",
          "message": "chore: Upgrade to `0.21.8` (#4168)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2026-02-12T09:45:33-05:00",
          "tree_id": "3de2b0c6e4f9a21b26370ecab28ddd8db57c65ff",
          "url": "https://github.com/paradedb/paradedb/commit/80daf35915e5edde9b7e091036a88ce3d6c6aea1"
        },
        "date": 1770912603271,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - tps",
            "value": 578.0319433012037,
            "unit": "median tps",
            "extra": "avg tps: 580.0062879786643, max tps: 714.2710126549088, count: 53879"
          },
          {
            "name": "Index Only Scan - Subscriber - tps",
            "value": 647.6179666487996,
            "unit": "median tps",
            "extra": "avg tps: 649.1221990802333, max tps: 833.537614664245, count: 53879"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - tps",
            "value": 84.09683220679074,
            "unit": "median tps",
            "extra": "avg tps: 84.21522141310075, max tps: 90.35198176294296, count: 53879"
          },
          {
            "name": "Top N - Subscriber - tps",
            "value": 550.8440191985388,
            "unit": "median tps",
            "extra": "avg tps: 497.89019841346254, max tps: 708.218008644088, count: 107758"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "2aec8447addadd4def1cf10f4d11e24c1755fadb",
          "message": "chore: Remove tuned_postgres from /benchmarks (#4167)\n\n# Ticket(s) Closed\n\n- Closes #N/A\n\n## What\nWe are moving user-facing benchmarks to a much more complete project,\nwhich will be in a separate repository. This is no longer relevant to\nkeep here, so removing.\n\n## Why\n^\n\n## How\n^\n\n## Tests\n^",
          "timestamp": "2026-02-12T12:21:38-05:00",
          "tree_id": "3502c5fdf7ad1b45110e95b70c181dd5ca1eae37",
          "url": "https://github.com/paradedb/paradedb/commit/2aec8447addadd4def1cf10f4d11e24c1755fadb"
        },
        "date": 1770921796157,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - tps",
            "value": 570.9453542239963,
            "unit": "median tps",
            "extra": "avg tps: 575.509830091533, max tps: 695.0363354570633, count: 53861"
          },
          {
            "name": "Index Only Scan - Subscriber - tps",
            "value": 652.2509146452974,
            "unit": "median tps",
            "extra": "avg tps: 659.7466993655285, max tps: 802.9763902916759, count: 53861"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - tps",
            "value": 83.66044499978611,
            "unit": "median tps",
            "extra": "avg tps: 83.84180603189853, max tps: 91.39933845408098, count: 53861"
          },
          {
            "name": "Top N - Subscriber - tps",
            "value": 561.35488535509,
            "unit": "median tps",
            "extra": "avg tps: 503.15518807616843, max tps: 750.610949058973, count: 107722"
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
          "id": "ba868f34636e9fc6068c68b3b0d8a098eb4971d8",
          "message": "feat: join-scan: pre-materialization dynamic filter pushdown from TopK and HashJoin (#4161)\n\n## Ticket(s) Closed\n\n- Closes #4151\n\n## What\n\nDynamic filters from DataFusion's `SortExec(TopK)` and `HashJoinExec`\nare now pushed down into `PgSearchScan` and applied *before* column\nmaterialization — at the term-ordinal level for strings and the\nfast-field level for numerics. This avoids expensive term dictionary I/O\nfor documents that will be discarded anyway.\n\n## Why\n\nPreviously, `PgSearchScan` had no awareness of dynamic filters. Every\ndocument that passed the Tantivy query and visibility checks was fully\nmaterialized (all fast-field columns loaded, string dictionaries walked)\nbefore any join-key or TopK pruning could happen upstream. For selective\njoins or tight LIMIT queries, this meant loading data for rows that were\nimmediately thrown away by HashJoin or TopK.\n\n## How\n\n- Enabled DataFusion's TopK dynamic filter pushdown in the JoinScan\nsession config.\n- `SegmentPlan` now accepts dynamic filters from parent operators (TopK\nthresholds, HashJoin key bounds) and passes them to the Scanner on each\nbatch.\n- Before column materialization, the Scanner converts these filters to\nterm-ordinal comparisons (for strings) or direct fast-field comparisons\n(for numerics) and prunes non-matching documents in-place — skipping\ndictionary I/O entirely for pruned rows.\n\n## Tests\n\n- New `topk_dynamic_filter` regression test covering. You can take a\nlook at EXPLAIN ANALYZE diff in the follow-up PR (#4162):\nhttps://github.com/paradedb/paradedb/blob/3b074a9b5516a7a0a75a948201ef32e07b0127e4/pg_search/tests/pg_regress/expected/topk_dynamic_filter.out#L170-L181\n- All existing regression tests pass.",
          "timestamp": "2026-02-12T10:25:25-08:00",
          "tree_id": "748bfdacf0d0b82f9ceb26840b3100a7ca8e2252",
          "url": "https://github.com/paradedb/paradedb/commit/ba868f34636e9fc6068c68b3b0d8a098eb4971d8"
        },
        "date": 1770925592636,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - tps",
            "value": 591.4588779426449,
            "unit": "median tps",
            "extra": "avg tps: 595.1788994249215, max tps: 715.8582328323664, count: 53886"
          },
          {
            "name": "Index Only Scan - Subscriber - tps",
            "value": 666.6281810409769,
            "unit": "median tps",
            "extra": "avg tps: 672.8605787192458, max tps: 798.4713688363764, count: 53886"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - tps",
            "value": 84.85608794442248,
            "unit": "median tps",
            "extra": "avg tps: 84.99973700520086, max tps: 90.12570153470551, count: 53886"
          },
          {
            "name": "Top N - Subscriber - tps",
            "value": 547.5937145711243,
            "unit": "median tps",
            "extra": "avg tps: 488.54029334483715, max tps: 705.3415357761766, count: 107772"
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
          "id": "655ee8b04cab31c056cb41a89d677b896630ed16",
          "message": "feat: join-scan: surface dynamic filter metrics in EXPLAIN ANALYZE (#4162)\n\n# Ticket(s) Closed\n\n- Closes #4151\n\n## What\n\nUsed DataFusion metrics, and made dynamic filter pruning stats visible\nthrough `EXPLAIN ANALYZE`.\n\n## Why\n\n`EXPLAIN ANALYZE` is the natural place for execution-time stats.\n\n## How\n\n- Added `ExecutionPlanMetricsSet` to `SegmentPlan` with two custom\ncounters (`rows_scanned`, `rows_pruned`), only registered when dynamic\nfilters are present.\n\n## Tests\n\n- Updated `topk_dynamic_filter` regression test to use `EXPLAIN\n(ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)` — verifying\n`Dynamic Filter` lines appear with correct pruning stats (e.g., `30\nscanned, 24 pruned (80.0%)`).\n- Updated `join_custom_scan` and `filter_pushdown_datafusion` expected\noutput.\n\n---------\n\nSigned-off-by: Moe <mdashti@gmail.com>",
          "timestamp": "2026-02-12T12:45:44-08:00",
          "tree_id": "8c73104c0b40b30047e010ebfba45fb9add3f7e8",
          "url": "https://github.com/paradedb/paradedb/commit/655ee8b04cab31c056cb41a89d677b896630ed16"
        },
        "date": 1770934055375,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - tps",
            "value": 572.5516043769965,
            "unit": "median tps",
            "extra": "avg tps: 573.6313270288473, max tps: 726.1698612969354, count: 53879"
          },
          {
            "name": "Index Only Scan - Subscriber - tps",
            "value": 640.8914553287352,
            "unit": "median tps",
            "extra": "avg tps: 642.862002537645, max tps: 788.3013556260752, count: 53879"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - tps",
            "value": 84.13235413408405,
            "unit": "median tps",
            "extra": "avg tps: 84.21297531033564, max tps: 91.94764437473242, count: 53879"
          },
          {
            "name": "Top N - Subscriber - tps",
            "value": 545.0311005576893,
            "unit": "median tps",
            "extra": "avg tps: 507.80751831075577, max tps: 714.1328166185665, count: 107758"
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
          "id": "1da48a637b80cc0ce2312ab8c4e448762e152223",
          "message": "perf: Add statistics to joinscan (#4132)\n\n## Ticket(s) Closed\n\n- Closes #4062.\n\n## What\n\n* Exposes sorting from the joinscan's `TableProvider`, but does not yet\nforce `SortMergeJoin`.\n* Adds statistics on `TableProvider` and our `ExecutionPlan`s using\nTantivy's query estimates.\n* Removes the `ParallelSegmentPlan` that was added in #4101, as it makes\nmore sense to let DataFusion coalesce for us if needed.\n\n## Why\n\nTo allow the DataFusion optimizer to re-order joins based on table\nsizes, and use sortedness in plans (although it does not yet by\ndefault).\n\n## Tests\n\nExisting tests show the impact of join reordering due to statistics.",
          "timestamp": "2026-02-12T14:34:01-08:00",
          "tree_id": "fbc185b154055782f4973f483feb5ad00a4ca2bb",
          "url": "https://github.com/paradedb/paradedb/commit/1da48a637b80cc0ce2312ab8c4e448762e152223"
        },
        "date": 1770940496717,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - tps",
            "value": 606.9007682807625,
            "unit": "median tps",
            "extra": "avg tps: 603.0709957385341, max tps: 741.7562093290678, count: 53855"
          },
          {
            "name": "Index Only Scan - Subscriber - tps",
            "value": 668.9518624945719,
            "unit": "median tps",
            "extra": "avg tps: 663.3465502951036, max tps: 806.1926653051992, count: 53855"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - tps",
            "value": 85.96887004218542,
            "unit": "median tps",
            "extra": "avg tps: 85.92674924359544, max tps: 92.88520718761102, count: 53855"
          },
          {
            "name": "Top N - Subscriber - tps",
            "value": 547.1190327698434,
            "unit": "median tps",
            "extra": "avg tps: 500.4641738627781, max tps: 673.9678072182484, count: 107710"
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
        "date": 1770669881940,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - cpu",
            "value": 4.567079,
            "unit": "median cpu",
            "extra": "avg cpu: 5.000822767810848, max cpu: 9.248554, count: 53920"
          },
          {
            "name": "Custom Scan - Subscriber - mem",
            "value": 49.8984375,
            "unit": "median mem",
            "extra": "avg mem: 49.96232497218101, max mem: 55.80859375, count: 53920"
          },
          {
            "name": "Delete values - Publisher - cpu",
            "value": 4.567079,
            "unit": "median cpu",
            "extra": "avg cpu: 4.492686893290857, max cpu: 4.6021094, count: 53920"
          },
          {
            "name": "Delete values - Publisher - mem",
            "value": 31.8984375,
            "unit": "median mem",
            "extra": "avg mem: 31.174773028908568, max mem: 32.23828125, count: 53920"
          },
          {
            "name": "Find by ctid - Subscriber - cpu",
            "value": 9.090909,
            "unit": "median cpu",
            "extra": "avg cpu: 7.72870896124984, max cpu: 18.268316, count: 53920"
          },
          {
            "name": "Find by ctid - Subscriber - mem",
            "value": 53.73046875,
            "unit": "median mem",
            "extra": "avg mem: 53.44485920982474, max mem: 59.6015625, count: 53920"
          },
          {
            "name": "Index Only Scan - Subscriber - cpu",
            "value": 4.567079,
            "unit": "median cpu",
            "extra": "avg cpu: 4.913626193832668, max cpu: 9.239654, count: 53920"
          },
          {
            "name": "Index Only Scan - Subscriber - mem",
            "value": 49.7734375,
            "unit": "median mem",
            "extra": "avg mem: 49.856114961980715, max mem: 55.65625, count: 53920"
          },
          {
            "name": "Index Size Info - Subscriber - cpu",
            "value": 4.567079,
            "unit": "median cpu",
            "extra": "avg cpu: 4.6489665380611545, max cpu: 9.230769, count: 53920"
          },
          {
            "name": "Index Size Info - Subscriber - mem",
            "value": 33.66796875,
            "unit": "median mem",
            "extra": "avg mem: 33.70501632916821, max mem: 38.74609375, count: 53920"
          },
          {
            "name": "Index Size Info - Subscriber - pages",
            "value": 1098,
            "unit": "median pages",
            "extra": "avg pages: 1102.5926928783383, max pages: 1831.0, count: 53920"
          },
          {
            "name": "Index Size Info - Subscriber - relation_size:MB",
            "value": 8.578125,
            "unit": "median relation_size:MB",
            "extra": "avg relation_size:MB: 8.614005558002596, max relation_size:MB: 14.3046875, count: 53920"
          },
          {
            "name": "Index Size Info - Subscriber - segment_count",
            "value": 9,
            "unit": "median segment_count",
            "extra": "avg segment_count: 9.036387240356083, max segment_count: 16.0, count: 53920"
          },
          {
            "name": "Insert value A - Publisher - cpu",
            "value": 4.562738,
            "unit": "median cpu",
            "extra": "avg cpu: 4.0147872157832305, max cpu: 4.5933013, count: 53920"
          },
          {
            "name": "Insert value A - Publisher - mem",
            "value": 29.59765625,
            "unit": "median mem",
            "extra": "avg mem: 28.874158620409865, max mem: 29.94921875, count: 53920"
          },
          {
            "name": "Insert value B - Publisher - cpu",
            "value": 4.5540795,
            "unit": "median cpu",
            "extra": "avg cpu: 4.326812220067055, max cpu: 4.5714283, count: 53920"
          },
          {
            "name": "Insert value B - Publisher - mem",
            "value": 29.72265625,
            "unit": "median mem",
            "extra": "avg mem: 29.015301604228487, max mem: 30.08203125, count: 53920"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - cpu",
            "value": 4.597701,
            "unit": "median cpu",
            "extra": "avg cpu: 6.8427825052796525, max cpu: 22.922636, count: 53920"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - mem",
            "value": 47.818359375,
            "unit": "median mem",
            "extra": "avg mem: 47.884332836841615, max mem: 53.6953125, count: 53920"
          },
          {
            "name": "SELECT\n  pid,\n  pg_wal_lsn_diff(sent_lsn, replay_lsn) AS replication_lag,\n  application_name::text,\n  state::text\nFROM pg_stat_replication; - Publisher - replication_lag:MB",
            "value": 0,
            "unit": "median replication_lag:MB",
            "extra": "avg replication_lag:MB: 0.00002842529591183988, max replication_lag:MB: 0.20058441162109375, count: 53920"
          },
          {
            "name": "Top N - Subscriber - cpu",
            "value": 4.567079,
            "unit": "median cpu",
            "extra": "avg cpu: 5.073411682935185, max cpu: 13.779904, count: 107840"
          },
          {
            "name": "Top N - Subscriber - mem",
            "value": 48.4453125,
            "unit": "median mem",
            "extra": "avg mem: 48.49160714078728, max mem: 54.5859375, count: 107840"
          },
          {
            "name": "Update 1..9 - Publisher - cpu",
            "value": 4.567079,
            "unit": "median cpu",
            "extra": "avg cpu: 4.385082796991468, max cpu: 4.597701, count: 53920"
          },
          {
            "name": "Update 1..9 - Publisher - mem",
            "value": 32.5625,
            "unit": "median mem",
            "extra": "avg mem: 31.865175404534497, max mem: 32.890625, count: 53920"
          },
          {
            "name": "Update 10,11 - Publisher - cpu",
            "value": 4.5411544,
            "unit": "median cpu",
            "extra": "avg cpu: 4.395930655341971, max cpu: 4.597701, count: 53920"
          },
          {
            "name": "Update 10,11 - Publisher - mem",
            "value": 32.55078125,
            "unit": "median mem",
            "extra": "avg mem: 31.885622580327336, max mem: 32.640625, count: 53920"
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
          "id": "30c4c9cbf69783f13dbfa4ed26b331f441be585c",
          "message": "perf: Use parallel workers for the join scan (#4101)\n\n## Ticket(s) Closed\n\n- Closes #4063\n\n## What\n\nAdds support for parallel workers in the `joinscan`, by relying (for\nnow) on the fact that we only support INNER joins, and can thus do a\nbroadcast join.\n\n## Why\n\nTo get an implementation of parallel workers in place without (yet)\ntackling the problem of partitioning DataFusion plans across parallel\nworkers and introducing RPC.\n\n## How\n\n- Implemented a \"broadcast join\" strategy for `JoinScan` where the\nlargest index scan is partitioned across workers while the others are\nreplicated.\n- Introduced `ParallelSegmentPlan` and `ParallelScanStream` for dynamic\nworker-driven scanning.\n- This strategy is necessary in order to continue to use the lazy work\nclaiming strategy that we use in `ParallelScanState`, but after #4062\nthe replicated/un-partitioned indexes could begin using\n`MultiSegmentPlan` to provide sorted access.\n- In future, if/when we change our parallel worker partitioning\nstrategy, we might be able to use `MultiSegmentPlan` and assign _ranges_\nof an index to the parallel workers. TBD.\n- Centralized `RowEstimate` handling to better manage unanalyzed tables,\nand ease determining the largest index to scan.\n- Cleaned up registration of the `CustomScan`'s vtable\n(`CustomExecMethods`).\n- Before this, encountered some segfaults due to registration issues\naround having multiple parallel `CustomScan` implementations.\n- Remove \"lazy checkout\" from `MultiSegmentPlan`, as no consumer will\nactually use it lazily.\n\n## Tests\n\nExisting tests (and proptests) pass.\n\nBenchmarks show speedups across a few of our joins. Notably: we are\nfaster than Postgres for the `semi_join_filter` join for the first time.",
          "timestamp": "2026-02-09T17:53:18-08:00",
          "tree_id": "6616d18d10f8cf9e48caa5c264c26297828fd02b",
          "url": "https://github.com/paradedb/paradedb/commit/30c4c9cbf69783f13dbfa4ed26b331f441be585c"
        },
        "date": 1770693248863,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - cpu",
            "value": 4.5801525,
            "unit": "median cpu",
            "extra": "avg cpu: 4.943079131033911, max cpu: 13.688212, count: 53888"
          },
          {
            "name": "Custom Scan - Subscriber - mem",
            "value": 50.640625,
            "unit": "median mem",
            "extra": "avg mem: 50.65258013437593, max mem: 56.56640625, count: 53888"
          },
          {
            "name": "Delete values - Publisher - cpu",
            "value": 4.562738,
            "unit": "median cpu",
            "extra": "avg cpu: 3.282839790137006, max cpu: 4.6021094, count: 53888"
          },
          {
            "name": "Delete values - Publisher - mem",
            "value": 32.140625,
            "unit": "median mem",
            "extra": "avg mem: 31.435062798071925, max mem: 32.78515625, count: 53888"
          },
          {
            "name": "Find by ctid - Subscriber - cpu",
            "value": 9.142857,
            "unit": "median cpu",
            "extra": "avg cpu: 8.350099745624785, max cpu: 18.514948, count: 53888"
          },
          {
            "name": "Find by ctid - Subscriber - mem",
            "value": 54.1015625,
            "unit": "median mem",
            "extra": "avg mem: 53.76302984604643, max mem: 60.05859375, count: 53888"
          },
          {
            "name": "Index Only Scan - Subscriber - cpu",
            "value": 4.5801525,
            "unit": "median cpu",
            "extra": "avg cpu: 4.938166451927254, max cpu: 9.257474, count: 53888"
          },
          {
            "name": "Index Only Scan - Subscriber - mem",
            "value": 50.7578125,
            "unit": "median mem",
            "extra": "avg mem: 50.78728798619359, max mem: 56.7421875, count: 53888"
          },
          {
            "name": "Index Size Info - Subscriber - cpu",
            "value": 4.5801525,
            "unit": "median cpu",
            "extra": "avg cpu: 4.623628401557984, max cpu: 9.195402, count: 53888"
          },
          {
            "name": "Index Size Info - Subscriber - mem",
            "value": 33.29296875,
            "unit": "median mem",
            "extra": "avg mem: 33.31188225462997, max mem: 38.51171875, count: 53888"
          },
          {
            "name": "Index Size Info - Subscriber - pages",
            "value": 1115,
            "unit": "median pages",
            "extra": "avg pages: 1112.635317695962, max pages: 1852.0, count: 53888"
          },
          {
            "name": "Index Size Info - Subscriber - relation_size:MB",
            "value": 8.7109375,
            "unit": "median relation_size:MB",
            "extra": "avg relation_size:MB: 8.692463419499703, max relation_size:MB: 14.46875, count: 53888"
          },
          {
            "name": "Index Size Info - Subscriber - segment_count",
            "value": 11,
            "unit": "median segment_count",
            "extra": "avg segment_count: 11.264604364608076, max segment_count: 19.0, count: 53888"
          },
          {
            "name": "Insert value A - Publisher - cpu",
            "value": 4.5845275,
            "unit": "median cpu",
            "extra": "avg cpu: 4.148338766514286, max cpu: 4.6021094, count: 53888"
          },
          {
            "name": "Insert value A - Publisher - mem",
            "value": 29.41015625,
            "unit": "median mem",
            "extra": "avg mem: 28.76083033328385, max mem: 29.83984375, count: 53888"
          },
          {
            "name": "Insert value B - Publisher - cpu",
            "value": 4.5714283,
            "unit": "median cpu",
            "extra": "avg cpu: 4.3653066567829235, max cpu: 4.5933013, count: 53888"
          },
          {
            "name": "Insert value B - Publisher - mem",
            "value": 29.37890625,
            "unit": "median mem",
            "extra": "avg mem: 28.746336658323745, max mem: 29.86328125, count: 53888"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - cpu",
            "value": 4.610951,
            "unit": "median cpu",
            "extra": "avg cpu: 6.840948146298463, max cpu: 18.33811, count: 53888"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - mem",
            "value": 48.3359375,
            "unit": "median mem",
            "extra": "avg mem: 48.34943728775423, max mem: 54.3515625, count: 53888"
          },
          {
            "name": "SELECT\n  pid,\n  pg_wal_lsn_diff(sent_lsn, replay_lsn) AS replication_lag,\n  application_name::text,\n  state::text\nFROM pg_stat_replication; - Publisher - replication_lag:MB",
            "value": 0,
            "unit": "median replication_lag:MB",
            "extra": "avg replication_lag:MB: 0.000024418083335894587, max replication_lag:MB: 0.08852386474609375, count: 53888"
          },
          {
            "name": "Top N - Subscriber - cpu",
            "value": 4.5845275,
            "unit": "median cpu",
            "extra": "avg cpu: 5.0946401236272, max cpu: 13.832853, count: 107776"
          },
          {
            "name": "Top N - Subscriber - mem",
            "value": 48.78125,
            "unit": "median mem",
            "extra": "avg mem: 48.856136331082524, max mem: 55.09765625, count: 107776"
          },
          {
            "name": "Update 1..9 - Publisher - cpu",
            "value": 4.5411544,
            "unit": "median cpu",
            "extra": "avg cpu: 4.322419264950276, max cpu: 4.610951, count: 53888"
          },
          {
            "name": "Update 1..9 - Publisher - mem",
            "value": 32.703125,
            "unit": "median mem",
            "extra": "avg mem: 31.885049561706687, max mem: 32.703125, count: 53888"
          },
          {
            "name": "Update 10,11 - Publisher - cpu",
            "value": 4.5714283,
            "unit": "median cpu",
            "extra": "avg cpu: 4.4317913470225605, max cpu: 4.628737, count: 53888"
          },
          {
            "name": "Update 10,11 - Publisher - mem",
            "value": 32.82421875,
            "unit": "median mem",
            "extra": "avg mem: 32.14209172844603, max mem: 33.26953125, count: 53888"
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
          "id": "ff10528e88ec12dd794b2a6e8b75996ad447a713",
          "message": "fix: Joinscan row estimation needs `ExprContext` (#4147)\n\n# Ticket(s) Closed\n\n- Closes #4146 \n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2026-02-10T14:16:35-05:00",
          "tree_id": "bba16dfdbd260ac92fb27be55e3b411b77039476",
          "url": "https://github.com/paradedb/paradedb/commit/ff10528e88ec12dd794b2a6e8b75996ad447a713"
        },
        "date": 1770755867689,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - cpu",
            "value": 4.5801525,
            "unit": "median cpu",
            "extra": "avg cpu: 4.933998700950531, max cpu: 9.4395275, count: 53820"
          },
          {
            "name": "Custom Scan - Subscriber - mem",
            "value": 50.484375,
            "unit": "median mem",
            "extra": "avg mem: 50.396242756526384, max mem: 56.1328125, count: 53820"
          },
          {
            "name": "Delete values - Publisher - cpu",
            "value": 4.5801525,
            "unit": "median cpu",
            "extra": "avg cpu: 3.9650387510435685, max cpu: 4.58891, count: 53820"
          },
          {
            "name": "Delete values - Publisher - mem",
            "value": 31.80859375,
            "unit": "median mem",
            "extra": "avg mem: 31.161797136287625, max mem: 32.1640625, count: 53820"
          },
          {
            "name": "Find by ctid - Subscriber - cpu",
            "value": 9.142857,
            "unit": "median cpu",
            "extra": "avg cpu: 8.473178470782925, max cpu: 18.461538, count: 53820"
          },
          {
            "name": "Find by ctid - Subscriber - mem",
            "value": 53.95703125,
            "unit": "median mem",
            "extra": "avg mem: 53.59603125870959, max mem: 59.75390625, count: 53820"
          },
          {
            "name": "Index Only Scan - Subscriber - cpu",
            "value": 4.5801525,
            "unit": "median cpu",
            "extra": "avg cpu: 4.972168146332433, max cpu: 9.257474, count: 53820"
          },
          {
            "name": "Index Only Scan - Subscriber - mem",
            "value": 50.08984375,
            "unit": "median mem",
            "extra": "avg mem: 50.07747257931531, max mem: 55.8671875, count: 53820"
          },
          {
            "name": "Index Size Info - Subscriber - cpu",
            "value": 4.5801525,
            "unit": "median cpu",
            "extra": "avg cpu: 4.63808620314863, max cpu: 9.213051, count: 53820"
          },
          {
            "name": "Index Size Info - Subscriber - mem",
            "value": 33.859375,
            "unit": "median mem",
            "extra": "avg mem: 33.822185133895395, max mem: 38.9609375, count: 53820"
          },
          {
            "name": "Index Size Info - Subscriber - pages",
            "value": 1107,
            "unit": "median pages",
            "extra": "avg pages: 1101.614994425864, max pages: 1825.0, count: 53820"
          },
          {
            "name": "Index Size Info - Subscriber - relation_size:MB",
            "value": 8.6484375,
            "unit": "median relation_size:MB",
            "extra": "avg relation_size:MB: 8.606367289111855, max relation_size:MB: 14.2578125, count: 53820"
          },
          {
            "name": "Index Size Info - Subscriber - segment_count",
            "value": 10,
            "unit": "median segment_count",
            "extra": "avg segment_count: 9.888015607580824, max segment_count: 18.0, count: 53820"
          },
          {
            "name": "Insert value A - Publisher - cpu",
            "value": 4.5714283,
            "unit": "median cpu",
            "extra": "avg cpu: 3.955777820996677, max cpu: 4.628737, count: 53820"
          },
          {
            "name": "Insert value A - Publisher - mem",
            "value": 29.49609375,
            "unit": "median mem",
            "extra": "avg mem: 28.812116052350426, max mem: 29.8359375, count: 53820"
          },
          {
            "name": "Insert value B - Publisher - cpu",
            "value": 4.5714283,
            "unit": "median cpu",
            "extra": "avg cpu: 3.3883727992082573, max cpu: 4.6065254, count: 53820"
          },
          {
            "name": "Insert value B - Publisher - mem",
            "value": 29.48828125,
            "unit": "median mem",
            "extra": "avg mem: 28.785965080360462, max mem: 29.82421875, count: 53820"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - cpu",
            "value": 4.6021094,
            "unit": "median cpu",
            "extra": "avg cpu: 6.680394881674587, max cpu: 14.159292, count: 53820"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - mem",
            "value": 48.1328125,
            "unit": "median mem",
            "extra": "avg mem: 48.07431978121516, max mem: 53.8984375, count: 53820"
          },
          {
            "name": "SELECT\n  pid,\n  pg_wal_lsn_diff(sent_lsn, replay_lsn) AS replication_lag,\n  application_name::text,\n  state::text\nFROM pg_stat_replication; - Publisher - replication_lag:MB",
            "value": 0,
            "unit": "median replication_lag:MB",
            "extra": "avg replication_lag:MB: 0.00002003176776683273, max replication_lag:MB: 0.06201934814453125, count: 53820"
          },
          {
            "name": "Top N - Subscriber - cpu",
            "value": 4.5845275,
            "unit": "median cpu",
            "extra": "avg cpu: 5.160499498087826, max cpu: 13.859479, count: 107640"
          },
          {
            "name": "Top N - Subscriber - mem",
            "value": 48.75,
            "unit": "median mem",
            "extra": "avg mem: 48.73204181037486, max mem: 54.8671875, count: 107640"
          },
          {
            "name": "Update 1..9 - Publisher - cpu",
            "value": 4.5540795,
            "unit": "median cpu",
            "extra": "avg cpu: 4.304549257515497, max cpu: 4.58891, count: 53820"
          },
          {
            "name": "Update 1..9 - Publisher - mem",
            "value": 32.5,
            "unit": "median mem",
            "extra": "avg mem: 31.824822396994612, max mem: 32.85546875, count: 53820"
          },
          {
            "name": "Update 10,11 - Publisher - cpu",
            "value": 4.5714283,
            "unit": "median cpu",
            "extra": "avg cpu: 4.488105272501397, max cpu: 4.6065254, count: 53820"
          },
          {
            "name": "Update 10,11 - Publisher - mem",
            "value": 32.5234375,
            "unit": "median mem",
            "extra": "avg mem: 31.81417848267373, max mem: 32.6875, count: 53820"
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
          "id": "084451f652ebc5c322fbf12b0189bc5e229dce3a",
          "message": "fix: reduce overhead for ngram match queries and add TEXT[] regression coverage (#4150)\n\n# Ticket(s) Closed\n\n- Closes #2884\n\n## What\n\nMinor optimization to `match_query` and new regression test covering\nngram search on TEXT[] columns with `conjunction_mode`.\n\n## Why\n\nA it's reported in #2884, slow ngram searches (~16 queries/s vs ~70\nwithout ngram) on a 350k-row TEXT[] column. We investigated and found\nthe N-way posting list intersection in `BooleanQuery` with many Must\nclauses is inherently expensive and can't be fundamentally improved at\nthe pg_search level. However, we identified two sources of unnecessary\noverhead in how `match_query` constructs the query.\n\n## How\n\n1. **`IndexRecordOption::WithFreqs` instead of `WithFreqsAndPositions`**\n— `match_query` creates `TermQuery` instances inside a `BooleanQuery`.\nThe BooleanQuery scorer only uses doc iteration and BM25 scores, never\npositions. `WithFreqsAndPositions` was requesting position data that was\nnever read. `WithFreqs` produces identical BM25 scores with less\nper-document overhead.\n\n2. **Deduplicate terms for conjunction mode** — For queries with\nrepeated ngram tokens (e.g., strings with repeated substrings),\nduplicate Must clauses add intersection work without changing which\ndocuments match. Dedup removes them before building the query.\n\nBoth changes preserve identical matching semantics and BM25 scoring.\n\n## Tests\n\nNew `ngram-text-array` regression test covering the exact pattern from\nthe reported issue: TEXT[] column with ICU + ngram alias fields, `match`\nwith `conjunction_mode`, `disjunction_max`, edge cases (short queries,\nsingle-token queries), and the JSON `::jsonb` query path.",
          "timestamp": "2026-02-10T15:11:24-08:00",
          "tree_id": "ce5fefd07b9871c52c5cd32b82b7f79613310334",
          "url": "https://github.com/paradedb/paradedb/commit/084451f652ebc5c322fbf12b0189bc5e229dce3a"
        },
        "date": 1770769892966,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - cpu",
            "value": 4.58891,
            "unit": "median cpu",
            "extra": "avg cpu: 4.928160057854427, max cpu: 9.311348, count: 53834"
          },
          {
            "name": "Custom Scan - Subscriber - mem",
            "value": 50.01953125,
            "unit": "median mem",
            "extra": "avg mem: 50.0644420231638, max mem: 55.7265625, count: 53834"
          },
          {
            "name": "Delete values - Publisher - cpu",
            "value": 4.5714283,
            "unit": "median cpu",
            "extra": "avg cpu: 3.5245632592702303, max cpu: 4.6065254, count: 53834"
          },
          {
            "name": "Delete values - Publisher - mem",
            "value": 31.84765625,
            "unit": "median mem",
            "extra": "avg mem: 31.139205996674963, max mem: 32.21484375, count: 53834"
          },
          {
            "name": "Find by ctid - Subscriber - cpu",
            "value": 9.169055,
            "unit": "median cpu",
            "extra": "avg cpu: 8.83372430532931, max cpu: 18.532818, count: 53834"
          },
          {
            "name": "Find by ctid - Subscriber - mem",
            "value": 53.76171875,
            "unit": "median mem",
            "extra": "avg mem: 53.40540945313371, max mem: 59.40625, count: 53834"
          },
          {
            "name": "Index Only Scan - Subscriber - cpu",
            "value": 4.5933013,
            "unit": "median cpu",
            "extra": "avg cpu: 4.947672268151997, max cpu: 9.284333, count: 53834"
          },
          {
            "name": "Index Only Scan - Subscriber - mem",
            "value": 49.7109375,
            "unit": "median mem",
            "extra": "avg mem: 49.730174297377125, max mem: 55.40625, count: 53834"
          },
          {
            "name": "Index Size Info - Subscriber - cpu",
            "value": 4.58891,
            "unit": "median cpu",
            "extra": "avg cpu: 4.664149754735565, max cpu: 9.257474, count: 53834"
          },
          {
            "name": "Index Size Info - Subscriber - mem",
            "value": 32.93359375,
            "unit": "median mem",
            "extra": "avg mem: 32.95321961957127, max mem: 37.984375, count: 53834"
          },
          {
            "name": "Index Size Info - Subscriber - pages",
            "value": 1088,
            "unit": "median pages",
            "extra": "avg pages: 1087.6218003492218, max pages: 1793.0, count: 53834"
          },
          {
            "name": "Index Size Info - Subscriber - relation_size:MB",
            "value": 8.5,
            "unit": "median relation_size:MB",
            "extra": "avg relation_size:MB: 8.497045315228295, max relation_size:MB: 14.0078125, count: 53834"
          },
          {
            "name": "Index Size Info - Subscriber - segment_count",
            "value": 11,
            "unit": "median segment_count",
            "extra": "avg segment_count: 10.87611918118661, max segment_count: 19.0, count: 53834"
          },
          {
            "name": "Insert value A - Publisher - cpu",
            "value": 4.567079,
            "unit": "median cpu",
            "extra": "avg cpu: 4.320799467287833, max cpu: 4.5933013, count: 53834"
          },
          {
            "name": "Insert value A - Publisher - mem",
            "value": 29.515625,
            "unit": "median mem",
            "extra": "avg mem: 28.755558537007282, max mem: 29.859375, count: 53834"
          },
          {
            "name": "Insert value B - Publisher - cpu",
            "value": 4.5757866,
            "unit": "median cpu",
            "extra": "avg cpu: 2.9881495481405853, max cpu: 4.6153846, count: 53834"
          },
          {
            "name": "Insert value B - Publisher - mem",
            "value": 29.46484375,
            "unit": "median mem",
            "extra": "avg mem: 28.737907197937176, max mem: 29.76953125, count: 53834"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - cpu",
            "value": 4.610951,
            "unit": "median cpu",
            "extra": "avg cpu: 6.570718833964682, max cpu: 13.872832, count: 53834"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - mem",
            "value": 48.390625,
            "unit": "median mem",
            "extra": "avg mem: 48.43494512645354, max mem: 54.109375, count: 53834"
          },
          {
            "name": "SELECT\n  pid,\n  pg_wal_lsn_diff(sent_lsn, replay_lsn) AS replication_lag,\n  application_name::text,\n  state::text\nFROM pg_stat_replication; - Publisher - replication_lag:MB",
            "value": 0,
            "unit": "median replication_lag:MB",
            "extra": "avg replication_lag:MB: 0.00002853547179973971, max replication_lag:MB: 0.21314239501953125, count: 53834"
          },
          {
            "name": "Top N - Subscriber - cpu",
            "value": 4.5933013,
            "unit": "median cpu",
            "extra": "avg cpu: 5.2415491698641095, max cpu: 13.9265, count: 107668"
          },
          {
            "name": "Top N - Subscriber - mem",
            "value": 48.59375,
            "unit": "median mem",
            "extra": "avg mem: 48.592106166351655, max mem: 54.6640625, count: 107668"
          },
          {
            "name": "Update 1..9 - Publisher - cpu",
            "value": 4.567079,
            "unit": "median cpu",
            "extra": "avg cpu: 3.610642807535867, max cpu: 4.6065254, count: 53834"
          },
          {
            "name": "Update 1..9 - Publisher - mem",
            "value": 32.43359375,
            "unit": "median mem",
            "extra": "avg mem: 31.719149448420144, max mem: 32.78515625, count: 53834"
          },
          {
            "name": "Update 10,11 - Publisher - cpu",
            "value": 4.5933013,
            "unit": "median cpu",
            "extra": "avg cpu: 4.186518697967765, max cpu: 4.5933013, count: 53834"
          },
          {
            "name": "Update 10,11 - Publisher - mem",
            "value": 32.484375,
            "unit": "median mem",
            "extra": "avg mem: 31.793193036115653, max mem: 32.6015625, count: 53834"
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
          "id": "59354e0de782d993f3e4a260eb7c56ad4804a1ad",
          "message": "fix: add field validation for `paradedb.aggregate()` API (#4141)\n\n# Ticket(s) Closed\n\n- Closes #N/A\n\n## What\n\nAdds field validation to the `paradedb.aggregate()` direct SQL function\nso that referencing a nonexistent or unsupported field returns a clear\nerror instead of silently producing null results.\n\n## Why\n\nThe `pdb.agg()` aggregate path already validates fields at plan time via\n`AggregateType::validate_fields()`. However, the `paradedb.aggregate()`\nfunction is a plain `pg_extern` that calls `execute_aggregate()`\ndirectly — it never enters the custom scan planner, so it skipped\nvalidation entirely. An invalid field like `\"nonexistent_field\"` would\nquietly return `{\"value\": null}` instead of telling the user something\nis wrong.\n\n## How\n\n- Extracted the field validation logic from\n`AggregateType::validate_fields()` into a standalone\n`validate_agg_json_fields()` function in `aggregate_type.rs`. The\nexisting `validate_fields()` now delegates to it for custom aggregates.\n- Called `validate_agg_json_fields()` in `aggregate_impl()`\n(`api/aggregate.rs`) before executing, so the direct API gets the same\nvalidation as the planner path.\n\n## Tests\n\n- Added regression tests (tests 13–15 in `agg-validate.sql`) covering\nthe `paradedb.aggregate()` path: valid field succeeds, invalid field\nerrors, invalid nested field errors.",
          "timestamp": "2026-02-10T15:12:54-08:00",
          "tree_id": "a2a30dc05294896dfaef747d15452a4024f5d8aa",
          "url": "https://github.com/paradedb/paradedb/commit/59354e0de782d993f3e4a260eb7c56ad4804a1ad"
        },
        "date": 1770770027419,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - cpu",
            "value": 4.5801525,
            "unit": "median cpu",
            "extra": "avg cpu: 4.910501295896724, max cpu: 9.257474, count: 53871"
          },
          {
            "name": "Custom Scan - Subscriber - mem",
            "value": 50.1015625,
            "unit": "median mem",
            "extra": "avg mem: 50.109083867595736, max mem: 55.9296875, count: 53871"
          },
          {
            "name": "Delete values - Publisher - cpu",
            "value": 4.597701,
            "unit": "median cpu",
            "extra": "avg cpu: 4.410108238264646, max cpu: 4.610951, count: 53871"
          },
          {
            "name": "Delete values - Publisher - mem",
            "value": 31.984375,
            "unit": "median mem",
            "extra": "avg mem: 31.230668953379368, max mem: 32.30078125, count: 53871"
          },
          {
            "name": "Find by ctid - Subscriber - cpu",
            "value": 9.099526,
            "unit": "median cpu",
            "extra": "avg cpu: 7.356275596609236, max cpu: 18.408438, count: 53871"
          },
          {
            "name": "Find by ctid - Subscriber - mem",
            "value": 53.97265625,
            "unit": "median mem",
            "extra": "avg mem: 53.65433519716081, max mem: 59.8359375, count: 53871"
          },
          {
            "name": "Index Only Scan - Subscriber - cpu",
            "value": 4.5801525,
            "unit": "median cpu",
            "extra": "avg cpu: 4.9186357610350955, max cpu: 9.266409, count: 53871"
          },
          {
            "name": "Index Only Scan - Subscriber - mem",
            "value": 49.73828125,
            "unit": "median mem",
            "extra": "avg mem: 49.75009419202818, max mem: 55.53125, count: 53871"
          },
          {
            "name": "Index Size Info - Subscriber - cpu",
            "value": 4.5801525,
            "unit": "median cpu",
            "extra": "avg cpu: 4.638564223665322, max cpu: 9.221902, count: 53871"
          },
          {
            "name": "Index Size Info - Subscriber - mem",
            "value": 33.6015625,
            "unit": "median mem",
            "extra": "avg mem: 33.60192708623378, max mem: 38.62890625, count: 53871"
          },
          {
            "name": "Index Size Info - Subscriber - pages",
            "value": 1101,
            "unit": "median pages",
            "extra": "avg pages: 1100.25742978597, max pages: 1821.0, count: 53871"
          },
          {
            "name": "Index Size Info - Subscriber - relation_size:MB",
            "value": 8.6015625,
            "unit": "median relation_size:MB",
            "extra": "avg relation_size:MB: 8.595761170202891, max relation_size:MB: 14.2265625, count: 53871"
          },
          {
            "name": "Index Size Info - Subscriber - segment_count",
            "value": 7,
            "unit": "median segment_count",
            "extra": "avg segment_count: 7.36838001893412, max segment_count: 14.0, count: 53871"
          },
          {
            "name": "Insert value A - Publisher - cpu",
            "value": 4.5714283,
            "unit": "median cpu",
            "extra": "avg cpu: 4.3587932946912264, max cpu: 9.195402, count: 53871"
          },
          {
            "name": "Insert value A - Publisher - mem",
            "value": 29.65625,
            "unit": "median mem",
            "extra": "avg mem: 28.95148298423549, max mem: 29.9921875, count: 53871"
          },
          {
            "name": "Insert value B - Publisher - cpu",
            "value": 4.5801525,
            "unit": "median cpu",
            "extra": "avg cpu: 3.8791631582503574, max cpu: 4.5801525, count: 53871"
          },
          {
            "name": "Insert value B - Publisher - mem",
            "value": 29.6640625,
            "unit": "median mem",
            "extra": "avg mem: 28.966649423507082, max mem: 30.05859375, count: 53871"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - cpu",
            "value": 4.610951,
            "unit": "median cpu",
            "extra": "avg cpu: 6.886195317548848, max cpu: 22.878933, count: 53871"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - mem",
            "value": 48.0390625,
            "unit": "median mem",
            "extra": "avg mem: 48.054576195332366, max mem: 53.89453125, count: 53871"
          },
          {
            "name": "SELECT\n  pid,\n  pg_wal_lsn_diff(sent_lsn, replay_lsn) AS replication_lag,\n  application_name::text,\n  state::text\nFROM pg_stat_replication; - Publisher - replication_lag:MB",
            "value": 0,
            "unit": "median replication_lag:MB",
            "extra": "avg replication_lag:MB: 0.000029161250761004877, max replication_lag:MB: 0.31922149658203125, count: 53871"
          },
          {
            "name": "Top N - Subscriber - cpu",
            "value": 4.5845275,
            "unit": "median cpu",
            "extra": "avg cpu: 5.106363938924294, max cpu: 13.779904, count: 107742"
          },
          {
            "name": "Top N - Subscriber - mem",
            "value": 48.41015625,
            "unit": "median mem",
            "extra": "avg mem: 48.40327432228147, max mem: 54.44140625, count: 107742"
          },
          {
            "name": "Update 1..9 - Publisher - cpu",
            "value": 4.58891,
            "unit": "median cpu",
            "extra": "avg cpu: 4.549563895612631, max cpu: 4.619827, count: 53871"
          },
          {
            "name": "Update 1..9 - Publisher - mem",
            "value": 32.6328125,
            "unit": "median mem",
            "extra": "avg mem: 31.92013494911455, max mem: 32.94921875, count: 53871"
          },
          {
            "name": "Update 10,11 - Publisher - cpu",
            "value": 4.5845275,
            "unit": "median cpu",
            "extra": "avg cpu: 4.396362550583581, max cpu: 4.597701, count: 53871"
          },
          {
            "name": "Update 10,11 - Publisher - mem",
            "value": 32.765625,
            "unit": "median mem",
            "extra": "avg mem: 32.09002256258005, max mem: 32.85546875, count: 53871"
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
          "id": "44d1f928882599cf5fd9fbc853c8eee1fb5c57ed",
          "message": "fix: Rebase against Tantivy, inherit upstream bugfix for intersection queries (#4155)\n\n# Ticket(s) Closed\n\n- Closes #4149 \n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2026-02-11T11:47:17-05:00",
          "tree_id": "b88ad04015b13dee26d44c4c9d585ea252d0323c",
          "url": "https://github.com/paradedb/paradedb/commit/44d1f928882599cf5fd9fbc853c8eee1fb5c57ed"
        },
        "date": 1770832967686,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - cpu",
            "value": 4.5757866,
            "unit": "median cpu",
            "extra": "avg cpu: 4.974884771095925, max cpu: 9.248554, count: 53890"
          },
          {
            "name": "Custom Scan - Subscriber - mem",
            "value": 50.6796875,
            "unit": "median mem",
            "extra": "avg mem: 50.6897995337725, max mem: 56.421875, count: 53890"
          },
          {
            "name": "Delete values - Publisher - cpu",
            "value": 4.5757866,
            "unit": "median cpu",
            "extra": "avg cpu: 4.408640968160904, max cpu: 4.5801525, count: 53890"
          },
          {
            "name": "Delete values - Publisher - mem",
            "value": 32.015625,
            "unit": "median mem",
            "extra": "avg mem: 31.328876313439412, max mem: 32.421875, count: 53890"
          },
          {
            "name": "Find by ctid - Subscriber - cpu",
            "value": 9.108159,
            "unit": "median cpu",
            "extra": "avg cpu: 7.7329473343020405, max cpu: 18.33811, count: 53890"
          },
          {
            "name": "Find by ctid - Subscriber - mem",
            "value": 54.453125,
            "unit": "median mem",
            "extra": "avg mem: 54.09559461402858, max mem: 60.17578125, count: 53890"
          },
          {
            "name": "Index Only Scan - Subscriber - cpu",
            "value": 4.5757866,
            "unit": "median cpu",
            "extra": "avg cpu: 4.9364183665049985, max cpu: 9.257474, count: 53890"
          },
          {
            "name": "Index Only Scan - Subscriber - mem",
            "value": 50.328125,
            "unit": "median mem",
            "extra": "avg mem: 50.31179725192522, max mem: 56.04296875, count: 53890"
          },
          {
            "name": "Index Size Info - Subscriber - cpu",
            "value": 4.5757866,
            "unit": "median cpu",
            "extra": "avg cpu: 4.65233272336917, max cpu: 9.204219, count: 53890"
          },
          {
            "name": "Index Size Info - Subscriber - mem",
            "value": 33.58984375,
            "unit": "median mem",
            "extra": "avg mem: 33.565579334060125, max mem: 38.58203125, count: 53890"
          },
          {
            "name": "Index Size Info - Subscriber - pages",
            "value": 1107,
            "unit": "median pages",
            "extra": "avg pages: 1102.8333085915754, max pages: 1815.0, count: 53890"
          },
          {
            "name": "Index Size Info - Subscriber - relation_size:MB",
            "value": 8.6484375,
            "unit": "median relation_size:MB",
            "extra": "avg relation_size:MB: 8.615885223371682, max relation_size:MB: 14.1796875, count: 53890"
          },
          {
            "name": "Index Size Info - Subscriber - segment_count",
            "value": 9,
            "unit": "median segment_count",
            "extra": "avg segment_count: 9.762664687326035, max segment_count: 19.0, count: 53890"
          },
          {
            "name": "Insert value A - Publisher - cpu",
            "value": 4.5411544,
            "unit": "median cpu",
            "extra": "avg cpu: 2.37725292767596, max cpu: 4.58891, count: 53890"
          },
          {
            "name": "Insert value A - Publisher - mem",
            "value": 29.4921875,
            "unit": "median mem",
            "extra": "avg mem: 28.77284906174615, max mem: 29.85546875, count: 53890"
          },
          {
            "name": "Insert value B - Publisher - cpu",
            "value": 4.5845275,
            "unit": "median cpu",
            "extra": "avg cpu: 4.002448038431663, max cpu: 4.5845275, count: 53890"
          },
          {
            "name": "Insert value B - Publisher - mem",
            "value": 29.4765625,
            "unit": "median mem",
            "extra": "avg mem: 28.75983086205697, max mem: 29.8828125, count: 53890"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - cpu",
            "value": 4.6065254,
            "unit": "median cpu",
            "extra": "avg cpu: 6.885329696119326, max cpu: 23.054754, count: 53890"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - mem",
            "value": 48.7265625,
            "unit": "median mem",
            "extra": "avg mem: 48.687802917401186, max mem: 54.390625, count: 53890"
          },
          {
            "name": "SELECT\n  pid,\n  pg_wal_lsn_diff(sent_lsn, replay_lsn) AS replication_lag,\n  application_name::text,\n  state::text\nFROM pg_stat_replication; - Publisher - replication_lag:MB",
            "value": 0,
            "unit": "median replication_lag:MB",
            "extra": "avg replication_lag:MB: 0.000027447698907786696, max replication_lag:MB: 0.14870452880859375, count: 53890"
          },
          {
            "name": "Top N - Subscriber - cpu",
            "value": 4.5757866,
            "unit": "median cpu",
            "extra": "avg cpu: 5.113570764163025, max cpu: 13.819577, count: 107780"
          },
          {
            "name": "Top N - Subscriber - mem",
            "value": 49.20703125,
            "unit": "median mem",
            "extra": "avg mem: 49.19321335272082, max mem: 55.0703125, count: 107780"
          },
          {
            "name": "Update 1..9 - Publisher - cpu",
            "value": 4.5283017,
            "unit": "median cpu",
            "extra": "avg cpu: 3.6426036818564063, max cpu: 4.5933013, count: 53890"
          },
          {
            "name": "Update 1..9 - Publisher - mem",
            "value": 32.74609375,
            "unit": "median mem",
            "extra": "avg mem: 32.052583097513455, max mem: 33.109375, count: 53890"
          },
          {
            "name": "Update 10,11 - Publisher - cpu",
            "value": 4.5845275,
            "unit": "median cpu",
            "extra": "avg cpu: 4.561765477179295, max cpu: 9.186603, count: 53890"
          },
          {
            "name": "Update 10,11 - Publisher - mem",
            "value": 32.75,
            "unit": "median mem",
            "extra": "avg mem: 32.079090290986265, max mem: 32.875, count: 53890"
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
          "id": "80daf35915e5edde9b7e091036a88ce3d6c6aea1",
          "message": "chore: Upgrade to `0.21.8` (#4168)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2026-02-12T09:45:33-05:00",
          "tree_id": "3de2b0c6e4f9a21b26370ecab28ddd8db57c65ff",
          "url": "https://github.com/paradedb/paradedb/commit/80daf35915e5edde9b7e091036a88ce3d6c6aea1"
        },
        "date": 1770912608249,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - cpu",
            "value": 4.5801525,
            "unit": "median cpu",
            "extra": "avg cpu: 4.967496325020059, max cpu: 9.257474, count: 53879"
          },
          {
            "name": "Custom Scan - Subscriber - mem",
            "value": 50.41796875,
            "unit": "median mem",
            "extra": "avg mem: 50.42741374190315, max mem: 56.30078125, count: 53879"
          },
          {
            "name": "Delete values - Publisher - cpu",
            "value": 4.5757866,
            "unit": "median cpu",
            "extra": "avg cpu: 4.220485034162298, max cpu: 4.6065254, count: 53879"
          },
          {
            "name": "Delete values - Publisher - mem",
            "value": 32.5625,
            "unit": "median mem",
            "extra": "avg mem: 31.889172526633754, max mem: 33.26953125, count: 53879"
          },
          {
            "name": "Find by ctid - Subscriber - cpu",
            "value": 9.099526,
            "unit": "median cpu",
            "extra": "avg cpu: 7.396522213128477, max cpu: 18.33811, count: 53879"
          },
          {
            "name": "Find by ctid - Subscriber - mem",
            "value": 54.015625,
            "unit": "median mem",
            "extra": "avg mem: 53.65804772035487, max mem: 59.8359375, count: 53879"
          },
          {
            "name": "Index Only Scan - Subscriber - cpu",
            "value": 4.5801525,
            "unit": "median cpu",
            "extra": "avg cpu: 4.901641212354606, max cpu: 9.257474, count: 53879"
          },
          {
            "name": "Index Only Scan - Subscriber - mem",
            "value": 50.9296875,
            "unit": "median mem",
            "extra": "avg mem: 50.92853314835094, max mem: 56.74609375, count: 53879"
          },
          {
            "name": "Index Size Info - Subscriber - cpu",
            "value": 4.5801525,
            "unit": "median cpu",
            "extra": "avg cpu: 4.644897269016387, max cpu: 9.230769, count: 53879"
          },
          {
            "name": "Index Size Info - Subscriber - mem",
            "value": 33.390625,
            "unit": "median mem",
            "extra": "avg mem: 33.3733483679866, max mem: 38.51953125, count: 53879"
          },
          {
            "name": "Index Size Info - Subscriber - pages",
            "value": 1103,
            "unit": "median pages",
            "extra": "avg pages: 1102.8755916034077, max pages: 1833.0, count: 53879"
          },
          {
            "name": "Index Size Info - Subscriber - relation_size:MB",
            "value": 8.6171875,
            "unit": "median relation_size:MB",
            "extra": "avg relation_size:MB: 8.616215849403293, max relation_size:MB: 14.3203125, count: 53879"
          },
          {
            "name": "Index Size Info - Subscriber - segment_count",
            "value": 7,
            "unit": "median segment_count",
            "extra": "avg segment_count: 7.044748417750887, max segment_count: 13.0, count: 53879"
          },
          {
            "name": "Insert value A - Publisher - cpu",
            "value": 4.562738,
            "unit": "median cpu",
            "extra": "avg cpu: 4.330918366675515, max cpu: 4.5757866, count: 53879"
          },
          {
            "name": "Insert value A - Publisher - mem",
            "value": 29.5625,
            "unit": "median mem",
            "extra": "avg mem: 28.91902798110117, max mem: 30.125, count: 53879"
          },
          {
            "name": "Insert value B - Publisher - cpu",
            "value": 0,
            "unit": "median cpu",
            "extra": "avg cpu: 2.12173268278939, max cpu: 4.597701, count: 53879"
          },
          {
            "name": "Insert value B - Publisher - mem",
            "value": 29.4609375,
            "unit": "median mem",
            "extra": "avg mem: 28.824802450862116, max mem: 29.80078125, count: 53879"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - cpu",
            "value": 4.6065254,
            "unit": "median cpu",
            "extra": "avg cpu: 6.772294835434446, max cpu: 23.032629, count: 53879"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - mem",
            "value": 48.8828125,
            "unit": "median mem",
            "extra": "avg mem: 48.86076998053509, max mem: 54.7578125, count: 53879"
          },
          {
            "name": "SELECT\n  pid,\n  pg_wal_lsn_diff(sent_lsn, replay_lsn) AS replication_lag,\n  application_name::text,\n  state::text\nFROM pg_stat_replication; - Publisher - replication_lag:MB",
            "value": 0,
            "unit": "median replication_lag:MB",
            "extra": "avg replication_lag:MB: 0.000024269939599539593, max replication_lag:MB: 0.0942840576171875, count: 53879"
          },
          {
            "name": "Top N - Subscriber - cpu",
            "value": 4.5801525,
            "unit": "median cpu",
            "extra": "avg cpu: 5.028741572643115, max cpu: 13.859479, count: 107758"
          },
          {
            "name": "Top N - Subscriber - mem",
            "value": 48.69921875,
            "unit": "median mem",
            "extra": "avg mem: 48.69690189040489, max mem: 54.61328125, count: 107758"
          },
          {
            "name": "Update 1..9 - Publisher - cpu",
            "value": 4.5757866,
            "unit": "median cpu",
            "extra": "avg cpu: 4.16048597686006, max cpu: 4.5933013, count: 53879"
          },
          {
            "name": "Update 1..9 - Publisher - mem",
            "value": 32.82421875,
            "unit": "median mem",
            "extra": "avg mem: 32.06818164022625, max mem: 32.83203125, count: 53879"
          },
          {
            "name": "Update 10,11 - Publisher - cpu",
            "value": 4.5714283,
            "unit": "median cpu",
            "extra": "avg cpu: 4.056209638788272, max cpu: 4.610951, count: 53879"
          },
          {
            "name": "Update 10,11 - Publisher - mem",
            "value": 33.109375,
            "unit": "median mem",
            "extra": "avg mem: 32.40630046029065, max mem: 33.515625, count: 53879"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "2aec8447addadd4def1cf10f4d11e24c1755fadb",
          "message": "chore: Remove tuned_postgres from /benchmarks (#4167)\n\n# Ticket(s) Closed\n\n- Closes #N/A\n\n## What\nWe are moving user-facing benchmarks to a much more complete project,\nwhich will be in a separate repository. This is no longer relevant to\nkeep here, so removing.\n\n## Why\n^\n\n## How\n^\n\n## Tests\n^",
          "timestamp": "2026-02-12T12:21:38-05:00",
          "tree_id": "3502c5fdf7ad1b45110e95b70c181dd5ca1eae37",
          "url": "https://github.com/paradedb/paradedb/commit/2aec8447addadd4def1cf10f4d11e24c1755fadb"
        },
        "date": 1770921801051,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - cpu",
            "value": 4.5757866,
            "unit": "median cpu",
            "extra": "avg cpu: 4.9632212699319895, max cpu: 9.266409, count: 53861"
          },
          {
            "name": "Custom Scan - Subscriber - mem",
            "value": 50.515625,
            "unit": "median mem",
            "extra": "avg mem: 50.50436206740499, max mem: 56.33984375, count: 53861"
          },
          {
            "name": "Delete values - Publisher - cpu",
            "value": 4.567079,
            "unit": "median cpu",
            "extra": "avg cpu: 4.143654480322024, max cpu: 4.6021094, count: 53861"
          },
          {
            "name": "Delete values - Publisher - mem",
            "value": 32.515625,
            "unit": "median mem",
            "extra": "avg mem: 31.840695334402444, max mem: 33.20703125, count: 53861"
          },
          {
            "name": "Find by ctid - Subscriber - cpu",
            "value": 9.099526,
            "unit": "median cpu",
            "extra": "avg cpu: 7.4612696681638715, max cpu: 18.268316, count: 53861"
          },
          {
            "name": "Find by ctid - Subscriber - mem",
            "value": 54.2265625,
            "unit": "median mem",
            "extra": "avg mem: 53.8365640404235, max mem: 59.97265625, count: 53861"
          },
          {
            "name": "Index Only Scan - Subscriber - cpu",
            "value": 4.5757866,
            "unit": "median cpu",
            "extra": "avg cpu: 4.913626960377779, max cpu: 9.266409, count: 53861"
          },
          {
            "name": "Index Only Scan - Subscriber - mem",
            "value": 50.23046875,
            "unit": "median mem",
            "extra": "avg mem: 50.19463584504558, max mem: 55.984375, count: 53861"
          },
          {
            "name": "Index Size Info - Subscriber - cpu",
            "value": 4.5714283,
            "unit": "median cpu",
            "extra": "avg cpu: 4.607485560807655, max cpu: 9.213051, count: 53861"
          },
          {
            "name": "Index Size Info - Subscriber - mem",
            "value": 33.2890625,
            "unit": "median mem",
            "extra": "avg mem: 33.270442812285324, max mem: 38.4921875, count: 53861"
          },
          {
            "name": "Index Size Info - Subscriber - pages",
            "value": 1114,
            "unit": "median pages",
            "extra": "avg pages: 1109.4342845472606, max pages: 1832.0, count: 53861"
          },
          {
            "name": "Index Size Info - Subscriber - relation_size:MB",
            "value": 8.703125,
            "unit": "median relation_size:MB",
            "extra": "avg relation_size:MB: 8.66745542055012, max relation_size:MB: 14.3125, count: 53861"
          },
          {
            "name": "Index Size Info - Subscriber - segment_count",
            "value": 8,
            "unit": "median segment_count",
            "extra": "avg segment_count: 8.261989194407828, max segment_count: 13.0, count: 53861"
          },
          {
            "name": "Insert value A - Publisher - cpu",
            "value": 4.58891,
            "unit": "median cpu",
            "extra": "avg cpu: 3.621875233175954, max cpu: 4.597701, count: 53861"
          },
          {
            "name": "Insert value A - Publisher - mem",
            "value": 29.828125,
            "unit": "median mem",
            "extra": "avg mem: 29.149367338496315, max mem: 30.28125, count: 53861"
          },
          {
            "name": "Insert value B - Publisher - cpu",
            "value": 4.5540795,
            "unit": "median cpu",
            "extra": "avg cpu: 3.1979982570287406, max cpu: 4.597701, count: 53861"
          },
          {
            "name": "Insert value B - Publisher - mem",
            "value": 29.65625,
            "unit": "median mem",
            "extra": "avg mem: 29.005078610915135, max mem: 30.1796875, count: 53861"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - cpu",
            "value": 4.6065254,
            "unit": "median cpu",
            "extra": "avg cpu: 6.899807102715558, max cpu: 22.966507, count: 53861"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - mem",
            "value": 48.84375,
            "unit": "median mem",
            "extra": "avg mem: 48.807635191743564, max mem: 54.6015625, count: 53861"
          },
          {
            "name": "SELECT\n  pid,\n  pg_wal_lsn_diff(sent_lsn, replay_lsn) AS replication_lag,\n  application_name::text,\n  state::text\nFROM pg_stat_replication; - Publisher - replication_lag:MB",
            "value": 0,
            "unit": "median replication_lag:MB",
            "extra": "avg replication_lag:MB: 0.000053267369961132594, max replication_lag:MB: 0.31899261474609375, count: 53861"
          },
          {
            "name": "Top N - Subscriber - cpu",
            "value": 4.5757866,
            "unit": "median cpu",
            "extra": "avg cpu: 5.046003144863495, max cpu: 13.819577, count: 107722"
          },
          {
            "name": "Top N - Subscriber - mem",
            "value": 48.85546875,
            "unit": "median mem",
            "extra": "avg mem: 48.84416592884926, max mem: 54.83984375, count: 107722"
          },
          {
            "name": "Update 1..9 - Publisher - cpu",
            "value": 4.5714283,
            "unit": "median cpu",
            "extra": "avg cpu: 3.7132753986561924, max cpu: 4.610951, count: 53861"
          },
          {
            "name": "Update 1..9 - Publisher - mem",
            "value": 32.9375,
            "unit": "median mem",
            "extra": "avg mem: 32.180028655938436, max mem: 32.9375, count: 53861"
          },
          {
            "name": "Update 10,11 - Publisher - cpu",
            "value": 4.567079,
            "unit": "median cpu",
            "extra": "avg cpu: 4.402035830987926, max cpu: 4.58891, count: 53861"
          },
          {
            "name": "Update 10,11 - Publisher - mem",
            "value": 33.14453125,
            "unit": "median mem",
            "extra": "avg mem: 32.442021694152544, max mem: 33.546875, count: 53861"
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
          "id": "ba868f34636e9fc6068c68b3b0d8a098eb4971d8",
          "message": "feat: join-scan: pre-materialization dynamic filter pushdown from TopK and HashJoin (#4161)\n\n## Ticket(s) Closed\n\n- Closes #4151\n\n## What\n\nDynamic filters from DataFusion's `SortExec(TopK)` and `HashJoinExec`\nare now pushed down into `PgSearchScan` and applied *before* column\nmaterialization — at the term-ordinal level for strings and the\nfast-field level for numerics. This avoids expensive term dictionary I/O\nfor documents that will be discarded anyway.\n\n## Why\n\nPreviously, `PgSearchScan` had no awareness of dynamic filters. Every\ndocument that passed the Tantivy query and visibility checks was fully\nmaterialized (all fast-field columns loaded, string dictionaries walked)\nbefore any join-key or TopK pruning could happen upstream. For selective\njoins or tight LIMIT queries, this meant loading data for rows that were\nimmediately thrown away by HashJoin or TopK.\n\n## How\n\n- Enabled DataFusion's TopK dynamic filter pushdown in the JoinScan\nsession config.\n- `SegmentPlan` now accepts dynamic filters from parent operators (TopK\nthresholds, HashJoin key bounds) and passes them to the Scanner on each\nbatch.\n- Before column materialization, the Scanner converts these filters to\nterm-ordinal comparisons (for strings) or direct fast-field comparisons\n(for numerics) and prunes non-matching documents in-place — skipping\ndictionary I/O entirely for pruned rows.\n\n## Tests\n\n- New `topk_dynamic_filter` regression test covering. You can take a\nlook at EXPLAIN ANALYZE diff in the follow-up PR (#4162):\nhttps://github.com/paradedb/paradedb/blob/3b074a9b5516a7a0a75a948201ef32e07b0127e4/pg_search/tests/pg_regress/expected/topk_dynamic_filter.out#L170-L181\n- All existing regression tests pass.",
          "timestamp": "2026-02-12T10:25:25-08:00",
          "tree_id": "748bfdacf0d0b82f9ceb26840b3100a7ca8e2252",
          "url": "https://github.com/paradedb/paradedb/commit/ba868f34636e9fc6068c68b3b0d8a098eb4971d8"
        },
        "date": 1770925597590,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - cpu",
            "value": 4.5801525,
            "unit": "median cpu",
            "extra": "avg cpu: 4.976684838642917, max cpu: 9.266409, count: 53886"
          },
          {
            "name": "Custom Scan - Subscriber - mem",
            "value": 50.08984375,
            "unit": "median mem",
            "extra": "avg mem: 50.04897477429203, max mem: 55.65234375, count: 53886"
          },
          {
            "name": "Delete values - Publisher - cpu",
            "value": 4.5757866,
            "unit": "median cpu",
            "extra": "avg cpu: 3.8432915807427896, max cpu: 4.6021094, count: 53886"
          },
          {
            "name": "Delete values - Publisher - mem",
            "value": 31.83984375,
            "unit": "median mem",
            "extra": "avg mem: 31.141091262108898, max mem: 32.1875, count: 53886"
          },
          {
            "name": "Find by ctid - Subscriber - cpu",
            "value": 9.090909,
            "unit": "median cpu",
            "extra": "avg cpu: 7.413548526331132, max cpu: 18.390804, count: 53886"
          },
          {
            "name": "Find by ctid - Subscriber - mem",
            "value": 53.78125,
            "unit": "median mem",
            "extra": "avg mem: 53.405582067930446, max mem: 59.3046875, count: 53886"
          },
          {
            "name": "Index Only Scan - Subscriber - cpu",
            "value": 4.5801525,
            "unit": "median cpu",
            "extra": "avg cpu: 4.927435046120626, max cpu: 9.257474, count: 53886"
          },
          {
            "name": "Index Only Scan - Subscriber - mem",
            "value": 49.94921875,
            "unit": "median mem",
            "extra": "avg mem: 49.9146185059663, max mem: 55.44921875, count: 53886"
          },
          {
            "name": "Index Size Info - Subscriber - cpu",
            "value": 4.5757866,
            "unit": "median cpu",
            "extra": "avg cpu: 4.631799844639645, max cpu: 9.248554, count: 53886"
          },
          {
            "name": "Index Size Info - Subscriber - mem",
            "value": 33.09765625,
            "unit": "median mem",
            "extra": "avg mem: 33.04934513080856, max mem: 37.9453125, count: 53886"
          },
          {
            "name": "Index Size Info - Subscriber - pages",
            "value": 1094,
            "unit": "median pages",
            "extra": "avg pages: 1085.7588984151728, max pages: 1775.0, count: 53886"
          },
          {
            "name": "Index Size Info - Subscriber - relation_size:MB",
            "value": 8.546875,
            "unit": "median relation_size:MB",
            "extra": "avg relation_size:MB: 8.482491538850537, max relation_size:MB: 13.8671875, count: 53886"
          },
          {
            "name": "Index Size Info - Subscriber - segment_count",
            "value": 6,
            "unit": "median segment_count",
            "extra": "avg segment_count: 6.828062947704413, max segment_count: 14.0, count: 53886"
          },
          {
            "name": "Insert value A - Publisher - cpu",
            "value": 4.562738,
            "unit": "median cpu",
            "extra": "avg cpu: 3.4388965737685004, max cpu: 4.5801525, count: 53886"
          },
          {
            "name": "Insert value A - Publisher - mem",
            "value": 29.73828125,
            "unit": "median mem",
            "extra": "avg mem: 29.055385733307354, max mem: 30.11328125, count: 53886"
          },
          {
            "name": "Insert value B - Publisher - cpu",
            "value": 4.58891,
            "unit": "median cpu",
            "extra": "avg cpu: 3.4180557748531397, max cpu: 4.6021094, count: 53886"
          },
          {
            "name": "Insert value B - Publisher - mem",
            "value": 29.7109375,
            "unit": "median mem",
            "extra": "avg mem: 29.047600127468172, max mem: 30.08203125, count: 53886"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - cpu",
            "value": 4.6065254,
            "unit": "median cpu",
            "extra": "avg cpu: 6.85078875526536, max cpu: 22.944551, count: 53886"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - mem",
            "value": 47.859375,
            "unit": "median mem",
            "extra": "avg mem: 47.828163347738744, max mem: 53.3359375, count: 53886"
          },
          {
            "name": "SELECT\n  pid,\n  pg_wal_lsn_diff(sent_lsn, replay_lsn) AS replication_lag,\n  application_name::text,\n  state::text\nFROM pg_stat_replication; - Publisher - replication_lag:MB",
            "value": 0,
            "unit": "median replication_lag:MB",
            "extra": "avg replication_lag:MB: 0.000019438631421397718, max replication_lag:MB: 0.1060791015625, count: 53886"
          },
          {
            "name": "Top N - Subscriber - cpu",
            "value": 4.5801525,
            "unit": "median cpu",
            "extra": "avg cpu: 5.0904292621854, max cpu: 13.793103, count: 107772"
          },
          {
            "name": "Top N - Subscriber - mem",
            "value": 48.49609375,
            "unit": "median mem",
            "extra": "avg mem: 48.53044828579084, max mem: 54.453125, count: 107772"
          },
          {
            "name": "Update 1..9 - Publisher - cpu",
            "value": 4.5714283,
            "unit": "median cpu",
            "extra": "avg cpu: 4.474684760427811, max cpu: 4.5845275, count: 53886"
          },
          {
            "name": "Update 1..9 - Publisher - mem",
            "value": 32.58984375,
            "unit": "median mem",
            "extra": "avg mem: 31.94936141228705, max mem: 32.91796875, count: 53886"
          },
          {
            "name": "Update 10,11 - Publisher - cpu",
            "value": 4.5801525,
            "unit": "median cpu",
            "extra": "avg cpu: 3.814105456100634, max cpu: 4.6065254, count: 53886"
          },
          {
            "name": "Update 10,11 - Publisher - mem",
            "value": 32.83984375,
            "unit": "median mem",
            "extra": "avg mem: 32.18080937070853, max mem: 32.91796875, count: 53886"
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
          "id": "655ee8b04cab31c056cb41a89d677b896630ed16",
          "message": "feat: join-scan: surface dynamic filter metrics in EXPLAIN ANALYZE (#4162)\n\n# Ticket(s) Closed\n\n- Closes #4151\n\n## What\n\nUsed DataFusion metrics, and made dynamic filter pruning stats visible\nthrough `EXPLAIN ANALYZE`.\n\n## Why\n\n`EXPLAIN ANALYZE` is the natural place for execution-time stats.\n\n## How\n\n- Added `ExecutionPlanMetricsSet` to `SegmentPlan` with two custom\ncounters (`rows_scanned`, `rows_pruned`), only registered when dynamic\nfilters are present.\n\n## Tests\n\n- Updated `topk_dynamic_filter` regression test to use `EXPLAIN\n(ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)` — verifying\n`Dynamic Filter` lines appear with correct pruning stats (e.g., `30\nscanned, 24 pruned (80.0%)`).\n- Updated `join_custom_scan` and `filter_pushdown_datafusion` expected\noutput.\n\n---------\n\nSigned-off-by: Moe <mdashti@gmail.com>",
          "timestamp": "2026-02-12T12:45:44-08:00",
          "tree_id": "8c73104c0b40b30047e010ebfba45fb9add3f7e8",
          "url": "https://github.com/paradedb/paradedb/commit/655ee8b04cab31c056cb41a89d677b896630ed16"
        },
        "date": 1770934060338,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - cpu",
            "value": 4.5757866,
            "unit": "median cpu",
            "extra": "avg cpu: 4.972716954401995, max cpu: 9.248554, count: 53879"
          },
          {
            "name": "Custom Scan - Subscriber - mem",
            "value": 50.74609375,
            "unit": "median mem",
            "extra": "avg mem: 50.794781622942146, max mem: 56.65625, count: 53879"
          },
          {
            "name": "Delete values - Publisher - cpu",
            "value": 4.567079,
            "unit": "median cpu",
            "extra": "avg cpu: 4.418728278343918, max cpu: 4.5933013, count: 53879"
          },
          {
            "name": "Delete values - Publisher - mem",
            "value": 31.9140625,
            "unit": "median mem",
            "extra": "avg mem: 31.19727349129531, max mem: 32.2265625, count: 53879"
          },
          {
            "name": "Find by ctid - Subscriber - cpu",
            "value": 9.116809,
            "unit": "median cpu",
            "extra": "avg cpu: 7.864657515430778, max cpu: 18.461538, count: 53879"
          },
          {
            "name": "Find by ctid - Subscriber - mem",
            "value": 53.90625,
            "unit": "median mem",
            "extra": "avg mem: 53.543344229662765, max mem: 59.6015625, count: 53879"
          },
          {
            "name": "Index Only Scan - Subscriber - cpu",
            "value": 4.5757866,
            "unit": "median cpu",
            "extra": "avg cpu: 4.888852722647441, max cpu: 9.248554, count: 53879"
          },
          {
            "name": "Index Only Scan - Subscriber - mem",
            "value": 50.02734375,
            "unit": "median mem",
            "extra": "avg mem: 50.055396481583735, max mem: 55.8203125, count: 53879"
          },
          {
            "name": "Index Size Info - Subscriber - cpu",
            "value": 4.5757866,
            "unit": "median cpu",
            "extra": "avg cpu: 4.600459577181787, max cpu: 9.186603, count: 53879"
          },
          {
            "name": "Index Size Info - Subscriber - mem",
            "value": 33.30078125,
            "unit": "median mem",
            "extra": "avg mem: 33.3005254685267, max mem: 38.45703125, count: 53879"
          },
          {
            "name": "Index Size Info - Subscriber - pages",
            "value": 1107,
            "unit": "median pages",
            "extra": "avg pages: 1106.613504333785, max pages: 1823.0, count: 53879"
          },
          {
            "name": "Index Size Info - Subscriber - relation_size:MB",
            "value": 8.6484375,
            "unit": "median relation_size:MB",
            "extra": "avg relation_size:MB: 8.645418002607695, max relation_size:MB: 14.2421875, count: 53879"
          },
          {
            "name": "Index Size Info - Subscriber - segment_count",
            "value": 10,
            "unit": "median segment_count",
            "extra": "avg segment_count: 10.218341097644723, max segment_count: 18.0, count: 53879"
          },
          {
            "name": "Insert value A - Publisher - cpu",
            "value": 4.5411544,
            "unit": "median cpu",
            "extra": "avg cpu: 3.908182082396997, max cpu: 9.099526, count: 53879"
          },
          {
            "name": "Insert value A - Publisher - mem",
            "value": 29.6953125,
            "unit": "median mem",
            "extra": "avg mem: 29.006472402281037, max mem: 30.0234375, count: 53879"
          },
          {
            "name": "Insert value B - Publisher - cpu",
            "value": 4.58891,
            "unit": "median cpu",
            "extra": "avg cpu: 2.483831962918097, max cpu: 4.58891, count: 53879"
          },
          {
            "name": "Insert value B - Publisher - mem",
            "value": 29.58984375,
            "unit": "median mem",
            "extra": "avg mem: 28.921694618961006, max mem: 30.0546875, count: 53879"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - cpu",
            "value": 4.6153846,
            "unit": "median cpu",
            "extra": "avg cpu: 6.936550055720912, max cpu: 22.900763, count: 53879"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - mem",
            "value": 48.68359375,
            "unit": "median mem",
            "extra": "avg mem: 48.71947609168229, max mem: 54.578125, count: 53879"
          },
          {
            "name": "SELECT\n  pid,\n  pg_wal_lsn_diff(sent_lsn, replay_lsn) AS replication_lag,\n  application_name::text,\n  state::text\nFROM pg_stat_replication; - Publisher - replication_lag:MB",
            "value": 0,
            "unit": "median replication_lag:MB",
            "extra": "avg replication_lag:MB: 0.00002346521328462852, max replication_lag:MB: 0.25904083251953125, count: 53879"
          },
          {
            "name": "Top N - Subscriber - cpu",
            "value": 4.5757866,
            "unit": "median cpu",
            "extra": "avg cpu: 5.085834738271996, max cpu: 13.779904, count: 107758"
          },
          {
            "name": "Top N - Subscriber - mem",
            "value": 48.60546875,
            "unit": "median mem",
            "extra": "avg mem: 48.608167505544834, max mem: 54.62109375, count: 107758"
          },
          {
            "name": "Update 1..9 - Publisher - cpu",
            "value": 4.58891,
            "unit": "median cpu",
            "extra": "avg cpu: 4.388116330448976, max cpu: 4.610951, count: 53879"
          },
          {
            "name": "Update 1..9 - Publisher - mem",
            "value": 32.49609375,
            "unit": "median mem",
            "extra": "avg mem: 31.80449602639711, max mem: 32.83203125, count: 53879"
          },
          {
            "name": "Update 10,11 - Publisher - cpu",
            "value": 4.5714283,
            "unit": "median cpu",
            "extra": "avg cpu: 4.344481806112198, max cpu: 4.6021094, count: 53879"
          },
          {
            "name": "Update 10,11 - Publisher - mem",
            "value": 32.6015625,
            "unit": "median mem",
            "extra": "avg mem: 31.918721159333877, max mem: 32.70703125, count: 53879"
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
          "id": "1da48a637b80cc0ce2312ab8c4e448762e152223",
          "message": "perf: Add statistics to joinscan (#4132)\n\n## Ticket(s) Closed\n\n- Closes #4062.\n\n## What\n\n* Exposes sorting from the joinscan's `TableProvider`, but does not yet\nforce `SortMergeJoin`.\n* Adds statistics on `TableProvider` and our `ExecutionPlan`s using\nTantivy's query estimates.\n* Removes the `ParallelSegmentPlan` that was added in #4101, as it makes\nmore sense to let DataFusion coalesce for us if needed.\n\n## Why\n\nTo allow the DataFusion optimizer to re-order joins based on table\nsizes, and use sortedness in plans (although it does not yet by\ndefault).\n\n## Tests\n\nExisting tests show the impact of join reordering due to statistics.",
          "timestamp": "2026-02-12T14:34:01-08:00",
          "tree_id": "fbc185b154055782f4973f483feb5ad00a4ca2bb",
          "url": "https://github.com/paradedb/paradedb/commit/1da48a637b80cc0ce2312ab8c4e448762e152223"
        },
        "date": 1770940501474,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - cpu",
            "value": 4.5801525,
            "unit": "median cpu",
            "extra": "avg cpu: 4.95248572358839, max cpu: 9.266409, count: 53855"
          },
          {
            "name": "Custom Scan - Subscriber - mem",
            "value": 50.3984375,
            "unit": "median mem",
            "extra": "avg mem: 50.382436055148084, max mem: 56.14453125, count: 53855"
          },
          {
            "name": "Delete values - Publisher - cpu",
            "value": 4.5584044,
            "unit": "median cpu",
            "extra": "avg cpu: 2.949911115122017, max cpu: 4.6376815, count: 53855"
          },
          {
            "name": "Delete values - Publisher - mem",
            "value": 32.73828125,
            "unit": "median mem",
            "extra": "avg mem: 32.104560640260885, max mem: 33.44140625, count: 53855"
          },
          {
            "name": "Find by ctid - Subscriber - cpu",
            "value": 9.099526,
            "unit": "median cpu",
            "extra": "avg cpu: 7.412290850500884, max cpu: 18.408438, count: 53855"
          },
          {
            "name": "Find by ctid - Subscriber - mem",
            "value": 53.8203125,
            "unit": "median mem",
            "extra": "avg mem: 53.485513038482964, max mem: 59.5390625, count: 53855"
          },
          {
            "name": "Index Only Scan - Subscriber - cpu",
            "value": 4.5757866,
            "unit": "median cpu",
            "extra": "avg cpu: 4.866879905332011, max cpu: 9.257474, count: 53855"
          },
          {
            "name": "Index Only Scan - Subscriber - mem",
            "value": 49.97265625,
            "unit": "median mem",
            "extra": "avg mem: 49.96586341507288, max mem: 55.7109375, count: 53855"
          },
          {
            "name": "Index Size Info - Subscriber - cpu",
            "value": 4.5757866,
            "unit": "median cpu",
            "extra": "avg cpu: 4.611033445079684, max cpu: 9.221902, count: 53855"
          },
          {
            "name": "Index Size Info - Subscriber - mem",
            "value": 33.2890625,
            "unit": "median mem",
            "extra": "avg mem: 33.23825397769473, max mem: 38.3203125, count: 53855"
          },
          {
            "name": "Index Size Info - Subscriber - pages",
            "value": 1094,
            "unit": "median pages",
            "extra": "avg pages: 1089.4514715439607, max pages: 1805.0, count: 53855"
          },
          {
            "name": "Index Size Info - Subscriber - relation_size:MB",
            "value": 8.546875,
            "unit": "median relation_size:MB",
            "extra": "avg relation_size:MB: 8.511339766502646, max relation_size:MB: 14.1015625, count: 53855"
          },
          {
            "name": "Index Size Info - Subscriber - segment_count",
            "value": 6,
            "unit": "median segment_count",
            "extra": "avg segment_count: 7.07445919598923, max segment_count: 15.0, count: 53855"
          },
          {
            "name": "Insert value A - Publisher - cpu",
            "value": 4.567079,
            "unit": "median cpu",
            "extra": "avg cpu: 4.112791035505793, max cpu: 4.624277, count: 53855"
          },
          {
            "name": "Insert value A - Publisher - mem",
            "value": 29.875,
            "unit": "median mem",
            "extra": "avg mem: 29.260181926585275, max mem: 30.40625, count: 53855"
          },
          {
            "name": "Insert value B - Publisher - cpu",
            "value": 4.5757866,
            "unit": "median cpu",
            "extra": "avg cpu: 4.422100767812052, max cpu: 4.58891, count: 53855"
          },
          {
            "name": "Insert value B - Publisher - mem",
            "value": 29.82421875,
            "unit": "median mem",
            "extra": "avg mem: 29.177288697660384, max mem: 30.24609375, count: 53855"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - cpu",
            "value": 4.6065254,
            "unit": "median cpu",
            "extra": "avg cpu: 6.837668786965338, max cpu: 23.032629, count: 53855"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - mem",
            "value": 48.22265625,
            "unit": "median mem",
            "extra": "avg mem: 48.2182139831492, max mem: 54.0, count: 53855"
          },
          {
            "name": "SELECT\n  pid,\n  pg_wal_lsn_diff(sent_lsn, replay_lsn) AS replication_lag,\n  application_name::text,\n  state::text\nFROM pg_stat_replication; - Publisher - replication_lag:MB",
            "value": 0,
            "unit": "median replication_lag:MB",
            "extra": "avg replication_lag:MB: 0.000029518836477143486, max replication_lag:MB: 0.21288299560546875, count: 53855"
          },
          {
            "name": "Top N - Subscriber - cpu",
            "value": 4.5801525,
            "unit": "median cpu",
            "extra": "avg cpu: 5.096477664505018, max cpu: 13.793103, count: 107710"
          },
          {
            "name": "Top N - Subscriber - mem",
            "value": 48.48828125,
            "unit": "median mem",
            "extra": "avg mem: 48.46664201762835, max mem: 54.5625, count: 107710"
          },
          {
            "name": "Update 1..9 - Publisher - cpu",
            "value": 4.567079,
            "unit": "median cpu",
            "extra": "avg cpu: 4.462231467436292, max cpu: 4.6021094, count: 53855"
          },
          {
            "name": "Update 1..9 - Publisher - mem",
            "value": 32.96875,
            "unit": "median mem",
            "extra": "avg mem: 32.28520825596509, max mem: 32.984375, count: 53855"
          },
          {
            "name": "Update 10,11 - Publisher - cpu",
            "value": 4.562738,
            "unit": "median cpu",
            "extra": "avg cpu: 4.363557240555551, max cpu: 4.597701, count: 53855"
          },
          {
            "name": "Update 10,11 - Publisher - mem",
            "value": 33.28515625,
            "unit": "median mem",
            "extra": "avg mem: 32.64644698437935, max mem: 33.70703125, count: 53855"
          }
        ]
      }
    ]
  }
}