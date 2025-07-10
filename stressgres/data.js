window.BENCHMARK_DATA = {
  "lastUpdate": 1752156058663,
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
      }
    ]
  }
}