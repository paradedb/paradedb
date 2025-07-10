window.BENCHMARK_DATA = {
  "lastUpdate": 1752156069658,
  "repoUrl": "https://github.com/paradedb/paradedb",
  "entries": {
    "pg_search bulk-updates.toml Performance": [
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
        "date": 1752155706225,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 21.67317938193744,
            "unit": "avg cpu",
            "extra": "max cpu: 43.636364, count: 21257"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 225.03204642882343,
            "unit": "avg mem",
            "extra": "max mem: 238.80078125, count: 21257"
          },
          {
            "name": "Bulk Update - Primary - tps",
            "value": 3.7044897264516883,
            "unit": "avg tps",
            "extra": "max tps: 6.916997301059805, count: 21257"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 22.350089391383335,
            "unit": "avg cpu",
            "extra": "max cpu: 33.939396, count: 21257"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 161.5145430034812,
            "unit": "avg mem",
            "extra": "max mem: 162.421875, count: 21257"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.453447168607325,
            "unit": "avg tps",
            "extra": "max tps: 6.918542840944033, count: 21257"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 19384.215223220584,
            "unit": "avg block_count",
            "extra": "max block_count: 22156.0, count: 21257"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 26.50472785435386,
            "unit": "avg segment_count",
            "extra": "max segment_count: 59.0, count: 21257"
          }
        ]
      }
    ],
    "pg_search single-server.toml Performance": [
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
        "date": 1752156053422,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 7.855642356916622,
            "unit": "avg cpu",
            "extra": "max cpu: 35.0, count: 58434"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 109.21135029210306,
            "unit": "avg mem",
            "extra": "max mem: 113.15234375, count: 58434"
          },
          {
            "name": "Custom Scan - Primary - tps",
            "value": 415.42614016097895,
            "unit": "avg tps",
            "extra": "max tps: 592.9206984949097, count: 58434"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.7719016687781535,
            "unit": "avg cpu",
            "extra": "max cpu: 9.81595, count: 58434"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 90.3414423751583,
            "unit": "avg mem",
            "extra": "max mem: 95.46875, count: 58434"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2521.3383516822687,
            "unit": "avg tps",
            "extra": "max tps: 2865.9185629793024, count: 58434"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 7.870025982056177,
            "unit": "avg cpu",
            "extra": "max cpu: 30.188679, count: 58434"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 109.79254131798268,
            "unit": "avg mem",
            "extra": "max mem: 114.1484375, count: 58434"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 414.9338018164663,
            "unit": "avg tps",
            "extra": "max tps: 592.2500465565683, count: 58434"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.631185465058687,
            "unit": "avg cpu",
            "extra": "max cpu: 5.0, count: 58434"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 104.27221530113033,
            "unit": "avg mem",
            "extra": "max mem: 106.16796875, count: 58434"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 395.01816844539695,
            "unit": "avg tps",
            "extra": "max tps: 501.2510725519825, count: 58434"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 7.292312103701441,
            "unit": "avg cpu",
            "extra": "max cpu: 25.316456, count: 116868"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 118.39543514772008,
            "unit": "avg mem",
            "extra": "max mem: 124.24609375, count: 116868"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 287.8029569707913,
            "unit": "avg tps",
            "extra": "max tps: 330.98892245328, count: 116868"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 8574.262706643392,
            "unit": "avg block_count",
            "extra": "max block_count: 8585.0, count: 58434"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 117.79655679912379,
            "unit": "avg segment_count",
            "extra": "max segment_count: 296.0, count: 58434"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 5.881267011614395,
            "unit": "avg cpu",
            "extra": "max cpu: 19.875776, count: 58434"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 118.9705179534817,
            "unit": "avg mem",
            "extra": "max mem: 126.28125, count: 58434"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 266.76935942212276,
            "unit": "avg tps",
            "extra": "max tps: 293.0360409212464, count: 58434"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 12.245118274618548,
            "unit": "avg cpu",
            "extra": "max cpu: 29.62963, count: 58434"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 91.48297926132474,
            "unit": "avg mem",
            "extra": "max mem: 97.90625, count: 58434"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 21.6323500058667,
            "unit": "avg tps",
            "extra": "max tps: 1704.8641479003745, count: 58434"
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
        "date": 1752156055217,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 6.8215708501977455,
            "unit": "avg cpu",
            "extra": "max cpu: 24.691359, count: 58450"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 95.23434646332335,
            "unit": "avg mem",
            "extra": "max mem: 99.59375, count: 58450"
          },
          {
            "name": "Custom Scan - Primary - tps",
            "value": 497.52280765767745,
            "unit": "avg tps",
            "extra": "max tps: 647.5381478818036, count: 58450"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.828020140173067,
            "unit": "avg cpu",
            "extra": "max cpu: 9.876543, count: 58450"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 79.3191313355432,
            "unit": "avg mem",
            "extra": "max mem: 85.2421875, count: 58450"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2983.380218616359,
            "unit": "avg tps",
            "extra": "max tps: 3237.7095675289547, count: 58450"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 6.80343302097104,
            "unit": "avg cpu",
            "extra": "max cpu: 24.84472, count: 58450"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 95.81746838911462,
            "unit": "avg mem",
            "extra": "max mem: 100.55859375, count: 58450"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 497.5174239202883,
            "unit": "avg tps",
            "extra": "max tps: 646.152469523439, count: 58450"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.704471546638148,
            "unit": "avg cpu",
            "extra": "max cpu: 5.0314465, count: 58450"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 92.215718429213,
            "unit": "avg mem",
            "extra": "max mem: 95.41015625, count: 58450"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 458.83755457540644,
            "unit": "avg tps",
            "extra": "max tps: 574.4985604215072, count: 58450"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 7.01865730946624,
            "unit": "avg cpu",
            "extra": "max cpu: 20.0, count: 116900"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 99.28503542023097,
            "unit": "avg mem",
            "extra": "max mem: 106.4375, count: 116900"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 317.4829589804477,
            "unit": "avg tps",
            "extra": "max tps: 324.68216037620454, count: 116900"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 7190.149392643285,
            "unit": "avg block_count",
            "extra": "max block_count: 7411.0, count: 58450"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 117.2043113772455,
            "unit": "avg segment_count",
            "extra": "max segment_count: 360.0, count: 58450"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 5.675070685712289,
            "unit": "avg cpu",
            "extra": "max cpu: 14.814815, count: 58450"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 98.67097779886656,
            "unit": "avg mem",
            "extra": "max mem: 108.35546875, count: 58450"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 288.1845053347515,
            "unit": "avg tps",
            "extra": "max tps: 299.5747719186087, count: 58450"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 14.571036148713613,
            "unit": "avg cpu",
            "extra": "max cpu: 29.62963, count: 58450"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 82.52467633928572,
            "unit": "avg mem",
            "extra": "max mem: 88.31640625, count: 58450"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 20.402791422903903,
            "unit": "avg tps",
            "extra": "max tps: 1758.8386036932093, count: 58450"
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
        "date": 1752156057789,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 6.8789195184291065,
            "unit": "avg cpu",
            "extra": "max cpu: 29.813665, count: 58455"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 85.08287932490805,
            "unit": "avg mem",
            "extra": "max mem: 92.10546875, count: 58455"
          },
          {
            "name": "Custom Scan - Primary - tps",
            "value": 487.2937448586161,
            "unit": "avg tps",
            "extra": "max tps: 655.9507928813618, count: 58455"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.740318885711953,
            "unit": "avg cpu",
            "extra": "max cpu: 9.81595, count: 58455"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 71.43475804037294,
            "unit": "avg mem",
            "extra": "max mem: 76.296875, count: 58455"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2912.4772319365825,
            "unit": "avg tps",
            "extra": "max tps: 3305.082090078421, count: 58455"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 6.962764445720116,
            "unit": "avg cpu",
            "extra": "max cpu: 25.0, count: 58455"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 85.10274489992302,
            "unit": "avg mem",
            "extra": "max mem: 92.46875, count: 58455"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 488.08733861064695,
            "unit": "avg tps",
            "extra": "max tps: 654.4671601356512, count: 58455"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.543539013997248,
            "unit": "avg cpu",
            "extra": "max cpu: 5.063291, count: 58455"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 81.08891708900009,
            "unit": "avg mem",
            "extra": "max mem: 86.3671875, count: 58455"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 456.47896378576354,
            "unit": "avg tps",
            "extra": "max tps: 572.8982354562478, count: 58455"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 6.773881498810911,
            "unit": "avg cpu",
            "extra": "max cpu: 24.691359, count: 116910"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 88.65517735998846,
            "unit": "avg mem",
            "extra": "max mem: 98.72265625, count: 116910"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 342.66918341901203,
            "unit": "avg tps",
            "extra": "max tps: 373.5774497980123, count: 116910"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 5903.966777863314,
            "unit": "avg block_count",
            "extra": "max block_count: 6429.0, count: 58455"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 116.22997177315884,
            "unit": "avg segment_count",
            "extra": "max segment_count: 286.0, count: 58455"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 5.752832774352822,
            "unit": "avg cpu",
            "extra": "max cpu: 15.000001, count: 58455"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 90.02364378849114,
            "unit": "avg mem",
            "extra": "max mem: 98.1171875, count: 58455"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 282.8463057093361,
            "unit": "avg tps",
            "extra": "max tps: 302.3104432904451, count: 58455"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 16.798362962026502,
            "unit": "avg cpu",
            "extra": "max cpu: 30.000002, count: 58455"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 71.15102048317081,
            "unit": "avg mem",
            "extra": "max mem: 82.54296875, count: 58455"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 25.674767114796666,
            "unit": "avg tps",
            "extra": "max tps: 526.51420219409, count: 58455"
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
        "date": 1752156060199,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 7.492153579559261,
            "unit": "avg cpu",
            "extra": "max cpu: 33.532936, count: 58434"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 107.90505968806688,
            "unit": "avg mem",
            "extra": "max mem: 111.30078125, count: 58434"
          },
          {
            "name": "Custom Scan - Primary - tps",
            "value": 439.27865216515585,
            "unit": "avg tps",
            "extra": "max tps: 605.817969179427, count: 58434"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.831833574148227,
            "unit": "avg cpu",
            "extra": "max cpu: 9.876543, count: 58434"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 90.34662891574254,
            "unit": "avg mem",
            "extra": "max mem: 95.17578125, count: 58434"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2675.6437352189373,
            "unit": "avg tps",
            "extra": "max tps: 2858.4022901022036, count: 58434"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 7.462915936401029,
            "unit": "avg cpu",
            "extra": "max cpu: 34.782608, count: 58434"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 108.68059316921655,
            "unit": "avg mem",
            "extra": "max mem: 112.578125, count: 58434"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 439.1568379378175,
            "unit": "avg tps",
            "extra": "max tps: 614.6052252485662, count: 58434"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.690417876310695,
            "unit": "avg cpu",
            "extra": "max cpu: 5.0314465, count: 58434"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 104.60293477470309,
            "unit": "avg mem",
            "extra": "max mem: 107.93359375, count: 58434"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 402.3240266797566,
            "unit": "avg tps",
            "extra": "max tps: 510.7546803898958, count: 58434"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 7.283219582930968,
            "unit": "avg cpu",
            "extra": "max cpu: 24.84472, count: 116868"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 116.42693964157212,
            "unit": "avg mem",
            "extra": "max mem: 123.66015625, count: 116868"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 293.7078042660877,
            "unit": "avg tps",
            "extra": "max tps: 333.92776039613943, count: 116868"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 8281.681486805626,
            "unit": "avg block_count",
            "extra": "max block_count: 8472.0, count: 58434"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 117.05388985864394,
            "unit": "avg segment_count",
            "extra": "max segment_count: 362.0, count: 58434"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 5.73490190790389,
            "unit": "avg cpu",
            "extra": "max cpu: 19.6319, count: 58434"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 118.48030730984958,
            "unit": "avg mem",
            "extra": "max mem: 125.84765625, count: 58434"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 270.03699194583714,
            "unit": "avg tps",
            "extra": "max tps: 293.42244140746766, count: 58434"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 16.286857157384354,
            "unit": "avg cpu",
            "extra": "max cpu: 29.447853, count: 58434"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 90.75416208083051,
            "unit": "avg mem",
            "extra": "max mem: 97.4453125, count: 58434"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 28.517103997958852,
            "unit": "avg tps",
            "extra": "max tps: 1760.8113818847726, count: 58434"
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
        "date": 1752156062272,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 6.809039558923789,
            "unit": "avg cpu",
            "extra": "max cpu: 30.000002, count: 58478"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 87.122713483062,
            "unit": "avg mem",
            "extra": "max mem: 99.1640625, count: 58478"
          },
          {
            "name": "Custom Scan - Primary - tps",
            "value": 488.19828027484226,
            "unit": "avg tps",
            "extra": "max tps: 673.0017607049055, count: 58478"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.9391157922351585,
            "unit": "avg cpu",
            "extra": "max cpu: 10.062893, count: 58478"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 73.79160919758286,
            "unit": "avg mem",
            "extra": "max mem: 81.30078125, count: 58478"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2880.885533531417,
            "unit": "avg tps",
            "extra": "max tps: 3236.0754941986465, count: 58478"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 6.84629811464069,
            "unit": "avg cpu",
            "extra": "max cpu: 35.0, count: 58478"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 87.8298752575755,
            "unit": "avg mem",
            "extra": "max mem: 99.828125, count: 58478"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 488.0679226186709,
            "unit": "avg tps",
            "extra": "max tps: 674.4841454459686, count: 58478"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.770984404151833,
            "unit": "avg cpu",
            "extra": "max cpu: 4.968944, count: 58478"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 83.95082311937823,
            "unit": "avg mem",
            "extra": "max mem: 94.7265625, count: 58478"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 447.86594646762506,
            "unit": "avg tps",
            "extra": "max tps: 582.47960753456, count: 58478"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 7.096149819742158,
            "unit": "avg cpu",
            "extra": "max cpu: 25.157234, count: 116956"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 91.84799067731669,
            "unit": "avg mem",
            "extra": "max mem: 106.7265625, count: 116956"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 297.86271144354987,
            "unit": "avg tps",
            "extra": "max tps: 318.3627897732719, count: 116956"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 6227.811433359554,
            "unit": "avg block_count",
            "extra": "max block_count: 7421.0, count: 58478"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 117.0353466260816,
            "unit": "avg segment_count",
            "extra": "max segment_count: 380.0, count: 58478"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 5.880092433687984,
            "unit": "avg cpu",
            "extra": "max cpu: 15.094339, count: 58478"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 93.10901943091847,
            "unit": "avg mem",
            "extra": "max mem: 104.5078125, count: 58478"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 266.9432424136969,
            "unit": "avg tps",
            "extra": "max tps: 271.0316511895816, count: 58478"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 14.642715286707226,
            "unit": "avg cpu",
            "extra": "max cpu: 29.62963, count: 58478"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 70.93886442871678,
            "unit": "avg mem",
            "extra": "max mem: 80.44140625, count: 58478"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 26.55222102181398,
            "unit": "avg tps",
            "extra": "max tps: 1610.6431298017296, count: 58478"
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
        "date": 1752156066069,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 7.480735434980999,
            "unit": "avg cpu",
            "extra": "max cpu: 30.000002, count: 58450"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 110.009655822284,
            "unit": "avg mem",
            "extra": "max mem: 117.69140625, count: 58450"
          },
          {
            "name": "Custom Scan - Primary - tps",
            "value": 429.3221540341651,
            "unit": "avg tps",
            "extra": "max tps: 600.5153507859247, count: 58450"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.939189286532671,
            "unit": "avg cpu",
            "extra": "max cpu: 9.876543, count: 58450"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 90.6508495508982,
            "unit": "avg mem",
            "extra": "max mem: 96.859375, count: 58450"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2689.5608985732,
            "unit": "avg tps",
            "extra": "max tps: 2927.8785979094746, count: 58450"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 7.562028005753056,
            "unit": "avg cpu",
            "extra": "max cpu: 29.447853, count: 58450"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 110.49182374091103,
            "unit": "avg mem",
            "extra": "max mem: 117.79296875, count: 58450"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 429.45560697108834,
            "unit": "avg tps",
            "extra": "max tps: 611.1052566046649, count: 58450"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.354601120332436,
            "unit": "avg cpu",
            "extra": "max cpu: 5.0, count: 58450"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 107.27565961826348,
            "unit": "avg mem",
            "extra": "max mem: 113.984375, count: 58450"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 395.6000950613749,
            "unit": "avg tps",
            "extra": "max tps: 488.6118503129779, count: 58450"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 7.197762328636675,
            "unit": "avg cpu",
            "extra": "max cpu: 25.157234, count: 116900"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 118.75125664964713,
            "unit": "avg mem",
            "extra": "max mem: 126.828125, count: 116900"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 296.89535053379274,
            "unit": "avg tps",
            "extra": "max tps: 342.35229028420616, count: 116900"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 8796.576834901625,
            "unit": "avg block_count",
            "extra": "max block_count: 9487.0, count: 58450"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 117.99929854576561,
            "unit": "avg segment_count",
            "extra": "max segment_count: 359.0, count: 58450"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 5.869989141144621,
            "unit": "avg cpu",
            "extra": "max cpu: 19.512194, count: 58450"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 121.32559465889649,
            "unit": "avg mem",
            "extra": "max mem: 132.453125, count: 58450"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 271.0023365635679,
            "unit": "avg tps",
            "extra": "max tps: 298.9696901087139, count: 58450"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 14.360337966409158,
            "unit": "avg cpu",
            "extra": "max cpu: 29.62963, count: 58450"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 93.94771365750641,
            "unit": "avg mem",
            "extra": "max mem: 98.140625, count: 58450"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 21.104392632890004,
            "unit": "avg tps",
            "extra": "max tps: 1753.2970751498192, count: 58450"
          }
        ]
      }
    ],
    "pg_search wide-table.toml Performance": [
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
        "date": 1752156060223,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 19.66513412462908,
            "unit": "avg cpu",
            "extra": "max cpu: 49.68944, count: 59119"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 172.62086400945552,
            "unit": "avg mem",
            "extra": "max mem: 177.8125, count: 59119"
          },
          {
            "name": "Bulk Update - Primary - tps",
            "value": 38.787437915137296,
            "unit": "avg tps",
            "extra": "max tps: 39.429077161025795, count: 59119"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 22013.62912092559,
            "unit": "avg block_count",
            "extra": "max block_count: 30180.0, count: 59119"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 54.06072497843333,
            "unit": "avg segment_count",
            "extra": "max segment_count: 148.0, count: 59119"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 8.852621651535877,
            "unit": "avg cpu",
            "extra": "max cpu: 34.355827, count: 59119"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 159.67353848804953,
            "unit": "avg mem",
            "extra": "max mem: 175.53125, count: 59119"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 203.02218911934162,
            "unit": "avg tps",
            "extra": "max tps: 224.54740207412596, count: 59119"
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
        "date": 1752156062237,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 21.57444871056124,
            "unit": "avg cpu",
            "extra": "max cpu: 49.382717, count: 59103"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 175.2912968144595,
            "unit": "avg mem",
            "extra": "max mem: 178.91796875, count: 59103"
          },
          {
            "name": "Bulk Update - Primary - tps",
            "value": 19.961384189322334,
            "unit": "avg tps",
            "extra": "max tps: 30.85109107326987, count: 59103"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 7996.705192629815,
            "unit": "avg block_count",
            "extra": "max block_count: 9514.0, count: 59103"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 42.99671759470755,
            "unit": "avg segment_count",
            "extra": "max segment_count: 91.0, count: 59103"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 13.499628383399536,
            "unit": "avg cpu",
            "extra": "max cpu: 38.787876, count: 59103"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 163.9098259209558,
            "unit": "avg mem",
            "extra": "max mem: 177.03125, count: 59103"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 111.40455185647092,
            "unit": "avg tps",
            "extra": "max tps: 114.90431896051523, count: 59103"
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
        "date": 1752156062976,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 19.49478776843815,
            "unit": "avg cpu",
            "extra": "max cpu: 49.079754, count: 59121"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 171.72761132888482,
            "unit": "avg mem",
            "extra": "max mem: 175.53515625, count: 59121"
          },
          {
            "name": "Bulk Update - Primary - tps",
            "value": 38.262295750957584,
            "unit": "avg tps",
            "extra": "max tps: 38.981558510785796, count: 59121"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 18419.098273033273,
            "unit": "avg block_count",
            "extra": "max block_count: 20544.0, count: 59121"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 53.51628017117437,
            "unit": "avg segment_count",
            "extra": "max segment_count: 147.0, count: 59121"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 8.935382735396159,
            "unit": "avg cpu",
            "extra": "max cpu: 31.372551, count: 59121"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 158.25416697059845,
            "unit": "avg mem",
            "extra": "max mem: 174.9375, count: 59121"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 199.65894751537644,
            "unit": "avg tps",
            "extra": "max tps: 215.8753897753016, count: 59121"
          }
        ]
      }
    ]
  }
}