window.BENCHMARK_DATA = {
  "lastUpdate": 1752157565948,
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
        "date": 1752156078434,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 20.089708908878308,
            "unit": "avg cpu",
            "extra": "max cpu: 44.444447, count: 59043"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 227.8767857086149,
            "unit": "avg mem",
            "extra": "max mem: 238.15625, count: 59043"
          },
          {
            "name": "Bulk Update - Primary - tps",
            "value": 7.427505481510032,
            "unit": "avg tps",
            "extra": "max tps: 10.881456698477175, count: 59043"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 21.52508312530768,
            "unit": "avg cpu",
            "extra": "max cpu: 34.146343, count: 59043"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 161.9555575491591,
            "unit": "avg mem",
            "extra": "max mem: 163.8828125, count: 59043"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 7.173672176142664,
            "unit": "avg tps",
            "extra": "max tps: 8.589139969746464, count: 59043"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 22402.021763799265,
            "unit": "avg block_count",
            "extra": "max block_count: 24998.0, count: 59043"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 32.27623935098149,
            "unit": "avg segment_count",
            "extra": "max segment_count: 63.0, count: 59043"
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
        "date": 1752156080653,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.089293987189457,
            "unit": "avg cpu",
            "extra": "max cpu: 44.444447, count: 59033"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 178.77030299789948,
            "unit": "avg mem",
            "extra": "max mem: 183.75, count: 59033"
          },
          {
            "name": "Bulk Update - Primary - tps",
            "value": 5.12600317257181,
            "unit": "avg tps",
            "extra": "max tps: 11.31570841179324, count: 59033"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 20.843774414260864,
            "unit": "avg cpu",
            "extra": "max cpu: 33.939396, count: 59033"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 162.15587964305982,
            "unit": "avg mem",
            "extra": "max mem: 163.265625, count: 59033"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 7.855921583467932,
            "unit": "avg tps",
            "extra": "max tps: 10.199975246725893, count: 59033"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 20338,
            "unit": "avg block_count",
            "extra": "max block_count: 20338.0, count: 59033"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 24.670726542781157,
            "unit": "avg segment_count",
            "extra": "max segment_count: 44.0, count: 59033"
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
        "date": 1752156086978,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.221586583430536,
            "unit": "avg cpu",
            "extra": "max cpu: 44.17178, count: 59038"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 183.98222304913784,
            "unit": "avg mem",
            "extra": "max mem: 184.546875, count: 59038"
          },
          {
            "name": "Bulk Update - Primary - tps",
            "value": 5.120284884626034,
            "unit": "avg tps",
            "extra": "max tps: 11.321548040961526, count: 59038"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 20.860158132987852,
            "unit": "avg cpu",
            "extra": "max cpu: 34.355827, count: 59038"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 160.939688540114,
            "unit": "avg mem",
            "extra": "max mem: 163.5625, count: 59038"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 7.773519282015295,
            "unit": "avg tps",
            "extra": "max tps: 10.12616732732971, count: 59038"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 20344,
            "unit": "avg block_count",
            "extra": "max block_count: 20344.0, count: 59038"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 24.78132728073444,
            "unit": "avg segment_count",
            "extra": "max segment_count: 44.0, count: 59038"
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
        "date": 1752156088695,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 20.19384724084506,
            "unit": "avg cpu",
            "extra": "max cpu: 44.17178, count: 59043"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 227.66881301022136,
            "unit": "avg mem",
            "extra": "max mem: 239.328125, count: 59043"
          },
          {
            "name": "Bulk Update - Primary - tps",
            "value": 7.431964286465918,
            "unit": "avg tps",
            "extra": "max tps: 10.852994834062413, count: 59043"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 21.44578361946624,
            "unit": "avg cpu",
            "extra": "max cpu: 34.355827, count: 59043"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 161.76262374455905,
            "unit": "avg mem",
            "extra": "max mem: 163.8125, count: 59043"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 7.160045143943519,
            "unit": "avg tps",
            "extra": "max tps: 8.51684874352496, count: 59043"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 21484.46408549701,
            "unit": "avg block_count",
            "extra": "max block_count: 22842.0, count: 59043"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 32.347898988872515,
            "unit": "avg segment_count",
            "extra": "max segment_count: 64.0, count: 59043"
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
        "date": 1752156091485,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.201287304304092,
            "unit": "avg cpu",
            "extra": "max cpu: 43.90244, count: 59046"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 183.6715611557091,
            "unit": "avg mem",
            "extra": "max mem: 183.9453125, count: 59046"
          },
          {
            "name": "Bulk Update - Primary - tps",
            "value": 5.146699917344818,
            "unit": "avg tps",
            "extra": "max tps: 11.628145845410375, count: 59046"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 20.668450236949653,
            "unit": "avg cpu",
            "extra": "max cpu: 34.355827, count: 59046"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 162.5577640870042,
            "unit": "avg mem",
            "extra": "max mem: 164.05859375, count: 59046"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 7.863629491521986,
            "unit": "avg tps",
            "extra": "max tps: 10.242559003087196, count: 59046"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 20208,
            "unit": "avg block_count",
            "extra": "max block_count: 20208.0, count: 59046"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 24.76062730752295,
            "unit": "avg segment_count",
            "extra": "max segment_count: 45.0, count: 59046"
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
        "date": 1752156092420,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 20.312897405923763,
            "unit": "avg cpu",
            "extra": "max cpu: 44.17178, count: 59043"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 227.23124771088445,
            "unit": "avg mem",
            "extra": "max mem: 237.81640625, count: 59043"
          },
          {
            "name": "Bulk Update - Primary - tps",
            "value": 7.289390535392586,
            "unit": "avg tps",
            "extra": "max tps: 10.696052901732074, count: 59043"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 21.435771561438845,
            "unit": "avg cpu",
            "extra": "max cpu: 34.146343, count: 59043"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 160.1599795382179,
            "unit": "avg mem",
            "extra": "max mem: 163.40625, count: 59043"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 7.190602004018975,
            "unit": "avg tps",
            "extra": "max tps: 8.674354297446003, count: 59043"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 21813.86530155988,
            "unit": "avg block_count",
            "extra": "max block_count: 22693.0, count: 59043"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 31.855410463560457,
            "unit": "avg segment_count",
            "extra": "max segment_count: 63.0, count: 59043"
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
        "date": 1752156170977,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.119449258287602,
            "unit": "avg cpu",
            "extra": "max cpu: 59.428574, count: 59065"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 400.15144577213664,
            "unit": "avg mem",
            "extra": "max mem: 464.94140625, count: 59065"
          },
          {
            "name": "Bulk Update - Primary - tps",
            "value": 7.511571237308383,
            "unit": "avg tps",
            "extra": "max tps: 11.401042151131193, count: 59065"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 18.217537490631223,
            "unit": "avg cpu",
            "extra": "max cpu: 34.355827, count: 59065"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 174.78884842175992,
            "unit": "avg mem",
            "extra": "max mem: 211.69921875, count: 59065"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 4.356369298185254,
            "unit": "avg tps",
            "extra": "max tps: 4.688107779635526, count: 59065"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 31493.29208499111,
            "unit": "avg block_count",
            "extra": "max block_count: 35238.0, count: 59065"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 28.648472022348262,
            "unit": "avg segment_count",
            "extra": "max segment_count: 64.0, count: 59065"
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
        "date": 1752156968318,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 20.394104839586014,
            "unit": "avg cpu",
            "extra": "max cpu: 44.444447, count: 44585"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 227.52560803801728,
            "unit": "avg mem",
            "extra": "max mem: 232.34765625, count: 44585"
          },
          {
            "name": "Bulk Update - Primary - tps",
            "value": 6.535510428853111,
            "unit": "avg tps",
            "extra": "max tps: 10.307144012527926, count: 44585"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 21.516546736266527,
            "unit": "avg cpu",
            "extra": "max cpu: 34.355827, count: 44585"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 162.79924354463384,
            "unit": "avg mem",
            "extra": "max mem: 163.44921875, count: 44585"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 6.891379748342331,
            "unit": "avg tps",
            "extra": "max tps: 8.400446017692609, count: 44585"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 21814.77007962319,
            "unit": "avg block_count",
            "extra": "max block_count: 24198.0, count: 44585"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 31.812021980486712,
            "unit": "avg segment_count",
            "extra": "max segment_count: 64.0, count: 44585"
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
        "date": 1752157558095,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 20.28991310484392,
            "unit": "avg cpu",
            "extra": "max cpu: 44.444447, count: 59026"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 226.40699152968608,
            "unit": "avg mem",
            "extra": "max mem: 239.0546875, count: 59026"
          },
          {
            "name": "Bulk Update - Primary - tps",
            "value": 7.379895584048198,
            "unit": "avg tps",
            "extra": "max tps: 10.82041793457817, count: 59026"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 21.547545289995423,
            "unit": "avg cpu",
            "extra": "max cpu: 34.5679, count: 59026"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 162.36236011196337,
            "unit": "avg mem",
            "extra": "max mem: 164.0390625, count: 59026"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 7.225337879046261,
            "unit": "avg tps",
            "extra": "max tps: 8.592141118595144, count: 59026"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 21814.209788906584,
            "unit": "avg block_count",
            "extra": "max block_count: 22644.0, count: 59026"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 32.167095856063426,
            "unit": "avg segment_count",
            "extra": "max segment_count: 63.0, count: 59026"
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
        "date": 1752157565093,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.20547170496527,
            "unit": "avg cpu",
            "extra": "max cpu: 44.444447, count: 59038"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 183.69742898641553,
            "unit": "avg mem",
            "extra": "max mem: 184.2578125, count: 59038"
          },
          {
            "name": "Bulk Update - Primary - tps",
            "value": 5.171001159012762,
            "unit": "avg tps",
            "extra": "max tps: 11.706381110883123, count: 59038"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 20.664194972668557,
            "unit": "avg cpu",
            "extra": "max cpu: 34.355827, count: 59038"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 162.00521810370017,
            "unit": "avg mem",
            "extra": "max mem: 163.90234375, count: 59038"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 7.896384940406813,
            "unit": "avg tps",
            "extra": "max tps: 10.251083904325922, count: 59038"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 20226,
            "unit": "avg block_count",
            "extra": "max block_count: 20226.0, count: 59038"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 24.87447067990108,
            "unit": "avg segment_count",
            "extra": "max segment_count: 45.0, count: 59038"
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
        "date": 1752156066667,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 7.2226627422042124,
            "unit": "avg cpu",
            "extra": "max cpu: 29.813665, count: 58417"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 109.17942537596076,
            "unit": "avg mem",
            "extra": "max mem: 114.546875, count: 58417"
          },
          {
            "name": "Custom Scan - Primary - tps",
            "value": 447.02268668058394,
            "unit": "avg tps",
            "extra": "max tps: 604.815021516931, count: 58417"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.7552733695733655,
            "unit": "avg cpu",
            "extra": "max cpu: 9.937888, count: 58417"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 90.44987332454593,
            "unit": "avg mem",
            "extra": "max mem: 95.046875, count: 58417"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2783.7990978121124,
            "unit": "avg tps",
            "extra": "max tps: 3032.0371075419853, count: 58417"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 7.327300746321173,
            "unit": "avg cpu",
            "extra": "max cpu: 29.268291, count: 58417"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 109.93126505875858,
            "unit": "avg mem",
            "extra": "max mem: 115.16796875, count: 58417"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 446.763200956646,
            "unit": "avg tps",
            "extra": "max tps: 601.4003734305936, count: 58417"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.771693090373095,
            "unit": "avg cpu",
            "extra": "max cpu: 5.0314465, count: 58417"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 106.62466472195594,
            "unit": "avg mem",
            "extra": "max mem: 108.89453125, count: 58417"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 403.5867668565542,
            "unit": "avg tps",
            "extra": "max tps: 502.98213075503355, count: 58417"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 7.138664474318655,
            "unit": "avg cpu",
            "extra": "max cpu: 25.157234, count: 116834"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 118.0119674000612,
            "unit": "avg mem",
            "extra": "max mem: 124.60546875, count: 116834"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 300.81905881853027,
            "unit": "avg tps",
            "extra": "max tps: 342.452236348122, count: 116834"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 8470.773935669411,
            "unit": "avg block_count",
            "extra": "max block_count: 8533.0, count: 58417"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 117.92332711368266,
            "unit": "avg segment_count",
            "extra": "max segment_count: 394.0, count: 58417"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 5.705749662076507,
            "unit": "avg cpu",
            "extra": "max cpu: 16.393442, count: 58417"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 118.21212842580071,
            "unit": "avg mem",
            "extra": "max mem: 124.453125, count: 58417"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 263.39095156676524,
            "unit": "avg tps",
            "extra": "max tps: 298.80409583366617, count: 58417"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 16.68702200437984,
            "unit": "avg cpu",
            "extra": "max cpu: 29.813665, count: 58417"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 90.44109564638718,
            "unit": "avg mem",
            "extra": "max mem: 97.99609375, count: 58417"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 21.367028826749205,
            "unit": "avg tps",
            "extra": "max tps: 1592.15133079969, count: 58417"
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
        "date": 1752156096693,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 8.267928443541447,
            "unit": "avg cpu",
            "extra": "max cpu: 24.539877, count: 58356"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 107.06749459941994,
            "unit": "avg mem",
            "extra": "max mem: 110.46484375, count: 58356"
          },
          {
            "name": "Custom Scan - Primary - tps",
            "value": 337.02661962705224,
            "unit": "avg tps",
            "extra": "max tps: 546.3504182312452, count: 58356"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.929281213014016,
            "unit": "avg cpu",
            "extra": "max cpu: 9.696969, count: 58356"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 91.40668322451847,
            "unit": "avg mem",
            "extra": "max mem: 93.640625, count: 58356"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2137.9522436296597,
            "unit": "avg tps",
            "extra": "max tps: 2775.2585560961106, count: 58356"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 8.430227269560884,
            "unit": "avg cpu",
            "extra": "max cpu: 26.016258, count: 58356"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 107.9117679897954,
            "unit": "avg mem",
            "extra": "max mem: 110.921875, count: 58356"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 336.38070075657004,
            "unit": "avg tps",
            "extra": "max tps: 538.4683443907568, count: 58356"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.754037055621946,
            "unit": "avg cpu",
            "extra": "max cpu: 9.756097, count: 58356"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 104.56001538509322,
            "unit": "avg mem",
            "extra": "max mem: 106.50390625, count: 58356"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 288.70417291162397,
            "unit": "avg tps",
            "extra": "max tps: 427.8277478210412, count: 58356"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 14.156104658440233,
            "unit": "avg cpu",
            "extra": "max cpu: 53.98773, count: 116712"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 149.48631376673993,
            "unit": "avg mem",
            "extra": "max mem: 178.2578125, count: 116712"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 156.35047385194113,
            "unit": "avg tps",
            "extra": "max tps: 237.17346001765776, count: 116712"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 8046.387894989375,
            "unit": "avg block_count",
            "extra": "max block_count: 8109.0, count: 58356"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 117.16915141545,
            "unit": "avg segment_count",
            "extra": "max segment_count: 252.0, count: 58356"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.788231981635036,
            "unit": "avg cpu",
            "extra": "max cpu: 35.820896, count: 58356"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 145.79575974139334,
            "unit": "avg mem",
            "extra": "max mem: 168.84375, count: 58356"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 139.74748599205802,
            "unit": "avg tps",
            "extra": "max tps: 162.78054595283538, count: 58356"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 8.666228118458228,
            "unit": "avg cpu",
            "extra": "max cpu: 19.753086, count: 58356"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 94.35054952307817,
            "unit": "avg mem",
            "extra": "max mem: 102.3359375, count: 58356"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 14.42893912946626,
            "unit": "avg tps",
            "extra": "max tps: 1770.9364534872395, count: 58356"
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
        "date": 1752157087112,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 6.841011180185005,
            "unit": "avg cpu",
            "extra": "max cpu: 25.0, count: 58468"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 91.24731543857324,
            "unit": "avg mem",
            "extra": "max mem: 104.7421875, count: 58468"
          },
          {
            "name": "Custom Scan - Primary - tps",
            "value": 495.24323093922095,
            "unit": "avg tps",
            "extra": "max tps: 696.9332574187955, count: 58468"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.70864276158034,
            "unit": "avg cpu",
            "extra": "max cpu: 9.81595, count: 58468"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 81.41914727928098,
            "unit": "avg mem",
            "extra": "max mem: 93.421875, count: 58468"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3025.842900595129,
            "unit": "avg tps",
            "extra": "max tps: 3429.3039374515038, count: 58468"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 6.798296873995379,
            "unit": "avg cpu",
            "extra": "max cpu: 29.090908, count: 58468"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 92.15484678852107,
            "unit": "avg mem",
            "extra": "max mem: 105.765625, count: 58468"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 495.27867167782193,
            "unit": "avg tps",
            "extra": "max tps: 699.8854483251769, count: 58468"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.830757606594548,
            "unit": "avg cpu",
            "extra": "max cpu: 9.81595, count: 58468"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 88.80505174839229,
            "unit": "avg mem",
            "extra": "max mem: 101.23046875, count: 58468"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 444.31681087456457,
            "unit": "avg tps",
            "extra": "max tps: 575.2625814816301, count: 58468"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 6.938359434867679,
            "unit": "avg cpu",
            "extra": "max cpu: 25.157234, count: 116936"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 95.6618804837689,
            "unit": "avg mem",
            "extra": "max mem: 111.69140625, count: 116936"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 317.67574093424304,
            "unit": "avg tps",
            "extra": "max tps: 321.735167284884, count: 116936"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 6689.9266607374975,
            "unit": "avg block_count",
            "extra": "max block_count: 8108.0, count: 58468"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 117.87045905452555,
            "unit": "avg segment_count",
            "extra": "max segment_count: 426.0, count: 58468"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 5.669654594134737,
            "unit": "avg cpu",
            "extra": "max cpu: 14.906833, count: 58468"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 98.40268454806476,
            "unit": "avg mem",
            "extra": "max mem: 113.6328125, count: 58468"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 283.7146522583016,
            "unit": "avg tps",
            "extra": "max tps: 291.15784333703454, count: 58468"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 14.345670207068826,
            "unit": "avg cpu",
            "extra": "max cpu: 29.62963, count: 58468"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 80.55820518274953,
            "unit": "avg mem",
            "extra": "max mem: 95.4453125, count: 58468"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 20.42529061783705,
            "unit": "avg tps",
            "extra": "max tps: 1695.650994329743, count: 58468"
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
        "date": 1752157509946,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 6.758660806053241,
            "unit": "avg cpu",
            "extra": "max cpu: 30.000002, count: 58495"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 97.49843716610394,
            "unit": "avg mem",
            "extra": "max mem: 101.12890625, count: 58495"
          },
          {
            "name": "Custom Scan - Primary - tps",
            "value": 501.779437037536,
            "unit": "avg tps",
            "extra": "max tps: 694.7140059077066, count: 58495"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.720576234724999,
            "unit": "avg cpu",
            "extra": "max cpu: 9.876543, count: 58495"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 85.97405881378751,
            "unit": "avg mem",
            "extra": "max mem: 89.02734375, count: 58495"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3072.3418752687144,
            "unit": "avg tps",
            "extra": "max tps: 3518.6819417464794, count: 58495"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 6.8496225314962045,
            "unit": "avg cpu",
            "extra": "max cpu: 25.0, count: 58495"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 98.1499376949953,
            "unit": "avg mem",
            "extra": "max mem: 101.76953125, count: 58495"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 501.3892498905185,
            "unit": "avg tps",
            "extra": "max tps: 686.9813734548464, count: 58495"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.609705263016079,
            "unit": "avg cpu",
            "extra": "max cpu: 5.0, count: 58495"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 94.74034239037525,
            "unit": "avg mem",
            "extra": "max mem: 96.79296875, count: 58495"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 465.2070251526338,
            "unit": "avg tps",
            "extra": "max tps: 590.2987005872764, count: 58495"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 7.281292791090943,
            "unit": "avg cpu",
            "extra": "max cpu: 25.0, count: 116990"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 101.67500971637533,
            "unit": "avg mem",
            "extra": "max mem: 107.59765625, count: 116990"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 300.14730972611727,
            "unit": "avg tps",
            "extra": "max tps: 313.0760094569068, count: 116990"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 7480.566219334986,
            "unit": "avg block_count",
            "extra": "max block_count: 7564.0, count: 58495"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 117.79266603983247,
            "unit": "avg segment_count",
            "extra": "max segment_count: 371.0, count: 58495"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 5.893785552143856,
            "unit": "avg cpu",
            "extra": "max cpu: 15.000001, count: 58495"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 104.29055968993076,
            "unit": "avg mem",
            "extra": "max mem: 110.43359375, count: 58495"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 275.8414770024026,
            "unit": "avg tps",
            "extra": "max tps: 296.1885782023579, count: 58495"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 14.611720755035943,
            "unit": "avg cpu",
            "extra": "max cpu: 29.813665, count: 58495"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 87.64591251121891,
            "unit": "avg mem",
            "extra": "max mem: 90.6171875, count: 58495"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 17.904657738376418,
            "unit": "avg tps",
            "extra": "max tps: 495.0647000056437, count: 58495"
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
        "date": 1752156065773,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 19.45007552239405,
            "unit": "avg cpu",
            "extra": "max cpu: 49.382717, count: 59112"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 182.25461894634338,
            "unit": "avg mem",
            "extra": "max mem: 183.30859375, count: 59112"
          },
          {
            "name": "Bulk Update - Primary - tps",
            "value": 38.441011405934184,
            "unit": "avg tps",
            "extra": "max tps: 39.51849238040642, count: 59112"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 18113.662809581812,
            "unit": "avg block_count",
            "extra": "max block_count: 20157.0, count: 59112"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 54.03748815807281,
            "unit": "avg segment_count",
            "extra": "max segment_count: 154.0, count: 59112"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 9.189438694903778,
            "unit": "avg cpu",
            "extra": "max cpu: 34.146343, count: 59112"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 159.09955128876118,
            "unit": "avg mem",
            "extra": "max mem: 175.2734375, count: 59112"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 189.67710260004205,
            "unit": "avg tps",
            "extra": "max tps: 204.97936968290034, count: 59112"
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
        "date": 1752156064282,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 21.479224280852804,
            "unit": "avg cpu",
            "extra": "max cpu: 58.536583, count: 59095"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 174.48603605687876,
            "unit": "avg mem",
            "extra": "max mem: 178.25, count: 59095"
          },
          {
            "name": "Bulk Update - Primary - tps",
            "value": 20.064376959339967,
            "unit": "avg tps",
            "extra": "max tps: 30.30136200186079, count: 59095"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 8157.147745156105,
            "unit": "avg block_count",
            "extra": "max block_count: 9504.0, count: 59095"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 43.23013791352906,
            "unit": "avg segment_count",
            "extra": "max segment_count: 97.0, count: 59095"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 13.288322988968552,
            "unit": "avg cpu",
            "extra": "max cpu: 34.782608, count: 59095"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 163.54484100019036,
            "unit": "avg mem",
            "extra": "max mem: 176.625, count: 59095"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 111.59817541741016,
            "unit": "avg tps",
            "extra": "max tps: 116.31279018281052, count: 59095"
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
        "date": 1752156063005,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 19.728113525213548,
            "unit": "avg cpu",
            "extra": "max cpu: 49.68944, count: 59128"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 172.366654030768,
            "unit": "avg mem",
            "extra": "max mem: 182.71484375, count: 59128"
          },
          {
            "name": "Bulk Update - Primary - tps",
            "value": 38.05735231415804,
            "unit": "avg tps",
            "extra": "max tps: 38.81404158605944, count: 59128"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 17895.59122581518,
            "unit": "avg block_count",
            "extra": "max block_count: 19886.0, count: 59128"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 53.37104248410229,
            "unit": "avg segment_count",
            "extra": "max segment_count: 145.0, count: 59128"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 8.675975307715682,
            "unit": "avg cpu",
            "extra": "max cpu: 34.5679, count: 59128"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 159.98834619120805,
            "unit": "avg mem",
            "extra": "max mem: 175.09375, count: 59128"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 211.28102735133518,
            "unit": "avg tps",
            "extra": "max tps: 229.92448736767787, count: 59128"
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
        "date": 1752156074800,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 21.51157971205102,
            "unit": "avg cpu",
            "extra": "max cpu: 49.68944, count: 59098"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 175.11730746757505,
            "unit": "avg mem",
            "extra": "max mem: 184.08984375, count: 59098"
          },
          {
            "name": "Bulk Update - Primary - tps",
            "value": 20.081843596294416,
            "unit": "avg tps",
            "extra": "max tps: 31.9461527507645, count: 59098"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 7846.1962164540255,
            "unit": "avg block_count",
            "extra": "max block_count: 9017.0, count: 59098"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 42.9178144776473,
            "unit": "avg segment_count",
            "extra": "max segment_count: 84.0, count: 59098"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 13.502964659325778,
            "unit": "avg cpu",
            "extra": "max cpu: 44.720497, count: 59098"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 164.03387329097433,
            "unit": "avg mem",
            "extra": "max mem: 176.83984375, count: 59098"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 112.02076572485826,
            "unit": "avg tps",
            "extra": "max tps: 114.4914862157759, count: 59098"
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
        "date": 1752156089437,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 26.574514329340765,
            "unit": "avg cpu",
            "extra": "max cpu: 60.000004, count: 59123"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 241.57130057517801,
            "unit": "avg mem",
            "extra": "max mem: 264.98828125, count: 59123"
          },
          {
            "name": "Bulk Update - Primary - tps",
            "value": 37.662438548840825,
            "unit": "avg tps",
            "extra": "max tps: 38.26180125927395, count: 59123"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 17848.064222045567,
            "unit": "avg block_count",
            "extra": "max block_count: 19822.0, count: 59123"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 54.51857990967982,
            "unit": "avg segment_count",
            "extra": "max segment_count: 159.0, count: 59123"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 14.257746453279713,
            "unit": "avg cpu",
            "extra": "max cpu: 49.079754, count: 59123"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 230.38953577182315,
            "unit": "avg mem",
            "extra": "max mem: 279.6875, count: 59123"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 183.86334139916895,
            "unit": "avg tps",
            "extra": "max tps: 199.7260610296731, count: 59123"
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
        "date": 1752157089366,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 19.505552493711264,
            "unit": "avg cpu",
            "extra": "max cpu: 59.62733, count: 59098"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 182.13612168823394,
            "unit": "avg mem",
            "extra": "max mem: 182.92578125, count: 59098"
          },
          {
            "name": "Bulk Update - Primary - tps",
            "value": 39.44144521406265,
            "unit": "avg tps",
            "extra": "max tps: 40.59281370994881, count: 59098"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 21971.04076280077,
            "unit": "avg block_count",
            "extra": "max block_count: 29987.0, count: 59098"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 54.94077633760872,
            "unit": "avg segment_count",
            "extra": "max segment_count: 146.0, count: 59098"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 9.086377800506979,
            "unit": "avg cpu",
            "extra": "max cpu: 34.5679, count: 59098"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 160.44632690351196,
            "unit": "avg mem",
            "extra": "max mem: 174.88671875, count: 59098"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 195.9097419890316,
            "unit": "avg tps",
            "extra": "max tps: 218.23101698056374, count: 59098"
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
        "date": 1752157531056,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 20.49039620039785,
            "unit": "avg cpu",
            "extra": "max cpu: 49.68944, count: 59101"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 172.52664651930593,
            "unit": "avg mem",
            "extra": "max mem: 176.0234375, count: 59101"
          },
          {
            "name": "Bulk Update - Primary - tps",
            "value": 32.9353351390796,
            "unit": "avg tps",
            "extra": "max tps: 33.56502133272702, count: 59101"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 15963.72063078459,
            "unit": "avg block_count",
            "extra": "max block_count: 17704.0, count: 59101"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 49.75289758210521,
            "unit": "avg segment_count",
            "extra": "max segment_count: 142.0, count: 59101"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 13.31298848150347,
            "unit": "avg cpu",
            "extra": "max cpu: 34.5679, count: 59101"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 159.43109174337152,
            "unit": "avg mem",
            "extra": "max mem: 176.15234375, count: 59101"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 108.43515174854694,
            "unit": "avg tps",
            "extra": "max tps: 114.81344714163264, count: 59101"
          }
        ]
      }
    ]
  }
}