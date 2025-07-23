window.BENCHMARK_DATA = {
  "lastUpdate": 1753292526239,
  "repoUrl": "https://github.com/paradedb/paradedb",
  "entries": {
    "pg_search single-server.toml Performance - TPS": [
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
        "date": 1752440985886,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 306.3111752901644,
            "unit": "median tps",
            "extra": "avg tps: 307.6931278290426, max tps: 520.3365980533484, count: 55107"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2536.7213297437415,
            "unit": "median tps",
            "extra": "avg tps: 2524.1791177870427, max tps: 2577.5381654331127, count: 55107"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 305.2038031331571,
            "unit": "median tps",
            "extra": "avg tps: 306.3767194694763, max tps: 484.06616612217107, count: 55107"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 267.7627374966694,
            "unit": "median tps",
            "extra": "avg tps: 267.2598482688497, max tps: 430.7737903441194, count: 55107"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 155.08945061007833,
            "unit": "median tps",
            "extra": "avg tps: 154.11631341151, max tps: 163.90287680657562, count: 110214"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 135.65325779836144,
            "unit": "median tps",
            "extra": "avg tps: 134.99656233652175, max tps: 147.78242179006236, count: 55107"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 5.390145064605325,
            "unit": "median tps",
            "extra": "avg tps: 8.90633164011802, max tps: 940.1747972983138, count: 55107"
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
        "date": 1752440998308,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 285.03547980341597,
            "unit": "median tps",
            "extra": "avg tps: 284.5664475907636, max tps: 443.99798977665193, count: 55117"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2280.7349466269866,
            "unit": "median tps",
            "extra": "avg tps: 2266.1489210943914, max tps: 2296.1671773055514, count: 55117"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 237.356420152653,
            "unit": "median tps",
            "extra": "avg tps: 239.7133171667209, max tps: 446.9448088244301, count: 55117"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 233.32858450425005,
            "unit": "median tps",
            "extra": "avg tps: 233.01200561653596, max tps: 356.4562333234179, count: 55117"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 133.1109599699799,
            "unit": "median tps",
            "extra": "avg tps: 134.161937956422, max tps: 145.3352230343854, count: 110234"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 135.0507020088833,
            "unit": "median tps",
            "extra": "avg tps: 138.08492580934154, max tps: 157.536776469744, count: 55117"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 3.94053259686009,
            "unit": "median tps",
            "extra": "avg tps: 8.313916343810918, max tps: 1121.39052425007, count: 55117"
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
        "date": 1752440998850,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 271.9593434748166,
            "unit": "median tps",
            "extra": "avg tps: 272.28677116029274, max tps: 475.5747474993365, count: 55130"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2019.6208111655794,
            "unit": "median tps",
            "extra": "avg tps: 2018.8954776980363, max tps: 2408.5971419167427, count: 55130"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 269.81723373540353,
            "unit": "median tps",
            "extra": "avg tps: 270.34188692359044, max tps: 443.5282070858116, count: 55130"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 224.698678800447,
            "unit": "median tps",
            "extra": "avg tps: 225.53739780980712, max tps: 362.169458800037, count: 55130"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 135.49123934417017,
            "unit": "median tps",
            "extra": "avg tps: 139.12872467955398, max tps: 153.29109269682283, count: 110260"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 130.71653115941604,
            "unit": "median tps",
            "extra": "avg tps: 133.08391895358787, max tps: 148.70270170304914, count: 55130"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 4.3696814096749925,
            "unit": "median tps",
            "extra": "avg tps: 8.5874659065905, max tps: 1163.9817487661794, count: 55130"
          }
        ]
      },
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
          "id": "4fd1b2b6b6664d03946be0f4836732f0f40df0cc",
          "message": "chore: Rename datasets and add string paging queries (#2834)\n\n## What\n\nAdd a high-cardinality paging/top-n query to the benchmarks, and rename\ndatasets to match their content. Additionally, improve the generation\nscript for the `docs` dataset to avoid joins and allow for deterministic\nrelative-position queries.\n\n## Why\n\nWe don't currently have a high-cardinality string paging/top-n query in\nthe benchmark. We have top-n on a string column, but only for\nlow-cardinality values (`top_n-string.sql`). The top-n case represented\nan important gap that a user encountered, which #2828 addresses.\n\nThe names of the `benchmark` datasets don't currently describe their\nshape / schema, and for the `join` dataset in particular, that would\ndiscourage using it for other types of queries. We rename it to `docs`\nhere, and then use the `pages` table as the dataset for top-n.\n\n## Tests\n\nTested locally that the new query demonstrates a speedup for #2828.",
          "timestamp": "2025-07-13T18:04:27Z",
          "url": "https://github.com/paradedb/paradedb/commit/4fd1b2b6b6664d03946be0f4836732f0f40df0cc"
        },
        "date": 1752441065373,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 287.4410162524614,
            "unit": "median tps",
            "extra": "avg tps: 290.79458171237866, max tps: 532.1087582874854, count: 54617"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2309.093793540984,
            "unit": "median tps",
            "extra": "avg tps: 2307.4051934213358, max tps: 2514.506580333097, count: 54617"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 287.910233299443,
            "unit": "median tps",
            "extra": "avg tps: 291.8019135736587, max tps: 537.4094673147708, count: 54617"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 257.8723536897191,
            "unit": "median tps",
            "extra": "avg tps: 259.8010282477663, max tps: 447.463310357733, count: 54617"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 139.78571655359298,
            "unit": "median tps",
            "extra": "avg tps: 139.88588374700302, max tps: 159.86662399337084, count: 109234"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 152.4945316017401,
            "unit": "median tps",
            "extra": "avg tps: 150.75898831967797, max tps: 153.2469948209186, count: 54617"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 4.815295851075635,
            "unit": "median tps",
            "extra": "avg tps: 9.03018688698432, max tps: 1067.3486312854827, count: 54617"
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
        "date": 1752441066775,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 318.8854337388586,
            "unit": "median tps",
            "extra": "avg tps: 318.32041794923697, max tps: 515.6490259342258, count: 55224"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2234.454466260472,
            "unit": "median tps",
            "extra": "avg tps: 2234.9747233696885, max tps: 2473.4211331042775, count: 55224"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 288.76413822570146,
            "unit": "median tps",
            "extra": "avg tps: 289.73811805418615, max tps: 535.3061366046683, count: 55224"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 255.28983754468982,
            "unit": "median tps",
            "extra": "avg tps: 257.15995840263514, max tps: 448.30273704511563, count: 55224"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 145.41953728256377,
            "unit": "median tps",
            "extra": "avg tps: 154.36036451251746, max tps: 166.76116958026768, count: 110448"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 128.80537515210105,
            "unit": "median tps",
            "extra": "avg tps: 128.6905406449407, max tps: 135.49748103407882, count: 55224"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 4.275111938382058,
            "unit": "median tps",
            "extra": "avg tps: 8.695642606860186, max tps: 1225.4661673300523, count: 55224"
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
        "date": 1752441100600,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 316.80640254613456,
            "unit": "median tps",
            "extra": "avg tps: 317.9973664370396, max tps: 542.0596433937652, count: 54990"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2349.9317704166933,
            "unit": "median tps",
            "extra": "avg tps: 2351.8042800405537, max tps: 2597.4815900786593, count: 54990"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 310.5760512857851,
            "unit": "median tps",
            "extra": "avg tps: 312.72894358990874, max tps: 515.8315746887479, count: 54990"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 275.23553296850116,
            "unit": "median tps",
            "extra": "avg tps: 277.10509750780125, max tps: 448.7479371618263, count: 54990"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 152.6247417026238,
            "unit": "median tps",
            "extra": "avg tps: 154.75131838636202, max tps: 160.09704709826937, count: 109980"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 133.64550229646755,
            "unit": "median tps",
            "extra": "avg tps: 133.29140476702466, max tps: 140.69086106635774, count: 54990"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 4.692168069075153,
            "unit": "median tps",
            "extra": "avg tps: 9.25591832582167, max tps: 1144.4777233133545, count: 54990"
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
          "id": "47bbe518381e1429f228328336dad78e99636ad9",
          "message": "chore: Upgrade to `0.16.0` (#2720)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-06-23T23:04:37Z",
          "url": "https://github.com/paradedb/paradedb/commit/47bbe518381e1429f228328336dad78e99636ad9"
        },
        "date": 1752441102327,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 131.5665344788818,
            "unit": "median tps",
            "extra": "avg tps: 131.66352032989778, max tps: 260.5206026623101, count: 55093"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 828.4482148821078,
            "unit": "median tps",
            "extra": "avg tps: 828.7021117439185, max tps: 1376.3662611014565, count: 55093"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 129.60446593374806,
            "unit": "median tps",
            "extra": "avg tps: 130.0377860181478, max tps: 260.5918005043881, count: 55093"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 105.46496219117446,
            "unit": "median tps",
            "extra": "avg tps: 105.95759092764341, max tps: 213.0519320689028, count: 55093"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 131.7245998775656,
            "unit": "median tps",
            "extra": "avg tps: 134.33373631853283, max tps: 143.81621734699058, count: 110186"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 116.81072348268977,
            "unit": "median tps",
            "extra": "avg tps: 115.23857962531804, max tps: 121.9170023545966, count: 55093"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 2.8368780156117754,
            "unit": "median tps",
            "extra": "avg tps: 6.900378916766535, max tps: 1112.3792234258165, count: 55093"
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
          "id": "b96e41e25c3fd2126f54aa5cb66d4deffb482047",
          "message": "perf: Lazily load fast fields dictionaries. (#2842)\n\n## What\n\nLazily load fast field dictionaries from buffers: see\nhttps://github.com/paradedb/tantivy/pull/55\n\n## Why\n\nA customer reported slower-than-expected paging on a string/uuid column.\n85% of the time for that query was being spent in _opening_ a fast\nfields string/bytes column, with a large fraction of that time spent\nfully consuming the column's `Dictionary`.\n\n## Tests\n\nSee the attached benchmark results:\n* [`docs`\ndataset](https://github.com/paradedb/paradedb/pull/2842#pullrequestreview-3014379545)\n    * No regressions.\n    * 2x faster for `top_n-score`\n    * 1.4x faster for `highlighting` \n* [`logs`\ndataset](https://github.com/paradedb/paradedb/pull/2842#pullrequestreview-3014350211)\n    * No regressions.\n    * 4.5x faster for `paging-string-max`\n    * 1.7x faster for `paging-string-median`\n    * 1.6x faster for `paging-string-min`\n\nThe `paging-string-*` benchmarks were added in #2834 to highlight this\nparticular issue.",
          "timestamp": "2025-07-14T08:28:09-07:00",
          "tree_id": "d144335dcb7c7f138a112c01e5b9ff5e0168fe37",
          "url": "https://github.com/paradedb/paradedb/commit/b96e41e25c3fd2126f54aa5cb66d4deffb482047"
        },
        "date": 1752507918468,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 320.34033261446194,
            "unit": "median tps",
            "extra": "avg tps: 320.0366677774066, max tps: 505.1612771787176, count: 54984"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2186.9465033112137,
            "unit": "median tps",
            "extra": "avg tps: 2193.2580398231044, max tps: 2565.4430057568543, count: 54984"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 320.6164037390294,
            "unit": "median tps",
            "extra": "avg tps: 321.20580390459014, max tps: 534.9963127705213, count: 54984"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 289.3068538659048,
            "unit": "median tps",
            "extra": "avg tps: 289.06498789477143, max tps: 434.09221334069554, count: 54984"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 149.44506617730954,
            "unit": "median tps",
            "extra": "avg tps: 149.26738588429603, max tps: 159.83780103582333, count: 109968"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 134.36034939221275,
            "unit": "median tps",
            "extra": "avg tps: 134.31913067408195, max tps: 144.850989381662, count: 54984"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 5.864855221310597,
            "unit": "median tps",
            "extra": "avg tps: 9.952981181211312, max tps: 1062.695868132195, count: 54984"
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
          "id": "ee6395b4b4d4ca6f44e2c89b74afd2308d4415a8",
          "message": "fix: orphaned delete entries get GCed too early (#2845)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nWhen running a new stressgres suite (coming in a future PR), I hit a\nmysterious bug where it looked like vacuum could cause corruption of\nsome pages.\n\nTurns out it's caused by scenarios where:\n\n1. A `DeleteEntry` already exists for a `SegmentMetaEntry`, and a new\none is created\n2. A new, \"fake\" `SegmentMetaEntry` gets created for the purpose of\nstoring the old `DeleteEntry`, so its blocks can get garbage collected\n3. Because this \"fake\" entry is invisible to all readers besides the\ngarbage collector, it doesn't get pinned and can get garbage collected\ntoo early (i.e. while a reader is still pinning the old `DeleteEntry`)\n\nThe solution is to copy all of the contents of the old\n`SegmentMetaEntry` to the fake one, so that the \"pintest blockno\" of the\nfake entry is that same as that of the entry with the new `DeleteEntry`.\nThat way, the `DeleteEntry` doesn't get garbage collected until the pin\nis released.\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-07-14T15:46:29-04:00",
          "tree_id": "3dc55f49de121cf04534f48e3584a2a3ae333407",
          "url": "https://github.com/paradedb/paradedb/commit/ee6395b4b4d4ca6f44e2c89b74afd2308d4415a8"
        },
        "date": 1752523325508,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 301.9666978664777,
            "unit": "median tps",
            "extra": "avg tps: 302.06048906291755, max tps: 531.7082304256362, count: 55164"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2313.7055097403972,
            "unit": "median tps",
            "extra": "avg tps: 2304.1777467335255, max tps: 2665.074111076118, count: 55164"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 303.562357343436,
            "unit": "median tps",
            "extra": "avg tps: 304.21293647018865, max tps: 504.7387284589783, count: 55164"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 247.90344007713242,
            "unit": "median tps",
            "extra": "avg tps: 248.78827685846798, max tps: 416.59428340992423, count: 55164"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 155.6409323961807,
            "unit": "median tps",
            "extra": "avg tps: 155.7193212636969, max tps: 160.05364832848377, count: 110328"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 139.16054140037977,
            "unit": "median tps",
            "extra": "avg tps: 138.58103916617492, max tps: 140.3050881236465, count: 55164"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 4.961278230928719,
            "unit": "median tps",
            "extra": "avg tps: 9.438792350180849, max tps: 1254.4832093694843, count: 55164"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "eb3bc1d570c85d60804f6aab2f2d2cf998bd7597",
          "message": "ci: benchmark workflow cleanups (#2851)\n\nThis is an attempt to cleanup the benchmark workflows a little bit.  \n\n- Centralizes checking out the latest benchmark code/suites/actions into\na composite action.\n- figures out the PR #/title being tested\n- Changes the slack notification messages to be reactive to the\nenvironment to hopefully avoid conflicts with -enterprise",
          "timestamp": "2025-07-15T12:15:54-04:00",
          "tree_id": "223c726790d68868f538b7f5aab9cf9904494f44",
          "url": "https://github.com/paradedb/paradedb/commit/eb3bc1d570c85d60804f6aab2f2d2cf998bd7597"
        },
        "date": 1752597089068,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 305.4751239219894,
            "unit": "median tps",
            "extra": "avg tps: 306.23446307359137, max tps: 528.2094493721808, count: 54712"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2498.87633454221,
            "unit": "median tps",
            "extra": "avg tps: 2471.2933767677114, max tps: 2624.154564729526, count: 54712"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 306.4802582445157,
            "unit": "median tps",
            "extra": "avg tps: 307.4277099132346, max tps: 516.6447994389714, count: 54712"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 242.84460041715704,
            "unit": "median tps",
            "extra": "avg tps: 245.16532482624004, max tps: 435.8525509076869, count: 54712"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 153.651459454252,
            "unit": "median tps",
            "extra": "avg tps: 152.73316934924685, max tps: 157.1763490858229, count: 109424"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 137.07902744223273,
            "unit": "median tps",
            "extra": "avg tps: 136.50087448419276, max tps: 142.10078604826063, count: 54712"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 4.981613165249287,
            "unit": "median tps",
            "extra": "avg tps: 9.659748879884381, max tps: 1248.7730804484593, count: 54712"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "c7bdeebed6249725c87b87c276d2e09dfaffd152",
          "message": "ci: publish stressgres benchmark graphs (#2852)\n\nThis will publish every stressgres benchmark graph to the new\n`paradedb/benchmark-data` repo.\n\nIt also ensures that when one of our \"[benchmark]\" labels are applied,\nthat it does *not* pull the benchmarks from main, but instead from the\nPR branch itself.",
          "timestamp": "2025-07-15T18:13:26-04:00",
          "tree_id": "f18bf519521a03613c5b12c861092f24314476ad",
          "url": "https://github.com/paradedb/paradedb/commit/c7bdeebed6249725c87b87c276d2e09dfaffd152"
        },
        "date": 1752618537950,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 325.0219765492938,
            "unit": "median tps",
            "extra": "avg tps: 324.0733157615584, max tps: 519.9401336408907, count: 55161"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2127.3484985872155,
            "unit": "median tps",
            "extra": "avg tps: 2126.8694309698953, max tps: 2568.5770449329925, count: 55161"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 277.81309281998165,
            "unit": "median tps",
            "extra": "avg tps: 279.98713002297825, max tps: 484.64899614168775, count: 55161"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 292.6490681643571,
            "unit": "median tps",
            "extra": "avg tps: 292.0705978891881, max tps: 448.55441900338474, count: 55161"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 141.08016889491398,
            "unit": "median tps",
            "extra": "avg tps: 140.7201466343214, max tps: 159.68848153523228, count: 110322"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 126.12401338176589,
            "unit": "median tps",
            "extra": "avg tps: 125.70250727258181, max tps: 145.37493662885853, count: 55161"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 4.267097069170552,
            "unit": "median tps",
            "extra": "avg tps: 8.382422137054183, max tps: 1124.984672083843, count: 55161"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Philippe Noël",
            "username": "philippemnoel",
            "email": "21990816+philippemnoel@users.noreply.github.com"
          },
          "committer": {
            "name": "Stu Hood",
            "username": "stuhood",
            "email": "stuhood@gmail.com"
          },
          "id": "8c164d666c4af2049b439690afa7823ab5be2c88",
          "message": "ci: Post \"One-branch Release Model\" Improvements (#2792)\n\n# Ticket(s) Closed\n\n- Closes #N/A\n\n## What\n- Don't make Docker Scout post on PRs that don't modify the Dockerfile,\nnot necessary.\n- Make sure the SchemaBot workflow errors when community contributors\nmake a PR, so we can catch needed changes to the SQL upgrade script.\n- Rename it to SchemaBot\n\n## Why\nQoL\n\n## How\n^\n\n## Tests\nCI",
          "timestamp": "2025-07-08T15:49:16Z",
          "url": "https://github.com/paradedb/paradedb/commit/8c164d666c4af2049b439690afa7823ab5be2c88"
        },
        "date": 1752668513520,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 315.8428380993767,
            "unit": "median tps",
            "extra": "avg tps: 316.6870899892203, max tps: 522.5800943866939, count: 55077"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2320.926556132953,
            "unit": "median tps",
            "extra": "avg tps: 2326.3086373013957, max tps: 2591.1336883581325, count: 55077"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 301.3644541628425,
            "unit": "median tps",
            "extra": "avg tps: 302.7329004447359, max tps: 525.3100041333333, count: 55077"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 275.69848964891816,
            "unit": "median tps",
            "extra": "avg tps: 275.54867864964064, max tps: 447.0924638183717, count: 55077"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 155.54810824512654,
            "unit": "median tps",
            "extra": "avg tps: 162.95433038112125, max tps: 173.331819532512, count: 110154"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 137.51159221897996,
            "unit": "median tps",
            "extra": "avg tps: 137.32489841228025, max tps: 145.5387283399039, count: 55077"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 4.968872895106366,
            "unit": "median tps",
            "extra": "avg tps: 9.338879413398578, max tps: 1134.758211394107, count: 55077"
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
          "id": "f7c13c858851200e8ab5395779f821ca484cda0d",
          "message": "feat: Add a custom scan for aggregates (#2763)\n\n## What\n\nAdd a new `CustomScan` (installed with `create_upper_paths_hook`) which\nreplaces simple aggregate plans on `bm25`-indexed tables with aggregate\nimplementations using [Tantivy\naggregates](https://docs.rs/tantivy/latest/tantivy/aggregation/index.html).\n\n## Why\n\nTantivy aggregates can be significantly faster (in benchmarks, we've\nmeasured between 4-10x for bucketing/faceting queries). They have been\nexposed via `paradedb.aggregate` for a while now, but that function\nrequires learning a new API, and does not feel \"Postgres native\".\n\n## How\n\n* Adjust `CustomPathBuilder` and `CustomPathMethods` to allow multiple\n`CustomScan` implementations.\n* Remove the `CustomScan::PrivateData: Default` bound, as it requires\nthe `PrivateData` to start in an illegal state.\n* Move `qual_inspect` to a reusable location.\n* Split out a module to be used by both the `aggregate` API method and\nby the aggregate custom scan.\n* Implement the \"ParadeDB Aggregate Scan\" `CustomScan` type\n    * Add the new `CustomScan` type, hidden behind a GUC\n    * Filter Paths to those which represent `count(*)` queries\n    * Extract `quals` during `CustomPath` generation\n* Replace `Aggrefs` in target lists with `FuncExprs` while producing a\n`CustomPlan`\n* Execute a `count(*)` aggregate by pushing down a `value_count`\naggregate on the `ctid`\n\n## Tests\n\nAdded tests to validate that:\n* the GUC properly controls usage\n* the scan does not trigger for unsupported aggregates, tables without a\n`bm25` index, or group-bys (for now)",
          "timestamp": "2025-07-16T09:12:24-07:00",
          "tree_id": "69b043a9363fcf6ce2de468c97d14e41f593f017",
          "url": "https://github.com/paradedb/paradedb/commit/f7c13c858851200e8ab5395779f821ca484cda0d"
        },
        "date": 1752683373932,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 330.9111602214654,
            "unit": "median tps",
            "extra": "avg tps: 329.92167490019744, max tps: 537.8537178898554, count: 54969"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2172.65116421297,
            "unit": "median tps",
            "extra": "avg tps: 2177.7799334667047, max tps: 2557.0574311107525, count: 54969"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 297.67917720904876,
            "unit": "median tps",
            "extra": "avg tps: 298.6746436727678, max tps: 521.0461764487536, count: 54969"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 300.5033144677903,
            "unit": "median tps",
            "extra": "avg tps: 298.9508004809313, max tps: 445.0016062332977, count: 54969"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 142.3523680439219,
            "unit": "median tps",
            "extra": "avg tps: 142.28634535921998, max tps: 160.0348674824421, count: 109938"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 128.57636426402675,
            "unit": "median tps",
            "extra": "avg tps: 128.79681433520523, max tps: 142.46938263004637, count: 54969"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 5.080037678584389,
            "unit": "median tps",
            "extra": "avg tps: 8.826804752925225, max tps: 1010.3071535808316, count: 54969"
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
          "id": "c779dbb72178c1aa0e14ad94c72eeb9937251acd",
          "message": "feat: a background merger for large layers (#2743)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nLaunches Postgres [dynamic background\nworkers](https://docs.rs/pgrx/latest/pgrx/bgworkers/struct.DynamicBackgroundWorker.html)\nto merge large layers in the background.\n\nThis is configured by a new index setting, `background_layer_sizes`. Any\nlayers specified here will be merged in the background.\n\n## Why\n\nWrite throughput, and create more balanced indexes with fewer segments.\n\n## How\n\n1. The default (foreground) `layer_sizes` is now `10kb`, `100kb`, `1mb`\n2. The default `background_layer_sizes` is `10mb`, `100mb`, `1gb`,\n`10gb`, `100gb`, `1tb`\n3. After `aminsert` merges in the foreground, it checks to see if there\nare enough candidates for a background merge. If so, it spawns a dynamic\nbackground worker process to do the merge.\n4. `amvacuumcleanup` can also spawn a background worker, allowing the\nuser to rebalance their index with a `VACUUM`.\n\nAdditionally, I've tweaked merge policy to make it smarter:\n\n1. Terminate a merge early if we predict that, after the merge, we will\nend up with fewer than `target_segment_count` segments.\n2. Set the max layer size as byte size of index divided by\n`target_segment_count`. Discard all layer sizes above that, which avoids\nthe problem of merging too many segments into one giant segment.\n\nThis has allowed us to delete some code:\n\n1. `force_merge` is deprecated\n2. No longer need to store the segments created by an index build and\navoid merging them, since we just do expensive merges in the background\n\n## Tests\n\nAdded a new stressgres `.toml` file that configures the layer\nsizes/threshold, which triggers background merging.\n\n---------\n\nSigned-off-by: Ming <ming.ying.nyc@gmail.com>\nCo-authored-by: Stu Hood <stuhood@paradedb.com>\nCo-authored-by: Philippe Noël <21990816+philippemnoel@users.noreply.github.com>\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>",
          "timestamp": "2025-07-16T12:44:23-04:00",
          "tree_id": "153c406e456a638a6b68ca9123210bbd498d66a5",
          "url": "https://github.com/paradedb/paradedb/commit/c779dbb72178c1aa0e14ad94c72eeb9937251acd"
        },
        "date": 1752685298473,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 1208.0354635097945,
            "unit": "median tps",
            "extra": "avg tps: 1206.4410751573776, max tps: 1246.9093546096562, count: 55119"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2728.6415683566192,
            "unit": "median tps",
            "extra": "avg tps: 2720.6510578172797, max tps: 2767.966283109858, count: 55119"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 1201.7330530861334,
            "unit": "median tps",
            "extra": "avg tps: 1198.897374649925, max tps: 1207.797083605834, count: 55119"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 1042.902855021601,
            "unit": "median tps",
            "extra": "avg tps: 1039.4849084799466, max tps: 1050.2722969022095, count: 55119"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 168.85197205251944,
            "unit": "median tps",
            "extra": "avg tps: 173.53358419604928, max tps: 183.05003792102082, count: 110238"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 149.79281249322253,
            "unit": "median tps",
            "extra": "avg tps: 150.34212698643736, max tps: 154.83215221855724, count: 55119"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 42.310029001092055,
            "unit": "median tps",
            "extra": "avg tps: 48.50842428049031, max tps: 779.5635847140054, count: 55119"
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
          "id": "d197630155f55d13eea0a81841951645a367a5f7",
          "message": "test: Make GUCs a tested property, and add proptests for the aggregate custom scan (#2788)\n\n## What\n\nMove the GUCs that are in use into a test property, which reduces\nruntime while allowing for testing arbitrary combinations of GUCs.\n\nAdditionally, added proptests for the aggregate custom scan by:\n* Adding its GUC to the generated set\n* Expanding the `generated_single_relation` test to use `COUNT(*)` in\nits target list.\n\n## Why\n\nTo add additional test coverage for #2763.",
          "timestamp": "2025-07-16T10:57:16-07:00",
          "tree_id": "6a92b62b640f7a0b2dc7e6507966b75f3bd00594",
          "url": "https://github.com/paradedb/paradedb/commit/d197630155f55d13eea0a81841951645a367a5f7"
        },
        "date": 1752689572238,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 1232.79691495007,
            "unit": "median tps",
            "extra": "avg tps: 1228.3090654304374, max tps: 1238.4637743778426, count: 55128"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2776.989134830413,
            "unit": "median tps",
            "extra": "avg tps: 2763.7941970742245, max tps: 2796.2593347019438, count: 55128"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 1226.3711804743555,
            "unit": "median tps",
            "extra": "avg tps: 1222.8106965527497, max tps: 1231.0797813540948, count: 55128"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 1021.2117073605026,
            "unit": "median tps",
            "extra": "avg tps: 1009.4873950060686, max tps: 1034.5354264583887, count: 55128"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 171.89605644805638,
            "unit": "median tps",
            "extra": "avg tps: 173.15487975133402, max tps: 180.4668051232225, count: 110256"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 152.39942396934728,
            "unit": "median tps",
            "extra": "avg tps: 151.03715323983516, max tps: 154.02371908111525, count: 55128"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 49.87543201351079,
            "unit": "median tps",
            "extra": "avg tps: 50.88819643787881, max tps: 748.9722228671704, count: 55128"
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
          "id": "58f6742b2cde46fe2c15077e59a0d5b091ba6000",
          "message": "chore: Upgrade to `0.17.0` (#2870)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-07-16T19:49:53-04:00",
          "tree_id": "b77118d365b31a4f3478b7c0571cf1f7e9f8a5b2",
          "url": "https://github.com/paradedb/paradedb/commit/58f6742b2cde46fe2c15077e59a0d5b091ba6000"
        },
        "date": 1752710829344,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 1258.3634672686396,
            "unit": "median tps",
            "extra": "avg tps: 1252.4765769847932, max tps: 1260.555691246181, count: 55161"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2637.4363518177065,
            "unit": "median tps",
            "extra": "avg tps: 2636.9434755766333, max tps: 2683.8522153066915, count: 55161"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 1247.7712730589403,
            "unit": "median tps",
            "extra": "avg tps: 1241.723333001524, max tps: 1250.4847908825875, count: 55161"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 1061.0642151901386,
            "unit": "median tps",
            "extra": "avg tps: 1054.4104553841385, max tps: 1069.3335397045223, count: 55161"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 173.26323740486797,
            "unit": "median tps",
            "extra": "avg tps: 172.4132669004323, max tps: 177.05807720913137, count: 110322"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 155.023010459012,
            "unit": "median tps",
            "extra": "avg tps: 153.44878294108676, max tps: 155.45973538018717, count: 55161"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 28.836449623973515,
            "unit": "median tps",
            "extra": "avg tps: 30.66624333407178, max tps: 784.8541976357053, count: 55161"
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
          "id": "86bc2b092d2857a997cd05af4b554ac96ba225fd",
          "message": "fix: Fix `rescan` for top-n (#2868)\n\n## What\n\nComments in both `Normal` and `TopN` scans claimed that their search\nresults would be reset by `PdbScanState`, but this was incorrect: they\nwere local variables in the struct, and needed to be reset locally.\n\n## Why\n\nFixes issue detected by the prop tests and repro'd by @eeeebbbbrrrr in\n#2751.\n\n## Tests\n\nNew regression test from #2751: thanks @eeeebbbbrrrr!\n\n---------\n\nCo-authored-by: Eric B. Ridge <eebbrr@paradedb.com>",
          "timestamp": "2025-07-17T08:59:32-07:00",
          "tree_id": "ccc9162bb96008badcb3046612d16394604ed6f7",
          "url": "https://github.com/paradedb/paradedb/commit/86bc2b092d2857a997cd05af4b554ac96ba225fd"
        },
        "date": 1752768913065,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 1246.9269240074839,
            "unit": "median tps",
            "extra": "avg tps: 1241.4896151905957, max tps: 1251.2348455074155, count: 54877"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2790.053911501425,
            "unit": "median tps",
            "extra": "avg tps: 2779.329643640563, max tps: 2805.869410750366, count: 54877"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 1242.8435857271666,
            "unit": "median tps",
            "extra": "avg tps: 1236.9044241867246, max tps: 1245.088001655496, count: 54877"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 980.8910714797896,
            "unit": "median tps",
            "extra": "avg tps: 974.6635948777052, max tps: 1014.4512526021379, count: 54877"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 170.60048264847552,
            "unit": "median tps",
            "extra": "avg tps: 179.00815440145536, max tps: 192.70584414080597, count: 109754"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 149.3005028495966,
            "unit": "median tps",
            "extra": "avg tps: 148.05838627043275, max tps: 162.39627008246873, count: 54877"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 37.71610939171841,
            "unit": "median tps",
            "extra": "avg tps: 40.15324910471507, max tps: 760.5169994562303, count: 54877"
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
          "id": "5772be0d3dea751c3609729efcc66cc69d9d30ee",
          "message": "chore: Remove debug output in aggregate scan (#2876)\n\n## What\n\nRemove a stray `println`.\n\n## Why\n\nIt's not doing anyone any good.",
          "timestamp": "2025-07-17T09:46:08-07:00",
          "tree_id": "06a002186c2a701283d73854f64c4225a08aa1a8",
          "url": "https://github.com/paradedb/paradedb/commit/5772be0d3dea751c3609729efcc66cc69d9d30ee"
        },
        "date": 1752771706812,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 1275.7235229655867,
            "unit": "median tps",
            "extra": "avg tps: 1267.4639562878053, max tps: 1285.8281186229488, count: 55255"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2691.76666892226,
            "unit": "median tps",
            "extra": "avg tps: 2693.1131080980194, max tps: 2731.4857478558024, count: 55255"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 1183.8335245747842,
            "unit": "median tps",
            "extra": "avg tps: 1183.3916865068961, max tps: 1201.9999424528216, count: 55255"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 1016.9156509271561,
            "unit": "median tps",
            "extra": "avg tps: 1014.4520531017663, max tps: 1026.7108755749962, count: 55255"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 161.6289975713852,
            "unit": "median tps",
            "extra": "avg tps: 172.57261080966006, max tps: 188.67077346504232, count: 110510"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 140.41029090970167,
            "unit": "median tps",
            "extra": "avg tps: 139.28778265321444, max tps: 140.9882793596449, count: 55255"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 34.127194813166504,
            "unit": "median tps",
            "extra": "avg tps: 37.059012597733066, max tps: 737.9545523309401, count: 55255"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "16307a200509cd31b42c1338f3c5b6e097a77fbf",
          "message": "fix: issue #2844 - Use `paradedb.with_index()` when necessary (#2879)\n\n# Ticket(s) Closed\n\n- Closes #2844\n\n## What\n\nWhen the right-hand-side of the `@@@` operator is a complex expression\nthat can't be evaluated to a `T_Const` during planning, we need to\nrewrite it to be a call to our `paradedb.with_index(<indexrelid>,\n<rhs>)` so that it can be properly used in Filter contexts during\nexecution.\n\nThis seems to come up, at least, during RLS rule evaluation if those\nrules use our `@@@` operator.\n\n## Why\n\nSo long as the right-hand-side of `@@@` is a `SearchQueryInput`, we need\nto be able to execute it when Postgres wants us to, whether that's\nduring a scan or, as in this case, a filter being run as part of RLS\nevaluation.\n\n## How\n\nWrapping Postgres expression nodes!\n\n## Tests\n\nI used the re-create the OP from #2844 put together as a regression\ntest. I seem to be lacking the creative energy today to reduce the test\ndown to its simplest form that doesn't require a complicated RLS\nconfiguration.",
          "timestamp": "2025-07-18T13:21:25-04:00",
          "tree_id": "f2e8c7ca2ce45eefd03b85ff45d56f0481ebae1a",
          "url": "https://github.com/paradedb/paradedb/commit/16307a200509cd31b42c1338f3c5b6e097a77fbf"
        },
        "date": 1752860221374,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 1219.8935673009012,
            "unit": "median tps",
            "extra": "avg tps: 1215.2523246262278, max tps: 1224.0012521428505, count: 55078"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2923.7198640999736,
            "unit": "median tps",
            "extra": "avg tps: 2907.378868072173, max tps: 2944.1100398541594, count: 55078"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 1239.869549404189,
            "unit": "median tps",
            "extra": "avg tps: 1233.2906111684024, max tps: 1242.2240863168583, count: 55078"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 1033.4249411391309,
            "unit": "median tps",
            "extra": "avg tps: 1025.7617725178384, max tps: 1052.9816156688282, count: 55078"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 163.87020591205558,
            "unit": "median tps",
            "extra": "avg tps: 166.47466553054034, max tps: 172.15221781836465, count: 110156"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 157.93152964244683,
            "unit": "median tps",
            "extra": "avg tps: 156.43678479197553, max tps: 158.516816002322, count: 55078"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 34.58616595823601,
            "unit": "median tps",
            "extra": "avg tps: 41.78505249654763, max tps: 707.6835324165057, count: 55078"
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
          "id": "8b45b24d47a7cac36696bae2d8505c750a7ee801",
          "message": "test: added more join regression tests + improved when to use Join quals (#2808)\n\n# Ticket(s) Closed\n\n- Closes #2807\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-07-18T10:40:03-07:00",
          "tree_id": "fb63bd81696cd27d03b6d5724551c3ff8fd59613",
          "url": "https://github.com/paradedb/paradedb/commit/8b45b24d47a7cac36696bae2d8505c750a7ee801"
        },
        "date": 1752861339896,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 1205.4208568896743,
            "unit": "median tps",
            "extra": "avg tps: 1207.1144608225204, max tps: 1253.3580580706457, count: 55215"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2699.2008447227113,
            "unit": "median tps",
            "extra": "avg tps: 2711.853278873035, max tps: 2849.940958762393, count: 55215"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 1219.1244938605862,
            "unit": "median tps",
            "extra": "avg tps: 1217.2018261189007, max tps: 1255.3973832199456, count: 55215"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 1007.9035544867553,
            "unit": "median tps",
            "extra": "avg tps: 994.4996902478134, max tps: 1018.646029954793, count: 55215"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 164.7726173113627,
            "unit": "median tps",
            "extra": "avg tps: 175.0111704805521, max tps: 191.37880673840797, count: 110430"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 143.50312481472966,
            "unit": "median tps",
            "extra": "avg tps: 144.42915709607087, max tps: 152.85131249629617, count: 55215"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 21.796006548150203,
            "unit": "median tps",
            "extra": "avg tps: 26.411693402303865, max tps: 767.2247715396436, count: 55215"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "77c6cd1aeb5f5c03bba892a38cd792a113cfbd39",
          "message": "chore: move code around (#2889)\n\n## What\n\nThis relocates the `src/boostrap/create_bm25.rs` source to\n`src/api/admin.rs`, which I believe is a more sensibile location and\nname.\n\nAdditionally it moves the `#[pg_extern] schema()` function from\n`src/api/builder_fns.rs` to this new(ly renamed) `admin.rs`, as it's an\nadministrative function, not a query builder function.\n\n## Why\n\nTrying to better organize things\n\n## How\n\n## Tests\n\nNo functional changes here.",
          "timestamp": "2025-07-22T12:39:21-04:00",
          "tree_id": "1ab036752fc8adf7ad2757741d4e274ab0e73c64",
          "url": "https://github.com/paradedb/paradedb/commit/77c6cd1aeb5f5c03bba892a38cd792a113cfbd39"
        },
        "date": 1753203295706,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 1223.7565385311345,
            "unit": "median tps",
            "extra": "avg tps: 1218.0213060236151, max tps: 1227.936233405042, count: 55046"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2700.78007463258,
            "unit": "median tps",
            "extra": "avg tps: 2699.6694861410824, max tps: 2741.4607643979557, count: 55046"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 1236.1452401072272,
            "unit": "median tps",
            "extra": "avg tps: 1229.496712311209, max tps: 1239.6467823315336, count: 55046"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 1014.8156984151757,
            "unit": "median tps",
            "extra": "avg tps: 1008.5281916773866, max tps: 1024.4658788644676, count: 55046"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 173.05159585579165,
            "unit": "median tps",
            "extra": "avg tps: 177.87356913818854, max tps: 186.77714521969176, count: 110092"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 145.05097473742177,
            "unit": "median tps",
            "extra": "avg tps: 144.79626064904642, max tps: 150.4990918608882, count: 55046"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 41.61645133167983,
            "unit": "median tps",
            "extra": "avg tps: 50.94915649313984, max tps: 810.5356668114823, count: 55046"
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
          "id": "1e26a38533fa872a935955128b2ecd08b3febf66",
          "message": "perf: Skip computing scores if not requested for top-n field queries (#2892)\n\n## What\n\n`SearchIndexReader::{top_by_field_in_segments,\ntop_by_string_field_in_segments}` were not using\n`SearchIndexReader::need_scores`, and were instead universally enabling\nscores.\n\nUse the `enable_scoring` helper and `self.need_scores` to skip computing\nscores where possible.\n\n## Why\n\nComputing scores requires field norms, which ([if they have not been\ndisabled](https://docs.paradedb.com/documentation/indexing/record)) can\ntake time to load and use.",
          "timestamp": "2025-07-22T15:58:56-07:00",
          "tree_id": "2a41c9e06be04ef92fed2b265dc3f90340fb0e37",
          "url": "https://github.com/paradedb/paradedb/commit/1e26a38533fa872a935955128b2ecd08b3febf66"
        },
        "date": 1753226071223,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 1230.540494198395,
            "unit": "median tps",
            "extra": "avg tps: 1228.5048266499005, max tps: 1237.4384718998551, count: 55159"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2852.4112867588165,
            "unit": "median tps",
            "extra": "avg tps: 2830.4466073086696, max tps: 2859.4945005732516, count: 55159"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 1263.1318840772733,
            "unit": "median tps",
            "extra": "avg tps: 1256.104347610509, max tps: 1265.4076108560569, count: 55159"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 1059.6132992501766,
            "unit": "median tps",
            "extra": "avg tps: 1046.5864871514912, max tps: 1065.0338273218633, count: 55159"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 177.22306197677597,
            "unit": "median tps",
            "extra": "avg tps: 182.39038309700967, max tps: 192.61772313668288, count: 110318"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 153.03500347603702,
            "unit": "median tps",
            "extra": "avg tps: 152.66859364683089, max tps: 154.5240866737964, count: 55159"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 41.89032597054898,
            "unit": "median tps",
            "extra": "avg tps: 52.69106112926876, max tps: 705.1122045051028, count: 55159"
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
          "id": "cbd2e802901988c181281a9b7f772543e1df32d2",
          "message": "chore: add `background-merge.toml` to CI (#2895)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nThis new stressgres file was instrumental in catching the bug fixed in\n#2845 and was used to test background merging, but has not been added to\nCI.\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-07-23T10:26:09-04:00",
          "tree_id": "bb8e3ca51c2d0de5e56be65f478f11096b0d410e",
          "url": "https://github.com/paradedb/paradedb/commit/cbd2e802901988c181281a9b7f772543e1df32d2"
        },
        "date": 1753281702768,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 1238.1554277707633,
            "unit": "median tps",
            "extra": "avg tps: 1227.86208954594, max tps: 1241.154754241053, count: 55205"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2775.393228591363,
            "unit": "median tps",
            "extra": "avg tps: 2754.786799935381, max tps: 2791.255540404496, count: 55205"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 1245.2597555285251,
            "unit": "median tps",
            "extra": "avg tps: 1240.5476385436898, max tps: 1249.3464526845712, count: 55205"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 1046.0873249629806,
            "unit": "median tps",
            "extra": "avg tps: 1041.083569661939, max tps: 1061.436176848277, count: 55205"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 180.18424545496453,
            "unit": "median tps",
            "extra": "avg tps: 180.73226397279493, max tps: 185.17380010463705, count: 110410"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 158.37373785019372,
            "unit": "median tps",
            "extra": "avg tps: 156.96237973840113, max tps: 158.9642442931985, count: 55205"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 42.96335750849179,
            "unit": "median tps",
            "extra": "avg tps: 49.423641971005026, max tps: 901.1282125220777, count: 55205"
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
          "id": "53fa29a57e7daa2bdec9bf05f86d8db50088fd51",
          "message": "perf: Avoid a re-query when a top-n query is exhausted (#2888)\n\n## What\n\nSplit the iterators consumed by the `TopN` exec method and by the\n`Normal`/`FF` exec methods into two separate implementations.\n\nAfter splitting them, use the known exact size of the `TopN` iterator to\nexit early (as `exhausted: bool`) if we had fewer matches than were\nrequested. This avoids re-querying an iterator (and re-scanning its\ncolumns) that we know has no more results.\n\n## Why\n\nThe two types of consumers of these iterators have very different\nconsumption patterns and constraints:\n* TopN knows exactly how many results there will be, and already buffers\nthem all in memory. It would like to be able to know the precise count\nof results.\n* `Normal` and `FF` exec methods do not know the total number of results\nto expect, and in some cases would like to be able to consume in a\nsegment-aware fashion in order to be able to late-fetch fast field\ncolumns (see #2623).\n\n## Tests\n\nAdded a test to cement the change in query count.\n\nBenchmarks show a 1.5x speedup for `paging-string-max`, and no change\nfor other queries.",
          "timestamp": "2025-07-23T09:13:45-07:00",
          "tree_id": "c162e59fb9a6b4523c0ff39e10d9d54d92761460",
          "url": "https://github.com/paradedb/paradedb/commit/53fa29a57e7daa2bdec9bf05f86d8db50088fd51"
        },
        "date": 1753288159284,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 1238.629942194621,
            "unit": "median tps",
            "extra": "avg tps: 1236.555006221168, max tps: 1245.9307340737812, count: 55213"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2634.2268220939786,
            "unit": "median tps",
            "extra": "avg tps: 2638.332278912309, max tps: 2673.94474843985, count: 55213"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 1205.51400474282,
            "unit": "median tps",
            "extra": "avg tps: 1204.2073079390982, max tps: 1228.886654891459, count: 55213"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 1049.680852658833,
            "unit": "median tps",
            "extra": "avg tps: 1043.8969711508491, max tps: 1066.1527017486392, count: 55213"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 177.31748109191562,
            "unit": "median tps",
            "extra": "avg tps: 177.40880578103796, max tps: 181.18959983070656, count: 110426"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 155.18906055263136,
            "unit": "median tps",
            "extra": "avg tps: 153.63754183082375, max tps: 156.11638100409218, count: 55213"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 68.86646114848666,
            "unit": "median tps",
            "extra": "avg tps: 66.84238445452274, max tps: 875.1915575521593, count: 55213"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "880ee607618b7ffe12f2784d03be3110b4df6cdf",
          "message": "feat: text search operators (#2877)\n\n## What\n\nThis is the implementation of 4 new operators designed around common\n\"text search\" patterns: `===` (exact token), `&&&` (tokenized term\nconjunction) `|||` (tokenized term disjunction), and `###` (phrase).\n\nFirst, `@@@` still exists and still does what it's always done, which is\nrun the right-hand-side through tantivy's \"query parser\".\n\nThese new operators have specific meanings. The spelling of these\noperators is both to maintain some kind of consistency in that they're\nall 3 of the same symbol and also to try and confer their meaning.\n\nIn all cases, the left-hand-side of the operator is required to be a\nreference to a Postgres `TEXT`/`VARCHAR` field and the right-hand-side\nis text. The operator definition defines how the right-hand-side input\nis applied to the left-hand-side field. These are \"text search\"\noperators and would be nonsensical applied to non-text fields.\n\n- `field === 'TEXT'` - this is the exact term match operator. The\nright-hand-side value is used, unmodified, to perform the equivalent of\nthe `paradedb.term()` function.\n\n- `field === ARRAY['...', '...', '...']` - similar to the above, but\nrewrites to the `paradedb.term_set()` function.\n\n- `field &&& 'TEXT'` - this is the \"match conjunction\" operator. It\ntokenizes the right-hand-side using the index tokenizer defined by the\nleft-hand-side field and searches for documents that contain **all** of\nthe tokenized terms. Think, \"find all documents that contain all terms\ntokenized from this text input\". This is the equivalent of the\n`paradedb.match` function with the `conjunction_mode => true`.\n\n- `field ||| 'TEXT'` - this is the \"match disjunction\" operator. It\ntokenizes the right-hand-side using the index tokenizer defined by the\nleft-hand-side field and searches for documents that contain **any** of\nthe tokenized terms. Think, \"find all documents that contain one or more\nof the terms tokenized from this text input\". This is the equivalent of\nthe `paradedb.match` function with the `conjunction_mode => false`.\n\n- `field ### 'TEXT'` - this is the \"phrase search\" operator. It\ntokenizes the right-hand-side using the index tokenizer defined by the\nleft-hand-side field and searches for documents that contain all the\ntokenized terms in the order written. Think, \"find all documents that\ncontain this tokenized phrase\". This is akin to the `paradedb.phrase()`\nfunction, except the `###` is transparently handling the tokenization.\n\nIf you wanted to find all documents that talk about bbq and chicken and\nmaybe hotdogs and happen on July 4th, you might write a query like this:\n\n```sql\nSELECT * \nFROM events \nWHERE (\n       description &&& 'bbq chicken' OR description ||| 'hot dog hotdogs'\n) AND description ### 'July 4th';\n```\n\n## Why\n\nThese operators are in furtherance of our\n[roadmap](https://github.com/orgs/paradedb/discussions/2041) --\nspecifically the \"Database UX improvements\" line item.\n\nOur overall goal with that roadmap item is to start moving users and ORM\ntools away from needing to use our \"builder functions\" in the common\ntext-search cases. What we're striving for is for users, and ORM tools,\nto write standard SQL WHERE clauses. In order to do that we need to make\ncommon things more accessible using custom operators.\n\n## How\n\nLargely just adding new `#[pg_operator]` functions and ensuring they all\nhave a SUPPORT function that can rewrite the expression to what we need.\n\n@rebasedming said he was going to tackle documentation, which can come\nin a separate PR.\n\n## Tests\n\nThere's a new regression test named `operators.sql` that validates\nEXPLAIN output and search results against our standard \"mock_items\"\ntable.",
          "timestamp": "2025-07-23T12:54:32-04:00",
          "tree_id": "f4a0dd94cd1e423e30c42103a8a23d86905d32e7",
          "url": "https://github.com/paradedb/paradedb/commit/880ee607618b7ffe12f2784d03be3110b4df6cdf"
        },
        "date": 1753290611811,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 1200.6728036966936,
            "unit": "median tps",
            "extra": "avg tps: 1198.0763767473193, max tps: 1204.6454556245822, count: 55136"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2629.8727002868527,
            "unit": "median tps",
            "extra": "avg tps: 2635.5311296086184, max tps: 2685.437524923138, count: 55136"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 1210.4335810192874,
            "unit": "median tps",
            "extra": "avg tps: 1206.0801244842166, max tps: 1212.945869241844, count: 55136"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 1002.3911259206418,
            "unit": "median tps",
            "extra": "avg tps: 995.3448686130946, max tps: 1012.7575803111628, count: 55136"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 172.10123218915396,
            "unit": "median tps",
            "extra": "avg tps: 184.24843853060412, max tps: 200.49867705043948, count: 110272"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 148.6824499344657,
            "unit": "median tps",
            "extra": "avg tps: 148.5645478048296, max tps: 158.62843959215155, count: 55136"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 56.09449838219721,
            "unit": "median tps",
            "extra": "avg tps: 59.27040622953118, max tps: 724.8220380691031, count: 55136"
          }
        ]
      }
    ],
    "pg_search single-server.toml Performance - Other Metrics": [
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
        "date": 1752440987843,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 9.160305,
            "unit": "median cpu",
            "extra": "avg cpu: 7.4726942799140685, max cpu: 23.506365, count: 55107"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 103.12890625,
            "unit": "median mem",
            "extra": "avg mem: 100.72339756349012, max mem: 105.32421875, count: 55107"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.619827,
            "unit": "median cpu",
            "extra": "avg cpu: 4.587054184366673, max cpu: 9.221902, count: 55107"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 86.71484375,
            "unit": "median mem",
            "extra": "avg mem: 84.90314311816103, max mem: 86.71484375, count: 55107"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 9.142857,
            "unit": "median cpu",
            "extra": "avg cpu: 7.3917408358814, max cpu: 23.210833, count: 55107"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 102.515625,
            "unit": "median mem",
            "extra": "avg mem: 101.6168992114205, max mem: 106.18359375, count: 55107"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.619827,
            "unit": "median cpu",
            "extra": "avg cpu: 4.5210036859302285, max cpu: 9.230769, count: 55107"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 99.08984375,
            "unit": "median mem",
            "extra": "avg mem: 98.27602266953382, max mem: 101.33984375, count: 55107"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.17782,
            "unit": "median cpu",
            "extra": "avg cpu: 7.6169880760372815, max cpu: 24.048098, count: 110214"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 112.9609375,
            "unit": "median mem",
            "extra": "avg mem: 112.4219112221451, max mem: 119.33203125, count: 110214"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 8535,
            "unit": "median block_count",
            "extra": "avg block_count: 8463.170559094126, max block_count: 8535.0, count: 55107"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 119,
            "unit": "median segment_count",
            "extra": "avg segment_count: 118.37008002613098, max segment_count: 270.0, count: 55107"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 6.235926841931639, max cpu: 19.238478, count: 55107"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 112.75390625,
            "unit": "median mem",
            "extra": "avg mem: 112.79075827764622, max mem: 118.375, count: 55107"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 18.461538,
            "unit": "median cpu",
            "extra": "avg cpu: 17.207486968736323, max cpu: 28.402367, count: 55107"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 97.140625,
            "unit": "median mem",
            "extra": "avg mem: 94.69297365522982, max mem: 99.703125, count: 55107"
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
        "date": 1752441000636,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 9.142857,
            "unit": "median cpu",
            "extra": "avg cpu: 7.372498752266969, max cpu: 23.143684, count: 55117"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 99.9921875,
            "unit": "median mem",
            "extra": "avg mem: 106.5030090109449, max mem: 139.4140625, count: 55117"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.619827,
            "unit": "median cpu",
            "extra": "avg cpu: 4.698644794060423, max cpu: 9.402546, count: 55117"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 86.29296875,
            "unit": "median mem",
            "extra": "avg mem: 87.64859908807627, max mem: 99.79296875, count: 55117"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 9.213051,
            "unit": "median cpu",
            "extra": "avg cpu: 8.294642715711806, max cpu: 24.0, count: 55117"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 104.26953125,
            "unit": "median mem",
            "extra": "avg mem: 110.44533921873015, max mem: 143.88671875, count: 55117"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.610951,
            "unit": "median cpu",
            "extra": "avg cpu: 4.561810237250614, max cpu: 9.257474, count: 55117"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 99.41796875,
            "unit": "median mem",
            "extra": "avg mem: 105.16435548469619, max mem: 132.04296875, count: 55117"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.221902,
            "unit": "median cpu",
            "extra": "avg cpu: 8.626394826952847, max cpu: 28.8, count: 110234"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 112.3984375,
            "unit": "median mem",
            "extra": "avg mem: 117.72671944596269, max mem: 162.28515625, count: 110234"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 8125,
            "unit": "median block_count",
            "extra": "avg block_count: 9345.314549050203, max block_count: 14412.0, count: 55117"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 120,
            "unit": "median segment_count",
            "extra": "avg segment_count: 119.52584502059256, max segment_count: 429.0, count: 55117"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 5.995498716428834, max cpu: 23.099133, count: 55117"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 117.640625,
            "unit": "median mem",
            "extra": "avg mem: 122.2538302043834, max mem: 152.56640625, count: 55117"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 18.426102,
            "unit": "median cpu",
            "extra": "avg cpu: 15.938639467034454, max cpu: 28.180038, count: 55117"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 94.78515625,
            "unit": "median mem",
            "extra": "avg mem: 93.84444029292233, max mem: 98.8125, count: 55117"
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
        "date": 1752441002300,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 9.186603,
            "unit": "median cpu",
            "extra": "avg cpu: 7.806153008008782, max cpu: 36.994217, count: 55130"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 103.1875,
            "unit": "median mem",
            "extra": "avg mem: 102.25979814756032, max mem: 106.32421875, count: 55130"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6153846,
            "unit": "median cpu",
            "extra": "avg cpu: 4.608452600735903, max cpu: 9.275363, count: 55130"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 87.04296875,
            "unit": "median mem",
            "extra": "avg mem: 85.86739042943951, max mem: 87.41796875, count: 55130"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 9.186603,
            "unit": "median cpu",
            "extra": "avg cpu: 7.76685727779277, max cpu: 32.36994, count: 55130"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 103.84765625,
            "unit": "median mem",
            "extra": "avg mem: 102.48738776528207, max mem: 106.01171875, count: 55130"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6153846,
            "unit": "median cpu",
            "extra": "avg cpu: 4.628196297955122, max cpu: 9.239654, count: 55130"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 101.66796875,
            "unit": "median mem",
            "extra": "avg mem: 101.02651956171776, max mem: 104.8125, count: 55130"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.204219,
            "unit": "median cpu",
            "extra": "avg cpu: 8.278521581700955, max cpu: 28.374382, count: 110260"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 114.189453125,
            "unit": "median mem",
            "extra": "avg mem: 113.88865171668103, max mem: 124.03515625, count: 110260"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 8535,
            "unit": "median block_count",
            "extra": "avg block_count: 8517.909341556322, max block_count: 8535.0, count: 55130"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 120,
            "unit": "median segment_count",
            "extra": "avg segment_count: 119.3121349537457, max segment_count: 355.0, count: 55130"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 6.113521859631946, max cpu: 18.497108, count: 55130"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 112.36328125,
            "unit": "median mem",
            "extra": "avg mem: 113.4691090944132, max mem: 122.44140625, count: 55130"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 18.497108,
            "unit": "median cpu",
            "extra": "avg cpu: 17.34055356965306, max cpu: 32.40116, count: 55130"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 95.3984375,
            "unit": "median mem",
            "extra": "avg mem: 94.00955872766643, max mem: 96.5625, count: 55130"
          }
        ]
      },
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
          "id": "4fd1b2b6b6664d03946be0f4836732f0f40df0cc",
          "message": "chore: Rename datasets and add string paging queries (#2834)\n\n## What\n\nAdd a high-cardinality paging/top-n query to the benchmarks, and rename\ndatasets to match their content. Additionally, improve the generation\nscript for the `docs` dataset to avoid joins and allow for deterministic\nrelative-position queries.\n\n## Why\n\nWe don't currently have a high-cardinality string paging/top-n query in\nthe benchmark. We have top-n on a string column, but only for\nlow-cardinality values (`top_n-string.sql`). The top-n case represented\nan important gap that a user encountered, which #2828 addresses.\n\nThe names of the `benchmark` datasets don't currently describe their\nshape / schema, and for the `join` dataset in particular, that would\ndiscourage using it for other types of queries. We rename it to `docs`\nhere, and then use the `pages` table as the dataset for top-n.\n\n## Tests\n\nTested locally that the new query demonstrates a speedup for #2828.",
          "timestamp": "2025-07-13T18:04:27Z",
          "url": "https://github.com/paradedb/paradedb/commit/4fd1b2b6b6664d03946be0f4836732f0f40df0cc"
        },
        "date": 1752441067389,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 9.195402,
            "unit": "median cpu",
            "extra": "avg cpu: 7.58274748325849, max cpu: 23.233301, count: 54617"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 104.80859375,
            "unit": "median mem",
            "extra": "avg mem: 100.76107005659868, max mem: 107.1640625, count: 54617"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.619827,
            "unit": "median cpu",
            "extra": "avg cpu: 4.642421419043236, max cpu: 9.402546, count: 54617"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 94.56640625,
            "unit": "median mem",
            "extra": "avg mem: 89.85905358565098, max mem: 94.94140625, count: 54617"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 9.195402,
            "unit": "median cpu",
            "extra": "avg cpu: 7.613237327175098, max cpu: 23.645319, count: 54617"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 105.96484375,
            "unit": "median mem",
            "extra": "avg mem: 101.25729168573888, max mem: 108.0625, count: 54617"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.619827,
            "unit": "median cpu",
            "extra": "avg cpu: 4.652455109774368, max cpu: 9.248554, count: 54617"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 105.44140625,
            "unit": "median mem",
            "extra": "avg mem: 100.85321727609993, max mem: 106.12109375, count: 54617"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.221902,
            "unit": "median cpu",
            "extra": "avg cpu: 8.387311715820049, max cpu: 28.20764, count: 109234"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 108.30859375,
            "unit": "median mem",
            "extra": "avg mem: 105.49320359560667, max mem: 114.00390625, count: 109234"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 9337,
            "unit": "median block_count",
            "extra": "avg block_count: 8813.086749546845, max block_count: 9337.0, count: 54617"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 119,
            "unit": "median segment_count",
            "extra": "avg segment_count: 118.4046542285369, max segment_count: 324.0, count: 54617"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 5.929190891395205, max cpu: 15.094339, count: 54617"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 111.78515625,
            "unit": "median mem",
            "extra": "avg mem: 109.0478350233215, max mem: 118.140625, count: 54617"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 18.497108,
            "unit": "median cpu",
            "extra": "avg cpu: 17.5414371612892, max cpu: 28.430405, count: 54617"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 105.03515625,
            "unit": "median mem",
            "extra": "avg mem: 98.37930111618178, max mem: 107.33984375, count: 54617"
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
        "date": 1752441071536,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 6.875928976852056, max cpu: 23.645319, count: 55224"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 96.421875,
            "unit": "median mem",
            "extra": "avg mem: 103.10111064709184, max mem: 121.37109375, count: 55224"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.755382149344175, max cpu: 9.4395275, count: 55224"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 85.1796875,
            "unit": "median mem",
            "extra": "avg mem: 91.70267979614842, max mem: 108.8046875, count: 55224"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 9.186603,
            "unit": "median cpu",
            "extra": "avg cpu: 7.530496356781523, max cpu: 23.59882, count: 55224"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 96.22265625,
            "unit": "median mem",
            "extra": "avg mem: 102.68009217290037, max mem: 119.4296875, count: 55224"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6153846,
            "unit": "median cpu",
            "extra": "avg cpu: 4.576214288189471, max cpu: 4.733728, count: 55224"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 96.0546875,
            "unit": "median mem",
            "extra": "avg mem: 102.43293832278539, max mem: 118.25, count: 55224"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.195402,
            "unit": "median cpu",
            "extra": "avg cpu: 7.7245861701493475, max cpu: 23.66864, count: 110448"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 108.46484375,
            "unit": "median mem",
            "extra": "avg mem: 112.28974625643968, max mem: 137.36328125, count: 110448"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 8120,
            "unit": "median block_count",
            "extra": "avg block_count: 9028.35490004346, max block_count: 10979.0, count: 55224"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 120,
            "unit": "median segment_count",
            "extra": "avg segment_count: 119.71336737650297, max segment_count: 432.0, count: 55224"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 6.2657325796473735, max cpu: 18.568666, count: 55224"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 109.5703125,
            "unit": "median mem",
            "extra": "avg mem: 112.68041543022508, max mem: 131.83984375, count: 55224"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 18.408438,
            "unit": "median cpu",
            "extra": "avg cpu: 16.639432010082693, max cpu: 28.486649, count: 55224"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 95.55078125,
            "unit": "median mem",
            "extra": "avg mem: 99.67783460599105, max mem: 118.5625, count: 55224"
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
        "date": 1752441102616,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.673807,
            "unit": "median cpu",
            "extra": "avg cpu: 7.07636040830243, max cpu: 27.612656, count: 54990"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 113.0390625,
            "unit": "median mem",
            "extra": "avg mem: 106.26956222154028, max mem: 114.40625, count: 54990"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6021094,
            "unit": "median cpu",
            "extra": "avg cpu: 4.660013624948625, max cpu: 9.402546, count: 54990"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 92.72265625,
            "unit": "median mem",
            "extra": "avg mem: 88.06516056896709, max mem: 92.72265625, count: 54990"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.669261,
            "unit": "median cpu",
            "extra": "avg cpu: 7.023318065170739, max cpu: 23.483368, count: 54990"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 114.0625,
            "unit": "median mem",
            "extra": "avg mem: 107.97806916598472, max mem: 116.82421875, count: 54990"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.597701,
            "unit": "median cpu",
            "extra": "avg cpu: 4.5864949920132885, max cpu: 4.6875, count: 54990"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 112.59765625,
            "unit": "median mem",
            "extra": "avg mem: 106.11153093176031, max mem: 112.59765625, count: 54990"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.160305,
            "unit": "median cpu",
            "extra": "avg cpu: 7.682333996918835, max cpu: 23.952095, count: 109980"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 118.6328125,
            "unit": "median mem",
            "extra": "avg mem: 114.58749623511093, max mem: 124.57421875, count: 109980"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 10646,
            "unit": "median block_count",
            "extra": "avg block_count: 9764.458865248227, max block_count: 10646.0, count: 54990"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 119,
            "unit": "median segment_count",
            "extra": "avg segment_count: 118.93229678123295, max segment_count: 382.0, count: 54990"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.610951,
            "unit": "median cpu",
            "extra": "avg cpu: 6.150257976547244, max cpu: 18.443804, count: 54990"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 130.265625,
            "unit": "median mem",
            "extra": "avg mem: 125.15317316216584, max mem: 136.1328125, count: 54990"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 18.390804,
            "unit": "median cpu",
            "extra": "avg cpu: 16.58427548365091, max cpu: 32.24568, count: 54990"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 113.07421875,
            "unit": "median mem",
            "extra": "avg mem: 104.92958691352973, max mem: 113.82421875, count: 54990"
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
          "id": "47bbe518381e1429f228328336dad78e99636ad9",
          "message": "chore: Upgrade to `0.16.0` (#2720)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-06-23T23:04:37Z",
          "url": "https://github.com/paradedb/paradedb/commit/47bbe518381e1429f228328336dad78e99636ad9"
        },
        "date": 1752441104551,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 13.753581,
            "unit": "median cpu",
            "extra": "avg cpu: 11.77143013102557, max cpu: 32.876713, count: 55093"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 105.15625,
            "unit": "median mem",
            "extra": "avg mem: 104.50623519548763, max mem: 109.7890625, count: 55093"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.610951,
            "unit": "median cpu",
            "extra": "avg cpu: 4.856029980569178, max cpu: 9.430255, count: 55093"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 87.875,
            "unit": "median mem",
            "extra": "avg mem: 86.45992458207031, max mem: 87.875, count: 55093"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 13.753581,
            "unit": "median cpu",
            "extra": "avg cpu: 11.864516733718906, max cpu: 32.876713, count: 55093"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 106.9453125,
            "unit": "median mem",
            "extra": "avg mem: 106.53049807541339, max mem: 113.203125, count: 55093"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.610951,
            "unit": "median cpu",
            "extra": "avg cpu: 5.369551076710137, max cpu: 13.93998, count: 55093"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 109.94140625,
            "unit": "median mem",
            "extra": "avg mem: 109.70710817957817, max mem: 114.59375, count: 55093"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.195402,
            "unit": "median cpu",
            "extra": "avg cpu: 8.233382989828433, max cpu: 23.483368, count: 110186"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 126.6328125,
            "unit": "median mem",
            "extra": "avg mem: 126.10799455721462, max mem: 128.328125, count: 110186"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 8279,
            "unit": "median block_count",
            "extra": "avg block_count: 8242.162289220047, max block_count: 8279.0, count: 55093"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 122,
            "unit": "median segment_count",
            "extra": "avg segment_count: 120.28219556023451, max segment_count: 339.0, count: 55093"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.619827,
            "unit": "median cpu",
            "extra": "avg cpu: 6.097631639339264, max cpu: 22.944551, count: 55093"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 128.43359375,
            "unit": "median mem",
            "extra": "avg mem: 126.46983658597735, max mem: 128.43359375, count: 55093"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 18.443804,
            "unit": "median cpu",
            "extra": "avg cpu: 16.77400349986527, max cpu: 28.263002, count: 55093"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 96.546875,
            "unit": "median mem",
            "extra": "avg mem: 95.33485474277585, max mem: 97.671875, count: 55093"
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
          "id": "b96e41e25c3fd2126f54aa5cb66d4deffb482047",
          "message": "perf: Lazily load fast fields dictionaries. (#2842)\n\n## What\n\nLazily load fast field dictionaries from buffers: see\nhttps://github.com/paradedb/tantivy/pull/55\n\n## Why\n\nA customer reported slower-than-expected paging on a string/uuid column.\n85% of the time for that query was being spent in _opening_ a fast\nfields string/bytes column, with a large fraction of that time spent\nfully consuming the column's `Dictionary`.\n\n## Tests\n\nSee the attached benchmark results:\n* [`docs`\ndataset](https://github.com/paradedb/paradedb/pull/2842#pullrequestreview-3014379545)\n    * No regressions.\n    * 2x faster for `top_n-score`\n    * 1.4x faster for `highlighting` \n* [`logs`\ndataset](https://github.com/paradedb/paradedb/pull/2842#pullrequestreview-3014350211)\n    * No regressions.\n    * 4.5x faster for `paging-string-max`\n    * 1.7x faster for `paging-string-median`\n    * 1.6x faster for `paging-string-min`\n\nThe `paging-string-*` benchmarks were added in #2834 to highlight this\nparticular issue.",
          "timestamp": "2025-07-14T08:28:09-07:00",
          "tree_id": "d144335dcb7c7f138a112c01e5b9ff5e0168fe37",
          "url": "https://github.com/paradedb/paradedb/commit/b96e41e25c3fd2126f54aa5cb66d4deffb482047"
        },
        "date": 1752507920354,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.678363,
            "unit": "median cpu",
            "extra": "avg cpu: 6.979654476088491, max cpu: 23.506365, count: 54984"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 96.609375,
            "unit": "median mem",
            "extra": "avg mem: 95.47897108420177, max mem: 101.33984375, count: 54984"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.619827,
            "unit": "median cpu",
            "extra": "avg cpu: 4.607765417650276, max cpu: 9.239654, count: 54984"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 85.93359375,
            "unit": "median mem",
            "extra": "avg mem: 84.5554340950322, max mem: 88.55859375, count: 54984"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.692082,
            "unit": "median cpu",
            "extra": "avg cpu: 6.9990201618881835, max cpu: 23.529411, count: 54984"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 96.8046875,
            "unit": "median mem",
            "extra": "avg mem: 95.5972280004183, max mem: 102.02734375, count: 54984"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6153846,
            "unit": "median cpu",
            "extra": "avg cpu: 4.378226419366776, max cpu: 4.7197638, count: 54984"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 96.80859375,
            "unit": "median mem",
            "extra": "avg mem: 95.96765280581624, max mem: 101.30859375, count: 54984"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.195402,
            "unit": "median cpu",
            "extra": "avg cpu: 7.966471162119436, max cpu: 27.988338, count: 109968"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 100.9765625,
            "unit": "median mem",
            "extra": "avg mem: 101.13206341844901, max mem: 112.375, count: 109968"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 8203,
            "unit": "median block_count",
            "extra": "avg block_count: 8138.0711297832095, max block_count: 8596.0, count: 54984"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 120,
            "unit": "median segment_count",
            "extra": "avg segment_count: 118.94698457733159, max segment_count: 303.0, count: 54984"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 6.44646407151505, max cpu: 23.121387, count: 54984"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 102.4140625,
            "unit": "median mem",
            "extra": "avg mem: 101.9646891595737, max mem: 110.18359375, count: 54984"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 18.426102,
            "unit": "median cpu",
            "extra": "avg cpu: 16.738145131213006, max cpu: 32.24568, count: 54984"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 95.70703125,
            "unit": "median mem",
            "extra": "avg mem: 93.75278319602066, max mem: 99.36328125, count: 54984"
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
          "id": "ee6395b4b4d4ca6f44e2c89b74afd2308d4415a8",
          "message": "fix: orphaned delete entries get GCed too early (#2845)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nWhen running a new stressgres suite (coming in a future PR), I hit a\nmysterious bug where it looked like vacuum could cause corruption of\nsome pages.\n\nTurns out it's caused by scenarios where:\n\n1. A `DeleteEntry` already exists for a `SegmentMetaEntry`, and a new\none is created\n2. A new, \"fake\" `SegmentMetaEntry` gets created for the purpose of\nstoring the old `DeleteEntry`, so its blocks can get garbage collected\n3. Because this \"fake\" entry is invisible to all readers besides the\ngarbage collector, it doesn't get pinned and can get garbage collected\ntoo early (i.e. while a reader is still pinning the old `DeleteEntry`)\n\nThe solution is to copy all of the contents of the old\n`SegmentMetaEntry` to the fake one, so that the \"pintest blockno\" of the\nfake entry is that same as that of the entry with the new `DeleteEntry`.\nThat way, the `DeleteEntry` doesn't get garbage collected until the pin\nis released.\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-07-14T15:46:29-04:00",
          "tree_id": "3dc55f49de121cf04534f48e3584a2a3ae333407",
          "url": "https://github.com/paradedb/paradedb/commit/ee6395b4b4d4ca6f44e2c89b74afd2308d4415a8"
        },
        "date": 1752523327858,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 7.470817,
            "unit": "median cpu",
            "extra": "avg cpu: 7.274012821663131, max cpu: 23.622047, count: 55164"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 111.72265625,
            "unit": "median mem",
            "extra": "avg mem: 106.07697643617215, max mem: 115.23046875, count: 55164"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6153846,
            "unit": "median cpu",
            "extra": "avg cpu: 4.666822819955045, max cpu: 9.266409, count: 55164"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 99.47265625,
            "unit": "median mem",
            "extra": "avg mem: 93.34382994627384, max mem: 100.97265625, count: 55164"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 9.116809,
            "unit": "median cpu",
            "extra": "avg cpu: 7.281864593742709, max cpu: 23.622047, count: 55164"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 110.62890625,
            "unit": "median mem",
            "extra": "avg mem: 104.44843805233032, max mem: 112.62890625, count: 55164"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6153846,
            "unit": "median cpu",
            "extra": "avg cpu: 4.578298620432221, max cpu: 9.195402, count: 55164"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 110.34765625,
            "unit": "median mem",
            "extra": "avg mem: 104.26415969597473, max mem: 110.34765625, count: 55164"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.186603,
            "unit": "median cpu",
            "extra": "avg cpu: 7.703995005384538, max cpu: 24.0, count: 110328"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 111.24609375,
            "unit": "median mem",
            "extra": "avg mem: 110.46807520104824, max mem: 124.49609375, count: 110328"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 9829,
            "unit": "median block_count",
            "extra": "avg block_count: 9131.244326009717, max block_count: 9829.0, count: 55164"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 120,
            "unit": "median segment_count",
            "extra": "avg segment_count: 119.26651439344501, max segment_count: 341.0, count: 55164"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 6.107539509745548, max cpu: 18.916256, count: 55164"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 115.09765625,
            "unit": "median mem",
            "extra": "avg mem: 110.78700110409416, max mem: 120.984375, count: 55164"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 18.390804,
            "unit": "median cpu",
            "extra": "avg cpu: 16.420937973394224, max cpu: 32.36994, count: 55164"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 109.36328125,
            "unit": "median mem",
            "extra": "avg mem: 102.61008927640762, max mem: 110.92578125, count: 55164"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "eb3bc1d570c85d60804f6aab2f2d2cf998bd7597",
          "message": "ci: benchmark workflow cleanups (#2851)\n\nThis is an attempt to cleanup the benchmark workflows a little bit.  \n\n- Centralizes checking out the latest benchmark code/suites/actions into\na composite action.\n- figures out the PR #/title being tested\n- Changes the slack notification messages to be reactive to the\nenvironment to hopefully avoid conflicts with -enterprise",
          "timestamp": "2025-07-15T12:15:54-04:00",
          "tree_id": "223c726790d68868f538b7f5aab9cf9904494f44",
          "url": "https://github.com/paradedb/paradedb/commit/eb3bc1d570c85d60804f6aab2f2d2cf998bd7597"
        },
        "date": 1752597090889,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 9.116809,
            "unit": "median cpu",
            "extra": "avg cpu: 7.3532065107184446, max cpu: 28.68526, count: 54712"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 99.41796875,
            "unit": "median mem",
            "extra": "avg mem: 97.64304648660257, max mem: 102.953125, count: 54712"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.622928866823426, max cpu: 9.248554, count: 54712"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 85.1953125,
            "unit": "median mem",
            "extra": "avg mem: 84.20635940812802, max mem: 86.6953125, count: 54712"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 9.142857,
            "unit": "median cpu",
            "extra": "avg cpu: 7.394481106352058, max cpu: 27.745665, count: 54712"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 96.01171875,
            "unit": "median mem",
            "extra": "avg mem: 94.80663184321995, max mem: 98.89453125, count: 54712"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.555122618497994, max cpu: 9.125476, count: 54712"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 95.28515625,
            "unit": "median mem",
            "extra": "avg mem: 94.39945711464121, max mem: 97.109375, count: 54712"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.195402,
            "unit": "median cpu",
            "extra": "avg cpu: 7.740311294842536, max cpu: 24.0, count: 109424"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 103.46484375,
            "unit": "median mem",
            "extra": "avg mem: 103.57474665635509, max mem: 115.60546875, count: 109424"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 8097,
            "unit": "median block_count",
            "extra": "avg block_count: 8040.143423746162, max block_count: 8218.0, count: 54712"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 120,
            "unit": "median segment_count",
            "extra": "avg segment_count: 119.07881269191402, max segment_count: 317.0, count: 54712"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 6.415463429926581, max cpu: 28.20764, count: 54712"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 107.84375,
            "unit": "median mem",
            "extra": "avg mem: 107.91947386142436, max mem: 114.921875, count: 54712"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 18.461538,
            "unit": "median cpu",
            "extra": "avg cpu: 17.05850979773692, max cpu: 33.07087, count: 54712"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 95.47265625,
            "unit": "median mem",
            "extra": "avg mem: 92.990129065036, max mem: 97.7109375, count: 54712"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "c7bdeebed6249725c87b87c276d2e09dfaffd152",
          "message": "ci: publish stressgres benchmark graphs (#2852)\n\nThis will publish every stressgres benchmark graph to the new\n`paradedb/benchmark-data` repo.\n\nIt also ensures that when one of our \"[benchmark]\" labels are applied,\nthat it does *not* pull the benchmarks from main, but instead from the\nPR branch itself.",
          "timestamp": "2025-07-15T18:13:26-04:00",
          "tree_id": "f18bf519521a03613c5b12c861092f24314476ad",
          "url": "https://github.com/paradedb/paradedb/commit/c7bdeebed6249725c87b87c276d2e09dfaffd152"
        },
        "date": 1752618540322,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 6.794347817478516, max cpu: 23.762377, count: 55161"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 116.05078125,
            "unit": "median mem",
            "extra": "avg mem: 112.0491904521537, max mem: 131.81640625, count: 55161"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6153846,
            "unit": "median cpu",
            "extra": "avg cpu: 4.676702435962727, max cpu: 9.4395275, count: 55161"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 106.57421875,
            "unit": "median mem",
            "extra": "avg mem: 102.07561119382353, max mem: 118.94921875, count: 55161"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 9.186603,
            "unit": "median cpu",
            "extra": "avg cpu: 7.707024462918296, max cpu: 23.904383, count: 55161"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 117.984375,
            "unit": "median mem",
            "extra": "avg mem: 114.09775362121789, max mem: 132.4765625, count: 55161"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.487988651532281, max cpu: 9.275363, count: 55161"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 116.69921875,
            "unit": "median mem",
            "extra": "avg mem: 112.57466637434057, max mem: 129.78125, count: 55161"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.213051,
            "unit": "median cpu",
            "extra": "avg cpu: 8.204901996500626, max cpu: 27.745665, count: 110322"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 129.17578125,
            "unit": "median mem",
            "extra": "avg mem: 125.50078612210167, max mem: 148.921875, count: 110322"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 10882,
            "unit": "median block_count",
            "extra": "avg block_count: 10389.186979931474, max block_count: 12517.0, count: 55161"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 119,
            "unit": "median segment_count",
            "extra": "avg segment_count: 119.76118997117528, max segment_count: 487.0, count: 55161"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 6.194028010823544, max cpu: 18.786694, count: 55161"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 130.37109375,
            "unit": "median mem",
            "extra": "avg mem: 127.40700276803358, max mem: 148.26171875, count: 55161"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 18.285713,
            "unit": "median cpu",
            "extra": "avg cpu: 16.006508718621102, max cpu: 32.463768, count: 55161"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 115.17578125,
            "unit": "median mem",
            "extra": "avg mem: 109.97686417373689, max mem: 130.921875, count: 55161"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Philippe Noël",
            "username": "philippemnoel",
            "email": "21990816+philippemnoel@users.noreply.github.com"
          },
          "committer": {
            "name": "Stu Hood",
            "username": "stuhood",
            "email": "stuhood@gmail.com"
          },
          "id": "8c164d666c4af2049b439690afa7823ab5be2c88",
          "message": "ci: Post \"One-branch Release Model\" Improvements (#2792)\n\n# Ticket(s) Closed\n\n- Closes #N/A\n\n## What\n- Don't make Docker Scout post on PRs that don't modify the Dockerfile,\nnot necessary.\n- Make sure the SchemaBot workflow errors when community contributors\nmake a PR, so we can catch needed changes to the SQL upgrade script.\n- Rename it to SchemaBot\n\n## Why\nQoL\n\n## How\n^\n\n## Tests\nCI",
          "timestamp": "2025-07-08T15:49:16Z",
          "url": "https://github.com/paradedb/paradedb/commit/8c164d666c4af2049b439690afa7823ab5be2c88"
        },
        "date": 1752668515825,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 6.970152331474589, max cpu: 23.552504, count: 55077"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 97.0234375,
            "unit": "median mem",
            "extra": "avg mem: 98.52426829711132, max mem: 125.77734375, count: 55077"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.619827,
            "unit": "median cpu",
            "extra": "avg cpu: 4.629864980548322, max cpu: 9.213051, count: 55077"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 84.09765625,
            "unit": "median mem",
            "extra": "avg mem: 83.45504730876773, max mem: 95.34765625, count: 55077"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 9.151573,
            "unit": "median cpu",
            "extra": "avg cpu: 7.3212605744362715, max cpu: 23.143684, count: 55077"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 97.91015625,
            "unit": "median mem",
            "extra": "avg mem: 99.62358968750567, max mem: 125.09765625, count: 55077"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.619827,
            "unit": "median cpu",
            "extra": "avg cpu: 4.564069200672729, max cpu: 9.257474, count: 55077"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 96.09765625,
            "unit": "median mem",
            "extra": "avg mem: 98.15665426357644, max mem: 122.89453125, count: 55077"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.186603,
            "unit": "median cpu",
            "extra": "avg cpu: 7.49939632128954, max cpu: 24.072216, count: 110154"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 100.7109375,
            "unit": "median mem",
            "extra": "avg mem: 102.49071970406204, max mem: 133.578125, count: 110154"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 8209,
            "unit": "median block_count",
            "extra": "avg block_count: 8547.47500771647, max block_count: 12193.0, count: 55077"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 120,
            "unit": "median segment_count",
            "extra": "avg segment_count: 119.26615465620858, max segment_count: 448.0, count: 55077"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 6.183837131784775, max cpu: 14.117648, count: 55077"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 106.09765625,
            "unit": "median mem",
            "extra": "avg mem: 107.73512083083683, max mem: 139.65234375, count: 55077"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 18.461538,
            "unit": "median cpu",
            "extra": "avg cpu: 17.356566977081453, max cpu: 32.214767, count: 55077"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 95.671875,
            "unit": "median mem",
            "extra": "avg mem: 96.78512823524339, max mem: 125.95703125, count: 55077"
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
          "id": "f7c13c858851200e8ab5395779f821ca484cda0d",
          "message": "feat: Add a custom scan for aggregates (#2763)\n\n## What\n\nAdd a new `CustomScan` (installed with `create_upper_paths_hook`) which\nreplaces simple aggregate plans on `bm25`-indexed tables with aggregate\nimplementations using [Tantivy\naggregates](https://docs.rs/tantivy/latest/tantivy/aggregation/index.html).\n\n## Why\n\nTantivy aggregates can be significantly faster (in benchmarks, we've\nmeasured between 4-10x for bucketing/faceting queries). They have been\nexposed via `paradedb.aggregate` for a while now, but that function\nrequires learning a new API, and does not feel \"Postgres native\".\n\n## How\n\n* Adjust `CustomPathBuilder` and `CustomPathMethods` to allow multiple\n`CustomScan` implementations.\n* Remove the `CustomScan::PrivateData: Default` bound, as it requires\nthe `PrivateData` to start in an illegal state.\n* Move `qual_inspect` to a reusable location.\n* Split out a module to be used by both the `aggregate` API method and\nby the aggregate custom scan.\n* Implement the \"ParadeDB Aggregate Scan\" `CustomScan` type\n    * Add the new `CustomScan` type, hidden behind a GUC\n    * Filter Paths to those which represent `count(*)` queries\n    * Extract `quals` during `CustomPath` generation\n* Replace `Aggrefs` in target lists with `FuncExprs` while producing a\n`CustomPlan`\n* Execute a `count(*)` aggregate by pushing down a `value_count`\naggregate on the `ctid`\n\n## Tests\n\nAdded tests to validate that:\n* the GUC properly controls usage\n* the scan does not trigger for unsupported aggregates, tables without a\n`bm25` index, or group-bys (for now)",
          "timestamp": "2025-07-16T09:12:24-07:00",
          "tree_id": "69b043a9363fcf6ce2de468c97d14e41f593f017",
          "url": "https://github.com/paradedb/paradedb/commit/f7c13c858851200e8ab5395779f821ca484cda0d"
        },
        "date": 1752683375722,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 6.824514352499456, max cpu: 19.257774, count: 54969"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 90.375,
            "unit": "median mem",
            "extra": "avg mem: 89.77890528354618, max mem: 95.48828125, count: 54969"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.610951,
            "unit": "median cpu",
            "extra": "avg cpu: 4.631264794249504, max cpu: 9.213051, count: 54969"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 83.6796875,
            "unit": "median mem",
            "extra": "avg mem: 82.53111640197201, max mem: 87.0546875, count: 54969"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 9.142857,
            "unit": "median cpu",
            "extra": "avg cpu: 7.305128389502062, max cpu: 23.575638, count: 54969"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 89.7734375,
            "unit": "median mem",
            "extra": "avg mem: 89.0264992399125, max mem: 95.6328125, count: 54969"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6153846,
            "unit": "median cpu",
            "extra": "avg cpu: 4.584491629761695, max cpu: 9.204219, count: 54969"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 91.1796875,
            "unit": "median mem",
            "extra": "avg mem: 90.5710431675126, max mem: 94.5546875, count: 54969"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.204219,
            "unit": "median cpu",
            "extra": "avg cpu: 8.132321590552975, max cpu: 23.645319, count: 109938"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 105.73046875,
            "unit": "median mem",
            "extra": "avg mem: 105.3538991508282, max mem: 117.3515625, count: 109938"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 7790,
            "unit": "median block_count",
            "extra": "avg block_count: 7734.265476905165, max block_count: 8482.0, count: 54969"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 119,
            "unit": "median segment_count",
            "extra": "avg segment_count: 117.9896669031636, max segment_count: 322.0, count: 54969"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 6.169691655339136, max cpu: 27.665707, count: 54969"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 104.4765625,
            "unit": "median mem",
            "extra": "avg mem: 104.99349363391184, max mem: 113.5625, count: 54969"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.872832,
            "unit": "median cpu",
            "extra": "avg cpu: 15.562644612493461, max cpu: 28.263002, count: 54969"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 91.76953125,
            "unit": "median mem",
            "extra": "avg mem: 90.14222327300388, max mem: 98.03515625, count: 54969"
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
          "id": "c779dbb72178c1aa0e14ad94c72eeb9937251acd",
          "message": "feat: a background merger for large layers (#2743)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nLaunches Postgres [dynamic background\nworkers](https://docs.rs/pgrx/latest/pgrx/bgworkers/struct.DynamicBackgroundWorker.html)\nto merge large layers in the background.\n\nThis is configured by a new index setting, `background_layer_sizes`. Any\nlayers specified here will be merged in the background.\n\n## Why\n\nWrite throughput, and create more balanced indexes with fewer segments.\n\n## How\n\n1. The default (foreground) `layer_sizes` is now `10kb`, `100kb`, `1mb`\n2. The default `background_layer_sizes` is `10mb`, `100mb`, `1gb`,\n`10gb`, `100gb`, `1tb`\n3. After `aminsert` merges in the foreground, it checks to see if there\nare enough candidates for a background merge. If so, it spawns a dynamic\nbackground worker process to do the merge.\n4. `amvacuumcleanup` can also spawn a background worker, allowing the\nuser to rebalance their index with a `VACUUM`.\n\nAdditionally, I've tweaked merge policy to make it smarter:\n\n1. Terminate a merge early if we predict that, after the merge, we will\nend up with fewer than `target_segment_count` segments.\n2. Set the max layer size as byte size of index divided by\n`target_segment_count`. Discard all layer sizes above that, which avoids\nthe problem of merging too many segments into one giant segment.\n\nThis has allowed us to delete some code:\n\n1. `force_merge` is deprecated\n2. No longer need to store the segments created by an index build and\navoid merging them, since we just do expensive merges in the background\n\n## Tests\n\nAdded a new stressgres `.toml` file that configures the layer\nsizes/threshold, which triggers background merging.\n\n---------\n\nSigned-off-by: Ming <ming.ying.nyc@gmail.com>\nCo-authored-by: Stu Hood <stuhood@paradedb.com>\nCo-authored-by: Philippe Noël <21990816+philippemnoel@users.noreply.github.com>\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>",
          "timestamp": "2025-07-16T12:44:23-04:00",
          "tree_id": "153c406e456a638a6b68ca9123210bbd498d66a5",
          "url": "https://github.com/paradedb/paradedb/commit/c779dbb72178c1aa0e14ad94c72eeb9937251acd"
        },
        "date": 1752685300450,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.6065254,
            "unit": "median cpu",
            "extra": "avg cpu: 4.74353993511877, max cpu: 9.571285, count: 55119"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 58.9453125,
            "unit": "median mem",
            "extra": "avg mem: 58.97769570157296, max mem: 83.3203125, count: 55119"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6065254,
            "unit": "median cpu",
            "extra": "avg cpu: 4.5965984828445965, max cpu: 9.320388, count: 55119"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 53.3203125,
            "unit": "median mem",
            "extra": "avg mem: 53.00785587206771, max mem: 76.9453125, count: 55119"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.6065254,
            "unit": "median cpu",
            "extra": "avg cpu: 4.771042490419082, max cpu: 9.448819, count: 55119"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 59.6953125,
            "unit": "median mem",
            "extra": "avg mem: 60.03921260137158, max mem: 83.6953125, count: 55119"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.597701,
            "unit": "median cpu",
            "extra": "avg cpu: 4.432769750103771, max cpu: 4.6966734, count: 55119"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 58.5703125,
            "unit": "median mem",
            "extra": "avg mem: 58.35947917800577, max mem: 81.4453125, count: 55119"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.151573,
            "unit": "median cpu",
            "extra": "avg cpu: 7.449142992295745, max cpu: 23.529411, count: 110238"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 75.01953125,
            "unit": "median mem",
            "extra": "avg mem: 75.00406418243936, max mem: 103.51953125, count: 110238"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 3662,
            "unit": "median block_count",
            "extra": "avg block_count: 3696.1490955931713, max block_count: 6682.0, count: 55119"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 8,
            "unit": "median segment_count",
            "extra": "avg segment_count: 8.907182641194506, max segment_count: 28.0, count: 55119"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 6.050266210385479, max cpu: 14.159292, count: 55119"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 78.0703125,
            "unit": "median mem",
            "extra": "avg mem: 78.37628351555271, max mem: 106.58984375, count: 55119"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.597701,
            "unit": "median cpu",
            "extra": "avg cpu: 4.171832329408024, max cpu: 4.6966734, count: 55119"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 58.73828125,
            "unit": "median mem",
            "extra": "avg mem: 57.87885926359331, max mem: 83.7265625, count: 55119"
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
          "id": "d197630155f55d13eea0a81841951645a367a5f7",
          "message": "test: Make GUCs a tested property, and add proptests for the aggregate custom scan (#2788)\n\n## What\n\nMove the GUCs that are in use into a test property, which reduces\nruntime while allowing for testing arbitrary combinations of GUCs.\n\nAdditionally, added proptests for the aggregate custom scan by:\n* Adding its GUC to the generated set\n* Expanding the `generated_single_relation` test to use `COUNT(*)` in\nits target list.\n\n## Why\n\nTo add additional test coverage for #2763.",
          "timestamp": "2025-07-16T10:57:16-07:00",
          "tree_id": "6a92b62b640f7a0b2dc7e6507966b75f3bd00594",
          "url": "https://github.com/paradedb/paradedb/commit/d197630155f55d13eea0a81841951645a367a5f7"
        },
        "date": 1752689574219,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.619827,
            "unit": "median cpu",
            "extra": "avg cpu: 4.68898471577226, max cpu: 9.448819, count: 55128"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 58.21875,
            "unit": "median mem",
            "extra": "avg mem: 58.395410453852854, max mem: 81.46875, count: 55128"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.619827,
            "unit": "median cpu",
            "extra": "avg cpu: 4.673888574374334, max cpu: 9.284333, count: 55128"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 54.09375,
            "unit": "median mem",
            "extra": "avg mem: 52.845797507618634, max mem: 76.96875, count: 55128"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.619827,
            "unit": "median cpu",
            "extra": "avg cpu: 4.730189762706926, max cpu: 9.458128, count: 55128"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 60.09375,
            "unit": "median mem",
            "extra": "avg mem: 59.739345817914675, max mem: 83.34375, count: 55128"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.610951,
            "unit": "median cpu",
            "extra": "avg cpu: 4.276940094603659, max cpu: 4.701273, count: 55128"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 58.21875,
            "unit": "median mem",
            "extra": "avg mem: 58.4714131203744, max mem: 82.59375, count: 55128"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.17782,
            "unit": "median cpu",
            "extra": "avg cpu: 7.423783469092818, max cpu: 23.575638, count: 110256"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 75.57421875,
            "unit": "median mem",
            "extra": "avg mem: 75.69435464396496, max mem: 105.7734375, count: 110256"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 3699,
            "unit": "median block_count",
            "extra": "avg block_count: 3698.9790124800465, max block_count: 6676.0, count: 55128"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 8,
            "unit": "median segment_count",
            "extra": "avg segment_count: 8.929944855608765, max segment_count: 27.0, count: 55128"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 6.1935063847833725, max cpu: 14.215202, count: 55128"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 78.203125,
            "unit": "median mem",
            "extra": "avg mem: 77.87373922677678, max mem: 105.58203125, count: 55128"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.184462204130538, max cpu: 4.7058825, count: 55128"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 59.15625,
            "unit": "median mem",
            "extra": "avg mem: 57.32826572363409, max mem: 81.98828125, count: 55128"
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
          "id": "58f6742b2cde46fe2c15077e59a0d5b091ba6000",
          "message": "chore: Upgrade to `0.17.0` (#2870)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-07-16T19:49:53-04:00",
          "tree_id": "b77118d365b31a4f3478b7c0571cf1f7e9f8a5b2",
          "url": "https://github.com/paradedb/paradedb/commit/58f6742b2cde46fe2c15077e59a0d5b091ba6000"
        },
        "date": 1752710831881,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.619827,
            "unit": "median cpu",
            "extra": "avg cpu: 4.717540500029253, max cpu: 9.599999, count: 55161"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 60.109375,
            "unit": "median mem",
            "extra": "avg mem: 59.832754629629626, max mem: 83.734375, count: 55161"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.619827,
            "unit": "median cpu",
            "extra": "avg cpu: 4.65095285858455, max cpu: 9.275363, count: 55161"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 54.109375,
            "unit": "median mem",
            "extra": "avg mem: 52.797050055745906, max mem: 76.984375, count: 55161"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.619827,
            "unit": "median cpu",
            "extra": "avg cpu: 4.74604281157357, max cpu: 9.4395275, count: 55161"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 60.109375,
            "unit": "median mem",
            "extra": "avg mem: 59.768524467014736, max mem: 82.984375, count: 55161"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6153846,
            "unit": "median cpu",
            "extra": "avg cpu: 4.437008036622552, max cpu: 4.7244096, count: 55161"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 60.109375,
            "unit": "median mem",
            "extra": "avg mem: 59.946429259349905, max mem: 82.984375, count: 55161"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.186603,
            "unit": "median cpu",
            "extra": "avg cpu: 7.476386611309005, max cpu: 23.414635, count: 110322"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 64.50390625,
            "unit": "median mem",
            "extra": "avg mem: 64.26616549877858, max mem: 92.4296875, count: 110322"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 3677,
            "unit": "median block_count",
            "extra": "avg block_count: 3699.369735864107, max block_count: 6657.0, count: 55161"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 8,
            "unit": "median segment_count",
            "extra": "avg segment_count: 8.916716520730226, max segment_count: 27.0, count: 55161"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 6.204782471533129, max cpu: 14.173229, count: 55161"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 69.2890625,
            "unit": "median mem",
            "extra": "avg mem: 69.83178941530248, max mem: 96.53515625, count: 55161"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.619827,
            "unit": "median cpu",
            "extra": "avg cpu: 4.4038150416563075, max cpu: 4.729064, count: 55161"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 58.46484375,
            "unit": "median mem",
            "extra": "avg mem: 57.81151148739599, max mem: 82.33984375, count: 55161"
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
          "id": "86bc2b092d2857a997cd05af4b554ac96ba225fd",
          "message": "fix: Fix `rescan` for top-n (#2868)\n\n## What\n\nComments in both `Normal` and `TopN` scans claimed that their search\nresults would be reset by `PdbScanState`, but this was incorrect: they\nwere local variables in the struct, and needed to be reset locally.\n\n## Why\n\nFixes issue detected by the prop tests and repro'd by @eeeebbbbrrrr in\n#2751.\n\n## Tests\n\nNew regression test from #2751: thanks @eeeebbbbrrrr!\n\n---------\n\nCo-authored-by: Eric B. Ridge <eebbrr@paradedb.com>",
          "timestamp": "2025-07-17T08:59:32-07:00",
          "tree_id": "ccc9162bb96008badcb3046612d16394604ed6f7",
          "url": "https://github.com/paradedb/paradedb/commit/86bc2b092d2857a997cd05af4b554ac96ba225fd"
        },
        "date": 1752768915241,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.7229969805054015, max cpu: 9.60961, count: 54877"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 58.625,
            "unit": "median mem",
            "extra": "avg mem: 58.90388049638282, max mem: 82.625, count: 54877"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.675521636550232, max cpu: 9.4395275, count: 54877"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 52.25,
            "unit": "median mem",
            "extra": "avg mem: 51.91981157862128, max mem: 75.125, count: 54877"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.696334845591201, max cpu: 9.495549, count: 54877"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 60.5,
            "unit": "median mem",
            "extra": "avg mem: 60.79483663465568, max mem: 85.25, count: 54877"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.4488339840171465, max cpu: 4.6966734, count: 54877"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 59.375,
            "unit": "median mem",
            "extra": "avg mem: 59.27695801519762, max mem: 83.375, count: 54877"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.17782,
            "unit": "median cpu",
            "extra": "avg cpu: 7.4059308116246605, max cpu: 23.856857, count: 109754"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 69.71484375,
            "unit": "median mem",
            "extra": "avg mem: 70.19956871588963, max mem: 103.953125, count: 109754"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 3727,
            "unit": "median block_count",
            "extra": "avg block_count: 3723.8177560726717, max block_count: 6696.0, count: 54877"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 8,
            "unit": "median segment_count",
            "extra": "avg segment_count: 8.874428266851321, max segment_count: 28.0, count: 54877"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 6.223512085067533, max cpu: 14.414414, count: 54877"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 65.47265625,
            "unit": "median mem",
            "extra": "avg mem: 65.66019041731964, max mem: 93.22265625, count: 54877"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 5.221618857648629, max cpu: 9.430255, count: 54877"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 58.84375,
            "unit": "median mem",
            "extra": "avg mem: 58.32079205256756, max mem: 83.08203125, count: 54877"
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
          "id": "5772be0d3dea751c3609729efcc66cc69d9d30ee",
          "message": "chore: Remove debug output in aggregate scan (#2876)\n\n## What\n\nRemove a stray `println`.\n\n## Why\n\nIt's not doing anyone any good.",
          "timestamp": "2025-07-17T09:46:08-07:00",
          "tree_id": "06a002186c2a701283d73854f64c4225a08aa1a8",
          "url": "https://github.com/paradedb/paradedb/commit/5772be0d3dea751c3609729efcc66cc69d9d30ee"
        },
        "date": 1752771709335,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.7811712952202425, max cpu: 9.4395275, count: 55255"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 58.93359375,
            "unit": "median mem",
            "extra": "avg mem: 59.28518000350647, max mem: 81.80859375, count: 55255"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.652742042975775, max cpu: 9.302325, count: 55255"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 52.55859375,
            "unit": "median mem",
            "extra": "avg mem: 52.954281569541216, max mem: 76.55859375, count: 55255"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.781764045611883, max cpu: 9.628887, count: 55255"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 59.68359375,
            "unit": "median mem",
            "extra": "avg mem: 59.78342528391096, max mem: 82.93359375, count: 55255"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.325312489519427, max cpu: 4.7151275, count: 55255"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 58.55859375,
            "unit": "median mem",
            "extra": "avg mem: 59.43515964335807, max mem: 83.68359375, count: 55255"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.195402,
            "unit": "median cpu",
            "extra": "avg cpu: 7.481974793784744, max cpu: 24.072216, count: 110510"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 70.02734375,
            "unit": "median mem",
            "extra": "avg mem: 69.51437864785993, max mem: 105.33984375, count: 110510"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 3656,
            "unit": "median block_count",
            "extra": "avg block_count: 3695.4555786806623, max block_count: 6623.0, count: 55255"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 8,
            "unit": "median segment_count",
            "extra": "avg segment_count: 8.944385123518234, max segment_count: 28.0, count: 55255"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 6.29554876550422, max cpu: 19.257774, count: 55255"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 66.875,
            "unit": "median mem",
            "extra": "avg mem: 66.69264496708443, max mem: 94.1640625, count: 55255"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.732411741734988, max cpu: 9.4395275, count: 55255"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 56.89453125,
            "unit": "median mem",
            "extra": "avg mem: 57.70776463951226, max mem: 82.6953125, count: 55255"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "16307a200509cd31b42c1338f3c5b6e097a77fbf",
          "message": "fix: issue #2844 - Use `paradedb.with_index()` when necessary (#2879)\n\n# Ticket(s) Closed\n\n- Closes #2844\n\n## What\n\nWhen the right-hand-side of the `@@@` operator is a complex expression\nthat can't be evaluated to a `T_Const` during planning, we need to\nrewrite it to be a call to our `paradedb.with_index(<indexrelid>,\n<rhs>)` so that it can be properly used in Filter contexts during\nexecution.\n\nThis seems to come up, at least, during RLS rule evaluation if those\nrules use our `@@@` operator.\n\n## Why\n\nSo long as the right-hand-side of `@@@` is a `SearchQueryInput`, we need\nto be able to execute it when Postgres wants us to, whether that's\nduring a scan or, as in this case, a filter being run as part of RLS\nevaluation.\n\n## How\n\nWrapping Postgres expression nodes!\n\n## Tests\n\nI used the re-create the OP from #2844 put together as a regression\ntest. I seem to be lacking the creative energy today to reduce the test\ndown to its simplest form that doesn't require a complicated RLS\nconfiguration.",
          "timestamp": "2025-07-18T13:21:25-04:00",
          "tree_id": "f2e8c7ca2ce45eefd03b85ff45d56f0481ebae1a",
          "url": "https://github.com/paradedb/paradedb/commit/16307a200509cd31b42c1338f3c5b6e097a77fbf"
        },
        "date": 1752860223415,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.6153846,
            "unit": "median cpu",
            "extra": "avg cpu: 4.771444477648214, max cpu: 13.93998, count: 55078"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 59.328125,
            "unit": "median mem",
            "extra": "avg mem: 58.848353982079956, max mem: 82.578125, count: 55078"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6153846,
            "unit": "median cpu",
            "extra": "avg cpu: 4.586453739221171, max cpu: 9.134158, count: 55078"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 52.203125,
            "unit": "median mem",
            "extra": "avg mem: 52.286323770380186, max mem: 75.828125, count: 55078"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.6153846,
            "unit": "median cpu",
            "extra": "avg cpu: 4.707664054390533, max cpu: 9.421001, count: 55078"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 59.703125,
            "unit": "median mem",
            "extra": "avg mem: 59.1321265409964, max mem: 82.953125, count: 55078"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.610951,
            "unit": "median cpu",
            "extra": "avg cpu: 4.171505248691127, max cpu: 4.7197638, count: 55078"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 58.953125,
            "unit": "median mem",
            "extra": "avg mem: 58.93119246795454, max mem: 82.203125, count: 55078"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.17782,
            "unit": "median cpu",
            "extra": "avg cpu: 7.577457112667741, max cpu: 23.575638, count: 110156"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 64.9921875,
            "unit": "median mem",
            "extra": "avg mem: 64.83417962367007, max mem: 92.30859375, count: 110156"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 3711,
            "unit": "median block_count",
            "extra": "avg block_count: 3690.3445114201677, max block_count: 6643.0, count: 55078"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 8,
            "unit": "median segment_count",
            "extra": "avg segment_count: 8.860888194923563, max segment_count: 28.0, count: 55078"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 6.064215335209124, max cpu: 14.131501, count: 55078"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 67.75,
            "unit": "median mem",
            "extra": "avg mem: 67.81306922115, max mem: 93.9609375, count: 55078"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6065254,
            "unit": "median cpu",
            "extra": "avg cpu: 4.500789835529026, max cpu: 4.7058825, count: 55078"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 58.83984375,
            "unit": "median mem",
            "extra": "avg mem: 57.77359707483024, max mem: 81.33203125, count: 55078"
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
          "id": "8b45b24d47a7cac36696bae2d8505c750a7ee801",
          "message": "test: added more join regression tests + improved when to use Join quals (#2808)\n\n# Ticket(s) Closed\n\n- Closes #2807\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-07-18T10:40:03-07:00",
          "tree_id": "fb63bd81696cd27d03b6d5724551c3ff8fd59613",
          "url": "https://github.com/paradedb/paradedb/commit/8b45b24d47a7cac36696bae2d8505c750a7ee801"
        },
        "date": 1752861342363,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.774436167341388, max cpu: 9.458128, count: 55215"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 58.94140625,
            "unit": "median mem",
            "extra": "avg mem: 58.6716353827085, max mem: 81.81640625, count: 55215"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.662948127028246, max cpu: 9.421001, count: 55215"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 52.94140625,
            "unit": "median mem",
            "extra": "avg mem: 53.31329447953455, max mem: 76.94140625, count: 55215"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.723780941904499, max cpu: 9.542743, count: 55215"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 60.06640625,
            "unit": "median mem",
            "extra": "avg mem: 59.63092633625374, max mem: 83.31640625, count: 55215"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6153846,
            "unit": "median cpu",
            "extra": "avg cpu: 4.2141208736682065, max cpu: 4.729064, count: 55215"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 61.19140625,
            "unit": "median mem",
            "extra": "avg mem: 59.66707296194422, max mem: 83.31640625, count: 55215"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.204219,
            "unit": "median cpu",
            "extra": "avg cpu: 7.481829664227786, max cpu: 23.59882, count: 110430"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 63.46484375,
            "unit": "median mem",
            "extra": "avg mem: 63.814107671432126, max mem: 93.14453125, count: 110430"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 3721,
            "unit": "median block_count",
            "extra": "avg block_count: 3701.3529475685955, max block_count: 6647.0, count: 55215"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 8,
            "unit": "median segment_count",
            "extra": "avg segment_count: 8.89269220320565, max segment_count: 28.0, count: 55215"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 6.3180191332501705, max cpu: 18.550726, count: 55215"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 77.203125,
            "unit": "median mem",
            "extra": "avg mem: 77.36818785090102, max mem: 103.84765625, count: 55215"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.8270527941345565, max cpu: 9.248554, count: 55215"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 59.5546875,
            "unit": "median mem",
            "extra": "avg mem: 58.685225369012045, max mem: 83.47265625, count: 55215"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "77c6cd1aeb5f5c03bba892a38cd792a113cfbd39",
          "message": "chore: move code around (#2889)\n\n## What\n\nThis relocates the `src/boostrap/create_bm25.rs` source to\n`src/api/admin.rs`, which I believe is a more sensibile location and\nname.\n\nAdditionally it moves the `#[pg_extern] schema()` function from\n`src/api/builder_fns.rs` to this new(ly renamed) `admin.rs`, as it's an\nadministrative function, not a query builder function.\n\n## Why\n\nTrying to better organize things\n\n## How\n\n## Tests\n\nNo functional changes here.",
          "timestamp": "2025-07-22T12:39:21-04:00",
          "tree_id": "1ab036752fc8adf7ad2757741d4e274ab0e73c64",
          "url": "https://github.com/paradedb/paradedb/commit/77c6cd1aeb5f5c03bba892a38cd792a113cfbd39"
        },
        "date": 1753203297751,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.610951,
            "unit": "median cpu",
            "extra": "avg cpu: 4.728058499733118, max cpu: 9.4395275, count: 55046"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 58.609375,
            "unit": "median mem",
            "extra": "avg mem: 59.06565276087272, max mem: 81.859375, count: 55046"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6065254,
            "unit": "median cpu",
            "extra": "avg cpu: 4.577862812576371, max cpu: 9.266409, count: 55046"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 54.859375,
            "unit": "median mem",
            "extra": "avg mem: 53.741005250154416, max mem: 76.984375, count: 55046"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.610951,
            "unit": "median cpu",
            "extra": "avg cpu: 4.737606933187179, max cpu: 9.402546, count: 55046"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 59.734375,
            "unit": "median mem",
            "extra": "avg mem: 59.31283012162555, max mem: 81.859375, count: 55046"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6021094,
            "unit": "median cpu",
            "extra": "avg cpu: 4.291801104196035, max cpu: 4.692082, count: 55046"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 59.359375,
            "unit": "median mem",
            "extra": "avg mem: 59.885491225520475, max mem: 82.984375, count: 55046"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.160305,
            "unit": "median cpu",
            "extra": "avg cpu: 7.3716793049992475, max cpu: 23.346306, count: 110092"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 64.05859375,
            "unit": "median mem",
            "extra": "avg mem: 63.92471190288577, max mem: 92.41015625, count: 110092"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 3695,
            "unit": "median block_count",
            "extra": "avg block_count: 3714.4300403299058, max block_count: 6672.0, count: 55046"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 8,
            "unit": "median segment_count",
            "extra": "avg segment_count: 8.936543981397376, max segment_count: 27.0, count: 55046"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 6.209776214579861, max cpu: 14.173229, count: 55046"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 68.296875,
            "unit": "median mem",
            "extra": "avg mem: 68.15582216180557, max mem: 96.20703125, count: 55046"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6153846,
            "unit": "median cpu",
            "extra": "avg cpu: 4.375605597686752, max cpu: 4.7058825, count: 55046"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 58.8046875,
            "unit": "median mem",
            "extra": "avg mem: 56.487118373156086, max mem: 80.828125, count: 55046"
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
          "id": "1e26a38533fa872a935955128b2ecd08b3febf66",
          "message": "perf: Skip computing scores if not requested for top-n field queries (#2892)\n\n## What\n\n`SearchIndexReader::{top_by_field_in_segments,\ntop_by_string_field_in_segments}` were not using\n`SearchIndexReader::need_scores`, and were instead universally enabling\nscores.\n\nUse the `enable_scoring` helper and `self.need_scores` to skip computing\nscores where possible.\n\n## Why\n\nComputing scores requires field norms, which ([if they have not been\ndisabled](https://docs.paradedb.com/documentation/indexing/record)) can\ntake time to load and use.",
          "timestamp": "2025-07-22T15:58:56-07:00",
          "tree_id": "2a41c9e06be04ef92fed2b265dc3f90340fb0e37",
          "url": "https://github.com/paradedb/paradedb/commit/1e26a38533fa872a935955128b2ecd08b3febf66"
        },
        "date": 1753226073332,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.7000672420128735, max cpu: 9.421001, count: 55159"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 60.0859375,
            "unit": "median mem",
            "extra": "avg mem: 59.99806496899871, max mem: 83.7109375, count: 55159"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.689766112496035, max cpu: 9.476802, count: 55159"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 52.5859375,
            "unit": "median mem",
            "extra": "avg mem: 53.02815879434906, max mem: 77.7109375, count: 55159"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.782111730917777, max cpu: 9.4395275, count: 55159"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 58.9609375,
            "unit": "median mem",
            "extra": "avg mem: 58.82779357176526, max mem: 83.3359375, count: 55159"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.4425078554270545, max cpu: 4.7197638, count: 55159"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 58.5859375,
            "unit": "median mem",
            "extra": "avg mem: 59.49116573791222, max mem: 83.7109375, count: 55159"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.186603,
            "unit": "median cpu",
            "extra": "avg cpu: 7.30794938479934, max cpu: 23.809525, count: 110318"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 72.85546875,
            "unit": "median mem",
            "extra": "avg mem: 73.30897889368688, max mem: 104.1328125, count: 110318"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 3714,
            "unit": "median block_count",
            "extra": "avg block_count: 3748.4774742109175, max block_count: 6738.0, count: 55159"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 8,
            "unit": "median segment_count",
            "extra": "avg segment_count: 8.89356224732138, max segment_count: 28.0, count: 55159"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 6.24182648101196, max cpu: 14.285715, count: 55159"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 78.265625,
            "unit": "median mem",
            "extra": "avg mem: 78.94608193755779, max mem: 106.18359375, count: 55159"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.619827,
            "unit": "median cpu",
            "extra": "avg cpu: 4.1252370982549325, max cpu: 4.6829267, count: 55159"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 59.4375,
            "unit": "median mem",
            "extra": "avg mem: 58.24563201664733, max mem: 83.02734375, count: 55159"
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
          "id": "cbd2e802901988c181281a9b7f772543e1df32d2",
          "message": "chore: add `background-merge.toml` to CI (#2895)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nThis new stressgres file was instrumental in catching the bug fixed in\n#2845 and was used to test background merging, but has not been added to\nCI.\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-07-23T10:26:09-04:00",
          "tree_id": "bb8e3ca51c2d0de5e56be65f478f11096b0d410e",
          "url": "https://github.com/paradedb/paradedb/commit/cbd2e802901988c181281a9b7f772543e1df32d2"
        },
        "date": 1753281704910,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.723666786486285, max cpu: 9.448819, count: 55205"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 59.734375,
            "unit": "median mem",
            "extra": "avg mem: 58.97496179014582, max mem: 81.859375, count: 55205"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.629144149309058, max cpu: 9.467456, count: 55205"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 52.609375,
            "unit": "median mem",
            "extra": "avg mem: 52.35062975500408, max mem: 75.484375, count: 55205"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.723927866162107, max cpu: 9.4395275, count: 55205"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 60.109375,
            "unit": "median mem",
            "extra": "avg mem: 60.04589371433747, max mem: 83.734375, count: 55205"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.447449667408074, max cpu: 4.655674, count: 55205"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 58.984375,
            "unit": "median mem",
            "extra": "avg mem: 59.27575032832171, max mem: 82.984375, count: 55205"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.17782,
            "unit": "median cpu",
            "extra": "avg cpu: 7.289432838702109, max cpu: 23.59882, count: 110410"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 64.091796875,
            "unit": "median mem",
            "extra": "avg mem: 64.30166853234535, max mem: 92.87890625, count: 110410"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 3725,
            "unit": "median block_count",
            "extra": "avg block_count: 3735.9671587718503, max block_count: 6715.0, count: 55205"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 8,
            "unit": "median segment_count",
            "extra": "avg segment_count: 8.93315822842134, max segment_count: 28.0, count: 55205"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 6.108100578514883, max cpu: 14.201183, count: 55205"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 77.05078125,
            "unit": "median mem",
            "extra": "avg mem: 77.60639979678018, max mem: 106.796875, count: 55205"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.523926904960996, max cpu: 4.6966734, count: 55205"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 59.953125,
            "unit": "median mem",
            "extra": "avg mem: 58.66057662915497, max mem: 83.8671875, count: 55205"
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
          "id": "53fa29a57e7daa2bdec9bf05f86d8db50088fd51",
          "message": "perf: Avoid a re-query when a top-n query is exhausted (#2888)\n\n## What\n\nSplit the iterators consumed by the `TopN` exec method and by the\n`Normal`/`FF` exec methods into two separate implementations.\n\nAfter splitting them, use the known exact size of the `TopN` iterator to\nexit early (as `exhausted: bool`) if we had fewer matches than were\nrequested. This avoids re-querying an iterator (and re-scanning its\ncolumns) that we know has no more results.\n\n## Why\n\nThe two types of consumers of these iterators have very different\nconsumption patterns and constraints:\n* TopN knows exactly how many results there will be, and already buffers\nthem all in memory. It would like to be able to know the precise count\nof results.\n* `Normal` and `FF` exec methods do not know the total number of results\nto expect, and in some cases would like to be able to consume in a\nsegment-aware fashion in order to be able to late-fetch fast field\ncolumns (see #2623).\n\n## Tests\n\nAdded a test to cement the change in query count.\n\nBenchmarks show a 1.5x speedup for `paging-string-max`, and no change\nfor other queries.",
          "timestamp": "2025-07-23T09:13:45-07:00",
          "tree_id": "c162e59fb9a6b4523c0ff39e10d9d54d92761460",
          "url": "https://github.com/paradedb/paradedb/commit/53fa29a57e7daa2bdec9bf05f86d8db50088fd51"
        },
        "date": 1753288161443,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.714484220361027, max cpu: 9.628887, count: 55213"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 58.58203125,
            "unit": "median mem",
            "extra": "avg mem: 58.5160194949559, max mem: 81.83203125, count: 55213"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.579826378348073, max cpu: 4.738401, count: 55213"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 52.20703125,
            "unit": "median mem",
            "extra": "avg mem: 52.535016379747525, max mem: 75.83203125, count: 55213"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.722597005653521, max cpu: 13.806328, count: 55213"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 59.33203125,
            "unit": "median mem",
            "extra": "avg mem: 59.588097175936824, max mem: 82.95703125, count: 55213"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6153846,
            "unit": "median cpu",
            "extra": "avg cpu: 4.589408137404227, max cpu: 4.7058825, count: 55213"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 58.95703125,
            "unit": "median mem",
            "extra": "avg mem: 58.784488240427976, max mem: 82.20703125, count: 55213"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.186603,
            "unit": "median cpu",
            "extra": "avg cpu: 7.436629582476354, max cpu: 28.944725, count: 110426"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 70,
            "unit": "median mem",
            "extra": "avg mem: 70.20937313930823, max mem: 103.37890625, count: 110426"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 3729,
            "unit": "median block_count",
            "extra": "avg block_count: 3725.3542825059317, max block_count: 6690.0, count: 55213"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 8,
            "unit": "median segment_count",
            "extra": "avg segment_count: 8.874685309619112, max segment_count: 27.0, count: 55213"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 6.202789853575317, max cpu: 14.385615, count: 55213"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 81.0078125,
            "unit": "median mem",
            "extra": "avg mem: 79.49597934929274, max mem: 107.27734375, count: 55213"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.5845275,
            "unit": "median cpu",
            "extra": "avg cpu: 4.046897783479429, max cpu: 4.701273, count: 55213"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 57.26171875,
            "unit": "median mem",
            "extra": "avg mem: 57.153220326847844, max mem: 82.2578125, count: 55213"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "880ee607618b7ffe12f2784d03be3110b4df6cdf",
          "message": "feat: text search operators (#2877)\n\n## What\n\nThis is the implementation of 4 new operators designed around common\n\"text search\" patterns: `===` (exact token), `&&&` (tokenized term\nconjunction) `|||` (tokenized term disjunction), and `###` (phrase).\n\nFirst, `@@@` still exists and still does what it's always done, which is\nrun the right-hand-side through tantivy's \"query parser\".\n\nThese new operators have specific meanings. The spelling of these\noperators is both to maintain some kind of consistency in that they're\nall 3 of the same symbol and also to try and confer their meaning.\n\nIn all cases, the left-hand-side of the operator is required to be a\nreference to a Postgres `TEXT`/`VARCHAR` field and the right-hand-side\nis text. The operator definition defines how the right-hand-side input\nis applied to the left-hand-side field. These are \"text search\"\noperators and would be nonsensical applied to non-text fields.\n\n- `field === 'TEXT'` - this is the exact term match operator. The\nright-hand-side value is used, unmodified, to perform the equivalent of\nthe `paradedb.term()` function.\n\n- `field === ARRAY['...', '...', '...']` - similar to the above, but\nrewrites to the `paradedb.term_set()` function.\n\n- `field &&& 'TEXT'` - this is the \"match conjunction\" operator. It\ntokenizes the right-hand-side using the index tokenizer defined by the\nleft-hand-side field and searches for documents that contain **all** of\nthe tokenized terms. Think, \"find all documents that contain all terms\ntokenized from this text input\". This is the equivalent of the\n`paradedb.match` function with the `conjunction_mode => true`.\n\n- `field ||| 'TEXT'` - this is the \"match disjunction\" operator. It\ntokenizes the right-hand-side using the index tokenizer defined by the\nleft-hand-side field and searches for documents that contain **any** of\nthe tokenized terms. Think, \"find all documents that contain one or more\nof the terms tokenized from this text input\". This is the equivalent of\nthe `paradedb.match` function with the `conjunction_mode => false`.\n\n- `field ### 'TEXT'` - this is the \"phrase search\" operator. It\ntokenizes the right-hand-side using the index tokenizer defined by the\nleft-hand-side field and searches for documents that contain all the\ntokenized terms in the order written. Think, \"find all documents that\ncontain this tokenized phrase\". This is akin to the `paradedb.phrase()`\nfunction, except the `###` is transparently handling the tokenization.\n\nIf you wanted to find all documents that talk about bbq and chicken and\nmaybe hotdogs and happen on July 4th, you might write a query like this:\n\n```sql\nSELECT * \nFROM events \nWHERE (\n       description &&& 'bbq chicken' OR description ||| 'hot dog hotdogs'\n) AND description ### 'July 4th';\n```\n\n## Why\n\nThese operators are in furtherance of our\n[roadmap](https://github.com/orgs/paradedb/discussions/2041) --\nspecifically the \"Database UX improvements\" line item.\n\nOur overall goal with that roadmap item is to start moving users and ORM\ntools away from needing to use our \"builder functions\" in the common\ntext-search cases. What we're striving for is for users, and ORM tools,\nto write standard SQL WHERE clauses. In order to do that we need to make\ncommon things more accessible using custom operators.\n\n## How\n\nLargely just adding new `#[pg_operator]` functions and ensuring they all\nhave a SUPPORT function that can rewrite the expression to what we need.\n\n@rebasedming said he was going to tackle documentation, which can come\nin a separate PR.\n\n## Tests\n\nThere's a new regression test named `operators.sql` that validates\nEXPLAIN output and search results against our standard \"mock_items\"\ntable.",
          "timestamp": "2025-07-23T12:54:32-04:00",
          "tree_id": "f4a0dd94cd1e423e30c42103a8a23d86905d32e7",
          "url": "https://github.com/paradedb/paradedb/commit/880ee607618b7ffe12f2784d03be3110b4df6cdf"
        },
        "date": 1753290614438,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.610951,
            "unit": "median cpu",
            "extra": "avg cpu: 4.7245441940530215, max cpu: 9.571285, count: 55136"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 60.09375,
            "unit": "median mem",
            "extra": "avg mem: 59.435088916497385, max mem: 84.46875, count: 55136"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.610951,
            "unit": "median cpu",
            "extra": "avg cpu: 4.6196473698442775, max cpu: 9.421001, count: 55136"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 53.71875,
            "unit": "median mem",
            "extra": "avg mem: 53.47295040898868, max mem: 77.34375, count: 55136"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.610951,
            "unit": "median cpu",
            "extra": "avg cpu: 4.718309111691525, max cpu: 9.421001, count: 55136"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 59.34375,
            "unit": "median mem",
            "extra": "avg mem: 59.45411233132617, max mem: 82.59375, count: 55136"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6065254,
            "unit": "median cpu",
            "extra": "avg cpu: 4.062548633201506, max cpu: 4.701273, count: 55136"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 59.71875,
            "unit": "median mem",
            "extra": "avg mem: 59.179602482951246, max mem: 82.59375, count: 55136"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.142857,
            "unit": "median cpu",
            "extra": "avg cpu: 7.289649350438846, max cpu: 23.928215, count: 110272"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 64.39453125,
            "unit": "median mem",
            "extra": "avg mem: 64.26194680823328, max mem: 93.17578125, count: 110272"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 3746,
            "unit": "median block_count",
            "extra": "avg block_count: 3756.412688624492, max block_count: 6761.0, count: 55136"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 8,
            "unit": "median segment_count",
            "extra": "avg segment_count: 8.873784822983168, max segment_count: 28.0, count: 55136"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 6.235979681372493, max cpu: 14.0625, count: 55136"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 71.109375,
            "unit": "median mem",
            "extra": "avg mem: 70.6088753831435, max mem: 96.94921875, count: 55136"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.58891,
            "unit": "median cpu",
            "extra": "avg cpu: 4.099237585153095, max cpu: 9.134158, count: 55136"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 57.67578125,
            "unit": "median mem",
            "extra": "avg mem: 56.864297841360454, max mem: 81.921875, count: 55136"
          }
        ]
      }
    ],
    "pg_search bulk-updates.toml Performance - TPS": [
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
        "date": 1752441623360,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 6.72887545032169,
            "unit": "median tps",
            "extra": "avg tps: 5.768584719539373, max tps: 8.716128800210523, count: 57645"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.713253241624696,
            "unit": "median tps",
            "extra": "avg tps: 5.115628346933139, max tps: 6.426405477077364, count: 57645"
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
        "date": 1752441637744,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 4.332930784886186,
            "unit": "median tps",
            "extra": "avg tps: 4.36622335309167, max tps: 10.15291804459109, count: 57697"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 6.450037872175752,
            "unit": "median tps",
            "extra": "avg tps: 5.7540834249475745, max tps: 7.855376728286136, count: 57697"
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
        "date": 1752441639624,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 4.301456465473804,
            "unit": "median tps",
            "extra": "avg tps: 4.35498004147254, max tps: 10.498134694926783, count: 57813"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 6.641036523304004,
            "unit": "median tps",
            "extra": "avg tps: 5.90960445933477, max tps: 8.050172021645913, count: 57813"
          }
        ]
      },
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
          "id": "4fd1b2b6b6664d03946be0f4836732f0f40df0cc",
          "message": "chore: Rename datasets and add string paging queries (#2834)\n\n## What\n\nAdd a high-cardinality paging/top-n query to the benchmarks, and rename\ndatasets to match their content. Additionally, improve the generation\nscript for the `docs` dataset to avoid joins and allow for deterministic\nrelative-position queries.\n\n## Why\n\nWe don't currently have a high-cardinality string paging/top-n query in\nthe benchmark. We have top-n on a string column, but only for\nlow-cardinality values (`top_n-string.sql`). The top-n case represented\nan important gap that a user encountered, which #2828 addresses.\n\nThe names of the `benchmark` datasets don't currently describe their\nshape / schema, and for the `join` dataset in particular, that would\ndiscourage using it for other types of queries. We rename it to `docs`\nhere, and then use the `pages` table as the dataset for top-n.\n\n## Tests\n\nTested locally that the new query demonstrates a speedup for #2828.",
          "timestamp": "2025-07-13T18:04:27Z",
          "url": "https://github.com/paradedb/paradedb/commit/4fd1b2b6b6664d03946be0f4836732f0f40df0cc"
        },
        "date": 1752441703127,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 6.689807276935049,
            "unit": "median tps",
            "extra": "avg tps: 5.7493096415121485, max tps: 8.652629976360576, count: 57145"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.6241845479414,
            "unit": "median tps",
            "extra": "avg tps: 5.028228812972219, max tps: 6.32167699286851, count: 57145"
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
        "date": 1752441706737,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 6.639582445520897,
            "unit": "median tps",
            "extra": "avg tps: 5.6956952292315375, max tps: 8.517918359131018, count: 57747"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.852217281970316,
            "unit": "median tps",
            "extra": "avg tps: 5.236515098604407, max tps: 6.6221895895401435, count: 57747"
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
        "date": 1752441740010,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 6.769696596844461,
            "unit": "median tps",
            "extra": "avg tps: 5.794739073987815, max tps: 8.691386703175874, count: 57113"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.883741315690291,
            "unit": "median tps",
            "extra": "avg tps: 5.26567750779199, max tps: 6.649636526050661, count: 57113"
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
          "id": "47bbe518381e1429f228328336dad78e99636ad9",
          "message": "chore: Upgrade to `0.16.0` (#2720)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-06-23T23:04:37Z",
          "url": "https://github.com/paradedb/paradedb/commit/47bbe518381e1429f228328336dad78e99636ad9"
        },
        "date": 1752441742481,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 4.240568452740235,
            "unit": "median tps",
            "extra": "avg tps: 4.293881237732563, max tps: 9.909646027642086, count: 57023"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 6.288452413373211,
            "unit": "median tps",
            "extra": "avg tps: 5.602531168572085, max tps: 7.634190946984871, count: 57023"
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
          "id": "b96e41e25c3fd2126f54aa5cb66d4deffb482047",
          "message": "perf: Lazily load fast fields dictionaries. (#2842)\n\n## What\n\nLazily load fast field dictionaries from buffers: see\nhttps://github.com/paradedb/tantivy/pull/55\n\n## Why\n\nA customer reported slower-than-expected paging on a string/uuid column.\n85% of the time for that query was being spent in _opening_ a fast\nfields string/bytes column, with a large fraction of that time spent\nfully consuming the column's `Dictionary`.\n\n## Tests\n\nSee the attached benchmark results:\n* [`docs`\ndataset](https://github.com/paradedb/paradedb/pull/2842#pullrequestreview-3014379545)\n    * No regressions.\n    * 2x faster for `top_n-score`\n    * 1.4x faster for `highlighting` \n* [`logs`\ndataset](https://github.com/paradedb/paradedb/pull/2842#pullrequestreview-3014350211)\n    * No regressions.\n    * 4.5x faster for `paging-string-max`\n    * 1.7x faster for `paging-string-median`\n    * 1.6x faster for `paging-string-min`\n\nThe `paging-string-*` benchmarks were added in #2834 to highlight this\nparticular issue.",
          "timestamp": "2025-07-14T08:28:09-07:00",
          "tree_id": "d144335dcb7c7f138a112c01e5b9ff5e0168fe37",
          "url": "https://github.com/paradedb/paradedb/commit/b96e41e25c3fd2126f54aa5cb66d4deffb482047"
        },
        "date": 1752508556118,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 6.789385451876705,
            "unit": "median tps",
            "extra": "avg tps: 5.804636214719281, max tps: 8.674379744397005, count: 57838"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.748672714614006,
            "unit": "median tps",
            "extra": "avg tps: 5.152934932786995, max tps: 6.5177701114906945, count: 57838"
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
          "id": "ee6395b4b4d4ca6f44e2c89b74afd2308d4415a8",
          "message": "fix: orphaned delete entries get GCed too early (#2845)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nWhen running a new stressgres suite (coming in a future PR), I hit a\nmysterious bug where it looked like vacuum could cause corruption of\nsome pages.\n\nTurns out it's caused by scenarios where:\n\n1. A `DeleteEntry` already exists for a `SegmentMetaEntry`, and a new\none is created\n2. A new, \"fake\" `SegmentMetaEntry` gets created for the purpose of\nstoring the old `DeleteEntry`, so its blocks can get garbage collected\n3. Because this \"fake\" entry is invisible to all readers besides the\ngarbage collector, it doesn't get pinned and can get garbage collected\ntoo early (i.e. while a reader is still pinning the old `DeleteEntry`)\n\nThe solution is to copy all of the contents of the old\n`SegmentMetaEntry` to the fake one, so that the \"pintest blockno\" of the\nfake entry is that same as that of the entry with the new `DeleteEntry`.\nThat way, the `DeleteEntry` doesn't get garbage collected until the pin\nis released.\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-07-14T15:46:29-04:00",
          "tree_id": "3dc55f49de121cf04534f48e3584a2a3ae333407",
          "url": "https://github.com/paradedb/paradedb/commit/ee6395b4b4d4ca6f44e2c89b74afd2308d4415a8"
        },
        "date": 1752523963812,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 6.480494609292578,
            "unit": "median tps",
            "extra": "avg tps: 5.585624422620473, max tps: 8.383146404457035, count: 57841"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.912292079074403,
            "unit": "median tps",
            "extra": "avg tps: 5.283329559252446, max tps: 6.710982312410053, count: 57841"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "eb3bc1d570c85d60804f6aab2f2d2cf998bd7597",
          "message": "ci: benchmark workflow cleanups (#2851)\n\nThis is an attempt to cleanup the benchmark workflows a little bit.  \n\n- Centralizes checking out the latest benchmark code/suites/actions into\na composite action.\n- figures out the PR #/title being tested\n- Changes the slack notification messages to be reactive to the\nenvironment to hopefully avoid conflicts with -enterprise",
          "timestamp": "2025-07-15T12:15:54-04:00",
          "tree_id": "223c726790d68868f538b7f5aab9cf9904494f44",
          "url": "https://github.com/paradedb/paradedb/commit/eb3bc1d570c85d60804f6aab2f2d2cf998bd7597"
        },
        "date": 1752597726806,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 6.612745000134143,
            "unit": "median tps",
            "extra": "avg tps: 5.689107998613614, max tps: 8.618832301511047, count: 57799"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.8638299940017795,
            "unit": "median tps",
            "extra": "avg tps: 5.243316530524578, max tps: 6.600835639002686, count: 57799"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "c7bdeebed6249725c87b87c276d2e09dfaffd152",
          "message": "ci: publish stressgres benchmark graphs (#2852)\n\nThis will publish every stressgres benchmark graph to the new\n`paradedb/benchmark-data` repo.\n\nIt also ensures that when one of our \"[benchmark]\" labels are applied,\nthat it does *not* pull the benchmarks from main, but instead from the\nPR branch itself.",
          "timestamp": "2025-07-15T18:13:26-04:00",
          "tree_id": "f18bf519521a03613c5b12c861092f24314476ad",
          "url": "https://github.com/paradedb/paradedb/commit/c7bdeebed6249725c87b87c276d2e09dfaffd152"
        },
        "date": 1752619178568,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 6.566766925788112,
            "unit": "median tps",
            "extra": "avg tps: 5.655814390670281, max tps: 8.593382946119812, count: 57657"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.934081329120296,
            "unit": "median tps",
            "extra": "avg tps: 5.301774457563865, max tps: 6.681558501984027, count: 57657"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Philippe Noël",
            "username": "philippemnoel",
            "email": "21990816+philippemnoel@users.noreply.github.com"
          },
          "committer": {
            "name": "Stu Hood",
            "username": "stuhood",
            "email": "stuhood@gmail.com"
          },
          "id": "8c164d666c4af2049b439690afa7823ab5be2c88",
          "message": "ci: Post \"One-branch Release Model\" Improvements (#2792)\n\n# Ticket(s) Closed\n\n- Closes #N/A\n\n## What\n- Don't make Docker Scout post on PRs that don't modify the Dockerfile,\nnot necessary.\n- Make sure the SchemaBot workflow errors when community contributors\nmake a PR, so we can catch needed changes to the SQL upgrade script.\n- Rename it to SchemaBot\n\n## Why\nQoL\n\n## How\n^\n\n## Tests\nCI",
          "timestamp": "2025-07-08T15:49:16Z",
          "url": "https://github.com/paradedb/paradedb/commit/8c164d666c4af2049b439690afa7823ab5be2c88"
        },
        "date": 1752669154339,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 6.878177037662716,
            "unit": "median tps",
            "extra": "avg tps: 5.888555221224742, max tps: 8.814803211894557, count: 57286"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.611810368976334,
            "unit": "median tps",
            "extra": "avg tps: 4.998988687018463, max tps: 6.381433860339025, count: 57286"
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
          "id": "f7c13c858851200e8ab5395779f821ca484cda0d",
          "message": "feat: Add a custom scan for aggregates (#2763)\n\n## What\n\nAdd a new `CustomScan` (installed with `create_upper_paths_hook`) which\nreplaces simple aggregate plans on `bm25`-indexed tables with aggregate\nimplementations using [Tantivy\naggregates](https://docs.rs/tantivy/latest/tantivy/aggregation/index.html).\n\n## Why\n\nTantivy aggregates can be significantly faster (in benchmarks, we've\nmeasured between 4-10x for bucketing/faceting queries). They have been\nexposed via `paradedb.aggregate` for a while now, but that function\nrequires learning a new API, and does not feel \"Postgres native\".\n\n## How\n\n* Adjust `CustomPathBuilder` and `CustomPathMethods` to allow multiple\n`CustomScan` implementations.\n* Remove the `CustomScan::PrivateData: Default` bound, as it requires\nthe `PrivateData` to start in an illegal state.\n* Move `qual_inspect` to a reusable location.\n* Split out a module to be used by both the `aggregate` API method and\nby the aggregate custom scan.\n* Implement the \"ParadeDB Aggregate Scan\" `CustomScan` type\n    * Add the new `CustomScan` type, hidden behind a GUC\n    * Filter Paths to those which represent `count(*)` queries\n    * Extract `quals` during `CustomPath` generation\n* Replace `Aggrefs` in target lists with `FuncExprs` while producing a\n`CustomPlan`\n* Execute a `count(*)` aggregate by pushing down a `value_count`\naggregate on the `ctid`\n\n## Tests\n\nAdded tests to validate that:\n* the GUC properly controls usage\n* the scan does not trigger for unsupported aggregates, tables without a\n`bm25` index, or group-bys (for now)",
          "timestamp": "2025-07-16T09:12:24-07:00",
          "tree_id": "69b043a9363fcf6ce2de468c97d14e41f593f017",
          "url": "https://github.com/paradedb/paradedb/commit/f7c13c858851200e8ab5395779f821ca484cda0d"
        },
        "date": 1752684017007,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 6.849339035773006,
            "unit": "median tps",
            "extra": "avg tps: 5.874582869349273, max tps: 8.81310725908372, count: 57682"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.736725883429835,
            "unit": "median tps",
            "extra": "avg tps: 5.1368885469373, max tps: 6.485192803846836, count: 57682"
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
          "id": "c779dbb72178c1aa0e14ad94c72eeb9937251acd",
          "message": "feat: a background merger for large layers (#2743)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nLaunches Postgres [dynamic background\nworkers](https://docs.rs/pgrx/latest/pgrx/bgworkers/struct.DynamicBackgroundWorker.html)\nto merge large layers in the background.\n\nThis is configured by a new index setting, `background_layer_sizes`. Any\nlayers specified here will be merged in the background.\n\n## Why\n\nWrite throughput, and create more balanced indexes with fewer segments.\n\n## How\n\n1. The default (foreground) `layer_sizes` is now `10kb`, `100kb`, `1mb`\n2. The default `background_layer_sizes` is `10mb`, `100mb`, `1gb`,\n`10gb`, `100gb`, `1tb`\n3. After `aminsert` merges in the foreground, it checks to see if there\nare enough candidates for a background merge. If so, it spawns a dynamic\nbackground worker process to do the merge.\n4. `amvacuumcleanup` can also spawn a background worker, allowing the\nuser to rebalance their index with a `VACUUM`.\n\nAdditionally, I've tweaked merge policy to make it smarter:\n\n1. Terminate a merge early if we predict that, after the merge, we will\nend up with fewer than `target_segment_count` segments.\n2. Set the max layer size as byte size of index divided by\n`target_segment_count`. Discard all layer sizes above that, which avoids\nthe problem of merging too many segments into one giant segment.\n\nThis has allowed us to delete some code:\n\n1. `force_merge` is deprecated\n2. No longer need to store the segments created by an index build and\navoid merging them, since we just do expensive merges in the background\n\n## Tests\n\nAdded a new stressgres `.toml` file that configures the layer\nsizes/threshold, which triggers background merging.\n\n---------\n\nSigned-off-by: Ming <ming.ying.nyc@gmail.com>\nCo-authored-by: Stu Hood <stuhood@paradedb.com>\nCo-authored-by: Philippe Noël <21990816+philippemnoel@users.noreply.github.com>\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>",
          "timestamp": "2025-07-16T12:44:23-04:00",
          "tree_id": "153c406e456a638a6b68ca9123210bbd498d66a5",
          "url": "https://github.com/paradedb/paradedb/commit/c779dbb72178c1aa0e14ad94c72eeb9937251acd"
        },
        "date": 1752685942981,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 6.577212358905091,
            "unit": "median tps",
            "extra": "avg tps: 5.6823475038272395, max tps: 8.535648948910694, count: 57100"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.709153143729733,
            "unit": "median tps",
            "extra": "avg tps: 5.097887215175038, max tps: 6.450583641664189, count: 57100"
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
          "id": "d197630155f55d13eea0a81841951645a367a5f7",
          "message": "test: Make GUCs a tested property, and add proptests for the aggregate custom scan (#2788)\n\n## What\n\nMove the GUCs that are in use into a test property, which reduces\nruntime while allowing for testing arbitrary combinations of GUCs.\n\nAdditionally, added proptests for the aggregate custom scan by:\n* Adding its GUC to the generated set\n* Expanding the `generated_single_relation` test to use `COUNT(*)` in\nits target list.\n\n## Why\n\nTo add additional test coverage for #2763.",
          "timestamp": "2025-07-16T10:57:16-07:00",
          "tree_id": "6a92b62b640f7a0b2dc7e6507966b75f3bd00594",
          "url": "https://github.com/paradedb/paradedb/commit/d197630155f55d13eea0a81841951645a367a5f7"
        },
        "date": 1752690218199,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 7.028125822196618,
            "unit": "median tps",
            "extra": "avg tps: 6.01131681076183, max tps: 9.024771931488527, count: 57673"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.686592793039652,
            "unit": "median tps",
            "extra": "avg tps: 5.102593450694813, max tps: 6.425915842587539, count: 57673"
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
          "id": "58f6742b2cde46fe2c15077e59a0d5b091ba6000",
          "message": "chore: Upgrade to `0.17.0` (#2870)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-07-16T19:49:53-04:00",
          "tree_id": "b77118d365b31a4f3478b7c0571cf1f7e9f8a5b2",
          "url": "https://github.com/paradedb/paradedb/commit/58f6742b2cde46fe2c15077e59a0d5b091ba6000"
        },
        "date": 1752711473893,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 6.65753785428296,
            "unit": "median tps",
            "extra": "avg tps: 5.725175095076279, max tps: 8.611581137764894, count: 57393"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.814467328276157,
            "unit": "median tps",
            "extra": "avg tps: 5.198471742557322, max tps: 6.559758100059857, count: 57393"
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
          "id": "86bc2b092d2857a997cd05af4b554ac96ba225fd",
          "message": "fix: Fix `rescan` for top-n (#2868)\n\n## What\n\nComments in both `Normal` and `TopN` scans claimed that their search\nresults would be reset by `PdbScanState`, but this was incorrect: they\nwere local variables in the struct, and needed to be reset locally.\n\n## Why\n\nFixes issue detected by the prop tests and repro'd by @eeeebbbbrrrr in\n#2751.\n\n## Tests\n\nNew regression test from #2751: thanks @eeeebbbbrrrr!\n\n---------\n\nCo-authored-by: Eric B. Ridge <eebbrr@paradedb.com>",
          "timestamp": "2025-07-17T08:59:32-07:00",
          "tree_id": "ccc9162bb96008badcb3046612d16394604ed6f7",
          "url": "https://github.com/paradedb/paradedb/commit/86bc2b092d2857a997cd05af4b554ac96ba225fd"
        },
        "date": 1752769562347,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 6.626537041753655,
            "unit": "median tps",
            "extra": "avg tps: 5.714168301975587, max tps: 8.606851060256512, count: 57850"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.716161967135559,
            "unit": "median tps",
            "extra": "avg tps: 5.069803781438279, max tps: 6.494772456869284, count: 57850"
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
          "id": "5772be0d3dea751c3609729efcc66cc69d9d30ee",
          "message": "chore: Remove debug output in aggregate scan (#2876)\n\n## What\n\nRemove a stray `println`.\n\n## Why\n\nIt's not doing anyone any good.",
          "timestamp": "2025-07-17T09:46:08-07:00",
          "tree_id": "06a002186c2a701283d73854f64c4225a08aa1a8",
          "url": "https://github.com/paradedb/paradedb/commit/5772be0d3dea751c3609729efcc66cc69d9d30ee"
        },
        "date": 1752772357236,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 6.783551187720308,
            "unit": "median tps",
            "extra": "avg tps: 5.814183703078753, max tps: 8.68759643312559, count: 57123"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.754033669342192,
            "unit": "median tps",
            "extra": "avg tps: 5.149000423436561, max tps: 6.496827530426565, count: 57123"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "16307a200509cd31b42c1338f3c5b6e097a77fbf",
          "message": "fix: issue #2844 - Use `paradedb.with_index()` when necessary (#2879)\n\n# Ticket(s) Closed\n\n- Closes #2844\n\n## What\n\nWhen the right-hand-side of the `@@@` operator is a complex expression\nthat can't be evaluated to a `T_Const` during planning, we need to\nrewrite it to be a call to our `paradedb.with_index(<indexrelid>,\n<rhs>)` so that it can be properly used in Filter contexts during\nexecution.\n\nThis seems to come up, at least, during RLS rule evaluation if those\nrules use our `@@@` operator.\n\n## Why\n\nSo long as the right-hand-side of `@@@` is a `SearchQueryInput`, we need\nto be able to execute it when Postgres wants us to, whether that's\nduring a scan or, as in this case, a filter being run as part of RLS\nevaluation.\n\n## How\n\nWrapping Postgres expression nodes!\n\n## Tests\n\nI used the re-create the OP from #2844 put together as a regression\ntest. I seem to be lacking the creative energy today to reduce the test\ndown to its simplest form that doesn't require a complicated RLS\nconfiguration.",
          "timestamp": "2025-07-18T13:21:25-04:00",
          "tree_id": "f2e8c7ca2ce45eefd03b85ff45d56f0481ebae1a",
          "url": "https://github.com/paradedb/paradedb/commit/16307a200509cd31b42c1338f3c5b6e097a77fbf"
        },
        "date": 1752860866119,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 6.765328112560796,
            "unit": "median tps",
            "extra": "avg tps: 5.8071809928134535, max tps: 8.736528945223796, count: 57123"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.7337192378670405,
            "unit": "median tps",
            "extra": "avg tps: 5.128294259143312, max tps: 6.472686673491412, count: 57123"
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
          "id": "8b45b24d47a7cac36696bae2d8505c750a7ee801",
          "message": "test: added more join regression tests + improved when to use Join quals (#2808)\n\n# Ticket(s) Closed\n\n- Closes #2807\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-07-18T10:40:03-07:00",
          "tree_id": "fb63bd81696cd27d03b6d5724551c3ff8fd59613",
          "url": "https://github.com/paradedb/paradedb/commit/8b45b24d47a7cac36696bae2d8505c750a7ee801"
        },
        "date": 1752861985821,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 6.922205591763881,
            "unit": "median tps",
            "extra": "avg tps: 5.92464675899667, max tps: 8.912683925923632, count: 57679"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.7212590228355555,
            "unit": "median tps",
            "extra": "avg tps: 5.129602402622485, max tps: 6.460135104348981, count: 57679"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "77c6cd1aeb5f5c03bba892a38cd792a113cfbd39",
          "message": "chore: move code around (#2889)\n\n## What\n\nThis relocates the `src/boostrap/create_bm25.rs` source to\n`src/api/admin.rs`, which I believe is a more sensibile location and\nname.\n\nAdditionally it moves the `#[pg_extern] schema()` function from\n`src/api/builder_fns.rs` to this new(ly renamed) `admin.rs`, as it's an\nadministrative function, not a query builder function.\n\n## Why\n\nTrying to better organize things\n\n## How\n\n## Tests\n\nNo functional changes here.",
          "timestamp": "2025-07-22T12:39:21-04:00",
          "tree_id": "1ab036752fc8adf7ad2757741d4e274ab0e73c64",
          "url": "https://github.com/paradedb/paradedb/commit/77c6cd1aeb5f5c03bba892a38cd792a113cfbd39"
        },
        "date": 1753203942901,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 6.6959259666478435,
            "unit": "median tps",
            "extra": "avg tps: 5.721619230040979, max tps: 8.544108203805319, count: 57774"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.727776863497542,
            "unit": "median tps",
            "extra": "avg tps: 5.123212965196485, max tps: 6.478526154598859, count: 57774"
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
          "id": "1e26a38533fa872a935955128b2ecd08b3febf66",
          "message": "perf: Skip computing scores if not requested for top-n field queries (#2892)\n\n## What\n\n`SearchIndexReader::{top_by_field_in_segments,\ntop_by_string_field_in_segments}` were not using\n`SearchIndexReader::need_scores`, and were instead universally enabling\nscores.\n\nUse the `enable_scoring` helper and `self.need_scores` to skip computing\nscores where possible.\n\n## Why\n\nComputing scores requires field norms, which ([if they have not been\ndisabled](https://docs.paradedb.com/documentation/indexing/record)) can\ntake time to load and use.",
          "timestamp": "2025-07-22T15:58:56-07:00",
          "tree_id": "2a41c9e06be04ef92fed2b265dc3f90340fb0e37",
          "url": "https://github.com/paradedb/paradedb/commit/1e26a38533fa872a935955128b2ecd08b3febf66"
        },
        "date": 1753226720391,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 6.906109918599827,
            "unit": "median tps",
            "extra": "avg tps: 5.926036621842602, max tps: 8.926910903061751, count: 57332"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.727330454936477,
            "unit": "median tps",
            "extra": "avg tps: 5.125484989699577, max tps: 6.457580610699357, count: 57332"
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
          "id": "cbd2e802901988c181281a9b7f772543e1df32d2",
          "message": "chore: add `background-merge.toml` to CI (#2895)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nThis new stressgres file was instrumental in catching the bug fixed in\n#2845 and was used to test background merging, but has not been added to\nCI.\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-07-23T10:26:09-04:00",
          "tree_id": "bb8e3ca51c2d0de5e56be65f478f11096b0d410e",
          "url": "https://github.com/paradedb/paradedb/commit/cbd2e802901988c181281a9b7f772543e1df32d2"
        },
        "date": 1753282353770,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 6.737143974914526,
            "unit": "median tps",
            "extra": "avg tps: 5.76481239017628, max tps: 8.6319738902815, count: 57272"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.80857458580108,
            "unit": "median tps",
            "extra": "avg tps: 5.199256166374755, max tps: 6.5951442859068425, count: 57272"
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
          "id": "53fa29a57e7daa2bdec9bf05f86d8db50088fd51",
          "message": "perf: Avoid a re-query when a top-n query is exhausted (#2888)\n\n## What\n\nSplit the iterators consumed by the `TopN` exec method and by the\n`Normal`/`FF` exec methods into two separate implementations.\n\nAfter splitting them, use the known exact size of the `TopN` iterator to\nexit early (as `exhausted: bool`) if we had fewer matches than were\nrequested. This avoids re-querying an iterator (and re-scanning its\ncolumns) that we know has no more results.\n\n## Why\n\nThe two types of consumers of these iterators have very different\nconsumption patterns and constraints:\n* TopN knows exactly how many results there will be, and already buffers\nthem all in memory. It would like to be able to know the precise count\nof results.\n* `Normal` and `FF` exec methods do not know the total number of results\nto expect, and in some cases would like to be able to consume in a\nsegment-aware fashion in order to be able to late-fetch fast field\ncolumns (see #2623).\n\n## Tests\n\nAdded a test to cement the change in query count.\n\nBenchmarks show a 1.5x speedup for `paging-string-max`, and no change\nfor other queries.",
          "timestamp": "2025-07-23T09:13:45-07:00",
          "tree_id": "c162e59fb9a6b4523c0ff39e10d9d54d92761460",
          "url": "https://github.com/paradedb/paradedb/commit/53fa29a57e7daa2bdec9bf05f86d8db50088fd51"
        },
        "date": 1753288811450,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 6.769897954069627,
            "unit": "median tps",
            "extra": "avg tps: 5.814791630660318, max tps: 8.74801254383186, count: 57319"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.898272564884106,
            "unit": "median tps",
            "extra": "avg tps: 5.273281654326356, max tps: 6.670788174563515, count: 57319"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "880ee607618b7ffe12f2784d03be3110b4df6cdf",
          "message": "feat: text search operators (#2877)\n\n## What\n\nThis is the implementation of 4 new operators designed around common\n\"text search\" patterns: `===` (exact token), `&&&` (tokenized term\nconjunction) `|||` (tokenized term disjunction), and `###` (phrase).\n\nFirst, `@@@` still exists and still does what it's always done, which is\nrun the right-hand-side through tantivy's \"query parser\".\n\nThese new operators have specific meanings. The spelling of these\noperators is both to maintain some kind of consistency in that they're\nall 3 of the same symbol and also to try and confer their meaning.\n\nIn all cases, the left-hand-side of the operator is required to be a\nreference to a Postgres `TEXT`/`VARCHAR` field and the right-hand-side\nis text. The operator definition defines how the right-hand-side input\nis applied to the left-hand-side field. These are \"text search\"\noperators and would be nonsensical applied to non-text fields.\n\n- `field === 'TEXT'` - this is the exact term match operator. The\nright-hand-side value is used, unmodified, to perform the equivalent of\nthe `paradedb.term()` function.\n\n- `field === ARRAY['...', '...', '...']` - similar to the above, but\nrewrites to the `paradedb.term_set()` function.\n\n- `field &&& 'TEXT'` - this is the \"match conjunction\" operator. It\ntokenizes the right-hand-side using the index tokenizer defined by the\nleft-hand-side field and searches for documents that contain **all** of\nthe tokenized terms. Think, \"find all documents that contain all terms\ntokenized from this text input\". This is the equivalent of the\n`paradedb.match` function with the `conjunction_mode => true`.\n\n- `field ||| 'TEXT'` - this is the \"match disjunction\" operator. It\ntokenizes the right-hand-side using the index tokenizer defined by the\nleft-hand-side field and searches for documents that contain **any** of\nthe tokenized terms. Think, \"find all documents that contain one or more\nof the terms tokenized from this text input\". This is the equivalent of\nthe `paradedb.match` function with the `conjunction_mode => false`.\n\n- `field ### 'TEXT'` - this is the \"phrase search\" operator. It\ntokenizes the right-hand-side using the index tokenizer defined by the\nleft-hand-side field and searches for documents that contain all the\ntokenized terms in the order written. Think, \"find all documents that\ncontain this tokenized phrase\". This is akin to the `paradedb.phrase()`\nfunction, except the `###` is transparently handling the tokenization.\n\nIf you wanted to find all documents that talk about bbq and chicken and\nmaybe hotdogs and happen on July 4th, you might write a query like this:\n\n```sql\nSELECT * \nFROM events \nWHERE (\n       description &&& 'bbq chicken' OR description ||| 'hot dog hotdogs'\n) AND description ### 'July 4th';\n```\n\n## Why\n\nThese operators are in furtherance of our\n[roadmap](https://github.com/orgs/paradedb/discussions/2041) --\nspecifically the \"Database UX improvements\" line item.\n\nOur overall goal with that roadmap item is to start moving users and ORM\ntools away from needing to use our \"builder functions\" in the common\ntext-search cases. What we're striving for is for users, and ORM tools,\nto write standard SQL WHERE clauses. In order to do that we need to make\ncommon things more accessible using custom operators.\n\n## How\n\nLargely just adding new `#[pg_operator]` functions and ensuring they all\nhave a SUPPORT function that can rewrite the expression to what we need.\n\n@rebasedming said he was going to tackle documentation, which can come\nin a separate PR.\n\n## Tests\n\nThere's a new regression test named `operators.sql` that validates\nEXPLAIN output and search results against our standard \"mock_items\"\ntable.",
          "timestamp": "2025-07-23T12:54:32-04:00",
          "tree_id": "f4a0dd94cd1e423e30c42103a8a23d86905d32e7",
          "url": "https://github.com/paradedb/paradedb/commit/880ee607618b7ffe12f2784d03be3110b4df6cdf"
        },
        "date": 1753291265435,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 6.9609205337527476,
            "unit": "median tps",
            "extra": "avg tps: 5.960229748975193, max tps: 8.983528726104336, count: 57349"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.777475086587337,
            "unit": "median tps",
            "extra": "avg tps: 5.1730079015473995, max tps: 6.5489164486734595, count: 57349"
          }
        ]
      }
    ],
    "pg_search bulk-updates.toml Performance - Other Metrics": [
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
        "date": 1752441625288,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.188406,
            "unit": "median cpu",
            "extra": "avg cpu: 21.423161388961258, max cpu: 42.60355, count: 57645"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 228.234375,
            "unit": "median mem",
            "extra": "avg mem: 226.6519946981525, max mem: 237.48828125, count: 57645"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.233301,
            "unit": "median cpu",
            "extra": "avg cpu: 22.30869997676292, max cpu: 33.168808, count: 57645"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 162.671875,
            "unit": "median mem",
            "extra": "avg mem: 161.87639674950125, max mem: 164.92578125, count: 57645"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 21660,
            "unit": "median block_count",
            "extra": "avg block_count: 20000.568444791395, max block_count: 21686.0, count: 57645"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 67,
            "unit": "median segment_count",
            "extra": "avg segment_count: 68.59701621996705, max segment_count: 97.0, count: 57645"
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
        "date": 1752441641568,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.27837,
            "unit": "median cpu",
            "extra": "avg cpu: 23.329479612772385, max cpu: 42.772278, count: 57813"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 183.796875,
            "unit": "median mem",
            "extra": "avg mem: 183.66963231669348, max mem: 184.171875, count: 57813"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.233301,
            "unit": "median cpu",
            "extra": "avg cpu: 21.67115836056725, max cpu: 33.168808, count: 57813"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 160.22265625,
            "unit": "median mem",
            "extra": "avg mem: 160.03793832053344, max mem: 161.61328125, count: 57813"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 19014,
            "unit": "median block_count",
            "extra": "avg block_count: 18652.29761472333, max block_count: 19014.0, count: 57813"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 63,
            "unit": "median segment_count",
            "extra": "avg segment_count: 62.98384446404788, max segment_count: 79.0, count: 57813"
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
        "date": 1752441639950,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.27837,
            "unit": "median cpu",
            "extra": "avg cpu: 23.20506223583672, max cpu: 42.72997, count: 57697"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 182.96484375,
            "unit": "median mem",
            "extra": "avg mem: 182.63918184654315, max mem: 182.96484375, count: 57697"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.233301,
            "unit": "median cpu",
            "extra": "avg cpu: 21.805026193691344, max cpu: 33.103447, count: 57697"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 160.5,
            "unit": "median mem",
            "extra": "avg mem: 160.398920356561, max mem: 162.40234375, count: 57697"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 18980,
            "unit": "median block_count",
            "extra": "avg block_count: 18621.226510910445, max block_count: 18980.0, count: 57697"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 63,
            "unit": "median segment_count",
            "extra": "avg segment_count: 63.121340797615126, max segment_count: 79.0, count: 57697"
          }
        ]
      },
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
          "id": "4fd1b2b6b6664d03946be0f4836732f0f40df0cc",
          "message": "chore: Rename datasets and add string paging queries (#2834)\n\n## What\n\nAdd a high-cardinality paging/top-n query to the benchmarks, and rename\ndatasets to match their content. Additionally, improve the generation\nscript for the `docs` dataset to avoid joins and allow for deterministic\nrelative-position queries.\n\n## Why\n\nWe don't currently have a high-cardinality string paging/top-n query in\nthe benchmark. We have top-n on a string column, but only for\nlow-cardinality values (`top_n-string.sql`). The top-n case represented\nan important gap that a user encountered, which #2828 addresses.\n\nThe names of the `benchmark` datasets don't currently describe their\nshape / schema, and for the `join` dataset in particular, that would\ndiscourage using it for other types of queries. We rename it to `docs`\nhere, and then use the `pages` table as the dataset for top-n.\n\n## Tests\n\nTested locally that the new query demonstrates a speedup for #2828.",
          "timestamp": "2025-07-13T18:04:27Z",
          "url": "https://github.com/paradedb/paradedb/commit/4fd1b2b6b6664d03946be0f4836732f0f40df0cc"
        },
        "date": 1752441705380,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.143684,
            "unit": "median cpu",
            "extra": "avg cpu: 21.482318445584657, max cpu: 42.772278, count: 57145"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 226.8125,
            "unit": "median mem",
            "extra": "avg mem: 226.4875818914603, max mem: 237.80078125, count: 57145"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.210833,
            "unit": "median cpu",
            "extra": "avg cpu: 22.32175746906075, max cpu: 33.168808, count: 57145"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 159.84765625,
            "unit": "median mem",
            "extra": "avg mem: 159.68846314747572, max mem: 162.4140625, count: 57145"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 22262,
            "unit": "median block_count",
            "extra": "avg block_count: 20660.539487269227, max block_count: 23588.0, count: 57145"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 67,
            "unit": "median segment_count",
            "extra": "avg segment_count: 68.46527255227929, max segment_count: 97.0, count: 57145"
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
        "date": 1752441709902,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.166023,
            "unit": "median cpu",
            "extra": "avg cpu: 21.491786556043493, max cpu: 42.687748, count: 57747"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 226.60546875,
            "unit": "median mem",
            "extra": "avg mem: 225.51253217158467, max mem: 231.29296875, count: 57747"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.210833,
            "unit": "median cpu",
            "extra": "avg cpu: 22.25276688639335, max cpu: 33.103447, count: 57747"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 158.75390625,
            "unit": "median mem",
            "extra": "avg mem: 158.4819971032045, max mem: 159.90625, count: 57747"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 22034,
            "unit": "median block_count",
            "extra": "avg block_count: 20634.828995445652, max block_count: 23225.0, count: 57747"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 67,
            "unit": "median segment_count",
            "extra": "avg segment_count: 68.29629244809254, max segment_count: 96.0, count: 57747"
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
        "date": 1752441742084,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.188406,
            "unit": "median cpu",
            "extra": "avg cpu: 21.471455355693664, max cpu: 42.687748, count: 57113"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 228.1875,
            "unit": "median mem",
            "extra": "avg mem: 226.68652687435434, max mem: 236.2421875, count: 57113"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.233301,
            "unit": "median cpu",
            "extra": "avg cpu: 22.200655045323412, max cpu: 33.168808, count: 57113"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 159.1171875,
            "unit": "median mem",
            "extra": "avg mem: 158.95574610585592, max mem: 160.58203125, count: 57113"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 21594,
            "unit": "median block_count",
            "extra": "avg block_count: 19933.594435592597, max block_count: 21594.0, count: 57113"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 67,
            "unit": "median segment_count",
            "extra": "avg segment_count: 68.5560380298706, max segment_count: 97.0, count: 57113"
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
          "id": "47bbe518381e1429f228328336dad78e99636ad9",
          "message": "chore: Upgrade to `0.16.0` (#2720)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-06-23T23:04:37Z",
          "url": "https://github.com/paradedb/paradedb/commit/47bbe518381e1429f228328336dad78e99636ad9"
        },
        "date": 1752441746373,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.27837,
            "unit": "median cpu",
            "extra": "avg cpu: 23.358179463007666, max cpu: 42.942345, count: 57023"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 190.34375,
            "unit": "median mem",
            "extra": "avg mem: 189.60837184118688, max mem: 190.34375, count: 57023"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.210833,
            "unit": "median cpu",
            "extra": "avg cpu: 21.925685365751935, max cpu: 33.103447, count: 57023"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 168.97265625,
            "unit": "median mem",
            "extra": "avg mem: 168.9195519927266, max mem: 173.2578125, count: 57023"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 19078,
            "unit": "median block_count",
            "extra": "avg block_count: 18709.642354839274, max block_count: 19078.0, count: 57023"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 63,
            "unit": "median segment_count",
            "extra": "avg segment_count: 62.82834996404959, max segment_count: 79.0, count: 57023"
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
          "id": "b96e41e25c3fd2126f54aa5cb66d4deffb482047",
          "message": "perf: Lazily load fast fields dictionaries. (#2842)\n\n## What\n\nLazily load fast field dictionaries from buffers: see\nhttps://github.com/paradedb/tantivy/pull/55\n\n## Why\n\nA customer reported slower-than-expected paging on a string/uuid column.\n85% of the time for that query was being spent in _opening_ a fast\nfields string/bytes column, with a large fraction of that time spent\nfully consuming the column's `Dictionary`.\n\n## Tests\n\nSee the attached benchmark results:\n* [`docs`\ndataset](https://github.com/paradedb/paradedb/pull/2842#pullrequestreview-3014379545)\n    * No regressions.\n    * 2x faster for `top_n-score`\n    * 1.4x faster for `highlighting` \n* [`logs`\ndataset](https://github.com/paradedb/paradedb/pull/2842#pullrequestreview-3014350211)\n    * No regressions.\n    * 4.5x faster for `paging-string-max`\n    * 1.7x faster for `paging-string-median`\n    * 1.6x faster for `paging-string-min`\n\nThe `paging-string-*` benchmarks were added in #2834 to highlight this\nparticular issue.",
          "timestamp": "2025-07-14T08:28:09-07:00",
          "tree_id": "d144335dcb7c7f138a112c01e5b9ff5e0168fe37",
          "url": "https://github.com/paradedb/paradedb/commit/b96e41e25c3fd2126f54aa5cb66d4deffb482047"
        },
        "date": 1752508557951,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.166023,
            "unit": "median cpu",
            "extra": "avg cpu: 21.401998710133135, max cpu: 42.772278, count: 57838"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 225.66015625,
            "unit": "median mem",
            "extra": "avg mem: 224.94116655842612, max mem: 237.3515625, count: 57838"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.233301,
            "unit": "median cpu",
            "extra": "avg cpu: 22.245485424478094, max cpu: 33.168808, count: 57838"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 159.8046875,
            "unit": "median mem",
            "extra": "avg mem: 159.71143228176112, max mem: 161.21875, count: 57838"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 22400,
            "unit": "median block_count",
            "extra": "avg block_count: 20731.73832082714, max block_count: 23460.0, count: 57838"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 67,
            "unit": "median segment_count",
            "extra": "avg segment_count: 68.57635118780041, max segment_count: 96.0, count: 57838"
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
          "id": "ee6395b4b4d4ca6f44e2c89b74afd2308d4415a8",
          "message": "fix: orphaned delete entries get GCed too early (#2845)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nWhen running a new stressgres suite (coming in a future PR), I hit a\nmysterious bug where it looked like vacuum could cause corruption of\nsome pages.\n\nTurns out it's caused by scenarios where:\n\n1. A `DeleteEntry` already exists for a `SegmentMetaEntry`, and a new\none is created\n2. A new, \"fake\" `SegmentMetaEntry` gets created for the purpose of\nstoring the old `DeleteEntry`, so its blocks can get garbage collected\n3. Because this \"fake\" entry is invisible to all readers besides the\ngarbage collector, it doesn't get pinned and can get garbage collected\ntoo early (i.e. while a reader is still pinning the old `DeleteEntry`)\n\nThe solution is to copy all of the contents of the old\n`SegmentMetaEntry` to the fake one, so that the \"pintest blockno\" of the\nfake entry is that same as that of the entry with the new `DeleteEntry`.\nThat way, the `DeleteEntry` doesn't get garbage collected until the pin\nis released.\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-07-14T15:46:29-04:00",
          "tree_id": "3dc55f49de121cf04534f48e3584a2a3ae333407",
          "url": "https://github.com/paradedb/paradedb/commit/ee6395b4b4d4ca6f44e2c89b74afd2308d4415a8"
        },
        "date": 1752523965734,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.188406,
            "unit": "median cpu",
            "extra": "avg cpu: 21.666938319763997, max cpu: 42.687748, count: 57841"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 226.375,
            "unit": "median mem",
            "extra": "avg mem: 225.44932119949516, max mem: 236.4765625, count: 57841"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.210833,
            "unit": "median cpu",
            "extra": "avg cpu: 22.09615333318723, max cpu: 33.07087, count: 57841"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 160.66796875,
            "unit": "median mem",
            "extra": "avg mem: 160.57799472638354, max mem: 163.609375, count: 57841"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 21877,
            "unit": "median block_count",
            "extra": "avg block_count: 20393.73112498055, max block_count: 23329.0, count: 57841"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 66,
            "unit": "median segment_count",
            "extra": "avg segment_count: 67.84244739890389, max segment_count: 95.0, count: 57841"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "eb3bc1d570c85d60804f6aab2f2d2cf998bd7597",
          "message": "ci: benchmark workflow cleanups (#2851)\n\nThis is an attempt to cleanup the benchmark workflows a little bit.  \n\n- Centralizes checking out the latest benchmark code/suites/actions into\na composite action.\n- figures out the PR #/title being tested\n- Changes the slack notification messages to be reactive to the\nenvironment to hopefully avoid conflicts with -enterprise",
          "timestamp": "2025-07-15T12:15:54-04:00",
          "tree_id": "223c726790d68868f538b7f5aab9cf9904494f44",
          "url": "https://github.com/paradedb/paradedb/commit/eb3bc1d570c85d60804f6aab2f2d2cf998bd7597"
        },
        "date": 1752597728709,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.166023,
            "unit": "median cpu",
            "extra": "avg cpu: 21.499330744225375, max cpu: 42.561577, count: 57799"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 226.98046875,
            "unit": "median mem",
            "extra": "avg mem: 225.98931568074275, max mem: 237.671875, count: 57799"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.233301,
            "unit": "median cpu",
            "extra": "avg cpu: 22.211796220687102, max cpu: 33.168808, count: 57799"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 159.79296875,
            "unit": "median mem",
            "extra": "avg mem: 159.41483815409867, max mem: 161.18359375, count: 57799"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 22234,
            "unit": "median block_count",
            "extra": "avg block_count: 20653.595961867853, max block_count: 23496.0, count: 57799"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 67,
            "unit": "median segment_count",
            "extra": "avg segment_count: 68.42324261665426, max segment_count: 96.0, count: 57799"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "c7bdeebed6249725c87b87c276d2e09dfaffd152",
          "message": "ci: publish stressgres benchmark graphs (#2852)\n\nThis will publish every stressgres benchmark graph to the new\n`paradedb/benchmark-data` repo.\n\nIt also ensures that when one of our \"[benchmark]\" labels are applied,\nthat it does *not* pull the benchmarks from main, but instead from the\nPR branch itself.",
          "timestamp": "2025-07-15T18:13:26-04:00",
          "tree_id": "f18bf519521a03613c5b12c861092f24314476ad",
          "url": "https://github.com/paradedb/paradedb/commit/c7bdeebed6249725c87b87c276d2e09dfaffd152"
        },
        "date": 1752619180447,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.233301,
            "unit": "median cpu",
            "extra": "avg cpu: 21.43613138932945, max cpu: 42.857143, count: 57657"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 226.31640625,
            "unit": "median mem",
            "extra": "avg mem: 225.18311611393673, max mem: 230.53125, count: 57657"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.27837,
            "unit": "median cpu",
            "extra": "avg cpu: 22.157898716739325, max cpu: 33.333336, count: 57657"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 159.3203125,
            "unit": "median mem",
            "extra": "avg mem: 159.34853523585602, max mem: 161.203125, count: 57657"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 21914,
            "unit": "median block_count",
            "extra": "avg block_count: 20509.367049967914, max block_count: 23527.0, count: 57657"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 67,
            "unit": "median segment_count",
            "extra": "avg segment_count: 68.3133704493817, max segment_count: 97.0, count: 57657"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Philippe Noël",
            "username": "philippemnoel",
            "email": "21990816+philippemnoel@users.noreply.github.com"
          },
          "committer": {
            "name": "Stu Hood",
            "username": "stuhood",
            "email": "stuhood@gmail.com"
          },
          "id": "8c164d666c4af2049b439690afa7823ab5be2c88",
          "message": "ci: Post \"One-branch Release Model\" Improvements (#2792)\n\n# Ticket(s) Closed\n\n- Closes #N/A\n\n## What\n- Don't make Docker Scout post on PRs that don't modify the Dockerfile,\nnot necessary.\n- Make sure the SchemaBot workflow errors when community contributors\nmake a PR, so we can catch needed changes to the SQL upgrade script.\n- Rename it to SchemaBot\n\n## Why\nQoL\n\n## How\n^\n\n## Tests\nCI",
          "timestamp": "2025-07-08T15:49:16Z",
          "url": "https://github.com/paradedb/paradedb/commit/8c164d666c4af2049b439690afa7823ab5be2c88"
        },
        "date": 1752669156374,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.188406,
            "unit": "median cpu",
            "extra": "avg cpu: 21.289877036708354, max cpu: 42.772278, count: 57286"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 228.16015625,
            "unit": "median mem",
            "extra": "avg mem: 227.31538178503473, max mem: 239.4609375, count: 57286"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.255816,
            "unit": "median cpu",
            "extra": "avg cpu: 22.296618725773587, max cpu: 33.267326, count: 57286"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 159.2890625,
            "unit": "median mem",
            "extra": "avg mem: 159.00267858311804, max mem: 161.9296875, count: 57286"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 21494,
            "unit": "median block_count",
            "extra": "avg block_count: 19986.319327584402, max block_count: 21623.0, count: 57286"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 67,
            "unit": "median segment_count",
            "extra": "avg segment_count: 68.68140557902454, max segment_count: 96.0, count: 57286"
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
          "id": "f7c13c858851200e8ab5395779f821ca484cda0d",
          "message": "feat: Add a custom scan for aggregates (#2763)\n\n## What\n\nAdd a new `CustomScan` (installed with `create_upper_paths_hook`) which\nreplaces simple aggregate plans on `bm25`-indexed tables with aggregate\nimplementations using [Tantivy\naggregates](https://docs.rs/tantivy/latest/tantivy/aggregation/index.html).\n\n## Why\n\nTantivy aggregates can be significantly faster (in benchmarks, we've\nmeasured between 4-10x for bucketing/faceting queries). They have been\nexposed via `paradedb.aggregate` for a while now, but that function\nrequires learning a new API, and does not feel \"Postgres native\".\n\n## How\n\n* Adjust `CustomPathBuilder` and `CustomPathMethods` to allow multiple\n`CustomScan` implementations.\n* Remove the `CustomScan::PrivateData: Default` bound, as it requires\nthe `PrivateData` to start in an illegal state.\n* Move `qual_inspect` to a reusable location.\n* Split out a module to be used by both the `aggregate` API method and\nby the aggregate custom scan.\n* Implement the \"ParadeDB Aggregate Scan\" `CustomScan` type\n    * Add the new `CustomScan` type, hidden behind a GUC\n    * Filter Paths to those which represent `count(*)` queries\n    * Extract `quals` during `CustomPath` generation\n* Replace `Aggrefs` in target lists with `FuncExprs` while producing a\n`CustomPlan`\n* Execute a `count(*)` aggregate by pushing down a `value_count`\naggregate on the `ctid`\n\n## Tests\n\nAdded tests to validate that:\n* the GUC properly controls usage\n* the scan does not trigger for unsupported aggregates, tables without a\n`bm25` index, or group-bys (for now)",
          "timestamp": "2025-07-16T09:12:24-07:00",
          "tree_id": "69b043a9363fcf6ce2de468c97d14e41f593f017",
          "url": "https://github.com/paradedb/paradedb/commit/f7c13c858851200e8ab5395779f821ca484cda0d"
        },
        "date": 1752684018954,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.143684,
            "unit": "median cpu",
            "extra": "avg cpu: 21.22296708182704, max cpu: 42.72997, count: 57682"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 227.658203125,
            "unit": "median mem",
            "extra": "avg mem: 226.41788748548075, max mem: 238.91796875, count: 57682"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.210833,
            "unit": "median cpu",
            "extra": "avg cpu: 22.25931379199962, max cpu: 33.07087, count: 57682"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 160.64453125,
            "unit": "median mem",
            "extra": "avg mem: 160.3478258219635, max mem: 162.47265625, count: 57682"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 22184,
            "unit": "median block_count",
            "extra": "avg block_count: 20699.68728546167, max block_count: 23490.0, count: 57682"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 67,
            "unit": "median segment_count",
            "extra": "avg segment_count: 68.62745743906244, max segment_count: 96.0, count: 57682"
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
          "id": "c779dbb72178c1aa0e14ad94c72eeb9937251acd",
          "message": "feat: a background merger for large layers (#2743)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nLaunches Postgres [dynamic background\nworkers](https://docs.rs/pgrx/latest/pgrx/bgworkers/struct.DynamicBackgroundWorker.html)\nto merge large layers in the background.\n\nThis is configured by a new index setting, `background_layer_sizes`. Any\nlayers specified here will be merged in the background.\n\n## Why\n\nWrite throughput, and create more balanced indexes with fewer segments.\n\n## How\n\n1. The default (foreground) `layer_sizes` is now `10kb`, `100kb`, `1mb`\n2. The default `background_layer_sizes` is `10mb`, `100mb`, `1gb`,\n`10gb`, `100gb`, `1tb`\n3. After `aminsert` merges in the foreground, it checks to see if there\nare enough candidates for a background merge. If so, it spawns a dynamic\nbackground worker process to do the merge.\n4. `amvacuumcleanup` can also spawn a background worker, allowing the\nuser to rebalance their index with a `VACUUM`.\n\nAdditionally, I've tweaked merge policy to make it smarter:\n\n1. Terminate a merge early if we predict that, after the merge, we will\nend up with fewer than `target_segment_count` segments.\n2. Set the max layer size as byte size of index divided by\n`target_segment_count`. Discard all layer sizes above that, which avoids\nthe problem of merging too many segments into one giant segment.\n\nThis has allowed us to delete some code:\n\n1. `force_merge` is deprecated\n2. No longer need to store the segments created by an index build and\navoid merging them, since we just do expensive merges in the background\n\n## Tests\n\nAdded a new stressgres `.toml` file that configures the layer\nsizes/threshold, which triggers background merging.\n\n---------\n\nSigned-off-by: Ming <ming.ying.nyc@gmail.com>\nCo-authored-by: Stu Hood <stuhood@paradedb.com>\nCo-authored-by: Philippe Noël <21990816+philippemnoel@users.noreply.github.com>\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>",
          "timestamp": "2025-07-16T12:44:23-04:00",
          "tree_id": "153c406e456a638a6b68ca9123210bbd498d66a5",
          "url": "https://github.com/paradedb/paradedb/commit/c779dbb72178c1aa0e14ad94c72eeb9937251acd"
        },
        "date": 1752685945099,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.188406,
            "unit": "median cpu",
            "extra": "avg cpu: 21.633339028789774, max cpu: 42.772278, count: 57100"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 228.04296875,
            "unit": "median mem",
            "extra": "avg mem: 226.87540526488615, max mem: 240.7734375, count: 57100"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.233301,
            "unit": "median cpu",
            "extra": "avg cpu: 22.311687262958266, max cpu: 33.136093, count: 57100"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 159.9765625,
            "unit": "median mem",
            "extra": "avg mem: 159.58649258428196, max mem: 161.078125, count: 57100"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 22063,
            "unit": "median block_count",
            "extra": "avg block_count: 20651.66176882662, max block_count: 23610.0, count: 57100"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 67,
            "unit": "median segment_count",
            "extra": "avg segment_count: 68.31493870402802, max segment_count: 97.0, count: 57100"
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
          "id": "d197630155f55d13eea0a81841951645a367a5f7",
          "message": "test: Make GUCs a tested property, and add proptests for the aggregate custom scan (#2788)\n\n## What\n\nMove the GUCs that are in use into a test property, which reduces\nruntime while allowing for testing arbitrary combinations of GUCs.\n\nAdditionally, added proptests for the aggregate custom scan by:\n* Adding its GUC to the generated set\n* Expanding the `generated_single_relation` test to use `COUNT(*)` in\nits target list.\n\n## Why\n\nTo add additional test coverage for #2763.",
          "timestamp": "2025-07-16T10:57:16-07:00",
          "tree_id": "6a92b62b640f7a0b2dc7e6507966b75f3bd00594",
          "url": "https://github.com/paradedb/paradedb/commit/d197630155f55d13eea0a81841951645a367a5f7"
        },
        "date": 1752690220049,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.166023,
            "unit": "median cpu",
            "extra": "avg cpu: 21.31314547043461, max cpu: 43.199997, count: 57673"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 226.63671875,
            "unit": "median mem",
            "extra": "avg mem: 225.93933395230437, max mem: 231.703125, count: 57673"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.255816,
            "unit": "median cpu",
            "extra": "avg cpu: 22.41056677723073, max cpu: 33.168808, count: 57673"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 159.83984375,
            "unit": "median mem",
            "extra": "avg mem: 159.7338768384903, max mem: 161.1640625, count: 57673"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 22670,
            "unit": "median block_count",
            "extra": "avg block_count: 20906.578364225894, max block_count: 23892.0, count: 57673"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 67,
            "unit": "median segment_count",
            "extra": "avg segment_count: 69.00568723666187, max segment_count: 96.0, count: 57673"
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
          "id": "58f6742b2cde46fe2c15077e59a0d5b091ba6000",
          "message": "chore: Upgrade to `0.17.0` (#2870)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-07-16T19:49:53-04:00",
          "tree_id": "b77118d365b31a4f3478b7c0571cf1f7e9f8a5b2",
          "url": "https://github.com/paradedb/paradedb/commit/58f6742b2cde46fe2c15077e59a0d5b091ba6000"
        },
        "date": 1752711475919,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.210833,
            "unit": "median cpu",
            "extra": "avg cpu: 21.391909908974103, max cpu: 42.772278, count: 57393"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 227.39453125,
            "unit": "median mem",
            "extra": "avg mem: 226.43896597527137, max mem: 237.2578125, count: 57393"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.233301,
            "unit": "median cpu",
            "extra": "avg cpu: 22.225343367938137, max cpu: 33.20158, count: 57393"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 160.37109375,
            "unit": "median mem",
            "extra": "avg mem: 160.13710533895684, max mem: 161.40625, count: 57393"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 21973,
            "unit": "median block_count",
            "extra": "avg block_count: 20639.334134824803, max block_count: 23413.0, count: 57393"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 67,
            "unit": "median segment_count",
            "extra": "avg segment_count: 68.4695694596902, max segment_count: 97.0, count: 57393"
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
          "id": "86bc2b092d2857a997cd05af4b554ac96ba225fd",
          "message": "fix: Fix `rescan` for top-n (#2868)\n\n## What\n\nComments in both `Normal` and `TopN` scans claimed that their search\nresults would be reset by `PdbScanState`, but this was incorrect: they\nwere local variables in the struct, and needed to be reset locally.\n\n## Why\n\nFixes issue detected by the prop tests and repro'd by @eeeebbbbrrrr in\n#2751.\n\n## Tests\n\nNew regression test from #2751: thanks @eeeebbbbrrrr!\n\n---------\n\nCo-authored-by: Eric B. Ridge <eebbrr@paradedb.com>",
          "timestamp": "2025-07-17T08:59:32-07:00",
          "tree_id": "ccc9162bb96008badcb3046612d16394604ed6f7",
          "url": "https://github.com/paradedb/paradedb/commit/86bc2b092d2857a997cd05af4b554ac96ba225fd"
        },
        "date": 1752769564480,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.210833,
            "unit": "median cpu",
            "extra": "avg cpu: 21.67883978823504, max cpu: 42.64561, count: 57850"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 228.375,
            "unit": "median mem",
            "extra": "avg mem: 227.0925817712835, max mem: 240.7265625, count: 57850"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.255816,
            "unit": "median cpu",
            "extra": "avg cpu: 22.211785783486604, max cpu: 33.23442, count: 57850"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 159.84765625,
            "unit": "median mem",
            "extra": "avg mem: 159.69268136884185, max mem: 160.79296875, count: 57850"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 22333,
            "unit": "median block_count",
            "extra": "avg block_count: 20680.245652549696, max block_count: 23424.0, count: 57850"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 67,
            "unit": "median segment_count",
            "extra": "avg segment_count: 68.43790838375108, max segment_count: 95.0, count: 57850"
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
          "id": "5772be0d3dea751c3609729efcc66cc69d9d30ee",
          "message": "chore: Remove debug output in aggregate scan (#2876)\n\n## What\n\nRemove a stray `println`.\n\n## Why\n\nIt's not doing anyone any good.",
          "timestamp": "2025-07-17T09:46:08-07:00",
          "tree_id": "06a002186c2a701283d73854f64c4225a08aa1a8",
          "url": "https://github.com/paradedb/paradedb/commit/5772be0d3dea751c3609729efcc66cc69d9d30ee"
        },
        "date": 1752772359262,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.166023,
            "unit": "median cpu",
            "extra": "avg cpu: 21.53856958816693, max cpu: 42.64561, count: 57123"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 227.97265625,
            "unit": "median mem",
            "extra": "avg mem: 227.16129161524253, max mem: 231.4765625, count: 57123"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.233301,
            "unit": "median cpu",
            "extra": "avg cpu: 22.31710791228252, max cpu: 33.168808, count: 57123"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 159.37109375,
            "unit": "median mem",
            "extra": "avg mem: 159.13455927888066, max mem: 160.15625, count: 57123"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 22325,
            "unit": "median block_count",
            "extra": "avg block_count: 20770.602121737305, max block_count: 23717.0, count: 57123"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 67,
            "unit": "median segment_count",
            "extra": "avg segment_count: 68.69605938063476, max segment_count: 97.0, count: 57123"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "16307a200509cd31b42c1338f3c5b6e097a77fbf",
          "message": "fix: issue #2844 - Use `paradedb.with_index()` when necessary (#2879)\n\n# Ticket(s) Closed\n\n- Closes #2844\n\n## What\n\nWhen the right-hand-side of the `@@@` operator is a complex expression\nthat can't be evaluated to a `T_Const` during planning, we need to\nrewrite it to be a call to our `paradedb.with_index(<indexrelid>,\n<rhs>)` so that it can be properly used in Filter contexts during\nexecution.\n\nThis seems to come up, at least, during RLS rule evaluation if those\nrules use our `@@@` operator.\n\n## Why\n\nSo long as the right-hand-side of `@@@` is a `SearchQueryInput`, we need\nto be able to execute it when Postgres wants us to, whether that's\nduring a scan or, as in this case, a filter being run as part of RLS\nevaluation.\n\n## How\n\nWrapping Postgres expression nodes!\n\n## Tests\n\nI used the re-create the OP from #2844 put together as a regression\ntest. I seem to be lacking the creative energy today to reduce the test\ndown to its simplest form that doesn't require a complicated RLS\nconfiguration.",
          "timestamp": "2025-07-18T13:21:25-04:00",
          "tree_id": "f2e8c7ca2ce45eefd03b85ff45d56f0481ebae1a",
          "url": "https://github.com/paradedb/paradedb/commit/16307a200509cd31b42c1338f3c5b6e097a77fbf"
        },
        "date": 1752860868291,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.210833,
            "unit": "median cpu",
            "extra": "avg cpu: 21.492889293635105, max cpu: 42.814667, count: 57123"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 228.90625,
            "unit": "median mem",
            "extra": "avg mem: 228.02069458394604, max mem: 234.37109375, count: 57123"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.255816,
            "unit": "median cpu",
            "extra": "avg cpu: 22.284744258650274, max cpu: 33.20158, count: 57123"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 160.22265625,
            "unit": "median mem",
            "extra": "avg mem: 159.96487431726274, max mem: 161.84765625, count: 57123"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 22301,
            "unit": "median block_count",
            "extra": "avg block_count: 20759.757995903576, max block_count: 23812.0, count: 57123"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 67,
            "unit": "median segment_count",
            "extra": "avg segment_count: 68.74362340913467, max segment_count: 97.0, count: 57123"
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
          "id": "8b45b24d47a7cac36696bae2d8505c750a7ee801",
          "message": "test: added more join regression tests + improved when to use Join quals (#2808)\n\n# Ticket(s) Closed\n\n- Closes #2807\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-07-18T10:40:03-07:00",
          "tree_id": "fb63bd81696cd27d03b6d5724551c3ff8fd59613",
          "url": "https://github.com/paradedb/paradedb/commit/8b45b24d47a7cac36696bae2d8505c750a7ee801"
        },
        "date": 1752861987929,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.210833,
            "unit": "median cpu",
            "extra": "avg cpu: 21.3908645451741, max cpu: 42.64561, count: 57679"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 228.1015625,
            "unit": "median mem",
            "extra": "avg mem: 227.32159742659806, max mem: 231.64453125, count: 57679"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.27837,
            "unit": "median cpu",
            "extra": "avg cpu: 22.31702151119204, max cpu: 33.168808, count: 57679"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 159.2734375,
            "unit": "median mem",
            "extra": "avg mem: 159.26343433322788, max mem: 161.0703125, count: 57679"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 22517,
            "unit": "median block_count",
            "extra": "avg block_count: 20929.595970803934, max block_count: 23869.0, count: 57679"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 67,
            "unit": "median segment_count",
            "extra": "avg segment_count: 68.92818876887603, max segment_count: 96.0, count: 57679"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "77c6cd1aeb5f5c03bba892a38cd792a113cfbd39",
          "message": "chore: move code around (#2889)\n\n## What\n\nThis relocates the `src/boostrap/create_bm25.rs` source to\n`src/api/admin.rs`, which I believe is a more sensibile location and\nname.\n\nAdditionally it moves the `#[pg_extern] schema()` function from\n`src/api/builder_fns.rs` to this new(ly renamed) `admin.rs`, as it's an\nadministrative function, not a query builder function.\n\n## Why\n\nTrying to better organize things\n\n## How\n\n## Tests\n\nNo functional changes here.",
          "timestamp": "2025-07-22T12:39:21-04:00",
          "tree_id": "1ab036752fc8adf7ad2757741d4e274ab0e73c64",
          "url": "https://github.com/paradedb/paradedb/commit/77c6cd1aeb5f5c03bba892a38cd792a113cfbd39"
        },
        "date": 1753203945270,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.121387,
            "unit": "median cpu",
            "extra": "avg cpu: 21.475938005126512, max cpu: 42.561577, count: 57774"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 228.8046875,
            "unit": "median mem",
            "extra": "avg mem: 228.69132687281476, max mem: 236.390625, count: 57774"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.166023,
            "unit": "median cpu",
            "extra": "avg cpu: 22.259629460821138, max cpu: 33.168808, count: 57774"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 159.17578125,
            "unit": "median mem",
            "extra": "avg mem: 159.18903480588153, max mem: 160.56640625, count: 57774"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 22247,
            "unit": "median block_count",
            "extra": "avg block_count: 20703.81206078859, max block_count: 23457.0, count: 57774"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 67,
            "unit": "median segment_count",
            "extra": "avg segment_count: 68.39417384982865, max segment_count: 97.0, count: 57774"
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
          "id": "1e26a38533fa872a935955128b2ecd08b3febf66",
          "message": "perf: Skip computing scores if not requested for top-n field queries (#2892)\n\n## What\n\n`SearchIndexReader::{top_by_field_in_segments,\ntop_by_string_field_in_segments}` were not using\n`SearchIndexReader::need_scores`, and were instead universally enabling\nscores.\n\nUse the `enable_scoring` helper and `self.need_scores` to skip computing\nscores where possible.\n\n## Why\n\nComputing scores requires field norms, which ([if they have not been\ndisabled](https://docs.paradedb.com/documentation/indexing/record)) can\ntake time to load and use.",
          "timestamp": "2025-07-22T15:58:56-07:00",
          "tree_id": "2a41c9e06be04ef92fed2b265dc3f90340fb0e37",
          "url": "https://github.com/paradedb/paradedb/commit/1e26a38533fa872a935955128b2ecd08b3febf66"
        },
        "date": 1753226722539,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.210833,
            "unit": "median cpu",
            "extra": "avg cpu: 21.25636637999243, max cpu: 42.72997, count: 57332"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 228.81640625,
            "unit": "median mem",
            "extra": "avg mem: 227.75997282004377, max mem: 231.57421875, count: 57332"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.255816,
            "unit": "median cpu",
            "extra": "avg cpu: 22.33732230142304, max cpu: 33.103447, count: 57332"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 159.21484375,
            "unit": "median mem",
            "extra": "avg mem: 158.9373458812269, max mem: 160.35546875, count: 57332"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 22599,
            "unit": "median block_count",
            "extra": "avg block_count: 20892.9723191237, max block_count: 23815.0, count: 57332"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 67,
            "unit": "median segment_count",
            "extra": "avg segment_count: 68.89136956673411, max segment_count: 97.0, count: 57332"
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
          "id": "cbd2e802901988c181281a9b7f772543e1df32d2",
          "message": "chore: add `background-merge.toml` to CI (#2895)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nThis new stressgres file was instrumental in catching the bug fixed in\n#2845 and was used to test background merging, but has not been added to\nCI.\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-07-23T10:26:09-04:00",
          "tree_id": "bb8e3ca51c2d0de5e56be65f478f11096b0d410e",
          "url": "https://github.com/paradedb/paradedb/commit/cbd2e802901988c181281a9b7f772543e1df32d2"
        },
        "date": 1753282355868,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.188406,
            "unit": "median cpu",
            "extra": "avg cpu: 21.445183563583686, max cpu: 42.477875, count: 57272"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 224.58203125,
            "unit": "median mem",
            "extra": "avg mem: 224.71211315356982, max mem: 235.37890625, count: 57272"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.233301,
            "unit": "median cpu",
            "extra": "avg cpu: 22.18101931242329, max cpu: 33.267326, count: 57272"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 160.2265625,
            "unit": "median mem",
            "extra": "avg mem: 159.87671297438104, max mem: 161.26171875, count: 57272"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 22132,
            "unit": "median block_count",
            "extra": "avg block_count: 20656.015260511245, max block_count: 23581.0, count: 57272"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 67,
            "unit": "median segment_count",
            "extra": "avg segment_count: 68.4196116776086, max segment_count: 95.0, count: 57272"
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
          "id": "53fa29a57e7daa2bdec9bf05f86d8db50088fd51",
          "message": "perf: Avoid a re-query when a top-n query is exhausted (#2888)\n\n## What\n\nSplit the iterators consumed by the `TopN` exec method and by the\n`Normal`/`FF` exec methods into two separate implementations.\n\nAfter splitting them, use the known exact size of the `TopN` iterator to\nexit early (as `exhausted: bool`) if we had fewer matches than were\nrequested. This avoids re-querying an iterator (and re-scanning its\ncolumns) that we know has no more results.\n\n## Why\n\nThe two types of consumers of these iterators have very different\nconsumption patterns and constraints:\n* TopN knows exactly how many results there will be, and already buffers\nthem all in memory. It would like to be able to know the precise count\nof results.\n* `Normal` and `FF` exec methods do not know the total number of results\nto expect, and in some cases would like to be able to consume in a\nsegment-aware fashion in order to be able to late-fetch fast field\ncolumns (see #2623).\n\n## Tests\n\nAdded a test to cement the change in query count.\n\nBenchmarks show a 1.5x speedup for `paging-string-max`, and no change\nfor other queries.",
          "timestamp": "2025-07-23T09:13:45-07:00",
          "tree_id": "c162e59fb9a6b4523c0ff39e10d9d54d92761460",
          "url": "https://github.com/paradedb/paradedb/commit/53fa29a57e7daa2bdec9bf05f86d8db50088fd51"
        },
        "date": 1753288813669,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.188406,
            "unit": "median cpu",
            "extra": "avg cpu: 21.33187967137643, max cpu: 42.772278, count: 57319"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 226.41796875,
            "unit": "median mem",
            "extra": "avg mem: 225.80655526799578, max mem: 231.17578125, count: 57319"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.233301,
            "unit": "median cpu",
            "extra": "avg cpu: 22.203146847996134, max cpu: 33.136093, count: 57319"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 159.65234375,
            "unit": "median mem",
            "extra": "avg mem: 159.4213017280483, max mem: 160.94921875, count: 57319"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 22195,
            "unit": "median block_count",
            "extra": "avg block_count: 20721.82180428828, max block_count: 23553.0, count: 57319"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 67,
            "unit": "median segment_count",
            "extra": "avg segment_count: 68.58313997103927, max segment_count: 94.0, count: 57319"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "880ee607618b7ffe12f2784d03be3110b4df6cdf",
          "message": "feat: text search operators (#2877)\n\n## What\n\nThis is the implementation of 4 new operators designed around common\n\"text search\" patterns: `===` (exact token), `&&&` (tokenized term\nconjunction) `|||` (tokenized term disjunction), and `###` (phrase).\n\nFirst, `@@@` still exists and still does what it's always done, which is\nrun the right-hand-side through tantivy's \"query parser\".\n\nThese new operators have specific meanings. The spelling of these\noperators is both to maintain some kind of consistency in that they're\nall 3 of the same symbol and also to try and confer their meaning.\n\nIn all cases, the left-hand-side of the operator is required to be a\nreference to a Postgres `TEXT`/`VARCHAR` field and the right-hand-side\nis text. The operator definition defines how the right-hand-side input\nis applied to the left-hand-side field. These are \"text search\"\noperators and would be nonsensical applied to non-text fields.\n\n- `field === 'TEXT'` - this is the exact term match operator. The\nright-hand-side value is used, unmodified, to perform the equivalent of\nthe `paradedb.term()` function.\n\n- `field === ARRAY['...', '...', '...']` - similar to the above, but\nrewrites to the `paradedb.term_set()` function.\n\n- `field &&& 'TEXT'` - this is the \"match conjunction\" operator. It\ntokenizes the right-hand-side using the index tokenizer defined by the\nleft-hand-side field and searches for documents that contain **all** of\nthe tokenized terms. Think, \"find all documents that contain all terms\ntokenized from this text input\". This is the equivalent of the\n`paradedb.match` function with the `conjunction_mode => true`.\n\n- `field ||| 'TEXT'` - this is the \"match disjunction\" operator. It\ntokenizes the right-hand-side using the index tokenizer defined by the\nleft-hand-side field and searches for documents that contain **any** of\nthe tokenized terms. Think, \"find all documents that contain one or more\nof the terms tokenized from this text input\". This is the equivalent of\nthe `paradedb.match` function with the `conjunction_mode => false`.\n\n- `field ### 'TEXT'` - this is the \"phrase search\" operator. It\ntokenizes the right-hand-side using the index tokenizer defined by the\nleft-hand-side field and searches for documents that contain all the\ntokenized terms in the order written. Think, \"find all documents that\ncontain this tokenized phrase\". This is akin to the `paradedb.phrase()`\nfunction, except the `###` is transparently handling the tokenization.\n\nIf you wanted to find all documents that talk about bbq and chicken and\nmaybe hotdogs and happen on July 4th, you might write a query like this:\n\n```sql\nSELECT * \nFROM events \nWHERE (\n       description &&& 'bbq chicken' OR description ||| 'hot dog hotdogs'\n) AND description ### 'July 4th';\n```\n\n## Why\n\nThese operators are in furtherance of our\n[roadmap](https://github.com/orgs/paradedb/discussions/2041) --\nspecifically the \"Database UX improvements\" line item.\n\nOur overall goal with that roadmap item is to start moving users and ORM\ntools away from needing to use our \"builder functions\" in the common\ntext-search cases. What we're striving for is for users, and ORM tools,\nto write standard SQL WHERE clauses. In order to do that we need to make\ncommon things more accessible using custom operators.\n\n## How\n\nLargely just adding new `#[pg_operator]` functions and ensuring they all\nhave a SUPPORT function that can rewrite the expression to what we need.\n\n@rebasedming said he was going to tackle documentation, which can come\nin a separate PR.\n\n## Tests\n\nThere's a new regression test named `operators.sql` that validates\nEXPLAIN output and search results against our standard \"mock_items\"\ntable.",
          "timestamp": "2025-07-23T12:54:32-04:00",
          "tree_id": "f4a0dd94cd1e423e30c42103a8a23d86905d32e7",
          "url": "https://github.com/paradedb/paradedb/commit/880ee607618b7ffe12f2784d03be3110b4df6cdf"
        },
        "date": 1753291267666,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.188406,
            "unit": "median cpu",
            "extra": "avg cpu: 21.407990295030217, max cpu: 42.772278, count: 57349"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 228.1640625,
            "unit": "median mem",
            "extra": "avg mem: 227.24593157138747, max mem: 231.77734375, count: 57349"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.255816,
            "unit": "median cpu",
            "extra": "avg cpu: 22.26806501742115, max cpu: 33.267326, count: 57349"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 160.2578125,
            "unit": "median mem",
            "extra": "avg mem: 160.15853923183056, max mem: 162.0546875, count: 57349"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 22404,
            "unit": "median block_count",
            "extra": "avg block_count: 20806.48503025336, max block_count: 23892.0, count: 57349"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 67,
            "unit": "median segment_count",
            "extra": "avg segment_count: 69.04678372770232, max segment_count: 97.0, count: 57349"
          }
        ]
      }
    ],
    "pg_search wide-table.toml Performance - TPS": [
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
        "date": 1752442236359,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 27.394910104031283,
            "unit": "median tps",
            "extra": "avg tps: 27.40363733171049, max tps: 29.8013126016641, count: 56545"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 130.94928355084787,
            "unit": "median tps",
            "extra": "avg tps: 130.83334295106317, max tps: 139.2821900187232, count: 56545"
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
        "date": 1752442252713,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 16.159447636951437,
            "unit": "median tps",
            "extra": "avg tps: 14.809828617457812, max tps: 24.403885476014544, count: 57332"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 77.12986876221302,
            "unit": "median tps",
            "extra": "avg tps: 76.76276613533433, max tps: 77.30224567934269, count: 57332"
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
        "date": 1752442254437,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 16.554145989849115,
            "unit": "median tps",
            "extra": "avg tps: 15.269708280453532, max tps: 24.994078246768787, count: 57542"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 75.53674769001191,
            "unit": "median tps",
            "extra": "avg tps: 75.52729230044916, max tps: 77.8814162149735, count: 57542"
          }
        ]
      },
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
          "id": "4fd1b2b6b6664d03946be0f4836732f0f40df0cc",
          "message": "chore: Rename datasets and add string paging queries (#2834)\n\n## What\n\nAdd a high-cardinality paging/top-n query to the benchmarks, and rename\ndatasets to match their content. Additionally, improve the generation\nscript for the `docs` dataset to avoid joins and allow for deterministic\nrelative-position queries.\n\n## Why\n\nWe don't currently have a high-cardinality string paging/top-n query in\nthe benchmark. We have top-n on a string column, but only for\nlow-cardinality values (`top_n-string.sql`). The top-n case represented\nan important gap that a user encountered, which #2828 addresses.\n\nThe names of the `benchmark` datasets don't currently describe their\nshape / schema, and for the `join` dataset in particular, that would\ndiscourage using it for other types of queries. We rename it to `docs`\nhere, and then use the `pages` table as the dataset for top-n.\n\n## Tests\n\nTested locally that the new query demonstrates a speedup for #2828.",
          "timestamp": "2025-07-13T18:04:27Z",
          "url": "https://github.com/paradedb/paradedb/commit/4fd1b2b6b6664d03946be0f4836732f0f40df0cc"
        },
        "date": 1752442316929,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 27.820080055862302,
            "unit": "median tps",
            "extra": "avg tps: 27.834241494196217, max tps: 30.453223515019523, count: 56451"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 127.3977321254237,
            "unit": "median tps",
            "extra": "avg tps: 127.118831626562, max tps: 144.56693435062684, count: 56451"
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
        "date": 1752442321110,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 27.422879453280174,
            "unit": "median tps",
            "extra": "avg tps: 27.43783426401097, max tps: 30.12035266579926, count: 56530"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 128.50874295170954,
            "unit": "median tps",
            "extra": "avg tps: 128.82428203223077, max tps: 140.94530954189634, count: 56530"
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
        "date": 1752442353084,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 27.553686989507455,
            "unit": "median tps",
            "extra": "avg tps: 27.558566783599648, max tps: 30.117527128519693, count: 56533"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 130.55712472358144,
            "unit": "median tps",
            "extra": "avg tps: 130.27248344836332, max tps: 142.806393353869, count: 56533"
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
          "id": "47bbe518381e1429f228328336dad78e99636ad9",
          "message": "chore: Upgrade to `0.16.0` (#2720)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-06-23T23:04:37Z",
          "url": "https://github.com/paradedb/paradedb/commit/47bbe518381e1429f228328336dad78e99636ad9"
        },
        "date": 1752442357506,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 15.471582808662124,
            "unit": "median tps",
            "extra": "avg tps: 14.295234446550012, max tps: 23.179330306033904, count: 57290"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 72.94771597895858,
            "unit": "median tps",
            "extra": "avg tps: 73.04437425920533, max tps: 78.16802493947438, count: 57290"
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
          "id": "b96e41e25c3fd2126f54aa5cb66d4deffb482047",
          "message": "perf: Lazily load fast fields dictionaries. (#2842)\n\n## What\n\nLazily load fast field dictionaries from buffers: see\nhttps://github.com/paradedb/tantivy/pull/55\n\n## Why\n\nA customer reported slower-than-expected paging on a string/uuid column.\n85% of the time for that query was being spent in _opening_ a fast\nfields string/bytes column, with a large fraction of that time spent\nfully consuming the column's `Dictionary`.\n\n## Tests\n\nSee the attached benchmark results:\n* [`docs`\ndataset](https://github.com/paradedb/paradedb/pull/2842#pullrequestreview-3014379545)\n    * No regressions.\n    * 2x faster for `top_n-score`\n    * 1.4x faster for `highlighting` \n* [`logs`\ndataset](https://github.com/paradedb/paradedb/pull/2842#pullrequestreview-3014350211)\n    * No regressions.\n    * 4.5x faster for `paging-string-max`\n    * 1.7x faster for `paging-string-median`\n    * 1.6x faster for `paging-string-min`\n\nThe `paging-string-*` benchmarks were added in #2834 to highlight this\nparticular issue.",
          "timestamp": "2025-07-14T08:28:09-07:00",
          "tree_id": "d144335dcb7c7f138a112c01e5b9ff5e0168fe37",
          "url": "https://github.com/paradedb/paradedb/commit/b96e41e25c3fd2126f54aa5cb66d4deffb482047"
        },
        "date": 1752509169384,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 28.31715639607377,
            "unit": "median tps",
            "extra": "avg tps: 28.253948384602385, max tps: 30.115363538507868, count: 57172"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 142.15854575608682,
            "unit": "median tps",
            "extra": "avg tps: 141.66622780230318, max tps: 148.52373904162434, count: 57172"
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
          "id": "ee6395b4b4d4ca6f44e2c89b74afd2308d4415a8",
          "message": "fix: orphaned delete entries get GCed too early (#2845)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nWhen running a new stressgres suite (coming in a future PR), I hit a\nmysterious bug where it looked like vacuum could cause corruption of\nsome pages.\n\nTurns out it's caused by scenarios where:\n\n1. A `DeleteEntry` already exists for a `SegmentMetaEntry`, and a new\none is created\n2. A new, \"fake\" `SegmentMetaEntry` gets created for the purpose of\nstoring the old `DeleteEntry`, so its blocks can get garbage collected\n3. Because this \"fake\" entry is invisible to all readers besides the\ngarbage collector, it doesn't get pinned and can get garbage collected\ntoo early (i.e. while a reader is still pinning the old `DeleteEntry`)\n\nThe solution is to copy all of the contents of the old\n`SegmentMetaEntry` to the fake one, so that the \"pintest blockno\" of the\nfake entry is that same as that of the entry with the new `DeleteEntry`.\nThat way, the `DeleteEntry` doesn't get garbage collected until the pin\nis released.\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-07-14T15:46:29-04:00",
          "tree_id": "3dc55f49de121cf04534f48e3584a2a3ae333407",
          "url": "https://github.com/paradedb/paradedb/commit/ee6395b4b4d4ca6f44e2c89b74afd2308d4415a8"
        },
        "date": 1752524576541,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 27.330048024244046,
            "unit": "median tps",
            "extra": "avg tps: 27.305459812830478, max tps: 29.261644685497295, count: 56467"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 130.87823847781894,
            "unit": "median tps",
            "extra": "avg tps: 130.8731015204667, max tps: 142.55683895605753, count: 56467"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "eb3bc1d570c85d60804f6aab2f2d2cf998bd7597",
          "message": "ci: benchmark workflow cleanups (#2851)\n\nThis is an attempt to cleanup the benchmark workflows a little bit.  \n\n- Centralizes checking out the latest benchmark code/suites/actions into\na composite action.\n- figures out the PR #/title being tested\n- Changes the slack notification messages to be reactive to the\nenvironment to hopefully avoid conflicts with -enterprise",
          "timestamp": "2025-07-15T12:15:54-04:00",
          "tree_id": "223c726790d68868f538b7f5aab9cf9904494f44",
          "url": "https://github.com/paradedb/paradedb/commit/eb3bc1d570c85d60804f6aab2f2d2cf998bd7597"
        },
        "date": 1752598339826,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 28.382677648563025,
            "unit": "median tps",
            "extra": "avg tps: 28.290879402955007, max tps: 30.52045789090136, count: 57164"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 137.55425267303485,
            "unit": "median tps",
            "extra": "avg tps: 137.65325569812168, max tps: 145.6595574352143, count: 57164"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "c7bdeebed6249725c87b87c276d2e09dfaffd152",
          "message": "ci: publish stressgres benchmark graphs (#2852)\n\nThis will publish every stressgres benchmark graph to the new\n`paradedb/benchmark-data` repo.\n\nIt also ensures that when one of our \"[benchmark]\" labels are applied,\nthat it does *not* pull the benchmarks from main, but instead from the\nPR branch itself.",
          "timestamp": "2025-07-15T18:13:26-04:00",
          "tree_id": "f18bf519521a03613c5b12c861092f24314476ad",
          "url": "https://github.com/paradedb/paradedb/commit/c7bdeebed6249725c87b87c276d2e09dfaffd152"
        },
        "date": 1752619793346,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 27.494826383202582,
            "unit": "median tps",
            "extra": "avg tps: 27.52106285429916, max tps: 29.705591447831516, count: 56424"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 128.51072601320266,
            "unit": "median tps",
            "extra": "avg tps: 128.23104107317803, max tps: 145.19202534924474, count: 56424"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Philippe Noël",
            "username": "philippemnoel",
            "email": "21990816+philippemnoel@users.noreply.github.com"
          },
          "committer": {
            "name": "Stu Hood",
            "username": "stuhood",
            "email": "stuhood@gmail.com"
          },
          "id": "8c164d666c4af2049b439690afa7823ab5be2c88",
          "message": "ci: Post \"One-branch Release Model\" Improvements (#2792)\n\n# Ticket(s) Closed\n\n- Closes #N/A\n\n## What\n- Don't make Docker Scout post on PRs that don't modify the Dockerfile,\nnot necessary.\n- Make sure the SchemaBot workflow errors when community contributors\nmake a PR, so we can catch needed changes to the SQL upgrade script.\n- Rename it to SchemaBot\n\n## Why\nQoL\n\n## How\n^\n\n## Tests\nCI",
          "timestamp": "2025-07-08T15:49:16Z",
          "url": "https://github.com/paradedb/paradedb/commit/8c164d666c4af2049b439690afa7823ab5be2c88"
        },
        "date": 1752669770232,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 28.895989538945585,
            "unit": "median tps",
            "extra": "avg tps: 28.80995836001874, max tps: 29.972178637932505, count: 57536"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 139.77297254808417,
            "unit": "median tps",
            "extra": "avg tps: 139.4712054480924, max tps: 155.96113186579203, count: 57536"
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
          "id": "f7c13c858851200e8ab5395779f821ca484cda0d",
          "message": "feat: Add a custom scan for aggregates (#2763)\n\n## What\n\nAdd a new `CustomScan` (installed with `create_upper_paths_hook`) which\nreplaces simple aggregate plans on `bm25`-indexed tables with aggregate\nimplementations using [Tantivy\naggregates](https://docs.rs/tantivy/latest/tantivy/aggregation/index.html).\n\n## Why\n\nTantivy aggregates can be significantly faster (in benchmarks, we've\nmeasured between 4-10x for bucketing/faceting queries). They have been\nexposed via `paradedb.aggregate` for a while now, but that function\nrequires learning a new API, and does not feel \"Postgres native\".\n\n## How\n\n* Adjust `CustomPathBuilder` and `CustomPathMethods` to allow multiple\n`CustomScan` implementations.\n* Remove the `CustomScan::PrivateData: Default` bound, as it requires\nthe `PrivateData` to start in an illegal state.\n* Move `qual_inspect` to a reusable location.\n* Split out a module to be used by both the `aggregate` API method and\nby the aggregate custom scan.\n* Implement the \"ParadeDB Aggregate Scan\" `CustomScan` type\n    * Add the new `CustomScan` type, hidden behind a GUC\n    * Filter Paths to those which represent `count(*)` queries\n    * Extract `quals` during `CustomPath` generation\n* Replace `Aggrefs` in target lists with `FuncExprs` while producing a\n`CustomPlan`\n* Execute a `count(*)` aggregate by pushing down a `value_count`\naggregate on the `ctid`\n\n## Tests\n\nAdded tests to validate that:\n* the GUC properly controls usage\n* the scan does not trigger for unsupported aggregates, tables without a\n`bm25` index, or group-bys (for now)",
          "timestamp": "2025-07-16T09:12:24-07:00",
          "tree_id": "69b043a9363fcf6ce2de468c97d14e41f593f017",
          "url": "https://github.com/paradedb/paradedb/commit/f7c13c858851200e8ab5395779f821ca484cda0d"
        },
        "date": 1752684636196,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 26.848671487710114,
            "unit": "median tps",
            "extra": "avg tps: 26.875068178105664, max tps: 29.664177954774157, count: 57963"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 131.03485228825542,
            "unit": "median tps",
            "extra": "avg tps: 130.80097684615797, max tps: 144.10530655851247, count: 57963"
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
          "id": "c779dbb72178c1aa0e14ad94c72eeb9937251acd",
          "message": "feat: a background merger for large layers (#2743)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nLaunches Postgres [dynamic background\nworkers](https://docs.rs/pgrx/latest/pgrx/bgworkers/struct.DynamicBackgroundWorker.html)\nto merge large layers in the background.\n\nThis is configured by a new index setting, `background_layer_sizes`. Any\nlayers specified here will be merged in the background.\n\n## Why\n\nWrite throughput, and create more balanced indexes with fewer segments.\n\n## How\n\n1. The default (foreground) `layer_sizes` is now `10kb`, `100kb`, `1mb`\n2. The default `background_layer_sizes` is `10mb`, `100mb`, `1gb`,\n`10gb`, `100gb`, `1tb`\n3. After `aminsert` merges in the foreground, it checks to see if there\nare enough candidates for a background merge. If so, it spawns a dynamic\nbackground worker process to do the merge.\n4. `amvacuumcleanup` can also spawn a background worker, allowing the\nuser to rebalance their index with a `VACUUM`.\n\nAdditionally, I've tweaked merge policy to make it smarter:\n\n1. Terminate a merge early if we predict that, after the merge, we will\nend up with fewer than `target_segment_count` segments.\n2. Set the max layer size as byte size of index divided by\n`target_segment_count`. Discard all layer sizes above that, which avoids\nthe problem of merging too many segments into one giant segment.\n\nThis has allowed us to delete some code:\n\n1. `force_merge` is deprecated\n2. No longer need to store the segments created by an index build and\navoid merging them, since we just do expensive merges in the background\n\n## Tests\n\nAdded a new stressgres `.toml` file that configures the layer\nsizes/threshold, which triggers background merging.\n\n---------\n\nSigned-off-by: Ming <ming.ying.nyc@gmail.com>\nCo-authored-by: Stu Hood <stuhood@paradedb.com>\nCo-authored-by: Philippe Noël <21990816+philippemnoel@users.noreply.github.com>\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>",
          "timestamp": "2025-07-16T12:44:23-04:00",
          "tree_id": "153c406e456a638a6b68ca9123210bbd498d66a5",
          "url": "https://github.com/paradedb/paradedb/commit/c779dbb72178c1aa0e14ad94c72eeb9937251acd"
        },
        "date": 1752686563291,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 28.272880944737512,
            "unit": "median tps",
            "extra": "avg tps: 28.113727458335315, max tps: 28.488902051741423, count: 57220"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 139.84343942746045,
            "unit": "median tps",
            "extra": "avg tps: 139.07330414219692, max tps: 142.1886810616866, count: 57220"
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
          "id": "d197630155f55d13eea0a81841951645a367a5f7",
          "message": "test: Make GUCs a tested property, and add proptests for the aggregate custom scan (#2788)\n\n## What\n\nMove the GUCs that are in use into a test property, which reduces\nruntime while allowing for testing arbitrary combinations of GUCs.\n\nAdditionally, added proptests for the aggregate custom scan by:\n* Adding its GUC to the generated set\n* Expanding the `generated_single_relation` test to use `COUNT(*)` in\nits target list.\n\n## Why\n\nTo add additional test coverage for #2763.",
          "timestamp": "2025-07-16T10:57:16-07:00",
          "tree_id": "6a92b62b640f7a0b2dc7e6507966b75f3bd00594",
          "url": "https://github.com/paradedb/paradedb/commit/d197630155f55d13eea0a81841951645a367a5f7"
        },
        "date": 1752690839206,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 28.557130855444164,
            "unit": "median tps",
            "extra": "avg tps: 28.367013931267547, max tps: 28.658773131998693, count: 57786"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 140.46948305208474,
            "unit": "median tps",
            "extra": "avg tps: 139.39578803939588, max tps: 142.72541952634325, count: 57786"
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
          "id": "58f6742b2cde46fe2c15077e59a0d5b091ba6000",
          "message": "chore: Upgrade to `0.17.0` (#2870)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-07-16T19:49:53-04:00",
          "tree_id": "b77118d365b31a4f3478b7c0571cf1f7e9f8a5b2",
          "url": "https://github.com/paradedb/paradedb/commit/58f6742b2cde46fe2c15077e59a0d5b091ba6000"
        },
        "date": 1752712094245,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 28.578566985165228,
            "unit": "median tps",
            "extra": "avg tps: 28.530358458421155, max tps: 28.83825378649162, count: 57203"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 138.57341161682152,
            "unit": "median tps",
            "extra": "avg tps: 137.7358465206127, max tps: 141.07341832498489, count: 57203"
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
          "id": "86bc2b092d2857a997cd05af4b554ac96ba225fd",
          "message": "fix: Fix `rescan` for top-n (#2868)\n\n## What\n\nComments in both `Normal` and `TopN` scans claimed that their search\nresults would be reset by `PdbScanState`, but this was incorrect: they\nwere local variables in the struct, and needed to be reset locally.\n\n## Why\n\nFixes issue detected by the prop tests and repro'd by @eeeebbbbrrrr in\n#2751.\n\n## Tests\n\nNew regression test from #2751: thanks @eeeebbbbrrrr!\n\n---------\n\nCo-authored-by: Eric B. Ridge <eebbrr@paradedb.com>",
          "timestamp": "2025-07-17T08:59:32-07:00",
          "tree_id": "ccc9162bb96008badcb3046612d16394604ed6f7",
          "url": "https://github.com/paradedb/paradedb/commit/86bc2b092d2857a997cd05af4b554ac96ba225fd"
        },
        "date": 1752770187038,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 27.35247002402735,
            "unit": "median tps",
            "extra": "avg tps: 27.427404596092366, max tps: 27.931419819075415, count: 56030"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 136.83059434639068,
            "unit": "median tps",
            "extra": "avg tps: 136.2806159099001, max tps: 138.73334834985792, count: 56030"
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
          "id": "5772be0d3dea751c3609729efcc66cc69d9d30ee",
          "message": "chore: Remove debug output in aggregate scan (#2876)\n\n## What\n\nRemove a stray `println`.\n\n## Why\n\nIt's not doing anyone any good.",
          "timestamp": "2025-07-17T09:46:08-07:00",
          "tree_id": "06a002186c2a701283d73854f64c4225a08aa1a8",
          "url": "https://github.com/paradedb/paradedb/commit/5772be0d3dea751c3609729efcc66cc69d9d30ee"
        },
        "date": 1752772983917,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 27.562956943979223,
            "unit": "median tps",
            "extra": "avg tps: 27.4248947817487, max tps: 27.732300954052057, count: 57758"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 128.0365126809508,
            "unit": "median tps",
            "extra": "avg tps: 127.43020980762667, max tps: 130.66972624057132, count: 57758"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "16307a200509cd31b42c1338f3c5b6e097a77fbf",
          "message": "fix: issue #2844 - Use `paradedb.with_index()` when necessary (#2879)\n\n# Ticket(s) Closed\n\n- Closes #2844\n\n## What\n\nWhen the right-hand-side of the `@@@` operator is a complex expression\nthat can't be evaluated to a `T_Const` during planning, we need to\nrewrite it to be a call to our `paradedb.with_index(<indexrelid>,\n<rhs>)` so that it can be properly used in Filter contexts during\nexecution.\n\nThis seems to come up, at least, during RLS rule evaluation if those\nrules use our `@@@` operator.\n\n## Why\n\nSo long as the right-hand-side of `@@@` is a `SearchQueryInput`, we need\nto be able to execute it when Postgres wants us to, whether that's\nduring a scan or, as in this case, a filter being run as part of RLS\nevaluation.\n\n## How\n\nWrapping Postgres expression nodes!\n\n## Tests\n\nI used the re-create the OP from #2844 put together as a regression\ntest. I seem to be lacking the creative energy today to reduce the test\ndown to its simplest form that doesn't require a complicated RLS\nconfiguration.",
          "timestamp": "2025-07-18T13:21:25-04:00",
          "tree_id": "f2e8c7ca2ce45eefd03b85ff45d56f0481ebae1a",
          "url": "https://github.com/paradedb/paradedb/commit/16307a200509cd31b42c1338f3c5b6e097a77fbf"
        },
        "date": 1752861486450,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 27.560602646610846,
            "unit": "median tps",
            "extra": "avg tps: 27.47694540024023, max tps: 27.863115748161295, count: 57764"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 135.72585042157178,
            "unit": "median tps",
            "extra": "avg tps: 134.99500063424355, max tps: 136.88170470308302, count: 57764"
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
          "id": "8b45b24d47a7cac36696bae2d8505c750a7ee801",
          "message": "test: added more join regression tests + improved when to use Join quals (#2808)\n\n# Ticket(s) Closed\n\n- Closes #2807\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-07-18T10:40:03-07:00",
          "tree_id": "fb63bd81696cd27d03b6d5724551c3ff8fd59613",
          "url": "https://github.com/paradedb/paradedb/commit/8b45b24d47a7cac36696bae2d8505c750a7ee801"
        },
        "date": 1752862607331,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 27.345083907530327,
            "unit": "median tps",
            "extra": "avg tps: 27.31875994755776, max tps: 27.6725178435038, count: 56580"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 129.5415322449111,
            "unit": "median tps",
            "extra": "avg tps: 128.96287903135322, max tps: 133.79855913063307, count: 56580"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "77c6cd1aeb5f5c03bba892a38cd792a113cfbd39",
          "message": "chore: move code around (#2889)\n\n## What\n\nThis relocates the `src/boostrap/create_bm25.rs` source to\n`src/api/admin.rs`, which I believe is a more sensibile location and\nname.\n\nAdditionally it moves the `#[pg_extern] schema()` function from\n`src/api/builder_fns.rs` to this new(ly renamed) `admin.rs`, as it's an\nadministrative function, not a query builder function.\n\n## Why\n\nTrying to better organize things\n\n## How\n\n## Tests\n\nNo functional changes here.",
          "timestamp": "2025-07-22T12:39:21-04:00",
          "tree_id": "1ab036752fc8adf7ad2757741d4e274ab0e73c64",
          "url": "https://github.com/paradedb/paradedb/commit/77c6cd1aeb5f5c03bba892a38cd792a113cfbd39"
        },
        "date": 1753204566572,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 29.00341934298043,
            "unit": "median tps",
            "extra": "avg tps: 28.816883910229965, max tps: 29.258784756170176, count: 57628"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 135.12259497023186,
            "unit": "median tps",
            "extra": "avg tps: 134.58698459138492, max tps: 136.55890154706543, count: 57628"
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
          "id": "1e26a38533fa872a935955128b2ecd08b3febf66",
          "message": "perf: Skip computing scores if not requested for top-n field queries (#2892)\n\n## What\n\n`SearchIndexReader::{top_by_field_in_segments,\ntop_by_string_field_in_segments}` were not using\n`SearchIndexReader::need_scores`, and were instead universally enabling\nscores.\n\nUse the `enable_scoring` helper and `self.need_scores` to skip computing\nscores where possible.\n\n## Why\n\nComputing scores requires field norms, which ([if they have not been\ndisabled](https://docs.paradedb.com/documentation/indexing/record)) can\ntake time to load and use.",
          "timestamp": "2025-07-22T15:58:56-07:00",
          "tree_id": "2a41c9e06be04ef92fed2b265dc3f90340fb0e37",
          "url": "https://github.com/paradedb/paradedb/commit/1e26a38533fa872a935955128b2ecd08b3febf66"
        },
        "date": 1753227345123,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 28.474239519577406,
            "unit": "median tps",
            "extra": "avg tps: 28.325083300341763, max tps: 28.645523483582853, count: 57209"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 142.73025596612604,
            "unit": "median tps",
            "extra": "avg tps: 141.8068853762286, max tps: 144.4662259408821, count: 57209"
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
          "id": "cbd2e802901988c181281a9b7f772543e1df32d2",
          "message": "chore: add `background-merge.toml` to CI (#2895)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nThis new stressgres file was instrumental in catching the bug fixed in\n#2845 and was used to test background merging, but has not been added to\nCI.\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-07-23T10:26:09-04:00",
          "tree_id": "bb8e3ca51c2d0de5e56be65f478f11096b0d410e",
          "url": "https://github.com/paradedb/paradedb/commit/cbd2e802901988c181281a9b7f772543e1df32d2"
        },
        "date": 1753282979801,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 27.083452531112716,
            "unit": "median tps",
            "extra": "avg tps: 27.145568441724137, max tps: 27.91271441171455, count: 57904"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 136.63278988451967,
            "unit": "median tps",
            "extra": "avg tps: 136.1532840704618, max tps: 140.4793490341162, count: 57904"
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
          "id": "53fa29a57e7daa2bdec9bf05f86d8db50088fd51",
          "message": "perf: Avoid a re-query when a top-n query is exhausted (#2888)\n\n## What\n\nSplit the iterators consumed by the `TopN` exec method and by the\n`Normal`/`FF` exec methods into two separate implementations.\n\nAfter splitting them, use the known exact size of the `TopN` iterator to\nexit early (as `exhausted: bool`) if we had fewer matches than were\nrequested. This avoids re-querying an iterator (and re-scanning its\ncolumns) that we know has no more results.\n\n## Why\n\nThe two types of consumers of these iterators have very different\nconsumption patterns and constraints:\n* TopN knows exactly how many results there will be, and already buffers\nthem all in memory. It would like to be able to know the precise count\nof results.\n* `Normal` and `FF` exec methods do not know the total number of results\nto expect, and in some cases would like to be able to consume in a\nsegment-aware fashion in order to be able to late-fetch fast field\ncolumns (see #2623).\n\n## Tests\n\nAdded a test to cement the change in query count.\n\nBenchmarks show a 1.5x speedup for `paging-string-max`, and no change\nfor other queries.",
          "timestamp": "2025-07-23T09:13:45-07:00",
          "tree_id": "c162e59fb9a6b4523c0ff39e10d9d54d92761460",
          "url": "https://github.com/paradedb/paradedb/commit/53fa29a57e7daa2bdec9bf05f86d8db50088fd51"
        },
        "date": 1753289439584,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 28.736037194702345,
            "unit": "median tps",
            "extra": "avg tps: 28.62591646989888, max tps: 28.898146120364817, count: 57616"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 138.83721228847457,
            "unit": "median tps",
            "extra": "avg tps: 138.22258104805857, max tps: 141.43269600911353, count: 57616"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "880ee607618b7ffe12f2784d03be3110b4df6cdf",
          "message": "feat: text search operators (#2877)\n\n## What\n\nThis is the implementation of 4 new operators designed around common\n\"text search\" patterns: `===` (exact token), `&&&` (tokenized term\nconjunction) `|||` (tokenized term disjunction), and `###` (phrase).\n\nFirst, `@@@` still exists and still does what it's always done, which is\nrun the right-hand-side through tantivy's \"query parser\".\n\nThese new operators have specific meanings. The spelling of these\noperators is both to maintain some kind of consistency in that they're\nall 3 of the same symbol and also to try and confer their meaning.\n\nIn all cases, the left-hand-side of the operator is required to be a\nreference to a Postgres `TEXT`/`VARCHAR` field and the right-hand-side\nis text. The operator definition defines how the right-hand-side input\nis applied to the left-hand-side field. These are \"text search\"\noperators and would be nonsensical applied to non-text fields.\n\n- `field === 'TEXT'` - this is the exact term match operator. The\nright-hand-side value is used, unmodified, to perform the equivalent of\nthe `paradedb.term()` function.\n\n- `field === ARRAY['...', '...', '...']` - similar to the above, but\nrewrites to the `paradedb.term_set()` function.\n\n- `field &&& 'TEXT'` - this is the \"match conjunction\" operator. It\ntokenizes the right-hand-side using the index tokenizer defined by the\nleft-hand-side field and searches for documents that contain **all** of\nthe tokenized terms. Think, \"find all documents that contain all terms\ntokenized from this text input\". This is the equivalent of the\n`paradedb.match` function with the `conjunction_mode => true`.\n\n- `field ||| 'TEXT'` - this is the \"match disjunction\" operator. It\ntokenizes the right-hand-side using the index tokenizer defined by the\nleft-hand-side field and searches for documents that contain **any** of\nthe tokenized terms. Think, \"find all documents that contain one or more\nof the terms tokenized from this text input\". This is the equivalent of\nthe `paradedb.match` function with the `conjunction_mode => false`.\n\n- `field ### 'TEXT'` - this is the \"phrase search\" operator. It\ntokenizes the right-hand-side using the index tokenizer defined by the\nleft-hand-side field and searches for documents that contain all the\ntokenized terms in the order written. Think, \"find all documents that\ncontain this tokenized phrase\". This is akin to the `paradedb.phrase()`\nfunction, except the `###` is transparently handling the tokenization.\n\nIf you wanted to find all documents that talk about bbq and chicken and\nmaybe hotdogs and happen on July 4th, you might write a query like this:\n\n```sql\nSELECT * \nFROM events \nWHERE (\n       description &&& 'bbq chicken' OR description ||| 'hot dog hotdogs'\n) AND description ### 'July 4th';\n```\n\n## Why\n\nThese operators are in furtherance of our\n[roadmap](https://github.com/orgs/paradedb/discussions/2041) --\nspecifically the \"Database UX improvements\" line item.\n\nOur overall goal with that roadmap item is to start moving users and ORM\ntools away from needing to use our \"builder functions\" in the common\ntext-search cases. What we're striving for is for users, and ORM tools,\nto write standard SQL WHERE clauses. In order to do that we need to make\ncommon things more accessible using custom operators.\n\n## How\n\nLargely just adding new `#[pg_operator]` functions and ensuring they all\nhave a SUPPORT function that can rewrite the expression to what we need.\n\n@rebasedming said he was going to tackle documentation, which can come\nin a separate PR.\n\n## Tests\n\nThere's a new regression test named `operators.sql` that validates\nEXPLAIN output and search results against our standard \"mock_items\"\ntable.",
          "timestamp": "2025-07-23T12:54:32-04:00",
          "tree_id": "f4a0dd94cd1e423e30c42103a8a23d86905d32e7",
          "url": "https://github.com/paradedb/paradedb/commit/880ee607618b7ffe12f2784d03be3110b4df6cdf"
        },
        "date": 1753291894389,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 27.773861153816227,
            "unit": "median tps",
            "extra": "avg tps: 27.807614320984662, max tps: 28.289003228445427, count: 57767"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 139.76140194669048,
            "unit": "median tps",
            "extra": "avg tps: 138.9593371066946, max tps: 140.84516682651764, count: 57767"
          }
        ]
      }
    ],
    "pg_search wide-table.toml Performance - Other Metrics": [
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
        "date": 1752442238335,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.677044,
            "unit": "median cpu",
            "extra": "avg cpu: 20.70271093751442, max cpu: 50.040096, count: 56545"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 175.515625,
            "unit": "median mem",
            "extra": "avg mem: 173.69064316860465, max mem: 178.2890625, count: 56545"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 15883,
            "unit": "median block_count",
            "extra": "avg block_count: 14488.321425413387, max block_count: 15883.0, count: 56545"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 82,
            "unit": "median segment_count",
            "extra": "avg segment_count: 84.01496153506056, max segment_count: 157.0, count: 56545"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 13.872832,
            "unit": "median cpu",
            "extra": "avg cpu: 12.100274904992668, max cpu: 32.55814, count: 56545"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 163.8671875,
            "unit": "median mem",
            "extra": "avg mem: 151.70462815843575, max mem: 173.2109375, count: 56545"
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
        "date": 1752442255077,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.121387,
            "unit": "median cpu",
            "extra": "avg cpu: 21.703221802642826, max cpu: 47.524754, count: 57332"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 174.19921875,
            "unit": "median mem",
            "extra": "avg mem: 171.5259118489674, max mem: 176.06640625, count: 57332"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 8052,
            "unit": "median block_count",
            "extra": "avg block_count: 7368.411585153143, max block_count: 8455.0, count: 57332"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 76,
            "unit": "median segment_count",
            "extra": "avg segment_count: 76.57784483360078, max segment_count: 118.0, count: 57332"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 13.980582,
            "unit": "median cpu",
            "extra": "avg cpu: 14.923658087833578, max cpu: 33.168808, count: 57332"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 165.203125,
            "unit": "median mem",
            "extra": "avg mem: 155.11725536131655, max mem: 170.78125, count: 57332"
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
        "date": 1752442256394,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.032629,
            "unit": "median cpu",
            "extra": "avg cpu: 21.406439580149527, max cpu: 47.33728, count: 57542"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 172.91796875,
            "unit": "median mem",
            "extra": "avg mem: 170.4589773828247, max mem: 176.57421875, count: 57542"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 8062,
            "unit": "median block_count",
            "extra": "avg block_count: 7328.47648674012, max block_count: 8159.0, count: 57542"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 76,
            "unit": "median segment_count",
            "extra": "avg segment_count: 76.71116749504709, max segment_count: 112.0, count: 57542"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 13.980582,
            "unit": "median cpu",
            "extra": "avg cpu: 15.232635930009609, max cpu: 38.155804, count: 57542"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 168.55078125,
            "unit": "median mem",
            "extra": "avg mem: 160.07558202052414, max mem: 178.06640625, count: 57542"
          }
        ]
      },
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
          "id": "4fd1b2b6b6664d03946be0f4836732f0f40df0cc",
          "message": "chore: Rename datasets and add string paging queries (#2834)\n\n## What\n\nAdd a high-cardinality paging/top-n query to the benchmarks, and rename\ndatasets to match their content. Additionally, improve the generation\nscript for the `docs` dataset to avoid joins and allow for deterministic\nrelative-position queries.\n\n## Why\n\nWe don't currently have a high-cardinality string paging/top-n query in\nthe benchmark. We have top-n on a string column, but only for\nlow-cardinality values (`top_n-string.sql`). The top-n case represented\nan important gap that a user encountered, which #2828 addresses.\n\nThe names of the `benchmark` datasets don't currently describe their\nshape / schema, and for the `join` dataset in particular, that would\ndiscourage using it for other types of queries. We rename it to `docs`\nhere, and then use the `pages` table as the dataset for top-n.\n\n## Tests\n\nTested locally that the new query demonstrates a speedup for #2828.",
          "timestamp": "2025-07-13T18:04:27Z",
          "url": "https://github.com/paradedb/paradedb/commit/4fd1b2b6b6664d03946be0f4836732f0f40df0cc"
        },
        "date": 1752442318866,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.622696,
            "unit": "median cpu",
            "extra": "avg cpu: 20.628861161282483, max cpu: 60.057747, count: 56451"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 176.18359375,
            "unit": "median mem",
            "extra": "avg mem: 174.40861093857947, max mem: 179.92578125, count: 56451"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 18715,
            "unit": "median block_count",
            "extra": "avg block_count: 17392.997023967688, max block_count: 22354.0, count: 56451"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 82,
            "unit": "median segment_count",
            "extra": "avg segment_count: 84.31962232732812, max segment_count: 158.0, count: 56451"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 13.846154,
            "unit": "median cpu",
            "extra": "avg cpu: 12.306341788287172, max cpu: 32.40116, count: 56451"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 161.2109375,
            "unit": "median mem",
            "extra": "avg mem: 151.64812950335246, max mem: 170.046875, count: 56451"
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
        "date": 1752442323005,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.658894,
            "unit": "median cpu",
            "extra": "avg cpu: 20.626631624411086, max cpu: 60.40658, count: 56530"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 174.6328125,
            "unit": "median mem",
            "extra": "avg mem: 172.8077671009862, max mem: 177.515625, count: 56530"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 18867,
            "unit": "median block_count",
            "extra": "avg block_count: 17546.706368300016, max block_count: 22568.0, count: 56530"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 82,
            "unit": "median segment_count",
            "extra": "avg segment_count: 84.06626569962852, max segment_count: 158.0, count: 56530"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 13.88621,
            "unit": "median cpu",
            "extra": "avg cpu: 12.246453096152907, max cpu: 32.589718, count: 56530"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 162.87109375,
            "unit": "median mem",
            "extra": "avg mem: 152.06358024776668, max mem: 172.01171875, count: 56530"
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
        "date": 1752442354982,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.677044,
            "unit": "median cpu",
            "extra": "avg cpu: 20.661374814177453, max cpu: 60.231655, count: 56533"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 173.8046875,
            "unit": "median mem",
            "extra": "avg mem: 171.95992398798046, max mem: 176.99609375, count: 56533"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 16943,
            "unit": "median block_count",
            "extra": "avg block_count: 15337.339200113209, max block_count: 16943.0, count: 56533"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 82,
            "unit": "median segment_count",
            "extra": "avg segment_count: 84.21332672952082, max segment_count: 173.0, count: 56533"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 13.859479,
            "unit": "median cpu",
            "extra": "avg cpu: 12.052747863190515, max cpu: 38.476955, count: 56533"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 163.87109375,
            "unit": "median mem",
            "extra": "avg mem: 152.14649670878072, max mem: 171.91796875, count: 56533"
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
          "id": "47bbe518381e1429f228328336dad78e99636ad9",
          "message": "chore: Upgrade to `0.16.0` (#2720)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-06-23T23:04:37Z",
          "url": "https://github.com/paradedb/paradedb/commit/47bbe518381e1429f228328336dad78e99636ad9"
        },
        "date": 1752442359744,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.121387,
            "unit": "median cpu",
            "extra": "avg cpu: 21.801249827819134, max cpu: 57.657658, count: 57290"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 180.50390625,
            "unit": "median mem",
            "extra": "avg mem: 179.34167564420056, max mem: 185.04296875, count: 57290"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 7747,
            "unit": "median block_count",
            "extra": "avg block_count: 7107.179612497818, max block_count: 7988.0, count: 57290"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 76,
            "unit": "median segment_count",
            "extra": "avg segment_count: 75.89303543375807, max segment_count: 110.0, count: 57290"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 13.967022,
            "unit": "median cpu",
            "extra": "avg cpu: 15.36213823865593, max cpu: 48.144432, count: 57290"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 164.4921875,
            "unit": "median mem",
            "extra": "avg mem: 155.0147005640164, max mem: 170.80859375, count: 57290"
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
          "id": "b96e41e25c3fd2126f54aa5cb66d4deffb482047",
          "message": "perf: Lazily load fast fields dictionaries. (#2842)\n\n## What\n\nLazily load fast field dictionaries from buffers: see\nhttps://github.com/paradedb/tantivy/pull/55\n\n## Why\n\nA customer reported slower-than-expected paging on a string/uuid column.\n85% of the time for that query was being spent in _opening_ a fast\nfields string/bytes column, with a large fraction of that time spent\nfully consuming the column's `Dictionary`.\n\n## Tests\n\nSee the attached benchmark results:\n* [`docs`\ndataset](https://github.com/paradedb/paradedb/pull/2842#pullrequestreview-3014379545)\n    * No regressions.\n    * 2x faster for `top_n-score`\n    * 1.4x faster for `highlighting` \n* [`logs`\ndataset](https://github.com/paradedb/paradedb/pull/2842#pullrequestreview-3014350211)\n    * No regressions.\n    * 4.5x faster for `paging-string-max`\n    * 1.7x faster for `paging-string-median`\n    * 1.6x faster for `paging-string-min`\n\nThe `paging-string-*` benchmarks were added in #2834 to highlight this\nparticular issue.",
          "timestamp": "2025-07-14T08:28:09-07:00",
          "tree_id": "d144335dcb7c7f138a112c01e5b9ff5e0168fe37",
          "url": "https://github.com/paradedb/paradedb/commit/b96e41e25c3fd2126f54aa5cb66d4deffb482047"
        },
        "date": 1752509171375,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.640776,
            "unit": "median cpu",
            "extra": "avg cpu: 20.53277628126518, max cpu: 47.244095, count: 57172"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 177.72265625,
            "unit": "median mem",
            "extra": "avg mem: 175.90110147340482, max mem: 181.08984375, count: 57172"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 19214,
            "unit": "median block_count",
            "extra": "avg block_count: 17846.88237249003, max block_count: 23111.0, count: 57172"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 82,
            "unit": "median segment_count",
            "extra": "avg segment_count: 84.50839571818372, max segment_count: 159.0, count: 57172"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 9.329447,
            "unit": "median cpu",
            "extra": "avg cpu: 11.27677413196564, max cpu: 28.543112, count: 57172"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 160.94921875,
            "unit": "median mem",
            "extra": "avg mem: 150.42495916926117, max mem: 171.2109375, count: 57172"
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
          "id": "ee6395b4b4d4ca6f44e2c89b74afd2308d4415a8",
          "message": "fix: orphaned delete entries get GCed too early (#2845)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nWhen running a new stressgres suite (coming in a future PR), I hit a\nmysterious bug where it looked like vacuum could cause corruption of\nsome pages.\n\nTurns out it's caused by scenarios where:\n\n1. A `DeleteEntry` already exists for a `SegmentMetaEntry`, and a new\none is created\n2. A new, \"fake\" `SegmentMetaEntry` gets created for the purpose of\nstoring the old `DeleteEntry`, so its blocks can get garbage collected\n3. Because this \"fake\" entry is invisible to all readers besides the\ngarbage collector, it doesn't get pinned and can get garbage collected\ntoo early (i.e. while a reader is still pinning the old `DeleteEntry`)\n\nThe solution is to copy all of the contents of the old\n`SegmentMetaEntry` to the fake one, so that the \"pintest blockno\" of the\nfake entry is that same as that of the entry with the new `DeleteEntry`.\nThat way, the `DeleteEntry` doesn't get garbage collected until the pin\nis released.\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-07-14T15:46:29-04:00",
          "tree_id": "3dc55f49de121cf04534f48e3584a2a3ae333407",
          "url": "https://github.com/paradedb/paradedb/commit/ee6395b4b4d4ca6f44e2c89b74afd2308d4415a8"
        },
        "date": 1752524578321,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.658894,
            "unit": "median cpu",
            "extra": "avg cpu: 20.707788917693303, max cpu: 46.73807, count: 56467"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 176.97265625,
            "unit": "median mem",
            "extra": "avg mem: 175.25031683328316, max mem: 180.6640625, count: 56467"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 18570,
            "unit": "median block_count",
            "extra": "avg block_count: 17403.396497069083, max block_count: 22700.0, count: 56467"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 82,
            "unit": "median segment_count",
            "extra": "avg segment_count: 83.90233233570049, max segment_count: 157.0, count: 56467"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 13.859479,
            "unit": "median cpu",
            "extra": "avg cpu: 12.130283500574265, max cpu: 37.10145, count: 56467"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 164.6640625,
            "unit": "median mem",
            "extra": "avg mem: 154.4701904845972, max mem: 173.96875, count: 56467"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "eb3bc1d570c85d60804f6aab2f2d2cf998bd7597",
          "message": "ci: benchmark workflow cleanups (#2851)\n\nThis is an attempt to cleanup the benchmark workflows a little bit.  \n\n- Centralizes checking out the latest benchmark code/suites/actions into\na composite action.\n- figures out the PR #/title being tested\n- Changes the slack notification messages to be reactive to the\nenvironment to hopefully avoid conflicts with -enterprise",
          "timestamp": "2025-07-15T12:15:54-04:00",
          "tree_id": "223c726790d68868f538b7f5aab9cf9904494f44",
          "url": "https://github.com/paradedb/paradedb/commit/eb3bc1d570c85d60804f6aab2f2d2cf998bd7597"
        },
        "date": 1752598341672,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.658894,
            "unit": "median cpu",
            "extra": "avg cpu: 20.48614730239338, max cpu: 47.38401, count: 57164"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 172.23828125,
            "unit": "median mem",
            "extra": "avg mem: 170.63902523058655, max mem: 176.1953125, count: 57164"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 19360,
            "unit": "median block_count",
            "extra": "avg block_count: 17934.165961094393, max block_count: 23063.0, count: 57164"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 82,
            "unit": "median segment_count",
            "extra": "avg segment_count: 84.53789098033728, max segment_count: 159.0, count: 57164"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 9.375,
            "unit": "median cpu",
            "extra": "avg cpu: 11.45333314504253, max cpu: 32.74854, count: 57164"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 164.13671875,
            "unit": "median mem",
            "extra": "avg mem: 153.75233415589358, max mem: 173.49609375, count: 57164"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "c7bdeebed6249725c87b87c276d2e09dfaffd152",
          "message": "ci: publish stressgres benchmark graphs (#2852)\n\nThis will publish every stressgres benchmark graph to the new\n`paradedb/benchmark-data` repo.\n\nIt also ensures that when one of our \"[benchmark]\" labels are applied,\nthat it does *not* pull the benchmarks from main, but instead from the\nPR branch itself.",
          "timestamp": "2025-07-15T18:13:26-04:00",
          "tree_id": "f18bf519521a03613c5b12c861092f24314476ad",
          "url": "https://github.com/paradedb/paradedb/commit/c7bdeebed6249725c87b87c276d2e09dfaffd152"
        },
        "date": 1752619795165,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.640776,
            "unit": "median cpu",
            "extra": "avg cpu: 20.626298320818297, max cpu: 67.267265, count: 56424"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 169.75,
            "unit": "median mem",
            "extra": "avg mem: 168.88672027306643, max mem: 176.5859375, count: 56424"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 19612,
            "unit": "median block_count",
            "extra": "avg block_count: 17942.553328370905, max block_count: 22538.0, count: 56424"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 82,
            "unit": "median segment_count",
            "extra": "avg segment_count: 83.916950233943, max segment_count: 180.0, count: 56424"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 13.859479,
            "unit": "median cpu",
            "extra": "avg cpu: 12.230637444066675, max cpu: 37.029896, count: 56424"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 164.42578125,
            "unit": "median mem",
            "extra": "avg mem: 154.04628335792393, max mem: 173.91796875, count: 56424"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Philippe Noël",
            "username": "philippemnoel",
            "email": "21990816+philippemnoel@users.noreply.github.com"
          },
          "committer": {
            "name": "Stu Hood",
            "username": "stuhood",
            "email": "stuhood@gmail.com"
          },
          "id": "8c164d666c4af2049b439690afa7823ab5be2c88",
          "message": "ci: Post \"One-branch Release Model\" Improvements (#2792)\n\n# Ticket(s) Closed\n\n- Closes #N/A\n\n## What\n- Don't make Docker Scout post on PRs that don't modify the Dockerfile,\nnot necessary.\n- Make sure the SchemaBot workflow errors when community contributors\nmake a PR, so we can catch needed changes to the SQL upgrade script.\n- Rename it to SchemaBot\n\n## Why\nQoL\n\n## How\n^\n\n## Tests\nCI",
          "timestamp": "2025-07-08T15:49:16Z",
          "url": "https://github.com/paradedb/paradedb/commit/8c164d666c4af2049b439690afa7823ab5be2c88"
        },
        "date": 1752669772361,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.622696,
            "unit": "median cpu",
            "extra": "avg cpu: 20.221248004004604, max cpu: 42.772278, count: 57536"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 176.4609375,
            "unit": "median mem",
            "extra": "avg mem: 174.66007056994752, max mem: 180.25390625, count: 57536"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 17079,
            "unit": "median block_count",
            "extra": "avg block_count: 15485.6257647386, max block_count: 17079.0, count: 57536"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 82,
            "unit": "median segment_count",
            "extra": "avg segment_count: 84.74428184093438, max segment_count: 175.0, count: 57536"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 9.338522,
            "unit": "median cpu",
            "extra": "avg cpu: 11.416365678004572, max cpu: 32.621357, count: 57536"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 163.359375,
            "unit": "median mem",
            "extra": "avg mem: 154.2193589258247, max mem: 174.5, count: 57536"
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
          "id": "f7c13c858851200e8ab5395779f821ca484cda0d",
          "message": "feat: Add a custom scan for aggregates (#2763)\n\n## What\n\nAdd a new `CustomScan` (installed with `create_upper_paths_hook`) which\nreplaces simple aggregate plans on `bm25`-indexed tables with aggregate\nimplementations using [Tantivy\naggregates](https://docs.rs/tantivy/latest/tantivy/aggregation/index.html).\n\n## Why\n\nTantivy aggregates can be significantly faster (in benchmarks, we've\nmeasured between 4-10x for bucketing/faceting queries). They have been\nexposed via `paradedb.aggregate` for a while now, but that function\nrequires learning a new API, and does not feel \"Postgres native\".\n\n## How\n\n* Adjust `CustomPathBuilder` and `CustomPathMethods` to allow multiple\n`CustomScan` implementations.\n* Remove the `CustomScan::PrivateData: Default` bound, as it requires\nthe `PrivateData` to start in an illegal state.\n* Move `qual_inspect` to a reusable location.\n* Split out a module to be used by both the `aggregate` API method and\nby the aggregate custom scan.\n* Implement the \"ParadeDB Aggregate Scan\" `CustomScan` type\n    * Add the new `CustomScan` type, hidden behind a GUC\n    * Filter Paths to those which represent `count(*)` queries\n    * Extract `quals` during `CustomPath` generation\n* Replace `Aggrefs` in target lists with `FuncExprs` while producing a\n`CustomPlan`\n* Execute a `count(*)` aggregate by pushing down a `value_count`\naggregate on the `ctid`\n\n## Tests\n\nAdded tests to validate that:\n* the GUC properly controls usage\n* the scan does not trigger for unsupported aggregates, tables without a\n`bm25` index, or group-bys (for now)",
          "timestamp": "2025-07-16T09:12:24-07:00",
          "tree_id": "69b043a9363fcf6ce2de468c97d14e41f593f017",
          "url": "https://github.com/paradedb/paradedb/commit/f7c13c858851200e8ab5395779f821ca484cda0d"
        },
        "date": 1752684638185,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.79405,
            "unit": "median cpu",
            "extra": "avg cpu: 20.97632553576398, max cpu: 61.356934, count: 57963"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 169.86328125,
            "unit": "median mem",
            "extra": "avg mem: 168.0544283772622, max mem: 173.28515625, count: 57963"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 18749,
            "unit": "median block_count",
            "extra": "avg block_count: 17595.505581146594, max block_count: 23003.0, count: 57963"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 82,
            "unit": "median segment_count",
            "extra": "avg segment_count: 83.95509204147473, max segment_count: 163.0, count: 57963"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 13.846154,
            "unit": "median cpu",
            "extra": "avg cpu: 12.351628186312494, max cpu: 41.941746, count: 57963"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 161.3125,
            "unit": "median mem",
            "extra": "avg mem: 152.38926610510154, max mem: 173.359375, count: 57963"
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
          "id": "c779dbb72178c1aa0e14ad94c72eeb9937251acd",
          "message": "feat: a background merger for large layers (#2743)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nLaunches Postgres [dynamic background\nworkers](https://docs.rs/pgrx/latest/pgrx/bgworkers/struct.DynamicBackgroundWorker.html)\nto merge large layers in the background.\n\nThis is configured by a new index setting, `background_layer_sizes`. Any\nlayers specified here will be merged in the background.\n\n## Why\n\nWrite throughput, and create more balanced indexes with fewer segments.\n\n## How\n\n1. The default (foreground) `layer_sizes` is now `10kb`, `100kb`, `1mb`\n2. The default `background_layer_sizes` is `10mb`, `100mb`, `1gb`,\n`10gb`, `100gb`, `1tb`\n3. After `aminsert` merges in the foreground, it checks to see if there\nare enough candidates for a background merge. If so, it spawns a dynamic\nbackground worker process to do the merge.\n4. `amvacuumcleanup` can also spawn a background worker, allowing the\nuser to rebalance their index with a `VACUUM`.\n\nAdditionally, I've tweaked merge policy to make it smarter:\n\n1. Terminate a merge early if we predict that, after the merge, we will\nend up with fewer than `target_segment_count` segments.\n2. Set the max layer size as byte size of index divided by\n`target_segment_count`. Discard all layer sizes above that, which avoids\nthe problem of merging too many segments into one giant segment.\n\nThis has allowed us to delete some code:\n\n1. `force_merge` is deprecated\n2. No longer need to store the segments created by an index build and\navoid merging them, since we just do expensive merges in the background\n\n## Tests\n\nAdded a new stressgres `.toml` file that configures the layer\nsizes/threshold, which triggers background merging.\n\n---------\n\nSigned-off-by: Ming <ming.ying.nyc@gmail.com>\nCo-authored-by: Stu Hood <stuhood@paradedb.com>\nCo-authored-by: Philippe Noël <21990816+philippemnoel@users.noreply.github.com>\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>",
          "timestamp": "2025-07-16T12:44:23-04:00",
          "tree_id": "153c406e456a638a6b68ca9123210bbd498d66a5",
          "url": "https://github.com/paradedb/paradedb/commit/c779dbb72178c1aa0e14ad94c72eeb9937251acd"
        },
        "date": 1752686565252,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.622696,
            "unit": "median cpu",
            "extra": "avg cpu: 20.5796680185914, max cpu: 47.33728, count: 57220"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 167.76171875,
            "unit": "median mem",
            "extra": "avg mem: 165.94558263336683, max mem: 169.453125, count: 57220"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 17964,
            "unit": "median block_count",
            "extra": "avg block_count: 16642.27936036351, max block_count: 22042.0, count: 57220"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 40,
            "unit": "median segment_count",
            "extra": "avg segment_count: 42.0743446347431, max segment_count: 114.0, count: 57220"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 9.320388,
            "unit": "median cpu",
            "extra": "avg cpu: 11.293786515610682, max cpu: 33.03835, count: 57220"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 161.22265625,
            "unit": "median mem",
            "extra": "avg mem: 151.48477538721164, max mem: 172.06640625, count: 57220"
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
          "id": "d197630155f55d13eea0a81841951645a367a5f7",
          "message": "test: Make GUCs a tested property, and add proptests for the aggregate custom scan (#2788)\n\n## What\n\nMove the GUCs that are in use into a test property, which reduces\nruntime while allowing for testing arbitrary combinations of GUCs.\n\nAdditionally, added proptests for the aggregate custom scan by:\n* Adding its GUC to the generated set\n* Expanding the `generated_single_relation` test to use `COUNT(*)` in\nits target list.\n\n## Why\n\nTo add additional test coverage for #2763.",
          "timestamp": "2025-07-16T10:57:16-07:00",
          "tree_id": "6a92b62b640f7a0b2dc7e6507966b75f3bd00594",
          "url": "https://github.com/paradedb/paradedb/commit/d197630155f55d13eea0a81841951645a367a5f7"
        },
        "date": 1752690841092,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.677044,
            "unit": "median cpu",
            "extra": "avg cpu: 20.530923451615596, max cpu: 57.54246, count: 57786"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 170.18359375,
            "unit": "median mem",
            "extra": "avg mem: 169.6581188972026, max mem: 178.15625, count: 57786"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 18423,
            "unit": "median block_count",
            "extra": "avg block_count: 16914.80130135327, max block_count: 22032.0, count: 57786"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 40,
            "unit": "median segment_count",
            "extra": "avg segment_count: 42.24062921814973, max segment_count: 118.0, count: 57786"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 9.347614,
            "unit": "median cpu",
            "extra": "avg cpu: 11.312314796563273, max cpu: 33.005894, count: 57786"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 163.7890625,
            "unit": "median mem",
            "extra": "avg mem: 153.97929057481483, max mem: 173.984375, count: 57786"
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
          "id": "58f6742b2cde46fe2c15077e59a0d5b091ba6000",
          "message": "chore: Upgrade to `0.17.0` (#2870)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-07-16T19:49:53-04:00",
          "tree_id": "b77118d365b31a4f3478b7c0571cf1f7e9f8a5b2",
          "url": "https://github.com/paradedb/paradedb/commit/58f6742b2cde46fe2c15077e59a0d5b091ba6000"
        },
        "date": 1752712096174,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.658894,
            "unit": "median cpu",
            "extra": "avg cpu: 20.513549248993094, max cpu: 46.28737, count: 57203"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 175.78125,
            "unit": "median mem",
            "extra": "avg mem: 174.00660702072005, max mem: 178.7578125, count: 57203"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 18116,
            "unit": "median block_count",
            "extra": "avg block_count: 16773.306207716378, max block_count: 22230.0, count: 57203"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 41,
            "unit": "median segment_count",
            "extra": "avg segment_count: 42.76789678862997, max segment_count: 118.0, count: 57203"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 9.356726,
            "unit": "median cpu",
            "extra": "avg cpu: 11.390439207145732, max cpu: 33.168808, count: 57203"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 168.7421875,
            "unit": "median mem",
            "extra": "avg mem: 157.04750126468454, max mem: 177.94921875, count: 57203"
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
          "id": "86bc2b092d2857a997cd05af4b554ac96ba225fd",
          "message": "fix: Fix `rescan` for top-n (#2868)\n\n## What\n\nComments in both `Normal` and `TopN` scans claimed that their search\nresults would be reset by `PdbScanState`, but this was incorrect: they\nwere local variables in the struct, and needed to be reset locally.\n\n## Why\n\nFixes issue detected by the prop tests and repro'd by @eeeebbbbrrrr in\n#2751.\n\n## Tests\n\nNew regression test from #2751: thanks @eeeebbbbrrrr!\n\n---------\n\nCo-authored-by: Eric B. Ridge <eebbrr@paradedb.com>",
          "timestamp": "2025-07-17T08:59:32-07:00",
          "tree_id": "ccc9162bb96008badcb3046612d16394604ed6f7",
          "url": "https://github.com/paradedb/paradedb/commit/86bc2b092d2857a997cd05af4b554ac96ba225fd"
        },
        "date": 1752770188966,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.768328,
            "unit": "median cpu",
            "extra": "avg cpu: 20.97580878297552, max cpu: 66.60059, count: 56030"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 167.58203125,
            "unit": "median mem",
            "extra": "avg mem: 165.88912120181152, max mem: 171.2734375, count: 56030"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 17658,
            "unit": "median block_count",
            "extra": "avg block_count: 16538.45862930573, max block_count: 22216.0, count: 56030"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 40,
            "unit": "median segment_count",
            "extra": "avg segment_count: 42.33733714081742, max segment_count: 115.0, count: 56030"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 13.766731,
            "unit": "median cpu",
            "extra": "avg cpu: 11.957664603406469, max cpu: 42.814667, count: 56030"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 162.15625,
            "unit": "median mem",
            "extra": "avg mem: 151.7431514785606, max mem: 169.23828125, count: 56030"
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
          "id": "5772be0d3dea751c3609729efcc66cc69d9d30ee",
          "message": "chore: Remove debug output in aggregate scan (#2876)\n\n## What\n\nRemove a stray `println`.\n\n## Why\n\nIt's not doing anyone any good.",
          "timestamp": "2025-07-17T09:46:08-07:00",
          "tree_id": "06a002186c2a701283d73854f64c4225a08aa1a8",
          "url": "https://github.com/paradedb/paradedb/commit/5772be0d3dea751c3609729efcc66cc69d9d30ee"
        },
        "date": 1752772986002,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.677044,
            "unit": "median cpu",
            "extra": "avg cpu: 20.721910584507285, max cpu: 47.244095, count: 57758"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 179.546875,
            "unit": "median mem",
            "extra": "avg mem: 177.59026766313758, max mem: 182.10546875, count: 57758"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 18304,
            "unit": "median block_count",
            "extra": "avg block_count: 16760.28768309152, max block_count: 21421.0, count: 57758"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 40,
            "unit": "median segment_count",
            "extra": "avg segment_count: 41.62498701478583, max segment_count: 119.0, count: 57758"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 13.872832,
            "unit": "median cpu",
            "extra": "avg cpu: 12.346812127906308, max cpu: 37.83251, count: 57758"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 165.8671875,
            "unit": "median mem",
            "extra": "avg mem: 154.91160471006182, max mem: 175.22265625, count: 57758"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "16307a200509cd31b42c1338f3c5b6e097a77fbf",
          "message": "fix: issue #2844 - Use `paradedb.with_index()` when necessary (#2879)\n\n# Ticket(s) Closed\n\n- Closes #2844\n\n## What\n\nWhen the right-hand-side of the `@@@` operator is a complex expression\nthat can't be evaluated to a `T_Const` during planning, we need to\nrewrite it to be a call to our `paradedb.with_index(<indexrelid>,\n<rhs>)` so that it can be properly used in Filter contexts during\nexecution.\n\nThis seems to come up, at least, during RLS rule evaluation if those\nrules use our `@@@` operator.\n\n## Why\n\nSo long as the right-hand-side of `@@@` is a `SearchQueryInput`, we need\nto be able to execute it when Postgres wants us to, whether that's\nduring a scan or, as in this case, a filter being run as part of RLS\nevaluation.\n\n## How\n\nWrapping Postgres expression nodes!\n\n## Tests\n\nI used the re-create the OP from #2844 put together as a regression\ntest. I seem to be lacking the creative energy today to reduce the test\ndown to its simplest form that doesn't require a complicated RLS\nconfiguration.",
          "timestamp": "2025-07-18T13:21:25-04:00",
          "tree_id": "f2e8c7ca2ce45eefd03b85ff45d56f0481ebae1a",
          "url": "https://github.com/paradedb/paradedb/commit/16307a200509cd31b42c1338f3c5b6e097a77fbf"
        },
        "date": 1752861488451,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.695229,
            "unit": "median cpu",
            "extra": "avg cpu: 20.731253800335843, max cpu: 65.912766, count: 57764"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 170.5859375,
            "unit": "median mem",
            "extra": "avg mem: 168.71374478751903, max mem: 172.24609375, count: 57764"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 18016,
            "unit": "median block_count",
            "extra": "avg block_count: 16652.91371788657, max block_count: 21821.0, count: 57764"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 40,
            "unit": "median segment_count",
            "extra": "avg segment_count: 41.74617408766706, max segment_count: 115.0, count: 57764"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 11.631664,
            "unit": "median cpu",
            "extra": "avg cpu: 11.849846631596078, max cpu: 34.894993, count: 57764"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 159.54296875,
            "unit": "median mem",
            "extra": "avg mem: 149.21988101639863, max mem: 167.453125, count: 57764"
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
          "id": "8b45b24d47a7cac36696bae2d8505c750a7ee801",
          "message": "test: added more join regression tests + improved when to use Join quals (#2808)\n\n# Ticket(s) Closed\n\n- Closes #2807\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-07-18T10:40:03-07:00",
          "tree_id": "fb63bd81696cd27d03b6d5724551c3ff8fd59613",
          "url": "https://github.com/paradedb/paradedb/commit/8b45b24d47a7cac36696bae2d8505c750a7ee801"
        },
        "date": 1752862609299,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.640776,
            "unit": "median cpu",
            "extra": "avg cpu: 20.705744208512268, max cpu: 57.715435, count: 56580"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 174.62890625,
            "unit": "median mem",
            "extra": "avg mem: 173.04714901743108, max mem: 177.89453125, count: 56580"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 17987,
            "unit": "median block_count",
            "extra": "avg block_count: 16513.99434429127, max block_count: 21331.0, count: 56580"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 40,
            "unit": "median segment_count",
            "extra": "avg segment_count: 42.02820784729587, max segment_count: 122.0, count: 56580"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 13.846154,
            "unit": "median cpu",
            "extra": "avg cpu: 12.145959377241704, max cpu: 33.667336, count: 56580"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 167.015625,
            "unit": "median mem",
            "extra": "avg mem: 156.39145547510162, max mem: 175.33984375, count: 56580"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "77c6cd1aeb5f5c03bba892a38cd792a113cfbd39",
          "message": "chore: move code around (#2889)\n\n## What\n\nThis relocates the `src/boostrap/create_bm25.rs` source to\n`src/api/admin.rs`, which I believe is a more sensibile location and\nname.\n\nAdditionally it moves the `#[pg_extern] schema()` function from\n`src/api/builder_fns.rs` to this new(ly renamed) `admin.rs`, as it's an\nadministrative function, not a query builder function.\n\n## Why\n\nTrying to better organize things\n\n## How\n\n## Tests\n\nNo functional changes here.",
          "timestamp": "2025-07-22T12:39:21-04:00",
          "tree_id": "1ab036752fc8adf7ad2757741d4e274ab0e73c64",
          "url": "https://github.com/paradedb/paradedb/commit/77c6cd1aeb5f5c03bba892a38cd792a113cfbd39"
        },
        "date": 1753204568735,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.640776,
            "unit": "median cpu",
            "extra": "avg cpu: 20.330572039900332, max cpu: 42.772278, count: 57628"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 178.96484375,
            "unit": "median mem",
            "extra": "avg mem: 177.09268491173995, max mem: 182.21484375, count: 57628"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 18177,
            "unit": "median block_count",
            "extra": "avg block_count: 16885.226521829667, max block_count: 22254.0, count: 57628"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 41,
            "unit": "median segment_count",
            "extra": "avg segment_count: 42.484920524744915, max segment_count: 116.0, count: 57628"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 11.258796,
            "unit": "median cpu",
            "extra": "avg cpu: 11.851710835118219, max cpu: 33.20158, count: 57628"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 166.94921875,
            "unit": "median mem",
            "extra": "avg mem: 156.93496366241237, max mem: 175.86328125, count: 57628"
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
          "id": "1e26a38533fa872a935955128b2ecd08b3febf66",
          "message": "perf: Skip computing scores if not requested for top-n field queries (#2892)\n\n## What\n\n`SearchIndexReader::{top_by_field_in_segments,\ntop_by_string_field_in_segments}` were not using\n`SearchIndexReader::need_scores`, and were instead universally enabling\nscores.\n\nUse the `enable_scoring` helper and `self.need_scores` to skip computing\nscores where possible.\n\n## Why\n\nComputing scores requires field norms, which ([if they have not been\ndisabled](https://docs.paradedb.com/documentation/indexing/record)) can\ntake time to load and use.",
          "timestamp": "2025-07-22T15:58:56-07:00",
          "tree_id": "2a41c9e06be04ef92fed2b265dc3f90340fb0e37",
          "url": "https://github.com/paradedb/paradedb/commit/1e26a38533fa872a935955128b2ecd08b3febf66"
        },
        "date": 1753227347192,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.658894,
            "unit": "median cpu",
            "extra": "avg cpu: 20.56424557360741, max cpu: 55.598457, count: 57209"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 171.7421875,
            "unit": "median mem",
            "extra": "avg mem: 170.5463334003391, max mem: 179.29296875, count: 57209"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 18133,
            "unit": "median block_count",
            "extra": "avg block_count: 16843.69961020119, max block_count: 21977.0, count: 57209"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 40,
            "unit": "median segment_count",
            "extra": "avg segment_count: 42.14649792864759, max segment_count: 116.0, count: 57209"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 9.329447,
            "unit": "median cpu",
            "extra": "avg cpu: 11.18349534168653, max cpu: 37.907207, count: 57209"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 159.7109375,
            "unit": "median mem",
            "extra": "avg mem: 149.18621523711303, max mem: 167.15625, count: 57209"
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
          "id": "cbd2e802901988c181281a9b7f772543e1df32d2",
          "message": "chore: add `background-merge.toml` to CI (#2895)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nThis new stressgres file was instrumental in catching the bug fixed in\n#2845 and was used to test background merging, but has not been added to\nCI.\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-07-23T10:26:09-04:00",
          "tree_id": "bb8e3ca51c2d0de5e56be65f478f11096b0d410e",
          "url": "https://github.com/paradedb/paradedb/commit/cbd2e802901988c181281a9b7f772543e1df32d2"
        },
        "date": 1753282982138,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.805092,
            "unit": "median cpu",
            "extra": "avg cpu: 20.970735859310633, max cpu: 49.760765, count: 57904"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 176.421875,
            "unit": "median mem",
            "extra": "avg mem: 174.99929807040533, max mem: 180.5, count: 57904"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 18119,
            "unit": "median block_count",
            "extra": "avg block_count: 16778.715943630836, max block_count: 21996.0, count: 57904"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 40,
            "unit": "median segment_count",
            "extra": "avg segment_count: 42.095641061066594, max segment_count: 114.0, count: 57904"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 11.294118,
            "unit": "median cpu",
            "extra": "avg cpu: 11.905509724933237, max cpu: 37.982197, count: 57904"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 160.2578125,
            "unit": "median mem",
            "extra": "avg mem: 150.70848516510085, max mem: 167.22265625, count: 57904"
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
          "id": "53fa29a57e7daa2bdec9bf05f86d8db50088fd51",
          "message": "perf: Avoid a re-query when a top-n query is exhausted (#2888)\n\n## What\n\nSplit the iterators consumed by the `TopN` exec method and by the\n`Normal`/`FF` exec methods into two separate implementations.\n\nAfter splitting them, use the known exact size of the `TopN` iterator to\nexit early (as `exhausted: bool`) if we had fewer matches than were\nrequested. This avoids re-querying an iterator (and re-scanning its\ncolumns) that we know has no more results.\n\n## Why\n\nThe two types of consumers of these iterators have very different\nconsumption patterns and constraints:\n* TopN knows exactly how many results there will be, and already buffers\nthem all in memory. It would like to be able to know the precise count\nof results.\n* `Normal` and `FF` exec methods do not know the total number of results\nto expect, and in some cases would like to be able to consume in a\nsegment-aware fashion in order to be able to late-fetch fast field\ncolumns (see #2623).\n\n## Tests\n\nAdded a test to cement the change in query count.\n\nBenchmarks show a 1.5x speedup for `paging-string-max`, and no change\nfor other queries.",
          "timestamp": "2025-07-23T09:13:45-07:00",
          "tree_id": "c162e59fb9a6b4523c0ff39e10d9d54d92761460",
          "url": "https://github.com/paradedb/paradedb/commit/53fa29a57e7daa2bdec9bf05f86d8db50088fd51"
        },
        "date": 1753289441705,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.677044,
            "unit": "median cpu",
            "extra": "avg cpu: 20.338844378798772, max cpu: 43.373497, count: 57616"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 173.796875,
            "unit": "median mem",
            "extra": "avg mem: 172.53398585299655, max mem: 178.9296875, count: 57616"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 18287,
            "unit": "median block_count",
            "extra": "avg block_count: 16935.253176201055, max block_count: 22279.0, count: 57616"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 41,
            "unit": "median segment_count",
            "extra": "avg segment_count: 42.73588933629547, max segment_count: 122.0, count: 57616"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 9.384164,
            "unit": "median cpu",
            "extra": "avg cpu: 11.5084081219152, max cpu: 32.65306, count: 57616"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 164.94921875,
            "unit": "median mem",
            "extra": "avg mem: 154.30285966571785, max mem: 174.18359375, count: 57616"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "880ee607618b7ffe12f2784d03be3110b4df6cdf",
          "message": "feat: text search operators (#2877)\n\n## What\n\nThis is the implementation of 4 new operators designed around common\n\"text search\" patterns: `===` (exact token), `&&&` (tokenized term\nconjunction) `|||` (tokenized term disjunction), and `###` (phrase).\n\nFirst, `@@@` still exists and still does what it's always done, which is\nrun the right-hand-side through tantivy's \"query parser\".\n\nThese new operators have specific meanings. The spelling of these\noperators is both to maintain some kind of consistency in that they're\nall 3 of the same symbol and also to try and confer their meaning.\n\nIn all cases, the left-hand-side of the operator is required to be a\nreference to a Postgres `TEXT`/`VARCHAR` field and the right-hand-side\nis text. The operator definition defines how the right-hand-side input\nis applied to the left-hand-side field. These are \"text search\"\noperators and would be nonsensical applied to non-text fields.\n\n- `field === 'TEXT'` - this is the exact term match operator. The\nright-hand-side value is used, unmodified, to perform the equivalent of\nthe `paradedb.term()` function.\n\n- `field === ARRAY['...', '...', '...']` - similar to the above, but\nrewrites to the `paradedb.term_set()` function.\n\n- `field &&& 'TEXT'` - this is the \"match conjunction\" operator. It\ntokenizes the right-hand-side using the index tokenizer defined by the\nleft-hand-side field and searches for documents that contain **all** of\nthe tokenized terms. Think, \"find all documents that contain all terms\ntokenized from this text input\". This is the equivalent of the\n`paradedb.match` function with the `conjunction_mode => true`.\n\n- `field ||| 'TEXT'` - this is the \"match disjunction\" operator. It\ntokenizes the right-hand-side using the index tokenizer defined by the\nleft-hand-side field and searches for documents that contain **any** of\nthe tokenized terms. Think, \"find all documents that contain one or more\nof the terms tokenized from this text input\". This is the equivalent of\nthe `paradedb.match` function with the `conjunction_mode => false`.\n\n- `field ### 'TEXT'` - this is the \"phrase search\" operator. It\ntokenizes the right-hand-side using the index tokenizer defined by the\nleft-hand-side field and searches for documents that contain all the\ntokenized terms in the order written. Think, \"find all documents that\ncontain this tokenized phrase\". This is akin to the `paradedb.phrase()`\nfunction, except the `###` is transparently handling the tokenization.\n\nIf you wanted to find all documents that talk about bbq and chicken and\nmaybe hotdogs and happen on July 4th, you might write a query like this:\n\n```sql\nSELECT * \nFROM events \nWHERE (\n       description &&& 'bbq chicken' OR description ||| 'hot dog hotdogs'\n) AND description ### 'July 4th';\n```\n\n## Why\n\nThese operators are in furtherance of our\n[roadmap](https://github.com/orgs/paradedb/discussions/2041) --\nspecifically the \"Database UX improvements\" line item.\n\nOur overall goal with that roadmap item is to start moving users and ORM\ntools away from needing to use our \"builder functions\" in the common\ntext-search cases. What we're striving for is for users, and ORM tools,\nto write standard SQL WHERE clauses. In order to do that we need to make\ncommon things more accessible using custom operators.\n\n## How\n\nLargely just adding new `#[pg_operator]` functions and ensuring they all\nhave a SUPPORT function that can rewrite the expression to what we need.\n\n@rebasedming said he was going to tackle documentation, which can come\nin a separate PR.\n\n## Tests\n\nThere's a new regression test named `operators.sql` that validates\nEXPLAIN output and search results against our standard \"mock_items\"\ntable.",
          "timestamp": "2025-07-23T12:54:32-04:00",
          "tree_id": "f4a0dd94cd1e423e30c42103a8a23d86905d32e7",
          "url": "https://github.com/paradedb/paradedb/commit/880ee607618b7ffe12f2784d03be3110b4df6cdf"
        },
        "date": 1753291896488,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.677044,
            "unit": "median cpu",
            "extra": "avg cpu: 20.72529959591255, max cpu: 46.466602, count: 57767"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 170.51953125,
            "unit": "median mem",
            "extra": "avg mem: 169.05558604501704, max mem: 174.6875, count: 57767"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 17680,
            "unit": "median block_count",
            "extra": "avg block_count: 16526.22975054962, max block_count: 22100.0, count: 57767"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 41,
            "unit": "median segment_count",
            "extra": "avg segment_count: 42.44530614364603, max segment_count: 114.0, count: 57767"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 9.365853,
            "unit": "median cpu",
            "extra": "avg cpu: 11.552221186337043, max cpu: 33.03835, count: 57767"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 164.2265625,
            "unit": "median mem",
            "extra": "avg mem: 154.55939897833105, max mem: 174.98046875, count: 57767"
          }
        ]
      }
    ],
    "pg_search background-merge.toml Performance - TPS": [
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
          "id": "cbd2e802901988c181281a9b7f772543e1df32d2",
          "message": "chore: add `background-merge.toml` to CI (#2895)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nThis new stressgres file was instrumental in catching the bug fixed in\n#2845 and was used to test background merging, but has not been added to\nCI.\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-07-23T10:26:09-04:00",
          "tree_id": "bb8e3ca51c2d0de5e56be65f478f11096b0d410e",
          "url": "https://github.com/paradedb/paradedb/commit/cbd2e802901988c181281a9b7f772543e1df32d2"
        },
        "date": 1753283608911,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 36.76488126750026,
            "unit": "median tps",
            "extra": "avg tps: 37.175937650243064, max tps: 39.120747245405454, count: 55371"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 250.53728920229037,
            "unit": "median tps",
            "extra": "avg tps: 285.05638061700535, max tps: 2564.6084796790014, count: 55371"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 140.75995583365787,
            "unit": "median tps",
            "extra": "avg tps: 141.1096455213053, max tps: 146.96560952660133, count: 55371"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 72.39408726599235,
            "unit": "median tps",
            "extra": "avg tps: 64.11955104369487, max tps: 112.87075964441861, count: 110742"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 17.685923715204176,
            "unit": "median tps",
            "extra": "avg tps: 17.867345026510524, max tps: 20.492187927134044, count: 55371"
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
          "id": "53fa29a57e7daa2bdec9bf05f86d8db50088fd51",
          "message": "perf: Avoid a re-query when a top-n query is exhausted (#2888)\n\n## What\n\nSplit the iterators consumed by the `TopN` exec method and by the\n`Normal`/`FF` exec methods into two separate implementations.\n\nAfter splitting them, use the known exact size of the `TopN` iterator to\nexit early (as `exhausted: bool`) if we had fewer matches than were\nrequested. This avoids re-querying an iterator (and re-scanning its\ncolumns) that we know has no more results.\n\n## Why\n\nThe two types of consumers of these iterators have very different\nconsumption patterns and constraints:\n* TopN knows exactly how many results there will be, and already buffers\nthem all in memory. It would like to be able to know the precise count\nof results.\n* `Normal` and `FF` exec methods do not know the total number of results\nto expect, and in some cases would like to be able to consume in a\nsegment-aware fashion in order to be able to late-fetch fast field\ncolumns (see #2623).\n\n## Tests\n\nAdded a test to cement the change in query count.\n\nBenchmarks show a 1.5x speedup for `paging-string-max`, and no change\nfor other queries.",
          "timestamp": "2025-07-23T09:13:45-07:00",
          "tree_id": "c162e59fb9a6b4523c0ff39e10d9d54d92761460",
          "url": "https://github.com/paradedb/paradedb/commit/53fa29a57e7daa2bdec9bf05f86d8db50088fd51"
        },
        "date": 1753290070191,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 38.70637577411575,
            "unit": "median tps",
            "extra": "avg tps: 38.808695808918145, max tps: 41.998425353038236, count: 55235"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 257.63869378968076,
            "unit": "median tps",
            "extra": "avg tps: 289.79539185348455, max tps: 2566.752882875581, count: 55235"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 146.23645394462724,
            "unit": "median tps",
            "extra": "avg tps: 145.38540185213307, max tps: 146.71599878433742, count: 55235"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 74.87354312492917,
            "unit": "median tps",
            "extra": "avg tps: 72.28300931983173, max tps: 119.99009389211133, count: 110470"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 18.00802059987152,
            "unit": "median tps",
            "extra": "avg tps: 18.153006824696185, max tps: 19.502466872885492, count: 55235"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "880ee607618b7ffe12f2784d03be3110b4df6cdf",
          "message": "feat: text search operators (#2877)\n\n## What\n\nThis is the implementation of 4 new operators designed around common\n\"text search\" patterns: `===` (exact token), `&&&` (tokenized term\nconjunction) `|||` (tokenized term disjunction), and `###` (phrase).\n\nFirst, `@@@` still exists and still does what it's always done, which is\nrun the right-hand-side through tantivy's \"query parser\".\n\nThese new operators have specific meanings. The spelling of these\noperators is both to maintain some kind of consistency in that they're\nall 3 of the same symbol and also to try and confer their meaning.\n\nIn all cases, the left-hand-side of the operator is required to be a\nreference to a Postgres `TEXT`/`VARCHAR` field and the right-hand-side\nis text. The operator definition defines how the right-hand-side input\nis applied to the left-hand-side field. These are \"text search\"\noperators and would be nonsensical applied to non-text fields.\n\n- `field === 'TEXT'` - this is the exact term match operator. The\nright-hand-side value is used, unmodified, to perform the equivalent of\nthe `paradedb.term()` function.\n\n- `field === ARRAY['...', '...', '...']` - similar to the above, but\nrewrites to the `paradedb.term_set()` function.\n\n- `field &&& 'TEXT'` - this is the \"match conjunction\" operator. It\ntokenizes the right-hand-side using the index tokenizer defined by the\nleft-hand-side field and searches for documents that contain **all** of\nthe tokenized terms. Think, \"find all documents that contain all terms\ntokenized from this text input\". This is the equivalent of the\n`paradedb.match` function with the `conjunction_mode => true`.\n\n- `field ||| 'TEXT'` - this is the \"match disjunction\" operator. It\ntokenizes the right-hand-side using the index tokenizer defined by the\nleft-hand-side field and searches for documents that contain **any** of\nthe tokenized terms. Think, \"find all documents that contain one or more\nof the terms tokenized from this text input\". This is the equivalent of\nthe `paradedb.match` function with the `conjunction_mode => false`.\n\n- `field ### 'TEXT'` - this is the \"phrase search\" operator. It\ntokenizes the right-hand-side using the index tokenizer defined by the\nleft-hand-side field and searches for documents that contain all the\ntokenized terms in the order written. Think, \"find all documents that\ncontain this tokenized phrase\". This is akin to the `paradedb.phrase()`\nfunction, except the `###` is transparently handling the tokenization.\n\nIf you wanted to find all documents that talk about bbq and chicken and\nmaybe hotdogs and happen on July 4th, you might write a query like this:\n\n```sql\nSELECT * \nFROM events \nWHERE (\n       description &&& 'bbq chicken' OR description ||| 'hot dog hotdogs'\n) AND description ### 'July 4th';\n```\n\n## Why\n\nThese operators are in furtherance of our\n[roadmap](https://github.com/orgs/paradedb/discussions/2041) --\nspecifically the \"Database UX improvements\" line item.\n\nOur overall goal with that roadmap item is to start moving users and ORM\ntools away from needing to use our \"builder functions\" in the common\ntext-search cases. What we're striving for is for users, and ORM tools,\nto write standard SQL WHERE clauses. In order to do that we need to make\ncommon things more accessible using custom operators.\n\n## How\n\nLargely just adding new `#[pg_operator]` functions and ensuring they all\nhave a SUPPORT function that can rewrite the expression to what we need.\n\n@rebasedming said he was going to tackle documentation, which can come\nin a separate PR.\n\n## Tests\n\nThere's a new regression test named `operators.sql` that validates\nEXPLAIN output and search results against our standard \"mock_items\"\ntable.",
          "timestamp": "2025-07-23T12:54:32-04:00",
          "tree_id": "f4a0dd94cd1e423e30c42103a8a23d86905d32e7",
          "url": "https://github.com/paradedb/paradedb/commit/880ee607618b7ffe12f2784d03be3110b4df6cdf"
        },
        "date": 1753292525162,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 38.72889793180519,
            "unit": "median tps",
            "extra": "avg tps: 38.89931385985902, max tps: 40.766486447426615, count: 55284"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 257.43870540520174,
            "unit": "median tps",
            "extra": "avg tps: 294.3657090265942, max tps: 2612.7795420687617, count: 55284"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 155.7670915721904,
            "unit": "median tps",
            "extra": "avg tps: 155.35967807212813, max tps: 161.27584994708846, count: 55284"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 83.21932936859552,
            "unit": "median tps",
            "extra": "avg tps: 78.48971362628333, max tps: 140.08865146889823, count: 110568"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 18.852728854317448,
            "unit": "median tps",
            "extra": "avg tps: 18.87700733747028, max tps: 19.98786949525579, count: 55284"
          }
        ]
      }
    ],
    "pg_search background-merge.toml Performance - Other Metrics": [
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
          "id": "cbd2e802901988c181281a9b7f772543e1df32d2",
          "message": "chore: add `background-merge.toml` to CI (#2895)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nThis new stressgres file was instrumental in catching the bug fixed in\n#2845 and was used to test background merging, but has not been added to\nCI.\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-07-23T10:26:09-04:00",
          "tree_id": "bb8e3ca51c2d0de5e56be65f478f11096b0d410e",
          "url": "https://github.com/paradedb/paradedb/commit/cbd2e802901988c181281a9b7f772543e1df32d2"
        },
        "date": 1753283611014,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.497108,
            "unit": "median cpu",
            "extra": "avg cpu: 18.79644423088196, max cpu: 37.72102, count: 55371"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 153.453125,
            "unit": "median mem",
            "extra": "avg mem: 140.15833028570913, max mem: 153.453125, count: 55371"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 7.577871649422924, max cpu: 27.853, count: 55371"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 145.578125,
            "unit": "median mem",
            "extra": "avg mem: 141.51240326615016, max mem: 145.953125, count: 55371"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.284333,
            "unit": "median cpu",
            "extra": "avg cpu: 11.231000912344507, max cpu: 32.526623, count: 55371"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 145.84765625,
            "unit": "median mem",
            "extra": "avg mem: 121.37304959105398, max mem: 152.78125, count: 55371"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 21456,
            "unit": "median block_count",
            "extra": "avg block_count: 21787.010637337233, max block_count: 43313.0, count: 55371"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.619827,
            "unit": "median cpu",
            "extra": "avg cpu: 4.13359864126847, max cpu: 4.6421666, count: 55371"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 99.828125,
            "unit": "median mem",
            "extra": "avg mem: 89.79152472639107, max mem: 129.453125, count: 55371"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 30,
            "unit": "median segment_count",
            "extra": "avg segment_count: 30.265102671073304, max segment_count: 46.0, count: 55371"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 13.899614,
            "unit": "median cpu",
            "extra": "avg cpu: 15.139585857598687, max cpu: 32.684826, count: 110742"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 160.29296875,
            "unit": "median mem",
            "extra": "avg mem: 148.1737127103425, max mem: 178.375, count: 110742"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.859479,
            "unit": "median cpu",
            "extra": "avg cpu: 13.514282810541026, max cpu: 27.87996, count: 55371"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 155.51171875,
            "unit": "median mem",
            "extra": "avg mem: 153.5563633405799, max mem: 157.19140625, count: 55371"
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
          "id": "53fa29a57e7daa2bdec9bf05f86d8db50088fd51",
          "message": "perf: Avoid a re-query when a top-n query is exhausted (#2888)\n\n## What\n\nSplit the iterators consumed by the `TopN` exec method and by the\n`Normal`/`FF` exec methods into two separate implementations.\n\nAfter splitting them, use the known exact size of the `TopN` iterator to\nexit early (as `exhausted: bool`) if we had fewer matches than were\nrequested. This avoids re-querying an iterator (and re-scanning its\ncolumns) that we know has no more results.\n\n## Why\n\nThe two types of consumers of these iterators have very different\nconsumption patterns and constraints:\n* TopN knows exactly how many results there will be, and already buffers\nthem all in memory. It would like to be able to know the precise count\nof results.\n* `Normal` and `FF` exec methods do not know the total number of results\nto expect, and in some cases would like to be able to consume in a\nsegment-aware fashion in order to be able to late-fetch fast field\ncolumns (see #2623).\n\n## Tests\n\nAdded a test to cement the change in query count.\n\nBenchmarks show a 1.5x speedup for `paging-string-max`, and no change\nfor other queries.",
          "timestamp": "2025-07-23T09:13:45-07:00",
          "tree_id": "c162e59fb9a6b4523c0ff39e10d9d54d92761460",
          "url": "https://github.com/paradedb/paradedb/commit/53fa29a57e7daa2bdec9bf05f86d8db50088fd51"
        },
        "date": 1753290072378,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.461538,
            "unit": "median cpu",
            "extra": "avg cpu: 18.500734084223225, max cpu: 42.436146, count: 55235"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 154.1875,
            "unit": "median mem",
            "extra": "avg mem: 141.39015506189463, max mem: 154.94921875, count: 55235"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.619827,
            "unit": "median cpu",
            "extra": "avg cpu: 7.592605944683425, max cpu: 27.826086, count: 55235"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 144.859375,
            "unit": "median mem",
            "extra": "avg mem: 140.23437075676654, max mem: 144.859375, count: 55235"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.266409,
            "unit": "median cpu",
            "extra": "avg cpu: 10.870756060089509, max cpu: 28.290766, count: 55235"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 143.953125,
            "unit": "median mem",
            "extra": "avg mem: 120.31137405800217, max mem: 150.96484375, count: 55235"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 22881,
            "unit": "median block_count",
            "extra": "avg block_count: 23186.18394134154, max block_count: 46585.0, count: 55235"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.610951,
            "unit": "median cpu",
            "extra": "avg cpu: 4.161818477737078, max cpu: 4.6376815, count: 55235"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 97.609375,
            "unit": "median mem",
            "extra": "avg mem: 89.815045657192, max mem: 128.734375, count: 55235"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 30,
            "unit": "median segment_count",
            "extra": "avg segment_count: 30.347008237530552, max segment_count: 50.0, count: 55235"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 13.846154,
            "unit": "median cpu",
            "extra": "avg cpu: 14.1240203008054, max cpu: 33.005894, count: 110470"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 153.34765625,
            "unit": "median mem",
            "extra": "avg mem: 140.89518410682766, max mem: 162.60546875, count: 110470"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.806328,
            "unit": "median cpu",
            "extra": "avg cpu: 13.033026123976722, max cpu: 27.639154, count: 55235"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 155.9140625,
            "unit": "median mem",
            "extra": "avg mem: 153.79799775957272, max mem: 157.50390625, count: 55235"
          }
        ]
      }
    ]
  }
}