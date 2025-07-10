window.BENCHMARK_DATA = {
  "lastUpdate": 1752168391853,
  "repoUrl": "https://github.com/paradedb/paradedb",
  "entries": {
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
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752168057334,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 2.536868810038378,
            "unit": "median tps",
            "extra": "avg tps: 3.650278702049354, max tps: 6.809720188752096, count: 21239"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 4.85396547468231,
            "unit": "median tps",
            "extra": "avg tps: 5.55575596198306, max tps: 7.042050255753952, count: 21239"
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
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752168066344,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 24.096386,
            "unit": "median cpu",
            "extra": "avg cpu: 21.7662600606495, max cpu: 44.17178, count: 21239"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 226.39453125,
            "unit": "median mem",
            "extra": "avg mem: 224.5520188391638, max mem: 231.8671875, count: 21239"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 24.096386,
            "unit": "median cpu",
            "extra": "avg cpu: 22.225273188478045, max cpu: 33.939396, count: 21239"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 161.99609375,
            "unit": "median mem",
            "extra": "avg mem: 161.98778595684826, max mem: 163.625, count: 21239"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 18803,
            "unit": "median block_count",
            "extra": "avg block_count: 19348.14492207731, max block_count: 22835.0, count: 21239"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 24,
            "unit": "median segment_count",
            "extra": "avg segment_count: 26.24826969254673, max segment_count: 59.0, count: 21239"
          }
        ]
      }
    ],
    "pg_search single-server.toml Performance - TPS": [
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
          "id": "70f65d99d8f6cd7c112b8c4aa00d8c410b55efbe",
          "message": "feat: Added a GUC (`enable_custom_scan_for_non_indexed_fields`) for handling non-indexed fields in queries (#2767)\n\n## What\n\n- Disabled using bm25 index for handling queries that require\nHeapFilter, but do not use the `@@@` operator.\n- Added a new GUC `paradedb.enable_filter_pushdown` to control whether\nParadeDB's custom scan should handle queries that include non-indexed\nfield predicates.\n\n## Why\n\nThis GUC provides users with fine-grained control over when ParadeDB's\ncustom scan is used, particularly for queries that mix indexed and\nnon-indexed predicates. This is useful for:\n\n- **Performance tuning**: Users can compare custom scan performance\nagainst standard PostgreSQL execution\n- **Debugging**: Helps isolate issues related to HeapExpr filtering vs\nstandard execution\n- **Backward compatibility**: Allows disabling the feature if it causes\nissues in specific scenarios\n\n## How\n\n1. **Added GUC definition** in `src/gucs.rs`.\n2. **Integrated GUC checks** in\n`src/postgres/customscan/pdbscan/qual_inspect.rs`\n\n## Tests\n\nAdded a regression test (Test Case 19) in\n`score_non_indexed_predicates.sql`\n\nThe test shows that users can control execution strategy with:\n```sql\nSET paradedb.enable_filter_pushdown = false; -- Disable HeapExpr\nSET paradedb.enable_filter_pushdown = true;  -- Enable HeapExpr (default)\n```",
          "timestamp": "2025-07-05T18:06:46Z",
          "url": "https://github.com/paradedb/paradedb/commit/70f65d99d8f6cd7c112b8c4aa00d8c410b55efbe"
        },
        "date": 1752168362644,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 439.7556089628702,
            "unit": "median tps",
            "extra": "avg tps: 441.30051468238287, max tps: 611.3181645734521, count: 58468"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2725.6829093015035,
            "unit": "median tps",
            "extra": "avg tps: 2728.048010559734, max tps: 2919.859012613137, count: 58468"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 439.53250327714164,
            "unit": "median tps",
            "extra": "avg tps: 441.1274851517368, max tps: 616.2216612116403, count: 58468"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 393.8081740222367,
            "unit": "median tps",
            "extra": "avg tps: 394.0111507703895, max tps: 487.77463602134713, count: 58468"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 290.23996597527207,
            "unit": "median tps",
            "extra": "avg tps: 293.5999904956123, max tps: 338.6579239144033, count: 116936"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 250.93976299669234,
            "unit": "median tps",
            "extra": "avg tps: 251.80889078572957, max tps: 277.51842536940194, count: 58468"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 14.452775520804673,
            "unit": "median tps",
            "extra": "avg tps: 24.291048794026157, max tps: 1741.8719898274674, count: 58468"
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
            "name": "Philippe Noël",
            "username": "philippemnoel",
            "email": "philippemnoel@gmail.com"
          },
          "id": "ce8e33ae49785f0afe220ca985de3d0c7c270503",
          "message": "chore: more improvements to index/schema configuration and management (#2771)\n\n## What\n\n#2660 brought a much needed round of cleanups to how we manage index\nschemas. Unfortunately, it introduced quite some overhead in\nreading/decoding/validating the schema. This process was happening quite\na bit throughout the execution paths of `aminsert` and other hot-spots.\n\n#2176 brought the ability to essentially keep one heavy-weight\n`PgSearchRelation` instantiated and cheaply clone it when necessary.\nThis PR cleans up things further such that the `SearchIndexSchema` is\nnow a lazily-evaluated property of `PgSearchRelation`. This means\n`SearchIndexSchema` is only evaluated when needed, and then only once\n(at least per statement).\n\nFurthermore, its internal properties are lazily-evaluated, ensuring any\ngiven code path doesn't do more work than it needs.\n\nThis also renames `SearchIndexOptions` to `BM25IndexOptions`, mainly\nbecause I kept getting confused about what `SearchIndexOptions`\nrepresented (it was too similarly named to `SearchIndexSchema` for my\ntastes). And `BM25IndexOptions` is now a property of `PgSearchRelation`\ntoo.\n\nThis seems to have drastically improved the write throughput of the\nINSERT/UPDATE jobs in our `single-server.toml` stressgress test.\nv0.15.26 was 176/s INSERTs and 154/s UPDATEs. This PR clocks in at 275/s\nand 260/s, respectively.\n\n# Other Notable Changes\n\n- Index configuration validation now happens during CREATE INDEX/REINDEX\nin `ambuildempty()` rather than on every instantiation of\n`SearchIndexSchema`.\n\n- The \"raw\" tokenizer deprecation warnings are now gone, unless somehow\nthe \"key_field\" is configured with it -- which is no longer possible\n\n## Why\n\nTrying to rollback performance regressions that were introduced in\n0.16.0\n\n## How\n\n## Tests\n\nAll existing tests pass, and a few were updated due to the \"raw\"\ntokenizer deprecation warning going away and a change in wording for a\nspecific validation error.",
          "timestamp": "2025-07-05T15:13:47Z",
          "url": "https://github.com/paradedb/paradedb/commit/ce8e33ae49785f0afe220ca985de3d0c7c270503"
        },
        "date": 1752168384236,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 464.8811990087773,
            "unit": "median tps",
            "extra": "avg tps: 469.85646322826295, max tps: 655.8886631256029, count: 58439"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2915.5618380316473,
            "unit": "median tps",
            "extra": "avg tps: 2932.1736560091085, max tps: 3039.753736522289, count: 58439"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 464.6816349695032,
            "unit": "median tps",
            "extra": "avg tps: 469.6128652393644, max tps: 653.5878111282227, count: 58439"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 457.92124424136614,
            "unit": "median tps",
            "extra": "avg tps: 461.806744246927, max tps: 590.0543971148701, count: 58439"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 317.96401099412475,
            "unit": "median tps",
            "extra": "avg tps: 317.78394137019785, max tps: 374.57464243615743, count: 116878"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 292.67575858066374,
            "unit": "median tps",
            "extra": "avg tps: 293.53381603010575, max tps: 336.7877280876234, count: 58439"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 16.53754131439579,
            "unit": "median tps",
            "extra": "avg tps: 24.487470245813206, max tps: 1773.7604075391914, count: 58439"
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
            "name": "Philippe Noël",
            "username": "philippemnoel",
            "email": "philippemnoel@gmail.com"
          },
          "id": "f49e7767f467152f4b210cae02c4ab6e6845b365",
          "message": "perf: the BAS_BULKWRITE strategy allocation now happens once (#2772)\n\n## What\n\nAllocating the `BAS_BULKWRITE` buffer access strategy is very very very\nexpensive. This moves its allocation up as a Lazy global so it only ever\nneeds to happen once per backend.\n\nLocally, this improved the stressgres \"single-server.toml\" select\nqueries from 296/s, 295/s, 263/s (Custom Scan, Index Only, Index Scan)\nto 523/s, 523/s, 466/s.\n\nAdditionally, the Insert and Update jobs moved from 255/s & 232/s to\n261/s & 238/s\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-07-05T15:13:02Z",
          "url": "https://github.com/paradedb/paradedb/commit/f49e7767f467152f4b210cae02c4ab6e6845b365"
        },
        "date": 1752168389144,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 489.00370402604835,
            "unit": "median tps",
            "extra": "avg tps: 495.11692609469424, max tps: 686.2278556250494, count: 58465"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3006.829399351906,
            "unit": "median tps",
            "extra": "avg tps: 3029.652368555175, max tps: 3266.3836198835943, count: 58465"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 488.80428857117823,
            "unit": "median tps",
            "extra": "avg tps: 494.86594765763675, max tps: 685.9438100509728, count: 58465"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 464.9860900052875,
            "unit": "median tps",
            "extra": "avg tps: 466.0473400643971, max tps: 581.0831855445098, count: 58465"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 301.2592833393006,
            "unit": "median tps",
            "extra": "avg tps: 300.8627059740218, max tps: 316.94699900390765, count: 116930"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 272.1667053933685,
            "unit": "median tps",
            "extra": "avg tps: 271.4938587751168, max tps: 277.205122390403, count: 58465"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 12.19658861548239,
            "unit": "median tps",
            "extra": "avg tps: 22.81999270595224, max tps: 1846.7050165741778, count: 58465"
          }
        ]
      }
    ],
    "pg_search wide-table.toml Performance - TPS": [
      {
        "commit": {
          "author": {
            "name": "Kaihong.Wang",
            "username": "wangkhc",
            "email": "wangkhc@163.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "7f560910072d570e0dca4d19a9fe02b47f6917e5",
          "message": "fix: Add missing stopword filters to Jieba tokenizer (#2790)\n\n### What\n\nThis PR fixes a bug where the Jieba tokenizer was missing stopword\nfiltering capabilities that are available in other tokenizers. The fix\nadds both custom stopword lists and language-based stopword filtering\nsupport to the Jieba tokenizer. (Fix #2789 )\n\n### Why\n\nThe Jieba tokenizer implementation was inconsistent with other\ntokenizers in the codebase - it lacked the\n`.filter(filters.stopwords_language())` and\n`.filter(filters.stopwords())` calls that are present in all other\ntokenizer variants (ICU, Chinese Lindera, etc.). This meant users\ncouldn't filter out common Chinese stop words like \"的\", \"了\", \"在\" or\nEnglish stop words when using mixed-language content, reducing search\nquality and relevance.\n\nThis inconsistency was discovered when comparing the Jieba tokenizer\nimplementation against other tokenizer variants in\n`tokenizers/src/manager.rs`.\n\n### How\n\n1. **Bug Fix:** Modified `tokenizers/src/manager.rs` in the\n`SearchTokenizer::Jieba` case within `to_tantivy_tokenizer()` method:\n- Added `.filter(filters.stopwords_language())` to support\nlanguage-based stopwords (e.g., English, Spanish, etc.)\n- Added `.filter(filters.stopwords())` to support custom stopword lists\n- This brings Jieba tokenizer in line with all other tokenizer\nimplementations\n\n2. **Code Changes:**\n   ```rust\n   // Before (missing stopword filters)\n   SearchTokenizer::Jieba(filters) => Some(\n       TextAnalyzer::builder(tantivy_jieba::JiebaTokenizer {})\n           .filter(filters.remove_long_filter())\n           .filter(filters.lower_caser())\n           .filter(filters.stemmer())\n           .build(),\n   ),\n\n   // After (with stopword filters added)\n   SearchTokenizer::Jieba(filters) => Some(\n       TextAnalyzer::builder(tantivy_jieba::JiebaTokenizer {})\n           .filter(filters.remove_long_filter())\n           .filter(filters.lower_caser())\n           .filter(filters.stemmer())\n           .filter(filters.stopwords_language())  // ← Added\n           .filter(filters.stopwords())           // ← Added\n           .build(),\n   ),\n   ```\n\n### Tests\n\nAdded comprehensive test coverage in `tokenizers/src/manager.rs`:\n\n1. **`test_jieba_tokenizer_with_stopwords`**: \n   - Tests custom stopword filtering with Chinese stopwords\n- Verifies stopwords are filtered out while content words are preserved\n\n2. **`test_jieba_tokenizer_with_language_stopwords`**:\n   - Tests language-based stopword filtering with English stopwords\n   - Tests the `stopwords_language: \"English\"` configuration option\n\nBoth tests use natural, conversational sentences instead of artificial\ntest data, making them more representative of real-world usage and\nsuitable for open-source community review.\n\n**All existing tests continue to pass** (12/12), ensuring no regressions\nwere introduced.\n\n### Ticket(s) Closed\n\nFix #2789\n\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>",
          "timestamp": "2025-07-09T12:38:14Z",
          "url": "https://github.com/paradedb/paradedb/commit/7f560910072d570e0dca4d19a9fe02b47f6917e5"
        },
        "date": 1752168363867,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 39.362759716853304,
            "unit": "median tps",
            "extra": "avg tps: 39.037484090953704, max tps: 40.01649936825948, count: 59107"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 214.23461762497593,
            "unit": "median tps",
            "extra": "avg tps: 205.54215563271296, max tps: 227.7281544867555, count: 59107"
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
          "id": "c0442237441f33c1c51d6c11e29849eda05816a7",
          "message": "chore: Upgrade to `0.16.2` (#2760)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-07-01T21:30:02Z",
          "url": "https://github.com/paradedb/paradedb/commit/c0442237441f33c1c51d6c11e29849eda05816a7"
        },
        "date": 1752168367681,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 21.939813678696908,
            "unit": "median tps",
            "extra": "avg tps: 20.13986664585194, max tps: 31.68734236180922, count: 59086"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 114.00569283896101,
            "unit": "median tps",
            "extra": "avg tps: 114.00197638656465, max tps: 117.56716921136099, count: 59086"
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
            "name": "Philippe Noël",
            "username": "philippemnoel",
            "email": "philippemnoel@gmail.com"
          },
          "id": "f49e7767f467152f4b210cae02c4ab6e6845b365",
          "message": "perf: the BAS_BULKWRITE strategy allocation now happens once (#2772)\n\n## What\n\nAllocating the `BAS_BULKWRITE` buffer access strategy is very very very\nexpensive. This moves its allocation up as a Lazy global so it only ever\nneeds to happen once per backend.\n\nLocally, this improved the stressgres \"single-server.toml\" select\nqueries from 296/s, 295/s, 263/s (Custom Scan, Index Only, Index Scan)\nto 523/s, 523/s, 466/s.\n\nAdditionally, the Insert and Update jobs moved from 255/s & 232/s to\n261/s & 238/s\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-07-05T15:13:02Z",
          "url": "https://github.com/paradedb/paradedb/commit/f49e7767f467152f4b210cae02c4ab6e6845b365"
        },
        "date": 1752168371354,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 33.64293991682778,
            "unit": "median tps",
            "extra": "avg tps: 33.45647586687845, max tps: 34.077285549956244, count: 59121"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 113.8187993419052,
            "unit": "median tps",
            "extra": "avg tps: 112.4258537579966, max tps: 119.34550335840962, count: 59121"
          }
        ]
      }
    ],
    "pg_search wide-table.toml Performance - Other Metrics": [
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
          "id": "c0442237441f33c1c51d6c11e29849eda05816a7",
          "message": "chore: Upgrade to `0.16.2` (#2760)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-07-01T21:30:02Z",
          "url": "https://github.com/paradedb/paradedb/commit/c0442237441f33c1c51d6c11e29849eda05816a7"
        },
        "date": 1752168385788,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 19.6319,
            "unit": "median cpu",
            "extra": "avg cpu: 21.57498978820386, max cpu: 49.68944, count: 59086"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 177.5390625,
            "unit": "median mem",
            "extra": "avg mem: 175.2446666061969, max mem: 179.390625, count: 59086"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 9127,
            "unit": "median block_count",
            "extra": "avg block_count: 7908.791118031344, max block_count: 9224.0, count: 59086"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 43,
            "unit": "median segment_count",
            "extra": "avg segment_count: 42.94988660596419, max segment_count: 94.0, count: 59086"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 14.545454,
            "unit": "median cpu",
            "extra": "avg cpu: 13.351249740605763, max cpu: 34.5679, count: 59086"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 170.79296875,
            "unit": "median mem",
            "extra": "avg mem: 163.69383425219172, max mem: 177.265625, count: 59086"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Kaihong.Wang",
            "username": "wangkhc",
            "email": "wangkhc@163.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "7f560910072d570e0dca4d19a9fe02b47f6917e5",
          "message": "fix: Add missing stopword filters to Jieba tokenizer (#2790)\n\n### What\n\nThis PR fixes a bug where the Jieba tokenizer was missing stopword\nfiltering capabilities that are available in other tokenizers. The fix\nadds both custom stopword lists and language-based stopword filtering\nsupport to the Jieba tokenizer. (Fix #2789 )\n\n### Why\n\nThe Jieba tokenizer implementation was inconsistent with other\ntokenizers in the codebase - it lacked the\n`.filter(filters.stopwords_language())` and\n`.filter(filters.stopwords())` calls that are present in all other\ntokenizer variants (ICU, Chinese Lindera, etc.). This meant users\ncouldn't filter out common Chinese stop words like \"的\", \"了\", \"在\" or\nEnglish stop words when using mixed-language content, reducing search\nquality and relevance.\n\nThis inconsistency was discovered when comparing the Jieba tokenizer\nimplementation against other tokenizer variants in\n`tokenizers/src/manager.rs`.\n\n### How\n\n1. **Bug Fix:** Modified `tokenizers/src/manager.rs` in the\n`SearchTokenizer::Jieba` case within `to_tantivy_tokenizer()` method:\n- Added `.filter(filters.stopwords_language())` to support\nlanguage-based stopwords (e.g., English, Spanish, etc.)\n- Added `.filter(filters.stopwords())` to support custom stopword lists\n- This brings Jieba tokenizer in line with all other tokenizer\nimplementations\n\n2. **Code Changes:**\n   ```rust\n   // Before (missing stopword filters)\n   SearchTokenizer::Jieba(filters) => Some(\n       TextAnalyzer::builder(tantivy_jieba::JiebaTokenizer {})\n           .filter(filters.remove_long_filter())\n           .filter(filters.lower_caser())\n           .filter(filters.stemmer())\n           .build(),\n   ),\n\n   // After (with stopword filters added)\n   SearchTokenizer::Jieba(filters) => Some(\n       TextAnalyzer::builder(tantivy_jieba::JiebaTokenizer {})\n           .filter(filters.remove_long_filter())\n           .filter(filters.lower_caser())\n           .filter(filters.stemmer())\n           .filter(filters.stopwords_language())  // ← Added\n           .filter(filters.stopwords())           // ← Added\n           .build(),\n   ),\n   ```\n\n### Tests\n\nAdded comprehensive test coverage in `tokenizers/src/manager.rs`:\n\n1. **`test_jieba_tokenizer_with_stopwords`**: \n   - Tests custom stopword filtering with Chinese stopwords\n- Verifies stopwords are filtered out while content words are preserved\n\n2. **`test_jieba_tokenizer_with_language_stopwords`**:\n   - Tests language-based stopword filtering with English stopwords\n   - Tests the `stopwords_language: \"English\"` configuration option\n\nBoth tests use natural, conversational sentences instead of artificial\ntest data, making them more representative of real-world usage and\nsuitable for open-source community review.\n\n**All existing tests continue to pass** (12/12), ensuring no regressions\nwere introduced.\n\n### Ticket(s) Closed\n\nFix #2789\n\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>",
          "timestamp": "2025-07-09T12:38:14Z",
          "url": "https://github.com/paradedb/paradedb/commit/7f560910072d570e0dca4d19a9fe02b47f6917e5"
        },
        "date": 1752168384703,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 19.512194,
            "unit": "median cpu",
            "extra": "avg cpu: 19.456504671132954, max cpu: 48.780487, count: 59107"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 182.21484375,
            "unit": "median mem",
            "extra": "avg mem: 181.41578501171605, max mem: 182.21484375, count: 59107"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 20721,
            "unit": "median block_count",
            "extra": "avg block_count: 18589.107550713114, max block_count: 20721.0, count: 59107"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 51,
            "unit": "median segment_count",
            "extra": "avg segment_count: 54.156749623564046, max segment_count: 152.0, count: 59107"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 9.756097,
            "unit": "median cpu",
            "extra": "avg cpu: 8.69461182997757, max cpu: 34.782608, count: 59107"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 168.953125,
            "unit": "median mem",
            "extra": "avg mem: 159.16871481486965, max mem: 174.875, count: 59107"
          }
        ]
      }
    ],
    "pg_search single-server.toml Performance - Other Metrics": [
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
          "id": "70f65d99d8f6cd7c112b8c4aa00d8c410b55efbe",
          "message": "feat: Added a GUC (`enable_custom_scan_for_non_indexed_fields`) for handling non-indexed fields in queries (#2767)\n\n## What\n\n- Disabled using bm25 index for handling queries that require\nHeapFilter, but do not use the `@@@` operator.\n- Added a new GUC `paradedb.enable_filter_pushdown` to control whether\nParadeDB's custom scan should handle queries that include non-indexed\nfield predicates.\n\n## Why\n\nThis GUC provides users with fine-grained control over when ParadeDB's\ncustom scan is used, particularly for queries that mix indexed and\nnon-indexed predicates. This is useful for:\n\n- **Performance tuning**: Users can compare custom scan performance\nagainst standard PostgreSQL execution\n- **Debugging**: Helps isolate issues related to HeapExpr filtering vs\nstandard execution\n- **Backward compatibility**: Allows disabling the feature if it causes\nissues in specific scenarios\n\n## How\n\n1. **Added GUC definition** in `src/gucs.rs`.\n2. **Integrated GUC checks** in\n`src/postgres/customscan/pdbscan/qual_inspect.rs`\n\n## Tests\n\nAdded a regression test (Test Case 19) in\n`score_non_indexed_predicates.sql`\n\nThe test shows that users can control execution strategy with:\n```sql\nSET paradedb.enable_filter_pushdown = false; -- Disable HeapExpr\nSET paradedb.enable_filter_pushdown = true;  -- Enable HeapExpr (default)\n```",
          "timestamp": "2025-07-05T18:06:46Z",
          "url": "https://github.com/paradedb/paradedb/commit/70f65d99d8f6cd7c112b8c4aa00d8c410b55efbe"
        },
        "date": 1752168388444,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.9382715,
            "unit": "median cpu",
            "extra": "avg cpu: 7.26708050909844, max cpu: 35.220127, count: 58468"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 104.21875,
            "unit": "median mem",
            "extra": "avg mem: 102.5424432889572, max mem: 108.953125, count: 58468"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.8780484,
            "unit": "median cpu",
            "extra": "avg cpu: 4.743104658194739, max cpu: 9.81595, count: 58468"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 91.18359375,
            "unit": "median mem",
            "extra": "avg mem: 87.71848857227458, max mem: 92.18359375, count: 58468"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.9382715,
            "unit": "median cpu",
            "extra": "avg cpu: 7.386868311354364, max cpu: 35.220127, count: 58468"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 106.82421875,
            "unit": "median mem",
            "extra": "avg mem: 104.12835954336133, max mem: 109.82421875, count: 58468"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.8780484,
            "unit": "median cpu",
            "extra": "avg cpu: 4.660955811468024, max cpu: 9.937888, count: 58468"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 104.01171875,
            "unit": "median mem",
            "extra": "avg mem: 101.12013709689488, max mem: 104.88671875, count: 58468"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.9382715,
            "unit": "median cpu",
            "extra": "avg cpu: 7.193565028314803, max cpu: 25.157234, count: 116936"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 112.09765625,
            "unit": "median mem",
            "extra": "avg mem: 112.0013559433686, max mem: 122.30078125, count: 116936"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 7784,
            "unit": "median block_count",
            "extra": "avg block_count: 7618.652134500923, max block_count: 8569.0, count: 58468"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 118,
            "unit": "median segment_count",
            "extra": "avg segment_count: 117.73371758910858, max segment_count: 386.0, count: 58468"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.907975,
            "unit": "median cpu",
            "extra": "avg cpu: 5.934068022345629, max cpu: 19.875776, count: 58468"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 115.71484375,
            "unit": "median mem",
            "extra": "avg mem: 114.98278959751488, max mem: 122.2421875, count: 58468"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 14.634146,
            "unit": "median cpu",
            "extra": "avg cpu: 14.153493186323573, max cpu: 29.447853, count: 58468"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 91.09375,
            "unit": "median mem",
            "extra": "avg mem: 84.29432091860077, max mem: 99.96875, count: 58468"
          }
        ]
      }
    ]
  }
}