window.BENCHMARK_DATA = {
  "lastUpdate": 1752174246718,
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
      }
    ]
  }
}