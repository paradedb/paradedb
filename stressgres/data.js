window.BENCHMARK_DATA = {
  "lastUpdate": 1770832075446,
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
      }
    ]
  }
}