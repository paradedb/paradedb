window.BENCHMARK_DATA = {
  "lastUpdate": 1752174768751,
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
      }
    ]
  }
}