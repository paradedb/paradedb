window.BENCHMARK_DATA = {
  "lastUpdate": 1752441638604,
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
      }
    ]
  }
}