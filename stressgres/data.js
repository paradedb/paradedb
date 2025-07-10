window.BENCHMARK_DATA = {
  "lastUpdate": 1752168461759,
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
        "date": 1752168423119,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.488810080150678,
            "unit": "median tps",
            "extra": "avg tps: 7.359979450056673, max tps: 10.741316249035114, count: 59042"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 7.817110977032365,
            "unit": "median tps",
            "extra": "avg tps: 7.176698272928818, max tps: 8.610738000998492, count: 59042"
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
          "id": "71ea95206a8e487805333d573e859dad68dab572",
          "message": "chore: Upgrade to `0.16.1` (#2748)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-06-30T19:38:06Z",
          "url": "https://github.com/paradedb/paradedb/commit/71ea95206a8e487805333d573e859dad68dab572"
        },
        "date": 1752168435292,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 5.035859308640268,
            "unit": "median tps",
            "extra": "avg tps: 5.137388238588778, max tps: 11.33778637917294, count: 59045"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 8.6100112686324,
            "unit": "median tps",
            "extra": "avg tps: 7.849335890485628, max tps: 10.205965325180708, count: 59045"
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
        "date": 1752168449478,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.638267249465038,
            "unit": "median tps",
            "extra": "avg tps: 7.459476817194935, max tps: 10.900573512286988, count: 59037"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 7.841499690996756,
            "unit": "median tps",
            "extra": "avg tps: 7.201536259804265, max tps: 8.605735843450596, count: 59037"
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
        "date": 1752168452647,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.372313520328255,
            "unit": "median tps",
            "extra": "avg tps: 7.260753982605406, max tps: 10.540485596378348, count: 59034"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 7.801591106820453,
            "unit": "median tps",
            "extra": "avg tps: 7.165385608920849, max tps: 8.55764919606473, count: 59034"
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
        "date": 1752168449066,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 5.009933378242491,
            "unit": "median tps",
            "extra": "avg tps: 5.116213655222489, max tps: 11.390353426016183, count: 59049"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 8.721946897992106,
            "unit": "median tps",
            "extra": "avg tps: 7.9188473675944, max tps: 10.23957477170576, count: 59049"
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
          "id": "70f65d99d8f6cd7c112b8c4aa00d8c410b55efbe",
          "message": "feat: Added a GUC (`enable_custom_scan_for_non_indexed_fields`) for handling non-indexed fields in queries (#2767)\n\n## What\n\n- Disabled using bm25 index for handling queries that require\nHeapFilter, but do not use the `@@@` operator.\n- Added a new GUC `paradedb.enable_filter_pushdown` to control whether\nParadeDB's custom scan should handle queries that include non-indexed\nfield predicates.\n\n## Why\n\nThis GUC provides users with fine-grained control over when ParadeDB's\ncustom scan is used, particularly for queries that mix indexed and\nnon-indexed predicates. This is useful for:\n\n- **Performance tuning**: Users can compare custom scan performance\nagainst standard PostgreSQL execution\n- **Debugging**: Helps isolate issues related to HeapExpr filtering vs\nstandard execution\n- **Backward compatibility**: Allows disabling the feature if it causes\nissues in specific scenarios\n\n## How\n\n1. **Added GUC definition** in `src/gucs.rs`.\n2. **Integrated GUC checks** in\n`src/postgres/customscan/pdbscan/qual_inspect.rs`\n\n## Tests\n\nAdded a regression test (Test Case 19) in\n`score_non_indexed_predicates.sql`\n\nThe test shows that users can control execution strategy with:\n```sql\nSET paradedb.enable_filter_pushdown = false; -- Disable HeapExpr\nSET paradedb.enable_filter_pushdown = true;  -- Enable HeapExpr (default)\n```",
          "timestamp": "2025-07-05T18:06:46Z",
          "url": "https://github.com/paradedb/paradedb/commit/70f65d99d8f6cd7c112b8c4aa00d8c410b55efbe"
        },
        "date": 1752168453021,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 5.043178258028606,
            "unit": "median tps",
            "extra": "avg tps: 5.138149415179151, max tps: 11.46623948860205, count: 59050"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 8.691786300816075,
            "unit": "median tps",
            "extra": "avg tps: 7.9078815265946245, max tps: 10.217018689798167, count: 59050"
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
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752168396172,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 490.9349786600615,
            "unit": "median tps",
            "extra": "avg tps: 495.2611708916468, max tps: 687.4984373366182, count: 58441"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3033.998123040645,
            "unit": "median tps",
            "extra": "avg tps: 3041.7463907182714, max tps: 3359.897952595054, count: 58441"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 490.90930007903506,
            "unit": "median tps",
            "extra": "avg tps: 495.30384402996367, max tps: 684.3231234818847, count: 58441"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 448.959070592096,
            "unit": "median tps",
            "extra": "avg tps: 449.1509057707429, max tps: 573.1091238743278, count: 58441"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 318.68385415756745,
            "unit": "median tps",
            "extra": "avg tps: 317.9303546950356, max tps: 321.02263890305517, count: 116882"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 285.07707154415635,
            "unit": "median tps",
            "extra": "avg tps: 285.0212913794637, max tps: 292.99267010470777, count: 58441"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 14.977807176975658,
            "unit": "median tps",
            "extra": "avg tps: 24.34945664817041, max tps: 1828.8155493213267, count: 58441"
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
        "date": 1752168397646,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 439.02973701894905,
            "unit": "median tps",
            "extra": "avg tps: 440.76415788751166, max tps: 615.1113298553976, count: 58480"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2831.416826304331,
            "unit": "median tps",
            "extra": "avg tps: 2831.6215491456296, max tps: 3099.2303553643155, count: 58480"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 438.7231403336101,
            "unit": "median tps",
            "extra": "avg tps: 440.53698700703717, max tps: 611.6638726392846, count: 58480"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 394.5647918015826,
            "unit": "median tps",
            "extra": "avg tps: 395.6096990819845, max tps: 500.3767336427597, count: 58480"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 291.41817294048724,
            "unit": "median tps",
            "extra": "avg tps: 291.54566703209036, max tps: 334.702720682826, count: 116960"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 257.65231520141157,
            "unit": "median tps",
            "extra": "avg tps: 261.82806314314354, max tps: 293.5829136131299, count: 58480"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 14.067382962938451,
            "unit": "median tps",
            "extra": "avg tps: 22.91145821903043, max tps: 1840.5529020917882, count: 58480"
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
          "id": "71ea95206a8e487805333d573e859dad68dab572",
          "message": "chore: Upgrade to `0.16.1` (#2748)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-06-30T19:38:06Z",
          "url": "https://github.com/paradedb/paradedb/commit/71ea95206a8e487805333d573e859dad68dab572"
        },
        "date": 1752168399250,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 442.1353642196263,
            "unit": "median tps",
            "extra": "avg tps: 445.5393071752972, max tps: 615.4819134003247, count: 58442"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2788.9381821035677,
            "unit": "median tps",
            "extra": "avg tps: 2812.925141515269, max tps: 3072.5780771838363, count: 58442"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 441.5458088216756,
            "unit": "median tps",
            "extra": "avg tps: 445.118735127726, max tps: 615.4073875161722, count: 58442"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 393.81941249612834,
            "unit": "median tps",
            "extra": "avg tps: 392.66968112986547, max tps: 486.8755632237375, count: 58442"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 297.1271639125962,
            "unit": "median tps",
            "extra": "avg tps: 300.4445606401976, max tps: 333.4672196140227, count: 116884"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 262.4576652437779,
            "unit": "median tps",
            "extra": "avg tps: 265.32119434159534, max tps: 287.3724532822702, count: 58442"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 12.423442033472217,
            "unit": "median tps",
            "extra": "avg tps: 14.774338414049655, max tps: 606.3457723147371, count: 58442"
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
        "date": 1752168402110,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 491.21032426791163,
            "unit": "median tps",
            "extra": "avg tps: 495.8638916578647, max tps: 664.1487832058575, count: 58420"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3044.35827719763,
            "unit": "median tps",
            "extra": "avg tps: 3057.449268914986, max tps: 3151.084122481109, count: 58420"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 491.18122169521814,
            "unit": "median tps",
            "extra": "avg tps: 495.7294836593001, max tps: 663.4695517922291, count: 58420"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 447.15279740439126,
            "unit": "median tps",
            "extra": "avg tps: 448.40938371346465, max tps: 530.5595397629438, count: 58420"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 313.42306584682564,
            "unit": "median tps",
            "extra": "avg tps: 313.75518429518434, max tps: 368.3453591039659, count: 116840"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 282.280689527415,
            "unit": "median tps",
            "extra": "avg tps: 281.80553399431574, max tps: 301.84306035233465, count: 58420"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 13.8138853679661,
            "unit": "median tps",
            "extra": "avg tps: 21.31709094365533, max tps: 1579.5767050345848, count: 58420"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Philippe Noël",
            "username": "philippemnoel",
            "email": "philippemnoel@gmail.com"
          },
          "committer": {
            "name": "Philippe Noël",
            "username": "philippemnoel",
            "email": "philippemnoel@gmail.com"
          },
          "id": "6603901ab5f5342e3de4b0bfc99065823a606d92",
          "message": "Fix mintlify check workflow",
          "timestamp": "2025-07-06T17:07:57Z",
          "url": "https://github.com/paradedb/paradedb/commit/6603901ab5f5342e3de4b0bfc99065823a606d92"
        },
        "date": 1752168407032,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 492.8489550030508,
            "unit": "median tps",
            "extra": "avg tps: 496.45492001984525, max tps: 691.2181443699485, count: 58455"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2985.566918252675,
            "unit": "median tps",
            "extra": "avg tps: 3005.7068487189945, max tps: 3394.429879825868, count: 58455"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 492.86398660093454,
            "unit": "median tps",
            "extra": "avg tps: 496.5019353029109, max tps: 687.7834689279828, count: 58455"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 448.03121919421716,
            "unit": "median tps",
            "extra": "avg tps: 449.63841128383837, max tps: 582.8458896017612, count: 58455"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 298.94401984063825,
            "unit": "median tps",
            "extra": "avg tps: 298.36758488052305, max tps: 304.09825767563444, count: 116910"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 269.50485298189517,
            "unit": "median tps",
            "extra": "avg tps: 269.0266584014645, max tps: 277.7045983704081, count: 58455"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 18.014285758809283,
            "unit": "median tps",
            "extra": "avg tps: 29.55735429375066, max tps: 1714.6129604164453, count: 58455"
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
          "id": "34939519373d98c52461b297080be89398f22c55",
          "message": "feat: implemented heap-based expression evaluation for non-indexed fields (#2740)\n\n# Ticket(s) Closed\n\n- Closes #2530\n\n## What\n\nImplemented heap-based expression evaluation for non-indexed fields in\npg_search, enabling scoring and filtering on database fields that aren't\nincluded in the search index. This allows queries to combine indexed\nsearch with PostgreSQL expressions on any table column.\n\n## Why\n\nPreviously, pg_search could only evaluate predicates on fields that were\nindexed in the search schema. This limitation prevented users from:\n- Scoring search results based on non-indexed numeric fields (e.g.,\npopularity, price, ratings)\n- Filtering search results using complex PostgreSQL expressions on\nnon-indexed columns\n- Combining full-text search with arbitrary SQL predicates in a single\nefficient query\n\nThis feature bridges the gap between search index capabilities and full\nPostgreSQL expression power.\n\n## How\n\n**Core Architecture:**\n- **HeapExpr Qual Variant**: New qual type that combines indexed search\nwith heap-based expression evaluation\n- **HeapFieldFilter**: PostgreSQL expression evaluator that works\ndirectly on heap tuples using ctid lookups\n- **Expression-Based Approach**: Stores and evaluates serialized\nPostgreSQL expression nodes rather than field-specific operators\n\n## Tests\n\n- Integration tests for various PostgreSQL expression types (boolean,\nNULL tests, constants)\n- All existing pg_search functionality remains intact and passes\nregression tests\n\n---------\n\nCo-authored-by: Eric B. Ridge <eebbrr@paradedb.com>",
          "timestamp": "2025-07-03T20:59:29Z",
          "url": "https://github.com/paradedb/paradedb/commit/34939519373d98c52461b297080be89398f22c55"
        },
        "date": 1752168417908,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 442.193207194664,
            "unit": "median tps",
            "extra": "avg tps: 446.86638487675, max tps: 611.0830030642253, count: 58445"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2776.01351490499,
            "unit": "median tps",
            "extra": "avg tps: 2753.7722372101457, max tps: 3104.211155215602, count: 58445"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 441.54270498608184,
            "unit": "median tps",
            "extra": "avg tps: 446.30664910525053, max tps: 612.4800685670904, count: 58445"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 395.41307129643,
            "unit": "median tps",
            "extra": "avg tps: 397.32588376909945, max tps: 483.95692473637416, count: 58445"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 298.74446959800235,
            "unit": "median tps",
            "extra": "avg tps: 305.3889744468572, max tps: 350.6661583262986, count: 116890"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 264.39852674741127,
            "unit": "median tps",
            "extra": "avg tps: 269.61705015755206, max tps: 297.8147463901349, count: 58445"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 13.737075485001755,
            "unit": "median tps",
            "extra": "avg tps: 17.161247141767817, max tps: 453.5036330176041, count: 58445"
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
            "name": "Philippe Noël",
            "username": "philippemnoel",
            "email": "philippemnoel@gmail.com"
          },
          "id": "3b55dd29838d74a68bcabe4e5216cc3fb73f358d",
          "message": "chore: upgrade to `0.15.26` (#2705)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\n---------\n\nSigned-off-by: Ming <ming.ying.nyc@gmail.com>\nCo-authored-by: Copilot <175728472+Copilot@users.noreply.github.com>",
          "timestamp": "2025-06-17T20:34:23Z",
          "url": "https://github.com/paradedb/paradedb/commit/3b55dd29838d74a68bcabe4e5216cc3fb73f358d"
        },
        "date": 1752168420932,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 332.30657537154053,
            "unit": "median tps",
            "extra": "avg tps: 336.93297814019473, max tps: 528.698681308794, count: 58352"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2134.0867468501256,
            "unit": "median tps",
            "extra": "avg tps: 2147.7776572762027, max tps: 2673.0751829125948, count: 58352"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 331.8877946170038,
            "unit": "median tps",
            "extra": "avg tps: 336.68581930452115, max tps: 526.9114367446997, count: 58352"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 286.40372388731413,
            "unit": "median tps",
            "extra": "avg tps: 287.90925063333947, max tps: 382.981937193107, count: 58352"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 159.18581715274263,
            "unit": "median tps",
            "extra": "avg tps: 156.88773917716793, max tps: 304.8468434450687, count: 116704"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 155.35317164352327,
            "unit": "median tps",
            "extra": "avg tps: 155.74782763288434, max tps: 240.85495518372886, count: 58352"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 8.67612938945515,
            "unit": "median tps",
            "extra": "avg tps: 14.481191863627425, max tps: 1551.6746448604656, count: 58352"
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
      },
      {
        "commit": {
          "author": {
            "name": "Philippe Noël",
            "username": "philippemnoel",
            "email": "philippemnoel@gmail.com"
          },
          "committer": {
            "name": "Philippe Noël",
            "username": "philippemnoel",
            "email": "philippemnoel@gmail.com"
          },
          "id": "6603901ab5f5342e3de4b0bfc99065823a606d92",
          "message": "Fix mintlify check workflow",
          "timestamp": "2025-07-06T17:07:57Z",
          "url": "https://github.com/paradedb/paradedb/commit/6603901ab5f5342e3de4b0bfc99065823a606d92"
        },
        "date": 1752168401207,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 38.57053225135646,
            "unit": "median tps",
            "extra": "avg tps: 38.29878450832817, max tps: 39.16401759603262, count: 59125"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 200.6192638887635,
            "unit": "median tps",
            "extra": "avg tps: 194.9650887729076, max tps: 214.60263362601742, count: 59125"
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
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752168406055,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 37.894388868921034,
            "unit": "median tps",
            "extra": "avg tps: 37.77719332393526, max tps: 38.80545868942742, count: 59100"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 198.71812566051585,
            "unit": "median tps",
            "extra": "avg tps: 197.52936662957282, max tps: 223.19164596536942, count: 59100"
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
          "id": "34939519373d98c52461b297080be89398f22c55",
          "message": "feat: implemented heap-based expression evaluation for non-indexed fields (#2740)\n\n# Ticket(s) Closed\n\n- Closes #2530\n\n## What\n\nImplemented heap-based expression evaluation for non-indexed fields in\npg_search, enabling scoring and filtering on database fields that aren't\nincluded in the search index. This allows queries to combine indexed\nsearch with PostgreSQL expressions on any table column.\n\n## Why\n\nPreviously, pg_search could only evaluate predicates on fields that were\nindexed in the search schema. This limitation prevented users from:\n- Scoring search results based on non-indexed numeric fields (e.g.,\npopularity, price, ratings)\n- Filtering search results using complex PostgreSQL expressions on\nnon-indexed columns\n- Combining full-text search with arbitrary SQL predicates in a single\nefficient query\n\nThis feature bridges the gap between search index capabilities and full\nPostgreSQL expression power.\n\n## How\n\n**Core Architecture:**\n- **HeapExpr Qual Variant**: New qual type that combines indexed search\nwith heap-based expression evaluation\n- **HeapFieldFilter**: PostgreSQL expression evaluator that works\ndirectly on heap tuples using ctid lookups\n- **Expression-Based Approach**: Stores and evaluates serialized\nPostgreSQL expression nodes rather than field-specific operators\n\n## Tests\n\n- Integration tests for various PostgreSQL expression types (boolean,\nNULL tests, constants)\n- All existing pg_search functionality remains intact and passes\nregression tests\n\n---------\n\nCo-authored-by: Eric B. Ridge <eebbrr@paradedb.com>",
          "timestamp": "2025-07-03T20:59:29Z",
          "url": "https://github.com/paradedb/paradedb/commit/34939519373d98c52461b297080be89398f22c55"
        },
        "date": 1752168420876,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 21.829849566693856,
            "unit": "median tps",
            "extra": "avg tps: 20.04955242545152, max tps: 32.63984281821511, count: 59092"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 115.55711648816037,
            "unit": "median tps",
            "extra": "avg tps: 115.70062288968235, max tps: 119.32002825745383, count: 59092"
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
            "name": "Philippe Noël",
            "username": "philippemnoel",
            "email": "philippemnoel@gmail.com"
          },
          "id": "3b55dd29838d74a68bcabe4e5216cc3fb73f358d",
          "message": "chore: upgrade to `0.15.26` (#2705)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\n---------\n\nSigned-off-by: Ming <ming.ying.nyc@gmail.com>\nCo-authored-by: Copilot <175728472+Copilot@users.noreply.github.com>",
          "timestamp": "2025-06-17T20:34:23Z",
          "url": "https://github.com/paradedb/paradedb/commit/3b55dd29838d74a68bcabe4e5216cc3fb73f358d"
        },
        "date": 1752168430224,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 38.140620931667435,
            "unit": "median tps",
            "extra": "avg tps: 37.85885911347599, max tps: 38.6775407658864, count: 59127"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 178.61151839164094,
            "unit": "median tps",
            "extra": "avg tps: 176.669172671884, max tps: 191.89729061041461, count: 59127"
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
        "date": 1752168423351,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 38.96996069607547,
            "unit": "median tps",
            "extra": "avg tps: 38.596877557916514, max tps: 39.36670210551901, count: 59106"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 207.98820158471534,
            "unit": "median tps",
            "extra": "avg tps: 204.96758530724748, max tps: 244.63019595759366, count: 59106"
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
          "id": "71ea95206a8e487805333d573e859dad68dab572",
          "message": "chore: Upgrade to `0.16.1` (#2748)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-06-30T19:38:06Z",
          "url": "https://github.com/paradedb/paradedb/commit/71ea95206a8e487805333d573e859dad68dab572"
        },
        "date": 1752168436963,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 21.18606408976348,
            "unit": "median tps",
            "extra": "avg tps: 19.507349220259336, max tps: 31.38172128387256, count: 59066"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 109.8210598603399,
            "unit": "median tps",
            "extra": "avg tps: 110.31354684135977, max tps: 114.481707962841, count: 59066"
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
          "id": "70f65d99d8f6cd7c112b8c4aa00d8c410b55efbe",
          "message": "feat: Added a GUC (`enable_custom_scan_for_non_indexed_fields`) for handling non-indexed fields in queries (#2767)\n\n## What\n\n- Disabled using bm25 index for handling queries that require\nHeapFilter, but do not use the `@@@` operator.\n- Added a new GUC `paradedb.enable_filter_pushdown` to control whether\nParadeDB's custom scan should handle queries that include non-indexed\nfield predicates.\n\n## Why\n\nThis GUC provides users with fine-grained control over when ParadeDB's\ncustom scan is used, particularly for queries that mix indexed and\nnon-indexed predicates. This is useful for:\n\n- **Performance tuning**: Users can compare custom scan performance\nagainst standard PostgreSQL execution\n- **Debugging**: Helps isolate issues related to HeapExpr filtering vs\nstandard execution\n- **Backward compatibility**: Allows disabling the feature if it causes\nissues in specific scenarios\n\n## How\n\n1. **Added GUC definition** in `src/gucs.rs`.\n2. **Integrated GUC checks** in\n`src/postgres/customscan/pdbscan/qual_inspect.rs`\n\n## Tests\n\nAdded a regression test (Test Case 19) in\n`score_non_indexed_predicates.sql`\n\nThe test shows that users can control execution strategy with:\n```sql\nSET paradedb.enable_filter_pushdown = false; -- Disable HeapExpr\nSET paradedb.enable_filter_pushdown = true;  -- Enable HeapExpr (default)\n```",
          "timestamp": "2025-07-05T18:06:46Z",
          "url": "https://github.com/paradedb/paradedb/commit/70f65d99d8f6cd7c112b8c4aa00d8c410b55efbe"
        },
        "date": 1752168443617,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 21.799598636559065,
            "unit": "median tps",
            "extra": "avg tps: 20.00971269935591, max tps: 31.645745554575285, count: 59108"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 113.74797736649316,
            "unit": "median tps",
            "extra": "avg tps: 113.85101670901611, max tps: 117.72455899334057, count: 59108"
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
        "date": 1752168401102,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 19.6319,
            "unit": "median cpu",
            "extra": "avg cpu: 20.236658989092536, max cpu: 49.382717, count: 59121"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 174.3203125,
            "unit": "median mem",
            "extra": "avg mem: 171.99379787586054, max mem: 176.00390625, count: 59121"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 18391,
            "unit": "median block_count",
            "extra": "avg block_count: 16536.430422354155, max block_count: 18391.0, count: 59121"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 48,
            "unit": "median segment_count",
            "extra": "avg segment_count: 50.22870046176485, max segment_count: 135.0, count: 59121"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 10,
            "unit": "median cpu",
            "extra": "avg cpu: 12.870381048227696, max cpu: 34.782608, count: 59121"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 168.9375,
            "unit": "median mem",
            "extra": "avg mem: 158.43736151282963, max mem: 175.296875, count: 59121"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Philippe Noël",
            "username": "philippemnoel",
            "email": "philippemnoel@gmail.com"
          },
          "committer": {
            "name": "Philippe Noël",
            "username": "philippemnoel",
            "email": "philippemnoel@gmail.com"
          },
          "id": "6603901ab5f5342e3de4b0bfc99065823a606d92",
          "message": "Fix mintlify check workflow",
          "timestamp": "2025-07-06T17:07:57Z",
          "url": "https://github.com/paradedb/paradedb/commit/6603901ab5f5342e3de4b0bfc99065823a606d92"
        },
        "date": 1752168405502,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 19.512194,
            "unit": "median cpu",
            "extra": "avg cpu: 19.739965372453934, max cpu: 49.382717, count: 59125"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 174.01171875,
            "unit": "median mem",
            "extra": "avg mem: 173.911924352537, max mem: 182.65234375, count: 59125"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 20484,
            "unit": "median block_count",
            "extra": "avg block_count: 18372.570350951373, max block_count: 20484.0, count: 59125"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 51,
            "unit": "median segment_count",
            "extra": "avg segment_count: 53.595636363636366, max segment_count: 162.0, count: 59125"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 9.756097,
            "unit": "median cpu",
            "extra": "avg cpu: 9.075242084521662, max cpu: 34.5679, count: 59125"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 170.2421875,
            "unit": "median mem",
            "extra": "avg mem: 159.01350218023256, max mem: 175.19921875, count: 59125"
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
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752168421934,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 19.512194,
            "unit": "median cpu",
            "extra": "avg cpu: 19.77109047109874, max cpu: 49.68944, count: 59100"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 174.03125,
            "unit": "median mem",
            "extra": "avg mem: 172.3828613446489, max mem: 175.90234375, count: 59100"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 22754,
            "unit": "median block_count",
            "extra": "avg block_count: 21498.315329949237, max block_count: 29765.0, count: 59100"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 51,
            "unit": "median segment_count",
            "extra": "avg segment_count: 53.76456852791878, max segment_count: 143.0, count: 59100"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 9.756097,
            "unit": "median cpu",
            "extra": "avg cpu: 8.967950925119803, max cpu: 34.355827, count: 59100"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 169.11328125,
            "unit": "median mem",
            "extra": "avg mem: 159.65441558269882, max mem: 175.48828125, count: 59100"
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
          "id": "34939519373d98c52461b297080be89398f22c55",
          "message": "feat: implemented heap-based expression evaluation for non-indexed fields (#2740)\n\n# Ticket(s) Closed\n\n- Closes #2530\n\n## What\n\nImplemented heap-based expression evaluation for non-indexed fields in\npg_search, enabling scoring and filtering on database fields that aren't\nincluded in the search index. This allows queries to combine indexed\nsearch with PostgreSQL expressions on any table column.\n\n## Why\n\nPreviously, pg_search could only evaluate predicates on fields that were\nindexed in the search schema. This limitation prevented users from:\n- Scoring search results based on non-indexed numeric fields (e.g.,\npopularity, price, ratings)\n- Filtering search results using complex PostgreSQL expressions on\nnon-indexed columns\n- Combining full-text search with arbitrary SQL predicates in a single\nefficient query\n\nThis feature bridges the gap between search index capabilities and full\nPostgreSQL expression power.\n\n## How\n\n**Core Architecture:**\n- **HeapExpr Qual Variant**: New qual type that combines indexed search\nwith heap-based expression evaluation\n- **HeapFieldFilter**: PostgreSQL expression evaluator that works\ndirectly on heap tuples using ctid lookups\n- **Expression-Based Approach**: Stores and evaluates serialized\nPostgreSQL expression nodes rather than field-specific operators\n\n## Tests\n\n- Integration tests for various PostgreSQL expression types (boolean,\nNULL tests, constants)\n- All existing pg_search functionality remains intact and passes\nregression tests\n\n---------\n\nCo-authored-by: Eric B. Ridge <eebbrr@paradedb.com>",
          "timestamp": "2025-07-03T20:59:29Z",
          "url": "https://github.com/paradedb/paradedb/commit/34939519373d98c52461b297080be89398f22c55"
        },
        "date": 1752168445191,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 19.6319,
            "unit": "median cpu",
            "extra": "avg cpu: 21.508122238777943, max cpu: 49.382717, count: 59092"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 177.53515625,
            "unit": "median mem",
            "extra": "avg mem: 174.88529016459503, max mem: 183.44140625, count: 59092"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 8901,
            "unit": "median block_count",
            "extra": "avg block_count: 7853.971231300346, max block_count: 9146.0, count: 59092"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 43,
            "unit": "median segment_count",
            "extra": "avg segment_count: 42.89506193731808, max segment_count: 85.0, count: 59092"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 14.545454,
            "unit": "median cpu",
            "extra": "avg cpu: 13.254030162263112, max cpu: 34.355827, count: 59092"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 171.359375,
            "unit": "median mem",
            "extra": "avg mem: 163.99831076435896, max mem: 177.30859375, count: 59092"
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
        "date": 1752168443651,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 19.512194,
            "unit": "median cpu",
            "extra": "avg cpu: 19.619259129804096, max cpu: 49.079754, count: 59106"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 173.84765625,
            "unit": "median mem",
            "extra": "avg mem: 172.01330395814298, max mem: 175.109375, count: 59106"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 20272,
            "unit": "median block_count",
            "extra": "avg block_count: 18211.015125367983, max block_count: 20272.0, count: 59106"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 51,
            "unit": "median segment_count",
            "extra": "avg segment_count: 53.83947484180963, max segment_count: 147.0, count: 59106"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 9.756097,
            "unit": "median cpu",
            "extra": "avg cpu: 8.883653516538814, max cpu: 34.355827, count: 59106"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 168.90625,
            "unit": "median mem",
            "extra": "avg mem: 158.57967379316398, max mem: 174.82421875, count: 59106"
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
            "name": "Philippe Noël",
            "username": "philippemnoel",
            "email": "philippemnoel@gmail.com"
          },
          "id": "3b55dd29838d74a68bcabe4e5216cc3fb73f358d",
          "message": "chore: upgrade to `0.15.26` (#2705)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\n---------\n\nSigned-off-by: Ming <ming.ying.nyc@gmail.com>\nCo-authored-by: Copilot <175728472+Copilot@users.noreply.github.com>",
          "timestamp": "2025-06-17T20:34:23Z",
          "url": "https://github.com/paradedb/paradedb/commit/3b55dd29838d74a68bcabe4e5216cc3fb73f358d"
        },
        "date": 1752168445002,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 24.539877,
            "unit": "median cpu",
            "extra": "avg cpu: 26.69482126126373, max cpu: 63.80368, count: 59127"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 241.734375,
            "unit": "median mem",
            "extra": "avg mem: 238.46046526438852, max mem: 263.62890625, count: 59127"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 19585,
            "unit": "median block_count",
            "extra": "avg block_count: 17638.29176180087, max block_count: 19585.0, count: 59127"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 52,
            "unit": "median segment_count",
            "extra": "avg segment_count: 54.628883589561454, max segment_count: 147.0, count: 59127"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 14.634146,
            "unit": "median cpu",
            "extra": "avg cpu: 15.037596160638227, max cpu: 48.780487, count: 59127"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 236.62890625,
            "unit": "median mem",
            "extra": "avg mem: 224.36151314120454, max mem: 270.84375, count: 59127"
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
        "date": 1752168408000,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.9382715,
            "unit": "median cpu",
            "extra": "avg cpu: 6.917651941248885, max cpu: 29.447853, count: 58420"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 87.078125,
            "unit": "median mem",
            "extra": "avg mem: 90.11653182236391, max mem: 104.0078125, count: 58420"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.8780484,
            "unit": "median cpu",
            "extra": "avg cpu: 4.779696180390482, max cpu: 9.937888, count: 58420"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 74.171875,
            "unit": "median mem",
            "extra": "avg mem: 74.32798351377953, max mem: 87.046875, count: 58420"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.9382715,
            "unit": "median cpu",
            "extra": "avg cpu: 6.888270180962858, max cpu: 29.268291, count: 58420"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 87.921875,
            "unit": "median mem",
            "extra": "avg mem: 90.81556642630949, max mem: 104.796875, count: 58420"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.8780484,
            "unit": "median cpu",
            "extra": "avg cpu: 4.716347711085222, max cpu: 4.968944, count: 58420"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 83.640625,
            "unit": "median mem",
            "extra": "avg mem: 85.34593588186837, max mem: 98.43359375, count: 58420"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.9382715,
            "unit": "median cpu",
            "extra": "avg cpu: 7.021459816871912, max cpu: 24.390244, count: 116840"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 90.58203125,
            "unit": "median mem",
            "extra": "avg mem: 94.62825180936537, max mem: 111.84375, count: 116840"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 6127,
            "unit": "median block_count",
            "extra": "avg block_count: 6722.465337213283, max block_count: 8125.0, count: 58420"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 117,
            "unit": "median segment_count",
            "extra": "avg segment_count: 117.4828825744608, max segment_count: 420.0, count: 58420"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.907975,
            "unit": "median cpu",
            "extra": "avg cpu: 5.7389033938455825, max cpu: 19.512194, count: 58420"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 93.6875,
            "unit": "median mem",
            "extra": "avg mem: 95.56017276564104, max mem: 110.26171875, count: 58420"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 14.634146,
            "unit": "median cpu",
            "extra": "avg cpu: 13.953768758633066, max cpu: 29.813665, count: 58420"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 73.90234375,
            "unit": "median mem",
            "extra": "avg mem: 78.05086336015063, max mem: 96.5078125, count: 58420"
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
        "date": 1752168418802,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.9382715,
            "unit": "median cpu",
            "extra": "avg cpu: 7.364178278574604, max cpu: 30.000002, count: 58480"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 111.2421875,
            "unit": "median mem",
            "extra": "avg mem: 108.26327163132694, max mem: 112.6171875, count: 58480"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.8780484,
            "unit": "median cpu",
            "extra": "avg cpu: 4.7804282334231045, max cpu: 9.937888, count: 58480"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 93.3203125,
            "unit": "median mem",
            "extra": "avg mem: 88.10699330967853, max mem: 96.0703125, count: 58480"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.9382715,
            "unit": "median cpu",
            "extra": "avg cpu: 7.421961577022006, max cpu: 29.447853, count: 58480"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 111.51953125,
            "unit": "median mem",
            "extra": "avg mem: 108.36526563568741, max mem: 112.26953125, count: 58480"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.8780484,
            "unit": "median cpu",
            "extra": "avg cpu: 4.589013517406926, max cpu: 5.0, count: 58480"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 108.6953125,
            "unit": "median mem",
            "extra": "avg mem: 105.06123668080969, max mem: 108.8203125, count: 58480"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.9382715,
            "unit": "median cpu",
            "extra": "avg cpu: 7.1933861524001435, max cpu: 25.0, count: 116960"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 116.109375,
            "unit": "median mem",
            "extra": "avg mem: 114.80104786759362, max mem: 123.17578125, count: 116960"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 8370,
            "unit": "median block_count",
            "extra": "avg block_count: 8145.970998632011, max block_count: 9003.0, count: 58480"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 118,
            "unit": "median segment_count",
            "extra": "avg segment_count: 118.04168946648427, max segment_count: 440.0, count: 58480"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.907975,
            "unit": "median cpu",
            "extra": "avg cpu: 5.985161780052626, max cpu: 19.512194, count: 58480"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 119.78125,
            "unit": "median mem",
            "extra": "avg mem: 118.63720435939638, max mem: 125.5546875, count: 58480"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 14.634146,
            "unit": "median cpu",
            "extra": "avg cpu: 14.20667965377837, max cpu: 29.447853, count: 58480"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 94.80859375,
            "unit": "median mem",
            "extra": "avg mem: 89.09276695825496, max mem: 96.58984375, count: 58480"
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
        "date": 1752168424258,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.9382715,
            "unit": "median cpu",
            "extra": "avg cpu: 6.839748801832486, max cpu: 30.000002, count: 58465"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 102.07421875,
            "unit": "median mem",
            "extra": "avg mem: 100.72576481388438, max mem: 104.6796875, count: 58465"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.8780484,
            "unit": "median cpu",
            "extra": "avg cpu: 4.726382900609668, max cpu: 5.0314465, count: 58465"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 92.796875,
            "unit": "median mem",
            "extra": "avg mem: 88.98464278842042, max mem: 92.796875, count: 58465"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.9382715,
            "unit": "median cpu",
            "extra": "avg cpu: 6.85641799646577, max cpu: 30.000002, count: 58465"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 102.390625,
            "unit": "median mem",
            "extra": "avg mem: 101.02558194539468, max mem: 104.96875, count: 58465"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.8780484,
            "unit": "median cpu",
            "extra": "avg cpu: 4.653619887769703, max cpu: 5.0, count: 58465"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 100.0078125,
            "unit": "median mem",
            "extra": "avg mem: 97.82566699895237, max mem: 100.35546875, count: 58465"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.9382715,
            "unit": "median cpu",
            "extra": "avg cpu: 7.218860275516013, max cpu: 30.000002, count: 116930"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 105.93359375,
            "unit": "median mem",
            "extra": "avg mem: 105.43003462942572, max mem: 111.96875, count: 116930"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 8059,
            "unit": "median block_count",
            "extra": "avg block_count: 7902.417788420423, max block_count: 8059.0, count: 58465"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 118,
            "unit": "median segment_count",
            "extra": "avg segment_count: 118.246147267596, max segment_count: 399.0, count: 58465"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.907975,
            "unit": "median cpu",
            "extra": "avg cpu: 5.887546764959426, max cpu: 15.6862755, count: 58465"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 108.69921875,
            "unit": "median mem",
            "extra": "avg mem: 107.74156366255453, max mem: 114.57421875, count: 58465"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 14.723927,
            "unit": "median cpu",
            "extra": "avg cpu: 14.225791845515312, max cpu: 29.090908, count: 58465"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 93.58203125,
            "unit": "median mem",
            "extra": "avg mem: 85.9617003762935, max mem: 95.9609375, count: 58465"
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
            "name": "Philippe Noël",
            "username": "philippemnoel",
            "email": "philippemnoel@gmail.com"
          },
          "id": "3b55dd29838d74a68bcabe4e5216cc3fb73f358d",
          "message": "chore: upgrade to `0.15.26` (#2705)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\n---------\n\nSigned-off-by: Ming <ming.ying.nyc@gmail.com>\nCo-authored-by: Copilot <175728472+Copilot@users.noreply.github.com>",
          "timestamp": "2025-06-17T20:34:23Z",
          "url": "https://github.com/paradedb/paradedb/commit/3b55dd29838d74a68bcabe4e5216cc3fb73f358d"
        },
        "date": 1752168437799,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 9.638554,
            "unit": "median cpu",
            "extra": "avg cpu: 8.342789550012265, max cpu: 26.016258, count: 58352"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 105.171875,
            "unit": "median mem",
            "extra": "avg mem: 104.86022557602996, max mem: 108.015625, count: 58352"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.8484845,
            "unit": "median cpu",
            "extra": "avg cpu: 4.8543851193210195, max cpu: 9.81595, count: 58352"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 90.0703125,
            "unit": "median mem",
            "extra": "avg mem: 89.12131586642275, max mem: 91.0703125, count: 58352"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 9.638554,
            "unit": "median cpu",
            "extra": "avg cpu: 8.286851546497223, max cpu: 24.691359, count: 58352"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 104.96484375,
            "unit": "median mem",
            "extra": "avg mem: 104.76614481133895, max mem: 108.0234375, count: 58352"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.8484845,
            "unit": "median cpu",
            "extra": "avg cpu: 4.6936120969333, max cpu: 4.968944, count: 58352"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 102.12109375,
            "unit": "median mem",
            "extra": "avg mem: 101.6166255815996, max mem: 103.12109375, count: 58352"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 14.545454,
            "unit": "median cpu",
            "extra": "avg cpu: 14.290956580179158, max cpu: 67.87879, count: 116704"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 147.296875,
            "unit": "median mem",
            "extra": "avg mem: 147.09051317863998, max mem: 168.046875, count: 116704"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 7714,
            "unit": "median block_count",
            "extra": "avg block_count: 7727.365625856869, max block_count: 7790.0, count: 58352"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 117,
            "unit": "median segment_count",
            "extra": "avg segment_count: 116.49100287907869, max segment_count: 252.0, count: 58352"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.580839,
            "unit": "median cpu",
            "extra": "avg cpu: 8.727633037830264, max cpu: 25.0, count: 58352"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 161.046875,
            "unit": "median mem",
            "extra": "avg mem: 161.06666625608378, max mem: 186.30078125, count: 58352"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.8780484,
            "unit": "median cpu",
            "extra": "avg cpu: 7.187719990466065, max cpu: 14.634146, count: 58352"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 96.98046875,
            "unit": "median mem",
            "extra": "avg mem: 94.98944538640492, max mem: 98.35546875, count: 58352"
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
        "date": 1752168438990,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.9382715,
            "unit": "median cpu",
            "extra": "avg cpu: 7.149000658515146, max cpu: 29.090908, count: 58439"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 101.14453125,
            "unit": "median mem",
            "extra": "avg mem: 95.55227693353325, max mem: 108.0625, count: 58439"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.8780484,
            "unit": "median cpu",
            "extra": "avg cpu: 4.63044096259283, max cpu: 10.0, count: 58439"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 75.73828125,
            "unit": "median mem",
            "extra": "avg mem: 74.20372391510806, max mem: 84.86328125, count: 58439"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.9382715,
            "unit": "median cpu",
            "extra": "avg cpu: 7.117024017328705, max cpu: 29.090908, count: 58439"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 101.99609375,
            "unit": "median mem",
            "extra": "avg mem: 96.2095574552097, max mem: 108.70703125, count: 58439"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.8780484,
            "unit": "median cpu",
            "extra": "avg cpu: 4.592827794893733, max cpu: 5.0, count: 58439"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 96.828125,
            "unit": "median mem",
            "extra": "avg mem: 90.9449372422526, max mem: 101.66796875, count: 58439"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.9382715,
            "unit": "median cpu",
            "extra": "avg cpu: 7.090125469838232, max cpu: 24.242424, count: 116878"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 103.14453125,
            "unit": "median mem",
            "extra": "avg mem: 97.8950776931929, max mem: 114.26953125, count: 116878"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 8651,
            "unit": "median block_count",
            "extra": "avg block_count: 7609.630880062971, max block_count: 8651.0, count: 58439"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 117,
            "unit": "median segment_count",
            "extra": "avg segment_count: 117.58057119389449, max segment_count: 448.0, count: 58439"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.907975,
            "unit": "median cpu",
            "extra": "avg cpu: 5.735843240902093, max cpu: 14.906833, count: 58439"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 110.19921875,
            "unit": "median mem",
            "extra": "avg mem: 103.85658623468488, max mem: 117.453125, count: 58439"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 14.634146,
            "unit": "median cpu",
            "extra": "avg cpu: 14.336556808025332, max cpu: 29.813665, count: 58439"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 69.0234375,
            "unit": "median mem",
            "extra": "avg mem: 74.51644496558377, max mem: 90.1015625, count: 58439"
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
          "id": "34939519373d98c52461b297080be89398f22c55",
          "message": "feat: implemented heap-based expression evaluation for non-indexed fields (#2740)\n\n# Ticket(s) Closed\n\n- Closes #2530\n\n## What\n\nImplemented heap-based expression evaluation for non-indexed fields in\npg_search, enabling scoring and filtering on database fields that aren't\nincluded in the search index. This allows queries to combine indexed\nsearch with PostgreSQL expressions on any table column.\n\n## Why\n\nPreviously, pg_search could only evaluate predicates on fields that were\nindexed in the search schema. This limitation prevented users from:\n- Scoring search results based on non-indexed numeric fields (e.g.,\npopularity, price, ratings)\n- Filtering search results using complex PostgreSQL expressions on\nnon-indexed columns\n- Combining full-text search with arbitrary SQL predicates in a single\nefficient query\n\nThis feature bridges the gap between search index capabilities and full\nPostgreSQL expression power.\n\n## How\n\n**Core Architecture:**\n- **HeapExpr Qual Variant**: New qual type that combines indexed search\nwith heap-based expression evaluation\n- **HeapFieldFilter**: PostgreSQL expression evaluator that works\ndirectly on heap tuples using ctid lookups\n- **Expression-Based Approach**: Stores and evaluates serialized\nPostgreSQL expression nodes rather than field-specific operators\n\n## Tests\n\n- Integration tests for various PostgreSQL expression types (boolean,\nNULL tests, constants)\n- All existing pg_search functionality remains intact and passes\nregression tests\n\n---------\n\nCo-authored-by: Eric B. Ridge <eebbrr@paradedb.com>",
          "timestamp": "2025-07-03T20:59:29Z",
          "url": "https://github.com/paradedb/paradedb/commit/34939519373d98c52461b297080be89398f22c55"
        },
        "date": 1752168438118,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.9382715,
            "unit": "median cpu",
            "extra": "avg cpu: 7.201295098543367, max cpu: 30.000002, count: 58445"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 104.796875,
            "unit": "median mem",
            "extra": "avg mem: 102.13653040518864, max mem: 105.171875, count: 58445"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.8780484,
            "unit": "median cpu",
            "extra": "avg cpu: 4.819028967300791, max cpu: 9.876543, count: 58445"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 89.80078125,
            "unit": "median mem",
            "extra": "avg mem: 84.70892465672854, max mem: 90.30078125, count: 58445"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.9382715,
            "unit": "median cpu",
            "extra": "avg cpu: 7.349864328758064, max cpu: 30.000002, count: 58445"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 102.2421875,
            "unit": "median mem",
            "extra": "avg mem: 101.47948271344855, max mem: 105.5, count: 58445"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.8780484,
            "unit": "median cpu",
            "extra": "avg cpu: 4.759743967839406, max cpu: 7.8431377, count: 58445"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 101.125,
            "unit": "median mem",
            "extra": "avg mem: 98.89583155103088, max mem: 101.375, count: 58445"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.9382715,
            "unit": "median cpu",
            "extra": "avg cpu: 7.066673355933671, max cpu: 25.0, count: 116890"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 111.07421875,
            "unit": "median mem",
            "extra": "avg mem: 110.68189413524468, max mem: 117.1640625, count: 116890"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 7654,
            "unit": "median block_count",
            "extra": "avg block_count: 7520.72447600308, max block_count: 7654.0, count: 58445"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 118,
            "unit": "median segment_count",
            "extra": "avg segment_count: 117.60744289502952, max segment_count: 294.0, count: 58445"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.907975,
            "unit": "median cpu",
            "extra": "avg cpu: 5.708131082455158, max cpu: 16.528925, count: 58445"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 114.35546875,
            "unit": "median mem",
            "extra": "avg mem: 113.61641821370519, max mem: 119.3046875, count: 58445"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 11.881188,
            "unit": "median cpu",
            "extra": "avg cpu: 13.759463493056465, max cpu: 29.447853, count: 58445"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 84.1796875,
            "unit": "median mem",
            "extra": "avg mem: 79.84647866049276, max mem: 92.203125, count: 58445"
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
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752168451946,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.9382715,
            "unit": "median cpu",
            "extra": "avg cpu: 6.91223807249487, max cpu: 25.157234, count: 58441"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 86.75390625,
            "unit": "median mem",
            "extra": "avg mem: 89.14520721218837, max mem: 96.4140625, count: 58441"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.8780484,
            "unit": "median cpu",
            "extra": "avg cpu: 4.807422906316046, max cpu: 9.937888, count: 58441"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 77.1953125,
            "unit": "median mem",
            "extra": "avg mem: 78.15128131363255, max mem: 83.3203125, count: 58441"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.9382715,
            "unit": "median cpu",
            "extra": "avg cpu: 6.855133988010053, max cpu: 25.157234, count: 58441"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 86.734375,
            "unit": "median mem",
            "extra": "avg mem: 89.0494192326449, max mem: 96.64453125, count: 58441"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.907975,
            "unit": "median cpu",
            "extra": "avg cpu: 4.700570627526681, max cpu: 4.968944, count: 58441"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 83.46484375,
            "unit": "median mem",
            "extra": "avg mem: 85.3848781516829, max mem: 91.765625, count: 58441"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.9382715,
            "unit": "median cpu",
            "extra": "avg cpu: 6.964897156158547, max cpu: 20.125786, count: 116882"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 93.4765625,
            "unit": "median mem",
            "extra": "avg mem: 92.58556172036755, max mem: 102.60546875, count: 116882"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 6038,
            "unit": "median block_count",
            "extra": "avg block_count: 6392.376294040143, max block_count: 7053.0, count: 58441"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 117,
            "unit": "median segment_count",
            "extra": "avg segment_count: 117.01320990400575, max segment_count: 297.0, count: 58441"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.907975,
            "unit": "median cpu",
            "extra": "avg cpu: 5.743209520707493, max cpu: 19.753086, count: 58441"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 97.296875,
            "unit": "median mem",
            "extra": "avg mem: 95.39907903911637, max mem: 104.92578125, count: 58441"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 14.723927,
            "unit": "median cpu",
            "extra": "avg cpu: 14.482145072612512, max cpu: 29.62963, count: 58441"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 77.7421875,
            "unit": "median mem",
            "extra": "avg mem: 75.94460131264866, max mem: 84.1953125, count: 58441"
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
          "id": "71ea95206a8e487805333d573e859dad68dab572",
          "message": "chore: Upgrade to `0.16.1` (#2748)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-06-30T19:38:06Z",
          "url": "https://github.com/paradedb/paradedb/commit/71ea95206a8e487805333d573e859dad68dab572"
        },
        "date": 1752168456546,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.9382715,
            "unit": "median cpu",
            "extra": "avg cpu: 7.28347832673168, max cpu: 25.0, count: 58442"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 103.62109375,
            "unit": "median mem",
            "extra": "avg mem: 104.48524525384141, max mem: 114.2265625, count: 58442"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.8780484,
            "unit": "median cpu",
            "extra": "avg cpu: 4.734227510091656, max cpu: 9.876543, count: 58442"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 90.07421875,
            "unit": "median mem",
            "extra": "avg mem: 86.74409751334656, max mem: 95.07421875, count: 58442"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.9382715,
            "unit": "median cpu",
            "extra": "avg cpu: 7.149959117987779, max cpu: 30.000002, count: 58442"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 105.30859375,
            "unit": "median mem",
            "extra": "avg mem: 105.80487665655693, max mem: 115.87109375, count: 58442"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.8780484,
            "unit": "median cpu",
            "extra": "avg cpu: 4.518096901388963, max cpu: 5.0, count: 58442"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 102.421875,
            "unit": "median mem",
            "extra": "avg mem: 101.08307127686425, max mem: 109.91796875, count: 58442"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.9382715,
            "unit": "median cpu",
            "extra": "avg cpu: 7.0463399233488975, max cpu: 25.0, count: 116884"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 114.73046875,
            "unit": "median mem",
            "extra": "avg mem: 113.70423888481315, max mem: 125.390625, count: 116884"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 7711,
            "unit": "median block_count",
            "extra": "avg block_count: 7883.014048116081, max block_count: 8841.0, count: 58442"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 118,
            "unit": "median segment_count",
            "extra": "avg segment_count: 118.0882413332877, max segment_count: 402.0, count: 58442"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.907975,
            "unit": "median cpu",
            "extra": "avg cpu: 5.709666327130543, max cpu: 19.393938, count: 58442"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 117.5390625,
            "unit": "median mem",
            "extra": "avg mem: 114.17680169281083, max mem: 125.38671875, count: 58442"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 11.881188,
            "unit": "median cpu",
            "extra": "avg cpu: 13.0535002142944, max cpu: 29.447853, count: 58442"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 83.546875,
            "unit": "median mem",
            "extra": "avg mem: 82.10095125027806, max mem: 98.88671875, count: 58442"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Philippe Noël",
            "username": "philippemnoel",
            "email": "philippemnoel@gmail.com"
          },
          "committer": {
            "name": "Philippe Noël",
            "username": "philippemnoel",
            "email": "philippemnoel@gmail.com"
          },
          "id": "6603901ab5f5342e3de4b0bfc99065823a606d92",
          "message": "Fix mintlify check workflow",
          "timestamp": "2025-07-06T17:07:57Z",
          "url": "https://github.com/paradedb/paradedb/commit/6603901ab5f5342e3de4b0bfc99065823a606d92"
        },
        "date": 1752168459040,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.9382715,
            "unit": "median cpu",
            "extra": "avg cpu: 6.872032767670038, max cpu: 34.355827, count: 58455"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 101.41796875,
            "unit": "median mem",
            "extra": "avg mem: 97.73966726541784, max mem: 108.953125, count: 58455"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.8780484,
            "unit": "median cpu",
            "extra": "avg cpu: 4.778855086226748, max cpu: 9.876543, count: 58455"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 78.48828125,
            "unit": "median mem",
            "extra": "avg mem: 76.32772090978102, max mem: 85.23828125, count: 58455"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.907975,
            "unit": "median cpu",
            "extra": "avg cpu: 6.8985688181363924, max cpu: 29.813665, count: 58455"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 101.625,
            "unit": "median mem",
            "extra": "avg mem: 98.20470988634419, max mem: 109.58203125, count: 58455"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.8780484,
            "unit": "median cpu",
            "extra": "avg cpu: 4.452904547870766, max cpu: 4.968944, count: 58455"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 95.05078125,
            "unit": "median mem",
            "extra": "avg mem: 92.01114191739372, max mem: 103.11328125, count: 58455"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.9382715,
            "unit": "median cpu",
            "extra": "avg cpu: 7.230375323533493, max cpu: 25.0, count: 116910"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 106.4140625,
            "unit": "median mem",
            "extra": "avg mem: 101.57099558421008, max mem: 115.33984375, count: 116910"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 8571,
            "unit": "median block_count",
            "extra": "avg block_count: 7845.7598665640235, max block_count: 8571.0, count: 58455"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 117,
            "unit": "median segment_count",
            "extra": "avg segment_count: 116.79148062612266, max segment_count: 251.0, count: 58455"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.907975,
            "unit": "median cpu",
            "extra": "avg cpu: 5.8248387091878335, max cpu: 15.841584, count: 58455"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 98.5546875,
            "unit": "median mem",
            "extra": "avg mem: 96.27139078671628, max mem: 109.97265625, count: 58455"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 14.634146,
            "unit": "median cpu",
            "extra": "avg cpu: 14.338233709376603, max cpu: 29.813665, count: 58455"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 88.5546875,
            "unit": "median mem",
            "extra": "avg mem: 81.45438097414677, max mem: 90.25, count: 58455"
          }
        ]
      }
    ]
  }
}