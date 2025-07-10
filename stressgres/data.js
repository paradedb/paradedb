window.BENCHMARK_DATA = {
  "lastUpdate": 1752175928033,
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
            "name": "Philippe Noël",
            "username": "philippemnoel",
            "email": "philippemnoel@gmail.com"
          },
          "id": "ce8e33ae49785f0afe220ca985de3d0c7c270503",
          "message": "chore: more improvements to index/schema configuration and management (#2771)\n\n## What\n\n#2660 brought a much needed round of cleanups to how we manage index\nschemas. Unfortunately, it introduced quite some overhead in\nreading/decoding/validating the schema. This process was happening quite\na bit throughout the execution paths of `aminsert` and other hot-spots.\n\n#2176 brought the ability to essentially keep one heavy-weight\n`PgSearchRelation` instantiated and cheaply clone it when necessary.\nThis PR cleans up things further such that the `SearchIndexSchema` is\nnow a lazily-evaluated property of `PgSearchRelation`. This means\n`SearchIndexSchema` is only evaluated when needed, and then only once\n(at least per statement).\n\nFurthermore, its internal properties are lazily-evaluated, ensuring any\ngiven code path doesn't do more work than it needs.\n\nThis also renames `SearchIndexOptions` to `BM25IndexOptions`, mainly\nbecause I kept getting confused about what `SearchIndexOptions`\nrepresented (it was too similarly named to `SearchIndexSchema` for my\ntastes). And `BM25IndexOptions` is now a property of `PgSearchRelation`\ntoo.\n\nThis seems to have drastically improved the write throughput of the\nINSERT/UPDATE jobs in our `single-server.toml` stressgress test.\nv0.15.26 was 176/s INSERTs and 154/s UPDATEs. This PR clocks in at 275/s\nand 260/s, respectively.\n\n# Other Notable Changes\n\n- Index configuration validation now happens during CREATE INDEX/REINDEX\nin `ambuildempty()` rather than on every instantiation of\n`SearchIndexSchema`.\n\n- The \"raw\" tokenizer deprecation warnings are now gone, unless somehow\nthe \"key_field\" is configured with it -- which is no longer possible\n\n## Why\n\nTrying to rollback performance regressions that were introduced in\n0.16.0\n\n## How\n\n## Tests\n\nAll existing tests pass, and a few were updated due to the \"raw\"\ntokenizer deprecation warning going away and a change in wording for a\nspecific validation error.",
          "timestamp": "2025-07-05T15:13:47Z",
          "url": "https://github.com/paradedb/paradedb/commit/ce8e33ae49785f0afe220ca985de3d0c7c270503"
        },
        "date": 1752173253157,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 10.343793727986103,
            "unit": "median tps",
            "extra": "avg tps: 8.977545702891543, max tps: 13.376704529902339, count: 58866"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 8.40597691919667,
            "unit": "median tps",
            "extra": "avg tps: 7.72393835063471, max tps: 9.323733848048539, count: 58866"
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
        "date": 1752173862149,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 2.9797112188046495,
            "unit": "median tps",
            "extra": "avg tps: 3.33780273086121, max tps: 5.734723590136291, count: 15480"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.273863875559506,
            "unit": "median tps",
            "extra": "avg tps: 5.48057418223913, max tps: 6.712912516202362, count: 15480"
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
        "date": 1752174407602,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 5.7708725646505865,
            "unit": "median tps",
            "extra": "avg tps: 5.908353171193552, max tps: 16.080190800527227, count: 58833"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 9.337969764171762,
            "unit": "median tps",
            "extra": "avg tps: 8.496159102907765, max tps: 11.206568322166026, count: 58833"
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
        "date": 1752174520220,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 10.567632900874756,
            "unit": "median tps",
            "extra": "avg tps: 9.155972323103592, max tps: 13.696306378749842, count: 58863"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 7.809102465130026,
            "unit": "median tps",
            "extra": "avg tps: 7.2266700187753505, max tps: 8.591847981863069, count: 58863"
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
        "date": 1752174766996,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 5.895874973885984,
            "unit": "median tps",
            "extra": "avg tps: 6.016952833927826, max tps: 15.215700580481318, count: 58855"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 9.605162122652562,
            "unit": "median tps",
            "extra": "avg tps: 8.708807188119051, max tps: 11.475794774017595, count: 58855"
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
        "date": 1752174919115,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 5.848595067371667,
            "unit": "median tps",
            "extra": "avg tps: 5.949310035392203, max tps: 14.900898314753448, count: 58838"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 9.415740906533701,
            "unit": "median tps",
            "extra": "avg tps: 8.54543908043576, max tps: 11.34969016197247, count: 58838"
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
        "date": 1752175301544,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 10.065248048913666,
            "unit": "median tps",
            "extra": "avg tps: 8.770651182482279, max tps: 13.250236607208217, count: 58845"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 8.418583493652815,
            "unit": "median tps",
            "extra": "avg tps: 7.734302646182228, max tps: 9.265874810991182, count: 58845"
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
        "date": 1752175481889,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 10.364295889170931,
            "unit": "median tps",
            "extra": "avg tps: 8.969474012726929, max tps: 13.393801994747202, count: 58851"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 8.243601732286322,
            "unit": "median tps",
            "extra": "avg tps: 7.574151600991649, max tps: 9.18397307665833, count: 58851"
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
        "date": 1752175763203,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 5.888781508716493,
            "unit": "median tps",
            "extra": "avg tps: 6.0246382303072625, max tps: 15.975239852865585, count: 58859"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 9.395292432507823,
            "unit": "median tps",
            "extra": "avg tps: 8.535472932221664, max tps: 11.234030719287158, count: 58859"
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
        "date": 1752175926759,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 10.10687938257046,
            "unit": "median tps",
            "extra": "avg tps: 8.811606633061881, max tps: 13.252575962279982, count: 58848"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 8.44230395997256,
            "unit": "median tps",
            "extra": "avg tps: 7.761977267123515, max tps: 9.332853759357228, count: 58848"
          }
        ]
      }
    ],
    "pg_search single-server.toml Performance - TPS": [
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
        "date": 1752173254649,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 594.1996781625423,
            "unit": "median tps",
            "extra": "avg tps: 597.5986931079183, max tps: 838.8337806305445, count: 57674"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2676.435195013042,
            "unit": "median tps",
            "extra": "avg tps: 2649.0959512451527, max tps: 2944.3630722737676, count: 57674"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 591.2267878333973,
            "unit": "median tps",
            "extra": "avg tps: 595.349336909976, max tps: 851.7202045856709, count: 57674"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 547.5736917950243,
            "unit": "median tps",
            "extra": "avg tps: 548.500980719966, max tps: 679.5816169366962, count: 57674"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 328.4160110169999,
            "unit": "median tps",
            "extra": "avg tps: 327.94481900108536, max tps: 336.1432424224372, count: 115348"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 303.5239485682182,
            "unit": "median tps",
            "extra": "avg tps: 302.80342959841914, max tps: 314.9353064149445, count: 57674"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 12.897728028034766,
            "unit": "median tps",
            "extra": "avg tps: 19.404932844662685, max tps: 1673.7298064498852, count: 57674"
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
        "date": 1752174245449,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 587.1316558436688,
            "unit": "median tps",
            "extra": "avg tps: 589.227719220513, max tps: 852.827966294409, count: 57662"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2476.5775164854135,
            "unit": "median tps",
            "extra": "avg tps: 2343.602433971075, max tps: 2984.8078427026853, count: 57662"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 585.6424347032557,
            "unit": "median tps",
            "extra": "avg tps: 586.9771033588031, max tps: 833.5181659866408, count: 57662"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 539.4718227922251,
            "unit": "median tps",
            "extra": "avg tps: 537.9606092323903, max tps: 734.3843989728313, count: 57662"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 309.6451452617598,
            "unit": "median tps",
            "extra": "avg tps: 304.1759777619652, max tps: 316.51075925233954, count: 115324"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 284.01598556179067,
            "unit": "median tps",
            "extra": "avg tps: 277.48811790346684, max tps: 306.62683079102266, count: 57662"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 11.92900477156219,
            "unit": "median tps",
            "extra": "avg tps: 19.437826895632117, max tps: 2104.337249504955, count: 57662"
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
        "date": 1752174332751,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 527.2251660818677,
            "unit": "median tps",
            "extra": "avg tps: 530.2993042344682, max tps: 736.3882618565567, count: 57650"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2316.233055794605,
            "unit": "median tps",
            "extra": "avg tps: 2206.3708337386406, max tps: 2690.3423820020535, count: 57650"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 525.8356033875743,
            "unit": "median tps",
            "extra": "avg tps: 527.6249625170971, max tps: 680.2649381443993, count: 57650"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 473.42180948460697,
            "unit": "median tps",
            "extra": "avg tps: 472.99389487602235, max tps: 621.9153387893136, count: 57650"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 294.6189896666312,
            "unit": "median tps",
            "extra": "avg tps: 305.126886934283, max tps: 360.84940047797915, count: 115300"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 270.7958260631609,
            "unit": "median tps",
            "extra": "avg tps: 268.4037156791376, max tps: 276.9729010552919, count: 57650"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 10.91597479832494,
            "unit": "median tps",
            "extra": "avg tps: 20.268233145728463, max tps: 2042.2125330583153, count: 57650"
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
        "date": 1752174494115,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 382.7694003210229,
            "unit": "median tps",
            "extra": "avg tps: 386.44963267634995, max tps: 670.3819614431333, count: 57576"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 1747.9940488602679,
            "unit": "median tps",
            "extra": "avg tps: 1644.5616652007232, max tps: 2563.9920979113012, count: 57576"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 382.3704725123645,
            "unit": "median tps",
            "extra": "avg tps: 385.90353450138196, max tps: 662.5960515801272, count: 57576"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 348.76117236345647,
            "unit": "median tps",
            "extra": "avg tps: 348.75546590513676, max tps: 564.6898850206691, count: 57576"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 143.9716324334865,
            "unit": "median tps",
            "extra": "avg tps: 152.541373633347, max tps: 239.5920731193148, count: 115152"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 140.13730453482876,
            "unit": "median tps",
            "extra": "avg tps: 137.08673463545293, max tps: 144.34090994306726, count: 57576"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 8.53909327553412,
            "unit": "median tps",
            "extra": "avg tps: 15.96119242183386, max tps: 2023.5869292473067, count: 57576"
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
        "date": 1752174510664,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 534.1656923999349,
            "unit": "median tps",
            "extra": "avg tps: 536.2679276088858, max tps: 709.3456615753084, count: 57671"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2264.2205665965876,
            "unit": "median tps",
            "extra": "avg tps: 2133.971286365394, max tps: 2848.2999565671735, count: 57671"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 535.3509449084279,
            "unit": "median tps",
            "extra": "avg tps: 539.8446882390008, max tps: 754.6099276246695, count: 57671"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 489.0989675280284,
            "unit": "median tps",
            "extra": "avg tps: 491.555604563131, max tps: 696.7071945886473, count: 57671"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 309.07464031177176,
            "unit": "median tps",
            "extra": "avg tps: 306.1693722698981, max tps: 320.7520246997721, count: 115342"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 280.74389191619395,
            "unit": "median tps",
            "extra": "avg tps: 278.7619509286631, max tps: 313.33005052245164, count: 57671"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 12.491350515399708,
            "unit": "median tps",
            "extra": "avg tps: 20.615037334631563, max tps: 2059.880732905565, count: 57671"
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
        "date": 1752174987437,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 539.2597011279865,
            "unit": "median tps",
            "extra": "avg tps: 543.5082743494074, max tps: 765.7223558458469, count: 57677"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2216.673976051388,
            "unit": "median tps",
            "extra": "avg tps: 2096.6455390311694, max tps: 2837.306356678463, count: 57677"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 535.0071744502883,
            "unit": "median tps",
            "extra": "avg tps: 539.7266468017781, max tps: 763.2855328775252, count: 57677"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 493.21871093230095,
            "unit": "median tps",
            "extra": "avg tps: 493.2784096601062, max tps: 625.2778969453236, count: 57677"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 299.416318155613,
            "unit": "median tps",
            "extra": "avg tps: 297.7972546853404, max tps: 343.09746065205735, count: 115354"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 273.5956854407689,
            "unit": "median tps",
            "extra": "avg tps: 270.0478263931931, max tps: 279.1112843863153, count: 57677"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 15.137398135137493,
            "unit": "median tps",
            "extra": "avg tps: 23.08903966586893, max tps: 2163.5889613691193, count: 57677"
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
        "date": 1752175362840,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 579.825403281405,
            "unit": "median tps",
            "extra": "avg tps: 583.3125790660972, max tps: 822.0735837763159, count: 57648"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2503.334483287761,
            "unit": "median tps",
            "extra": "avg tps: 2405.181286370046, max tps: 2962.9290576700423, count: 57648"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 577.9041976052158,
            "unit": "median tps",
            "extra": "avg tps: 581.1825681542821, max tps: 777.5888636833299, count: 57648"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 542.3528062818244,
            "unit": "median tps",
            "extra": "avg tps: 544.6081684529026, max tps: 693.7996443999302, count: 57648"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 303.3945501514716,
            "unit": "median tps",
            "extra": "avg tps: 304.8572468754131, max tps: 326.2375605095038, count: 115296"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 278.2940920543478,
            "unit": "median tps",
            "extra": "avg tps: 275.0103764700353, max tps: 286.13351962879847, count: 57648"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 12.097878601147324,
            "unit": "median tps",
            "extra": "avg tps: 19.152474652489452, max tps: 1816.398445162931, count: 57648"
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
        "date": 1752175503343,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 576.1503642734758,
            "unit": "median tps",
            "extra": "avg tps: 579.8783879117879, max tps: 812.1917764807696, count: 57667"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2333.3690337629396,
            "unit": "median tps",
            "extra": "avg tps: 2199.8174180817055, max tps: 2797.130619496602, count: 57667"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 575.9581831254046,
            "unit": "median tps",
            "extra": "avg tps: 578.6747090511985, max tps: 788.7883391205431, count: 57667"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 534.210044981809,
            "unit": "median tps",
            "extra": "avg tps: 534.8156130477985, max tps: 703.225781044746, count: 57667"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 321.0785617887077,
            "unit": "median tps",
            "extra": "avg tps: 315.83319861758787, max tps: 339.74711899296625, count: 115334"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 293.6763766964619,
            "unit": "median tps",
            "extra": "avg tps: 288.04069200037156, max tps: 306.8215569642649, count: 57667"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 13.401394721230936,
            "unit": "median tps",
            "extra": "avg tps: 22.28362647952062, max tps: 1984.2805296441588, count: 57667"
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
        "date": 1752175722641,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 533.7873180401348,
            "unit": "median tps",
            "extra": "avg tps: 537.2289845281066, max tps: 719.7214465448621, count: 57660"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2281.3540875407625,
            "unit": "median tps",
            "extra": "avg tps: 2150.9173151428913, max tps: 2772.0180003760875, count: 57660"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 531.1502967344068,
            "unit": "median tps",
            "extra": "avg tps: 532.940413297807, max tps: 779.8201998891456, count: 57660"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 489.46835408478876,
            "unit": "median tps",
            "extra": "avg tps: 489.20767804614604, max tps: 578.2194449185844, count: 57660"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 294.34294276499804,
            "unit": "median tps",
            "extra": "avg tps: 295.8823017870475, max tps: 314.3958928050774, count: 115320"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 269.8087523287835,
            "unit": "median tps",
            "extra": "avg tps: 266.36025676501663, max tps: 303.6938733291161, count: 57660"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 10.851269544666705,
            "unit": "median tps",
            "extra": "avg tps: 18.34825156844351, max tps: 1969.4343784465102, count: 57660"
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
        "date": 1752175835714,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 583.3120147373679,
            "unit": "median tps",
            "extra": "avg tps: 585.490145179396, max tps: 807.6627874448491, count: 57662"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2073.4074220139883,
            "unit": "median tps",
            "extra": "avg tps: 1948.2438913033786, max tps: 2582.1876653161453, count: 57662"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 582.9109393417011,
            "unit": "median tps",
            "extra": "avg tps: 584.9604659516402, max tps: 859.8126715189492, count: 57662"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 539.3245704049602,
            "unit": "median tps",
            "extra": "avg tps: 537.5444894021166, max tps: 691.6166449164758, count: 57662"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 313.53874856887785,
            "unit": "median tps",
            "extra": "avg tps: 308.0976777165562, max tps: 330.4798528604914, count: 115324"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 287.55610542706495,
            "unit": "median tps",
            "extra": "avg tps: 280.89568576353736, max tps: 294.6876392461318, count: 57662"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 18.34923363135176,
            "unit": "median tps",
            "extra": "avg tps: 25.02302779732774, max tps: 1813.6279632413884, count: 57662"
          }
        ]
      }
    ],
    "pg_search wide-table.toml Performance - TPS": [
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
        "date": 1752173269108,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 49.424779868026604,
            "unit": "median tps",
            "extra": "avg tps: 48.9314603378977, max tps: 51.06748962210755, count: 58910"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 230.86224278866604,
            "unit": "median tps",
            "extra": "avg tps: 227.12541748825345, max tps: 244.03866135319626, count: 58910"
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
        "date": 1752173936488,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 48.53954913535375,
            "unit": "median tps",
            "extra": "avg tps: 48.277656444840396, max tps: 49.523237035189396, count: 58892"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 241.12103179193275,
            "unit": "median tps",
            "extra": "avg tps: 234.2824725363889, max tps: 272.3349020084868, count: 58892"
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
        "date": 1752174322371,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 26.10772823780018,
            "unit": "median tps",
            "extra": "avg tps: 23.813595198294472, max tps: 38.900446153256595, count: 58882"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 131.245464908546,
            "unit": "median tps",
            "extra": "avg tps: 130.68366276426394, max tps: 134.58631816450114, count: 58882"
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
        "date": 1752174415627,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 43.455098413596275,
            "unit": "median tps",
            "extra": "avg tps: 42.89627018698154, max tps: 44.915962145017225, count: 58916"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 172.07501372126777,
            "unit": "median tps",
            "extra": "avg tps: 167.81594546945644, max tps: 193.94786687085113, count: 58916"
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
        "date": 1752174952039,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 26.711361388253856,
            "unit": "median tps",
            "extra": "avg tps: 24.34135820539003, max tps: 38.57098209408498, count: 58903"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 135.5300250685401,
            "unit": "median tps",
            "extra": "avg tps: 134.89224969061638, max tps: 138.06029442186147, count: 58903"
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
        "date": 1752175210833,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 26.15625080339816,
            "unit": "median tps",
            "extra": "avg tps: 23.918249389049386, max tps: 39.709261076907424, count: 58891"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 136.45143846704315,
            "unit": "median tps",
            "extra": "avg tps: 135.11403085147649, max tps: 137.4878289536281, count: 58891"
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
        "date": 1752175231175,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 42.096175948693,
            "unit": "median tps",
            "extra": "avg tps: 41.54897747358049, max tps: 42.84048966903384, count: 58907"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 133.27123985735776,
            "unit": "median tps",
            "extra": "avg tps: 131.63448802183376, max tps: 138.24960127137638, count: 58907"
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
        "date": 1752175526702,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 48.88029338552486,
            "unit": "median tps",
            "extra": "avg tps: 48.56503696039986, max tps: 50.05726481178207, count: 58893"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 230.68354051839202,
            "unit": "median tps",
            "extra": "avg tps: 227.62946558173275, max tps: 249.29362149740604, count: 58893"
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
        "date": 1752175766999,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 26.372910654013463,
            "unit": "median tps",
            "extra": "avg tps: 24.083746791468332, max tps: 39.798178115768614, count: 58901"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 134.40240743381707,
            "unit": "median tps",
            "extra": "avg tps: 133.4888462252407, max tps: 139.3769256818613, count: 58901"
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
        "date": 1752175917991,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 48.09448447336764,
            "unit": "median tps",
            "extra": "avg tps: 47.78744873690517, max tps: 49.37876352711199, count: 58888"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 227.24468262408223,
            "unit": "median tps",
            "extra": "avg tps: 224.1738335133765, max tps: 251.21276794273652, count: 58888"
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
            "name": "Philippe Noël",
            "username": "philippemnoel",
            "email": "philippemnoel@gmail.com"
          },
          "id": "ce8e33ae49785f0afe220ca985de3d0c7c270503",
          "message": "chore: more improvements to index/schema configuration and management (#2771)\n\n## What\n\n#2660 brought a much needed round of cleanups to how we manage index\nschemas. Unfortunately, it introduced quite some overhead in\nreading/decoding/validating the schema. This process was happening quite\na bit throughout the execution paths of `aminsert` and other hot-spots.\n\n#2176 brought the ability to essentially keep one heavy-weight\n`PgSearchRelation` instantiated and cheaply clone it when necessary.\nThis PR cleans up things further such that the `SearchIndexSchema` is\nnow a lazily-evaluated property of `PgSearchRelation`. This means\n`SearchIndexSchema` is only evaluated when needed, and then only once\n(at least per statement).\n\nFurthermore, its internal properties are lazily-evaluated, ensuring any\ngiven code path doesn't do more work than it needs.\n\nThis also renames `SearchIndexOptions` to `BM25IndexOptions`, mainly\nbecause I kept getting confused about what `SearchIndexOptions`\nrepresented (it was too similarly named to `SearchIndexSchema` for my\ntastes). And `BM25IndexOptions` is now a property of `PgSearchRelation`\ntoo.\n\nThis seems to have drastically improved the write throughput of the\nINSERT/UPDATE jobs in our `single-server.toml` stressgress test.\nv0.15.26 was 176/s INSERTs and 154/s UPDATEs. This PR clocks in at 275/s\nand 260/s, respectively.\n\n# Other Notable Changes\n\n- Index configuration validation now happens during CREATE INDEX/REINDEX\nin `ambuildempty()` rather than on every instantiation of\n`SearchIndexSchema`.\n\n- The \"raw\" tokenizer deprecation warnings are now gone, unless somehow\nthe \"key_field\" is configured with it -- which is no longer possible\n\n## Why\n\nTrying to rollback performance regressions that were introduced in\n0.16.0\n\n## How\n\n## Tests\n\nAll existing tests pass, and a few were updated due to the \"raw\"\ntokenizer deprecation warning going away and a change in wording for a\nspecific validation error.",
          "timestamp": "2025-07-05T15:13:47Z",
          "url": "https://github.com/paradedb/paradedb/commit/ce8e33ae49785f0afe220ca985de3d0c7c270503"
        },
        "date": 1752173270677,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 15.604681,
            "unit": "median cpu",
            "extra": "avg cpu: 18.61035121135894, max cpu: 53.745926, count: 58866"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 227.828125,
            "unit": "median mem",
            "extra": "avg mem: 227.05741077988569, max mem: 231.75390625, count: 58866"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 24,
            "unit": "median cpu",
            "extra": "avg cpu: 20.43058360726388, max cpu: 34.146343, count: 58866"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 159.671875,
            "unit": "median mem",
            "extra": "avg mem: 160.29548903972582, max mem: 163.4140625, count: 58866"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 24431,
            "unit": "median block_count",
            "extra": "avg block_count: 22282.98296130194, max block_count: 24431.0, count: 58866"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 55,
            "unit": "median segment_count",
            "extra": "avg segment_count: 57.8033839567832, max segment_count: 94.0, count: 58866"
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
        "date": 1752173874143,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 24.038462,
            "unit": "median cpu",
            "extra": "avg cpu: 22.05722395695438, max cpu: 44.189854, count: 15480"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 225.6484375,
            "unit": "median mem",
            "extra": "avg mem: 226.09547576510013, max mem: 231.234375, count: 15480"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 24.038462,
            "unit": "median cpu",
            "extra": "avg cpu: 21.819227877871562, max cpu: 29.268291, count: 15480"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 159.78515625,
            "unit": "median mem",
            "extra": "avg mem: 159.65440841004522, max mem: 160.50390625, count: 15480"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 16774,
            "unit": "median block_count",
            "extra": "avg block_count: 17157.08785529716, max block_count: 21043.0, count: 15480"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 46,
            "unit": "median segment_count",
            "extra": "avg segment_count: 47.04799741602067, max segment_count: 74.0, count: 15480"
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
        "date": 1752174435247,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 24.115755,
            "unit": "median cpu",
            "extra": "avg cpu: 22.892735438803005, max cpu: 43.90244, count: 58833"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 174.27734375,
            "unit": "median mem",
            "extra": "avg mem: 173.82767809148353, max mem: 182.38671875, count: 58833"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 19.448946,
            "unit": "median cpu",
            "extra": "avg cpu: 19.147848811631164, max cpu: 29.411766, count: 58833"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 159.8359375,
            "unit": "median mem",
            "extra": "avg mem: 159.88007262080805, max mem: 161.703125, count: 58833"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 20545,
            "unit": "median block_count",
            "extra": "avg block_count: 20061.48843336223, max block_count: 20545.0, count: 58833"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 49,
            "unit": "median segment_count",
            "extra": "avg segment_count: 48.95055496065134, max segment_count: 70.0, count: 58833"
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
        "date": 1752174535403,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 19.512194,
            "unit": "median cpu",
            "extra": "avg cpu: 22.923667208236765, max cpu: 68.29269, count: 58863"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 295.05078125,
            "unit": "median mem",
            "extra": "avg mem: 293.7316002837096, max mem: 325.109375, count: 58863"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 24.038462,
            "unit": "median cpu",
            "extra": "avg cpu: 20.88758986671816, max cpu: 33.816425, count: 58863"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 172.9296875,
            "unit": "median mem",
            "extra": "avg mem: 172.034900292841, max mem: 178.52734375, count: 58863"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 26080,
            "unit": "median block_count",
            "extra": "avg block_count: 23708.88903046056, max block_count: 26080.0, count: 58863"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 56,
            "unit": "median segment_count",
            "extra": "avg segment_count: 58.63853354399198, max segment_count: 95.0, count: 58863"
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
        "date": 1752174823786,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 24.115755,
            "unit": "median cpu",
            "extra": "avg cpu: 22.891373342354086, max cpu: 43.97394, count: 58855"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 175.49609375,
            "unit": "median mem",
            "extra": "avg mem: 175.07746335007647, max mem: 181.4375, count: 58855"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 19.448946,
            "unit": "median cpu",
            "extra": "avg cpu: 18.96352119160932, max cpu: 29.363787, count: 58855"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 160.62109375,
            "unit": "median mem",
            "extra": "avg mem: 160.5575476807408, max mem: 162.0234375, count: 58855"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 20679,
            "unit": "median block_count",
            "extra": "avg block_count: 20190.81986237363, max block_count: 20679.0, count: 58855"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 49,
            "unit": "median segment_count",
            "extra": "avg segment_count: 48.98481012658228, max segment_count: 70.0, count: 58855"
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
        "date": 1752174980543,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 24.115755,
            "unit": "median cpu",
            "extra": "avg cpu: 22.9021632477531, max cpu: 43.97394, count: 58838"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 173.8359375,
            "unit": "median mem",
            "extra": "avg mem: 173.2275418176816, max mem: 181.25, count: 58838"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 19.455252,
            "unit": "median cpu",
            "extra": "avg cpu: 19.05668285603413, max cpu: 33.76206, count: 58838"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 160.57421875,
            "unit": "median mem",
            "extra": "avg mem: 160.41249164814832, max mem: 161.296875, count: 58838"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 20558,
            "unit": "median block_count",
            "extra": "avg block_count: 20059.894218022368, max block_count: 20558.0, count: 58838"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 49,
            "unit": "median segment_count",
            "extra": "avg segment_count: 48.79327985315612, max segment_count: 70.0, count: 58838"
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
        "date": 1752175336379,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 15.523932,
            "unit": "median cpu",
            "extra": "avg cpu: 18.45605325030095, max cpu: 49.099834, count: 58845"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 228.015625,
            "unit": "median mem",
            "extra": "avg mem: 226.49968189735748, max mem: 238.61328125, count: 58845"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.961662,
            "unit": "median cpu",
            "extra": "avg cpu: 20.439430991715746, max cpu: 33.92569, count: 58845"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 159.70703125,
            "unit": "median mem",
            "extra": "avg mem: 159.68510553413628, max mem: 161.5, count: 58845"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 24195,
            "unit": "median block_count",
            "extra": "avg block_count: 22151.140878579317, max block_count: 24355.0, count: 58845"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 55,
            "unit": "median segment_count",
            "extra": "avg segment_count: 57.57733027444983, max segment_count: 94.0, count: 58845"
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
        "date": 1752175487535,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 15.503876,
            "unit": "median cpu",
            "extra": "avg cpu: 18.474403984241306, max cpu: 49.261086, count: 58851"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 227.5078125,
            "unit": "median mem",
            "extra": "avg mem: 226.6958322182291, max mem: 234.203125, count: 58851"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 24,
            "unit": "median cpu",
            "extra": "avg cpu: 20.521187822327164, max cpu: 34.090908, count: 58851"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 160.2890625,
            "unit": "median mem",
            "extra": "avg mem: 160.24273131244584, max mem: 161.54296875, count: 58851"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 25280,
            "unit": "median block_count",
            "extra": "avg block_count: 23238.078197481776, max block_count: 27045.0, count: 58851"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 55,
            "unit": "median segment_count",
            "extra": "avg segment_count: 57.72600295661926, max segment_count: 93.0, count: 58851"
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
        "date": 1752175783310,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 24.115755,
            "unit": "median cpu",
            "extra": "avg cpu: 22.952105949273886, max cpu: 44.189854, count: 58859"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 173.21875,
            "unit": "median mem",
            "extra": "avg mem: 172.62158087813674, max mem: 179.9453125, count: 58859"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 19.48052,
            "unit": "median cpu",
            "extra": "avg cpu: 19.20470292238869, max cpu: 33.92569, count: 58859"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 159.54296875,
            "unit": "median mem",
            "extra": "avg mem: 159.54516500768787, max mem: 160.7265625, count: 58859"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 20705,
            "unit": "median block_count",
            "extra": "avg block_count: 20220.116430792234, max block_count: 20705.0, count: 58859"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 49,
            "unit": "median segment_count",
            "extra": "avg segment_count: 48.974617305764625, max segment_count: 70.0, count: 58859"
          }
        ]
      }
    ],
    "pg_search single-server.toml Performance - Other Metrics": [
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
        "date": 1752173316906,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.83871,
            "unit": "median cpu",
            "extra": "avg cpu: 5.644031635079202, max cpu: 19.23077, count: 57674"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 98.3515625,
            "unit": "median mem",
            "extra": "avg mem: 97.97659128517616, max mem: 104.125, count: 57674"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.83871,
            "unit": "median cpu",
            "extra": "avg cpu: 4.824110258603734, max cpu: 9.74026, count: 57674"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 78.5703125,
            "unit": "median mem",
            "extra": "avg mem: 77.29807882993204, max mem: 86.2734375, count: 57674"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.83871,
            "unit": "median cpu",
            "extra": "avg cpu: 5.619310313109151, max cpu: 19.35484, count: 57674"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 99.73046875,
            "unit": "median mem",
            "extra": "avg mem: 98.75077428737386, max mem: 104.53125, count: 57674"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.830918,
            "unit": "median cpu",
            "extra": "avg cpu: 4.899149679223541, max cpu: 9.693053, count: 57674"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 93.2578125,
            "unit": "median mem",
            "extra": "avg mem: 92.04995352379322, max mem: 97.3671875, count: 57674"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.8543687,
            "unit": "median cpu",
            "extra": "avg cpu: 6.470277318476125, max cpu: 19.292604, count: 115348"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 101.16015625,
            "unit": "median mem",
            "extra": "avg mem: 99.94637921089875, max mem: 107.76953125, count: 115348"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 7792,
            "unit": "median block_count",
            "extra": "avg block_count: 7780.993792696882, max block_count: 7792.0, count: 57674"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 117,
            "unit": "median segment_count",
            "extra": "avg segment_count: 117.03237160592295, max segment_count: 329.0, count: 57674"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.83871,
            "unit": "median cpu",
            "extra": "avg cpu: 5.424596265628701, max cpu: 14.51613, count: 57674"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 104.68359375,
            "unit": "median mem",
            "extra": "avg mem: 102.36149338577609, max mem: 110.14453125, count: 57674"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 14.563106,
            "unit": "median cpu",
            "extra": "avg cpu: 15.301401998160715, max cpu: 29.31596, count: 57674"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 91.421875,
            "unit": "median mem",
            "extra": "avg mem: 89.8251066202925, max mem: 92.2265625, count: 57674"
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
        "date": 1752174278912,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.83871,
            "unit": "median cpu",
            "extra": "avg cpu: 5.6483716176381735, max cpu: 24.038462, count: 57662"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 99.1796875,
            "unit": "median mem",
            "extra": "avg mem: 97.334633971463, max mem: 102.08203125, count: 57662"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.830918,
            "unit": "median cpu",
            "extra": "avg cpu: 4.724294219043499, max cpu: 4.950495, count: 57662"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 86.05078125,
            "unit": "median mem",
            "extra": "avg mem: 83.09500421096216, max mem: 87.8203125, count: 57662"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.83871,
            "unit": "median cpu",
            "extra": "avg cpu: 5.606351869183033, max cpu: 19.512194, count: 57662"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 100.7421875,
            "unit": "median mem",
            "extra": "avg mem: 98.23575135325258, max mem: 103.1484375, count: 57662"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.83871,
            "unit": "median cpu",
            "extra": "avg cpu: 4.7688306645840814, max cpu: 4.8780484, count: 57662"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 97.00390625,
            "unit": "median mem",
            "extra": "avg mem: 94.69748972189224, max mem: 99.1796875, count: 57662"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.8543687,
            "unit": "median cpu",
            "extra": "avg cpu: 6.633848605577827, max cpu: 24.271845, count: 115324"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 101.796875,
            "unit": "median mem",
            "extra": "avg mem: 101.23239389676043, max mem: 109.84375, count: 115324"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 7732,
            "unit": "median block_count",
            "extra": "avg block_count: 7489.2179771773435, max block_count: 7732.0, count: 57662"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 117,
            "unit": "median segment_count",
            "extra": "avg segment_count: 116.78269917796815, max segment_count: 314.0, count: 57662"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.83871,
            "unit": "median cpu",
            "extra": "avg cpu: 5.434581858417324, max cpu: 14.729951, count: 57662"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 104.23828125,
            "unit": "median mem",
            "extra": "avg mem: 104.28294292059762, max mem: 113.66015625, count: 57662"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 14.53958,
            "unit": "median cpu",
            "extra": "avg cpu: 15.74504855363021, max cpu: 29.126211, count: 57662"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 88.98046875,
            "unit": "median mem",
            "extra": "avg mem: 85.85231655758992, max mem: 92.80078125, count: 57662"
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
        "date": 1752174376815,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.83871,
            "unit": "median cpu",
            "extra": "avg cpu: 5.864793264498222, max cpu: 19.448946, count: 57650"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 102.27734375,
            "unit": "median mem",
            "extra": "avg mem: 109.55347449588031, max mem: 145.25390625, count: 57650"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.830918,
            "unit": "median cpu",
            "extra": "avg cpu: 4.825951452816704, max cpu: 9.787929, count: 57650"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 89.57421875,
            "unit": "median mem",
            "extra": "avg mem: 89.02644026452732, max mem: 104.6875, count: 57650"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.83871,
            "unit": "median cpu",
            "extra": "avg cpu: 5.86565384147814, max cpu: 19.417475, count: 57650"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 105.6484375,
            "unit": "median mem",
            "extra": "avg mem: 112.03936341337814, max mem: 146.1875, count: 57650"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.830918,
            "unit": "median cpu",
            "extra": "avg cpu: 4.7409448655115405, max cpu: 4.9261084, count: 57650"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 99.91796875,
            "unit": "median mem",
            "extra": "avg mem: 105.4453061985039, max mem: 137.07421875, count: 57650"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.8465266,
            "unit": "median cpu",
            "extra": "avg cpu: 6.393402860396292, max cpu: 19.35484, count: 115300"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 115.7265625,
            "unit": "median mem",
            "extra": "avg mem: 121.23643311605593, max mem: 157.98828125, count: 115300"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 7769,
            "unit": "median block_count",
            "extra": "avg block_count: 8784.834605377277, max block_count: 13091.0, count: 57650"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 118,
            "unit": "median segment_count",
            "extra": "avg segment_count: 117.98039895923678, max segment_count: 486.0, count: 57650"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.830918,
            "unit": "median cpu",
            "extra": "avg cpu: 5.475421441157109, max cpu: 14.53958, count: 57650"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 116.828125,
            "unit": "median mem",
            "extra": "avg mem: 121.38897360147442, max mem: 157.375, count: 57650"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 9.771987,
            "unit": "median cpu",
            "extra": "avg cpu: 13.429105695780297, max cpu: 28.938908, count: 57650"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 94.81640625,
            "unit": "median mem",
            "extra": "avg mem: 98.80846528078925, max mem: 134.9609375, count: 57650"
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
        "date": 1752174552155,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.8543687,
            "unit": "median cpu",
            "extra": "avg cpu: 6.995542295816955, max cpu: 24.469822, count: 57576"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 113.2578125,
            "unit": "median mem",
            "extra": "avg mem: 111.92209956731104, max mem: 115.0859375, count: 57576"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.823151,
            "unit": "median cpu",
            "extra": "avg cpu: 4.846639095347339, max cpu: 9.756097, count: 57576"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 94.31640625,
            "unit": "median mem",
            "extra": "avg mem: 92.74346068131686, max mem: 94.55078125, count: 57576"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.8543687,
            "unit": "median cpu",
            "extra": "avg cpu: 6.967139393409854, max cpu: 19.639935, count: 57576"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 114.75,
            "unit": "median mem",
            "extra": "avg mem: 113.20225129068535, max mem: 115.85546875, count: 57576"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.823151,
            "unit": "median cpu",
            "extra": "avg cpu: 4.763188767516035, max cpu: 4.9180326, count: 57576"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 108.17578125,
            "unit": "median mem",
            "extra": "avg mem: 107.28942160416233, max mem: 108.65234375, count: 57576"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 14.469454,
            "unit": "median cpu",
            "extra": "avg cpu: 14.465811631611235, max cpu: 39.34426, count: 115152"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 157.76171875,
            "unit": "median mem",
            "extra": "avg mem: 157.08889619128414, max mem: 179.265625, count: 115152"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 8908,
            "unit": "median block_count",
            "extra": "avg block_count: 8880.768844657496, max block_count: 8908.0, count: 57576"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 116,
            "unit": "median segment_count",
            "extra": "avg segment_count: 115.63597679588717, max segment_count: 249.0, count: 57576"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.615385,
            "unit": "median cpu",
            "extra": "avg cpu: 8.362954380755482, max cpu: 24.35065, count: 57576"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 153.1328125,
            "unit": "median mem",
            "extra": "avg mem: 154.3415906258684, max mem: 183.484375, count: 57576"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 7.7319584,
            "unit": "median cpu",
            "extra": "avg cpu: 8.26372330279195, max cpu: 24.0, count: 57576"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 94.5234375,
            "unit": "median mem",
            "extra": "avg mem: 92.02309155778883, max mem: 100.3515625, count: 57576"
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
        "date": 1752174580032,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.8465266,
            "unit": "median cpu",
            "extra": "avg cpu: 5.878166103318246, max cpu: 19.575857, count: 57671"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 130.37890625,
            "unit": "median mem",
            "extra": "avg mem: 118.42380045592672, max mem: 130.84765625, count: 57671"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.83871,
            "unit": "median cpu",
            "extra": "avg cpu: 4.741566148298451, max cpu: 9.630818, count: 57671"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 98.125,
            "unit": "median mem",
            "extra": "avg mem: 92.43806272866779, max mem: 98.359375, count: 57671"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.8465266,
            "unit": "median cpu",
            "extra": "avg cpu: 5.906800200169273, max cpu: 19.543974, count: 57671"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 133.140625,
            "unit": "median mem",
            "extra": "avg mem: 120.59046459388168, max mem: 133.140625, count: 57671"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.823151,
            "unit": "median cpu",
            "extra": "avg cpu: 4.699061322923375, max cpu: 4.9099836, count: 57671"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 123.41796875,
            "unit": "median mem",
            "extra": "avg mem: 113.09549603028385, max mem: 123.65234375, count: 57671"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.8543687,
            "unit": "median cpu",
            "extra": "avg cpu: 6.48229666759274, max cpu: 19.607843, count: 115342"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 132.34375,
            "unit": "median mem",
            "extra": "avg mem: 129.41761396271522, max mem: 152.859375, count: 115342"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 12179,
            "unit": "median block_count",
            "extra": "avg block_count: 10413.056319467323, max block_count: 12179.0, count: 57671"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 118,
            "unit": "median segment_count",
            "extra": "avg segment_count: 117.38632935097363, max segment_count: 386.0, count: 57671"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.83871,
            "unit": "median cpu",
            "extra": "avg cpu: 5.376593304868893, max cpu: 14.827018, count: 57671"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 127.6875,
            "unit": "median mem",
            "extra": "avg mem: 123.81690259998526, max mem: 135.30078125, count: 57671"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 9.693053,
            "unit": "median cpu",
            "extra": "avg cpu: 12.00667823257283, max cpu: 29.411766, count: 57671"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 93.66015625,
            "unit": "median mem",
            "extra": "avg mem: 87.79450399410015, max mem: 95.375, count: 57671"
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
        "date": 1752175043036,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.83871,
            "unit": "median cpu",
            "extra": "avg cpu: 5.804807118169401, max cpu: 19.417475, count: 57677"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 110.79296875,
            "unit": "median mem",
            "extra": "avg mem: 107.21616346474765, max mem: 113.2421875, count: 57677"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.830918,
            "unit": "median cpu",
            "extra": "avg cpu: 4.811190693637466, max cpu: 9.693053, count: 57677"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 92.69921875,
            "unit": "median mem",
            "extra": "avg mem: 88.41575992434159, max mem: 94.73046875, count: 57677"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.83871,
            "unit": "median cpu",
            "extra": "avg cpu: 5.8536789654796175, max cpu: 19.575857, count: 57677"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 109.3515625,
            "unit": "median mem",
            "extra": "avg mem: 106.68602553064913, max mem: 113.765625, count: 57677"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.830918,
            "unit": "median cpu",
            "extra": "avg cpu: 4.730291773373129, max cpu: 4.901961, count: 57677"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 102.875,
            "unit": "median mem",
            "extra": "avg mem: 100.22091507446642, max mem: 106.27734375, count: 57677"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.8465266,
            "unit": "median cpu",
            "extra": "avg cpu: 6.450145506717026, max cpu: 24.232634, count: 115354"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 120.19140625,
            "unit": "median mem",
            "extra": "avg mem: 118.12973051297527, max mem: 129.75390625, count: 115354"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 9111,
            "unit": "median block_count",
            "extra": "avg block_count: 8594.990550826153, max block_count: 9111.0, count: 57677"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 118,
            "unit": "median segment_count",
            "extra": "avg segment_count: 117.49309083343447, max segment_count: 321.0, count: 57677"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.83871,
            "unit": "median cpu",
            "extra": "avg cpu: 5.4736744436756615, max cpu: 14.53958, count: 57677"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 119.921875,
            "unit": "median mem",
            "extra": "avg mem: 117.52775200415677, max mem: 127.73828125, count: 57677"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 14.446229,
            "unit": "median cpu",
            "extra": "avg cpu: 14.746599965666944, max cpu: 29.31596, count: 57677"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 102.31640625,
            "unit": "median mem",
            "extra": "avg mem: 97.46919719678121, max mem: 104.89453125, count: 57677"
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
        "date": 1752175423806,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.83871,
            "unit": "median cpu",
            "extra": "avg cpu: 5.67979000845745, max cpu: 24.671053, count: 57648"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 102.97265625,
            "unit": "median mem",
            "extra": "avg mem: 102.30598490786758, max mem: 104.7421875, count: 57648"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.830918,
            "unit": "median cpu",
            "extra": "avg cpu: 4.7885296194282665, max cpu: 7.772021, count: 57648"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 92.19140625,
            "unit": "median mem",
            "extra": "avg mem: 89.34941381856265, max mem: 92.46484375, count: 57648"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.83871,
            "unit": "median cpu",
            "extra": "avg cpu: 5.622329371895744, max cpu: 19.834711, count: 57648"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 102.8671875,
            "unit": "median mem",
            "extra": "avg mem: 102.34408832754389, max mem: 105.36328125, count: 57648"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.823151,
            "unit": "median cpu",
            "extra": "avg cpu: 4.707474368781307, max cpu: 4.8859935, count: 57648"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 100.11328125,
            "unit": "median mem",
            "extra": "avg mem: 98.90781499358174, max mem: 101.23828125, count: 57648"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.8465266,
            "unit": "median cpu",
            "extra": "avg cpu: 6.528230432576246, max cpu: 19.966722, count: 115296"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 108.7734375,
            "unit": "median mem",
            "extra": "avg mem: 109.58766925186043, max mem: 115.515625, count: 115296"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 8119,
            "unit": "median block_count",
            "extra": "avg block_count: 8109.127133638634, max block_count: 8119.0, count: 57648"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 117,
            "unit": "median segment_count",
            "extra": "avg segment_count: 117.09280460727172, max segment_count: 304.0, count: 57648"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.830918,
            "unit": "median cpu",
            "extra": "avg cpu: 5.524034607161205, max cpu: 14.876034, count: 57648"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 111.11328125,
            "unit": "median mem",
            "extra": "avg mem: 110.45827533045379, max mem: 116.72265625, count: 57648"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 9.693053,
            "unit": "median cpu",
            "extra": "avg cpu: 13.286711144668388, max cpu: 29.220781, count: 57648"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 91.9375,
            "unit": "median mem",
            "extra": "avg mem: 90.60810090170084, max mem: 94.125, count: 57648"
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
        "date": 1752175512997,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.83871,
            "unit": "median cpu",
            "extra": "avg cpu: 5.638349624906138, max cpu: 19.86755, count: 57667"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 108.77734375,
            "unit": "median mem",
            "extra": "avg mem: 105.10776303767753, max mem: 111.94140625, count: 57667"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.830918,
            "unit": "median cpu",
            "extra": "avg cpu: 4.814108737549443, max cpu: 9.67742, count: 57667"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 97.875,
            "unit": "median mem",
            "extra": "avg mem: 92.85913480088266, max mem: 98.12890625, count: 57667"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.83871,
            "unit": "median cpu",
            "extra": "avg cpu: 5.613226746797614, max cpu: 19.582247, count: 57667"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 109.36328125,
            "unit": "median mem",
            "extra": "avg mem: 105.51984345466211, max mem: 112.8046875, count: 57667"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.830918,
            "unit": "median cpu",
            "extra": "avg cpu: 4.671812828041278, max cpu: 4.901961, count: 57667"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 104.00390625,
            "unit": "median mem",
            "extra": "avg mem: 100.1985268735802, max mem: 105.7109375, count: 57667"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.8543687,
            "unit": "median cpu",
            "extra": "avg cpu: 6.557024109158583, max cpu: 24.469822, count: 115334"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 109.86328125,
            "unit": "median mem",
            "extra": "avg mem: 106.89911535464391, max mem: 115.6796875, count: 115334"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 8904,
            "unit": "median block_count",
            "extra": "avg block_count: 8411.079352142473, max block_count: 8904.0, count: 57667"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 117,
            "unit": "median segment_count",
            "extra": "avg segment_count: 117.28733937954115, max segment_count: 377.0, count: 57667"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.83871,
            "unit": "median cpu",
            "extra": "avg cpu: 5.399129302879134, max cpu: 14.681893, count: 57667"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 112.68359375,
            "unit": "median mem",
            "extra": "avg mem: 108.70059369689336, max mem: 118.11328125, count: 57667"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 9.74026,
            "unit": "median cpu",
            "extra": "avg cpu: 13.204853741076311, max cpu: 29.31596, count: 57667"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 96.22265625,
            "unit": "median mem",
            "extra": "avg mem: 88.93093523483968, max mem: 99.10546875, count: 57667"
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
        "date": 1752175747489,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.83871,
            "unit": "median cpu",
            "extra": "avg cpu: 5.842619772269407, max cpu: 24.390244, count: 57660"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 104.69921875,
            "unit": "median mem",
            "extra": "avg mem: 104.72904343619928, max mem: 109.9453125, count: 57660"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.83871,
            "unit": "median cpu",
            "extra": "avg cpu: 4.890235917980431, max cpu: 9.852217, count: 57660"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 92.328125,
            "unit": "median mem",
            "extra": "avg mem: 89.7703431213146, max mem: 92.5859375, count: 57660"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.83871,
            "unit": "median cpu",
            "extra": "avg cpu: 5.900393317500811, max cpu: 19.704433, count: 57660"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 105.93359375,
            "unit": "median mem",
            "extra": "avg mem: 105.49807343153833, max mem: 108.46875, count: 57660"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.815409,
            "unit": "median cpu",
            "extra": "avg cpu: 4.505890247158963, max cpu: 4.8780484, count: 57660"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 102.625,
            "unit": "median mem",
            "extra": "avg mem: 101.81337683998872, max mem: 105.0, count: 57660"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.8465266,
            "unit": "median cpu",
            "extra": "avg cpu: 6.3773689242401135, max cpu: 19.448946, count: 115320"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 116.140625,
            "unit": "median mem",
            "extra": "avg mem: 115.67322054256634, max mem: 122.734375, count: 115320"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 8093,
            "unit": "median block_count",
            "extra": "avg block_count: 8067.022996878251, max block_count: 8148.0, count: 57660"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 117,
            "unit": "median segment_count",
            "extra": "avg segment_count: 116.97377731529657, max segment_count: 252.0, count: 57660"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.83871,
            "unit": "median cpu",
            "extra": "avg cpu: 5.490556695083487, max cpu: 14.563106, count: 57660"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 118.6875,
            "unit": "median mem",
            "extra": "avg mem: 118.59985725860649, max mem: 125.11328125, count: 57660"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 14.4,
            "unit": "median cpu",
            "extra": "avg cpu: 13.547556877615254, max cpu: 29.126211, count: 57660"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 95.27734375,
            "unit": "median mem",
            "extra": "avg mem: 92.47688971449011, max mem: 96.8203125, count: 57660"
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
        "date": 1752175854659,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.83871,
            "unit": "median cpu",
            "extra": "avg cpu: 5.631615944834793, max cpu: 19.639935, count: 57662"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 96.65625,
            "unit": "median mem",
            "extra": "avg mem: 98.02616270247303, max mem: 123.0, count: 57662"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.83871,
            "unit": "median cpu",
            "extra": "avg cpu: 4.834567597434873, max cpu: 9.771987, count: 57662"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 84.08203125,
            "unit": "median mem",
            "extra": "avg mem: 81.93426902140925, max mem: 93.4453125, count: 57662"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.83871,
            "unit": "median cpu",
            "extra": "avg cpu: 5.613439226799961, max cpu: 19.386106, count: 57662"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 97.484375,
            "unit": "median mem",
            "extra": "avg mem: 98.68628548697582, max mem: 123.703125, count: 57662"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.83871,
            "unit": "median cpu",
            "extra": "avg cpu: 4.770202613991446, max cpu: 4.9099836, count: 57662"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 93.55859375,
            "unit": "median mem",
            "extra": "avg mem: 92.80447790030696, max mem: 115.32421875, count: 57662"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.8543687,
            "unit": "median cpu",
            "extra": "avg cpu: 6.565714534651066, max cpu: 19.769358, count: 115324"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 102.07421875,
            "unit": "median mem",
            "extra": "avg mem: 102.62816892515869, max mem: 130.30078125, count: 115324"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 7360,
            "unit": "median block_count",
            "extra": "avg block_count: 7579.500901807082, max block_count: 10696.0, count: 57662"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 118,
            "unit": "median segment_count",
            "extra": "avg segment_count: 117.28516180500156, max segment_count: 388.0, count: 57662"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.83871,
            "unit": "median cpu",
            "extra": "avg cpu: 5.309790498410811, max cpu: 14.51613, count: 57662"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 105.421875,
            "unit": "median mem",
            "extra": "avg mem: 105.91540979111026, max mem: 132.9921875, count: 57662"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 15.483871,
            "unit": "median cpu",
            "extra": "avg cpu: 15.592293241100839, max cpu: 29.07916, count: 57662"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 85.76953125,
            "unit": "median mem",
            "extra": "avg mem: 82.38705150814228, max mem: 113.5078125, count: 57662"
          }
        ]
      }
    ],
    "pg_search wide-table.toml Performance - Other Metrics": [
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
        "date": 1752173329622,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 19.261637,
            "unit": "median cpu",
            "extra": "avg cpu: 18.26996530147593, max cpu: 58.06452, count: 58910"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 188.90234375,
            "unit": "median mem",
            "extra": "avg mem: 186.8327326642336, max mem: 244.19140625, count: 58910"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 36863,
            "unit": "median block_count",
            "extra": "avg block_count: 32095.72507214395, max block_count: 36863.0, count: 58910"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 77,
            "unit": "median segment_count",
            "extra": "avg segment_count: 78.83104736038024, max segment_count: 171.0, count: 58910"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 9.615385,
            "unit": "median cpu",
            "extra": "avg cpu: 8.530401903650937, max cpu: 34.76821, count: 58910"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 164.69921875,
            "unit": "median mem",
            "extra": "avg mem: 156.75720126570192, max mem: 173.02734375, count: 58910"
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
        "date": 1752173975035,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 19.23077,
            "unit": "median cpu",
            "extra": "avg cpu: 18.190230751651747, max cpu: 69.65174, count: 58892"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 180.5703125,
            "unit": "median mem",
            "extra": "avg mem: 179.9089808330291, max mem: 180.5703125, count: 58892"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 37326,
            "unit": "median block_count",
            "extra": "avg block_count: 32446.33011274876, max block_count: 37326.0, count: 58892"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 75,
            "unit": "median segment_count",
            "extra": "avg segment_count: 77.25322624465123, max segment_count: 174.0, count: 58892"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.8622365,
            "unit": "median cpu",
            "extra": "avg cpu: 7.456355695296276, max cpu: 34.596375, count: 58892"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 214.96875,
            "unit": "median mem",
            "extra": "avg mem: 197.45201319258135, max mem: 243.046875, count: 58892"
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
        "date": 1752174367340,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 19.323671,
            "unit": "median cpu",
            "extra": "avg cpu: 20.15879472173572, max cpu: 49.099834, count: 58882"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 175.32421875,
            "unit": "median mem",
            "extra": "avg mem: 173.32991061572298, max mem: 183.296875, count: 58882"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 10627,
            "unit": "median block_count",
            "extra": "avg block_count: 9478.259400156245, max block_count: 11325.0, count: 58882"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 67,
            "unit": "median segment_count",
            "extra": "avg segment_count: 67.24943106552087, max segment_count: 123.0, count: 58882"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 9.724473,
            "unit": "median cpu",
            "extra": "avg cpu: 12.06358166069403, max cpu: 34.090908, count: 58882"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 168.234375,
            "unit": "median mem",
            "extra": "avg mem: 162.68794481176334, max mem: 177.52734375, count: 58882"
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
        "date": 1752174481175,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 24.232634,
            "unit": "median cpu",
            "extra": "avg cpu: 24.923479458578562, max cpu: 99.66777, count: 58916"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 262.6015625,
            "unit": "median mem",
            "extra": "avg mem: 260.172845794224, max mem: 354.7109375, count: 58916"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 36858,
            "unit": "median block_count",
            "extra": "avg block_count: 31595.146921040126, max block_count: 36858.0, count: 58916"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 76,
            "unit": "median segment_count",
            "extra": "avg segment_count: 78.8623463914726, max segment_count: 172.0, count: 58916"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 14.586709,
            "unit": "median cpu",
            "extra": "avg cpu: 16.62558492833049, max cpu: 67.54967, count: 58916"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 225.9375,
            "unit": "median mem",
            "extra": "avg mem: 214.91310781716766, max mem: 251.0546875, count: 58916"
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
        "date": 1752174981300,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 19.323671,
            "unit": "median cpu",
            "extra": "avg cpu: 19.986740980919443, max cpu: 48.859932, count: 58903"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 177.44921875,
            "unit": "median mem",
            "extra": "avg mem: 175.689142862524, max mem: 180.44140625, count: 58903"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 10816,
            "unit": "median block_count",
            "extra": "avg block_count: 9328.010152284263, max block_count: 10905.0, count: 58903"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 67,
            "unit": "median segment_count",
            "extra": "avg segment_count: 67.58046279476427, max segment_count: 112.0, count: 58903"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 9.708737,
            "unit": "median cpu",
            "extra": "avg cpu: 11.783414488623434, max cpu: 34.31373, count: 58903"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 170.87109375,
            "unit": "median mem",
            "extra": "avg mem: 164.6047026599876, max mem: 177.48828125, count: 58903"
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
        "date": 1752175243336,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 19.323671,
            "unit": "median cpu",
            "extra": "avg cpu: 20.156603528273845, max cpu: 48.939644, count: 58891"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 175.578125,
            "unit": "median mem",
            "extra": "avg mem: 173.0467942098538, max mem: 183.00390625, count: 58891"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 11135,
            "unit": "median block_count",
            "extra": "avg block_count: 9532.885058837513, max block_count: 11341.0, count: 58891"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 67,
            "unit": "median segment_count",
            "extra": "avg segment_count: 67.46336452089453, max segment_count: 123.0, count: 58891"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 9.708737,
            "unit": "median cpu",
            "extra": "avg cpu: 11.825118376041354, max cpu: 38.27751, count: 58891"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 168.38671875,
            "unit": "median mem",
            "extra": "avg mem: 161.32158172778946, max mem: 175.66015625, count: 58891"
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
        "date": 1752175270131,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 19.292604,
            "unit": "median cpu",
            "extra": "avg cpu: 18.562342903220717, max cpu: 54.187195, count: 58907"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 173.75,
            "unit": "median mem",
            "extra": "avg mem: 172.7537086397839, max mem: 181.984375, count: 58907"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 21957,
            "unit": "median block_count",
            "extra": "avg block_count: 19677.461659904595, max block_count: 21957.0, count: 58907"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 73,
            "unit": "median segment_count",
            "extra": "avg segment_count: 75.8697947612338, max segment_count: 206.0, count: 58907"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 9.708737,
            "unit": "median cpu",
            "extra": "avg cpu: 11.594125130018181, max cpu: 49.261086, count: 58907"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 167.21875,
            "unit": "median mem",
            "extra": "avg mem: 158.17957185160506, max mem: 176.30078125, count: 58907"
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
        "date": 1752175540390,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 19.261637,
            "unit": "median cpu",
            "extra": "avg cpu: 18.323685766427563, max cpu: 64.35644, count: 58893"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 186.71875,
            "unit": "median mem",
            "extra": "avg mem: 185.40375301924678, max mem: 243.90234375, count: 58893"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 40219,
            "unit": "median block_count",
            "extra": "avg block_count: 35149.61728898171, max block_count: 40219.0, count: 58893"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 74,
            "unit": "median segment_count",
            "extra": "avg segment_count: 76.80316845805105, max segment_count: 172.0, count: 58893"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 9.646302,
            "unit": "median cpu",
            "extra": "avg cpu: 8.589558159687785, max cpu: 34.653465, count: 58893"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 166.3359375,
            "unit": "median mem",
            "extra": "avg mem: 159.7304338615158, max mem: 177.57421875, count: 58893"
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
        "date": 1752175817249,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 19.35484,
            "unit": "median cpu",
            "extra": "avg cpu: 20.105450479850916, max cpu: 48.859932, count: 58901"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 176.30078125,
            "unit": "median mem",
            "extra": "avg mem: 175.3209668686652, max mem: 183.625, count: 58901"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 11171,
            "unit": "median block_count",
            "extra": "avg block_count: 9511.569990322745, max block_count: 11171.0, count: 58901"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 67,
            "unit": "median segment_count",
            "extra": "avg segment_count: 67.61136483251558, max segment_count: 121.0, count: 58901"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 9.708737,
            "unit": "median cpu",
            "extra": "avg cpu: 11.934945869493916, max cpu: 38.647343, count: 58901"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 168.0078125,
            "unit": "median mem",
            "extra": "avg mem: 162.0680514231507, max mem: 176.71875, count: 58901"
          }
        ]
      }
    ]
  }
}