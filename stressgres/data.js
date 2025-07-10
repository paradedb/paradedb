window.BENCHMARK_DATA = {
  "lastUpdate": 1752156054303,
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
      }
    ]
  }
}