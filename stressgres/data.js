window.BENCHMARK_DATA = {
  "lastUpdate": 1752174408890,
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
      }
    ]
  }
}