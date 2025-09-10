window.BENCHMARK_DATA = {
  "lastUpdate": 1757515075349,
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
          "id": "1c48d3c6427e9fe7bdb10861a16e04d0cf1a1f48",
          "message": "ci: fix benchmark links in slack messages (#2875)",
          "timestamp": "2025-07-17T12:13:25Z",
          "url": "https://github.com/paradedb/paradedb/commit/1c48d3c6427e9fe7bdb10861a16e04d0cf1a1f48"
        },
        "date": 1757449911042,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Delete value - Primary - tps",
            "value": 257.37922341132156,
            "unit": "median tps",
            "extra": "avg tps: 253.80006203166934, max tps: 638.2489232874052, count: 55469"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 137.78066152740456,
            "unit": "median tps",
            "extra": "avg tps: 134.1683484392248, max tps: 142.0512735389117, count: 55469"
          },
          {
            "name": "Monitor Segment Count - Primary - tps",
            "value": 1966.8896077373029,
            "unit": "median tps",
            "extra": "avg tps: 1953.0853917269808, max tps: 2037.2262274171694, count: 55469"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 18.706824150608274,
            "unit": "median tps",
            "extra": "avg tps: 23.379214572890387, max tps: 76.86792228713726, count: 166407"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 0.39684704292763207,
            "unit": "median tps",
            "extra": "avg tps: 0.6374368371601701, max tps: 5.008225638571735, count: 55469"
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
        "date": 1757449977724,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Delete value - Primary - tps",
            "value": 239.43997264071243,
            "unit": "median tps",
            "extra": "avg tps: 232.59771233327746, max tps: 467.4726351734924, count: 55414"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 441.330468670844,
            "unit": "median tps",
            "extra": "avg tps: 429.2415504769092, max tps: 467.44064658231, count: 55414"
          },
          {
            "name": "Monitor Segment Count - Primary - tps",
            "value": 1821.0405320870975,
            "unit": "median tps",
            "extra": "avg tps: 1810.4571194881944, max tps: 1926.9464892014237, count: 55414"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 20.578303380147254,
            "unit": "median tps",
            "extra": "avg tps: 42.18483800581096, max tps: 176.91247947912382, count: 166242"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 1.2257045926245544,
            "unit": "median tps",
            "extra": "avg tps: 1.436526869078975, max tps: 4.570195938223735, count: 55414"
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
        "date": 1757450018597,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Delete value - Primary - tps",
            "value": 17.203219850975373,
            "unit": "median tps",
            "extra": "avg tps: 38.91591464546438, max tps: 600.1211098956234, count: 55432"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 125.20829220139623,
            "unit": "median tps",
            "extra": "avg tps: 126.36501934279954, max tps: 131.90130808854295, count: 55432"
          },
          {
            "name": "Monitor Segment Count - Primary - tps",
            "value": 1960.8919891491348,
            "unit": "median tps",
            "extra": "avg tps: 1965.3882273494087, max tps: 2079.8661738078085, count: 55432"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 8.345706922694436,
            "unit": "median tps",
            "extra": "avg tps: 11.0358818139187, max tps: 72.75845306510419, count: 166296"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 0.313239370126525,
            "unit": "median tps",
            "extra": "avg tps: 0.6122699922746802, max tps: 4.869862830094795, count: 55432"
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
        "date": 1757450089779,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Delete value - Primary - tps",
            "value": 118.24031689524347,
            "unit": "median tps",
            "extra": "avg tps: 127.01234940050858, max tps: 426.2041672790953, count: 55482"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 430.0253332759028,
            "unit": "median tps",
            "extra": "avg tps: 419.8378119299461, max tps: 466.88179331192026, count: 55482"
          },
          {
            "name": "Monitor Segment Count - Primary - tps",
            "value": 1782.6648662582013,
            "unit": "median tps",
            "extra": "avg tps: 1785.9762820443725, max tps: 1985.7481202618706, count: 55482"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 20.25967331449874,
            "unit": "median tps",
            "extra": "avg tps: 29.30311856226434, max tps: 140.5196907664203, count: 166446"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 1.369970597301969,
            "unit": "median tps",
            "extra": "avg tps: 1.6614588556930197, max tps: 5.0947523089765, count: 55482"
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
        "date": 1757450178082,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Delete value - Primary - tps",
            "value": 61.626112774021365,
            "unit": "median tps",
            "extra": "avg tps: 79.29882369844694, max tps: 457.98714832734487, count: 55394"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 406.8878489366183,
            "unit": "median tps",
            "extra": "avg tps: 400.6808987209089, max tps: 445.98487300789725, count: 55394"
          },
          {
            "name": "Monitor Segment Count - Primary - tps",
            "value": 1757.4602547035747,
            "unit": "median tps",
            "extra": "avg tps: 1751.2677387356664, max tps: 1831.190174941316, count: 55394"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 20.686666202642595,
            "unit": "median tps",
            "extra": "avg tps: 21.990935467843173, max tps: 140.4036664451841, count: 166182"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 1.483394963593412,
            "unit": "median tps",
            "extra": "avg tps: 1.657874378120931, max tps: 4.992558413379201, count: 55394"
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
        "date": 1757451728263,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Delete value - Primary - tps",
            "value": 121.98702404440476,
            "unit": "median tps",
            "extra": "avg tps: 132.66414329216585, max tps: 523.5086356331054, count: 55468"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 143.56706327479247,
            "unit": "median tps",
            "extra": "avg tps: 142.83339215574082, max tps: 149.67775753804793, count: 55468"
          },
          {
            "name": "Monitor Segment Count - Primary - tps",
            "value": 1848.4187661107396,
            "unit": "median tps",
            "extra": "avg tps: 1860.5897622584635, max tps: 1945.3445493647484, count: 55468"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 19.279319482593763,
            "unit": "median tps",
            "extra": "avg tps: 20.761881965900653, max tps: 86.53869750850522, count: 166404"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 1.2199328067018052,
            "unit": "median tps",
            "extra": "avg tps: 1.4014335255228976, max tps: 4.8123952788198245, count: 55468"
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
          "id": "1c48d3c6427e9fe7bdb10861a16e04d0cf1a1f48",
          "message": "ci: fix benchmark links in slack messages (#2875)",
          "timestamp": "2025-07-17T12:13:25Z",
          "url": "https://github.com/paradedb/paradedb/commit/1c48d3c6427e9fe7bdb10861a16e04d0cf1a1f48"
        },
        "date": 1757449914230,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 7.614108289411669, max cpu: 32.71665, count: 55469"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 205.98828125,
            "unit": "median mem",
            "extra": "avg mem: 204.35682649035948, max mem: 205.98828125, count: 55469"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.320388,
            "unit": "median cpu",
            "extra": "avg cpu: 11.035495212142893, max cpu: 23.346306, count: 55469"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 164.60546875,
            "unit": "median mem",
            "extra": "avg mem: 160.27758149540733, max mem: 172.6171875, count: 55469"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 39298,
            "unit": "median block_count",
            "extra": "avg block_count: 40514.28520434837, max block_count: 57558.0, count: 55469"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 3.581360806301311, max cpu: 4.669261, count: 55469"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 116.73828125,
            "unit": "median mem",
            "extra": "avg mem: 105.34452922184013, max mem: 136.25390625, count: 55469"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 25,
            "unit": "median segment_count",
            "extra": "avg segment_count: 25.93836196794606, max segment_count: 54.0, count: 55469"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 23.099133,
            "unit": "median cpu",
            "extra": "avg cpu: 20.2987849109009, max cpu: 32.876713, count: 166407"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 220.1171875,
            "unit": "median mem",
            "extra": "avg mem: 235.90730476050436, max mem: 449.1171875, count: 166407"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 18.640776,
            "unit": "median cpu",
            "extra": "avg cpu: 17.776143115991996, max cpu: 32.684826, count: 55469"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 193.9375,
            "unit": "median mem",
            "extra": "avg mem: 192.5084291873614, max mem: 224.66015625, count: 55469"
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
        "date": 1757449980469,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 7.523528947424538, max cpu: 32.495163, count: 55414"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 206.0390625,
            "unit": "median mem",
            "extra": "avg mem: 204.3654832826542, max mem: 206.0390625, count: 55414"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 5.263671587621379, max cpu: 13.994169, count: 55414"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 161.38671875,
            "unit": "median mem",
            "extra": "avg mem: 151.2624746369374, max mem: 162.53125, count: 55414"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 49461,
            "unit": "median block_count",
            "extra": "avg block_count: 48301.005684484066, max block_count: 75553.0, count: 55414"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6153846,
            "unit": "median cpu",
            "extra": "avg cpu: 4.572083249339663, max cpu: 4.660194, count: 55414"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 130.1640625,
            "unit": "median mem",
            "extra": "avg mem: 115.55397975917639, max mem: 140.3203125, count: 55414"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 35,
            "unit": "median segment_count",
            "extra": "avg segment_count: 35.90917457682174, max segment_count: 65.0, count: 55414"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 23.032629,
            "unit": "median cpu",
            "extra": "avg cpu: 18.443299382050753, max cpu: 32.78049, count: 166242"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 220.28125,
            "unit": "median mem",
            "extra": "avg mem: 271.3721940144112, max mem: 497.7265625, count: 166242"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 23.143684,
            "unit": "median cpu",
            "extra": "avg cpu: 20.88384386167533, max cpu: 32.621357, count: 55414"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 192.1875,
            "unit": "median mem",
            "extra": "avg mem: 190.18504017768524, max mem: 222.56640625, count: 55414"
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
        "date": 1757450021760,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Delete value - Primary - cpu",
            "value": 18.60465,
            "unit": "median cpu",
            "extra": "avg cpu: 15.404292704459149, max cpu: 32.65306, count: 55432"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 204.0859375,
            "unit": "median mem",
            "extra": "avg mem: 202.52082365555995, max mem: 204.0859375, count: 55432"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 13.806328,
            "unit": "median cpu",
            "extra": "avg cpu: 11.695721374936051, max cpu: 23.188406, count: 55432"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 157.65234375,
            "unit": "median mem",
            "extra": "avg mem: 154.1930837698667, max mem: 164.12890625, count: 55432"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 37132,
            "unit": "median block_count",
            "extra": "avg block_count: 39637.99585077212, max block_count: 55068.0, count: 55432"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6153846,
            "unit": "median cpu",
            "extra": "avg cpu: 4.133324530861887, max cpu: 4.6376815, count: 55432"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 116.453125,
            "unit": "median mem",
            "extra": "avg mem: 106.35052237426035, max mem: 138.1875, count: 55432"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 25,
            "unit": "median segment_count",
            "extra": "avg segment_count: 25.316820609034494, max segment_count: 53.0, count: 55432"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 23.121387,
            "unit": "median cpu",
            "extra": "avg cpu: 21.73982378793115, max cpu: 32.71665, count: 166296"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 213.6953125,
            "unit": "median mem",
            "extra": "avg mem: 232.30759172462055, max mem: 454.65234375, count: 166296"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.953489,
            "unit": "median cpu",
            "extra": "avg cpu: 15.388408213138192, max cpu: 32.55814, count: 55432"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 192.77734375,
            "unit": "median mem",
            "extra": "avg mem: 191.02652179697648, max mem: 227.41015625, count: 55432"
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
        "date": 1757450092538,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 9.502869740412404, max cpu: 32.526623, count: 55482"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 203.9453125,
            "unit": "median mem",
            "extra": "avg mem: 202.33106464990897, max mem: 203.9453125, count: 55482"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 5.327565362901336, max cpu: 13.93998, count: 55482"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 160.890625,
            "unit": "median mem",
            "extra": "avg mem: 152.4138124188926, max mem: 162.015625, count: 55482"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 48812,
            "unit": "median block_count",
            "extra": "avg block_count: 47626.5678778703, max block_count: 74365.0, count: 55482"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6153846,
            "unit": "median cpu",
            "extra": "avg cpu: 3.678634217878743, max cpu: 4.6421666, count: 55482"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 132.05078125,
            "unit": "median mem",
            "extra": "avg mem: 116.16916311258606, max mem: 141.4375, count: 55482"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 35,
            "unit": "median segment_count",
            "extra": "avg segment_count: 35.977632385278106, max segment_count: 62.0, count: 55482"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 23.054754,
            "unit": "median cpu",
            "extra": "avg cpu: 19.02618626553094, max cpu: 32.8125, count: 166446"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 223.359375,
            "unit": "median mem",
            "extra": "avg mem: 270.4905002739252, max mem: 497.25, count: 166446"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 23.121387,
            "unit": "median cpu",
            "extra": "avg cpu: 20.589422732787803, max cpu: 32.495163, count: 55482"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 191.70703125,
            "unit": "median mem",
            "extra": "avg mem: 190.1631332853448, max mem: 222.09375, count: 55482"
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
        "date": 1757450181438,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 11.41932387033128, max cpu: 32.589718, count: 55394"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 205.1640625,
            "unit": "median mem",
            "extra": "avg mem: 203.54162743539462, max mem: 205.1640625, count: 55394"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 5.386292933443959, max cpu: 13.967022, count: 55394"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 161.64453125,
            "unit": "median mem",
            "extra": "avg mem: 151.15585002437086, max mem: 162.39453125, count: 55394"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 45561,
            "unit": "median block_count",
            "extra": "avg block_count: 45919.807632595584, max block_count: 73150.0, count: 55394"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.558979402140836, max cpu: 4.655674, count: 55394"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 129.390625,
            "unit": "median mem",
            "extra": "avg mem: 113.14798463619707, max mem: 140.65234375, count: 55394"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 35,
            "unit": "median segment_count",
            "extra": "avg segment_count: 35.663248727298985, max segment_count: 67.0, count: 55394"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 23.099133,
            "unit": "median cpu",
            "extra": "avg cpu: 19.624337054618564, max cpu: 32.65306, count: 166182"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 222.08203125,
            "unit": "median mem",
            "extra": "avg mem: 268.6127629694025, max mem: 493.671875, count: 166182"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 23.121387,
            "unit": "median cpu",
            "extra": "avg cpu: 20.688015276502032, max cpu: 32.589718, count: 55394"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 199.33203125,
            "unit": "median mem",
            "extra": "avg mem: 197.48555722697856, max mem: 229.8671875, count: 55394"
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
        "date": 1757451731516,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 9.565648435332964, max cpu: 32.621357, count: 55468"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 204.41796875,
            "unit": "median mem",
            "extra": "avg mem: 202.80740084654667, max mem: 204.41796875, count: 55468"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.29332,
            "unit": "median cpu",
            "extra": "avg cpu: 10.276845558352472, max cpu: 23.143684, count: 55468"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 162.54296875,
            "unit": "median mem",
            "extra": "avg mem: 156.72617721310036, max mem: 173.39453125, count: 55468"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 45999,
            "unit": "median block_count",
            "extra": "avg block_count: 41194.93147400303, max block_count: 58450.0, count: 55468"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.380083267368517, max cpu: 4.669261, count: 55468"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 133.55078125,
            "unit": "median mem",
            "extra": "avg mem: 117.81137118192021, max mem: 141.49609375, count: 55468"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 29,
            "unit": "median segment_count",
            "extra": "avg segment_count: 31.16537463041754, max segment_count: 140.0, count: 55468"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 23.099133,
            "unit": "median cpu",
            "extra": "avg cpu: 20.256610165128965, max cpu: 32.684826, count: 166404"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 226.17578125,
            "unit": "median mem",
            "extra": "avg mem: 234.6712632546393, max mem: 452.08984375, count: 166404"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 23.010548,
            "unit": "median cpu",
            "extra": "avg cpu: 18.106960671403435, max cpu: 32.55814, count: 55468"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 192.7734375,
            "unit": "median mem",
            "extra": "avg mem: 191.10849815715818, max mem: 223.1640625, count: 55468"
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
        "date": 1757449660171,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 1180.0214203271923,
            "unit": "median tps",
            "extra": "avg tps: 1176.0767980187588, max tps: 1183.743768786506, count: 55327"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2793.147810286606,
            "unit": "median tps",
            "extra": "avg tps: 2785.4420762597665, max tps: 2839.016217260532, count: 55327"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 1220.9690308473207,
            "unit": "median tps",
            "extra": "avg tps: 1215.0927490108131, max tps: 1224.2715878198649, count: 55327"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 966.8583786376862,
            "unit": "median tps",
            "extra": "avg tps: 966.4580882580532, max tps: 998.9358259804612, count: 55327"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 173.95875174820912,
            "unit": "median tps",
            "extra": "avg tps: 179.38478277040537, max tps: 188.8999478715099, count: 110654"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 151.57107576346408,
            "unit": "median tps",
            "extra": "avg tps: 150.79367813642259, max tps: 152.1667896254459, count: 55327"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 43.280328066742605,
            "unit": "median tps",
            "extra": "avg tps: 46.03327070212798, max tps: 815.3361467865972, count: 55327"
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
          "id": "125c2366536d456d1e0df1222b61accfdec48abc",
          "message": "chore: proptest for our pg_sys::TransactionIdPrecedesOrEquals function (#3140)\n\n## What\n\nAdd a proptest for our port of\n`pg_sys::TransactionIdPrecedesOrEquals()`.\n\n## Why\n\nThere was a moment of internal confusion around if it was actually\ncorrect or not.\n\n## How\n\n## Tests\n\nThis is a test.",
          "timestamp": "2025-09-10T09:58:41-04:00",
          "tree_id": "e61581d3b8eee310986ffc3e57941a858ac39e0a",
          "url": "https://github.com/paradedb/paradedb/commit/125c2366536d456d1e0df1222b61accfdec48abc"
        },
        "date": 1757513697404,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 706.4953127713309,
            "unit": "median tps",
            "extra": "avg tps: 707.4539530885796, max tps: 809.8993706973903, count: 55068"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2607.5203088097196,
            "unit": "median tps",
            "extra": "avg tps: 2594.4173165575808, max tps: 2615.4853241440437, count: 55068"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 765.8565673213077,
            "unit": "median tps",
            "extra": "avg tps: 765.1143092906575, max tps: 829.8917606755375, count: 55068"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 622.570919137133,
            "unit": "median tps",
            "extra": "avg tps: 624.476962276826, max tps: 688.6067235615577, count: 55068"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 505.58392348496136,
            "unit": "median tps",
            "extra": "avg tps: 520.1207673613534, max tps: 613.2207338400345, count: 110136"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 379.54908966990433,
            "unit": "median tps",
            "extra": "avg tps: 378.34324315159466, max tps: 417.124687790536, count: 55068"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 32.97180504703129,
            "unit": "median tps",
            "extra": "avg tps: 34.404727724605145, max tps: 812.7563738386727, count: 55068"
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
        "date": 1757449663331,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.760220334824922, max cpu: 9.495549, count: 55327"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 59.30859375,
            "unit": "median mem",
            "extra": "avg mem: 58.93795101171219, max mem: 82.89453125, count: 55327"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.718021376877034, max cpu: 9.29332, count: 55327"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 54.24609375,
            "unit": "median mem",
            "extra": "avg mem: 53.575325027789326, max mem: 77.14453125, count: 55327"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.788893680703068, max cpu: 9.476802, count: 55327"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 60.7578125,
            "unit": "median mem",
            "extra": "avg mem: 60.15461088166718, max mem: 85.48046875, count: 55327"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6153846,
            "unit": "median cpu",
            "extra": "avg cpu: 4.494212218189905, max cpu: 4.7058825, count: 55327"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 60.98828125,
            "unit": "median mem",
            "extra": "avg mem: 60.676939420965354, max mem: 84.5078125, count: 55327"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.195402,
            "unit": "median cpu",
            "extra": "avg cpu: 7.2019877227579645, max cpu: 23.529411, count: 110654"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 65.41796875,
            "unit": "median mem",
            "extra": "avg mem: 64.7024796889403, max mem: 93.51953125, count: 110654"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 3756,
            "unit": "median block_count",
            "extra": "avg block_count: 3742.06118170152, max block_count: 6767.0, count: 55327"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 9,
            "unit": "median segment_count",
            "extra": "avg segment_count: 9.959404992137655, max segment_count: 31.0, count: 55327"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 6.180771164804131, max cpu: 18.916256, count: 55327"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 68.87890625,
            "unit": "median mem",
            "extra": "avg mem: 68.83307320849224, max mem: 94.79296875, count: 55327"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 6.139461597780147, max cpu: 9.213051, count: 55327"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 59.38671875,
            "unit": "median mem",
            "extra": "avg mem: 58.53888705164296, max mem: 84.015625, count: 55327"
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
          "id": "125c2366536d456d1e0df1222b61accfdec48abc",
          "message": "chore: proptest for our pg_sys::TransactionIdPrecedesOrEquals function (#3140)\n\n## What\n\nAdd a proptest for our port of\n`pg_sys::TransactionIdPrecedesOrEquals()`.\n\n## Why\n\nThere was a moment of internal confusion around if it was actually\ncorrect or not.\n\n## How\n\n## Tests\n\nThis is a test.",
          "timestamp": "2025-09-10T09:58:41-04:00",
          "tree_id": "e61581d3b8eee310986ffc3e57941a858ac39e0a",
          "url": "https://github.com/paradedb/paradedb/commit/125c2366536d456d1e0df1222b61accfdec48abc"
        },
        "date": 1757513700484,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.902243613773916, max cpu: 14.4723625, count: 55068"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 154.25,
            "unit": "median mem",
            "extra": "avg mem: 142.46657413901903, max mem: 154.25, count: 55068"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.6348350535138305, max cpu: 9.284333, count: 55068"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 147.50390625,
            "unit": "median mem",
            "extra": "avg mem: 135.2751281654046, max mem: 147.50390625, count: 55068"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.852807176058607, max cpu: 13.9265, count: 55068"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 153.671875,
            "unit": "median mem",
            "extra": "avg mem: 141.93632332979226, max mem: 153.671875, count: 55068"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.555972670673722, max cpu: 4.824121, count: 55068"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 155.85546875,
            "unit": "median mem",
            "extra": "avg mem: 143.95960236094464, max mem: 155.85546875, count: 55068"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 5.100659955910161, max cpu: 14.385615, count: 110136"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 160.48046875,
            "unit": "median mem",
            "extra": "avg mem: 150.95391691862787, max mem: 168.12109375, count: 110136"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 40224,
            "unit": "median block_count",
            "extra": "avg block_count: 40575.48904990194, max block_count: 81745.0, count: 55068"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 33,
            "unit": "median segment_count",
            "extra": "avg segment_count: 32.821311832643275, max segment_count: 76.0, count: 55068"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.804161548774489, max cpu: 9.590409, count: 55068"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 162.6953125,
            "unit": "median mem",
            "extra": "avg mem: 150.16057866807856, max mem: 167.765625, count: 55068"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 6.137497038081621, max cpu: 13.967022, count: 55068"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 152.765625,
            "unit": "median mem",
            "extra": "avg mem: 137.25322222354635, max mem: 157.97265625, count: 55068"
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
        "date": 1757448732384,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.248873171046245,
            "unit": "median tps",
            "extra": "avg tps: 7.037475620052531, max tps: 10.969906378675118, count: 57952"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.406206678433305,
            "unit": "median tps",
            "extra": "avg tps: 4.881537452284274, max tps: 5.9868096262095705, count: 57952"
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
        "date": 1757450355108,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 7.545519227602616,
            "unit": "median tps",
            "extra": "avg tps: 6.493084470419602, max tps: 10.041132919182576, count: 57307"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.64359594119954,
            "unit": "median tps",
            "extra": "avg tps: 5.072704011201073, max tps: 6.295684443590618, count: 57307"
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
          "id": "125c2366536d456d1e0df1222b61accfdec48abc",
          "message": "chore: proptest for our pg_sys::TransactionIdPrecedesOrEquals function (#3140)\n\n## What\n\nAdd a proptest for our port of\n`pg_sys::TransactionIdPrecedesOrEquals()`.\n\n## Why\n\nThere was a moment of internal confusion around if it was actually\ncorrect or not.\n\n## How\n\n## Tests\n\nThis is a test.",
          "timestamp": "2025-09-10T09:58:41-04:00",
          "tree_id": "e61581d3b8eee310986ffc3e57941a858ac39e0a",
          "url": "https://github.com/paradedb/paradedb/commit/125c2366536d456d1e0df1222b61accfdec48abc"
        },
        "date": 1757514398617,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.135857355843761,
            "unit": "median tps",
            "extra": "avg tps: 6.968269342554377, max tps: 10.850643344735662, count: 57384"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.424470534289824,
            "unit": "median tps",
            "extra": "avg tps: 4.880357916063021, max tps: 6.0403099185790055, count: 57384"
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
        "date": 1757448735147,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.867926,
            "unit": "median cpu",
            "extra": "avg cpu: 19.569693143771666, max cpu: 42.814667, count: 57952"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 230.55859375,
            "unit": "median mem",
            "extra": "avg mem: 230.54507003101705, max mem: 231.02734375, count: 57952"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.30097,
            "unit": "median cpu",
            "extra": "avg cpu: 22.40874354805704, max cpu: 33.300297, count: 57952"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 162.15625,
            "unit": "median mem",
            "extra": "avg mem: 161.6593856095346, max mem: 163.65234375, count: 57952"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 24153,
            "unit": "median block_count",
            "extra": "avg block_count: 22920.18658545003, max block_count: 25703.0, count: 57952"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 71,
            "unit": "median segment_count",
            "extra": "avg segment_count: 72.4574647984539, max segment_count: 107.0, count: 57952"
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
        "date": 1757450357828,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.188406,
            "unit": "median cpu",
            "extra": "avg cpu: 20.751036326466835, max cpu: 42.64561, count: 57307"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 225.7265625,
            "unit": "median mem",
            "extra": "avg mem: 224.109070172492, max mem: 237.84765625, count: 57307"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.323614,
            "unit": "median cpu",
            "extra": "avg cpu: 22.37629778199888, max cpu: 33.267326, count: 57307"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 159.640625,
            "unit": "median mem",
            "extra": "avg mem: 159.32304088342175, max mem: 161.16796875, count: 57307"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 23461,
            "unit": "median block_count",
            "extra": "avg block_count: 22002.937808644667, max block_count: 25123.0, count: 57307"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 50,
            "unit": "median segment_count",
            "extra": "avg segment_count: 48.79885528818469, max segment_count: 70.0, count: 57307"
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
          "id": "125c2366536d456d1e0df1222b61accfdec48abc",
          "message": "chore: proptest for our pg_sys::TransactionIdPrecedesOrEquals function (#3140)\n\n## What\n\nAdd a proptest for our port of\n`pg_sys::TransactionIdPrecedesOrEquals()`.\n\n## Why\n\nThere was a moment of internal confusion around if it was actually\ncorrect or not.\n\n## How\n\n## Tests\n\nThis is a test.",
          "timestamp": "2025-09-10T09:58:41-04:00",
          "tree_id": "e61581d3b8eee310986ffc3e57941a858ac39e0a",
          "url": "https://github.com/paradedb/paradedb/commit/125c2366536d456d1e0df1222b61accfdec48abc"
        },
        "date": 1757514401205,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.972332,
            "unit": "median cpu",
            "extra": "avg cpu: 19.841786622203816, max cpu: 42.687748, count: 57384"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 229.53125,
            "unit": "median mem",
            "extra": "avg mem: 229.44987421635386, max mem: 230.65625, count: 57384"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.323614,
            "unit": "median cpu",
            "extra": "avg cpu: 22.341425294769977, max cpu: 33.267326, count: 57384"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 160.6484375,
            "unit": "median mem",
            "extra": "avg mem: 159.9087016851387, max mem: 164.56640625, count: 57384"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 24099,
            "unit": "median block_count",
            "extra": "avg block_count: 22860.479123100515, max block_count: 25753.0, count: 57384"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 71,
            "unit": "median segment_count",
            "extra": "avg segment_count: 72.4560330405688, max segment_count: 105.0, count: 57384"
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
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "1c48d3c6427e9fe7bdb10861a16e04d0cf1a1f48",
          "message": "ci: fix benchmark links in slack messages (#2875)",
          "timestamp": "2025-07-17T12:13:25Z",
          "url": "https://github.com/paradedb/paradedb/commit/1c48d3c6427e9fe7bdb10861a16e04d0cf1a1f48"
        },
        "date": 1757449216651,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 27.96517152266014,
            "unit": "median tps",
            "extra": "avg tps: 27.820052620517938, max tps: 28.140561729684794, count: 57309"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 138.5397352587379,
            "unit": "median tps",
            "extra": "avg tps: 137.89773191316308, max tps: 139.76222000561887, count: 57309"
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
        "date": 1757449230161,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 33.08612784754656,
            "unit": "median tps",
            "extra": "avg tps: 33.078519693092346, max tps: 33.87998249510987, count: 56745"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 390.19092366255484,
            "unit": "median tps",
            "extra": "avg tps: 388.5500916426741, max tps: 403.6978283824243, count: 56745"
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
        "date": 1757449305859,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 27.26820541651433,
            "unit": "median tps",
            "extra": "avg tps: 27.21181307723961, max tps: 27.573377730696283, count: 56572"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 137.26044509721117,
            "unit": "median tps",
            "extra": "avg tps: 136.89737555272274, max tps: 140.04881848769728, count: 56572"
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
        "date": 1757449394057,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 34.56101021936161,
            "unit": "median tps",
            "extra": "avg tps: 34.438235801284684, max tps: 34.87594572071984, count: 57486"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 365.95319458109077,
            "unit": "median tps",
            "extra": "avg tps: 365.48141864604014, max tps: 389.83647663828344, count: 57486"
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
        "date": 1757449442696,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 33.67668894768812,
            "unit": "median tps",
            "extra": "avg tps: 33.53277303661832, max tps: 33.94965167402004, count: 57655"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 375.5254253247341,
            "unit": "median tps",
            "extra": "avg tps: 373.26010831136927, max tps: 388.0165557199958, count: 57655"
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
        "date": 1757451027538,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 30.35858072536879,
            "unit": "median tps",
            "extra": "avg tps: 30.14262953371806, max tps: 30.56367442416706, count: 57738"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 154.91712976755116,
            "unit": "median tps",
            "extra": "avg tps: 154.66043276460658, max tps: 157.41341693390456, count: 57738"
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
          "id": "125c2366536d456d1e0df1222b61accfdec48abc",
          "message": "chore: proptest for our pg_sys::TransactionIdPrecedesOrEquals function (#3140)\n\n## What\n\nAdd a proptest for our port of\n`pg_sys::TransactionIdPrecedesOrEquals()`.\n\n## Why\n\nThere was a moment of internal confusion around if it was actually\ncorrect or not.\n\n## How\n\n## Tests\n\nThis is a test.",
          "timestamp": "2025-09-10T09:58:41-04:00",
          "tree_id": "e61581d3b8eee310986ffc3e57941a858ac39e0a",
          "url": "https://github.com/paradedb/paradedb/commit/125c2366536d456d1e0df1222b61accfdec48abc"
        },
        "date": 1757515073810,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 32.506184298652265,
            "unit": "median tps",
            "extra": "avg tps: 32.53592117222024, max tps: 33.315249014975045, count: 56690"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 404.39042565872126,
            "unit": "median tps",
            "extra": "avg tps: 402.2585638700203, max tps: 427.6332108544414, count: 56690"
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
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "1c48d3c6427e9fe7bdb10861a16e04d0cf1a1f48",
          "message": "ci: fix benchmark links in slack messages (#2875)",
          "timestamp": "2025-07-17T12:13:25Z",
          "url": "https://github.com/paradedb/paradedb/commit/1c48d3c6427e9fe7bdb10861a16e04d0cf1a1f48"
        },
        "date": 1757449219737,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.731707,
            "unit": "median cpu",
            "extra": "avg cpu: 20.51118572889491, max cpu: 46.198265, count: 57309"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 176.1640625,
            "unit": "median mem",
            "extra": "avg mem: 174.4066465618402, max mem: 179.48828125, count: 57309"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 18531,
            "unit": "median block_count",
            "extra": "avg block_count: 16973.152942818753, max block_count: 21990.0, count: 57309"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 40,
            "unit": "median segment_count",
            "extra": "avg segment_count: 41.9032961663962, max segment_count: 124.0, count: 57309"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 9.448819,
            "unit": "median cpu",
            "extra": "avg cpu: 11.542801015186145, max cpu: 37.907207, count: 57309"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 157.66796875,
            "unit": "median mem",
            "extra": "avg mem: 147.74279842880264, max mem: 165.40625, count: 57309"
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
        "date": 1757449233057,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.640776,
            "unit": "median cpu",
            "extra": "avg cpu: 20.75490140852724, max cpu: 57.6, count: 56745"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 163.71875,
            "unit": "median mem",
            "extra": "avg mem: 163.69668190754692, max mem: 170.98828125, count: 56745"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 22329,
            "unit": "median block_count",
            "extra": "avg block_count: 20903.904079654596, max block_count: 29599.0, count: 56745"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 46,
            "unit": "median segment_count",
            "extra": "avg segment_count: 47.96371486474579, max segment_count: 142.0, count: 56745"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 5.829490176788098, max cpu: 28.318584, count: 56745"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 164.1953125,
            "unit": "median mem",
            "extra": "avg mem: 154.82457030905806, max mem: 166.33203125, count: 56745"
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
        "date": 1757449309092,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.713451,
            "unit": "median cpu",
            "extra": "avg cpu: 20.786401688072452, max cpu: 46.376812, count: 56572"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 173.1953125,
            "unit": "median mem",
            "extra": "avg mem: 171.42596409221878, max mem: 177.03515625, count: 56572"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 18900,
            "unit": "median block_count",
            "extra": "avg block_count: 17181.290284946615, max block_count: 22352.0, count: 56572"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 40,
            "unit": "median segment_count",
            "extra": "avg segment_count: 41.85010252421693, max segment_count: 114.0, count: 56572"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 9.486166,
            "unit": "median cpu",
            "extra": "avg cpu: 11.779631481596462, max cpu: 38.772213, count: 56572"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 163.51171875,
            "unit": "median mem",
            "extra": "avg mem: 153.2018253564042, max mem: 169.95703125, count: 56572"
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
        "date": 1757449396775,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.677044,
            "unit": "median cpu",
            "extra": "avg cpu: 19.993501752886125, max cpu: 42.942345, count: 57486"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 164.484375,
            "unit": "median mem",
            "extra": "avg mem: 164.20603297705094, max mem: 171.37109375, count: 57486"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 22542,
            "unit": "median block_count",
            "extra": "avg block_count: 20872.938332811467, max block_count: 28798.0, count: 57486"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 46,
            "unit": "median segment_count",
            "extra": "avg segment_count: 48.4016804091431, max segment_count: 137.0, count: 57486"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.673807,
            "unit": "median cpu",
            "extra": "avg cpu: 5.852087152626906, max cpu: 28.263002, count: 57486"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 163.078125,
            "unit": "median mem",
            "extra": "avg mem: 154.07821822922102, max mem: 166.0234375, count: 57486"
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
        "date": 1757449445524,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.677044,
            "unit": "median cpu",
            "extra": "avg cpu: 20.07869653866716, max cpu: 42.72997, count: 57655"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 162.25390625,
            "unit": "median mem",
            "extra": "avg mem: 162.44138423055242, max mem: 169.74609375, count: 57655"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 22667,
            "unit": "median block_count",
            "extra": "avg block_count: 20801.537282109097, max block_count: 28336.0, count: 57655"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 46,
            "unit": "median segment_count",
            "extra": "avg segment_count: 47.98416442632902, max segment_count: 153.0, count: 57655"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.669261,
            "unit": "median cpu",
            "extra": "avg cpu: 5.7661174714401024, max cpu: 28.180038, count: 57655"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 163.9765625,
            "unit": "median mem",
            "extra": "avg mem: 155.05324722487208, max mem: 166.21875, count: 57655"
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
        "date": 1757451030723,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.677044,
            "unit": "median cpu",
            "extra": "avg cpu: 20.113583476378015, max cpu: 42.64561, count: 57738"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 168.2890625,
            "unit": "median mem",
            "extra": "avg mem: 166.3436605604195, max mem: 169.4296875, count: 57738"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 19326,
            "unit": "median block_count",
            "extra": "avg block_count: 17850.31862897918, max block_count: 24141.0, count: 57738"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 36,
            "unit": "median segment_count",
            "extra": "avg segment_count: 38.82891683120302, max segment_count: 101.0, count: 57738"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 9.329447,
            "unit": "median cpu",
            "extra": "avg cpu: 10.27529804711544, max cpu: 28.318584, count: 57738"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 161.7734375,
            "unit": "median mem",
            "extra": "avg mem: 150.91783438766498, max mem: 168.5859375, count: 57738"
          }
        ]
      }
    ]
  }
}