window.BENCHMARK_DATA = {
  "lastUpdate": 1757448675885,
  "repoUrl": "https://github.com/paradedb/paradedb",
  "entries": {
    "pg_search background-merge.toml Performance - TPS": [
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
          "id": "0544c54d64a963065cefc3a922582cc501a4c90e",
          "message": "fix: zero worker threads (#2959) (#3139)\n\nWe don't use any of Tantivy's threading features, and as of\nhttps://github.com/paradedb/tantivy/pull/59 it's now possible to set the\nnumber of merge and worker threads to zero.\n\nDoing so saves overhead of making threads that we never use, and joining\non them, for every segment merge operation.\n\n\n🍒 This is a cherry pick of 98d7dcdc33169d31d80e13ef39aa7242e1a09710 from\n`main/0.18.x`",
          "timestamp": "2025-09-09T18:06:14Z",
          "url": "https://github.com/paradedb/paradedb/commit/0544c54d64a963065cefc3a922582cc501a4c90e"
        },
        "date": 1757446934636,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Delete value - Primary - tps",
            "value": 246.36464082668698,
            "unit": "median tps",
            "extra": "avg tps: 238.717335185094, max tps: 479.59450641571533, count: 55741"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 432.5390208207214,
            "unit": "median tps",
            "extra": "avg tps: 425.98768000105474, max tps: 459.4577051934103, count: 55741"
          },
          {
            "name": "Monitor Segment Count - Primary - tps",
            "value": 1799.0441739571525,
            "unit": "median tps",
            "extra": "avg tps: 1800.078378524505, max tps: 1857.8331193913716, count: 55741"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 20.709789469776986,
            "unit": "median tps",
            "extra": "avg tps: 41.89573788596861, max tps: 173.69020992454986, count: 167223"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 1.2067561981357664,
            "unit": "median tps",
            "extra": "avg tps: 1.3994737710607075, max tps: 5.254380704612167, count: 55741"
          }
        ]
      }
    ],
    "pg_search background-merge.toml Performance - Other Metrics": [
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
          "id": "0544c54d64a963065cefc3a922582cc501a4c90e",
          "message": "fix: zero worker threads (#2959) (#3139)\n\nWe don't use any of Tantivy's threading features, and as of\nhttps://github.com/paradedb/tantivy/pull/59 it's now possible to set the\nnumber of merge and worker threads to zero.\n\nDoing so saves overhead of making threads that we never use, and joining\non them, for every segment merge operation.\n\n\n🍒 This is a cherry pick of 98d7dcdc33169d31d80e13ef39aa7242e1a09710 from\n`main/0.18.x`",
          "timestamp": "2025-09-09T18:06:14Z",
          "url": "https://github.com/paradedb/paradedb/commit/0544c54d64a963065cefc3a922582cc501a4c90e"
        },
        "date": 1757446937425,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 7.671846515387898, max cpu: 32.684826, count: 55741"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 207.15625,
            "unit": "median mem",
            "extra": "avg mem: 205.52454796514235, max mem: 207.15625, count: 55741"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 5.295032567335353, max cpu: 14.007783, count: 55741"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 164.6328125,
            "unit": "median mem",
            "extra": "avg mem: 153.93712627094507, max mem: 166.8984375, count: 55741"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 48878,
            "unit": "median block_count",
            "extra": "avg block_count: 48722.42017545433, max block_count: 76654.0, count: 55741"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 2.9487562947834243, max cpu: 4.6511626, count: 55741"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 132.2890625,
            "unit": "median mem",
            "extra": "avg mem: 116.26924701128881, max mem: 141.3125, count: 55741"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 35,
            "unit": "median segment_count",
            "extra": "avg segment_count: 35.800846773470155, max segment_count: 68.0, count: 55741"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 23.076923,
            "unit": "median cpu",
            "extra": "avg cpu: 18.56918237454024, max cpu: 32.71665, count: 167223"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 227.36328125,
            "unit": "median mem",
            "extra": "avg mem: 274.2931124811629, max mem: 500.05859375, count: 167223"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 23.166023,
            "unit": "median cpu",
            "extra": "avg cpu: 20.51355914761103, max cpu: 32.589718, count: 55741"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 200.5234375,
            "unit": "median mem",
            "extra": "avg mem: 197.4519551783023, max mem: 231.1328125, count: 55741"
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
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "1c48d3c6427e9fe7bdb10861a16e04d0cf1a1f48",
          "message": "ci: fix benchmark links in slack messages (#2875)",
          "timestamp": "2025-07-17T12:13:25Z",
          "url": "https://github.com/paradedb/paradedb/commit/1c48d3c6427e9fe7bdb10861a16e04d0cf1a1f48"
        },
        "date": 1757447856470,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 1259.5320384725492,
            "unit": "median tps",
            "extra": "avg tps: 1247.5969043336243, max tps: 1263.4199707943944, count: 55039"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2586.096064044068,
            "unit": "median tps",
            "extra": "avg tps: 2593.38679646088, max tps: 2641.4324289534175, count: 55039"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 1168.8658852724543,
            "unit": "median tps",
            "extra": "avg tps: 1168.454479301618, max tps: 1209.8411521199764, count: 55039"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 983.8142784805156,
            "unit": "median tps",
            "extra": "avg tps: 980.9535466043926, max tps: 995.1930174342793, count: 55039"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 157.70939915678613,
            "unit": "median tps",
            "extra": "avg tps: 157.52784547634755, max tps: 165.27334275816813, count: 110078"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 172.17911064606312,
            "unit": "median tps",
            "extra": "avg tps: 170.1576082088954, max tps: 173.21682646991584, count: 55039"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 27.577039448947644,
            "unit": "median tps",
            "extra": "avg tps: 33.67427968993795, max tps: 775.373516807384, count: 55039"
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
          "id": "8cd6a2c7cdf969cf43bd66f12beca6ddedd6889a",
          "message": "chore: Upgrade to `0.17.5` (#3005)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-08-20T17:19:49Z",
          "url": "https://github.com/paradedb/paradedb/commit/8cd6a2c7cdf969cf43bd66f12beca6ddedd6889a"
        },
        "date": 1757447856855,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 1191.5334706884983,
            "unit": "median tps",
            "extra": "avg tps: 1188.6565213097747, max tps: 1230.0197098802896, count: 55388"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2611.564012497589,
            "unit": "median tps",
            "extra": "avg tps: 2610.3277656497885, max tps: 2666.756447467065, count: 55388"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 1168.3529666692887,
            "unit": "median tps",
            "extra": "avg tps: 1167.4953517838517, max tps: 1223.4195990608114, count: 55388"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 1015.3834298829055,
            "unit": "median tps",
            "extra": "avg tps: 1014.5556087530746, max tps: 1030.9198975332492, count: 55388"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 166.93917115571108,
            "unit": "median tps",
            "extra": "avg tps: 181.59874122140928, max tps: 206.7103830442316, count: 110776"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 140.3886761938467,
            "unit": "median tps",
            "extra": "avg tps: 139.962679810926, max tps: 149.74950131300037, count: 55388"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 39.82233959216197,
            "unit": "median tps",
            "extra": "avg tps: 45.20561714558792, max tps: 757.4896792031209, count: 55388"
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
          "id": "0544c54d64a963065cefc3a922582cc501a4c90e",
          "message": "fix: zero worker threads (#2959) (#3139)\n\nWe don't use any of Tantivy's threading features, and as of\nhttps://github.com/paradedb/tantivy/pull/59 it's now possible to set the\nnumber of merge and worker threads to zero.\n\nDoing so saves overhead of making threads that we never use, and joining\non them, for every segment merge operation.\n\n\n🍒 This is a cherry pick of 98d7dcdc33169d31d80e13ef39aa7242e1a09710 from\n`main/0.18.x`",
          "timestamp": "2025-09-09T18:06:14Z",
          "url": "https://github.com/paradedb/paradedb/commit/0544c54d64a963065cefc3a922582cc501a4c90e"
        },
        "date": 1757447862844,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 1250.0017798832077,
            "unit": "median tps",
            "extra": "avg tps: 1246.127353753529, max tps: 1255.2601600682424, count: 55174"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2904.4777203115905,
            "unit": "median tps",
            "extra": "avg tps: 2896.912355524276, max tps: 2940.177534210128, count: 55174"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 1232.0182697196244,
            "unit": "median tps",
            "extra": "avg tps: 1228.9623003074375, max tps: 1236.0269740030935, count: 55174"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 1050.8057993579184,
            "unit": "median tps",
            "extra": "avg tps: 1046.9907961187823, max tps: 1056.9491193229817, count: 55174"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 528.5534842704149,
            "unit": "median tps",
            "extra": "avg tps: 570.68337502718, max tps: 626.9283468229802, count: 110348"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 470.71472137797576,
            "unit": "median tps",
            "extra": "avg tps: 467.555870913969, max tps: 471.18891793283103, count: 55174"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 116.40060577585945,
            "unit": "median tps",
            "extra": "avg tps: 124.73597312560425, max tps: 836.0686713363888, count: 55174"
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
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "4b08446356edfe8faed3798c0521304b8789d3e5",
          "message": "fix: make IndexLayerSizes calculations more accurate (#3040)\n\n## What\n\nThe `From<&PgSearchRelation> for IndexLayerSizes` `from()` ctor was\nimproperly calculating both the total `index_byte_size` and the\n`target_segment_byte_size`.\n\nFor the former, it needs to only consider SegmentMetaEntries that are\nactually visible. In almost every case, at least after a CREATE INDEX of\na large index, the index could actually have 2x+ total segments as there\nare actually visible segments.\n\nSecondly, we need to adjust the `target_segment_byte_size` down by 1/3\nto account for how `LayeredMergePolicy` does the same thing. Failure to\ndo this can cause the background worker to merge segments together that\ndon't actually need to be.\n\n## Why\n\nThese two miscalculations are what contributed to the recent query\nbenchmark regressions we saw after #3031 was merged.\n\n## How\n\n## Tests\n\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>",
          "timestamp": "2025-08-24T17:46:13Z",
          "url": "https://github.com/paradedb/paradedb/commit/4b08446356edfe8faed3798c0521304b8789d3e5"
        },
        "date": 1757447862254,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 1185.6494261280939,
            "unit": "median tps",
            "extra": "avg tps: 1183.9243876960215, max tps: 1206.6087860385494, count: 55127"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2812.1825349610904,
            "unit": "median tps",
            "extra": "avg tps: 2784.0981197118576, max tps: 2884.5545710750007, count: 55127"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 1184.210346139706,
            "unit": "median tps",
            "extra": "avg tps: 1183.4388010876341, max tps: 1205.3021870368218, count: 55127"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 996.3600115439618,
            "unit": "median tps",
            "extra": "avg tps: 987.3605297187564, max tps: 1000.1905489843774, count: 55127"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 172.14953885805912,
            "unit": "median tps",
            "extra": "avg tps: 174.98282719293996, max tps: 187.42583013987147, count: 110254"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 152.9723659432332,
            "unit": "median tps",
            "extra": "avg tps: 152.51985906052138, max tps: 156.86970706263682, count: 55127"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 44.671402617838226,
            "unit": "median tps",
            "extra": "avg tps: 46.87365147697865, max tps: 836.6653531020623, count: 55127"
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
          "id": "cea1e8a6034491ad9e758c9ca40b3bf03dd0ad80",
          "message": "chore: Upgrade to `0.17.10` (#3091)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-09-01T01:10:59Z",
          "url": "https://github.com/paradedb/paradedb/commit/cea1e8a6034491ad9e758c9ca40b3bf03dd0ad80"
        },
        "date": 1757447900519,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 1194.3818551656968,
            "unit": "median tps",
            "extra": "avg tps: 1192.8047054866233, max tps: 1248.9369866287789, count: 54901"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2801.587389108085,
            "unit": "median tps",
            "extra": "avg tps: 2802.5691996676114, max tps: 2861.412865719217, count: 54901"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 1199.618294402218,
            "unit": "median tps",
            "extra": "avg tps: 1196.833351373611, max tps: 1260.1133429948245, count: 54901"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 971.1710730311543,
            "unit": "median tps",
            "extra": "avg tps: 970.107386795085, max tps: 1025.4156919604616, count: 54901"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 528.5767569401883,
            "unit": "median tps",
            "extra": "avg tps: 550.7694100552657, max tps: 598.1665259040693, count: 109802"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 546.4341057914263,
            "unit": "median tps",
            "extra": "avg tps: 535.6977507036221, max tps: 553.1079471964302, count: 54901"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 118.07040262235938,
            "unit": "median tps",
            "extra": "avg tps: 127.96771552189627, max tps: 920.4441695384525, count: 54901"
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
          "id": "ec43ca34644c8c72e65ed2b7e9570066e839bab7",
          "message": "chore: Upgrade to `0.17.12` (#3120)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nNote that `0.17.11` was skipped because it was a hotfix in enterprise.\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-09-03T20:56:05Z",
          "url": "https://github.com/paradedb/paradedb/commit/ec43ca34644c8c72e65ed2b7e9570066e839bab7"
        },
        "date": 1757447950478,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 1259.1998613753083,
            "unit": "median tps",
            "extra": "avg tps: 1251.3799823992788, max tps: 1261.8714666955561, count: 55242"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2837.6264784356476,
            "unit": "median tps",
            "extra": "avg tps: 2826.0868720424955, max tps: 2864.398190098366, count: 55242"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 1232.738428708033,
            "unit": "median tps",
            "extra": "avg tps: 1230.675545300398, max tps: 1263.5110883228854, count: 55242"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 1019.2464285893527,
            "unit": "median tps",
            "extra": "avg tps: 1012.4667533523069, max tps: 1026.7151260423352, count: 55242"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 534.9695466849944,
            "unit": "median tps",
            "extra": "avg tps: 562.5582975100722, max tps: 608.1195202341505, count: 110484"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 502.512598413392,
            "unit": "median tps",
            "extra": "avg tps: 493.32471670146987, max tps: 521.0252964549699, count: 55242"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 126.88887157970048,
            "unit": "median tps",
            "extra": "avg tps: 133.0880394146037, max tps: 813.9557598765392, count: 55242"
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
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "1c48d3c6427e9fe7bdb10861a16e04d0cf1a1f48",
          "message": "ci: fix benchmark links in slack messages (#2875)",
          "timestamp": "2025-07-17T12:13:25Z",
          "url": "https://github.com/paradedb/paradedb/commit/1c48d3c6427e9fe7bdb10861a16e04d0cf1a1f48"
        },
        "date": 1757447859282,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.787988353805128, max cpu: 9.448819, count: 55039"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 59.49609375,
            "unit": "median mem",
            "extra": "avg mem: 59.61352887270844, max mem: 82.75390625, count: 55039"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.6959982956668025, max cpu: 9.320388, count: 55039"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 53.94140625,
            "unit": "median mem",
            "extra": "avg mem: 53.20595992160105, max mem: 76.22265625, count: 55039"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.750694175222884, max cpu: 9.467456, count: 55039"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 59.54296875,
            "unit": "median mem",
            "extra": "avg mem: 59.80519679795236, max mem: 83.2421875, count: 55039"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.619827,
            "unit": "median cpu",
            "extra": "avg cpu: 4.458919937868095, max cpu: 4.660194, count: 55039"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 59.98828125,
            "unit": "median mem",
            "extra": "avg mem: 60.09983496077781, max mem: 83.3046875, count: 55039"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.221902,
            "unit": "median cpu",
            "extra": "avg cpu: 7.9138276548492605, max cpu: 27.799229, count: 110078"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 65.04296875,
            "unit": "median mem",
            "extra": "avg mem: 65.09999464868093, max mem: 93.74609375, count: 110078"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 3636,
            "unit": "median block_count",
            "extra": "avg block_count: 3668.2462435727393, max block_count: 6601.0, count: 55039"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 8,
            "unit": "median segment_count",
            "extra": "avg segment_count: 8.85448500154436, max segment_count: 28.0, count: 55039"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 5.964023948836481, max cpu: 14.201183, count: 55039"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 69.05859375,
            "unit": "median mem",
            "extra": "avg mem: 69.15817477152565, max mem: 95.83203125, count: 55039"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.627355512717009, max cpu: 9.239654, count: 55039"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 57.98046875,
            "unit": "median mem",
            "extra": "avg mem: 58.246948896464325, max mem: 83.94921875, count: 55039"
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
          "id": "8cd6a2c7cdf969cf43bd66f12beca6ddedd6889a",
          "message": "chore: Upgrade to `0.17.5` (#3005)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-08-20T17:19:49Z",
          "url": "https://github.com/paradedb/paradedb/commit/8cd6a2c7cdf969cf43bd66f12beca6ddedd6889a"
        },
        "date": 1757447862321,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.7502387430123925, max cpu: 14.076246, count: 55388"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 58.703125,
            "unit": "median mem",
            "extra": "avg mem: 59.01468652111017, max mem: 82.8984375, count: 55388"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.634598595905058, max cpu: 9.284333, count: 55388"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 52.984375,
            "unit": "median mem",
            "extra": "avg mem: 53.73441449411425, max mem: 77.85546875, count: 55388"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.771228401743735, max cpu: 9.448819, count: 55388"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 59.95703125,
            "unit": "median mem",
            "extra": "avg mem: 60.192798840678485, max mem: 84.33984375, count: 55388"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.359875296444266, max cpu: 4.7197638, count: 55388"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 60.02734375,
            "unit": "median mem",
            "extra": "avg mem: 59.67863730918701, max mem: 82.5390625, count: 55388"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.204219,
            "unit": "median cpu",
            "extra": "avg cpu: 7.440193203323372, max cpu: 23.622047, count: 110776"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 68.6484375,
            "unit": "median mem",
            "extra": "avg mem: 68.63921079334648, max mem: 104.765625, count: 110776"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 3733,
            "unit": "median block_count",
            "extra": "avg block_count: 3746.4118581642233, max block_count: 6739.0, count: 55388"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 8,
            "unit": "median segment_count",
            "extra": "avg segment_count: 8.9434173467177, max segment_count: 33.0, count: 55388"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 6.296581998057856, max cpu: 14.229248, count: 55388"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 66.46484375,
            "unit": "median mem",
            "extra": "avg mem: 67.01471642379667, max mem: 93.6640625, count: 55388"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.619827,
            "unit": "median cpu",
            "extra": "avg cpu: 4.147180251899563, max cpu: 4.7151275, count: 55388"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 59.26171875,
            "unit": "median mem",
            "extra": "avg mem: 58.59918989108652, max mem: 83.47265625, count: 55388"
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
          "id": "0544c54d64a963065cefc3a922582cc501a4c90e",
          "message": "fix: zero worker threads (#2959) (#3139)\n\nWe don't use any of Tantivy's threading features, and as of\nhttps://github.com/paradedb/tantivy/pull/59 it's now possible to set the\nnumber of merge and worker threads to zero.\n\nDoing so saves overhead of making threads that we never use, and joining\non them, for every segment merge operation.\n\n\n🍒 This is a cherry pick of 98d7dcdc33169d31d80e13ef39aa7242e1a09710 from\n`main/0.18.x`",
          "timestamp": "2025-09-09T18:06:14Z",
          "url": "https://github.com/paradedb/paradedb/commit/0544c54d64a963065cefc3a922582cc501a4c90e"
        },
        "date": 1757447869289,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.732023785918534, max cpu: 9.552238, count: 55174"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 65.95703125,
            "unit": "median mem",
            "extra": "avg mem: 65.23202575601732, max mem: 95.609375, count: 55174"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.647154004165011, max cpu: 9.320388, count: 55174"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 60.29296875,
            "unit": "median mem",
            "extra": "avg mem: 59.240138088705734, max mem: 88.92578125, count: 55174"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.72649329522727, max cpu: 9.60961, count: 55174"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 67.21875,
            "unit": "median mem",
            "extra": "avg mem: 65.94824300168558, max mem: 95.59765625, count: 55174"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.317267463067173, max cpu: 4.729064, count: 55174"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 65.8671875,
            "unit": "median mem",
            "extra": "avg mem: 65.40930765623301, max mem: 95.62109375, count: 55174"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 5.0827912956907015, max cpu: 13.899614, count: 110348"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 78.2578125,
            "unit": "median mem",
            "extra": "avg mem: 78.14003715801374, max mem: 112.4609375, count: 110348"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 4455,
            "unit": "median block_count",
            "extra": "avg block_count: 4454.008989741545, max block_count: 8211.0, count: 55174"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 9,
            "unit": "median segment_count",
            "extra": "avg segment_count: 9.996429477652518, max segment_count: 29.0, count: 55174"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.681583525617156, max cpu: 9.60961, count: 55174"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 76.7421875,
            "unit": "median mem",
            "extra": "avg mem: 76.9553671254803, max mem: 105.85546875, count: 55174"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6153846,
            "unit": "median cpu",
            "extra": "avg cpu: 3.9905444200612306, max cpu: 4.6966734, count: 55174"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 63.89453125,
            "unit": "median mem",
            "extra": "avg mem: 63.50858937463751, max mem: 96.2109375, count: 55174"
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
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "4b08446356edfe8faed3798c0521304b8789d3e5",
          "message": "fix: make IndexLayerSizes calculations more accurate (#3040)\n\n## What\n\nThe `From<&PgSearchRelation> for IndexLayerSizes` `from()` ctor was\nimproperly calculating both the total `index_byte_size` and the\n`target_segment_byte_size`.\n\nFor the former, it needs to only consider SegmentMetaEntries that are\nactually visible. In almost every case, at least after a CREATE INDEX of\na large index, the index could actually have 2x+ total segments as there\nare actually visible segments.\n\nSecondly, we need to adjust the `target_segment_byte_size` down by 1/3\nto account for how `LayeredMergePolicy` does the same thing. Failure to\ndo this can cause the background worker to merge segments together that\ndon't actually need to be.\n\n## Why\n\nThese two miscalculations are what contributed to the recent query\nbenchmark regressions we saw after #3031 was merged.\n\n## How\n\n## Tests\n\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>",
          "timestamp": "2025-08-24T17:46:13Z",
          "url": "https://github.com/paradedb/paradedb/commit/4b08446356edfe8faed3798c0521304b8789d3e5"
        },
        "date": 1757447875094,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.757222843125721, max cpu: 9.467456, count: 55127"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 61.1796875,
            "unit": "median mem",
            "extra": "avg mem: 59.88205650418579, max mem: 83.17578125, count: 55127"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.649923642448752, max cpu: 9.302325, count: 55127"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 52.359375,
            "unit": "median mem",
            "extra": "avg mem: 52.1155707077521, max mem: 77.18359375, count: 55127"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.7202110639249, max cpu: 11.25, count: 55127"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 60.2421875,
            "unit": "median mem",
            "extra": "avg mem: 60.834354932700855, max mem: 84.49609375, count: 55127"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.610951,
            "unit": "median cpu",
            "extra": "avg cpu: 3.925795969053373, max cpu: 4.733728, count: 55127"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 60.94140625,
            "unit": "median mem",
            "extra": "avg mem: 60.353965970055505, max mem: 83.39453125, count: 55127"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.195402,
            "unit": "median cpu",
            "extra": "avg cpu: 7.443307235384497, max cpu: 24.096386, count: 110254"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 68.98828125,
            "unit": "median mem",
            "extra": "avg mem: 68.71458798405273, max mem: 101.26953125, count: 110254"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 3763,
            "unit": "median block_count",
            "extra": "avg block_count: 3715.969488635333, max block_count: 6646.0, count: 55127"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 9,
            "unit": "median segment_count",
            "extra": "avg segment_count: 9.897545667277377, max segment_count: 29.0, count: 55127"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 6.219181238094399, max cpu: 14.173229, count: 55127"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 77.03515625,
            "unit": "median mem",
            "extra": "avg mem: 77.33207766271518, max mem: 103.01171875, count: 55127"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.544458260618648, max cpu: 9.284333, count: 55127"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 60.2265625,
            "unit": "median mem",
            "extra": "avg mem: 59.46354476084768, max mem: 83.97265625, count: 55127"
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
          "id": "cea1e8a6034491ad9e758c9ca40b3bf03dd0ad80",
          "message": "chore: Upgrade to `0.17.10` (#3091)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-09-01T01:10:59Z",
          "url": "https://github.com/paradedb/paradedb/commit/cea1e8a6034491ad9e758c9ca40b3bf03dd0ad80"
        },
        "date": 1757447903356,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.768108857235694, max cpu: 11.44674, count: 54901"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 64.94140625,
            "unit": "median mem",
            "extra": "avg mem: 64.75015646060636, max mem: 95.17578125, count: 54901"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.671217090926292, max cpu: 9.4395275, count: 54901"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 58.3359375,
            "unit": "median mem",
            "extra": "avg mem: 58.298773303309595, max mem: 87.4609375, count: 54901"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.747015759097243, max cpu: 9.657948, count: 54901"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 64.78125,
            "unit": "median mem",
            "extra": "avg mem: 64.6159462743165, max mem: 94.34765625, count: 54901"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.573521337112461, max cpu: 4.733728, count: 54901"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 67.03125,
            "unit": "median mem",
            "extra": "avg mem: 65.84635604030437, max mem: 96.38671875, count: 54901"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 5.1946915329636605, max cpu: 13.899614, count: 109802"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 73.08984375,
            "unit": "median mem",
            "extra": "avg mem: 73.12563605256507, max mem: 102.32421875, count: 109802"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 4471,
            "unit": "median block_count",
            "extra": "avg block_count: 4433.089470137156, max block_count: 8196.0, count: 54901"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 9,
            "unit": "median segment_count",
            "extra": "avg segment_count: 10.041256079124242, max segment_count: 30.0, count: 54901"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.622403370008437, max cpu: 9.571285, count: 54901"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 77.9453125,
            "unit": "median mem",
            "extra": "avg mem: 77.08389874501376, max mem: 107.83984375, count: 54901"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.2050350200197775, max cpu: 4.6829267, count: 54901"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 60.5,
            "unit": "median mem",
            "extra": "avg mem: 61.96580727411614, max mem: 93.65234375, count: 54901"
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
          "id": "ec43ca34644c8c72e65ed2b7e9570066e839bab7",
          "message": "chore: Upgrade to `0.17.12` (#3120)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nNote that `0.17.11` was skipped because it was a hotfix in enterprise.\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-09-03T20:56:05Z",
          "url": "https://github.com/paradedb/paradedb/commit/ec43ca34644c8c72e65ed2b7e9570066e839bab7"
        },
        "date": 1757447953353,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.718990777864207, max cpu: 9.448819, count: 55242"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 63.8984375,
            "unit": "median mem",
            "extra": "avg mem: 64.87836771048296, max mem: 94.9296875, count: 55242"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.627731943353773, max cpu: 9.257474, count: 55242"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 58.28125,
            "unit": "median mem",
            "extra": "avg mem: 58.46274700974802, max mem: 87.81640625, count: 55242"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.770080968297096, max cpu: 9.448819, count: 55242"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 65.3046875,
            "unit": "median mem",
            "extra": "avg mem: 65.42833556499131, max mem: 94.75390625, count: 55242"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.619827,
            "unit": "median cpu",
            "extra": "avg cpu: 4.270927509096324, max cpu: 4.7151275, count: 55242"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 67.49609375,
            "unit": "median mem",
            "extra": "avg mem: 66.44288143532096, max mem: 96.8828125, count: 55242"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 5.157484379495918, max cpu: 14.4, count: 110484"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 74.9375,
            "unit": "median mem",
            "extra": "avg mem: 74.98711037203351, max mem: 107.79296875, count: 110484"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 4443,
            "unit": "median block_count",
            "extra": "avg block_count: 4472.860251258101, max block_count: 8180.0, count: 55242"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 9,
            "unit": "median segment_count",
            "extra": "avg segment_count: 10.017577205749248, max segment_count: 29.0, count: 55242"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.670691651590413, max cpu: 9.430255, count: 55242"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 75.828125,
            "unit": "median mem",
            "extra": "avg mem: 75.2854522487419, max mem: 104.8046875, count: 55242"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6153846,
            "unit": "median cpu",
            "extra": "avg cpu: 4.2246482933878315, max cpu: 4.64666, count: 55242"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 64.70703125,
            "unit": "median mem",
            "extra": "avg mem: 63.626327610219576, max mem: 93.546875, count: 55242"
          }
        ]
      }
    ],
    "pg_search bulk-updates.toml Performance - TPS": [
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
          "id": "1c48d3c6427e9fe7bdb10861a16e04d0cf1a1f48",
          "message": "ci: fix benchmark links in slack messages (#2875)",
          "timestamp": "2025-07-17T12:13:25Z",
          "url": "https://github.com/paradedb/paradedb/commit/1c48d3c6427e9fe7bdb10861a16e04d0cf1a1f48"
        },
        "date": 1757448549008,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 6.784904653350833,
            "unit": "median tps",
            "extra": "avg tps: 5.828211551874273, max tps: 8.816978865798157, count: 57753"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.830694573507872,
            "unit": "median tps",
            "extra": "avg tps: 5.224625625042285, max tps: 6.576885478904082, count: 57753"
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
          "id": "0544c54d64a963065cefc3a922582cc501a4c90e",
          "message": "fix: zero worker threads (#2959) (#3139)\n\nWe don't use any of Tantivy's threading features, and as of\nhttps://github.com/paradedb/tantivy/pull/59 it's now possible to set the\nnumber of merge and worker threads to zero.\n\nDoing so saves overhead of making threads that we never use, and joining\non them, for every segment merge operation.\n\n\n🍒 This is a cherry pick of 98d7dcdc33169d31d80e13ef39aa7242e1a09710 from\n`main/0.18.x`",
          "timestamp": "2025-09-09T18:06:14Z",
          "url": "https://github.com/paradedb/paradedb/commit/0544c54d64a963065cefc3a922582cc501a4c90e"
        },
        "date": 1757448557645,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.38988845607191,
            "unit": "median tps",
            "extra": "avg tps: 7.172048072172516, max tps: 11.125643270347044, count: 57677"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.3205133159313505,
            "unit": "median tps",
            "extra": "avg tps: 4.806557445337188, max tps: 5.930422337847166, count: 57677"
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
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "4b08446356edfe8faed3798c0521304b8789d3e5",
          "message": "fix: make IndexLayerSizes calculations more accurate (#3040)\n\n## What\n\nThe `From<&PgSearchRelation> for IndexLayerSizes` `from()` ctor was\nimproperly calculating both the total `index_byte_size` and the\n`target_segment_byte_size`.\n\nFor the former, it needs to only consider SegmentMetaEntries that are\nactually visible. In almost every case, at least after a CREATE INDEX of\na large index, the index could actually have 2x+ total segments as there\nare actually visible segments.\n\nSecondly, we need to adjust the `target_segment_byte_size` down by 1/3\nto account for how `LayeredMergePolicy` does the same thing. Failure to\ndo this can cause the background worker to merge segments together that\ndon't actually need to be.\n\n## Why\n\nThese two miscalculations are what contributed to the recent query\nbenchmark regressions we saw after #3031 was merged.\n\n## How\n\n## Tests\n\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>",
          "timestamp": "2025-08-24T17:46:13Z",
          "url": "https://github.com/paradedb/paradedb/commit/4b08446356edfe8faed3798c0521304b8789d3e5"
        },
        "date": 1757448570225,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 7.812210265053789,
            "unit": "median tps",
            "extra": "avg tps: 6.700796823875249, max tps: 10.3032474302258, count: 57789"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.421519055215924,
            "unit": "median tps",
            "extra": "avg tps: 4.890796828030142, max tps: 6.028286925084276, count: 57789"
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
          "id": "8cd6a2c7cdf969cf43bd66f12beca6ddedd6889a",
          "message": "chore: Upgrade to `0.17.5` (#3005)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-08-20T17:19:49Z",
          "url": "https://github.com/paradedb/paradedb/commit/8cd6a2c7cdf969cf43bd66f12beca6ddedd6889a"
        },
        "date": 1757448637683,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 6.621207233775652,
            "unit": "median tps",
            "extra": "avg tps: 5.708194457472612, max tps: 8.616493528622854, count: 57435"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.935776050860554,
            "unit": "median tps",
            "extra": "avg tps: 5.299040036463077, max tps: 6.668166281418053, count: 57435"
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
          "id": "cea1e8a6034491ad9e758c9ca40b3bf03dd0ad80",
          "message": "chore: Upgrade to `0.17.10` (#3091)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-09-01T01:10:59Z",
          "url": "https://github.com/paradedb/paradedb/commit/cea1e8a6034491ad9e758c9ca40b3bf03dd0ad80"
        },
        "date": 1757448671033,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.443446116947367,
            "unit": "median tps",
            "extra": "avg tps: 7.187260880754454, max tps: 11.080919252087979, count: 57577"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.348360655948239,
            "unit": "median tps",
            "extra": "avg tps: 4.837101597431798, max tps: 5.943103452696158, count: 57577"
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
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "1c48d3c6427e9fe7bdb10861a16e04d0cf1a1f48",
          "message": "ci: fix benchmark links in slack messages (#2875)",
          "timestamp": "2025-07-17T12:13:25Z",
          "url": "https://github.com/paradedb/paradedb/commit/1c48d3c6427e9fe7bdb10861a16e04d0cf1a1f48"
        },
        "date": 1757448551819,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.233301,
            "unit": "median cpu",
            "extra": "avg cpu: 21.35483005443725, max cpu: 42.857143, count: 57753"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 229.56640625,
            "unit": "median mem",
            "extra": "avg mem: 228.95327407234257, max mem: 235.77734375, count: 57753"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.30097,
            "unit": "median cpu",
            "extra": "avg cpu: 22.26818812009849, max cpu: 33.300297, count: 57753"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 160.19921875,
            "unit": "median mem",
            "extra": "avg mem: 159.8026199667117, max mem: 161.37109375, count: 57753"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 22523,
            "unit": "median block_count",
            "extra": "avg block_count: 20762.645334441502, max block_count: 23827.0, count: 57753"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 67,
            "unit": "median segment_count",
            "extra": "avg segment_count: 68.62156078472114, max segment_count: 97.0, count: 57753"
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
          "id": "0544c54d64a963065cefc3a922582cc501a4c90e",
          "message": "fix: zero worker threads (#2959) (#3139)\n\nWe don't use any of Tantivy's threading features, and as of\nhttps://github.com/paradedb/tantivy/pull/59 it's now possible to set the\nnumber of merge and worker threads to zero.\n\nDoing so saves overhead of making threads that we never use, and joining\non them, for every segment merge operation.\n\n\n🍒 This is a cherry pick of 98d7dcdc33169d31d80e13ef39aa7242e1a09710 from\n`main/0.18.x`",
          "timestamp": "2025-09-09T18:06:14Z",
          "url": "https://github.com/paradedb/paradedb/commit/0544c54d64a963065cefc3a922582cc501a4c90e"
        },
        "date": 1757448560430,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.720749,
            "unit": "median cpu",
            "extra": "avg cpu: 19.36071535868993, max cpu: 42.687748, count: 57677"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 229.14453125,
            "unit": "median mem",
            "extra": "avg mem: 229.20659272218563, max mem: 230.16015625, count: 57677"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.323614,
            "unit": "median cpu",
            "extra": "avg cpu: 22.4420193341431, max cpu: 33.267326, count: 57677"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 161.0234375,
            "unit": "median mem",
            "extra": "avg mem: 161.01764459836676, max mem: 162.5234375, count: 57677"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 24196,
            "unit": "median block_count",
            "extra": "avg block_count: 22939.148031277633, max block_count: 25909.0, count: 57677"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 71,
            "unit": "median segment_count",
            "extra": "avg segment_count: 72.74573920280181, max segment_count: 107.0, count: 57677"
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
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "4b08446356edfe8faed3798c0521304b8789d3e5",
          "message": "fix: make IndexLayerSizes calculations more accurate (#3040)\n\n## What\n\nThe `From<&PgSearchRelation> for IndexLayerSizes` `from()` ctor was\nimproperly calculating both the total `index_byte_size` and the\n`target_segment_byte_size`.\n\nFor the former, it needs to only consider SegmentMetaEntries that are\nactually visible. In almost every case, at least after a CREATE INDEX of\na large index, the index could actually have 2x+ total segments as there\nare actually visible segments.\n\nSecondly, we need to adjust the `target_segment_byte_size` down by 1/3\nto account for how `LayeredMergePolicy` does the same thing. Failure to\ndo this can cause the background worker to merge segments together that\ndon't actually need to be.\n\n## Why\n\nThese two miscalculations are what contributed to the recent query\nbenchmark regressions we saw after #3031 was merged.\n\n## How\n\n## Tests\n\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>",
          "timestamp": "2025-08-24T17:46:13Z",
          "url": "https://github.com/paradedb/paradedb/commit/4b08446356edfe8faed3798c0521304b8789d3e5"
        },
        "date": 1757448572944,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.143684,
            "unit": "median cpu",
            "extra": "avg cpu: 20.373953875219957, max cpu: 42.857143, count: 57789"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 226.0234375,
            "unit": "median mem",
            "extra": "avg mem: 224.66223317532317, max mem: 237.6640625, count: 57789"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.323614,
            "unit": "median cpu",
            "extra": "avg cpu: 22.480686038332465, max cpu: 33.333336, count: 57789"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 160.0390625,
            "unit": "median mem",
            "extra": "avg mem: 159.94515013670423, max mem: 162.08984375, count: 57789"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 23392,
            "unit": "median block_count",
            "extra": "avg block_count: 21942.760283098858, max block_count: 25301.0, count: 57789"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 50,
            "unit": "median segment_count",
            "extra": "avg segment_count: 49.15387011368946, max segment_count: 71.0, count: 57789"
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
          "id": "8cd6a2c7cdf969cf43bd66f12beca6ddedd6889a",
          "message": "chore: Upgrade to `0.17.5` (#3005)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-08-20T17:19:49Z",
          "url": "https://github.com/paradedb/paradedb/commit/8cd6a2c7cdf969cf43bd66f12beca6ddedd6889a"
        },
        "date": 1757448640824,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.233301,
            "unit": "median cpu",
            "extra": "avg cpu: 21.566918462833122, max cpu: 42.64561, count: 57435"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 226.6796875,
            "unit": "median mem",
            "extra": "avg mem: 225.40428174240446, max mem: 232.3671875, count: 57435"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.27837,
            "unit": "median cpu",
            "extra": "avg cpu: 22.182783635642203, max cpu: 33.267326, count: 57435"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 159.52734375,
            "unit": "median mem",
            "extra": "avg mem: 159.51831628961, max mem: 161.02734375, count: 57435"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 22187,
            "unit": "median block_count",
            "extra": "avg block_count: 20688.441890833117, max block_count: 23636.0, count: 57435"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 67,
            "unit": "median segment_count",
            "extra": "avg segment_count: 68.34926438582745, max segment_count: 94.0, count: 57435"
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
          "id": "cea1e8a6034491ad9e758c9ca40b3bf03dd0ad80",
          "message": "chore: Upgrade to `0.17.10` (#3091)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-09-01T01:10:59Z",
          "url": "https://github.com/paradedb/paradedb/commit/cea1e8a6034491ad9e758c9ca40b3bf03dd0ad80"
        },
        "date": 1757448674336,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.713451,
            "unit": "median cpu",
            "extra": "avg cpu: 19.311264442334153, max cpu: 42.772278, count: 57577"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 229.65625,
            "unit": "median mem",
            "extra": "avg mem: 229.4614318787233, max mem: 231.33203125, count: 57577"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.30097,
            "unit": "median cpu",
            "extra": "avg cpu: 22.503433388116903, max cpu: 33.267326, count: 57577"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 161.48828125,
            "unit": "median mem",
            "extra": "avg mem: 161.01219392889087, max mem: 164.765625, count: 57577"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 24164,
            "unit": "median block_count",
            "extra": "avg block_count: 22977.285478576516, max block_count: 25692.0, count: 57577"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 71,
            "unit": "median segment_count",
            "extra": "avg segment_count: 72.66347673550203, max segment_count: 108.0, count: 57577"
          }
        ]
      }
    ]
  }
}