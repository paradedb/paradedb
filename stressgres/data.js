window.BENCHMARK_DATA = {
  "lastUpdate": 1758552986088,
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
        "date": 1757515750920,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 37.54246386863071,
            "unit": "median tps",
            "extra": "avg tps: 37.885353004582385, max tps: 39.38566011730856, count: 55541"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 245.1134918286282,
            "unit": "median tps",
            "extra": "avg tps: 274.39032486847645, max tps: 2556.1389438063748, count: 55541"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 420.0138677708742,
            "unit": "median tps",
            "extra": "avg tps: 414.5298004028997, max tps: 432.06765043493664, count: 55541"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 105.47070614434679,
            "unit": "median tps",
            "extra": "avg tps: 106.94031220453634, max tps: 246.7988379050868, count: 111082"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 15.34857021481398,
            "unit": "median tps",
            "extra": "avg tps: 15.714950328197167, max tps: 19.189934894499782, count: 55541"
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
          "id": "da997100cd0e2873fa8692ec6c2382761719ce58",
          "message": "chore: Upgrade to `0.18.2` (#3144) (#3145)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-09-10T17:44:33-04:00",
          "tree_id": "d0d9fd4cb9ebc554c1e7f3e029694e863f4247c9",
          "url": "https://github.com/paradedb/paradedb/commit/da997100cd0e2873fa8692ec6c2382761719ce58"
        },
        "date": 1757543788332,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 38.00581277608246,
            "unit": "median tps",
            "extra": "avg tps: 38.118726902930284, max tps: 38.5741246250483, count: 55422"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 244.4796971501213,
            "unit": "median tps",
            "extra": "avg tps: 270.46409131631344, max tps: 2348.751988940927, count: 55422"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 398.6233775469945,
            "unit": "median tps",
            "extra": "avg tps: 394.98082568461336, max tps: 405.0300551061935, count: 55422"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 107.56948518497272,
            "unit": "median tps",
            "extra": "avg tps: 107.5840998993826, max tps: 248.03570116869398, count: 110844"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 15.626709944139485,
            "unit": "median tps",
            "extra": "avg tps: 15.944829648714222, max tps: 19.618739616046355, count: 55422"
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
          "id": "1cfaa7b311ca8b7ee91491411c8dfecd2ce5619c",
          "message": "fix: `GROUP BY` doesn't panic when Postgres eliminates group pathkeys (#3152)\n\n# Ticket(s) Closed\n\n- Closes #3050 \n\n## What\n\nIt's possible for Postgres to eliminate group pathkeys if it realizes\nthat one of the pathkeys is unique, making the other ones unnecessary.\n\nWe need to handle this case/not panic.\n\n## Why\n\nSee issue.\n\n## How\n\nInject the dropped group pathkeys back into our list of grouping\ncolumns.\n\n## Tests\n\nAdded regression test",
          "timestamp": "2025-09-14T17:56:19-04:00",
          "tree_id": "a41824569d62cfd5dbe40884e6ead540d3b1bd88",
          "url": "https://github.com/paradedb/paradedb/commit/1cfaa7b311ca8b7ee91491411c8dfecd2ce5619c"
        },
        "date": 1757890008496,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 37.038448454947996,
            "unit": "median tps",
            "extra": "avg tps: 37.55804601074373, max tps: 39.41458451689826, count: 55536"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 250.34878081291816,
            "unit": "median tps",
            "extra": "avg tps: 278.9503940092505, max tps: 2432.4098650590854, count: 55536"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 456.43630503693913,
            "unit": "median tps",
            "extra": "avg tps: 456.3986905951692, max tps: 470.19457314892986, count: 55536"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 100.79950710948889,
            "unit": "median tps",
            "extra": "avg tps: 123.39503516228245, max tps: 360.9424422601989, count: 111072"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 15.749388543460382,
            "unit": "median tps",
            "extra": "avg tps: 15.956376359022904, max tps: 18.862092227029876, count: 55536"
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
          "id": "a521487756693e82c46bfe2f1a2f2fd3aded0136",
          "message": "fix: fixed `rt_fetch out-of-bounds` error (#3141)\n\n# Ticket(s) Closed\n\n- Closes #3135\n\n## What\n\nFixed `rt_fetch used out-of-bounds` and `Cannot open relation with\noid=0` errors that occurred in complex SQL queries with nested `OR\nEXISTS` clauses, multiple `JOIN`s.\n\n## Why\n\nThe issue occurred when PostgreSQL's query planner generated `Var` nodes\nreferencing Range Table Entries (RTEs) that were valid in outer planning\ncontexts but didn't exist in inner execution contexts. This happened\nspecifically with:\n- `OR EXISTS` subqueries (not `AND EXISTS`)  \n- Multiple `JOIN`s within the `EXISTS` clause\n- ParadeDB functions applied to joined tables\n\nWhen ParadeDB's custom scan tried to access these out-of-bounds RTEs\nusing `rt_fetch`, it caused crashes.\n\n## How\n\nImplemented bounds checking across the codebase:\n\n1. **Early detection**: Added bounds checking in `find_var_relation()`\nto detect invalid `varno` values and return `pg_sys::InvalidOid`. This\nwas the main fix for the issue.\n2. **Graceful handling**: Modified all functions that receive relation\nOIDs to check for `InvalidOid` before attempting to open relations\n3. **Safe fallbacks**: Updated query optimization logic to skip\noptimizations when relation information is unavailable rather than\ncrashing\n\n## Tests\n\nAdded regression test `or_exists_join_bug.sql` covering:\n- Simple queries (baseline functionality)\n- `AND EXISTS` with multiple `JOIN`s (should work)  \n- `OR EXISTS` with multiple `JOIN`s (the problematic case, now fixed)\n- Various edge cases and workarounds\n- Minimal reproduction cases\n\n---------\n\nSigned-off-by: Moe <mdashti@gmail.com>",
          "timestamp": "2025-09-15T02:47:52-07:00",
          "tree_id": "4a0b5db116e0263111295cc53d05810e093ce68c",
          "url": "https://github.com/paradedb/paradedb/commit/a521487756693e82c46bfe2f1a2f2fd3aded0136"
        },
        "date": 1757932705957,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 37.89020302753164,
            "unit": "median tps",
            "extra": "avg tps: 38.188917156569836, max tps: 39.9097463625174, count: 55560"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 246.89006209904443,
            "unit": "median tps",
            "extra": "avg tps: 274.4227896748501, max tps: 2345.8986239887086, count: 55560"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 502.00118009048856,
            "unit": "median tps",
            "extra": "avg tps: 496.95802995720453, max tps: 503.62166452353097, count: 55560"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 103.08895022611539,
            "unit": "median tps",
            "extra": "avg tps: 106.10379856060364, max tps: 343.6267995529278, count: 111120"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 15.97137660706832,
            "unit": "median tps",
            "extra": "avg tps: 16.371886049012378, max tps: 20.28039265273823, count: 55560"
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
          "id": "b37fc5af676e3438c051381414d81996ed0fb8f6",
          "message": "feat: push down `group by ... order by ... limit` (#3134)\n\n# Ticket(s) Closed\n\n- Closes #3131 \n- Opens #3156 #3155 \n\n## What\n\nPushes down `group by ... order by ... limit` to Tantivy\n\n## Why\n\nBy pushing down the sort/limit to Tantivy, we can significantly speed up\n`group by` queries over high cardinality columns.\n\n## How\n\n- Before we were hard-coding a bucket size and sorting the results\nourselves, now the bucket size is set to the limit and we push the sort\ndown to the Tantivy term agg\n\n## Tests",
          "timestamp": "2025-09-15T15:51:50-04:00",
          "tree_id": "e58df02d60abc13101aaae8ef6333a9afafbcd78",
          "url": "https://github.com/paradedb/paradedb/commit/b37fc5af676e3438c051381414d81996ed0fb8f6"
        },
        "date": 1757968934684,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 36.540743068599305,
            "unit": "median tps",
            "extra": "avg tps: 36.616326111556525, max tps: 38.46376404209758, count: 55541"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 244.38450706339748,
            "unit": "median tps",
            "extra": "avg tps: 269.62671228412296, max tps: 2335.1367166635882, count: 55541"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 507.7066997459737,
            "unit": "median tps",
            "extra": "avg tps: 502.46290185201804, max tps: 512.6172444366305, count: 55541"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 104.04811570643962,
            "unit": "median tps",
            "extra": "avg tps: 106.03177414926351, max tps: 285.4828970184683, count: 111082"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 15.944331237945248,
            "unit": "median tps",
            "extra": "avg tps: 16.02396216361999, max tps: 18.90535248906831, count: 55541"
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
          "id": "8211eef7a0dd34237afebfa91364fb66c65a4906",
          "message": "perf: remove `ExactBuffer` in favor of a regular rust `BufWriter` (#3158)\n\n# Ticket(s) Closed\n\n- Closes #2981  (/cc @yjhjstz)\n\nWhile #2981 wasn't the impetus for this, it addresses the complaint made\nthere just the same.\n\n## What\n\nIn profiling, our `ExactBuffer` was a large percentage of certain\nprofiles. This replaces it, and its complexity, with a standard Rust\n`BufWriter`.\n\n## Why\n\nImproves performance of (at least) our `wide-table.toml` test's \"Single\nUpdate\" job by quite a bit.\n\n<img width=\"720\" height=\"141\" alt=\"screenshot_2025-09-15_at_3 28\n33___pm_720\"\nsrc=\"https://github.com/user-attachments/assets/a373a7ae-df38-4691-980a-d6843f073d26\"\n/>\n\n\n## How\n\n## Tests\n\nExisting tests pass",
          "timestamp": "2025-09-15T15:55:52-04:00",
          "tree_id": "4ddf140542c5525034023441aadac4b634c90fc6",
          "url": "https://github.com/paradedb/paradedb/commit/8211eef7a0dd34237afebfa91364fb66c65a4906"
        },
        "date": 1757969179715,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 36.84843585751081,
            "unit": "median tps",
            "extra": "avg tps: 37.10151801267816, max tps: 41.10375511065311, count: 55824"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 128.36000809151764,
            "unit": "median tps",
            "extra": "avg tps: 166.04608877906875, max tps: 2323.2528365740613, count: 55824"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1032.091452698935,
            "unit": "median tps",
            "extra": "avg tps: 1024.1227714695146, max tps: 1041.5447502461018, count: 55824"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 114.10223744169525,
            "unit": "median tps",
            "extra": "avg tps: 118.94875281059258, max tps: 779.5955051906514, count: 111648"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 18.623675155732993,
            "unit": "median tps",
            "extra": "avg tps: 18.678151477698776, max tps: 21.761777033054305, count: 55824"
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
          "id": "288a4bfa0c79838d86711b8a6231687c984ac0b5",
          "message": "chore: Upgrade to `0.18.3` (#3160)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-09-15T16:13:06-04:00",
          "tree_id": "ad59a6c86e8afe29cabad5b0bcc6a78bc448182e",
          "url": "https://github.com/paradedb/paradedb/commit/288a4bfa0c79838d86711b8a6231687c984ac0b5"
        },
        "date": 1757970208836,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 38.69298936570179,
            "unit": "median tps",
            "extra": "avg tps: 38.81195664674978, max tps: 39.764261768843184, count: 55634"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 237.52188814525027,
            "unit": "median tps",
            "extra": "avg tps: 261.92932711449095, max tps: 2373.5637317874684, count: 55634"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1027.3417047791613,
            "unit": "median tps",
            "extra": "avg tps: 1019.5806145583192, max tps: 1034.6207448837736, count: 55634"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 117.15586647318688,
            "unit": "median tps",
            "extra": "avg tps: 153.68490944214653, max tps: 775.0671160616886, count: 111268"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 17.088621081409237,
            "unit": "median tps",
            "extra": "avg tps: 17.507430902424154, max tps: 20.96974172166759, count: 55634"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "af5bea23effe976b411147e259e53afad947a393",
          "message": "perf: remove `ExactBuffer` in favor of a regular rust `BufWriter` (#3159)\n\n# Ticket(s) Closed\n\n- Closes #2981  (/cc @yjhjstz)\n\nWhile #2981 wasn't the impetus for this, it addresses the complaint made\nthere just the same.\n\n## What\n\nIn profiling, our `ExactBuffer` was a large percentage of certain\nprofiles. This replaces it, and its complexity, with a standard Rust\n`BufWriter`.\n\n## Why\n\nImproves performance of (at least) our `wide-table.toml` test's \"Single\nUpdate\" job by quite a bit.\n\n<img width=\"720\" height=\"141\" alt=\"screenshot_2025-09-15_at_3 28\n33___pm_720\"\nsrc=\"https://github.com/user-attachments/assets/a373a7ae-df38-4691-980a-d6843f073d26\"\n/>\n\n\n## How\n\n## Tests\n\nExisting tests pass\n\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>",
          "timestamp": "2025-09-15T16:26:19-04:00",
          "tree_id": "cbc00b9a93c129255360f60e5a70904e87f1e8c1",
          "url": "https://github.com/paradedb/paradedb/commit/af5bea23effe976b411147e259e53afad947a393"
        },
        "date": 1757971086896,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 39.0698321437685,
            "unit": "median tps",
            "extra": "avg tps: 39.15977297530289, max tps: 40.17196537891368, count: 55471"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 234.8993976280948,
            "unit": "median tps",
            "extra": "avg tps: 256.4365733922117, max tps: 2293.861417785219, count: 55471"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1031.4845959464083,
            "unit": "median tps",
            "extra": "avg tps: 1023.9028336604473, max tps: 1041.5340499694876, count: 55471"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 118.5031565057334,
            "unit": "median tps",
            "extra": "avg tps: 154.5839928366797, max tps: 783.8772412313865, count: 110942"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 18.490485584079217,
            "unit": "median tps",
            "extra": "avg tps: 18.641520628318478, max tps: 20.222984696155216, count: 55471"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "7800d096e107acdbdec6297d0cb98ef030569e2b",
          "message": "chore: Upgrade to `0.18.3` (#3161)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-09-15T16:36:57-04:00",
          "tree_id": "c0962cc02d5690156721fd003c985f724ee9b20f",
          "url": "https://github.com/paradedb/paradedb/commit/7800d096e107acdbdec6297d0cb98ef030569e2b"
        },
        "date": 1757971687385,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 38.57869232206891,
            "unit": "median tps",
            "extra": "avg tps: 38.63013008946604, max tps: 40.79395846539205, count: 55553"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 238.9745986566294,
            "unit": "median tps",
            "extra": "avg tps: 258.8246640179006, max tps: 2264.984529843966, count: 55553"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1034.67890754,
            "unit": "median tps",
            "extra": "avg tps: 1029.6528755282432, max tps: 1043.7407261637998, count: 55553"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 116.35174503058911,
            "unit": "median tps",
            "extra": "avg tps: 151.69820058036, max tps: 782.0786471771715, count: 111106"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 17.77174160765168,
            "unit": "median tps",
            "extra": "avg tps: 18.055106678141588, max tps: 22.158774347077955, count: 55553"
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
          "id": "f71a5572d645d23e58b949cc3f16645473c74735",
          "message": "chore: Sync `0.18.x` (#3162)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\n---------\n\nSigned-off-by: Moe <mdashti@gmail.com>\nCo-authored-by: Moe <mdashti@gmail.com>",
          "timestamp": "2025-09-15T17:11:39-04:00",
          "tree_id": "a75daf7f281149ef4317505338649d8b0d2ec8a4",
          "url": "https://github.com/paradedb/paradedb/commit/f71a5572d645d23e58b949cc3f16645473c74735"
        },
        "date": 1757973738986,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 38.26699688827,
            "unit": "median tps",
            "extra": "avg tps: 38.29226678117749, max tps: 40.49166932876593, count: 55471"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 241.57321418594387,
            "unit": "median tps",
            "extra": "avg tps: 264.18989609050595, max tps: 2393.552049301602, count: 55471"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1079.4082555211312,
            "unit": "median tps",
            "extra": "avg tps: 1073.2118848339069, max tps: 1085.7202174918923, count: 55471"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 124.74523505976472,
            "unit": "median tps",
            "extra": "avg tps: 155.4090131925052, max tps: 796.899706265132, count: 110942"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 17.885206238211765,
            "unit": "median tps",
            "extra": "avg tps: 17.95055923787478, max tps: 19.910899521079152, count: 55471"
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
          "id": "878c50feef96d61785ad711ebe46250c920bed70",
          "message": "fix: sequential scan segfault (#3163)\n\n# Ticket(s) Closed\n\n- Closes #3151 \n\n## What\n\nThe `@@@` return type should be `bool`, not `SearchQueryInput`.\n\n## Why\n\n## How\n\n## Tests\n\nAdded regression test.",
          "timestamp": "2025-09-16T10:27:13-04:00",
          "tree_id": "6859469869310b79c8c32af68b3ed77dfb787362",
          "url": "https://github.com/paradedb/paradedb/commit/878c50feef96d61785ad711ebe46250c920bed70"
        },
        "date": 1758035881379,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 35.68983408121335,
            "unit": "median tps",
            "extra": "avg tps: 35.92672362716859, max tps: 38.61083840037252, count: 55379"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 242.0953256847614,
            "unit": "median tps",
            "extra": "avg tps: 263.33284583797746, max tps: 2295.562906458107, count: 55379"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1016.2932991821962,
            "unit": "median tps",
            "extra": "avg tps: 1003.1550984464978, max tps: 1039.8838633985426, count: 55379"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 125.1904824360289,
            "unit": "median tps",
            "extra": "avg tps: 158.44750772346242, max tps: 773.0410951776693, count: 110758"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 18.115447530727703,
            "unit": "median tps",
            "extra": "avg tps: 18.003853335820224, max tps: 20.747306786919165, count: 55379"
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
          "id": "f2a0c9c43e4385628cc7b828a8ed12c30e55050e",
          "message": "chore: don't warn about `raw` tokenizer on UUID key fields (#3166)\n\n## What\n\nRemove the warning about using the `raw` tokenizer when the `key_field`\nis a UUID field.\n\nThe drama here is that we (pg_search) assign the `raw` tokenizer to UUID\nfields used as the key_field so there's nothing a user can do about it.\nWarning about our own bad decisions is not cool.\n\n## Why\n\nMany user and customer complaints.\n\n## How\n\n## Tests\n\nExisting tests pass but a couple of the regression test expected output\nis now different.",
          "timestamp": "2025-09-16T13:10:47-04:00",
          "tree_id": "2b24aea6e3a0645c584d8ebb8ce7465c8c90f904",
          "url": "https://github.com/paradedb/paradedb/commit/f2a0c9c43e4385628cc7b828a8ed12c30e55050e"
        },
        "date": 1758045690777,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 36.0054254117089,
            "unit": "median tps",
            "extra": "avg tps: 36.26048423089554, max tps: 41.16326678435976, count: 55745"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 238.3937046053868,
            "unit": "median tps",
            "extra": "avg tps: 261.8875149577405, max tps: 2324.94892289199, count: 55745"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1041.4808856975192,
            "unit": "median tps",
            "extra": "avg tps: 1029.6850125947426, max tps: 1055.4092246131395, count: 55745"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 113.59407178277267,
            "unit": "median tps",
            "extra": "avg tps: 146.32328496511124, max tps: 737.3983818486104, count: 111490"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 17.009899688812183,
            "unit": "median tps",
            "extra": "avg tps: 17.013181172887958, max tps: 18.417915430494553, count: 55745"
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
          "id": "489eb48583040067612195f9e1406d5e31a1599f",
          "message": "perf: teach custom scan callback to exit early if it can (#3168)\n\n## What\n\nThis does two things.  \n\nOne, the first commit (62d752572b2d7bc5a02b7203ac2c83949e38e27e) simply\nreorders some code in the custom scan callback so it can decide to exit\nearly if we're not going to submit a path. Specifically, this is\nintended to avoid opening a Directory and Index and related structures.\n\nTwo, the second commit (5ac1dde23ef0809bea4b942d04fd14acc9d1c152) makes\na new decision to not evaluate possible pushdown predicates when the\nstatement type is not a SELECT statement. This cuts out the overhead of\nneeding to read/deserialize the index's schema at all on (at least)\nUPDATE statements.\n\nThis does mean that we won't consider doing pushdowns for UPDATE\nstatements, even if doing one would make the UPDATE scan faster.\n\n## Why\n\nTrying to reduce per-query overhead, targeting our stressgres benchmarks\nlike \"single-server.toml\" and \"wide-table.toml\".\n\n## How\n\n## Tests\n\nAll existing tests pass.",
          "timestamp": "2025-09-16T17:39:51-04:00",
          "tree_id": "0ebcd01c6225cbb43b199470f7f78bd694493ed7",
          "url": "https://github.com/paradedb/paradedb/commit/489eb48583040067612195f9e1406d5e31a1599f"
        },
        "date": 1758061829495,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 36.32080912073299,
            "unit": "median tps",
            "extra": "avg tps: 36.52885856629034, max tps: 39.407915873053085, count: 55622"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 245.4967161986446,
            "unit": "median tps",
            "extra": "avg tps: 273.8810526091645, max tps: 2886.7913479127456, count: 55622"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1040.547851969739,
            "unit": "median tps",
            "extra": "avg tps: 1030.1191163267933, max tps: 1048.5184044614725, count: 55622"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 118.05089634929044,
            "unit": "median tps",
            "extra": "avg tps: 155.41439354505587, max tps: 803.4353278772444, count: 111244"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 18.319299063935397,
            "unit": "median tps",
            "extra": "avg tps: 18.219242700190005, max tps: 18.835532900702088, count: 55622"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "63daa7f2bf568127e538f19f942d6363508ca615",
          "message": "chore: don't warn about `raw` tokenizer on UUID key fields (#3167)\n\n## What\n\nRemove the warning about using the `raw` tokenizer when the `key_field`\nis a UUID field.\n\nThe drama here is that we (pg_search) assign the `raw` tokenizer to UUID\nfields used as the key_field so there's nothing a user can do about it.\nWarning about our own bad decisions is not cool.\n\n## Why\n\nMany user and customer complaints.\n\n## How\n\n## Tests\n\nExisting tests pass but a couple of the regression test expected output\nis now different.\n\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>",
          "timestamp": "2025-09-17T10:06:31-04:00",
          "tree_id": "2c472616485a1c2a1ed61c7f2c030286882deb06",
          "url": "https://github.com/paradedb/paradedb/commit/63daa7f2bf568127e538f19f942d6363508ca615"
        },
        "date": 1758121045600,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 37.23536429755855,
            "unit": "median tps",
            "extra": "avg tps: 37.485706341601265, max tps: 38.946791289050836, count: 55448"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 245.84051085056066,
            "unit": "median tps",
            "extra": "avg tps: 278.0010587481435, max tps: 2879.7959491782253, count: 55448"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 981.2931900957528,
            "unit": "median tps",
            "extra": "avg tps: 979.33191739944, max tps: 990.6449642220937, count: 55448"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 118.55323481730571,
            "unit": "median tps",
            "extra": "avg tps: 154.23101041228605, max tps: 800.9318402526686, count: 110896"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 17.406100869377777,
            "unit": "median tps",
            "extra": "avg tps: 17.468430492477005, max tps: 19.84249543032291, count: 55448"
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
          "id": "eb456f8c97d99e92e2795d88dd2c1082c13c83a6",
          "message": "perf: optimize `Timestamp` and `JsonB` datum decoding (#3171)\n\n## What\n\nOptimize `Timestamp` and `JsonB` to `TantivyValue` datum conversions.\n\nThese two show up quite high in profiles. The `JsonB` conversion in\nparticular has been bad due to how pgrx stupidly (I can say it) handles\n`JsonB` values by converting them to strings and then asking serde to\nparse the strings.\n\n## Why\n\nTrying to make things faster.\n\n## How\n\nFor the `Timestamp` conversion we memoize Postgres' understanding of the\ncurrent EPOCH and do the same math that it does to calculate a time\nvalue.\n\nFor the `JsonB` conversion we implement our own deserializer routine\nusing Postgres' internal `JsonbIteratorInit()` and `JsonbIteratorNext()`\nfunctions, building up a `serde_json::Value` structure as it goes.\n\n\n## Tests\n\nA new `#[pg_test]`-based proptest has been added to test our custom\njsonb deserializer against normal serde.\n\n---------\n\nSigned-off-by: Eric Ridge <eebbrr@gmail.com>\nCo-authored-by: Stu Hood <stuhood@paradedb.com>",
          "timestamp": "2025-09-17T15:26:06-04:00",
          "tree_id": "702cea735a514e9b33d6c1ee785606d39d4f705c",
          "url": "https://github.com/paradedb/paradedb/commit/eb456f8c97d99e92e2795d88dd2c1082c13c83a6"
        },
        "date": 1758140294926,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 38.77358080244808,
            "unit": "median tps",
            "extra": "avg tps: 38.84871538798397, max tps: 39.67371054472572, count: 55485"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 239.59691486841157,
            "unit": "median tps",
            "extra": "avg tps: 268.5951836320245, max tps: 2864.251998800356, count: 55485"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1008.7033689970119,
            "unit": "median tps",
            "extra": "avg tps: 1002.913883068084, max tps: 1017.0712156118867, count: 55485"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 118.12128988282105,
            "unit": "median tps",
            "extra": "avg tps: 155.0455960088688, max tps: 800.4834221505652, count: 110970"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 18.117429315949774,
            "unit": "median tps",
            "extra": "avg tps: 18.180012193855926, max tps: 19.183290586567477, count: 55485"
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
          "id": "849076799ca599dfbf0f2415149b12495b24624c",
          "message": "fix: Always use a shared expression context to execute HeapFilters. (#3174)\n\n## What\n\nEvaluating heap filters was leaking a `CreateStandaloneExprContext` per\nexecution, which could eventually lead to OOMs.\n\n## Why\n\n`PgBox::from_pg` does not free the resulting memory: it's expected to be\nfreed by a memory context, since PG created it.\n\n## How\n\nAlways use a global expression context created by\n`ExecAssignExprContext` in `begin_custom_scan`.\n\n## Tests\n\nA repro uses significantly less memory, and [a benchmark query added for\nthis purpose](https://github.com/paradedb/paradedb/pull/3175) is faster.",
          "timestamp": "2025-09-17T16:44:32-07:00",
          "tree_id": "7eef1c518a935389aa23e91c6bc47bbc325b18e6",
          "url": "https://github.com/paradedb/paradedb/commit/849076799ca599dfbf0f2415149b12495b24624c"
        },
        "date": 1758155727814,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 39.17596990071766,
            "unit": "median tps",
            "extra": "avg tps: 39.24382132968797, max tps: 39.71930623584084, count: 55524"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 244.14970295833825,
            "unit": "median tps",
            "extra": "avg tps: 272.9472759356801, max tps: 2840.1752657208185, count: 55524"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1032.3070391770511,
            "unit": "median tps",
            "extra": "avg tps: 1025.4797682513442, max tps: 1042.7611157968172, count: 55524"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 118.33203073528722,
            "unit": "median tps",
            "extra": "avg tps: 154.8246450134393, max tps: 809.4219397193018, count: 111048"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 17.37767362489944,
            "unit": "median tps",
            "extra": "avg tps: 17.39734631593961, max tps: 20.312637903768266, count: 55524"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "dc0e87d6fdabdb573d6f37a24c6c33befa43e8b0",
          "message": "fix: Always use a shared expression context to execute HeapFilters. (#3176)\n\n## What\n\nEvaluating heap filters was leaking a `CreateStandaloneExprContext` per\nexecution, which could eventually lead to OOMs.\n\n## Why\n\n`PgBox::from_pg` does not free the resulting memory: it's expected to be\nfreed by a memory context, since PG created it.\n\n## How\n\nAlways use a global expression context created by\n`ExecAssignExprContext` in `begin_custom_scan`.\n\n## Tests\n\nA repro uses significantly less memory, and [a benchmark query added for\nthis purpose](https://github.com/paradedb/paradedb/pull/3175) is faster.\n\nCo-authored-by: Stu Hood <stuhood@paradedb.com>",
          "timestamp": "2025-09-17T17:11:43-07:00",
          "tree_id": "0c30f446ad8404b4f66727777f1b6e6a5bc8958e",
          "url": "https://github.com/paradedb/paradedb/commit/dc0e87d6fdabdb573d6f37a24c6c33befa43e8b0"
        },
        "date": 1758157369840,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 37.424697500272906,
            "unit": "median tps",
            "extra": "avg tps: 37.70843286034739, max tps: 39.71290027087512, count: 55450"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 243.8816563956479,
            "unit": "median tps",
            "extra": "avg tps: 271.5316984882539, max tps: 2844.3098137799257, count: 55450"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 966.7691859380388,
            "unit": "median tps",
            "extra": "avg tps: 968.3789245496905, max tps: 984.6759778831548, count: 55450"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 119.52084671197088,
            "unit": "median tps",
            "extra": "avg tps: 154.00171724161572, max tps: 794.5844277257775, count: 110900"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 16.97060320521073,
            "unit": "median tps",
            "extra": "avg tps: 17.039312748193403, max tps: 18.452089629032546, count: 55450"
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
          "id": "3bcb1451087be74b7bd73bfc7d6546423046a0ce",
          "message": "fix: write all delete files atomically (#3178)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nIn `ambulkdelete`, write all delete files to the meta list atomically,\ninstead of one at a time.\n\n## Why\n\nReduces the number of writes to the metas list.\n\n## How\n\n## Tests",
          "timestamp": "2025-09-18T16:03:10-04:00",
          "tree_id": "ad9609f0419a34b8f0cf543e911c1dc3c25d4563",
          "url": "https://github.com/paradedb/paradedb/commit/3bcb1451087be74b7bd73bfc7d6546423046a0ce"
        },
        "date": 1758228860532,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 33.903341061122,
            "unit": "median tps",
            "extra": "avg tps: 33.93993988041677, max tps: 35.05220888921214, count: 55922"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 134.74151096783777,
            "unit": "median tps",
            "extra": "avg tps: 185.36014070874307, max tps: 3347.465560845961, count: 55922"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1090.622604451528,
            "unit": "median tps",
            "extra": "avg tps: 1089.2260796210676, max tps: 1102.4177032424932, count: 55922"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 108.99097688364432,
            "unit": "median tps",
            "extra": "avg tps: 119.9385411684643, max tps: 929.8347769843474, count: 111844"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 17.680469527210256,
            "unit": "median tps",
            "extra": "avg tps: 17.873244232701474, max tps: 19.672366561505136, count: 55922"
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
          "id": "6e11875ba052ccd6937ca0c535b3803309c8b6eb",
          "message": "feat: removed aggregation limitations re mix of aggregate functions and aggregation on group-by column. (#3179)\n\n# Ticket(s) Closed\n\n- Closes #2963\n\n## What\n\nRemoves aggregate limitations that prevented queries where the same\nfield is used in both `GROUP BY` and aggregate functions (e.g., `SELECT\nrating, AVG(rating) FROM table GROUP BY rating`).\n\n## Why\n\nPrevious safety checks blocked these queries due to Tantivy's\n\"incompatible fruit types\" errors, but testing shows the underlying\nissue is resolved. The limitations were overly restrictive and caused\nunnecessary fallbacks to slower PostgreSQL aggregation.\n\n## How\n\n- Removed `has_search_field_conflicts` function and field conflict\nvalidation\n- Eliminated ~35 lines of restrictive code in\n`extract_and_validate_aggregates`\n- Previously blocked queries now use faster `AggregateScan` instead of\n`GroupAggregate`\n\n## Tests\n\n- **`aggregate-groupby-conflict.sql`** - Tests `GROUP BY field` with\naggregates on same field\n- **`test-fruit-types-issue.sql`** - Validates #2963 issue resolution  \n- **`groupby_aggregate.out`** - Updated expectations showing\n`AggregateScan` usage",
          "timestamp": "2025-09-18T16:00:25-07:00",
          "tree_id": "f85924512f419186b824a986dd35eaa96d973884",
          "url": "https://github.com/paradedb/paradedb/commit/6e11875ba052ccd6937ca0c535b3803309c8b6eb"
        },
        "date": 1758239549300,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 36.0192169610009,
            "unit": "median tps",
            "extra": "avg tps: 36.18156770168731, max tps: 39.072692206978104, count: 55429"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 240.0940934017906,
            "unit": "median tps",
            "extra": "avg tps: 268.8409453711576, max tps: 2868.21976959819, count: 55429"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1050.0732359648218,
            "unit": "median tps",
            "extra": "avg tps: 1039.3991386919904, max tps: 1059.3094401177368, count: 55429"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 114.56794001342584,
            "unit": "median tps",
            "extra": "avg tps: 152.12400092169173, max tps: 807.2160586539114, count: 110858"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 18.286600283615762,
            "unit": "median tps",
            "extra": "avg tps: 18.43680263926529, max tps: 22.540350531020653, count: 55429"
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
          "id": "020f92b742187fe6fdc75a19390692e6d2e9a373",
          "message": "feat: Port `ambulkdelete_epoch` to community (#3180)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nAdd `ambulkdelete_epoch` to the index metadata and make sure it's passed\ndown to all scans. In enterprise, this value is used to detect query\nconflicts with concurrent vacuums on the primary.\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-09-18T19:16:09-04:00",
          "tree_id": "3642b293b38caa7676318f888b910c3f934e1976",
          "url": "https://github.com/paradedb/paradedb/commit/020f92b742187fe6fdc75a19390692e6d2e9a373"
        },
        "date": 1758240451706,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 38.56144804324377,
            "unit": "median tps",
            "extra": "avg tps: 38.56868259314059, max tps: 39.259951523441146, count: 55474"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 245.01034159352764,
            "unit": "median tps",
            "extra": "avg tps: 275.1384903568642, max tps: 2902.6092993221437, count: 55474"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1022.4316150455929,
            "unit": "median tps",
            "extra": "avg tps: 1014.8258942731354, max tps: 1039.667514438407, count: 55474"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 122.88907028501708,
            "unit": "median tps",
            "extra": "avg tps: 158.82353806715582, max tps: 803.1191223597275, count: 110948"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 18.998581235621792,
            "unit": "median tps",
            "extra": "avg tps: 19.034354634292296, max tps: 21.76421669104162, count: 55474"
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
          "id": "c0dc0e674f3524c2f65a7b387ccbf594b23e5e0e",
          "message": "chore: Upgrade to `0.18.4` (#3181)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\n---------\n\nSigned-off-by: Ming <ming.ying.nyc@gmail.com>\nCo-authored-by: Stu Hood <stuhood@paradedb.com>",
          "timestamp": "2025-09-18T19:18:34-04:00",
          "tree_id": "b67f22553ed7786ef556afbfad2b7f8ddc6b139e",
          "url": "https://github.com/paradedb/paradedb/commit/c0dc0e674f3524c2f65a7b387ccbf594b23e5e0e"
        },
        "date": 1758240697968,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 38.43209117945697,
            "unit": "median tps",
            "extra": "avg tps: 38.534483289943616, max tps: 40.520999052721045, count: 55560"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 241.033834857086,
            "unit": "median tps",
            "extra": "avg tps: 270.9816390042202, max tps: 2842.700416153574, count: 55560"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1038.4228786972626,
            "unit": "median tps",
            "extra": "avg tps: 1031.8735479458867, max tps: 1049.0803319863817, count: 55560"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 113.1444485375444,
            "unit": "median tps",
            "extra": "avg tps: 151.43344520951194, max tps: 797.5899010192973, count: 111120"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 19.610861164462083,
            "unit": "median tps",
            "extra": "avg tps: 19.644247440268206, max tps: 20.02576222215357, count: 55560"
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
          "id": "a97c525d0c11314fe01c8bbb250ea5fdf73ec8ce",
          "message": "fix: write all delete files atomically (#3178) (#3182)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nIn `ambulkdelete`, write all delete files to the meta list atomically,\ninstead of one at a time.\n\n## Why\n\nReduces the number of writes to the metas list.\n\n## How\n\n## Tests\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-09-18T21:50:00-04:00",
          "tree_id": "ba5917ed034f24a8e2ad95a64751e5faef3d55d5",
          "url": "https://github.com/paradedb/paradedb/commit/a97c525d0c11314fe01c8bbb250ea5fdf73ec8ce"
        },
        "date": 1758249755985,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 38.65800844064497,
            "unit": "median tps",
            "extra": "avg tps: 38.68212582934701, max tps: 39.52951421494349, count: 55413"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 130.2618111500218,
            "unit": "median tps",
            "extra": "avg tps: 172.97378991324035, max tps: 2875.405606883509, count: 55413"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1033.1526559219824,
            "unit": "median tps",
            "extra": "avg tps: 1027.0301410451652, max tps: 1042.5702674938248, count: 55413"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 116.96822578378558,
            "unit": "median tps",
            "extra": "avg tps: 121.51737258335808, max tps: 821.5877618873551, count: 110826"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 19.42646457360095,
            "unit": "median tps",
            "extra": "avg tps: 19.521656173290964, max tps: 20.43945210372601, count: 55413"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "e15e51abfc4b7834faea068d861d91d5d873580f",
          "message": "chore: Upgrade to `0.18.4` (#3184)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\nSigned-off-by: Ming <ming.ying.nyc@gmail.com>\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>\nCo-authored-by: Stu Hood <stuhood@paradedb.com>",
          "timestamp": "2025-09-18T21:52:13-04:00",
          "tree_id": "3d203e3468a4e7504d03af9c39ac9a0869033086",
          "url": "https://github.com/paradedb/paradedb/commit/e15e51abfc4b7834faea068d861d91d5d873580f"
        },
        "date": 1758249867503,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 38.567587849364486,
            "unit": "median tps",
            "extra": "avg tps: 38.63110501116834, max tps: 39.03835155744179, count: 55570"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 130.15928156464832,
            "unit": "median tps",
            "extra": "avg tps: 172.4721925300891, max tps: 2922.0778657975316, count: 55570"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 993.761823540272,
            "unit": "median tps",
            "extra": "avg tps: 989.9007304493706, max tps: 1013.8037236071993, count: 55570"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 123.29737131360588,
            "unit": "median tps",
            "extra": "avg tps: 124.75722535516077, max tps: 795.6100505558319, count: 111140"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 19.399675442566483,
            "unit": "median tps",
            "extra": "avg tps: 19.43238146090597, max tps: 22.27436760575638, count: 55570"
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
          "id": "1046018b2db9614ef172bd802c98a3987da7513e",
          "message": "chore: small diff ported from enterprise for query cancel fix (#3186)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nSome small changes in enterprise that should be in community\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-09-18T21:53:42-04:00",
          "tree_id": "85ed1f4eb7261157deabdfba479dc61164775f99",
          "url": "https://github.com/paradedb/paradedb/commit/1046018b2db9614ef172bd802c98a3987da7513e"
        },
        "date": 1758249975082,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 37.69705962842865,
            "unit": "median tps",
            "extra": "avg tps: 37.7615952720675, max tps: 38.42750540516792, count: 55639"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 241.79672985475437,
            "unit": "median tps",
            "extra": "avg tps: 270.99533709887527, max tps: 2980.486092266855, count: 55639"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1053.1808873451705,
            "unit": "median tps",
            "extra": "avg tps: 1048.4673987234676, max tps: 1061.8451057475097, count: 55639"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 119.48860386025014,
            "unit": "median tps",
            "extra": "avg tps: 156.7773902991015, max tps: 808.0054369589759, count: 111278"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 18.82306878992898,
            "unit": "median tps",
            "extra": "avg tps: 18.81644778295897, max tps: 22.08341970940903, count: 55639"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "f052aabf25719cee68a756a379c6b66e39452759",
          "message": "feat: Port `ambulkdelete_epoch` to community (#3183)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nAdd `ambulkdelete_epoch` to the index metadata and make sure it's passed\ndown to all scans. In enterprise, this value is used to detect query\nconflicts with concurrent vacuums on the primary.\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-09-18T22:01:15-04:00",
          "tree_id": "48ffae94b2f43d5c2d62b5adb846d1dcc2992aee",
          "url": "https://github.com/paradedb/paradedb/commit/f052aabf25719cee68a756a379c6b66e39452759"
        },
        "date": 1758250353306,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 35.79832098403295,
            "unit": "median tps",
            "extra": "avg tps: 36.12754946666774, max tps: 39.72228033470081, count: 55356"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 246.71382416992407,
            "unit": "median tps",
            "extra": "avg tps: 271.9450044593938, max tps: 2808.089080514356, count: 55356"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1019.663484374181,
            "unit": "median tps",
            "extra": "avg tps: 1012.307228575269, max tps: 1038.6630069265218, count: 55356"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 122.4744452965665,
            "unit": "median tps",
            "extra": "avg tps: 158.36942003241137, max tps: 793.1271773167298, count: 110712"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 17.853247199694504,
            "unit": "median tps",
            "extra": "avg tps: 17.834276282270782, max tps: 18.87853961054592, count: 55356"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "153f632ba06057571459a4b6e8767c135baf438c",
          "message": "chore: small diff ported from enterprise for query cancel fix (#3187)",
          "timestamp": "2025-09-18T22:31:35-04:00",
          "tree_id": "2c3b3f692c24ba8540a69da9d41f4d3a24d4ae6f",
          "url": "https://github.com/paradedb/paradedb/commit/153f632ba06057571459a4b6e8767c135baf438c"
        },
        "date": 1758252178219,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 37.3664511780092,
            "unit": "median tps",
            "extra": "avg tps: 37.32413945916568, max tps: 38.158719827370334, count: 55496"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 249.7124472707314,
            "unit": "median tps",
            "extra": "avg tps: 280.560545835095, max tps: 2994.1300911266267, count: 55496"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1048.5827836620092,
            "unit": "median tps",
            "extra": "avg tps: 1044.1309272565613, max tps: 1058.1020599502178, count: 55496"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 120.68687659354549,
            "unit": "median tps",
            "extra": "avg tps: 158.84092702800044, max tps: 832.1488085883605, count: 110992"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 18.600294451096786,
            "unit": "median tps",
            "extra": "avg tps: 18.542823625187015, max tps: 22.73350601350869, count: 55496"
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
          "id": "8101a67174703310a6a1655496fd5296e869901d",
          "message": "fix: Clone an Arc rather than a OnceLock. (#3185)\n\n## What\n\nInvert our use of `OnceLock` to ensure that we clone an\n`Arc<OnceLock<T>>` rather than a `OnceLock<Arc<T>>`.\n\n## Why\n\n`OnceLock` implements `Clone` by cloning its contents to create a\nseparate disconnected copy. If what is desired is \"exactly once\nbehavior\", then cloning the `OnceLock` before it has been computed the\nfirst time will defeat that.\n\nThis change has no impact on benchmarks in this case, but\n`Arc<OnceLock<T>>` matches the intent of this code, and sets a better\nexample for future us.\n\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>",
          "timestamp": "2025-09-19T15:01:21-07:00",
          "tree_id": "de6adf9a09b874a0e133e9cbfeca50d417e6c5bf",
          "url": "https://github.com/paradedb/paradedb/commit/8101a67174703310a6a1655496fd5296e869901d"
        },
        "date": 1758322375977,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 38.71511540219133,
            "unit": "median tps",
            "extra": "avg tps: 38.609010781186804, max tps: 40.11961405611928, count: 55450"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 77.11651628795957,
            "unit": "median tps",
            "extra": "avg tps: 123.84985579483052, max tps: 2874.7281963403043, count: 55450"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 974.8548287821834,
            "unit": "median tps",
            "extra": "avg tps: 972.9588010404958, max tps: 986.6793014372477, count: 55450"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 119.0446574443722,
            "unit": "median tps",
            "extra": "avg tps: 104.84093193445554, max tps: 798.350092436198, count: 110900"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 19.12779909447049,
            "unit": "median tps",
            "extra": "avg tps: 19.12985625731063, max tps: 22.126744826590045, count: 55450"
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
          "id": "3163a5f3e48d3027585287ce8a63074f70ba1836",
          "message": "perf: Configurable Top N requeries more granularly (#3190)",
          "timestamp": "2025-09-19T21:06:04-04:00",
          "tree_id": "8c74bdf97c37281e4641be0e94b4d464daa5a3ea",
          "url": "https://github.com/paradedb/paradedb/commit/3163a5f3e48d3027585287ce8a63074f70ba1836"
        },
        "date": 1758333454562,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 36.97062030655642,
            "unit": "median tps",
            "extra": "avg tps: 37.036404313261784, max tps: 41.023847032135166, count: 55389"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 239.36969976867564,
            "unit": "median tps",
            "extra": "avg tps: 265.44310102515453, max tps: 2755.7392102642575, count: 55389"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1042.6096157006382,
            "unit": "median tps",
            "extra": "avg tps: 1036.4850189463084, max tps: 1049.2280955180279, count: 55389"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 119.47287630869218,
            "unit": "median tps",
            "extra": "avg tps: 154.64921311808808, max tps: 789.5818749147222, count: 110778"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 18.72964613562999,
            "unit": "median tps",
            "extra": "avg tps: 18.724261444146354, max tps: 20.51676123904804, count: 55389"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "f573a31e6704d95d0a62271a23ba47658a1dae06",
          "message": "perf: Configurable Top N requeries more granularly (#3194)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nAllow the retry scale factor and max chunk size to be tuned, which is\nuseful for reducing Top N requeries.\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-09-20T09:26:21-04:00",
          "tree_id": "d4ee2092267660be53cb68f8b760756a5a07ab69",
          "url": "https://github.com/paradedb/paradedb/commit/f573a31e6704d95d0a62271a23ba47658a1dae06"
        },
        "date": 1758377871304,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 38.58784758465369,
            "unit": "median tps",
            "extra": "avg tps: 38.59764209660863, max tps: 39.15304505949049, count: 55426"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 135.6414283540376,
            "unit": "median tps",
            "extra": "avg tps: 177.95949103890618, max tps: 2908.4436856066063, count: 55426"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1022.1268511373573,
            "unit": "median tps",
            "extra": "avg tps: 1016.8468836161557, max tps: 1026.4639768776706, count: 55426"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 114.52220188393123,
            "unit": "median tps",
            "extra": "avg tps: 122.61932885287716, max tps: 806.8989588777167, count: 110852"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 18.781821138735477,
            "unit": "median tps",
            "extra": "avg tps: 18.814942004837132, max tps: 19.723465832399125, count: 55426"
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
        "date": 1757515753493,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.514948,
            "unit": "median cpu",
            "extra": "avg cpu: 18.633975354116153, max cpu: 46.69261, count: 55541"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 156.09375,
            "unit": "median mem",
            "extra": "avg mem: 145.6377153392314, max mem: 156.84375, count: 55541"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 7.7173376412759325, max cpu: 36.852203, count: 55541"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 149.9765625,
            "unit": "median mem",
            "extra": "avg mem: 145.97755810459842, max mem: 150.36328125, count: 55541"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 5.572058695157413, max cpu: 13.967022, count: 55541"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 150.9921875,
            "unit": "median mem",
            "extra": "avg mem: 129.39605581349812, max mem: 153.640625, count: 55541"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 25560,
            "unit": "median block_count",
            "extra": "avg block_count: 25943.320970094166, max block_count: 52179.0, count: 55541"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.4695327557037245, max cpu: 4.6511626, count: 55541"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 103.13671875,
            "unit": "median mem",
            "extra": "avg mem: 92.98822301599269, max mem: 130.62109375, count: 55541"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 32,
            "unit": "median segment_count",
            "extra": "avg segment_count: 32.47125546893286, max segment_count: 56.0, count: 55541"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.29332,
            "unit": "median cpu",
            "extra": "avg cpu: 11.303318829366528, max cpu: 28.235296, count: 111082"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 157.078125,
            "unit": "median mem",
            "extra": "avg mem: 146.51183697628778, max mem: 162.703125, count: 111082"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.9265,
            "unit": "median cpu",
            "extra": "avg cpu: 14.721920587842751, max cpu: 27.906979, count: 55541"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 156.5703125,
            "unit": "median mem",
            "extra": "avg mem: 154.81711462309374, max mem: 157.8203125, count: 55541"
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
          "id": "da997100cd0e2873fa8692ec6c2382761719ce58",
          "message": "chore: Upgrade to `0.18.2` (#3144) (#3145)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-09-10T17:44:33-04:00",
          "tree_id": "d0d9fd4cb9ebc554c1e7f3e029694e863f4247c9",
          "url": "https://github.com/paradedb/paradedb/commit/da997100cd0e2873fa8692ec6c2382761719ce58"
        },
        "date": 1757543790746,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.514948,
            "unit": "median cpu",
            "extra": "avg cpu: 18.42952917822349, max cpu: 41.901066, count: 55422"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 157.140625,
            "unit": "median mem",
            "extra": "avg mem: 145.67787343586483, max mem: 157.515625, count: 55422"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 7.714453839436766, max cpu: 42.1875, count: 55422"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 146.0390625,
            "unit": "median mem",
            "extra": "avg mem: 142.06473202203097, max mem: 146.0390625, count: 55422"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 5.765169563973277, max cpu: 18.622696, count: 55422"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 148.6328125,
            "unit": "median mem",
            "extra": "avg mem: 126.53472602824691, max mem: 150.515625, count: 55422"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 25368,
            "unit": "median block_count",
            "extra": "avg block_count: 25778.066489841578, max block_count: 51811.0, count: 55422"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 5.153746705451467, max cpu: 9.338522, count: 55422"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 100.16015625,
            "unit": "median mem",
            "extra": "avg mem: 91.3939427962948, max mem: 129.1171875, count: 55422"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 32,
            "unit": "median segment_count",
            "extra": "avg segment_count: 32.36065100501606, max segment_count: 55.0, count: 55422"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.29332,
            "unit": "median cpu",
            "extra": "avg cpu: 11.1468418889833, max cpu: 47.19764, count: 110844"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 157.28515625,
            "unit": "median mem",
            "extra": "avg mem: 145.79694932320425, max mem: 162.85546875, count: 110844"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.93998,
            "unit": "median cpu",
            "extra": "avg cpu: 14.5159055806959, max cpu: 27.87996, count: 55422"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 157.58984375,
            "unit": "median mem",
            "extra": "avg mem: 155.71522421150445, max mem: 159.2578125, count: 55422"
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
          "id": "1cfaa7b311ca8b7ee91491411c8dfecd2ce5619c",
          "message": "fix: `GROUP BY` doesn't panic when Postgres eliminates group pathkeys (#3152)\n\n# Ticket(s) Closed\n\n- Closes #3050 \n\n## What\n\nIt's possible for Postgres to eliminate group pathkeys if it realizes\nthat one of the pathkeys is unique, making the other ones unnecessary.\n\nWe need to handle this case/not panic.\n\n## Why\n\nSee issue.\n\n## How\n\nInject the dropped group pathkeys back into our list of grouping\ncolumns.\n\n## Tests\n\nAdded regression test",
          "timestamp": "2025-09-14T17:56:19-04:00",
          "tree_id": "a41824569d62cfd5dbe40884e6ead540d3b1bd88",
          "url": "https://github.com/paradedb/paradedb/commit/1cfaa7b311ca8b7ee91491411c8dfecd2ce5619c"
        },
        "date": 1757890010852,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.532818,
            "unit": "median cpu",
            "extra": "avg cpu: 18.83990470505555, max cpu: 46.421665, count: 55536"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 156.87109375,
            "unit": "median mem",
            "extra": "avg mem: 146.61008315552075, max mem: 157.6171875, count: 55536"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 7.6334098713584675, max cpu: 27.988338, count: 55536"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 145.1796875,
            "unit": "median mem",
            "extra": "avg mem: 141.6637110247182, max mem: 145.5625, count: 55536"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 5.0948126255658375, max cpu: 14.201183, count: 55536"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 151.94921875,
            "unit": "median mem",
            "extra": "avg mem: 131.04260538772147, max mem: 152.32421875, count: 55536"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 27273.5,
            "unit": "median block_count",
            "extra": "avg block_count: 27734.424301354076, max block_count: 56266.0, count: 55536"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6153846,
            "unit": "median cpu",
            "extra": "avg cpu: 3.8158430491407134, max cpu: 4.660194, count: 55536"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 103.2734375,
            "unit": "median mem",
            "extra": "avg mem: 91.32901666542872, max mem: 128.41015625, count: 55536"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 32,
            "unit": "median segment_count",
            "extra": "avg segment_count: 32.172140593488905, max segment_count: 57.0, count: 55536"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.284333,
            "unit": "median cpu",
            "extra": "avg cpu: 10.528874143997058, max cpu: 28.374382, count: 111072"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 158.4921875,
            "unit": "median mem",
            "extra": "avg mem: 147.34497988213727, max mem: 161.9375, count: 111072"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.913043,
            "unit": "median cpu",
            "extra": "avg cpu: 14.353769863653847, max cpu: 28.015566, count: 55536"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 156.04296875,
            "unit": "median mem",
            "extra": "avg mem: 154.29842002822494, max mem: 157.53515625, count: 55536"
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
          "id": "a521487756693e82c46bfe2f1a2f2fd3aded0136",
          "message": "fix: fixed `rt_fetch out-of-bounds` error (#3141)\n\n# Ticket(s) Closed\n\n- Closes #3135\n\n## What\n\nFixed `rt_fetch used out-of-bounds` and `Cannot open relation with\noid=0` errors that occurred in complex SQL queries with nested `OR\nEXISTS` clauses, multiple `JOIN`s.\n\n## Why\n\nThe issue occurred when PostgreSQL's query planner generated `Var` nodes\nreferencing Range Table Entries (RTEs) that were valid in outer planning\ncontexts but didn't exist in inner execution contexts. This happened\nspecifically with:\n- `OR EXISTS` subqueries (not `AND EXISTS`)  \n- Multiple `JOIN`s within the `EXISTS` clause\n- ParadeDB functions applied to joined tables\n\nWhen ParadeDB's custom scan tried to access these out-of-bounds RTEs\nusing `rt_fetch`, it caused crashes.\n\n## How\n\nImplemented bounds checking across the codebase:\n\n1. **Early detection**: Added bounds checking in `find_var_relation()`\nto detect invalid `varno` values and return `pg_sys::InvalidOid`. This\nwas the main fix for the issue.\n2. **Graceful handling**: Modified all functions that receive relation\nOIDs to check for `InvalidOid` before attempting to open relations\n3. **Safe fallbacks**: Updated query optimization logic to skip\noptimizations when relation information is unavailable rather than\ncrashing\n\n## Tests\n\nAdded regression test `or_exists_join_bug.sql` covering:\n- Simple queries (baseline functionality)\n- `AND EXISTS` with multiple `JOIN`s (should work)  \n- `OR EXISTS` with multiple `JOIN`s (the problematic case, now fixed)\n- Various edge cases and workarounds\n- Minimal reproduction cases\n\n---------\n\nSigned-off-by: Moe <mdashti@gmail.com>",
          "timestamp": "2025-09-15T02:47:52-07:00",
          "tree_id": "4a0b5db116e0263111295cc53d05810e093ce68c",
          "url": "https://github.com/paradedb/paradedb/commit/a521487756693e82c46bfe2f1a2f2fd3aded0136"
        },
        "date": 1757932708539,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.497108,
            "unit": "median cpu",
            "extra": "avg cpu: 18.618076194177224, max cpu: 42.39451, count: 55560"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 154.34375,
            "unit": "median mem",
            "extra": "avg mem: 142.5372989926431, max mem: 154.34375, count: 55560"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 7.679401212937065, max cpu: 37.065636, count: 55560"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 146.19140625,
            "unit": "median mem",
            "extra": "avg mem: 142.64773801689614, max mem: 147.37109375, count: 55560"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 5.0752994983706685, max cpu: 14.04878, count: 55560"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 151.37109375,
            "unit": "median mem",
            "extra": "avg mem: 130.2294420586978, max mem: 152.984375, count: 55560"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 26012,
            "unit": "median block_count",
            "extra": "avg block_count: 26286.583999280057, max block_count: 52591.0, count: 55560"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.235824776992024, max cpu: 4.701273, count: 55560"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 100.28125,
            "unit": "median mem",
            "extra": "avg mem: 90.62336909141018, max mem: 127.421875, count: 55560"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 32,
            "unit": "median segment_count",
            "extra": "avg segment_count: 32.33327933765299, max segment_count: 56.0, count: 55560"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.29332,
            "unit": "median cpu",
            "extra": "avg cpu: 11.33443495728534, max cpu: 46.332047, count: 111120"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 155.29296875,
            "unit": "median mem",
            "extra": "avg mem: 143.5768376811105, max mem: 161.58203125, count: 111120"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.872832,
            "unit": "median cpu",
            "extra": "avg cpu: 13.867521880956833, max cpu: 27.826086, count: 55560"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 156.34765625,
            "unit": "median mem",
            "extra": "avg mem: 154.77168953046257, max mem: 158.13671875, count: 55560"
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
          "id": "b37fc5af676e3438c051381414d81996ed0fb8f6",
          "message": "feat: push down `group by ... order by ... limit` (#3134)\n\n# Ticket(s) Closed\n\n- Closes #3131 \n- Opens #3156 #3155 \n\n## What\n\nPushes down `group by ... order by ... limit` to Tantivy\n\n## Why\n\nBy pushing down the sort/limit to Tantivy, we can significantly speed up\n`group by` queries over high cardinality columns.\n\n## How\n\n- Before we were hard-coding a bucket size and sorting the results\nourselves, now the bucket size is set to the limit and we push the sort\ndown to the Tantivy term agg\n\n## Tests",
          "timestamp": "2025-09-15T15:51:50-04:00",
          "tree_id": "e58df02d60abc13101aaae8ef6333a9afafbcd78",
          "url": "https://github.com/paradedb/paradedb/commit/b37fc5af676e3438c051381414d81996ed0fb8f6"
        },
        "date": 1757968939055,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.550726,
            "unit": "median cpu",
            "extra": "avg cpu: 18.782012664813266, max cpu: 45.78696, count: 55541"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 155.2578125,
            "unit": "median mem",
            "extra": "avg mem: 143.01004578824652, max mem: 155.6328125, count: 55541"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 7.706363112918955, max cpu: 37.75811, count: 55541"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 148.18359375,
            "unit": "median mem",
            "extra": "avg mem: 143.9787061866459, max mem: 148.18359375, count: 55541"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 5.058363384824364, max cpu: 14.201183, count: 55541"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 147.41796875,
            "unit": "median mem",
            "extra": "avg mem: 125.24185596001152, max mem: 147.80859375, count: 55541"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 26335,
            "unit": "median block_count",
            "extra": "avg block_count: 26672.57164977224, max block_count: 53479.0, count: 55541"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 3.8675566515064728, max cpu: 4.660194, count: 55541"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 100.7578125,
            "unit": "median mem",
            "extra": "avg mem: 90.77358617957906, max mem: 129.2734375, count: 55541"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 32,
            "unit": "median segment_count",
            "extra": "avg segment_count: 32.367908391998704, max segment_count: 54.0, count: 55541"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.311348,
            "unit": "median cpu",
            "extra": "avg cpu: 11.385908776043095, max cpu: 42.477875, count: 111082"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 158.73046875,
            "unit": "median mem",
            "extra": "avg mem: 145.55136327358392, max mem: 165.3125, count: 111082"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.9265,
            "unit": "median cpu",
            "extra": "avg cpu: 14.556140222954502, max cpu: 28.042841, count: 55541"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 155.26953125,
            "unit": "median mem",
            "extra": "avg mem: 153.40345251705946, max mem: 156.1328125, count: 55541"
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
          "id": "8211eef7a0dd34237afebfa91364fb66c65a4906",
          "message": "perf: remove `ExactBuffer` in favor of a regular rust `BufWriter` (#3158)\n\n# Ticket(s) Closed\n\n- Closes #2981  (/cc @yjhjstz)\n\nWhile #2981 wasn't the impetus for this, it addresses the complaint made\nthere just the same.\n\n## What\n\nIn profiling, our `ExactBuffer` was a large percentage of certain\nprofiles. This replaces it, and its complexity, with a standard Rust\n`BufWriter`.\n\n## Why\n\nImproves performance of (at least) our `wide-table.toml` test's \"Single\nUpdate\" job by quite a bit.\n\n<img width=\"720\" height=\"141\" alt=\"screenshot_2025-09-15_at_3 28\n33___pm_720\"\nsrc=\"https://github.com/user-attachments/assets/a373a7ae-df38-4691-980a-d6843f073d26\"\n/>\n\n\n## How\n\n## Tests\n\nExisting tests pass",
          "timestamp": "2025-09-15T15:55:52-04:00",
          "tree_id": "4ddf140542c5525034023441aadac4b634c90fc6",
          "url": "https://github.com/paradedb/paradedb/commit/8211eef7a0dd34237afebfa91364fb66c65a4906"
        },
        "date": 1757969183181,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.514948,
            "unit": "median cpu",
            "extra": "avg cpu: 18.79692076709724, max cpu: 41.618496, count: 55824"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 155.85546875,
            "unit": "median mem",
            "extra": "avg mem: 140.68226983465894, max mem: 155.85546875, count: 55824"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 9.357564449372935, max cpu: 36.958614, count: 55824"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 147.25390625,
            "unit": "median mem",
            "extra": "avg mem: 143.17162001334552, max mem: 147.25390625, count: 55824"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.950889861523378, max cpu: 13.88621, count: 55824"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 142.765625,
            "unit": "median mem",
            "extra": "avg mem: 115.01307989171325, max mem: 143.53125, count: 55824"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 28148,
            "unit": "median block_count",
            "extra": "avg block_count: 29388.566781312697, max block_count: 61826.0, count: 55824"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.4701049478208486, max cpu: 4.6647234, count: 55824"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 86.87109375,
            "unit": "median mem",
            "extra": "avg mem: 83.81838372608556, max mem: 125.1640625, count: 55824"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 31,
            "unit": "median segment_count",
            "extra": "avg segment_count: 31.382022069360847, max segment_count: 56.0, count: 55824"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.284333,
            "unit": "median cpu",
            "extra": "avg cpu: 10.905319486411264, max cpu: 36.958614, count: 111648"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 151.26171875,
            "unit": "median mem",
            "extra": "avg mem: 137.66774976516598, max mem: 155.97265625, count: 111648"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.832853,
            "unit": "median cpu",
            "extra": "avg cpu: 12.174375921219589, max cpu: 32.18391, count: 55824"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 155.7265625,
            "unit": "median mem",
            "extra": "avg mem: 153.90153988427917, max mem: 158.25390625, count: 55824"
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
          "id": "288a4bfa0c79838d86711b8a6231687c984ac0b5",
          "message": "chore: Upgrade to `0.18.3` (#3160)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-09-15T16:13:06-04:00",
          "tree_id": "ad59a6c86e8afe29cabad5b0bcc6a78bc448182e",
          "url": "https://github.com/paradedb/paradedb/commit/288a4bfa0c79838d86711b8a6231687c984ac0b5"
        },
        "date": 1757970211464,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.550726,
            "unit": "median cpu",
            "extra": "avg cpu: 18.340666788885493, max cpu: 41.69884, count: 55634"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 156.94140625,
            "unit": "median mem",
            "extra": "avg mem: 146.38292168177284, max mem: 156.94140625, count: 55634"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 7.689042183833102, max cpu: 37.72102, count: 55634"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 146.93359375,
            "unit": "median mem",
            "extra": "avg mem: 143.35449987586279, max mem: 146.93359375, count: 55634"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 5.001133139730712, max cpu: 13.93998, count: 55634"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 144.125,
            "unit": "median mem",
            "extra": "avg mem: 122.99011023778175, max mem: 144.5, count: 55634"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 30935,
            "unit": "median block_count",
            "extra": "avg block_count: 31424.562983067906, max block_count: 64424.0, count: 55634"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 3.9920720325901278, max cpu: 4.7105007, count: 55634"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 103.34765625,
            "unit": "median mem",
            "extra": "avg mem: 92.41695915211471, max mem: 129.234375, count: 55634"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 32,
            "unit": "median segment_count",
            "extra": "avg segment_count: 31.75401732753352, max segment_count: 55.0, count: 55634"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.275363,
            "unit": "median cpu",
            "extra": "avg cpu: 10.03173259337954, max cpu: 28.374382, count: 111268"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 150.12890625,
            "unit": "median mem",
            "extra": "avg mem: 141.15874657639662, max mem: 156.5078125, count: 111268"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.9265,
            "unit": "median cpu",
            "extra": "avg cpu: 13.578789222242412, max cpu: 27.988338, count: 55634"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 157.5546875,
            "unit": "median mem",
            "extra": "avg mem: 155.67684807177267, max mem: 159.30078125, count: 55634"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "af5bea23effe976b411147e259e53afad947a393",
          "message": "perf: remove `ExactBuffer` in favor of a regular rust `BufWriter` (#3159)\n\n# Ticket(s) Closed\n\n- Closes #2981  (/cc @yjhjstz)\n\nWhile #2981 wasn't the impetus for this, it addresses the complaint made\nthere just the same.\n\n## What\n\nIn profiling, our `ExactBuffer` was a large percentage of certain\nprofiles. This replaces it, and its complexity, with a standard Rust\n`BufWriter`.\n\n## Why\n\nImproves performance of (at least) our `wide-table.toml` test's \"Single\nUpdate\" job by quite a bit.\n\n<img width=\"720\" height=\"141\" alt=\"screenshot_2025-09-15_at_3 28\n33___pm_720\"\nsrc=\"https://github.com/user-attachments/assets/a373a7ae-df38-4691-980a-d6843f073d26\"\n/>\n\n\n## How\n\n## Tests\n\nExisting tests pass\n\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>",
          "timestamp": "2025-09-15T16:26:19-04:00",
          "tree_id": "cbc00b9a93c129255360f60e5a70904e87f1e8c1",
          "url": "https://github.com/paradedb/paradedb/commit/af5bea23effe976b411147e259e53afad947a393"
        },
        "date": 1757971089542,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.550726,
            "unit": "median cpu",
            "extra": "avg cpu: 18.254900690096537, max cpu: 42.814667, count: 55471"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 155.94921875,
            "unit": "median mem",
            "extra": "avg mem: 145.55986158589624, max mem: 157.44921875, count: 55471"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 7.738311582056315, max cpu: 28.125, count: 55471"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 146.99609375,
            "unit": "median mem",
            "extra": "avg mem: 143.5689481187242, max mem: 146.99609375, count: 55471"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 5.017539132875895, max cpu: 13.953489, count: 55471"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 143.9453125,
            "unit": "median mem",
            "extra": "avg mem: 122.88031316476177, max mem: 145.12109375, count: 55471"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 30686,
            "unit": "median block_count",
            "extra": "avg block_count: 31207.123181482217, max block_count: 63930.0, count: 55471"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.336930798357154, max cpu: 4.6376815, count: 55471"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 103.19140625,
            "unit": "median mem",
            "extra": "avg mem: 91.36301886627697, max mem: 131.01953125, count: 55471"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 32,
            "unit": "median segment_count",
            "extra": "avg segment_count: 31.749472697445512, max segment_count: 53.0, count: 55471"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.275363,
            "unit": "median cpu",
            "extra": "avg cpu: 9.989813733217321, max cpu: 28.318584, count: 110942"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 152.74609375,
            "unit": "median mem",
            "extra": "avg mem: 142.77510686890446, max mem: 157.5390625, count: 110942"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.872832,
            "unit": "median cpu",
            "extra": "avg cpu: 12.686852682557017, max cpu: 27.988338, count: 55471"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 154.94921875,
            "unit": "median mem",
            "extra": "avg mem: 153.6680731119639, max mem: 156.07421875, count: 55471"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "7800d096e107acdbdec6297d0cb98ef030569e2b",
          "message": "chore: Upgrade to `0.18.3` (#3161)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-09-15T16:36:57-04:00",
          "tree_id": "c0962cc02d5690156721fd003c985f724ee9b20f",
          "url": "https://github.com/paradedb/paradedb/commit/7800d096e107acdbdec6297d0cb98ef030569e2b"
        },
        "date": 1757971690068,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.497108,
            "unit": "median cpu",
            "extra": "avg cpu: 18.344661485949793, max cpu: 41.73913, count: 55553"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 156.9765625,
            "unit": "median mem",
            "extra": "avg mem: 146.14508358009468, max mem: 158.4765625, count: 55553"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 7.686889691731934, max cpu: 36.923077, count: 55553"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 147.29296875,
            "unit": "median mem",
            "extra": "avg mem: 144.3247008346759, max mem: 148.08984375, count: 55553"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.95785804535253, max cpu: 14.159292, count: 55553"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 142.89453125,
            "unit": "median mem",
            "extra": "avg mem: 121.96035901245207, max mem: 143.64453125, count: 55553"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 30448,
            "unit": "median block_count",
            "extra": "avg block_count: 30871.359710546683, max block_count: 62972.0, count: 55553"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6153846,
            "unit": "median cpu",
            "extra": "avg cpu: 4.477887658579023, max cpu: 4.6647234, count: 55553"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 103.31640625,
            "unit": "median mem",
            "extra": "avg mem: 90.87188227064695, max mem: 128.09765625, count: 55553"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 32,
            "unit": "median segment_count",
            "extra": "avg segment_count: 31.762695083973863, max segment_count: 55.0, count: 55553"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.257474,
            "unit": "median cpu",
            "extra": "avg cpu: 10.037714255317336, max cpu: 46.198265, count: 111106"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 150.74609375,
            "unit": "median mem",
            "extra": "avg mem: 140.1703754818034, max mem: 153.984375, count: 111106"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.859479,
            "unit": "median cpu",
            "extra": "avg cpu: 13.375084226210483, max cpu: 32.307693, count: 55553"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 159.76953125,
            "unit": "median mem",
            "extra": "avg mem: 157.58436306038828, max mem: 161.77734375, count: 55553"
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
          "id": "f71a5572d645d23e58b949cc3f16645473c74735",
          "message": "chore: Sync `0.18.x` (#3162)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\n---------\n\nSigned-off-by: Moe <mdashti@gmail.com>\nCo-authored-by: Moe <mdashti@gmail.com>",
          "timestamp": "2025-09-15T17:11:39-04:00",
          "tree_id": "a75daf7f281149ef4317505338649d8b0d2ec8a4",
          "url": "https://github.com/paradedb/paradedb/commit/f71a5572d645d23e58b949cc3f16645473c74735"
        },
        "date": 1757973741534,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.550726,
            "unit": "median cpu",
            "extra": "avg cpu: 18.548457935919863, max cpu: 42.477875, count: 55471"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 155.46875,
            "unit": "median mem",
            "extra": "avg mem: 143.77657934438716, max mem: 155.46875, count: 55471"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 7.722495460627092, max cpu: 28.125, count: 55471"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 146.96484375,
            "unit": "median mem",
            "extra": "avg mem: 143.2184338156424, max mem: 146.96484375, count: 55471"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.9950771889328385, max cpu: 14.0214205, count: 55471"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 142.6640625,
            "unit": "median mem",
            "extra": "avg mem: 122.4957056671504, max mem: 143.83984375, count: 55471"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 30506,
            "unit": "median block_count",
            "extra": "avg block_count: 31067.496818157237, max block_count: 63297.0, count: 55471"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.339717515895685, max cpu: 4.7244096, count: 55471"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 104.30859375,
            "unit": "median mem",
            "extra": "avg mem: 92.12194026495827, max mem: 129.93359375, count: 55471"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 32,
            "unit": "median segment_count",
            "extra": "avg segment_count: 32.04818734113321, max segment_count: 55.0, count: 55471"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.266409,
            "unit": "median cpu",
            "extra": "avg cpu: 9.88710643696675, max cpu: 28.346458, count: 110942"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 151.3828125,
            "unit": "median mem",
            "extra": "avg mem: 140.21435000242244, max mem: 155.94140625, count: 110942"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.913043,
            "unit": "median cpu",
            "extra": "avg cpu: 13.637420198973002, max cpu: 28.015566, count: 55471"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 157.6171875,
            "unit": "median mem",
            "extra": "avg mem: 155.7288591674028, max mem: 159.37109375, count: 55471"
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
          "id": "878c50feef96d61785ad711ebe46250c920bed70",
          "message": "fix: sequential scan segfault (#3163)\n\n# Ticket(s) Closed\n\n- Closes #3151 \n\n## What\n\nThe `@@@` return type should be `bool`, not `SearchQueryInput`.\n\n## Why\n\n## How\n\n## Tests\n\nAdded regression test.",
          "timestamp": "2025-09-16T10:27:13-04:00",
          "tree_id": "6859469869310b79c8c32af68b3ed77dfb787362",
          "url": "https://github.com/paradedb/paradedb/commit/878c50feef96d61785ad711ebe46250c920bed70"
        },
        "date": 1758035884256,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.550726,
            "unit": "median cpu",
            "extra": "avg cpu: 19.091496612108205, max cpu: 42.39451, count: 55379"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 155.12890625,
            "unit": "median mem",
            "extra": "avg mem: 144.5632146069584, max mem: 155.5390625, count: 55379"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 7.743983158453182, max cpu: 27.988338, count: 55379"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 145.34765625,
            "unit": "median mem",
            "extra": "avg mem: 141.65617748830783, max mem: 145.34765625, count: 55379"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.982660790329984, max cpu: 13.994169, count: 55379"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 144.734375,
            "unit": "median mem",
            "extra": "avg mem: 123.67641347013308, max mem: 145.1171875, count: 55379"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 31055,
            "unit": "median block_count",
            "extra": "avg block_count: 31645.79299012261, max block_count: 64940.0, count: 55379"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.489912071982629, max cpu: 4.655674, count: 55379"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 102.6953125,
            "unit": "median mem",
            "extra": "avg mem: 91.31856699967497, max mem: 128.9765625, count: 55379"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 32,
            "unit": "median segment_count",
            "extra": "avg segment_count: 31.687228010617744, max segment_count: 53.0, count: 55379"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.257474,
            "unit": "median cpu",
            "extra": "avg cpu: 9.630870265108802, max cpu: 28.152493, count: 110758"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 150.55859375,
            "unit": "median mem",
            "extra": "avg mem: 141.3048217312745, max mem: 156.640625, count: 110758"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.899614,
            "unit": "median cpu",
            "extra": "avg cpu: 13.570321501472495, max cpu: 28.015566, count: 55379"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 158.63671875,
            "unit": "median mem",
            "extra": "avg mem: 156.46915318758013, max mem: 160.296875, count: 55379"
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
          "id": "f2a0c9c43e4385628cc7b828a8ed12c30e55050e",
          "message": "chore: don't warn about `raw` tokenizer on UUID key fields (#3166)\n\n## What\n\nRemove the warning about using the `raw` tokenizer when the `key_field`\nis a UUID field.\n\nThe drama here is that we (pg_search) assign the `raw` tokenizer to UUID\nfields used as the key_field so there's nothing a user can do about it.\nWarning about our own bad decisions is not cool.\n\n## Why\n\nMany user and customer complaints.\n\n## How\n\n## Tests\n\nExisting tests pass but a couple of the regression test expected output\nis now different.",
          "timestamp": "2025-09-16T13:10:47-04:00",
          "tree_id": "2b24aea6e3a0645c584d8ebb8ce7465c8c90f904",
          "url": "https://github.com/paradedb/paradedb/commit/f2a0c9c43e4385628cc7b828a8ed12c30e55050e"
        },
        "date": 1758045694374,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.568666,
            "unit": "median cpu",
            "extra": "avg cpu: 19.044406526745707, max cpu: 42.814667, count: 55745"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 158.31640625,
            "unit": "median mem",
            "extra": "avg mem: 147.1735288058346, max mem: 159.06640625, count: 55745"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 7.741218513974173, max cpu: 27.988338, count: 55745"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 148.28125,
            "unit": "median mem",
            "extra": "avg mem: 144.5227877079783, max mem: 148.28125, count: 55745"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.918250686740787, max cpu: 13.967022, count: 55745"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 142.73828125,
            "unit": "median mem",
            "extra": "avg mem: 122.08610497578258, max mem: 142.73828125, count: 55745"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 30781,
            "unit": "median block_count",
            "extra": "avg block_count: 31139.878823212843, max block_count: 63509.0, count: 55745"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.610951,
            "unit": "median cpu",
            "extra": "avg cpu: 4.2827002266842475, max cpu: 4.673807, count: 55745"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 105.95703125,
            "unit": "median mem",
            "extra": "avg mem: 92.43183406303255, max mem: 128.56640625, count: 55745"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 32,
            "unit": "median segment_count",
            "extra": "avg segment_count: 31.74686518970311, max segment_count: 56.0, count: 55745"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.284333,
            "unit": "median cpu",
            "extra": "avg cpu: 10.265643596903201, max cpu: 33.333336, count: 111490"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 150.18359375,
            "unit": "median mem",
            "extra": "avg mem: 140.32953558054535, max mem: 156.71875, count: 111490"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.9265,
            "unit": "median cpu",
            "extra": "avg cpu: 14.217212777200617, max cpu: 28.09756, count: 55745"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 157.03515625,
            "unit": "median mem",
            "extra": "avg mem: 155.47381218326757, max mem: 158.421875, count: 55745"
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
          "id": "489eb48583040067612195f9e1406d5e31a1599f",
          "message": "perf: teach custom scan callback to exit early if it can (#3168)\n\n## What\n\nThis does two things.  \n\nOne, the first commit (62d752572b2d7bc5a02b7203ac2c83949e38e27e) simply\nreorders some code in the custom scan callback so it can decide to exit\nearly if we're not going to submit a path. Specifically, this is\nintended to avoid opening a Directory and Index and related structures.\n\nTwo, the second commit (5ac1dde23ef0809bea4b942d04fd14acc9d1c152) makes\na new decision to not evaluate possible pushdown predicates when the\nstatement type is not a SELECT statement. This cuts out the overhead of\nneeding to read/deserialize the index's schema at all on (at least)\nUPDATE statements.\n\nThis does mean that we won't consider doing pushdowns for UPDATE\nstatements, even if doing one would make the UPDATE scan faster.\n\n## Why\n\nTrying to reduce per-query overhead, targeting our stressgres benchmarks\nlike \"single-server.toml\" and \"wide-table.toml\".\n\n## How\n\n## Tests\n\nAll existing tests pass.",
          "timestamp": "2025-09-16T17:39:51-04:00",
          "tree_id": "0ebcd01c6225cbb43b199470f7f78bd694493ed7",
          "url": "https://github.com/paradedb/paradedb/commit/489eb48583040067612195f9e1406d5e31a1599f"
        },
        "date": 1758061832252,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.514948,
            "unit": "median cpu",
            "extra": "avg cpu: 18.79075168960984, max cpu: 45.845272, count: 55622"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 155.140625,
            "unit": "median mem",
            "extra": "avg mem: 144.65294181596312, max mem: 155.140625, count: 55622"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 7.693860121998328, max cpu: 27.906979, count: 55622"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 111.6875,
            "unit": "median mem",
            "extra": "avg mem: 110.38171770780896, max mem: 111.6875, count: 55622"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.926426639334823, max cpu: 9.356726, count: 55622"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 143.98828125,
            "unit": "median mem",
            "extra": "avg mem: 122.8826773803531, max mem: 144.37109375, count: 55622"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 30514,
            "unit": "median block_count",
            "extra": "avg block_count: 31057.46887922045, max block_count: 63386.0, count: 55622"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.394908856416252, max cpu: 4.64666, count: 55622"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 103.22265625,
            "unit": "median mem",
            "extra": "avg mem: 91.73515081431358, max mem: 128.421875, count: 55622"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 32,
            "unit": "median segment_count",
            "extra": "avg segment_count: 31.84935816763151, max segment_count: 53.0, count: 55622"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.248554,
            "unit": "median cpu",
            "extra": "avg cpu: 9.9244956631923, max cpu: 28.374382, count: 111244"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 149.828125,
            "unit": "median mem",
            "extra": "avg mem: 140.93789352543732, max mem: 156.5, count: 111244"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.846154,
            "unit": "median cpu",
            "extra": "avg cpu: 12.747097979502302, max cpu: 27.934044, count: 55622"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 155.91796875,
            "unit": "median mem",
            "extra": "avg mem: 154.1237025283386, max mem: 157.546875, count: 55622"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "63daa7f2bf568127e538f19f942d6363508ca615",
          "message": "chore: don't warn about `raw` tokenizer on UUID key fields (#3167)\n\n## What\n\nRemove the warning about using the `raw` tokenizer when the `key_field`\nis a UUID field.\n\nThe drama here is that we (pg_search) assign the `raw` tokenizer to UUID\nfields used as the key_field so there's nothing a user can do about it.\nWarning about our own bad decisions is not cool.\n\n## Why\n\nMany user and customer complaints.\n\n## How\n\n## Tests\n\nExisting tests pass but a couple of the regression test expected output\nis now different.\n\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>",
          "timestamp": "2025-09-17T10:06:31-04:00",
          "tree_id": "2c472616485a1c2a1ed61c7f2c030286882deb06",
          "url": "https://github.com/paradedb/paradedb/commit/63daa7f2bf568127e538f19f942d6363508ca615"
        },
        "date": 1758121048240,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.532818,
            "unit": "median cpu",
            "extra": "avg cpu: 18.72231216069834, max cpu: 55.54484, count: 55448"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 155.9296875,
            "unit": "median mem",
            "extra": "avg mem: 145.1588015178636, max mem: 155.9296875, count: 55448"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 7.684016705767617, max cpu: 41.65863, count: 55448"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 111.1015625,
            "unit": "median mem",
            "extra": "avg mem: 109.80174456810886, max mem: 111.1015625, count: 55448"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.984061865630285, max cpu: 9.476802, count: 55448"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 146.953125,
            "unit": "median mem",
            "extra": "avg mem: 125.0210256822834, max mem: 147.73046875, count: 55448"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 29774,
            "unit": "median block_count",
            "extra": "avg block_count: 30310.345044005193, max block_count: 61861.0, count: 55448"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.413212683120805, max cpu: 4.6511626, count: 55448"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 104.625,
            "unit": "median mem",
            "extra": "avg mem: 93.05668465555566, max mem: 129.81640625, count: 55448"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 32,
            "unit": "median segment_count",
            "extra": "avg segment_count: 31.850039676814312, max segment_count: 55.0, count: 55448"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.266409,
            "unit": "median cpu",
            "extra": "avg cpu: 9.91529962477923, max cpu: 46.64723, count: 110896"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 148.96484375,
            "unit": "median mem",
            "extra": "avg mem: 139.87468653766817, max mem: 155.34375, count: 110896"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.88621,
            "unit": "median cpu",
            "extra": "avg cpu: 13.536460720205008, max cpu: 32.526623, count: 55448"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 156.57421875,
            "unit": "median mem",
            "extra": "avg mem: 155.05422408720784, max mem: 157.81640625, count: 55448"
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
          "id": "eb456f8c97d99e92e2795d88dd2c1082c13c83a6",
          "message": "perf: optimize `Timestamp` and `JsonB` datum decoding (#3171)\n\n## What\n\nOptimize `Timestamp` and `JsonB` to `TantivyValue` datum conversions.\n\nThese two show up quite high in profiles. The `JsonB` conversion in\nparticular has been bad due to how pgrx stupidly (I can say it) handles\n`JsonB` values by converting them to strings and then asking serde to\nparse the strings.\n\n## Why\n\nTrying to make things faster.\n\n## How\n\nFor the `Timestamp` conversion we memoize Postgres' understanding of the\ncurrent EPOCH and do the same math that it does to calculate a time\nvalue.\n\nFor the `JsonB` conversion we implement our own deserializer routine\nusing Postgres' internal `JsonbIteratorInit()` and `JsonbIteratorNext()`\nfunctions, building up a `serde_json::Value` structure as it goes.\n\n\n## Tests\n\nA new `#[pg_test]`-based proptest has been added to test our custom\njsonb deserializer against normal serde.\n\n---------\n\nSigned-off-by: Eric Ridge <eebbrr@gmail.com>\nCo-authored-by: Stu Hood <stuhood@paradedb.com>",
          "timestamp": "2025-09-17T15:26:06-04:00",
          "tree_id": "702cea735a514e9b33d6c1ee785606d39d4f705c",
          "url": "https://github.com/paradedb/paradedb/commit/eb456f8c97d99e92e2795d88dd2c1082c13c83a6"
        },
        "date": 1758140297695,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.532818,
            "unit": "median cpu",
            "extra": "avg cpu: 18.344393928501177, max cpu: 41.819942, count: 55485"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 156.59375,
            "unit": "median mem",
            "extra": "avg mem: 145.96029022483555, max mem: 156.59375, count: 55485"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 7.611721705255461, max cpu: 28.042841, count: 55485"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 111.60546875,
            "unit": "median mem",
            "extra": "avg mem: 110.20275116585563, max mem: 111.60546875, count: 55485"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.998385932474398, max cpu: 13.967022, count: 55485"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 145.98828125,
            "unit": "median mem",
            "extra": "avg mem: 123.90991906596378, max mem: 145.98828125, count: 55485"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 30440,
            "unit": "median block_count",
            "extra": "avg block_count: 30983.565269892762, max block_count: 63333.0, count: 55485"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.57601869010603, max cpu: 4.7151275, count: 55485"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 99.7109375,
            "unit": "median mem",
            "extra": "avg mem: 88.76499602933225, max mem: 126.01171875, count: 55485"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 32,
            "unit": "median segment_count",
            "extra": "avg segment_count: 31.72648463548707, max segment_count: 56.0, count: 55485"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.266409,
            "unit": "median cpu",
            "extra": "avg cpu: 9.98672353937364, max cpu: 34.015747, count: 110970"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 147.15625,
            "unit": "median mem",
            "extra": "avg mem: 139.49671761652925, max mem: 156.109375, count: 110970"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.859479,
            "unit": "median cpu",
            "extra": "avg cpu: 12.885058295043974, max cpu: 33.23442, count: 55485"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 156.78515625,
            "unit": "median mem",
            "extra": "avg mem: 155.08578029253403, max mem: 158.41796875, count: 55485"
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
          "id": "849076799ca599dfbf0f2415149b12495b24624c",
          "message": "fix: Always use a shared expression context to execute HeapFilters. (#3174)\n\n## What\n\nEvaluating heap filters was leaking a `CreateStandaloneExprContext` per\nexecution, which could eventually lead to OOMs.\n\n## Why\n\n`PgBox::from_pg` does not free the resulting memory: it's expected to be\nfreed by a memory context, since PG created it.\n\n## How\n\nAlways use a global expression context created by\n`ExecAssignExprContext` in `begin_custom_scan`.\n\n## Tests\n\nA repro uses significantly less memory, and [a benchmark query added for\nthis purpose](https://github.com/paradedb/paradedb/pull/3175) is faster.",
          "timestamp": "2025-09-17T16:44:32-07:00",
          "tree_id": "7eef1c518a935389aa23e91c6bc47bbc325b18e6",
          "url": "https://github.com/paradedb/paradedb/commit/849076799ca599dfbf0f2415149b12495b24624c"
        },
        "date": 1758155730503,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.550726,
            "unit": "median cpu",
            "extra": "avg cpu: 18.29772029043088, max cpu: 42.64561, count: 55524"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 155.56640625,
            "unit": "median mem",
            "extra": "avg mem: 144.93084015470788, max mem: 155.56640625, count: 55524"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 7.691333646120417, max cpu: 37.24539, count: 55524"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 111.55078125,
            "unit": "median mem",
            "extra": "avg mem: 110.39466738201048, max mem: 111.55078125, count: 55524"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 5.020937432606781, max cpu: 14.145383, count: 55524"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 144.55078125,
            "unit": "median mem",
            "extra": "avg mem: 123.46326933229324, max mem: 144.9453125, count: 55524"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 30673,
            "unit": "median block_count",
            "extra": "avg block_count: 31101.616472156184, max block_count: 63621.0, count: 55524"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.316288582303742, max cpu: 4.64666, count: 55524"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 105.33984375,
            "unit": "median mem",
            "extra": "avg mem: 92.08110491915659, max mem: 130.8515625, count: 55524"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 32,
            "unit": "median segment_count",
            "extra": "avg segment_count: 31.946617678841584, max segment_count: 54.0, count: 55524"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.275363,
            "unit": "median cpu",
            "extra": "avg cpu: 9.950352128221436, max cpu: 37.24539, count: 111048"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 150.8671875,
            "unit": "median mem",
            "extra": "avg mem: 141.81730947006025, max mem: 157.6953125, count: 111048"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.913043,
            "unit": "median cpu",
            "extra": "avg cpu: 13.7701711813891, max cpu: 28.015566, count: 55524"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 155.73828125,
            "unit": "median mem",
            "extra": "avg mem: 154.3224225813387, max mem: 158.55078125, count: 55524"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "dc0e87d6fdabdb573d6f37a24c6c33befa43e8b0",
          "message": "fix: Always use a shared expression context to execute HeapFilters. (#3176)\n\n## What\n\nEvaluating heap filters was leaking a `CreateStandaloneExprContext` per\nexecution, which could eventually lead to OOMs.\n\n## Why\n\n`PgBox::from_pg` does not free the resulting memory: it's expected to be\nfreed by a memory context, since PG created it.\n\n## How\n\nAlways use a global expression context created by\n`ExecAssignExprContext` in `begin_custom_scan`.\n\n## Tests\n\nA repro uses significantly less memory, and [a benchmark query added for\nthis purpose](https://github.com/paradedb/paradedb/pull/3175) is faster.\n\nCo-authored-by: Stu Hood <stuhood@paradedb.com>",
          "timestamp": "2025-09-17T17:11:43-07:00",
          "tree_id": "0c30f446ad8404b4f66727777f1b6e6a5bc8958e",
          "url": "https://github.com/paradedb/paradedb/commit/dc0e87d6fdabdb573d6f37a24c6c33befa43e8b0"
        },
        "date": 1758157372518,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.497108,
            "unit": "median cpu",
            "extra": "avg cpu: 18.73088209462863, max cpu: 42.436146, count: 55450"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 155.61328125,
            "unit": "median mem",
            "extra": "avg mem: 144.17329843890892, max mem: 155.98828125, count: 55450"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 7.729472841649909, max cpu: 27.934044, count: 55450"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 111.3671875,
            "unit": "median mem",
            "extra": "avg mem: 110.16633066106853, max mem: 111.3671875, count: 55450"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 5.021887537579476, max cpu: 13.967022, count: 55450"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 143.46875,
            "unit": "median mem",
            "extra": "avg mem: 122.14387060414788, max mem: 144.984375, count: 55450"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 29840,
            "unit": "median block_count",
            "extra": "avg block_count: 30348.974301172228, max block_count: 61820.0, count: 55450"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.619827,
            "unit": "median cpu",
            "extra": "avg cpu: 4.53956593318033, max cpu: 4.6376815, count: 55450"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 99.51171875,
            "unit": "median mem",
            "extra": "avg mem: 90.0411246759468, max mem: 128.06640625, count: 55450"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 32,
            "unit": "median segment_count",
            "extra": "avg segment_count: 31.935094679891794, max segment_count: 54.0, count: 55450"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.248554,
            "unit": "median cpu",
            "extra": "avg cpu: 9.932756090323773, max cpu: 28.346458, count: 110900"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 149.9765625,
            "unit": "median mem",
            "extra": "avg mem: 139.24560284462353, max mem: 154.0, count: 110900"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.859479,
            "unit": "median cpu",
            "extra": "avg cpu: 13.35096854021878, max cpu: 27.906979, count: 55450"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 156.95703125,
            "unit": "median mem",
            "extra": "avg mem: 155.21588029756538, max mem: 158.4375, count: 55450"
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
          "id": "3bcb1451087be74b7bd73bfc7d6546423046a0ce",
          "message": "fix: write all delete files atomically (#3178)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nIn `ambulkdelete`, write all delete files to the meta list atomically,\ninstead of one at a time.\n\n## Why\n\nReduces the number of writes to the metas list.\n\n## How\n\n## Tests",
          "timestamp": "2025-09-18T16:03:10-04:00",
          "tree_id": "ad9609f0419a34b8f0cf543e911c1dc3c25d4563",
          "url": "https://github.com/paradedb/paradedb/commit/3bcb1451087be74b7bd73bfc7d6546423046a0ce"
        },
        "date": 1758228863656,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.60465,
            "unit": "median cpu",
            "extra": "avg cpu: 19.47234903111316, max cpu: 46.466602, count: 55922"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 158.3046875,
            "unit": "median mem",
            "extra": "avg mem: 143.61933041222596, max mem: 158.3046875, count: 55922"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 9.388210653661682, max cpu: 33.88235, count: 55922"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 114.73828125,
            "unit": "median mem",
            "extra": "avg mem: 113.21289125366582, max mem: 114.73828125, count: 55922"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 5.025218448474177, max cpu: 13.980582, count: 55922"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 144.6484375,
            "unit": "median mem",
            "extra": "avg mem: 118.5672591399628, max mem: 145.79296875, count: 55922"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 28897,
            "unit": "median block_count",
            "extra": "avg block_count: 30135.41107256536, max block_count: 63431.0, count: 55922"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.619827,
            "unit": "median cpu",
            "extra": "avg cpu: 4.102947651136923, max cpu: 4.7244096, count: 55922"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 92.38671875,
            "unit": "median mem",
            "extra": "avg mem: 86.86286199974965, max mem: 129.171875, count: 55922"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 31,
            "unit": "median segment_count",
            "extra": "avg segment_count: 31.42140839025786, max segment_count: 53.0, count: 55922"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.338522,
            "unit": "median cpu",
            "extra": "avg cpu: 11.20359234273246, max cpu: 33.267326, count: 111844"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 153.609375,
            "unit": "median mem",
            "extra": "avg mem: 140.80904680850114, max mem: 158.69921875, count: 111844"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.913043,
            "unit": "median cpu",
            "extra": "avg cpu: 13.33673111013111, max cpu: 28.070175, count: 55922"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 155.1328125,
            "unit": "median mem",
            "extra": "avg mem: 154.0424702878563, max mem: 156.16015625, count: 55922"
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
          "id": "6e11875ba052ccd6937ca0c535b3803309c8b6eb",
          "message": "feat: removed aggregation limitations re mix of aggregate functions and aggregation on group-by column. (#3179)\n\n# Ticket(s) Closed\n\n- Closes #2963\n\n## What\n\nRemoves aggregate limitations that prevented queries where the same\nfield is used in both `GROUP BY` and aggregate functions (e.g., `SELECT\nrating, AVG(rating) FROM table GROUP BY rating`).\n\n## Why\n\nPrevious safety checks blocked these queries due to Tantivy's\n\"incompatible fruit types\" errors, but testing shows the underlying\nissue is resolved. The limitations were overly restrictive and caused\nunnecessary fallbacks to slower PostgreSQL aggregation.\n\n## How\n\n- Removed `has_search_field_conflicts` function and field conflict\nvalidation\n- Eliminated ~35 lines of restrictive code in\n`extract_and_validate_aggregates`\n- Previously blocked queries now use faster `AggregateScan` instead of\n`GroupAggregate`\n\n## Tests\n\n- **`aggregate-groupby-conflict.sql`** - Tests `GROUP BY field` with\naggregates on same field\n- **`test-fruit-types-issue.sql`** - Validates #2963 issue resolution  \n- **`groupby_aggregate.out`** - Updated expectations showing\n`AggregateScan` usage",
          "timestamp": "2025-09-18T16:00:25-07:00",
          "tree_id": "f85924512f419186b824a986dd35eaa96d973884",
          "url": "https://github.com/paradedb/paradedb/commit/6e11875ba052ccd6937ca0c535b3803309c8b6eb"
        },
        "date": 1758239552093,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.568666,
            "unit": "median cpu",
            "extra": "avg cpu: 19.14796691610747, max cpu: 42.64561, count: 55429"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 155.78125,
            "unit": "median mem",
            "extra": "avg mem: 144.83542191925707, max mem: 156.57421875, count: 55429"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 7.760738821034685, max cpu: 28.125, count: 55429"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 110.86328125,
            "unit": "median mem",
            "extra": "avg mem: 109.72507150194393, max mem: 110.86328125, count: 55429"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 5.00202590381865, max cpu: 13.9265, count: 55429"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 142.8359375,
            "unit": "median mem",
            "extra": "avg mem: 121.61469585528333, max mem: 143.5859375, count: 55429"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 30139,
            "unit": "median block_count",
            "extra": "avg block_count: 30893.977538833464, max block_count: 63785.0, count: 55429"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.60263963600085, max cpu: 4.7244096, count: 55429"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 99.76171875,
            "unit": "median mem",
            "extra": "avg mem: 89.66276755793447, max mem: 128.32421875, count: 55429"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 31,
            "unit": "median segment_count",
            "extra": "avg segment_count: 31.654909884717387, max segment_count: 55.0, count: 55429"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.275363,
            "unit": "median cpu",
            "extra": "avg cpu: 10.129682550786187, max cpu: 42.561577, count: 110858"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 150.37109375,
            "unit": "median mem",
            "extra": "avg mem: 139.5345790405857, max mem: 154.234375, count: 110858"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.88621,
            "unit": "median cpu",
            "extra": "avg cpu: 13.237559786542835, max cpu: 28.015566, count: 55429"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 157.5703125,
            "unit": "median mem",
            "extra": "avg mem: 155.8559471209791, max mem: 159.05859375, count: 55429"
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
          "id": "020f92b742187fe6fdc75a19390692e6d2e9a373",
          "message": "feat: Port `ambulkdelete_epoch` to community (#3180)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nAdd `ambulkdelete_epoch` to the index metadata and make sure it's passed\ndown to all scans. In enterprise, this value is used to detect query\nconflicts with concurrent vacuums on the primary.\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-09-18T19:16:09-04:00",
          "tree_id": "3642b293b38caa7676318f888b910c3f934e1976",
          "url": "https://github.com/paradedb/paradedb/commit/020f92b742187fe6fdc75a19390692e6d2e9a373"
        },
        "date": 1758240454431,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.532818,
            "unit": "median cpu",
            "extra": "avg cpu: 18.440961098328316, max cpu: 42.27006, count: 55474"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 155.765625,
            "unit": "median mem",
            "extra": "avg mem: 144.45666290458144, max mem: 155.765625, count: 55474"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 7.731177759488853, max cpu: 33.8558, count: 55474"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 111.00390625,
            "unit": "median mem",
            "extra": "avg mem: 109.92793858102445, max mem: 111.00390625, count: 55474"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 5.010470350651836, max cpu: 9.4395275, count: 55474"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 144.76953125,
            "unit": "median mem",
            "extra": "avg mem: 123.41398095842196, max mem: 145.53515625, count: 55474"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 31045,
            "unit": "median block_count",
            "extra": "avg block_count: 31480.670277968056, max block_count: 64543.0, count: 55474"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.610951,
            "unit": "median cpu",
            "extra": "avg cpu: 4.264007275290997, max cpu: 4.6647234, count: 55474"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 100.66796875,
            "unit": "median mem",
            "extra": "avg mem: 89.80051479634604, max mem: 127.3046875, count: 55474"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 32,
            "unit": "median segment_count",
            "extra": "avg segment_count: 31.746656091141798, max segment_count: 53.0, count: 55474"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.266409,
            "unit": "median cpu",
            "extra": "avg cpu: 9.823762316757499, max cpu: 33.8558, count: 110948"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 150.62109375,
            "unit": "median mem",
            "extra": "avg mem: 140.13106579912437, max mem: 154.4765625, count: 110948"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.872832,
            "unit": "median cpu",
            "extra": "avg cpu: 12.695139733512804, max cpu: 27.853, count: 55474"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 157.19921875,
            "unit": "median mem",
            "extra": "avg mem: 155.4658726666997, max mem: 158.71875, count: 55474"
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
          "id": "c0dc0e674f3524c2f65a7b387ccbf594b23e5e0e",
          "message": "chore: Upgrade to `0.18.4` (#3181)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\n---------\n\nSigned-off-by: Ming <ming.ying.nyc@gmail.com>\nCo-authored-by: Stu Hood <stuhood@paradedb.com>",
          "timestamp": "2025-09-18T19:18:34-04:00",
          "tree_id": "b67f22553ed7786ef556afbfad2b7f8ddc6b139e",
          "url": "https://github.com/paradedb/paradedb/commit/c0dc0e674f3524c2f65a7b387ccbf594b23e5e0e"
        },
        "date": 1758240700823,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.461538,
            "unit": "median cpu",
            "extra": "avg cpu: 18.362618516327306, max cpu: 55.27831, count: 55560"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 155.94921875,
            "unit": "median mem",
            "extra": "avg mem: 144.2496597850297, max mem: 155.94921875, count: 55560"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.619827,
            "unit": "median cpu",
            "extra": "avg cpu: 7.737902516433426, max cpu: 27.934044, count: 55560"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 112.15625,
            "unit": "median mem",
            "extra": "avg mem: 111.01007138960134, max mem: 112.15625, count: 55560"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6153846,
            "unit": "median cpu",
            "extra": "avg cpu: 4.920916693869735, max cpu: 9.411765, count: 55560"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 141.58984375,
            "unit": "median mem",
            "extra": "avg mem: 120.85122509730472, max mem: 142.3671875, count: 55560"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 30325,
            "unit": "median block_count",
            "extra": "avg block_count: 30871.547516198705, max block_count: 62970.0, count: 55560"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.4984863106431, max cpu: 4.6376815, count: 55560"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 100.37109375,
            "unit": "median mem",
            "extra": "avg mem: 90.64469337765479, max mem: 129.68359375, count: 55560"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 31,
            "unit": "median segment_count",
            "extra": "avg segment_count: 31.458099352051835, max segment_count: 52.0, count: 55560"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.230769,
            "unit": "median cpu",
            "extra": "avg cpu: 10.146874230376516, max cpu: 28.290766, count: 111120"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 150.51171875,
            "unit": "median mem",
            "extra": "avg mem: 140.210701550126, max mem: 155.27734375, count: 111120"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 9.320388,
            "unit": "median cpu",
            "extra": "avg cpu: 11.719388167234936, max cpu: 27.612656, count: 55560"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 157.1875,
            "unit": "median mem",
            "extra": "avg mem: 155.48892715926476, max mem: 159.6328125, count: 55560"
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
          "id": "a97c525d0c11314fe01c8bbb250ea5fdf73ec8ce",
          "message": "fix: write all delete files atomically (#3178) (#3182)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nIn `ambulkdelete`, write all delete files to the meta list atomically,\ninstead of one at a time.\n\n## Why\n\nReduces the number of writes to the metas list.\n\n## How\n\n## Tests\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-09-18T21:50:00-04:00",
          "tree_id": "ba5917ed034f24a8e2ad95a64751e5faef3d55d5",
          "url": "https://github.com/paradedb/paradedb/commit/a97c525d0c11314fe01c8bbb250ea5fdf73ec8ce"
        },
        "date": 1758249759189,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.514948,
            "unit": "median cpu",
            "extra": "avg cpu: 18.376559193573033, max cpu: 41.941746, count: 55413"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 154.3359375,
            "unit": "median mem",
            "extra": "avg mem: 139.9079393739962, max mem: 154.3359375, count: 55413"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 9.343288874360514, max cpu: 28.015566, count: 55413"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 110.74609375,
            "unit": "median mem",
            "extra": "avg mem: 109.49861931654125, max mem: 110.74609375, count: 55413"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.937401949948193, max cpu: 13.913043, count: 55413"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 142.68359375,
            "unit": "median mem",
            "extra": "avg mem: 115.61034231024308, max mem: 144.61328125, count: 55413"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 27825,
            "unit": "median block_count",
            "extra": "avg block_count: 28989.14686084493, max block_count: 60789.0, count: 55413"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.619827,
            "unit": "median cpu",
            "extra": "avg cpu: 4.2034771429429885, max cpu: 4.6647234, count: 55413"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 91.8671875,
            "unit": "median mem",
            "extra": "avg mem: 84.21683342582065, max mem: 126.015625, count: 55413"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 31,
            "unit": "median segment_count",
            "extra": "avg segment_count: 31.62534062404129, max segment_count: 51.0, count: 55413"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.275363,
            "unit": "median cpu",
            "extra": "avg cpu: 10.768121849715648, max cpu: 32.495163, count: 110826"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 149.27734375,
            "unit": "median mem",
            "extra": "avg mem: 136.38122946527665, max mem: 154.859375, count: 110826"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.819577,
            "unit": "median cpu",
            "extra": "avg cpu: 12.077308244119186, max cpu: 28.235296, count: 55413"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 156.72265625,
            "unit": "median mem",
            "extra": "avg mem: 154.57593921157942, max mem: 158.1953125, count: 55413"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "e15e51abfc4b7834faea068d861d91d5d873580f",
          "message": "chore: Upgrade to `0.18.4` (#3184)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\nSigned-off-by: Ming <ming.ying.nyc@gmail.com>\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>\nCo-authored-by: Stu Hood <stuhood@paradedb.com>",
          "timestamp": "2025-09-18T21:52:13-04:00",
          "tree_id": "3d203e3468a4e7504d03af9c39ac9a0869033086",
          "url": "https://github.com/paradedb/paradedb/commit/e15e51abfc4b7834faea068d861d91d5d873580f"
        },
        "date": 1758249870215,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.497108,
            "unit": "median cpu",
            "extra": "avg cpu: 18.394909572655184, max cpu: 62.399994, count: 55570"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 156.484375,
            "unit": "median mem",
            "extra": "avg mem: 142.7506762304301, max mem: 157.609375, count: 55570"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 9.323227918679688, max cpu: 27.961164, count: 55570"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 110.88671875,
            "unit": "median mem",
            "extra": "avg mem: 109.70037621468418, max mem: 110.88671875, count: 55570"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.942137478611835, max cpu: 13.819577, count: 55570"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 142.171875,
            "unit": "median mem",
            "extra": "avg mem: 114.34473749325176, max mem: 142.953125, count: 55570"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 27717,
            "unit": "median block_count",
            "extra": "avg block_count: 28892.5878711535, max block_count: 60755.0, count: 55570"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.874068188826712, max cpu: 9.275363, count: 55570"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 90.6796875,
            "unit": "median mem",
            "extra": "avg mem: 85.55578247311949, max mem: 127.88671875, count: 55570"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 31,
            "unit": "median segment_count",
            "extra": "avg segment_count: 31.53413712434767, max segment_count: 50.0, count: 55570"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.257474,
            "unit": "median cpu",
            "extra": "avg cpu: 10.524196883448605, max cpu: 33.6, count: 111140"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 149.90625,
            "unit": "median mem",
            "extra": "avg mem: 136.35835344919695, max mem: 154.46875, count: 111140"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 9.329447,
            "unit": "median cpu",
            "extra": "avg cpu: 11.407198567513746, max cpu: 27.961164, count: 55570"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 156.0859375,
            "unit": "median mem",
            "extra": "avg mem: 154.38740974221702, max mem: 158.390625, count: 55570"
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
          "id": "1046018b2db9614ef172bd802c98a3987da7513e",
          "message": "chore: small diff ported from enterprise for query cancel fix (#3186)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nSome small changes in enterprise that should be in community\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-09-18T21:53:42-04:00",
          "tree_id": "85ed1f4eb7261157deabdfba479dc61164775f99",
          "url": "https://github.com/paradedb/paradedb/commit/1046018b2db9614ef172bd802c98a3987da7513e"
        },
        "date": 1758249977843,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.532818,
            "unit": "median cpu",
            "extra": "avg cpu: 18.48976886068057, max cpu: 41.69884, count: 55639"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 155.1171875,
            "unit": "median mem",
            "extra": "avg mem: 154.14579834682058, max mem: 155.1171875, count: 55639"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 7.65253059408901, max cpu: 28.042841, count: 55639"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 111.703125,
            "unit": "median mem",
            "extra": "avg mem: 110.52346853151566, max mem: 111.703125, count: 55639"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 5.036511518897995, max cpu: 13.980582, count: 55639"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 143.171875,
            "unit": "median mem",
            "extra": "avg mem: 122.18358399122019, max mem: 143.921875, count: 55639"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 30893,
            "unit": "median block_count",
            "extra": "avg block_count: 31456.438289688886, max block_count: 64374.0, count: 55639"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 3.7706203,
            "unit": "median cpu",
            "extra": "avg cpu: 3.811300365731306, max cpu: 4.6647234, count: 55639"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 102.859375,
            "unit": "median mem",
            "extra": "avg mem: 90.91879887814753, max mem: 128.765625, count: 55639"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 32,
            "unit": "median segment_count",
            "extra": "avg segment_count: 31.78843976347526, max segment_count: 54.0, count: 55639"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.266409,
            "unit": "median cpu",
            "extra": "avg cpu: 9.981390273284187, max cpu: 28.263002, count: 111278"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 150.359375,
            "unit": "median mem",
            "extra": "avg mem: 140.6080678851615, max mem: 156.08984375, count: 111278"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.859479,
            "unit": "median cpu",
            "extra": "avg cpu: 12.510985321250635, max cpu: 27.934044, count: 55639"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 156.375,
            "unit": "median mem",
            "extra": "avg mem: 154.5699546546712, max mem: 157.95703125, count: 55639"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "f052aabf25719cee68a756a379c6b66e39452759",
          "message": "feat: Port `ambulkdelete_epoch` to community (#3183)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nAdd `ambulkdelete_epoch` to the index metadata and make sure it's passed\ndown to all scans. In enterprise, this value is used to detect query\nconflicts with concurrent vacuums on the primary.\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-09-18T22:01:15-04:00",
          "tree_id": "48ffae94b2f43d5c2d62b5adb846d1dcc2992aee",
          "url": "https://github.com/paradedb/paradedb/commit/f052aabf25719cee68a756a379c6b66e39452759"
        },
        "date": 1758250355950,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.550726,
            "unit": "median cpu",
            "extra": "avg cpu: 19.08920458525631, max cpu: 45.933014, count: 55356"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 154.98828125,
            "unit": "median mem",
            "extra": "avg mem: 144.4317599520603, max mem: 154.98828125, count: 55356"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 7.689931459688417, max cpu: 27.906979, count: 55356"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 110.875,
            "unit": "median mem",
            "extra": "avg mem: 109.57582116211431, max mem: 110.875, count: 55356"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 5.016050192635771, max cpu: 13.9265, count: 55356"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 141.94921875,
            "unit": "median mem",
            "extra": "avg mem: 121.29500669529952, max mem: 142.71875, count: 55356"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 31124,
            "unit": "median block_count",
            "extra": "avg block_count: 31752.592763205434, max block_count: 65224.0, count: 55356"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.178378562058812, max cpu: 4.6511626, count: 55356"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 101.3984375,
            "unit": "median mem",
            "extra": "avg mem: 90.87121314762628, max mem: 129.9296875, count: 55356"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 31,
            "unit": "median segment_count",
            "extra": "avg segment_count: 31.603547944215624, max segment_count: 55.0, count: 55356"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.257474,
            "unit": "median cpu",
            "extra": "avg cpu: 9.733990634621863, max cpu: 28.374382, count: 110712"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 149.5,
            "unit": "median mem",
            "extra": "avg mem: 140.04244786727048, max mem: 155.265625, count: 110712"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.859479,
            "unit": "median cpu",
            "extra": "avg cpu: 12.854434485411426, max cpu: 27.87996, count: 55356"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 155.984375,
            "unit": "median mem",
            "extra": "avg mem: 154.36860467643527, max mem: 157.18359375, count: 55356"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "153f632ba06057571459a4b6e8767c135baf438c",
          "message": "chore: small diff ported from enterprise for query cancel fix (#3187)",
          "timestamp": "2025-09-18T22:31:35-04:00",
          "tree_id": "2c3b3f692c24ba8540a69da9d41f4d3a24d4ae6f",
          "url": "https://github.com/paradedb/paradedb/commit/153f632ba06057571459a4b6e8767c135baf438c"
        },
        "date": 1758252180911,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.532818,
            "unit": "median cpu",
            "extra": "avg cpu: 18.581035321291616, max cpu: 38.057484, count: 55496"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 155.30859375,
            "unit": "median mem",
            "extra": "avg mem: 143.85063577217278, max mem: 155.30859375, count: 55496"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 7.685638317233876, max cpu: 33.962265, count: 55496"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 111.83203125,
            "unit": "median mem",
            "extra": "avg mem: 110.66810945552382, max mem: 111.83203125, count: 55496"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 5.024883393580837, max cpu: 13.953489, count: 55496"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 141.67578125,
            "unit": "median mem",
            "extra": "avg mem: 120.77047536365234, max mem: 142.07421875, count: 55496"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 30568,
            "unit": "median block_count",
            "extra": "avg block_count: 31029.59827735332, max block_count: 63312.0, count: 55496"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.619827,
            "unit": "median cpu",
            "extra": "avg cpu: 4.55081505609179, max cpu: 4.64666, count: 55496"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 102.8125,
            "unit": "median mem",
            "extra": "avg mem: 91.12557288757297, max mem: 128.34375, count: 55496"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 32,
            "unit": "median segment_count",
            "extra": "avg segment_count: 31.740125414444286, max segment_count: 57.0, count: 55496"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.257474,
            "unit": "median cpu",
            "extra": "avg cpu: 9.894580215727125, max cpu: 37.73585, count: 110992"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 150.109375,
            "unit": "median mem",
            "extra": "avg mem: 140.51471157549642, max mem: 155.33984375, count: 110992"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.846154,
            "unit": "median cpu",
            "extra": "avg cpu: 12.26663532232743, max cpu: 32.36994, count: 55496"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 157.75390625,
            "unit": "median mem",
            "extra": "avg mem: 155.29191278988577, max mem: 158.5703125, count: 55496"
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
          "id": "8101a67174703310a6a1655496fd5296e869901d",
          "message": "fix: Clone an Arc rather than a OnceLock. (#3185)\n\n## What\n\nInvert our use of `OnceLock` to ensure that we clone an\n`Arc<OnceLock<T>>` rather than a `OnceLock<Arc<T>>`.\n\n## Why\n\n`OnceLock` implements `Clone` by cloning its contents to create a\nseparate disconnected copy. If what is desired is \"exactly once\nbehavior\", then cloning the `OnceLock` before it has been computed the\nfirst time will defeat that.\n\nThis change has no impact on benchmarks in this case, but\n`Arc<OnceLock<T>>` matches the intent of this code, and sets a better\nexample for future us.\n\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>",
          "timestamp": "2025-09-19T15:01:21-07:00",
          "tree_id": "de6adf9a09b874a0e133e9cbfeca50d417e6c5bf",
          "url": "https://github.com/paradedb/paradedb/commit/8101a67174703310a6a1655496fd5296e869901d"
        },
        "date": 1758322379167,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.497108,
            "unit": "median cpu",
            "extra": "avg cpu: 18.607199758954202, max cpu: 41.65863, count: 55450"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 157.1875,
            "unit": "median mem",
            "extra": "avg mem: 138.60685985403518, max mem: 157.56640625, count: 55450"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 10.922707711071492, max cpu: 42.27006, count: 55450"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 111.671875,
            "unit": "median mem",
            "extra": "avg mem: 110.29747731627593, max mem: 111.671875, count: 55450"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 5.000454690636974, max cpu: 13.9265, count: 55450"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 124.00390625,
            "unit": "median mem",
            "extra": "avg mem: 108.50386348906673, max mem: 144.3828125, count: 55450"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 24759,
            "unit": "median block_count",
            "extra": "avg block_count: 26815.287538322813, max block_count: 57004.0, count: 55450"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 3.9408416538118325, max cpu: 4.6421666, count: 55450"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 78.703125,
            "unit": "median mem",
            "extra": "avg mem: 77.40159596201534, max mem: 122.6484375, count: 55450"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 31,
            "unit": "median segment_count",
            "extra": "avg segment_count: 31.38605951307484, max segment_count: 52.0, count: 55450"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.284333,
            "unit": "median cpu",
            "extra": "avg cpu: 11.500935040956351, max cpu: 41.260746, count: 110900"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 148.953125,
            "unit": "median mem",
            "extra": "avg mem: 132.15496847525924, max mem: 154.37109375, count: 110900"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.819577,
            "unit": "median cpu",
            "extra": "avg cpu: 12.132791016282702, max cpu: 27.906979, count: 55450"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 157.87890625,
            "unit": "median mem",
            "extra": "avg mem: 155.66480859727233, max mem: 159.37109375, count: 55450"
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
          "id": "3163a5f3e48d3027585287ce8a63074f70ba1836",
          "message": "perf: Configurable Top N requeries more granularly (#3190)",
          "timestamp": "2025-09-19T21:06:04-04:00",
          "tree_id": "8c74bdf97c37281e4641be0e94b4d464daa5a3ea",
          "url": "https://github.com/paradedb/paradedb/commit/3163a5f3e48d3027585287ce8a63074f70ba1836"
        },
        "date": 1758333457350,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.479307,
            "unit": "median cpu",
            "extra": "avg cpu: 18.698913021521193, max cpu: 41.578438, count: 55389"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 159.328125,
            "unit": "median mem",
            "extra": "avg mem: 148.5027285708805, max mem: 161.94921875, count: 55389"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 7.756415120862889, max cpu: 41.7795, count: 55389"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 109.6953125,
            "unit": "median mem",
            "extra": "avg mem: 108.52517556227319, max mem: 109.6953125, count: 55389"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.619827,
            "unit": "median cpu",
            "extra": "avg cpu: 4.957523577148381, max cpu: 13.953489, count: 55389"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 142.578125,
            "unit": "median mem",
            "extra": "avg mem: 121.67881871569716, max mem: 143.40234375, count: 55389"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 30764,
            "unit": "median block_count",
            "extra": "avg block_count: 31318.67352723465, max block_count: 63907.0, count: 55389"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.619827,
            "unit": "median cpu",
            "extra": "avg cpu: 4.538424674765621, max cpu: 4.6421666, count: 55389"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 102.578125,
            "unit": "median mem",
            "extra": "avg mem: 90.8504201533698, max mem: 128.56640625, count: 55389"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 32,
            "unit": "median segment_count",
            "extra": "avg segment_count: 31.847839823791727, max segment_count: 51.0, count: 55389"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.239654,
            "unit": "median cpu",
            "extra": "avg cpu: 9.822419748978866, max cpu: 28.374382, count: 110778"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 151.09765625,
            "unit": "median mem",
            "extra": "avg mem: 141.013101618496, max mem: 155.51953125, count: 110778"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.806328,
            "unit": "median cpu",
            "extra": "avg cpu: 12.240556745944133, max cpu: 27.799229, count: 55389"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 156.375,
            "unit": "median mem",
            "extra": "avg mem: 154.61687663051327, max mem: 157.71875, count: 55389"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "f573a31e6704d95d0a62271a23ba47658a1dae06",
          "message": "perf: Configurable Top N requeries more granularly (#3194)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nAllow the retry scale factor and max chunk size to be tuned, which is\nuseful for reducing Top N requeries.\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-09-20T09:26:21-04:00",
          "tree_id": "d4ee2092267660be53cb68f8b760756a5a07ab69",
          "url": "https://github.com/paradedb/paradedb/commit/f573a31e6704d95d0a62271a23ba47658a1dae06"
        },
        "date": 1758377874566,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.497108,
            "unit": "median cpu",
            "extra": "avg cpu: 18.428652092825097, max cpu: 37.944664, count: 55426"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 154.07421875,
            "unit": "median mem",
            "extra": "avg mem: 139.09169123019882, max mem: 154.07421875, count: 55426"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 9.239418805111503, max cpu: 27.934044, count: 55426"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 111.3359375,
            "unit": "median mem",
            "extra": "avg mem: 109.76866833142388, max mem: 111.3359375, count: 55426"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 5.006034625819495, max cpu: 13.88621, count: 55426"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 141.1640625,
            "unit": "median mem",
            "extra": "avg mem: 113.7225115610228, max mem: 142.296875, count: 55426"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 27800,
            "unit": "median block_count",
            "extra": "avg block_count: 28946.083065709234, max block_count: 60773.0, count: 55426"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.447297685383547, max cpu: 4.6511626, count: 55426"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 91.4140625,
            "unit": "median mem",
            "extra": "avg mem: 84.79121831642641, max mem: 127.5078125, count: 55426"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 31,
            "unit": "median segment_count",
            "extra": "avg segment_count: 31.335348031609715, max segment_count: 52.0, count: 55426"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.275363,
            "unit": "median cpu",
            "extra": "avg cpu: 10.8205633943753, max cpu: 28.430405, count: 110852"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 149.27734375,
            "unit": "median mem",
            "extra": "avg mem: 135.8923135196591, max mem: 154.0078125, count: 110852"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.819577,
            "unit": "median cpu",
            "extra": "avg cpu: 12.156940937380213, max cpu: 27.934044, count: 55426"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 156.56640625,
            "unit": "median mem",
            "extra": "avg mem: 154.84863143820138, max mem: 157.81640625, count: 55426"
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
          "id": "4051856e9bea6cc1c5c5f61beb626af2f25b35c4",
          "message": "chore: Upgrade to `0.18.2` (#3144)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-09-10T17:33:06-04:00",
          "tree_id": "783d32dc8a220a1e0585e30bc3573d8af9a1767e",
          "url": "https://github.com/paradedb/paradedb/commit/4051856e9bea6cc1c5c5f61beb626af2f25b35c4"
        },
        "date": 1757540957998,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 763.2917914481199,
            "unit": "median tps",
            "extra": "avg tps: 762.6759576994908, max tps: 836.9407472254817, count: 55255"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2769.159235234248,
            "unit": "median tps",
            "extra": "avg tps: 2755.3424074886357, max tps: 2777.2930587596497, count: 55255"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 752.8560661153701,
            "unit": "median tps",
            "extra": "avg tps: 751.4041025259525, max tps: 814.8469766796755, count: 55255"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 648.1166935125998,
            "unit": "median tps",
            "extra": "avg tps: 645.0101072041421, max tps: 651.7928791242222, count: 55255"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 534.1971829244044,
            "unit": "median tps",
            "extra": "avg tps: 537.041689870793, max tps: 572.8003798194989, count: 110510"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 449.8060958580798,
            "unit": "median tps",
            "extra": "avg tps: 448.857309027427, max tps: 458.4510044506014, count: 55255"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 22.88609970375307,
            "unit": "median tps",
            "extra": "avg tps: 30.456562520502008, max tps: 869.8814003698736, count: 55255"
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
          "id": "da997100cd0e2873fa8692ec6c2382761719ce58",
          "message": "chore: Upgrade to `0.18.2` (#3144) (#3145)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-09-10T17:44:33-04:00",
          "tree_id": "d0d9fd4cb9ebc554c1e7f3e029694e863f4247c9",
          "url": "https://github.com/paradedb/paradedb/commit/da997100cd0e2873fa8692ec6c2382761719ce58"
        },
        "date": 1757541737574,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 751.7068558793982,
            "unit": "median tps",
            "extra": "avg tps: 749.6347941584683, max tps: 799.7295298240866, count: 55268"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2657.824557300422,
            "unit": "median tps",
            "extra": "avg tps: 2634.4889568207095, max tps: 2676.0213627384946, count: 55268"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 760.376295862273,
            "unit": "median tps",
            "extra": "avg tps: 758.829545973528, max tps: 802.348166732909, count: 55268"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 667.2795393328531,
            "unit": "median tps",
            "extra": "avg tps: 665.4818313127627, max tps: 678.6627662786384, count: 55268"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 557.7586497496695,
            "unit": "median tps",
            "extra": "avg tps: 590.0261773813613, max tps: 678.8962005411709, count: 110536"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 502.391752745918,
            "unit": "median tps",
            "extra": "avg tps: 490.7898729224728, max tps: 519.207250360319, count: 55268"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 32.60688205436597,
            "unit": "median tps",
            "extra": "avg tps: 39.67942600960603, max tps: 825.1914444151042, count: 55268"
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
          "id": "4051856e9bea6cc1c5c5f61beb626af2f25b35c4",
          "message": "chore: Upgrade to `0.18.2` (#3144)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-09-10T17:33:06-04:00",
          "tree_id": "783d32dc8a220a1e0585e30bc3573d8af9a1767e",
          "url": "https://github.com/paradedb/paradedb/commit/4051856e9bea6cc1c5c5f61beb626af2f25b35c4"
        },
        "date": 1757625494208,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 763.8003479621017,
            "unit": "median tps",
            "extra": "avg tps: 763.2658262432113, max tps: 797.9964352977181, count: 55201"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2685.4582341396704,
            "unit": "median tps",
            "extra": "avg tps: 2673.127018966212, max tps: 2694.3156082240316, count: 55201"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 739.8993551676407,
            "unit": "median tps",
            "extra": "avg tps: 740.2461678222123, max tps: 808.705816493195, count: 55201"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 637.77350717551,
            "unit": "median tps",
            "extra": "avg tps: 637.8881360198176, max tps: 708.2672049888192, count: 55201"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 543.9553852170275,
            "unit": "median tps",
            "extra": "avg tps: 542.3832862943182, max tps: 577.8181089688352, count: 110402"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 450.58307163617184,
            "unit": "median tps",
            "extra": "avg tps: 449.4649218978022, max tps: 455.2675106809652, count: 55201"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 28.63947027605233,
            "unit": "median tps",
            "extra": "avg tps: 33.62961904138893, max tps: 908.6209042776964, count: 55201"
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
          "id": "1cfaa7b311ca8b7ee91491411c8dfecd2ce5619c",
          "message": "fix: `GROUP BY` doesn't panic when Postgres eliminates group pathkeys (#3152)\n\n# Ticket(s) Closed\n\n- Closes #3050 \n\n## What\n\nIt's possible for Postgres to eliminate group pathkeys if it realizes\nthat one of the pathkeys is unique, making the other ones unnecessary.\n\nWe need to handle this case/not panic.\n\n## Why\n\nSee issue.\n\n## How\n\nInject the dropped group pathkeys back into our list of grouping\ncolumns.\n\n## Tests\n\nAdded regression test",
          "timestamp": "2025-09-14T17:56:19-04:00",
          "tree_id": "a41824569d62cfd5dbe40884e6ead540d3b1bd88",
          "url": "https://github.com/paradedb/paradedb/commit/1cfaa7b311ca8b7ee91491411c8dfecd2ce5619c"
        },
        "date": 1757887951841,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 762.8537314260527,
            "unit": "median tps",
            "extra": "avg tps: 762.8312879137642, max tps: 828.0112774492874, count: 55135"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2763.218217546463,
            "unit": "median tps",
            "extra": "avg tps: 2741.0457021362236, max tps: 2772.062958991789, count: 55135"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 761.0120474812861,
            "unit": "median tps",
            "extra": "avg tps: 762.2405535804423, max tps: 849.4352635829478, count: 55135"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 619.5486196460236,
            "unit": "median tps",
            "extra": "avg tps: 620.7802872777763, max tps: 685.3672203614054, count: 55135"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 563.4228422521315,
            "unit": "median tps",
            "extra": "avg tps: 568.8334873225288, max tps: 603.0823544702331, count: 110270"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 480.48017540354704,
            "unit": "median tps",
            "extra": "avg tps: 476.17913128854144, max tps: 484.51780814409295, count: 55135"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 34.062701063910566,
            "unit": "median tps",
            "extra": "avg tps: 37.13342029853091, max tps: 934.3654957509729, count: 55135"
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
          "id": "a521487756693e82c46bfe2f1a2f2fd3aded0136",
          "message": "fix: fixed `rt_fetch out-of-bounds` error (#3141)\n\n# Ticket(s) Closed\n\n- Closes #3135\n\n## What\n\nFixed `rt_fetch used out-of-bounds` and `Cannot open relation with\noid=0` errors that occurred in complex SQL queries with nested `OR\nEXISTS` clauses, multiple `JOIN`s.\n\n## Why\n\nThe issue occurred when PostgreSQL's query planner generated `Var` nodes\nreferencing Range Table Entries (RTEs) that were valid in outer planning\ncontexts but didn't exist in inner execution contexts. This happened\nspecifically with:\n- `OR EXISTS` subqueries (not `AND EXISTS`)  \n- Multiple `JOIN`s within the `EXISTS` clause\n- ParadeDB functions applied to joined tables\n\nWhen ParadeDB's custom scan tried to access these out-of-bounds RTEs\nusing `rt_fetch`, it caused crashes.\n\n## How\n\nImplemented bounds checking across the codebase:\n\n1. **Early detection**: Added bounds checking in `find_var_relation()`\nto detect invalid `varno` values and return `pg_sys::InvalidOid`. This\nwas the main fix for the issue.\n2. **Graceful handling**: Modified all functions that receive relation\nOIDs to check for `InvalidOid` before attempting to open relations\n3. **Safe fallbacks**: Updated query optimization logic to skip\noptimizations when relation information is unavailable rather than\ncrashing\n\n## Tests\n\nAdded regression test `or_exists_join_bug.sql` covering:\n- Simple queries (baseline functionality)\n- `AND EXISTS` with multiple `JOIN`s (should work)  \n- `OR EXISTS` with multiple `JOIN`s (the problematic case, now fixed)\n- Various edge cases and workarounds\n- Minimal reproduction cases\n\n---------\n\nSigned-off-by: Moe <mdashti@gmail.com>",
          "timestamp": "2025-09-15T02:47:52-07:00",
          "tree_id": "4a0b5db116e0263111295cc53d05810e093ce68c",
          "url": "https://github.com/paradedb/paradedb/commit/a521487756693e82c46bfe2f1a2f2fd3aded0136"
        },
        "date": 1757930636326,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 727.5141638354312,
            "unit": "median tps",
            "extra": "avg tps: 729.229175314588, max tps: 817.6312864234719, count: 55076"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2596.5522944987374,
            "unit": "median tps",
            "extra": "avg tps: 2583.948045540999, max tps: 2609.8656226573853, count: 55076"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 766.0570105220943,
            "unit": "median tps",
            "extra": "avg tps: 765.8968901656756, max tps: 797.4129883909986, count: 55076"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 628.9429641202081,
            "unit": "median tps",
            "extra": "avg tps: 629.2852489506965, max tps: 661.8009934076852, count: 55076"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 490.40179045843627,
            "unit": "median tps",
            "extra": "avg tps: 527.9336015512919, max tps: 610.2544466439864, count: 110152"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 394.8497752245124,
            "unit": "median tps",
            "extra": "avg tps: 396.8931019944775, max tps: 416.99472205720707, count: 55076"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 27.524271709755265,
            "unit": "median tps",
            "extra": "avg tps: 30.30492124646964, max tps: 894.5527996371694, count: 55076"
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
          "id": "b37fc5af676e3438c051381414d81996ed0fb8f6",
          "message": "feat: push down `group by ... order by ... limit` (#3134)\n\n# Ticket(s) Closed\n\n- Closes #3131 \n- Opens #3156 #3155 \n\n## What\n\nPushes down `group by ... order by ... limit` to Tantivy\n\n## Why\n\nBy pushing down the sort/limit to Tantivy, we can significantly speed up\n`group by` queries over high cardinality columns.\n\n## How\n\n- Before we were hard-coding a bucket size and sorting the results\nourselves, now the bucket size is set to the limit and we push the sort\ndown to the Tantivy term agg\n\n## Tests",
          "timestamp": "2025-09-15T15:51:50-04:00",
          "tree_id": "e58df02d60abc13101aaae8ef6333a9afafbcd78",
          "url": "https://github.com/paradedb/paradedb/commit/b37fc5af676e3438c051381414d81996ed0fb8f6"
        },
        "date": 1757966874673,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 756.509534753274,
            "unit": "median tps",
            "extra": "avg tps: 755.1118296523208, max tps: 818.3484884906408, count: 55041"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2627.109288199017,
            "unit": "median tps",
            "extra": "avg tps: 2607.304504067176, max tps: 2634.1901368999, count: 55041"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 751.0834282841528,
            "unit": "median tps",
            "extra": "avg tps: 751.0809993177268, max tps: 820.0966388489886, count: 55041"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 604.4560692304533,
            "unit": "median tps",
            "extra": "avg tps: 606.1407268497844, max tps: 677.5477319202598, count: 55041"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 637.5166105493716,
            "unit": "median tps",
            "extra": "avg tps: 628.582015511482, max tps: 678.3753573026895, count: 110082"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 461.84877737624635,
            "unit": "median tps",
            "extra": "avg tps: 461.22774157636644, max tps: 478.64930844059273, count: 55041"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 28.4524816130594,
            "unit": "median tps",
            "extra": "avg tps: 35.89928199895787, max tps: 944.0018124834799, count: 55041"
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
          "id": "8211eef7a0dd34237afebfa91364fb66c65a4906",
          "message": "perf: remove `ExactBuffer` in favor of a regular rust `BufWriter` (#3158)\n\n# Ticket(s) Closed\n\n- Closes #2981  (/cc @yjhjstz)\n\nWhile #2981 wasn't the impetus for this, it addresses the complaint made\nthere just the same.\n\n## What\n\nIn profiling, our `ExactBuffer` was a large percentage of certain\nprofiles. This replaces it, and its complexity, with a standard Rust\n`BufWriter`.\n\n## Why\n\nImproves performance of (at least) our `wide-table.toml` test's \"Single\nUpdate\" job by quite a bit.\n\n<img width=\"720\" height=\"141\" alt=\"screenshot_2025-09-15_at_3 28\n33___pm_720\"\nsrc=\"https://github.com/user-attachments/assets/a373a7ae-df38-4691-980a-d6843f073d26\"\n/>\n\n\n## How\n\n## Tests\n\nExisting tests pass",
          "timestamp": "2025-09-15T15:55:52-04:00",
          "tree_id": "4ddf140542c5525034023441aadac4b634c90fc6",
          "url": "https://github.com/paradedb/paradedb/commit/8211eef7a0dd34237afebfa91364fb66c65a4906"
        },
        "date": 1757967117785,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 780.7130493925574,
            "unit": "median tps",
            "extra": "avg tps: 781.7541433145838, max tps: 835.4275878518247, count: 55285"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2857.6015974890747,
            "unit": "median tps",
            "extra": "avg tps: 2838.1756282342694, max tps: 2861.5984667389303, count: 55285"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 793.5351389063458,
            "unit": "median tps",
            "extra": "avg tps: 794.4773335564745, max tps: 806.4060544414299, count: 55285"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 684.2267500844193,
            "unit": "median tps",
            "extra": "avg tps: 682.5842484808164, max tps: 719.3178412721517, count: 55285"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1655.648269027543,
            "unit": "median tps",
            "extra": "avg tps: 1657.9144699265867, max tps: 1712.7427535567076, count: 110570"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1201.1887306178205,
            "unit": "median tps",
            "extra": "avg tps: 1197.7192231072172, max tps: 1211.3586054422033, count: 55285"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 50.017852271797146,
            "unit": "median tps",
            "extra": "avg tps: 58.026707506260784, max tps: 1110.6372392223764, count: 55285"
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
          "id": "288a4bfa0c79838d86711b8a6231687c984ac0b5",
          "message": "chore: Upgrade to `0.18.3` (#3160)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-09-15T16:13:06-04:00",
          "tree_id": "ad59a6c86e8afe29cabad5b0bcc6a78bc448182e",
          "url": "https://github.com/paradedb/paradedb/commit/288a4bfa0c79838d86711b8a6231687c984ac0b5"
        },
        "date": 1757968147248,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 784.2151970040312,
            "unit": "median tps",
            "extra": "avg tps: 784.2395856217864, max tps: 845.6990426177692, count: 54748"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2847.991679070839,
            "unit": "median tps",
            "extra": "avg tps: 2824.505799531453, max tps: 2855.6592838875613, count: 54748"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 782.0174456994391,
            "unit": "median tps",
            "extra": "avg tps: 781.481663944179, max tps: 863.5553929016663, count: 54748"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 659.4473547018084,
            "unit": "median tps",
            "extra": "avg tps: 660.1764213469486, max tps: 720.1459722792247, count: 54748"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1631.9783538373697,
            "unit": "median tps",
            "extra": "avg tps: 1641.3636519108911, max tps: 1702.3027274298718, count: 109496"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1176.0164905249412,
            "unit": "median tps",
            "extra": "avg tps: 1172.8511832262661, max tps: 1180.933109071982, count: 54748"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 44.97214597671429,
            "unit": "median tps",
            "extra": "avg tps: 58.371324324546386, max tps: 962.0083656247475, count: 54748"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "af5bea23effe976b411147e259e53afad947a393",
          "message": "perf: remove `ExactBuffer` in favor of a regular rust `BufWriter` (#3159)\n\n# Ticket(s) Closed\n\n- Closes #2981  (/cc @yjhjstz)\n\nWhile #2981 wasn't the impetus for this, it addresses the complaint made\nthere just the same.\n\n## What\n\nIn profiling, our `ExactBuffer` was a large percentage of certain\nprofiles. This replaces it, and its complexity, with a standard Rust\n`BufWriter`.\n\n## Why\n\nImproves performance of (at least) our `wide-table.toml` test's \"Single\nUpdate\" job by quite a bit.\n\n<img width=\"720\" height=\"141\" alt=\"screenshot_2025-09-15_at_3 28\n33___pm_720\"\nsrc=\"https://github.com/user-attachments/assets/a373a7ae-df38-4691-980a-d6843f073d26\"\n/>\n\n\n## How\n\n## Tests\n\nExisting tests pass\n\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>",
          "timestamp": "2025-09-15T16:26:19-04:00",
          "tree_id": "cbc00b9a93c129255360f60e5a70904e87f1e8c1",
          "url": "https://github.com/paradedb/paradedb/commit/af5bea23effe976b411147e259e53afad947a393"
        },
        "date": 1757968933783,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 790.1583011008929,
            "unit": "median tps",
            "extra": "avg tps: 792.9538890402961, max tps: 858.7894594915847, count: 55374"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2784.5684994069625,
            "unit": "median tps",
            "extra": "avg tps: 2761.832361142758, max tps: 2791.0968994067907, count: 55374"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 779.5997671835739,
            "unit": "median tps",
            "extra": "avg tps: 780.0088625672128, max tps: 811.5576484665061, count: 55374"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 690.4710444669197,
            "unit": "median tps",
            "extra": "avg tps: 690.1958653571413, max tps: 698.9127690509614, count: 55374"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1654.8910393175106,
            "unit": "median tps",
            "extra": "avg tps: 1650.4881955145875, max tps: 1684.1349350303165, count: 110748"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1190.4038904061076,
            "unit": "median tps",
            "extra": "avg tps: 1185.8308558151043, max tps: 1204.6756387801856, count: 55374"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 38.14879266103322,
            "unit": "median tps",
            "extra": "avg tps: 46.5598963742497, max tps: 914.622713557544, count: 55374"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "7800d096e107acdbdec6297d0cb98ef030569e2b",
          "message": "chore: Upgrade to `0.18.3` (#3161)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-09-15T16:36:57-04:00",
          "tree_id": "c0962cc02d5690156721fd003c985f724ee9b20f",
          "url": "https://github.com/paradedb/paradedb/commit/7800d096e107acdbdec6297d0cb98ef030569e2b"
        },
        "date": 1757969569970,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 767.6301064144427,
            "unit": "median tps",
            "extra": "avg tps: 769.1393790092588, max tps: 849.4087622357757, count: 55412"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2672.764578160481,
            "unit": "median tps",
            "extra": "avg tps: 2666.1697332148347, max tps: 2746.9908432181824, count: 55412"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 794.1829178689102,
            "unit": "median tps",
            "extra": "avg tps: 793.6088060780927, max tps: 847.1441359271004, count: 55412"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 663.8010644687151,
            "unit": "median tps",
            "extra": "avg tps: 662.6703411399277, max tps: 699.5186556169208, count: 55412"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1651.9734649105803,
            "unit": "median tps",
            "extra": "avg tps: 1645.7792885562503, max tps: 1666.2872189729574, count: 110824"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1193.4749024409025,
            "unit": "median tps",
            "extra": "avg tps: 1189.8480289877193, max tps: 1204.5882135767672, count: 55412"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 58.466036808427916,
            "unit": "median tps",
            "extra": "avg tps: 67.6512998485621, max tps: 532.7976940515802, count: 55412"
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
          "id": "f71a5572d645d23e58b949cc3f16645473c74735",
          "message": "chore: Sync `0.18.x` (#3162)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\n---------\n\nSigned-off-by: Moe <mdashti@gmail.com>\nCo-authored-by: Moe <mdashti@gmail.com>",
          "timestamp": "2025-09-15T17:11:39-04:00",
          "tree_id": "a75daf7f281149ef4317505338649d8b0d2ec8a4",
          "url": "https://github.com/paradedb/paradedb/commit/f71a5572d645d23e58b949cc3f16645473c74735"
        },
        "date": 1757971670385,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 794.6389670499436,
            "unit": "median tps",
            "extra": "avg tps: 796.0759000849945, max tps: 890.4012078589185, count: 55284"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2857.049535024931,
            "unit": "median tps",
            "extra": "avg tps: 2834.94680140203, max tps: 2870.639846986761, count: 55284"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 793.4480498973464,
            "unit": "median tps",
            "extra": "avg tps: 794.1367930219722, max tps: 852.6442910986175, count: 55284"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 657.128059738158,
            "unit": "median tps",
            "extra": "avg tps: 655.5488696594324, max tps: 702.3120463722598, count: 55284"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1636.709356008699,
            "unit": "median tps",
            "extra": "avg tps: 1635.6737523764286, max tps: 1660.438442714089, count: 110568"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1206.0109748350121,
            "unit": "median tps",
            "extra": "avg tps: 1200.3598028160359, max tps: 1210.9564656727237, count: 55284"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 55.50543611502157,
            "unit": "median tps",
            "extra": "avg tps: 83.48466357797707, max tps: 927.8882377175433, count: 55284"
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
          "id": "878c50feef96d61785ad711ebe46250c920bed70",
          "message": "fix: sequential scan segfault (#3163)\n\n# Ticket(s) Closed\n\n- Closes #3151 \n\n## What\n\nThe `@@@` return type should be `bool`, not `SearchQueryInput`.\n\n## Why\n\n## How\n\n## Tests\n\nAdded regression test.",
          "timestamp": "2025-09-16T10:27:13-04:00",
          "tree_id": "6859469869310b79c8c32af68b3ed77dfb787362",
          "url": "https://github.com/paradedb/paradedb/commit/878c50feef96d61785ad711ebe46250c920bed70"
        },
        "date": 1758033799315,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 797.3675978022522,
            "unit": "median tps",
            "extra": "avg tps: 797.6847403600094, max tps: 870.1296157625586, count: 55171"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2765.189783369289,
            "unit": "median tps",
            "extra": "avg tps: 2750.54731688714, max tps: 2770.9693496992895, count: 55171"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 782.5882299984638,
            "unit": "median tps",
            "extra": "avg tps: 781.7753118443845, max tps: 820.3794923175269, count: 55171"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 679.7327612744666,
            "unit": "median tps",
            "extra": "avg tps: 677.8147367553728, max tps: 697.5362828253441, count: 55171"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1654.8602244282442,
            "unit": "median tps",
            "extra": "avg tps: 1650.7242531684508, max tps: 1675.5981273308585, count: 110342"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1199.893962802415,
            "unit": "median tps",
            "extra": "avg tps: 1192.4071111359256, max tps: 1206.8330955849226, count: 55171"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 65.90708817354783,
            "unit": "median tps",
            "extra": "avg tps: 84.66300895513895, max tps: 561.5125801278451, count: 55171"
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
          "id": "f2a0c9c43e4385628cc7b828a8ed12c30e55050e",
          "message": "chore: don't warn about `raw` tokenizer on UUID key fields (#3166)\n\n## What\n\nRemove the warning about using the `raw` tokenizer when the `key_field`\nis a UUID field.\n\nThe drama here is that we (pg_search) assign the `raw` tokenizer to UUID\nfields used as the key_field so there's nothing a user can do about it.\nWarning about our own bad decisions is not cool.\n\n## Why\n\nMany user and customer complaints.\n\n## How\n\n## Tests\n\nExisting tests pass but a couple of the regression test expected output\nis now different.",
          "timestamp": "2025-09-16T13:10:47-04:00",
          "tree_id": "2b24aea6e3a0645c584d8ebb8ce7465c8c90f904",
          "url": "https://github.com/paradedb/paradedb/commit/f2a0c9c43e4385628cc7b828a8ed12c30e55050e"
        },
        "date": 1758043614885,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 789.8206180184412,
            "unit": "median tps",
            "extra": "avg tps: 790.3375602845325, max tps: 840.3482093465142, count: 55255"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2792.600036179828,
            "unit": "median tps",
            "extra": "avg tps: 2772.5328715242326, max tps: 2798.5284818798805, count: 55255"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 785.1372951585745,
            "unit": "median tps",
            "extra": "avg tps: 787.355524220417, max tps: 805.4479514325625, count: 55255"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 678.5730105376329,
            "unit": "median tps",
            "extra": "avg tps: 676.7962371380088, max tps: 682.5460997890447, count: 55255"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1636.8016432873083,
            "unit": "median tps",
            "extra": "avg tps: 1635.8794435283778, max tps: 1667.6987979283367, count: 110510"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1196.859706025405,
            "unit": "median tps",
            "extra": "avg tps: 1191.8165327552588, max tps: 1203.8828990611114, count: 55255"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 47.63271206841265,
            "unit": "median tps",
            "extra": "avg tps: 64.81404693026934, max tps: 1046.1060793408694, count: 55255"
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
          "id": "489eb48583040067612195f9e1406d5e31a1599f",
          "message": "perf: teach custom scan callback to exit early if it can (#3168)\n\n## What\n\nThis does two things.  \n\nOne, the first commit (62d752572b2d7bc5a02b7203ac2c83949e38e27e) simply\nreorders some code in the custom scan callback so it can decide to exit\nearly if we're not going to submit a path. Specifically, this is\nintended to avoid opening a Directory and Index and related structures.\n\nTwo, the second commit (5ac1dde23ef0809bea4b942d04fd14acc9d1c152) makes\na new decision to not evaluate possible pushdown predicates when the\nstatement type is not a SELECT statement. This cuts out the overhead of\nneeding to read/deserialize the index's schema at all on (at least)\nUPDATE statements.\n\nThis does mean that we won't consider doing pushdowns for UPDATE\nstatements, even if doing one would make the UPDATE scan faster.\n\n## Why\n\nTrying to reduce per-query overhead, targeting our stressgres benchmarks\nlike \"single-server.toml\" and \"wide-table.toml\".\n\n## How\n\n## Tests\n\nAll existing tests pass.",
          "timestamp": "2025-09-16T17:39:51-04:00",
          "tree_id": "0ebcd01c6225cbb43b199470f7f78bd694493ed7",
          "url": "https://github.com/paradedb/paradedb/commit/489eb48583040067612195f9e1406d5e31a1599f"
        },
        "date": 1758059756090,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 806.0336550104148,
            "unit": "median tps",
            "extra": "avg tps: 806.1063183205249, max tps: 822.4021557726397, count: 55419"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3164.9789828148246,
            "unit": "median tps",
            "extra": "avg tps: 3162.8361739204975, max tps: 3307.2535630073467, count: 55419"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 758.9357070980343,
            "unit": "median tps",
            "extra": "avg tps: 760.4287041814945, max tps: 842.383499925791, count: 55419"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 665.5676070892193,
            "unit": "median tps",
            "extra": "avg tps: 664.366087358773, max tps: 671.288462155715, count: 55419"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1664.9681249819546,
            "unit": "median tps",
            "extra": "avg tps: 1661.1135462893596, max tps: 1687.606917944214, count: 110838"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1252.5021019795429,
            "unit": "median tps",
            "extra": "avg tps: 1248.1540263964848, max tps: 1263.256921365826, count: 55419"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 53.58778601742059,
            "unit": "median tps",
            "extra": "avg tps: 69.86918061388074, max tps: 572.8784449329188, count: 55419"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "63daa7f2bf568127e538f19f942d6363508ca615",
          "message": "chore: don't warn about `raw` tokenizer on UUID key fields (#3167)\n\n## What\n\nRemove the warning about using the `raw` tokenizer when the `key_field`\nis a UUID field.\n\nThe drama here is that we (pg_search) assign the `raw` tokenizer to UUID\nfields used as the key_field so there's nothing a user can do about it.\nWarning about our own bad decisions is not cool.\n\n## Why\n\nMany user and customer complaints.\n\n## How\n\n## Tests\n\nExisting tests pass but a couple of the regression test expected output\nis now different.\n\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>",
          "timestamp": "2025-09-17T10:06:31-04:00",
          "tree_id": "2c472616485a1c2a1ed61c7f2c030286882deb06",
          "url": "https://github.com/paradedb/paradedb/commit/63daa7f2bf568127e538f19f942d6363508ca615"
        },
        "date": 1758118964553,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 785.8818777240242,
            "unit": "median tps",
            "extra": "avg tps: 784.0698832441469, max tps: 847.4170272391838, count: 54896"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3379.113664190456,
            "unit": "median tps",
            "extra": "avg tps: 3350.5892947886614, max tps: 3390.457215726335, count: 54896"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 781.0821594323393,
            "unit": "median tps",
            "extra": "avg tps: 779.6715208368831, max tps: 849.4963414543122, count: 54896"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 680.8374586555161,
            "unit": "median tps",
            "extra": "avg tps: 681.5011565302483, max tps: 700.6012431097423, count: 54896"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1622.013748318342,
            "unit": "median tps",
            "extra": "avg tps: 1637.3832785937282, max tps: 1679.2093271597973, count: 109792"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1244.7838361598106,
            "unit": "median tps",
            "extra": "avg tps: 1243.2139936186613, max tps: 1253.1232975880207, count: 54896"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 50.7669939800257,
            "unit": "median tps",
            "extra": "avg tps: 78.24660420401776, max tps: 1139.293432998723, count: 54896"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "2d344837c944c402f2781abd28b10a2f0e7185ac",
          "message": "perf: teach custom scan callback to exit early if it can (#3169)\n\n## What\n\nThis does two things.  \n\nOne, the first commit (62d752572b2d7bc5a02b7203ac2c83949e38e27e) simply\nreorders some code in the custom scan callback so it can decide to exit\nearly if we're not going to submit a path. Specifically, this is\nintended to avoid opening a Directory and Index and related structures.\n\nTwo, the second commit (5ac1dde23ef0809bea4b942d04fd14acc9d1c152) makes\na new decision to not evaluate possible pushdown predicates when the\nstatement type is not a SELECT statement. This cuts out the overhead of\nneeding to read/deserialize the index's schema at all on (at least)\nUPDATE statements.\n\nThis does mean that we won't consider doing pushdowns for UPDATE\nstatements, even if doing one would make the UPDATE scan faster.\n\n## Why\n\nTrying to reduce per-query overhead, targeting our stressgres benchmarks\nlike \"single-server.toml\" and \"wide-table.toml\".\n\n## How\n\n## Tests\n\nAll existing tests pass.\n\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>",
          "timestamp": "2025-09-17T10:06:44-04:00",
          "tree_id": "efcbedd45680505ce75bd6fe8a623a0066b38fdb",
          "url": "https://github.com/paradedb/paradedb/commit/2d344837c944c402f2781abd28b10a2f0e7185ac"
        },
        "date": 1758118980251,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 790.5957557502066,
            "unit": "median tps",
            "extra": "avg tps: 792.0606654585147, max tps: 813.7528157660241, count: 55443"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3384.2650420510104,
            "unit": "median tps",
            "extra": "avg tps: 3372.828891928334, max tps: 3395.0341986631574, count: 55443"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 775.3541887199235,
            "unit": "median tps",
            "extra": "avg tps: 775.6694610535359, max tps: 861.8037304854556, count: 55443"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 691.9042104903008,
            "unit": "median tps",
            "extra": "avg tps: 690.7020641144002, max tps: 716.0505287897901, count: 55443"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1635.5033015717163,
            "unit": "median tps",
            "extra": "avg tps: 1643.889668052891, max tps: 1691.9088178238912, count: 110886"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1245.9333296030711,
            "unit": "median tps",
            "extra": "avg tps: 1241.5480886229977, max tps: 1257.2375246049642, count: 55443"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 48.080215981447296,
            "unit": "median tps",
            "extra": "avg tps: 49.45254437253611, max tps: 574.4013445586673, count: 55443"
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
          "id": "eb456f8c97d99e92e2795d88dd2c1082c13c83a6",
          "message": "perf: optimize `Timestamp` and `JsonB` datum decoding (#3171)\n\n## What\n\nOptimize `Timestamp` and `JsonB` to `TantivyValue` datum conversions.\n\nThese two show up quite high in profiles. The `JsonB` conversion in\nparticular has been bad due to how pgrx stupidly (I can say it) handles\n`JsonB` values by converting them to strings and then asking serde to\nparse the strings.\n\n## Why\n\nTrying to make things faster.\n\n## How\n\nFor the `Timestamp` conversion we memoize Postgres' understanding of the\ncurrent EPOCH and do the same math that it does to calculate a time\nvalue.\n\nFor the `JsonB` conversion we implement our own deserializer routine\nusing Postgres' internal `JsonbIteratorInit()` and `JsonbIteratorNext()`\nfunctions, building up a `serde_json::Value` structure as it goes.\n\n\n## Tests\n\nA new `#[pg_test]`-based proptest has been added to test our custom\njsonb deserializer against normal serde.\n\n---------\n\nSigned-off-by: Eric Ridge <eebbrr@gmail.com>\nCo-authored-by: Stu Hood <stuhood@paradedb.com>",
          "timestamp": "2025-09-17T15:26:06-04:00",
          "tree_id": "702cea735a514e9b33d6c1ee785606d39d4f705c",
          "url": "https://github.com/paradedb/paradedb/commit/eb456f8c97d99e92e2795d88dd2c1082c13c83a6"
        },
        "date": 1758138213606,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 758.4304813918445,
            "unit": "median tps",
            "extra": "avg tps: 759.4038673657201, max tps: 865.5226893621841, count: 55401"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3333.826639433374,
            "unit": "median tps",
            "extra": "avg tps: 3306.462192010138, max tps: 3345.8737536344966, count: 55401"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 806.0372034279685,
            "unit": "median tps",
            "extra": "avg tps: 805.2239248761815, max tps: 874.8307584189276, count: 55401"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 666.5472606493335,
            "unit": "median tps",
            "extra": "avg tps: 666.4072115856779, max tps: 718.8923202563335, count: 55401"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1631.0245322160133,
            "unit": "median tps",
            "extra": "avg tps: 1628.7558342006134, max tps: 1648.6771667876808, count: 110802"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1256.3112268914585,
            "unit": "median tps",
            "extra": "avg tps: 1248.2836446381582, max tps: 1261.4730197912677, count: 55401"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 43.18392931423646,
            "unit": "median tps",
            "extra": "avg tps: 51.99580624489311, max tps: 578.8490859394084, count: 55401"
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
          "id": "849076799ca599dfbf0f2415149b12495b24624c",
          "message": "fix: Always use a shared expression context to execute HeapFilters. (#3174)\n\n## What\n\nEvaluating heap filters was leaking a `CreateStandaloneExprContext` per\nexecution, which could eventually lead to OOMs.\n\n## Why\n\n`PgBox::from_pg` does not free the resulting memory: it's expected to be\nfreed by a memory context, since PG created it.\n\n## How\n\nAlways use a global expression context created by\n`ExecAssignExprContext` in `begin_custom_scan`.\n\n## Tests\n\nA repro uses significantly less memory, and [a benchmark query added for\nthis purpose](https://github.com/paradedb/paradedb/pull/3175) is faster.",
          "timestamp": "2025-09-17T16:44:32-07:00",
          "tree_id": "7eef1c518a935389aa23e91c6bc47bbc325b18e6",
          "url": "https://github.com/paradedb/paradedb/commit/849076799ca599dfbf0f2415149b12495b24624c"
        },
        "date": 1758153649642,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 782.8331270006661,
            "unit": "median tps",
            "extra": "avg tps: 785.0464046539265, max tps: 846.4996885171612, count: 55350"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3153.82296191719,
            "unit": "median tps",
            "extra": "avg tps: 3154.7028603788503, max tps: 3435.3267324739795, count: 55350"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 779.5744033012523,
            "unit": "median tps",
            "extra": "avg tps: 779.7447877040515, max tps: 874.8264541150988, count: 55350"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 658.6474512921685,
            "unit": "median tps",
            "extra": "avg tps: 657.6165363319536, max tps: 674.1832212722061, count: 55350"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1654.6580853913092,
            "unit": "median tps",
            "extra": "avg tps: 1652.7227503543259, max tps: 1696.82830037115, count: 110700"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1270.3642290064972,
            "unit": "median tps",
            "extra": "avg tps: 1266.6781435555, max tps: 1285.420893603765, count: 55350"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 52.588837850481276,
            "unit": "median tps",
            "extra": "avg tps: 55.13762270800364, max tps: 850.0791848760713, count: 55350"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "dc0e87d6fdabdb573d6f37a24c6c33befa43e8b0",
          "message": "fix: Always use a shared expression context to execute HeapFilters. (#3176)\n\n## What\n\nEvaluating heap filters was leaking a `CreateStandaloneExprContext` per\nexecution, which could eventually lead to OOMs.\n\n## Why\n\n`PgBox::from_pg` does not free the resulting memory: it's expected to be\nfreed by a memory context, since PG created it.\n\n## How\n\nAlways use a global expression context created by\n`ExecAssignExprContext` in `begin_custom_scan`.\n\n## Tests\n\nA repro uses significantly less memory, and [a benchmark query added for\nthis purpose](https://github.com/paradedb/paradedb/pull/3175) is faster.\n\nCo-authored-by: Stu Hood <stuhood@paradedb.com>",
          "timestamp": "2025-09-17T17:11:43-07:00",
          "tree_id": "0c30f446ad8404b4f66727777f1b6e6a5bc8958e",
          "url": "https://github.com/paradedb/paradedb/commit/dc0e87d6fdabdb573d6f37a24c6c33befa43e8b0"
        },
        "date": 1758155281228,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 764.3194517266609,
            "unit": "median tps",
            "extra": "avg tps: 765.2860178633518, max tps: 840.0559470920718, count: 55474"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3434.2219048977695,
            "unit": "median tps",
            "extra": "avg tps: 3412.6824386520057, max tps: 3463.1779533560602, count: 55474"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 787.381919931993,
            "unit": "median tps",
            "extra": "avg tps: 785.9158818470837, max tps: 842.7388374025005, count: 55474"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 696.665467183167,
            "unit": "median tps",
            "extra": "avg tps: 693.8132037538224, max tps: 723.3679749872507, count: 55474"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1672.8732449061276,
            "unit": "median tps",
            "extra": "avg tps: 1665.6448845282284, max tps: 1686.6330505904657, count: 110948"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1256.2744466656668,
            "unit": "median tps",
            "extra": "avg tps: 1253.3995445746905, max tps: 1267.2410851967704, count: 55474"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 50.90283890269544,
            "unit": "median tps",
            "extra": "avg tps: 64.07852258388115, max tps: 905.3182012413723, count: 55474"
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
          "id": "3bcb1451087be74b7bd73bfc7d6546423046a0ce",
          "message": "fix: write all delete files atomically (#3178)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nIn `ambulkdelete`, write all delete files to the meta list atomically,\ninstead of one at a time.\n\n## Why\n\nReduces the number of writes to the metas list.\n\n## How\n\n## Tests",
          "timestamp": "2025-09-18T16:03:10-04:00",
          "tree_id": "ad9609f0419a34b8f0cf543e911c1dc3c25d4563",
          "url": "https://github.com/paradedb/paradedb/commit/3bcb1451087be74b7bd73bfc7d6546423046a0ce"
        },
        "date": 1758226772335,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 790.0346921444743,
            "unit": "median tps",
            "extra": "avg tps: 789.8232166235093, max tps: 821.9969607585025, count: 54795"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3425.7943386707634,
            "unit": "median tps",
            "extra": "avg tps: 3376.4180590218466, max tps: 3435.393295364149, count: 54795"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 784.1811934571635,
            "unit": "median tps",
            "extra": "avg tps: 783.7379545940861, max tps: 843.5504145193826, count: 54795"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 687.6863014361272,
            "unit": "median tps",
            "extra": "avg tps: 683.635193427134, max tps: 729.2405630437217, count: 54795"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1690.1880871444225,
            "unit": "median tps",
            "extra": "avg tps: 1691.042591886711, max tps: 1738.2538906696257, count: 109590"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1286.279437856849,
            "unit": "median tps",
            "extra": "avg tps: 1274.2293576634672, max tps: 1290.3413705958644, count: 54795"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 90.0378114432283,
            "unit": "median tps",
            "extra": "avg tps: 89.71592216756898, max tps: 600.5122369381081, count: 54795"
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
          "id": "6e11875ba052ccd6937ca0c535b3803309c8b6eb",
          "message": "feat: removed aggregation limitations re mix of aggregate functions and aggregation on group-by column. (#3179)\n\n# Ticket(s) Closed\n\n- Closes #2963\n\n## What\n\nRemoves aggregate limitations that prevented queries where the same\nfield is used in both `GROUP BY` and aggregate functions (e.g., `SELECT\nrating, AVG(rating) FROM table GROUP BY rating`).\n\n## Why\n\nPrevious safety checks blocked these queries due to Tantivy's\n\"incompatible fruit types\" errors, but testing shows the underlying\nissue is resolved. The limitations were overly restrictive and caused\nunnecessary fallbacks to slower PostgreSQL aggregation.\n\n## How\n\n- Removed `has_search_field_conflicts` function and field conflict\nvalidation\n- Eliminated ~35 lines of restrictive code in\n`extract_and_validate_aggregates`\n- Previously blocked queries now use faster `AggregateScan` instead of\n`GroupAggregate`\n\n## Tests\n\n- **`aggregate-groupby-conflict.sql`** - Tests `GROUP BY field` with\naggregates on same field\n- **`test-fruit-types-issue.sql`** - Validates #2963 issue resolution  \n- **`groupby_aggregate.out`** - Updated expectations showing\n`AggregateScan` usage",
          "timestamp": "2025-09-18T16:00:25-07:00",
          "tree_id": "f85924512f419186b824a986dd35eaa96d973884",
          "url": "https://github.com/paradedb/paradedb/commit/6e11875ba052ccd6937ca0c535b3803309c8b6eb"
        },
        "date": 1758237400725,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 802.7734606484255,
            "unit": "median tps",
            "extra": "avg tps: 803.3655022112721, max tps: 818.5395468483744, count: 55250"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3191.376133437114,
            "unit": "median tps",
            "extra": "avg tps: 3183.1591559974463, max tps: 3311.0241365041816, count: 55250"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 786.4617940123576,
            "unit": "median tps",
            "extra": "avg tps: 786.7118494645048, max tps: 821.3398382996932, count: 55250"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 669.3545605969409,
            "unit": "median tps",
            "extra": "avg tps: 671.9255293044082, max tps: 699.3920887738077, count: 55250"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1702.3416768441261,
            "unit": "median tps",
            "extra": "avg tps: 1693.9705667693606, max tps: 1710.63774745472, count: 110500"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1271.6233314327262,
            "unit": "median tps",
            "extra": "avg tps: 1264.2020040645662, max tps: 1278.453568597078, count: 55250"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 171.60040250862255,
            "unit": "median tps",
            "extra": "avg tps: 175.26819908013618, max tps: 509.33301822597275, count: 55250"
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
          "id": "020f92b742187fe6fdc75a19390692e6d2e9a373",
          "message": "feat: Port `ambulkdelete_epoch` to community (#3180)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nAdd `ambulkdelete_epoch` to the index metadata and make sure it's passed\ndown to all scans. In enterprise, this value is used to detect query\nconflicts with concurrent vacuums on the primary.\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-09-18T19:16:09-04:00",
          "tree_id": "3642b293b38caa7676318f888b910c3f934e1976",
          "url": "https://github.com/paradedb/paradedb/commit/020f92b742187fe6fdc75a19390692e6d2e9a373"
        },
        "date": 1758238349880,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 776.8549053543793,
            "unit": "median tps",
            "extra": "avg tps: 777.6845728392888, max tps: 833.909043970863, count: 55475"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3413.6374838682386,
            "unit": "median tps",
            "extra": "avg tps: 3377.8117597550977, max tps: 3425.3861504962297, count: 55475"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 775.7105865689176,
            "unit": "median tps",
            "extra": "avg tps: 775.1627608076411, max tps: 814.6228322886433, count: 55475"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 679.6753090656463,
            "unit": "median tps",
            "extra": "avg tps: 678.6861549303172, max tps: 686.7335848601355, count: 55475"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1624.3318447186853,
            "unit": "median tps",
            "extra": "avg tps: 1661.3366342874656, max tps: 1720.2854725832137, count: 110950"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1251.172361999266,
            "unit": "median tps",
            "extra": "avg tps: 1241.808761230323, max tps: 1255.9682081699518, count: 55475"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 103.8642070632262,
            "unit": "median tps",
            "extra": "avg tps: 111.86770202687053, max tps: 995.5102487780113, count: 55475"
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
          "id": "c0dc0e674f3524c2f65a7b387ccbf594b23e5e0e",
          "message": "chore: Upgrade to `0.18.4` (#3181)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\n---------\n\nSigned-off-by: Ming <ming.ying.nyc@gmail.com>\nCo-authored-by: Stu Hood <stuhood@paradedb.com>",
          "timestamp": "2025-09-18T19:18:34-04:00",
          "tree_id": "b67f22553ed7786ef556afbfad2b7f8ddc6b139e",
          "url": "https://github.com/paradedb/paradedb/commit/c0dc0e674f3524c2f65a7b387ccbf594b23e5e0e"
        },
        "date": 1758238598272,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 798.3847691474879,
            "unit": "median tps",
            "extra": "avg tps: 799.4439476583165, max tps: 855.4495706343068, count: 55035"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3387.591809955574,
            "unit": "median tps",
            "extra": "avg tps: 3365.4654773634875, max tps: 3407.2757013754594, count: 55035"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 793.6849577922433,
            "unit": "median tps",
            "extra": "avg tps: 792.6663079992098, max tps: 838.5327928185177, count: 55035"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 696.4100995836019,
            "unit": "median tps",
            "extra": "avg tps: 693.0434980152555, max tps: 700.1981852964233, count: 55035"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1675.715710468698,
            "unit": "median tps",
            "extra": "avg tps: 1683.5226357884185, max tps: 1727.8730424004123, count: 110070"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1264.215475928443,
            "unit": "median tps",
            "extra": "avg tps: 1258.4940182956034, max tps: 1282.3579715327073, count: 55035"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 88.80980427366298,
            "unit": "median tps",
            "extra": "avg tps: 114.7195531245544, max tps: 608.5230961898544, count: 55035"
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
          "id": "a97c525d0c11314fe01c8bbb250ea5fdf73ec8ce",
          "message": "fix: write all delete files atomically (#3178) (#3182)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nIn `ambulkdelete`, write all delete files to the meta list atomically,\ninstead of one at a time.\n\n## Why\n\nReduces the number of writes to the metas list.\n\n## How\n\n## Tests\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-09-18T21:50:00-04:00",
          "tree_id": "ba5917ed034f24a8e2ad95a64751e5faef3d55d5",
          "url": "https://github.com/paradedb/paradedb/commit/a97c525d0c11314fe01c8bbb250ea5fdf73ec8ce"
        },
        "date": 1758247573738,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 779.2054636236248,
            "unit": "median tps",
            "extra": "avg tps: 779.7256960804295, max tps: 822.7343250770954, count: 55385"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3067.3864117118633,
            "unit": "median tps",
            "extra": "avg tps: 3056.0600034768427, max tps: 3206.101391805608, count: 55385"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 779.8375098678559,
            "unit": "median tps",
            "extra": "avg tps: 779.4002914440852, max tps: 833.1965144486497, count: 55385"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 687.222643487087,
            "unit": "median tps",
            "extra": "avg tps: 683.6172849955462, max tps: 689.6664654417167, count: 55385"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1684.1669613685087,
            "unit": "median tps",
            "extra": "avg tps: 1674.1856800752353, max tps: 1692.2600389219033, count: 110770"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1284.0864713009496,
            "unit": "median tps",
            "extra": "avg tps: 1272.4751954370017, max tps: 1287.9313004633939, count: 55385"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 79.70887895543777,
            "unit": "median tps",
            "extra": "avg tps: 93.2037990847549, max tps: 623.1158534381664, count: 55385"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "e15e51abfc4b7834faea068d861d91d5d873580f",
          "message": "chore: Upgrade to `0.18.4` (#3184)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\nSigned-off-by: Ming <ming.ying.nyc@gmail.com>\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>\nCo-authored-by: Stu Hood <stuhood@paradedb.com>",
          "timestamp": "2025-09-18T21:52:13-04:00",
          "tree_id": "3d203e3468a4e7504d03af9c39ac9a0869033086",
          "url": "https://github.com/paradedb/paradedb/commit/e15e51abfc4b7834faea068d861d91d5d873580f"
        },
        "date": 1758247702218,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 776.7289808018954,
            "unit": "median tps",
            "extra": "avg tps: 778.4893464750203, max tps: 791.6224382762696, count: 55346"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3391.3802632044753,
            "unit": "median tps",
            "extra": "avg tps: 3364.811216275684, max tps: 3411.4552590563026, count: 55346"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 787.6172570186282,
            "unit": "median tps",
            "extra": "avg tps: 787.4488445637724, max tps: 857.9523171298608, count: 55346"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 699.2308329629967,
            "unit": "median tps",
            "extra": "avg tps: 698.0828956299048, max tps: 708.580502136301, count: 55346"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1709.040672973903,
            "unit": "median tps",
            "extra": "avg tps: 1701.2251138030674, max tps: 1714.6267342070746, count: 110692"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1266.4571817106764,
            "unit": "median tps",
            "extra": "avg tps: 1259.0338068657702, max tps: 1270.3048441348938, count: 55346"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 121.33147888521476,
            "unit": "median tps",
            "extra": "avg tps: 132.92409555772113, max tps: 553.9138165492831, count: 55346"
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
          "id": "1046018b2db9614ef172bd802c98a3987da7513e",
          "message": "chore: small diff ported from enterprise for query cancel fix (#3186)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nSome small changes in enterprise that should be in community\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-09-18T21:53:42-04:00",
          "tree_id": "85ed1f4eb7261157deabdfba479dc61164775f99",
          "url": "https://github.com/paradedb/paradedb/commit/1046018b2db9614ef172bd802c98a3987da7513e"
        },
        "date": 1758247793131,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 800.3512832192049,
            "unit": "median tps",
            "extra": "avg tps: 800.7718981265153, max tps: 856.8137961353336, count: 55238"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3400.4643911905173,
            "unit": "median tps",
            "extra": "avg tps: 3347.6380926494553, max tps: 3438.8332375981495, count: 55238"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 808.1585682212983,
            "unit": "median tps",
            "extra": "avg tps: 808.6991952724444, max tps: 879.4881615822933, count: 55238"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 702.6868382626708,
            "unit": "median tps",
            "extra": "avg tps: 702.2997780016938, max tps: 709.6782147676341, count: 55238"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1692.6910532325087,
            "unit": "median tps",
            "extra": "avg tps: 1673.4655579424975, max tps: 1700.8435996115554, count: 110476"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1302.9579098279034,
            "unit": "median tps",
            "extra": "avg tps: 1287.021721271236, max tps: 1307.9140953813232, count: 55238"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 100.39196001576467,
            "unit": "median tps",
            "extra": "avg tps: 122.96785375211121, max tps: 998.453395690076, count: 55238"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "f052aabf25719cee68a756a379c6b66e39452759",
          "message": "feat: Port `ambulkdelete_epoch` to community (#3183)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nAdd `ambulkdelete_epoch` to the index metadata and make sure it's passed\ndown to all scans. In enterprise, this value is used to detect query\nconflicts with concurrent vacuums on the primary.\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-09-18T22:01:15-04:00",
          "tree_id": "48ffae94b2f43d5c2d62b5adb846d1dcc2992aee",
          "url": "https://github.com/paradedb/paradedb/commit/f052aabf25719cee68a756a379c6b66e39452759"
        },
        "date": 1758248247381,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 815.3357041183292,
            "unit": "median tps",
            "extra": "avg tps: 816.2183930023697, max tps: 862.4304762231966, count: 55384"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3396.358984492234,
            "unit": "median tps",
            "extra": "avg tps: 3368.0406897919647, max tps: 3405.4603913834394, count: 55384"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 799.2839330559461,
            "unit": "median tps",
            "extra": "avg tps: 798.7334909684644, max tps: 861.4286909109912, count: 55384"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 704.3342398457362,
            "unit": "median tps",
            "extra": "avg tps: 702.8390232117622, max tps: 720.0245706046976, count: 55384"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1691.355401322739,
            "unit": "median tps",
            "extra": "avg tps: 1701.811924234684, max tps: 1740.392175938719, count: 110768"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1308.2048622341283,
            "unit": "median tps",
            "extra": "avg tps: 1295.6624589847945, max tps: 1311.2428450721557, count: 55384"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 86.28765491972705,
            "unit": "median tps",
            "extra": "avg tps: 102.22983228152626, max tps: 904.2392544728195, count: 55384"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "153f632ba06057571459a4b6e8767c135baf438c",
          "message": "chore: small diff ported from enterprise for query cancel fix (#3187)",
          "timestamp": "2025-09-18T22:31:35-04:00",
          "tree_id": "2c3b3f692c24ba8540a69da9d41f4d3a24d4ae6f",
          "url": "https://github.com/paradedb/paradedb/commit/153f632ba06057571459a4b6e8767c135baf438c"
        },
        "date": 1758250074803,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 802.4982497720831,
            "unit": "median tps",
            "extra": "avg tps: 803.4183899427173, max tps: 836.4038761749049, count: 55203"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3410.8944969897802,
            "unit": "median tps",
            "extra": "avg tps: 3378.737221462811, max tps: 3443.7150599682896, count: 55203"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 792.1449102181161,
            "unit": "median tps",
            "extra": "avg tps: 793.0931601094604, max tps: 816.9164156508224, count: 55203"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 699.5515088940145,
            "unit": "median tps",
            "extra": "avg tps: 695.7907598378555, max tps: 705.000103986653, count: 55203"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1690.7096992024465,
            "unit": "median tps",
            "extra": "avg tps: 1705.055014963217, max tps: 1757.359630306017, count: 110406"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1306.9427868671626,
            "unit": "median tps",
            "extra": "avg tps: 1297.9263790693149, max tps: 1309.4495696945953, count: 55203"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 98.86890120380971,
            "unit": "median tps",
            "extra": "avg tps: 124.46459447501944, max tps: 567.348525035672, count: 55203"
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
          "id": "8101a67174703310a6a1655496fd5296e869901d",
          "message": "fix: Clone an Arc rather than a OnceLock. (#3185)\n\n## What\n\nInvert our use of `OnceLock` to ensure that we clone an\n`Arc<OnceLock<T>>` rather than a `OnceLock<Arc<T>>`.\n\n## Why\n\n`OnceLock` implements `Clone` by cloning its contents to create a\nseparate disconnected copy. If what is desired is \"exactly once\nbehavior\", then cloning the `OnceLock` before it has been computed the\nfirst time will defeat that.\n\nThis change has no impact on benchmarks in this case, but\n`Arc<OnceLock<T>>` matches the intent of this code, and sets a better\nexample for future us.\n\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>",
          "timestamp": "2025-09-19T15:01:21-07:00",
          "tree_id": "de6adf9a09b874a0e133e9cbfeca50d417e6c5bf",
          "url": "https://github.com/paradedb/paradedb/commit/8101a67174703310a6a1655496fd5296e869901d"
        },
        "date": 1758320251655,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 825.082384288462,
            "unit": "median tps",
            "extra": "avg tps: 824.5214796917282, max tps: 849.5568273806548, count: 55367"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3315.1873603764993,
            "unit": "median tps",
            "extra": "avg tps: 3301.333906848278, max tps: 3331.471960170429, count: 55367"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 783.2454249990076,
            "unit": "median tps",
            "extra": "avg tps: 783.4638643462426, max tps: 844.5644356551187, count: 55367"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 726.8043882502141,
            "unit": "median tps",
            "extra": "avg tps: 724.607508081995, max tps: 729.7159892755525, count: 55367"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1699.3295659608052,
            "unit": "median tps",
            "extra": "avg tps: 1693.5137491306546, max tps: 1709.5720323504884, count: 110734"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1287.8003246961166,
            "unit": "median tps",
            "extra": "avg tps: 1278.7815625123696, max tps: 1292.1020676317435, count: 55367"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 62.41926403050997,
            "unit": "median tps",
            "extra": "avg tps: 70.77719342002541, max tps: 1023.7573121866022, count: 55367"
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
          "id": "3163a5f3e48d3027585287ce8a63074f70ba1836",
          "message": "perf: Configurable Top N requeries more granularly (#3190)",
          "timestamp": "2025-09-19T21:06:04-04:00",
          "tree_id": "8c74bdf97c37281e4641be0e94b4d464daa5a3ea",
          "url": "https://github.com/paradedb/paradedb/commit/3163a5f3e48d3027585287ce8a63074f70ba1836"
        },
        "date": 1758331336573,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 788.1112605943559,
            "unit": "median tps",
            "extra": "avg tps: 787.6764915210454, max tps: 835.5819293611326, count: 55396"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3325.649413188449,
            "unit": "median tps",
            "extra": "avg tps: 3318.981514616384, max tps: 3353.0279833780096, count: 55396"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 819.8204178129251,
            "unit": "median tps",
            "extra": "avg tps: 819.0443682977892, max tps: 831.3916839748156, count: 55396"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 689.1183717308472,
            "unit": "median tps",
            "extra": "avg tps: 684.2297800457904, max tps: 742.6002668162758, count: 55396"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1696.1259985880454,
            "unit": "median tps",
            "extra": "avg tps: 1687.247360173578, max tps: 1704.199557042554, count: 110792"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1282.458972456947,
            "unit": "median tps",
            "extra": "avg tps: 1273.946511602027, max tps: 1295.0718047857608, count: 55396"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 93.87828247959055,
            "unit": "median tps",
            "extra": "avg tps: 125.25395871470296, max tps: 931.4733673134817, count: 55396"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "f573a31e6704d95d0a62271a23ba47658a1dae06",
          "message": "perf: Configurable Top N requeries more granularly (#3194)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nAllow the retry scale factor and max chunk size to be tuned, which is\nuseful for reducing Top N requeries.\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-09-20T09:26:21-04:00",
          "tree_id": "d4ee2092267660be53cb68f8b760756a5a07ab69",
          "url": "https://github.com/paradedb/paradedb/commit/f573a31e6704d95d0a62271a23ba47658a1dae06"
        },
        "date": 1758375747572,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 798.8194754321008,
            "unit": "median tps",
            "extra": "avg tps: 798.8034091995769, max tps: 811.9189704867455, count: 55345"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3283.1009843793245,
            "unit": "median tps",
            "extra": "avg tps: 3271.0360894293135, max tps: 3369.291251896385, count: 55345"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 792.222842022531,
            "unit": "median tps",
            "extra": "avg tps: 791.9111880601648, max tps: 823.1966340411615, count: 55345"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 698.0193328893423,
            "unit": "median tps",
            "extra": "avg tps: 696.1056871690372, max tps: 707.6417626338192, count: 55345"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1708.984854100911,
            "unit": "median tps",
            "extra": "avg tps: 1700.6722822860806, max tps: 1717.1120344316605, count: 110690"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1277.7829776831143,
            "unit": "median tps",
            "extra": "avg tps: 1271.261040305462, max tps: 1281.5529408992031, count: 55345"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 77.43868711357521,
            "unit": "median tps",
            "extra": "avg tps: 95.53610016360703, max tps: 554.0678832889248, count: 55345"
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
          "id": "8b5e7226ce38eaa98ce91906a3f8eb0b72906e66",
          "message": "fix: update tantivy dependency revision (#3192)\n\n## What\n\nUpdate tantivy to\n\nhttps://github.com/paradedb/tantivy/commit/7c6c6fc6ac977382b19ae7fb9fd5b0c53b8f1b58\nwhich fixes a bug that disallowed a segment, during indexing, to real\nthe real memory limit of 4GB.\n\n## Why\n\nWe had a bug in our tantivy fork that wouldn't allow a segment, during\nindexing, to cross over 2GB to reach the actual limit of 4GB.\n\n## How\n\n## Tests",
          "timestamp": "2025-09-22T10:19:44-04:00",
          "tree_id": "89f8538f4216f95c908cb451c1405afaa80946e6",
          "url": "https://github.com/paradedb/paradedb/commit/8b5e7226ce38eaa98ce91906a3f8eb0b72906e66"
        },
        "date": 1758551858070,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 816.664801274319,
            "unit": "median tps",
            "extra": "avg tps: 815.5592248199972, max tps: 861.7473485600898, count: 55420"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3367.2915305819033,
            "unit": "median tps",
            "extra": "avg tps: 3318.0045290553358, max tps: 3502.5852380674173, count: 55420"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 793.8004480754785,
            "unit": "median tps",
            "extra": "avg tps: 793.1497415346863, max tps: 828.194303172525, count: 55420"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 722.8939286736814,
            "unit": "median tps",
            "extra": "avg tps: 720.1893335274991, max tps: 726.2359077288027, count: 55420"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1728.635486736816,
            "unit": "median tps",
            "extra": "avg tps: 1709.6654816614914, max tps: 1735.003878950482, count: 110840"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1268.7417297059599,
            "unit": "median tps",
            "extra": "avg tps: 1253.1409904100776, max tps: 1272.0086168364164, count: 55420"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 142.4771773930733,
            "unit": "median tps",
            "extra": "avg tps: 153.77026568250446, max tps: 533.4852699382118, count: 55420"
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
          "id": "7efcca559340008f06df6d1861f1f0301970a0dc",
          "message": "fix: use correct xids when returning to the fsm (#3191)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nThere's a few places where we need to use a future xid when returning\nblocks to the FSM.\n\nSpecically blocks from the segment meta entries list when it's garbage\ncollected.\n\n## Why\n\nTo address some community reports of what appear to be corrupt index\npages.\n\n## How\n\n## Tests",
          "timestamp": "2025-09-22T10:21:41-04:00",
          "tree_id": "a4cdecfcfa42fbe38c487b7c4f6c5cc31eef4f46",
          "url": "https://github.com/paradedb/paradedb/commit/7efcca559340008f06df6d1861f1f0301970a0dc"
        },
        "date": 1758551971039,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 815.7904822117993,
            "unit": "median tps",
            "extra": "avg tps: 815.9687083653691, max tps: 829.321793447091, count: 54856"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3178.4602848093236,
            "unit": "median tps",
            "extra": "avg tps: 3179.834888529873, max tps: 3384.6448460988627, count: 54856"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 812.385548734505,
            "unit": "median tps",
            "extra": "avg tps: 812.1839345726411, max tps: 836.2383137368131, count: 54856"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 699.975276851706,
            "unit": "median tps",
            "extra": "avg tps: 699.5701889369637, max tps: 708.8559561213827, count: 54856"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1670.2090775776728,
            "unit": "median tps",
            "extra": "avg tps: 1665.6091545157433, max tps: 1683.7074434155277, count: 109712"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1288.3336493274878,
            "unit": "median tps",
            "extra": "avg tps: 1281.8647350436513, max tps: 1290.8482430262884, count: 54856"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 86.65795389111214,
            "unit": "median tps",
            "extra": "avg tps: 92.74807906007315, max tps: 861.2418763360015, count: 54856"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "5f4016019b33da6e3e77ef5d3f8663670394cff1",
          "message": "fix: update tantivy dependency revision (#3198)\n\n## What\n\nUpdate tantivy to\n\nhttps://github.com/paradedb/tantivy/commit/7c6c6fc6ac977382b19ae7fb9fd5b0c53b8f1b58\nwhich fixes a bug that disallowed a segment, during indexing, to real\nthe real memory limit of 4GB.\n\n## Why\n\nWe had a bug in our tantivy fork that wouldn't allow a segment, during\nindexing, to cross over 2GB to reach the actual limit of 4GB.\n\n## How\n\n## Tests\n\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>",
          "timestamp": "2025-09-22T10:36:38-04:00",
          "tree_id": "45c5f689aa0bacc2a9e85d97ac3ccc0047a14590",
          "url": "https://github.com/paradedb/paradedb/commit/5f4016019b33da6e3e77ef5d3f8663670394cff1"
        },
        "date": 1758552862790,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 821.3446740865195,
            "unit": "median tps",
            "extra": "avg tps: 821.2206414276168, max tps: 882.3903659683428, count: 55400"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3329.4162245282814,
            "unit": "median tps",
            "extra": "avg tps: 3299.455599444926, max tps: 3352.5310170494154, count: 55400"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 790.0335111244311,
            "unit": "median tps",
            "extra": "avg tps: 790.846086418895, max tps: 887.496063434234, count: 55400"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 695.7019768239911,
            "unit": "median tps",
            "extra": "avg tps: 694.8162317582951, max tps: 700.3880137794753, count: 55400"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1702.0741577186116,
            "unit": "median tps",
            "extra": "avg tps: 1694.4830353746784, max tps: 1717.302386559184, count: 110800"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1283.1564489202876,
            "unit": "median tps",
            "extra": "avg tps: 1272.9003077274515, max tps: 1291.6375946934722, count: 55400"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 101.38681651419634,
            "unit": "median tps",
            "extra": "avg tps: 106.47890227708588, max tps: 944.0285474232741, count: 55400"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "414e16a1fd61ad461377aedc953534abc28d76a4",
          "message": "fix: use correct xids when returning to the fsm (#3199)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nThere's a few places where we need to use a future xid when returning\nblocks to the FSM.\n\nSpecically blocks from the segment meta entries list when it's garbage\ncollected.\n\n## Why\n\nTo address some community reports of what appear to be corrupt index\npages.\n\n## How\n\n## Tests\n\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>",
          "timestamp": "2025-09-22T10:38:03-04:00",
          "tree_id": "7f77ddfdfc23d53ffdb8a2b66caf5806c3a67939",
          "url": "https://github.com/paradedb/paradedb/commit/414e16a1fd61ad461377aedc953534abc28d76a4"
        },
        "date": 1758552981025,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 794.9283296336216,
            "unit": "median tps",
            "extra": "avg tps: 794.9804261027958, max tps: 872.8599623781953, count: 55366"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3174.6243349083334,
            "unit": "median tps",
            "extra": "avg tps: 3136.136018203346, max tps: 3199.311437789557, count: 55366"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 793.1414756559352,
            "unit": "median tps",
            "extra": "avg tps: 794.1981017730308, max tps: 808.4424925751102, count: 55366"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 666.4612547606127,
            "unit": "median tps",
            "extra": "avg tps: 668.4889081235177, max tps: 743.3521845194746, count: 55366"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 1688.3819772373242,
            "unit": "median tps",
            "extra": "avg tps: 1675.9354303567497, max tps: 1716.25696279274, count: 110732"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1283.7978281809826,
            "unit": "median tps",
            "extra": "avg tps: 1268.5826316736402, max tps: 1291.9551867160733, count: 55366"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 86.75451964595732,
            "unit": "median tps",
            "extra": "avg tps: 97.84543779699452, max tps: 970.6099312808169, count: 55366"
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
          "id": "4051856e9bea6cc1c5c5f61beb626af2f25b35c4",
          "message": "chore: Upgrade to `0.18.2` (#3144)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-09-10T17:33:06-04:00",
          "tree_id": "783d32dc8a220a1e0585e30bc3573d8af9a1767e",
          "url": "https://github.com/paradedb/paradedb/commit/4051856e9bea6cc1c5c5f61beb626af2f25b35c4"
        },
        "date": 1757540960948,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.900172737712183, max cpu: 14.693878, count: 55255"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 151.375,
            "unit": "median mem",
            "extra": "avg mem: 139.97042193579767, max mem: 151.75, count: 55255"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.5570928978230185, max cpu: 9.257474, count: 55255"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 147.3671875,
            "unit": "median mem",
            "extra": "avg mem: 134.97081542394355, max mem: 147.3671875, count: 55255"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.881967141753466, max cpu: 14.414414, count: 55255"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 154.32421875,
            "unit": "median mem",
            "extra": "avg mem: 142.81601092378517, max mem: 154.32421875, count: 55255"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.581546622047043, max cpu: 4.729064, count: 55255"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 155.65234375,
            "unit": "median mem",
            "extra": "avg mem: 143.5662045572799, max mem: 155.65234375, count: 55255"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 5.231736210522871, max cpu: 14.693878, count: 110510"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 162.16015625,
            "unit": "median mem",
            "extra": "avg mem: 149.9586733171772, max mem: 164.41015625, count: 110510"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 36935,
            "unit": "median block_count",
            "extra": "avg block_count: 37394.812686634694, max block_count: 73875.0, count: 55255"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 32,
            "unit": "median segment_count",
            "extra": "avg segment_count: 32.15587729617229, max segment_count: 78.0, count: 55255"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.8066686913253704, max cpu: 9.795918, count: 55255"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 163.0703125,
            "unit": "median mem",
            "extra": "avg mem: 149.4130958878156, max mem: 165.328125, count: 55255"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 9.213051,
            "unit": "median cpu",
            "extra": "avg cpu: 7.838733512649481, max cpu: 18.677044, count: 55255"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 154.90234375,
            "unit": "median mem",
            "extra": "avg mem: 137.74992866878563, max mem: 157.26171875, count: 55255"
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
          "id": "da997100cd0e2873fa8692ec6c2382761719ce58",
          "message": "chore: Upgrade to `0.18.2` (#3144) (#3145)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-09-10T17:44:33-04:00",
          "tree_id": "d0d9fd4cb9ebc554c1e7f3e029694e863f4247c9",
          "url": "https://github.com/paradedb/paradedb/commit/da997100cd0e2873fa8692ec6c2382761719ce58"
        },
        "date": 1757541740068,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.914266149712122, max cpu: 14.428859, count: 55268"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 152.6171875,
            "unit": "median mem",
            "extra": "avg mem: 140.83578335056, max mem: 152.9921875, count: 55268"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.6272888117028845, max cpu: 9.458128, count: 55268"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 147.3203125,
            "unit": "median mem",
            "extra": "avg mem: 134.63680130229065, max mem: 147.3203125, count: 55268"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.877559276304345, max cpu: 14.4, count: 55268"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 154.265625,
            "unit": "median mem",
            "extra": "avg mem: 142.27886100173248, max mem: 154.265625, count: 55268"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.510412316294366, max cpu: 4.7524753, count: 55268"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 155.29296875,
            "unit": "median mem",
            "extra": "avg mem: 143.01893599709598, max mem: 155.29296875, count: 55268"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 5.014117287640229, max cpu: 13.953489, count: 110536"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 165.7890625,
            "unit": "median mem",
            "extra": "avg mem: 153.44570098344656, max mem: 171.12890625, count: 110536"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 38575,
            "unit": "median block_count",
            "extra": "avg block_count: 39284.61511181878, max block_count: 78510.0, count: 55268"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 33,
            "unit": "median segment_count",
            "extra": "avg segment_count: 32.446877035535934, max segment_count: 76.0, count: 55268"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.59984513809948, max cpu: 9.421001, count: 55268"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 166.3671875,
            "unit": "median mem",
            "extra": "avg mem: 151.90902108532515, max mem: 166.7421875, count: 55268"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 9.239654,
            "unit": "median cpu",
            "extra": "avg cpu: 8.387742668045675, max cpu: 14.215202, count: 55268"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 153.2890625,
            "unit": "median mem",
            "extra": "avg mem: 136.76730042983735, max mem: 156.6171875, count: 55268"
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
          "id": "4051856e9bea6cc1c5c5f61beb626af2f25b35c4",
          "message": "chore: Upgrade to `0.18.2` (#3144)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-09-10T17:33:06-04:00",
          "tree_id": "783d32dc8a220a1e0585e30bc3573d8af9a1767e",
          "url": "https://github.com/paradedb/paradedb/commit/4051856e9bea6cc1c5c5f61beb626af2f25b35c4"
        },
        "date": 1757625497266,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.901315629626237, max cpu: 14.754097, count: 55201"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 154.77734375,
            "unit": "median mem",
            "extra": "avg mem: 141.84512558366242, max mem: 155.15625, count: 55201"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.569042508887207, max cpu: 9.239654, count: 55201"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 147.28515625,
            "unit": "median mem",
            "extra": "avg mem: 134.1541435645414, max mem: 148.0546875, count: 55201"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.903448677762881, max cpu: 19.692308, count: 55201"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 154.09765625,
            "unit": "median mem",
            "extra": "avg mem: 141.37036565800437, max mem: 154.09765625, count: 55201"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.196637150584684, max cpu: 4.743083, count: 55201"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 155.77734375,
            "unit": "median mem",
            "extra": "avg mem: 143.0163385145876, max mem: 156.90625, count: 55201"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.97725615674421, max cpu: 14.754097, count: 110402"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 162.57421875,
            "unit": "median mem",
            "extra": "avg mem: 150.09162799643576, max mem: 166.78125, count: 110402"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 36269,
            "unit": "median block_count",
            "extra": "avg block_count: 37050.98192061738, max block_count: 74221.0, count: 55201"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 32,
            "unit": "median segment_count",
            "extra": "avg segment_count: 32.1281317367439, max segment_count: 75.0, count: 55201"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.762387792928991, max cpu: 9.486166, count: 55201"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 170,
            "unit": "median mem",
            "extra": "avg mem: 154.58763944776817, max mem: 170.75, count: 55201"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 9.204219,
            "unit": "median cpu",
            "extra": "avg cpu: 8.108900586254368, max cpu: 18.953604, count: 55201"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 154.37890625,
            "unit": "median mem",
            "extra": "avg mem: 136.81317834482167, max mem: 158.5, count: 55201"
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
          "id": "1cfaa7b311ca8b7ee91491411c8dfecd2ce5619c",
          "message": "fix: `GROUP BY` doesn't panic when Postgres eliminates group pathkeys (#3152)\n\n# Ticket(s) Closed\n\n- Closes #3050 \n\n## What\n\nIt's possible for Postgres to eliminate group pathkeys if it realizes\nthat one of the pathkeys is unique, making the other ones unnecessary.\n\nWe need to handle this case/not panic.\n\n## Why\n\nSee issue.\n\n## How\n\nInject the dropped group pathkeys back into our list of grouping\ncolumns.\n\n## Tests\n\nAdded regression test",
          "timestamp": "2025-09-14T17:56:19-04:00",
          "tree_id": "a41824569d62cfd5dbe40884e6ead540d3b1bd88",
          "url": "https://github.com/paradedb/paradedb/commit/1cfaa7b311ca8b7ee91491411c8dfecd2ce5619c"
        },
        "date": 1757887954363,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.874431188802093, max cpu: 14.328358, count: 55135"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 153.08203125,
            "unit": "median mem",
            "extra": "avg mem: 141.21766544617756, max mem: 153.08203125, count: 55135"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.681219017847826, max cpu: 9.467456, count: 55135"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 146.8359375,
            "unit": "median mem",
            "extra": "avg mem: 134.72240714552916, max mem: 147.59375, count: 55135"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.857821219695124, max cpu: 13.953489, count: 55135"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 153.53515625,
            "unit": "median mem",
            "extra": "avg mem: 141.79958680964904, max mem: 153.53515625, count: 55135"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.619827,
            "unit": "median cpu",
            "extra": "avg cpu: 4.545672089571022, max cpu: 4.7244096, count: 55135"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 155.30078125,
            "unit": "median mem",
            "extra": "avg mem: 143.32186713578037, max mem: 155.30078125, count: 55135"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 5.119471891317029, max cpu: 14.428859, count: 110270"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 162.609375,
            "unit": "median mem",
            "extra": "avg mem: 151.26742721700145, max mem: 167.3515625, count: 110270"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 36328,
            "unit": "median block_count",
            "extra": "avg block_count: 37302.09250022672, max block_count: 74557.0, count: 55135"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 32,
            "unit": "median segment_count",
            "extra": "avg segment_count: 31.917638523623832, max segment_count: 76.0, count: 55135"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.746704729402473, max cpu: 9.486166, count: 55135"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 164.8046875,
            "unit": "median mem",
            "extra": "avg mem: 152.2208185023125, max mem: 168.53125, count: 55135"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 6.976928693199615, max cpu: 23.121387, count: 55135"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 155.1640625,
            "unit": "median mem",
            "extra": "avg mem: 139.3788320712796, max mem: 158.01171875, count: 55135"
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
          "id": "a521487756693e82c46bfe2f1a2f2fd3aded0136",
          "message": "fix: fixed `rt_fetch out-of-bounds` error (#3141)\n\n# Ticket(s) Closed\n\n- Closes #3135\n\n## What\n\nFixed `rt_fetch used out-of-bounds` and `Cannot open relation with\noid=0` errors that occurred in complex SQL queries with nested `OR\nEXISTS` clauses, multiple `JOIN`s.\n\n## Why\n\nThe issue occurred when PostgreSQL's query planner generated `Var` nodes\nreferencing Range Table Entries (RTEs) that were valid in outer planning\ncontexts but didn't exist in inner execution contexts. This happened\nspecifically with:\n- `OR EXISTS` subqueries (not `AND EXISTS`)  \n- Multiple `JOIN`s within the `EXISTS` clause\n- ParadeDB functions applied to joined tables\n\nWhen ParadeDB's custom scan tried to access these out-of-bounds RTEs\nusing `rt_fetch`, it caused crashes.\n\n## How\n\nImplemented bounds checking across the codebase:\n\n1. **Early detection**: Added bounds checking in `find_var_relation()`\nto detect invalid `varno` values and return `pg_sys::InvalidOid`. This\nwas the main fix for the issue.\n2. **Graceful handling**: Modified all functions that receive relation\nOIDs to check for `InvalidOid` before attempting to open relations\n3. **Safe fallbacks**: Updated query optimization logic to skip\noptimizations when relation information is unavailable rather than\ncrashing\n\n## Tests\n\nAdded regression test `or_exists_join_bug.sql` covering:\n- Simple queries (baseline functionality)\n- `AND EXISTS` with multiple `JOIN`s (should work)  \n- `OR EXISTS` with multiple `JOIN`s (the problematic case, now fixed)\n- Various edge cases and workarounds\n- Minimal reproduction cases\n\n---------\n\nSigned-off-by: Moe <mdashti@gmail.com>",
          "timestamp": "2025-09-15T02:47:52-07:00",
          "tree_id": "4a0b5db116e0263111295cc53d05810e093ce68c",
          "url": "https://github.com/paradedb/paradedb/commit/a521487756693e82c46bfe2f1a2f2fd3aded0136"
        },
        "date": 1757930639512,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.9293424784242985, max cpu: 14.34263, count: 55076"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 153.30078125,
            "unit": "median mem",
            "extra": "avg mem: 142.26910492149483, max mem: 154.05078125, count: 55076"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.598644897786393, max cpu: 9.266409, count: 55076"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 147.85546875,
            "unit": "median mem",
            "extra": "avg mem: 135.47450405178753, max mem: 147.85546875, count: 55076"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.859046745113744, max cpu: 14.356929, count: 55076"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 154.3984375,
            "unit": "median mem",
            "extra": "avg mem: 142.90890215906202, max mem: 154.3984375, count: 55076"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.610951,
            "unit": "median cpu",
            "extra": "avg cpu: 4.487845991617897, max cpu: 4.729064, count: 55076"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 155.2578125,
            "unit": "median mem",
            "extra": "avg mem: 143.71444515306123, max mem: 155.2578125, count: 55076"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 5.086688657340288, max cpu: 14.34263, count: 110152"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 161.5,
            "unit": "median mem",
            "extra": "avg mem: 153.33502367041677, max mem: 171.40625, count: 110152"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 37952,
            "unit": "median block_count",
            "extra": "avg block_count: 38680.033626261895, max block_count: 77698.0, count: 55076"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 32,
            "unit": "median segment_count",
            "extra": "avg segment_count: 32.22240177209674, max segment_count: 77.0, count: 55076"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.879687660381722, max cpu: 9.561753, count: 55076"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 164.171875,
            "unit": "median mem",
            "extra": "avg mem: 150.427877855713, max mem: 166.06640625, count: 55076"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 6.806998711418643, max cpu: 18.640776, count: 55076"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 153.45703125,
            "unit": "median mem",
            "extra": "avg mem: 138.37679794170782, max mem: 158.1015625, count: 55076"
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
          "id": "b37fc5af676e3438c051381414d81996ed0fb8f6",
          "message": "feat: push down `group by ... order by ... limit` (#3134)\n\n# Ticket(s) Closed\n\n- Closes #3131 \n- Opens #3156 #3155 \n\n## What\n\nPushes down `group by ... order by ... limit` to Tantivy\n\n## Why\n\nBy pushing down the sort/limit to Tantivy, we can significantly speed up\n`group by` queries over high cardinality columns.\n\n## How\n\n- Before we were hard-coding a bucket size and sorting the results\nourselves, now the bucket size is set to the limit and we push the sort\ndown to the Tantivy term agg\n\n## Tests",
          "timestamp": "2025-09-15T15:51:50-04:00",
          "tree_id": "e58df02d60abc13101aaae8ef6333a9afafbcd78",
          "url": "https://github.com/paradedb/paradedb/commit/b37fc5af676e3438c051381414d81996ed0fb8f6"
        },
        "date": 1757966877268,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.870807724459763, max cpu: 14.443329, count: 55041"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 154.2890625,
            "unit": "median mem",
            "extra": "avg mem: 142.88109403671808, max mem: 155.0625, count: 55041"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.602799782968384, max cpu: 9.257474, count: 55041"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 147.8359375,
            "unit": "median mem",
            "extra": "avg mem: 135.0247536353582, max mem: 147.8359375, count: 55041"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.887821125591669, max cpu: 14.385615, count: 55041"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 155.1796875,
            "unit": "median mem",
            "extra": "avg mem: 143.34246466554478, max mem: 155.1796875, count: 55041"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.577540319682231, max cpu: 9.230769, count: 55041"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 154.35546875,
            "unit": "median mem",
            "extra": "avg mem: 142.3908217993178, max mem: 154.35546875, count: 55041"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.830039931032033, max cpu: 14.385615, count: 110082"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 165.83984375,
            "unit": "median mem",
            "extra": "avg mem: 154.97173048415044, max mem: 173.54296875, count: 110082"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 37576,
            "unit": "median block_count",
            "extra": "avg block_count: 37744.07265492996, max block_count: 74439.0, count: 55041"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 32,
            "unit": "median segment_count",
            "extra": "avg segment_count: 32.13846042041387, max segment_count: 76.0, count: 55041"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.9658150054880075, max cpu: 9.648242, count: 55041"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 162.8359375,
            "unit": "median mem",
            "extra": "avg mem: 150.03101572861593, max mem: 165.09765625, count: 55041"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 7.076988878597194, max cpu: 14.10382, count: 55041"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 152.734375,
            "unit": "median mem",
            "extra": "avg mem: 136.98991866574463, max mem: 155.8203125, count: 55041"
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
          "id": "8211eef7a0dd34237afebfa91364fb66c65a4906",
          "message": "perf: remove `ExactBuffer` in favor of a regular rust `BufWriter` (#3158)\n\n# Ticket(s) Closed\n\n- Closes #2981  (/cc @yjhjstz)\n\nWhile #2981 wasn't the impetus for this, it addresses the complaint made\nthere just the same.\n\n## What\n\nIn profiling, our `ExactBuffer` was a large percentage of certain\nprofiles. This replaces it, and its complexity, with a standard Rust\n`BufWriter`.\n\n## Why\n\nImproves performance of (at least) our `wide-table.toml` test's \"Single\nUpdate\" job by quite a bit.\n\n<img width=\"720\" height=\"141\" alt=\"screenshot_2025-09-15_at_3 28\n33___pm_720\"\nsrc=\"https://github.com/user-attachments/assets/a373a7ae-df38-4691-980a-d6843f073d26\"\n/>\n\n\n## How\n\n## Tests\n\nExisting tests pass",
          "timestamp": "2025-09-15T15:55:52-04:00",
          "tree_id": "4ddf140542c5525034023441aadac4b634c90fc6",
          "url": "https://github.com/paradedb/paradedb/commit/8211eef7a0dd34237afebfa91364fb66c65a4906"
        },
        "date": 1757967120948,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.889788496545054, max cpu: 14.229248, count: 55285"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 153.859375,
            "unit": "median mem",
            "extra": "avg mem: 138.85364722291308, max mem: 153.859375, count: 55285"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.6376622715630305, max cpu: 9.338522, count: 55285"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 148.0859375,
            "unit": "median mem",
            "extra": "avg mem: 131.84299404506194, max mem: 148.0859375, count: 55285"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.848878184815113, max cpu: 14.243324, count: 55285"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 153.4453125,
            "unit": "median mem",
            "extra": "avg mem: 138.65079516934972, max mem: 153.4453125, count: 55285"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.51973625154264, max cpu: 4.738401, count: 55285"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 156.171875,
            "unit": "median mem",
            "extra": "avg mem: 140.84619740322873, max mem: 156.546875, count: 55285"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.697879948094323, max cpu: 9.495549, count: 110570"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 154.01953125,
            "unit": "median mem",
            "extra": "avg mem: 137.72095851327214, max mem: 156.69921875, count: 110570"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 35860,
            "unit": "median block_count",
            "extra": "avg block_count: 35725.11357511079, max block_count: 73218.0, count: 55285"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 32,
            "unit": "median segment_count",
            "extra": "avg segment_count: 31.593054173826534, max segment_count: 75.0, count: 55285"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.772306785874071, max cpu: 9.514371, count: 55285"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 154.73828125,
            "unit": "median mem",
            "extra": "avg mem: 137.72399067050284, max mem: 157.74609375, count: 55285"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 5.408541896400163, max cpu: 9.365853, count: 55285"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 150.11328125,
            "unit": "median mem",
            "extra": "avg mem: 130.6616618007823, max mem: 150.87109375, count: 55285"
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
          "id": "288a4bfa0c79838d86711b8a6231687c984ac0b5",
          "message": "chore: Upgrade to `0.18.3` (#3160)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-09-15T16:13:06-04:00",
          "tree_id": "ad59a6c86e8afe29cabad5b0bcc6a78bc448182e",
          "url": "https://github.com/paradedb/paradedb/commit/288a4bfa0c79838d86711b8a6231687c984ac0b5"
        },
        "date": 1757968150307,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.900872185290812, max cpu: 14.356929, count: 54748"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 154.234375,
            "unit": "median mem",
            "extra": "avg mem: 140.63647865846698, max mem: 154.234375, count: 54748"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.555221908559234, max cpu: 9.239654, count: 54748"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 146.4375,
            "unit": "median mem",
            "extra": "avg mem: 132.2602580088542, max mem: 146.81640625, count: 54748"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.938907682271693, max cpu: 14.229248, count: 54748"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 154.18359375,
            "unit": "median mem",
            "extra": "avg mem: 140.8320515132973, max mem: 154.18359375, count: 54748"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.383420585029827, max cpu: 4.7477746, count: 54748"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 156.21875,
            "unit": "median mem",
            "extra": "avg mem: 142.3427085093291, max mem: 156.59765625, count: 54748"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.667767075402872, max cpu: 9.657948, count: 109496"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 155.11328125,
            "unit": "median mem",
            "extra": "avg mem: 140.22135522501964, max mem: 157.46484375, count: 109496"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 34955,
            "unit": "median block_count",
            "extra": "avg block_count: 36178.64342076423, max block_count: 74638.0, count: 54748"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 31,
            "unit": "median segment_count",
            "extra": "avg segment_count: 31.302111492657264, max segment_count: 76.0, count: 54748"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.545329367422088, max cpu: 9.619239, count: 54748"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 155.13671875,
            "unit": "median mem",
            "extra": "avg mem: 139.50547915277727, max mem: 156.6796875, count: 54748"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.91662551251048, max cpu: 13.9265, count: 54748"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 151.1015625,
            "unit": "median mem",
            "extra": "avg mem: 131.43817560972548, max mem: 153.04296875, count: 54748"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "af5bea23effe976b411147e259e53afad947a393",
          "message": "perf: remove `ExactBuffer` in favor of a regular rust `BufWriter` (#3159)\n\n# Ticket(s) Closed\n\n- Closes #2981  (/cc @yjhjstz)\n\nWhile #2981 wasn't the impetus for this, it addresses the complaint made\nthere just the same.\n\n## What\n\nIn profiling, our `ExactBuffer` was a large percentage of certain\nprofiles. This replaces it, and its complexity, with a standard Rust\n`BufWriter`.\n\n## Why\n\nImproves performance of (at least) our `wide-table.toml` test's \"Single\nUpdate\" job by quite a bit.\n\n<img width=\"720\" height=\"141\" alt=\"screenshot_2025-09-15_at_3 28\n33___pm_720\"\nsrc=\"https://github.com/user-attachments/assets/a373a7ae-df38-4691-980a-d6843f073d26\"\n/>\n\n\n## How\n\n## Tests\n\nExisting tests pass\n\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>",
          "timestamp": "2025-09-15T16:26:19-04:00",
          "tree_id": "cbc00b9a93c129255360f60e5a70904e87f1e8c1",
          "url": "https://github.com/paradedb/paradedb/commit/af5bea23effe976b411147e259e53afad947a393"
        },
        "date": 1757968936790,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.876959707682277, max cpu: 14.414414, count: 55374"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 154.87109375,
            "unit": "median mem",
            "extra": "avg mem: 143.35266889695524, max mem: 155.24609375, count: 55374"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.577329623547403, max cpu: 9.320388, count: 55374"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 145.85546875,
            "unit": "median mem",
            "extra": "avg mem: 133.76633840170928, max mem: 146.2421875, count: 55374"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.877056731903031, max cpu: 14.145383, count: 55374"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 155.25,
            "unit": "median mem",
            "extra": "avg mem: 143.58532236473798, max mem: 155.25, count: 55374"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.287427062042865, max cpu: 9.230769, count: 55374"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 153.55078125,
            "unit": "median mem",
            "extra": "avg mem: 142.10164919738958, max mem: 153.9296875, count: 55374"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.691320245491711, max cpu: 9.514371, count: 110748"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 154.84375,
            "unit": "median mem",
            "extra": "avg mem: 141.75295025079458, max mem: 157.51171875, count: 110748"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 39087,
            "unit": "median block_count",
            "extra": "avg block_count: 38745.22028388774, max block_count: 77116.0, count: 55374"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 32,
            "unit": "median segment_count",
            "extra": "avg segment_count: 31.693809369017952, max segment_count: 76.0, count: 55374"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.573582054090241, max cpu: 9.221902, count: 55374"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 155.703125,
            "unit": "median mem",
            "extra": "avg mem: 141.7621896247562, max mem: 157.203125, count: 55374"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.606897851728777, max cpu: 13.913043, count: 55374"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 149.37109375,
            "unit": "median mem",
            "extra": "avg mem: 134.19204156056995, max mem: 152.09375, count: 55374"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "7800d096e107acdbdec6297d0cb98ef030569e2b",
          "message": "chore: Upgrade to `0.18.3` (#3161)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-09-15T16:36:57-04:00",
          "tree_id": "c0962cc02d5690156721fd003c985f724ee9b20f",
          "url": "https://github.com/paradedb/paradedb/commit/7800d096e107acdbdec6297d0cb98ef030569e2b"
        },
        "date": 1757969572523,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.8916775551156695, max cpu: 14.51613, count: 55412"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 153.19921875,
            "unit": "median mem",
            "extra": "avg mem: 141.09307896134862, max mem: 153.19921875, count: 55412"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.639675459401109, max cpu: 9.311348, count: 55412"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 147.23828125,
            "unit": "median mem",
            "extra": "avg mem: 134.154749168727, max mem: 147.23828125, count: 55412"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.883982326567865, max cpu: 14.501511, count: 55412"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 155.85546875,
            "unit": "median mem",
            "extra": "avg mem: 143.6959088143137, max mem: 155.85546875, count: 55412"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.619827,
            "unit": "median cpu",
            "extra": "avg cpu: 4.422551049305816, max cpu: 4.7524753, count: 55412"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 153.5,
            "unit": "median mem",
            "extra": "avg mem: 141.4006297428806, max mem: 153.875, count: 55412"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.738458277882431, max cpu: 9.667674, count: 110824"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 153.2109375,
            "unit": "median mem",
            "extra": "avg mem: 139.7840608335063, max mem: 156.359375, count: 110824"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 35800,
            "unit": "median block_count",
            "extra": "avg block_count: 36462.221359994226, max block_count: 73438.0, count: 55412"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 31,
            "unit": "median segment_count",
            "extra": "avg segment_count: 31.17025193098968, max segment_count: 76.0, count: 55412"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.5459157354174495, max cpu: 9.320388, count: 55412"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 153.671875,
            "unit": "median mem",
            "extra": "avg mem: 139.61820840364, max mem: 157.83203125, count: 55412"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 5.039564506042983, max cpu: 9.320388, count: 55412"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 149.12890625,
            "unit": "median mem",
            "extra": "avg mem: 127.62169993356584, max mem: 150.34765625, count: 55412"
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
          "id": "f71a5572d645d23e58b949cc3f16645473c74735",
          "message": "chore: Sync `0.18.x` (#3162)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\n---------\n\nSigned-off-by: Moe <mdashti@gmail.com>\nCo-authored-by: Moe <mdashti@gmail.com>",
          "timestamp": "2025-09-15T17:11:39-04:00",
          "tree_id": "a75daf7f281149ef4317505338649d8b0d2ec8a4",
          "url": "https://github.com/paradedb/paradedb/commit/f71a5572d645d23e58b949cc3f16645473c74735"
        },
        "date": 1757971672897,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.968265770477942, max cpu: 19.692308, count: 55284"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 152.97265625,
            "unit": "median mem",
            "extra": "avg mem: 138.65013788177865, max mem: 152.97265625, count: 55284"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.55438972362856, max cpu: 9.411765, count: 55284"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 146.8984375,
            "unit": "median mem",
            "extra": "avg mem: 132.22945452685227, max mem: 147.6484375, count: 55284"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.951480343745078, max cpu: 14.723927, count: 55284"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 153.22265625,
            "unit": "median mem",
            "extra": "avg mem: 139.33095823949967, max mem: 153.6328125, count: 55284"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.569127979926768, max cpu: 4.923077, count: 55284"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 155.03125,
            "unit": "median mem",
            "extra": "avg mem: 140.36586987193402, max mem: 155.03125, count: 55284"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.620268886525446, max cpu: 9.486166, count: 110568"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 154.91796875,
            "unit": "median mem",
            "extra": "avg mem: 138.77668331083586, max mem: 156.16796875, count: 110568"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 35177.5,
            "unit": "median block_count",
            "extra": "avg block_count: 34965.72255987266, max block_count: 70584.0, count: 55284"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 31,
            "unit": "median segment_count",
            "extra": "avg segment_count: 30.71362419506548, max segment_count: 75.0, count: 55284"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.52853055299653, max cpu: 9.338522, count: 55284"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 156.17578125,
            "unit": "median mem",
            "extra": "avg mem: 138.7428079468065, max mem: 157.67578125, count: 55284"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.7105007,
            "unit": "median cpu",
            "extra": "avg cpu: 4.28047303678803, max cpu: 18.991098, count: 55284"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 150.05078125,
            "unit": "median mem",
            "extra": "avg mem: 130.26621308549943, max mem: 151.640625, count: 55284"
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
          "id": "878c50feef96d61785ad711ebe46250c920bed70",
          "message": "fix: sequential scan segfault (#3163)\n\n# Ticket(s) Closed\n\n- Closes #3151 \n\n## What\n\nThe `@@@` return type should be `bool`, not `SearchQueryInput`.\n\n## Why\n\n## How\n\n## Tests\n\nAdded regression test.",
          "timestamp": "2025-09-16T10:27:13-04:00",
          "tree_id": "6859469869310b79c8c32af68b3ed77dfb787362",
          "url": "https://github.com/paradedb/paradedb/commit/878c50feef96d61785ad711ebe46250c920bed70"
        },
        "date": 1758033801999,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.885027390254113, max cpu: 14.117648, count: 55171"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 153.96484375,
            "unit": "median mem",
            "extra": "avg mem: 141.2302627144469, max mem: 154.33984375, count: 55171"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.616405724641363, max cpu: 9.356726, count: 55171"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 146.5078125,
            "unit": "median mem",
            "extra": "avg mem: 132.62765205282213, max mem: 146.88671875, count: 55171"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.919067482981189, max cpu: 14.51613, count: 55171"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 152.37890625,
            "unit": "median mem",
            "extra": "avg mem: 139.78152846660836, max mem: 152.37890625, count: 55171"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.448594639515979, max cpu: 4.743083, count: 55171"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 154.4453125,
            "unit": "median mem",
            "extra": "avg mem: 141.34594049070165, max mem: 154.4453125, count: 55171"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.688410834589891, max cpu: 9.638554, count: 110342"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 155.70703125,
            "unit": "median mem",
            "extra": "avg mem: 140.5496682331977, max mem: 156.45703125, count: 110342"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 33937,
            "unit": "median block_count",
            "extra": "avg block_count: 34861.35357343532, max block_count: 70501.0, count: 55171"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 32,
            "unit": "median segment_count",
            "extra": "avg segment_count: 31.721520363959325, max segment_count: 77.0, count: 55171"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.615171251891738, max cpu: 9.320388, count: 55171"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 151.73828125,
            "unit": "median mem",
            "extra": "avg mem: 138.13707630310762, max mem: 156.99609375, count: 55171"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.641188057625666, max cpu: 13.967022, count: 55171"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 149.00390625,
            "unit": "median mem",
            "extra": "avg mem: 129.49249634092186, max mem: 151.05078125, count: 55171"
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
          "id": "f2a0c9c43e4385628cc7b828a8ed12c30e55050e",
          "message": "chore: don't warn about `raw` tokenizer on UUID key fields (#3166)\n\n## What\n\nRemove the warning about using the `raw` tokenizer when the `key_field`\nis a UUID field.\n\nThe drama here is that we (pg_search) assign the `raw` tokenizer to UUID\nfields used as the key_field so there's nothing a user can do about it.\nWarning about our own bad decisions is not cool.\n\n## Why\n\nMany user and customer complaints.\n\n## How\n\n## Tests\n\nExisting tests pass but a couple of the regression test expected output\nis now different.",
          "timestamp": "2025-09-16T13:10:47-04:00",
          "tree_id": "2b24aea6e3a0645c584d8ebb8ce7465c8c90f904",
          "url": "https://github.com/paradedb/paradedb/commit/f2a0c9c43e4385628cc7b828a8ed12c30e55050e"
        },
        "date": 1758043617972,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.881946526645582, max cpu: 13.953489, count: 55255"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 154.84375,
            "unit": "median mem",
            "extra": "avg mem: 141.25382926318886, max mem: 154.84375, count: 55255"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.62871916223159, max cpu: 9.4395275, count: 55255"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 147.21875,
            "unit": "median mem",
            "extra": "avg mem: 132.9837637006832, max mem: 147.59375, count: 55255"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.918860509560344, max cpu: 18.805092, count: 55255"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 153.60546875,
            "unit": "median mem",
            "extra": "avg mem: 139.76668952470365, max mem: 153.60546875, count: 55255"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.443454457171822, max cpu: 4.7477746, count: 55255"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 154.26171875,
            "unit": "median mem",
            "extra": "avg mem: 140.44754660211746, max mem: 154.63671875, count: 55255"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.671894871528699, max cpu: 9.486166, count: 110510"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 155.2421875,
            "unit": "median mem",
            "extra": "avg mem: 139.3321462706995, max mem: 156.50390625, count: 110510"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 34165,
            "unit": "median block_count",
            "extra": "avg block_count: 34452.66688987422, max block_count: 69678.0, count: 55255"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 32,
            "unit": "median segment_count",
            "extra": "avg segment_count: 31.56583114650258, max segment_count: 76.0, count: 55255"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.480399434123086, max cpu: 9.458128, count: 55255"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 155.11328125,
            "unit": "median mem",
            "extra": "avg mem: 138.41209364537147, max mem: 155.86328125, count: 55255"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.9130436124718155, max cpu: 9.311348, count: 55255"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 149.38671875,
            "unit": "median mem",
            "extra": "avg mem: 129.4262843860284, max mem: 150.2109375, count: 55255"
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
          "id": "489eb48583040067612195f9e1406d5e31a1599f",
          "message": "perf: teach custom scan callback to exit early if it can (#3168)\n\n## What\n\nThis does two things.  \n\nOne, the first commit (62d752572b2d7bc5a02b7203ac2c83949e38e27e) simply\nreorders some code in the custom scan callback so it can decide to exit\nearly if we're not going to submit a path. Specifically, this is\nintended to avoid opening a Directory and Index and related structures.\n\nTwo, the second commit (5ac1dde23ef0809bea4b942d04fd14acc9d1c152) makes\na new decision to not evaluate possible pushdown predicates when the\nstatement type is not a SELECT statement. This cuts out the overhead of\nneeding to read/deserialize the index's schema at all on (at least)\nUPDATE statements.\n\nThis does mean that we won't consider doing pushdowns for UPDATE\nstatements, even if doing one would make the UPDATE scan faster.\n\n## Why\n\nTrying to reduce per-query overhead, targeting our stressgres benchmarks\nlike \"single-server.toml\" and \"wide-table.toml\".\n\n## How\n\n## Tests\n\nAll existing tests pass.",
          "timestamp": "2025-09-16T17:39:51-04:00",
          "tree_id": "0ebcd01c6225cbb43b199470f7f78bd694493ed7",
          "url": "https://github.com/paradedb/paradedb/commit/489eb48583040067612195f9e1406d5e31a1599f"
        },
        "date": 1758059758790,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.883834192076605, max cpu: 14.530776, count: 55419"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 154.22265625,
            "unit": "median mem",
            "extra": "avg mem: 140.67235775692905, max mem: 154.22265625, count: 55419"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.594653734173677, max cpu: 9.347614, count: 55419"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 26.26171875,
            "unit": "median mem",
            "extra": "avg mem: 27.118226882928237, max mem: 32.2109375, count: 55419"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.952076620445947, max cpu: 19.6319, count: 55419"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 154.8046875,
            "unit": "median mem",
            "extra": "avg mem: 141.0659485859994, max mem: 154.8046875, count: 55419"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.620663263379893, max cpu: 9.275363, count: 55419"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 155.265625,
            "unit": "median mem",
            "extra": "avg mem: 141.2575285128972, max mem: 155.640625, count: 55419"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.669754555009268, max cpu: 9.458128, count: 110838"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 154.125,
            "unit": "median mem",
            "extra": "avg mem: 138.76265226332575, max mem: 157.5078125, count: 110838"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 34017,
            "unit": "median block_count",
            "extra": "avg block_count: 34679.66628773525, max block_count: 72636.0, count: 55419"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 31,
            "unit": "median segment_count",
            "extra": "avg segment_count: 31.375322542810228, max segment_count: 75.0, count: 55419"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.678658780926449, max cpu: 9.257474, count: 55419"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 156.1796875,
            "unit": "median mem",
            "extra": "avg mem: 140.05923763736263, max mem: 158.4296875, count: 55419"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.175743032691384, max cpu: 9.402546, count: 55419"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 149.76171875,
            "unit": "median mem",
            "extra": "avg mem: 132.25896691567874, max mem: 151.328125, count: 55419"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "63daa7f2bf568127e538f19f942d6363508ca615",
          "message": "chore: don't warn about `raw` tokenizer on UUID key fields (#3167)\n\n## What\n\nRemove the warning about using the `raw` tokenizer when the `key_field`\nis a UUID field.\n\nThe drama here is that we (pg_search) assign the `raw` tokenizer to UUID\nfields used as the key_field so there's nothing a user can do about it.\nWarning about our own bad decisions is not cool.\n\n## Why\n\nMany user and customer complaints.\n\n## How\n\n## Tests\n\nExisting tests pass but a couple of the regression test expected output\nis now different.\n\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>",
          "timestamp": "2025-09-17T10:06:31-04:00",
          "tree_id": "2c472616485a1c2a1ed61c7f2c030286882deb06",
          "url": "https://github.com/paradedb/paradedb/commit/63daa7f2bf568127e538f19f942d6363508ca615"
        },
        "date": 1758118967349,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.956287869161171, max cpu: 14.663951, count: 54896"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 155.0234375,
            "unit": "median mem",
            "extra": "avg mem: 141.09816659000657, max mem: 155.0234375, count: 54896"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.540714130419399, max cpu: 9.411765, count: 54896"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 25.31640625,
            "unit": "median mem",
            "extra": "avg mem: 25.52687937759582, max mem: 27.28515625, count: 54896"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.925365505266657, max cpu: 14.215202, count: 54896"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 154.44921875,
            "unit": "median mem",
            "extra": "avg mem: 140.68089639101666, max mem: 154.44921875, count: 54896"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.379148169401181, max cpu: 4.7477746, count: 54896"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 154.44921875,
            "unit": "median mem",
            "extra": "avg mem: 140.29175779542226, max mem: 154.44921875, count: 54896"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.626387105926296, max cpu: 9.448819, count: 109792"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 155.65625,
            "unit": "median mem",
            "extra": "avg mem: 139.83311148865582, max mem: 156.3671875, count: 109792"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 33753,
            "unit": "median block_count",
            "extra": "avg block_count: 34842.29018507724, max block_count: 72372.0, count: 54896"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 31,
            "unit": "median segment_count",
            "extra": "avg segment_count: 31.019017779073156, max segment_count: 77.0, count: 54896"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.506379861117358, max cpu: 9.284333, count: 54896"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 153.84375,
            "unit": "median mem",
            "extra": "avg mem: 138.67406102277488, max mem: 157.21875, count: 54896"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 5.2753185764653985, max cpu: 9.458128, count: 54896"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 149.7734375,
            "unit": "median mem",
            "extra": "avg mem: 131.46255696851773, max mem: 150.578125, count: 54896"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "2d344837c944c402f2781abd28b10a2f0e7185ac",
          "message": "perf: teach custom scan callback to exit early if it can (#3169)\n\n## What\n\nThis does two things.  \n\nOne, the first commit (62d752572b2d7bc5a02b7203ac2c83949e38e27e) simply\nreorders some code in the custom scan callback so it can decide to exit\nearly if we're not going to submit a path. Specifically, this is\nintended to avoid opening a Directory and Index and related structures.\n\nTwo, the second commit (5ac1dde23ef0809bea4b942d04fd14acc9d1c152) makes\na new decision to not evaluate possible pushdown predicates when the\nstatement type is not a SELECT statement. This cuts out the overhead of\nneeding to read/deserialize the index's schema at all on (at least)\nUPDATE statements.\n\nThis does mean that we won't consider doing pushdowns for UPDATE\nstatements, even if doing one would make the UPDATE scan faster.\n\n## Why\n\nTrying to reduce per-query overhead, targeting our stressgres benchmarks\nlike \"single-server.toml\" and \"wide-table.toml\".\n\n## How\n\n## Tests\n\nAll existing tests pass.\n\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>",
          "timestamp": "2025-09-17T10:06:44-04:00",
          "tree_id": "efcbedd45680505ce75bd6fe8a623a0066b38fdb",
          "url": "https://github.com/paradedb/paradedb/commit/2d344837c944c402f2781abd28b10a2f0e7185ac"
        },
        "date": 1758118983028,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.860291621297286, max cpu: 14.428859, count: 55443"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 154.18359375,
            "unit": "median mem",
            "extra": "avg mem: 141.53910723907887, max mem: 154.18359375, count: 55443"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.6228384310660635, max cpu: 9.448819, count: 55443"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 27.97265625,
            "unit": "median mem",
            "extra": "avg mem: 28.46805974999098, max mem: 33.37109375, count: 55443"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.901227296704622, max cpu: 14.589665, count: 55443"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 153.62109375,
            "unit": "median mem",
            "extra": "avg mem: 141.2760022821862, max mem: 153.62109375, count: 55443"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.2883729078428185, max cpu: 4.7524753, count: 55443"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 153.98046875,
            "unit": "median mem",
            "extra": "avg mem: 141.44521400357124, max mem: 153.98046875, count: 55443"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.693131150597858, max cpu: 9.628887, count: 110886"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 154.69140625,
            "unit": "median mem",
            "extra": "avg mem: 140.00097848240534, max mem: 155.4296875, count: 110886"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 38225,
            "unit": "median block_count",
            "extra": "avg block_count: 38295.33059177895, max block_count: 77797.0, count: 55443"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 32,
            "unit": "median segment_count",
            "extra": "avg segment_count: 31.75172699889977, max segment_count: 77.0, count: 55443"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.673348060256405, max cpu: 9.495549, count: 55443"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 156.703125,
            "unit": "median mem",
            "extra": "avg mem: 139.99374878816982, max mem: 157.078125, count: 55443"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 5.678487067754223, max cpu: 9.430255, count: 55443"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 150.42578125,
            "unit": "median mem",
            "extra": "avg mem: 133.71967641598127, max mem: 152.3671875, count: 55443"
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
          "id": "eb456f8c97d99e92e2795d88dd2c1082c13c83a6",
          "message": "perf: optimize `Timestamp` and `JsonB` datum decoding (#3171)\n\n## What\n\nOptimize `Timestamp` and `JsonB` to `TantivyValue` datum conversions.\n\nThese two show up quite high in profiles. The `JsonB` conversion in\nparticular has been bad due to how pgrx stupidly (I can say it) handles\n`JsonB` values by converting them to strings and then asking serde to\nparse the strings.\n\n## Why\n\nTrying to make things faster.\n\n## How\n\nFor the `Timestamp` conversion we memoize Postgres' understanding of the\ncurrent EPOCH and do the same math that it does to calculate a time\nvalue.\n\nFor the `JsonB` conversion we implement our own deserializer routine\nusing Postgres' internal `JsonbIteratorInit()` and `JsonbIteratorNext()`\nfunctions, building up a `serde_json::Value` structure as it goes.\n\n\n## Tests\n\nA new `#[pg_test]`-based proptest has been added to test our custom\njsonb deserializer against normal serde.\n\n---------\n\nSigned-off-by: Eric Ridge <eebbrr@gmail.com>\nCo-authored-by: Stu Hood <stuhood@paradedb.com>",
          "timestamp": "2025-09-17T15:26:06-04:00",
          "tree_id": "702cea735a514e9b33d6c1ee785606d39d4f705c",
          "url": "https://github.com/paradedb/paradedb/commit/eb456f8c97d99e92e2795d88dd2c1082c13c83a6"
        },
        "date": 1758138216850,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.943780256670326, max cpu: 14.271556, count: 55401"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 152.76953125,
            "unit": "median mem",
            "extra": "avg mem: 138.8553739158589, max mem: 152.76953125, count: 55401"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.708450455009531, max cpu: 9.311348, count: 55401"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 26.31640625,
            "unit": "median mem",
            "extra": "avg mem: 26.49723542614303, max mem: 30.97265625, count: 55401"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.845575118983158, max cpu: 14.159292, count: 55401"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 154.09765625,
            "unit": "median mem",
            "extra": "avg mem: 140.62730697268552, max mem: 154.09765625, count: 55401"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.404668758997092, max cpu: 9.284333, count: 55401"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 154.34375,
            "unit": "median mem",
            "extra": "avg mem: 140.80648730392954, max mem: 154.34375, count: 55401"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.701240382525659, max cpu: 9.628887, count: 110802"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 154.640625,
            "unit": "median mem",
            "extra": "avg mem: 139.0387328720375, max mem: 156.5390625, count: 110802"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 35154,
            "unit": "median block_count",
            "extra": "avg block_count: 35305.42988393711, max block_count: 72270.0, count: 55401"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 32,
            "unit": "median segment_count",
            "extra": "avg segment_count: 31.431761159545857, max segment_count: 76.0, count: 55401"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.5074127516125815, max cpu: 9.248554, count: 55401"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 155.85546875,
            "unit": "median mem",
            "extra": "avg mem: 138.57328740117507, max mem: 156.98046875, count: 55401"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.561227510922781, max cpu: 9.430255, count: 55401"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 148.7265625,
            "unit": "median mem",
            "extra": "avg mem: 130.70168246342575, max mem: 150.69140625, count: 55401"
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
          "id": "849076799ca599dfbf0f2415149b12495b24624c",
          "message": "fix: Always use a shared expression context to execute HeapFilters. (#3174)\n\n## What\n\nEvaluating heap filters was leaking a `CreateStandaloneExprContext` per\nexecution, which could eventually lead to OOMs.\n\n## Why\n\n`PgBox::from_pg` does not free the resulting memory: it's expected to be\nfreed by a memory context, since PG created it.\n\n## How\n\nAlways use a global expression context created by\n`ExecAssignExprContext` in `begin_custom_scan`.\n\n## Tests\n\nA repro uses significantly less memory, and [a benchmark query added for\nthis purpose](https://github.com/paradedb/paradedb/pull/3175) is faster.",
          "timestamp": "2025-09-17T16:44:32-07:00",
          "tree_id": "7eef1c518a935389aa23e91c6bc47bbc325b18e6",
          "url": "https://github.com/paradedb/paradedb/commit/849076799ca599dfbf0f2415149b12495b24624c"
        },
        "date": 1758153652327,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.924424480449956, max cpu: 14.131501, count: 55350"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 154.7890625,
            "unit": "median mem",
            "extra": "avg mem: 140.51464445009034, max mem: 154.7890625, count: 55350"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.604137890327435, max cpu: 9.458128, count: 55350"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 25.75,
            "unit": "median mem",
            "extra": "avg mem: 26.264929355803975, max mem: 29.68359375, count: 55350"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.964634272816684, max cpu: 14.589665, count: 55350"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 154.49609375,
            "unit": "median mem",
            "extra": "avg mem: 140.34583926151763, max mem: 154.87109375, count: 55350"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.46566711394464, max cpu: 4.738401, count: 55350"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 154.42578125,
            "unit": "median mem",
            "extra": "avg mem: 140.22266119015356, max mem: 154.80078125, count: 55350"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.646179177699537, max cpu: 9.495549, count: 110700"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 154.6484375,
            "unit": "median mem",
            "extra": "avg mem: 139.1226351132001, max mem: 156.3515625, count: 110700"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 35738,
            "unit": "median block_count",
            "extra": "avg block_count: 36349.19403794038, max block_count: 75085.0, count: 55350"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 32,
            "unit": "median segment_count",
            "extra": "avg segment_count: 31.483396567299007, max segment_count: 76.0, count: 55350"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.575380545408863, max cpu: 9.320388, count: 55350"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 155.81640625,
            "unit": "median mem",
            "extra": "avg mem: 139.36739357497743, max mem: 158.4453125, count: 55350"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.965216194306457, max cpu: 9.4395275, count: 55350"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 148.5234375,
            "unit": "median mem",
            "extra": "avg mem: 130.4504032576784, max mem: 150.49609375, count: 55350"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "dc0e87d6fdabdb573d6f37a24c6c33befa43e8b0",
          "message": "fix: Always use a shared expression context to execute HeapFilters. (#3176)\n\n## What\n\nEvaluating heap filters was leaking a `CreateStandaloneExprContext` per\nexecution, which could eventually lead to OOMs.\n\n## Why\n\n`PgBox::from_pg` does not free the resulting memory: it's expected to be\nfreed by a memory context, since PG created it.\n\n## How\n\nAlways use a global expression context created by\n`ExecAssignExprContext` in `begin_custom_scan`.\n\n## Tests\n\nA repro uses significantly less memory, and [a benchmark query added for\nthis purpose](https://github.com/paradedb/paradedb/pull/3175) is faster.\n\nCo-authored-by: Stu Hood <stuhood@paradedb.com>",
          "timestamp": "2025-09-17T17:11:43-07:00",
          "tree_id": "0c30f446ad8404b4f66727777f1b6e6a5bc8958e",
          "url": "https://github.com/paradedb/paradedb/commit/dc0e87d6fdabdb573d6f37a24c6c33befa43e8b0"
        },
        "date": 1758155283961,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.898359344447021, max cpu: 14.738997, count: 55474"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 152.24609375,
            "unit": "median mem",
            "extra": "avg mem: 138.08786886649602, max mem: 153.0, count: 55474"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.581584801186899, max cpu: 9.356726, count: 55474"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 25.65625,
            "unit": "median mem",
            "extra": "avg mem: 26.208246909588276, max mem: 30.35546875, count: 55474"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.894771169049648, max cpu: 13.899614, count: 55474"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 152.70703125,
            "unit": "median mem",
            "extra": "avg mem: 138.385695959481, max mem: 152.70703125, count: 55474"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.623298909219456, max cpu: 4.8144436, count: 55474"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 155.49609375,
            "unit": "median mem",
            "extra": "avg mem: 140.5384846673667, max mem: 155.49609375, count: 55474"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.688095063609038, max cpu: 9.458128, count: 110948"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 154.88671875,
            "unit": "median mem",
            "extra": "avg mem: 138.51632510979243, max mem: 156.78125, count: 110948"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 32450,
            "unit": "median block_count",
            "extra": "avg block_count: 33925.77241590655, max block_count: 69891.0, count: 55474"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 31,
            "unit": "median segment_count",
            "extra": "avg segment_count: 31.259581065003424, max segment_count: 76.0, count: 55474"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.631431699299266, max cpu: 9.347614, count: 55474"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 155.44921875,
            "unit": "median mem",
            "extra": "avg mem: 138.55946261142608, max mem: 156.94921875, count: 55474"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 5.458680152549942, max cpu: 9.430255, count: 55474"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 146.6484375,
            "unit": "median mem",
            "extra": "avg mem: 125.32485038035837, max mem: 149.8203125, count: 55474"
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
          "id": "3bcb1451087be74b7bd73bfc7d6546423046a0ce",
          "message": "fix: write all delete files atomically (#3178)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nIn `ambulkdelete`, write all delete files to the meta list atomically,\ninstead of one at a time.\n\n## Why\n\nReduces the number of writes to the metas list.\n\n## How\n\n## Tests",
          "timestamp": "2025-09-18T16:03:10-04:00",
          "tree_id": "ad9609f0419a34b8f0cf543e911c1dc3c25d4563",
          "url": "https://github.com/paradedb/paradedb/commit/3bcb1451087be74b7bd73bfc7d6546423046a0ce"
        },
        "date": 1758226774972,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.92650029247717, max cpu: 18.443804, count: 54795"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 153.92578125,
            "unit": "median mem",
            "extra": "avg mem: 137.15920511965052, max mem: 153.92578125, count: 54795"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.6192747577190225, max cpu: 9.476802, count: 54795"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 26.80859375,
            "unit": "median mem",
            "extra": "avg mem: 26.517847559654165, max mem: 29.96484375, count: 54795"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.941887194437177, max cpu: 14.769231, count: 54795"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 153.5078125,
            "unit": "median mem",
            "extra": "avg mem: 136.88815713682817, max mem: 153.5078125, count: 54795"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.603657932182009, max cpu: 4.824121, count: 54795"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 153.42578125,
            "unit": "median mem",
            "extra": "avg mem: 136.36273304190618, max mem: 153.42578125, count: 54795"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.686495580070083, max cpu: 9.648242, count: 109590"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 152.95703125,
            "unit": "median mem",
            "extra": "avg mem: 135.65632349838032, max mem: 155.640625, count: 109590"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 26592,
            "unit": "median block_count",
            "extra": "avg block_count: 26894.818651336802, max block_count: 53143.0, count: 54795"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 30,
            "unit": "median segment_count",
            "extra": "avg segment_count: 30.041445387352862, max segment_count: 74.0, count: 54795"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.635534623766033, max cpu: 9.467456, count: 54795"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 149.390625,
            "unit": "median mem",
            "extra": "avg mem: 133.1187726697235, max mem: 154.27734375, count: 54795"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.375155598062098, max cpu: 9.311348, count: 54795"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 145.8828125,
            "unit": "median mem",
            "extra": "avg mem: 127.23406282792682, max mem: 150.578125, count: 54795"
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
          "id": "6e11875ba052ccd6937ca0c535b3803309c8b6eb",
          "message": "feat: removed aggregation limitations re mix of aggregate functions and aggregation on group-by column. (#3179)\n\n# Ticket(s) Closed\n\n- Closes #2963\n\n## What\n\nRemoves aggregate limitations that prevented queries where the same\nfield is used in both `GROUP BY` and aggregate functions (e.g., `SELECT\nrating, AVG(rating) FROM table GROUP BY rating`).\n\n## Why\n\nPrevious safety checks blocked these queries due to Tantivy's\n\"incompatible fruit types\" errors, but testing shows the underlying\nissue is resolved. The limitations were overly restrictive and caused\nunnecessary fallbacks to slower PostgreSQL aggregation.\n\n## How\n\n- Removed `has_search_field_conflicts` function and field conflict\nvalidation\n- Eliminated ~35 lines of restrictive code in\n`extract_and_validate_aggregates`\n- Previously blocked queries now use faster `AggregateScan` instead of\n`GroupAggregate`\n\n## Tests\n\n- **`aggregate-groupby-conflict.sql`** - Tests `GROUP BY field` with\naggregates on same field\n- **`test-fruit-types-issue.sql`** - Validates #2963 issue resolution  \n- **`groupby_aggregate.out`** - Updated expectations showing\n`AggregateScan` usage",
          "timestamp": "2025-09-18T16:00:25-07:00",
          "tree_id": "f85924512f419186b824a986dd35eaa96d973884",
          "url": "https://github.com/paradedb/paradedb/commit/6e11875ba052ccd6937ca0c535b3803309c8b6eb"
        },
        "date": 1758237403362,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.806783096341936, max cpu: 14.4723625, count: 55250"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 154.1484375,
            "unit": "median mem",
            "extra": "avg mem: 138.28841084558823, max mem: 154.1484375, count: 55250"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.648725043613653, max cpu: 9.60961, count: 55250"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 25.734375,
            "unit": "median mem",
            "extra": "avg mem: 26.309173430429865, max mem: 30.04296875, count: 55250"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.897508925949183, max cpu: 14.486921, count: 55250"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 154.765625,
            "unit": "median mem",
            "extra": "avg mem: 138.88069753959277, max mem: 154.765625, count: 55250"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.589613258707651, max cpu: 4.729064, count: 55250"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 154.46484375,
            "unit": "median mem",
            "extra": "avg mem: 138.49936814196832, max mem: 154.83984375, count: 55250"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.704406555382931, max cpu: 9.657948, count: 110500"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 151.625,
            "unit": "median mem",
            "extra": "avg mem: 134.36915738122173, max mem: 152.75, count: 110500"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 28351,
            "unit": "median block_count",
            "extra": "avg block_count: 27992.53011764706, max block_count: 54302.0, count: 55250"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 30,
            "unit": "median segment_count",
            "extra": "avg segment_count: 29.819239819004526, max segment_count: 73.0, count: 55250"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.5574325536874465, max cpu: 9.458128, count: 55250"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 153.20703125,
            "unit": "median mem",
            "extra": "avg mem: 136.11357487273756, max mem: 158.09765625, count: 55250"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.127911249693383, max cpu: 4.7105007, count: 55250"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 145.90625,
            "unit": "median mem",
            "extra": "avg mem: 127.48790999717194, max mem: 149.51171875, count: 55250"
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
          "id": "020f92b742187fe6fdc75a19390692e6d2e9a373",
          "message": "feat: Port `ambulkdelete_epoch` to community (#3180)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nAdd `ambulkdelete_epoch` to the index metadata and make sure it's passed\ndown to all scans. In enterprise, this value is used to detect query\nconflicts with concurrent vacuums on the primary.\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-09-18T19:16:09-04:00",
          "tree_id": "3642b293b38caa7676318f888b910c3f934e1976",
          "url": "https://github.com/paradedb/paradedb/commit/020f92b742187fe6fdc75a19390692e6d2e9a373"
        },
        "date": 1758238352917,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.946423024118888, max cpu: 14.51613, count: 55475"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 154.71484375,
            "unit": "median mem",
            "extra": "avg mem: 138.14997380576835, max mem: 154.71484375, count: 55475"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.629018278817276, max cpu: 9.430255, count: 55475"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 28.2265625,
            "unit": "median mem",
            "extra": "avg mem: 27.41792129055881, max mem: 31.02734375, count: 55475"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.966910765655053, max cpu: 14.145383, count: 55475"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 156.2890625,
            "unit": "median mem",
            "extra": "avg mem: 139.65435993127534, max mem: 156.2890625, count: 55475"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.560607488521784, max cpu: 4.7952046, count: 55475"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 154.03125,
            "unit": "median mem",
            "extra": "avg mem: 137.28947308753942, max mem: 154.03125, count: 55475"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.674087703765533, max cpu: 9.495549, count: 110950"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 154.10546875,
            "unit": "median mem",
            "extra": "avg mem: 135.765834659475, max mem: 156.00390625, count: 110950"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 28205,
            "unit": "median block_count",
            "extra": "avg block_count: 27925.30765209554, max block_count: 54453.0, count: 55475"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 30,
            "unit": "median segment_count",
            "extra": "avg segment_count: 30.04441640378549, max segment_count: 76.0, count: 55475"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.551850941817225, max cpu: 4.828974, count: 55475"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 152.625,
            "unit": "median mem",
            "extra": "avg mem: 136.99722693217666, max mem: 157.51953125, count: 55475"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.610951,
            "unit": "median cpu",
            "extra": "avg cpu: 3.620486597331313, max cpu: 9.29332, count: 55475"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 147.32421875,
            "unit": "median mem",
            "extra": "avg mem: 126.50605283911672, max mem: 150.51953125, count: 55475"
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
          "id": "c0dc0e674f3524c2f65a7b387ccbf594b23e5e0e",
          "message": "chore: Upgrade to `0.18.4` (#3181)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\n---------\n\nSigned-off-by: Ming <ming.ying.nyc@gmail.com>\nCo-authored-by: Stu Hood <stuhood@paradedb.com>",
          "timestamp": "2025-09-18T19:18:34-04:00",
          "tree_id": "b67f22553ed7786ef556afbfad2b7f8ddc6b139e",
          "url": "https://github.com/paradedb/paradedb/commit/c0dc0e674f3524c2f65a7b387ccbf594b23e5e0e"
        },
        "date": 1758238601735,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.897066760252287, max cpu: 14.215202, count: 55035"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 154.6796875,
            "unit": "median mem",
            "extra": "avg mem: 140.16757042393476, max mem: 154.6796875, count: 55035"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.525190034607228, max cpu: 9.275363, count: 55035"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 25.30078125,
            "unit": "median mem",
            "extra": "avg mem: 25.584525330471518, max mem: 28.859375, count: 55035"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.897226728610807, max cpu: 14.4, count: 55035"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 152.546875,
            "unit": "median mem",
            "extra": "avg mem: 138.58194742550197, max mem: 152.93359375, count: 55035"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.596342383688474, max cpu: 9.302325, count: 55035"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 154.0859375,
            "unit": "median mem",
            "extra": "avg mem: 139.45780675081767, max mem: 154.0859375, count: 55035"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.6683667183316535, max cpu: 9.486166, count: 110070"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 155.04296875,
            "unit": "median mem",
            "extra": "avg mem: 138.54181926842008, max mem: 157.30859375, count: 110070"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 29126,
            "unit": "median block_count",
            "extra": "avg block_count: 28682.278513673118, max block_count: 54545.0, count: 55035"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 30,
            "unit": "median segment_count",
            "extra": "avg segment_count: 29.748959752884527, max segment_count: 74.0, count: 55035"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.541389700924106, max cpu: 6.3702717, count: 55035"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 150.765625,
            "unit": "median mem",
            "extra": "avg mem: 135.7300626164032, max mem: 155.3125, count: 55035"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 3.4945376100935444, max cpu: 9.29332, count: 55035"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 147.24609375,
            "unit": "median mem",
            "extra": "avg mem: 129.96281045584627, max mem: 151.26171875, count: 55035"
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
          "id": "a97c525d0c11314fe01c8bbb250ea5fdf73ec8ce",
          "message": "fix: write all delete files atomically (#3178) (#3182)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nIn `ambulkdelete`, write all delete files to the meta list atomically,\ninstead of one at a time.\n\n## Why\n\nReduces the number of writes to the metas list.\n\n## How\n\n## Tests\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-09-18T21:50:00-04:00",
          "tree_id": "ba5917ed034f24a8e2ad95a64751e5faef3d55d5",
          "url": "https://github.com/paradedb/paradedb/commit/a97c525d0c11314fe01c8bbb250ea5fdf73ec8ce"
        },
        "date": 1758247576908,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.936345769488332, max cpu: 14.769231, count: 55385"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 153.12109375,
            "unit": "median mem",
            "extra": "avg mem: 136.585875998691, max mem: 153.49609375, count: 55385"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.574012541133873, max cpu: 9.628887, count: 55385"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 26.12890625,
            "unit": "median mem",
            "extra": "avg mem: 26.424504745192742, max mem: 29.66015625, count: 55385"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.922360919106529, max cpu: 14.45783, count: 55385"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 153.5625,
            "unit": "median mem",
            "extra": "avg mem: 137.46464894883542, max mem: 153.9609375, count: 55385"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.573197536043365, max cpu: 4.7571855, count: 55385"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 154.28515625,
            "unit": "median mem",
            "extra": "avg mem: 137.49148742890674, max mem: 154.28515625, count: 55385"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.653593112302503, max cpu: 14.007783, count: 110770"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 151.69140625,
            "unit": "median mem",
            "extra": "avg mem: 134.44299029661687, max mem: 155.44140625, count: 110770"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 27279,
            "unit": "median block_count",
            "extra": "avg block_count: 27098.133375462672, max block_count: 52969.0, count: 55385"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 30,
            "unit": "median segment_count",
            "extra": "avg segment_count: 29.785194547260087, max segment_count: 75.0, count: 55385"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.670625839777491, max cpu: 9.347614, count: 55385"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 151.24609375,
            "unit": "median mem",
            "extra": "avg mem: 135.56622407353075, max mem: 157.265625, count: 55385"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.040400829828268, max cpu: 4.7244096, count: 55385"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 147.54296875,
            "unit": "median mem",
            "extra": "avg mem: 127.76954507368872, max mem: 151.07421875, count: 55385"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "e15e51abfc4b7834faea068d861d91d5d873580f",
          "message": "chore: Upgrade to `0.18.4` (#3184)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\nSigned-off-by: Ming <ming.ying.nyc@gmail.com>\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>\nCo-authored-by: Stu Hood <stuhood@paradedb.com>",
          "timestamp": "2025-09-18T21:52:13-04:00",
          "tree_id": "3d203e3468a4e7504d03af9c39ac9a0869033086",
          "url": "https://github.com/paradedb/paradedb/commit/e15e51abfc4b7834faea068d861d91d5d873580f"
        },
        "date": 1758247704947,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.8867609617982675, max cpu: 14.443329, count: 55346"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 152.95703125,
            "unit": "median mem",
            "extra": "avg mem: 136.3135209918061, max mem: 152.95703125, count: 55346"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.566452614894846, max cpu: 9.257474, count: 55346"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 25.2265625,
            "unit": "median mem",
            "extra": "avg mem: 25.879787284198496, max mem: 28.3203125, count: 55346"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.876010801071171, max cpu: 19.611849, count: 55346"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 153.0703125,
            "unit": "median mem",
            "extra": "avg mem: 136.71364341765891, max mem: 153.0703125, count: 55346"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.512967149485291, max cpu: 4.729064, count: 55346"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 153.984375,
            "unit": "median mem",
            "extra": "avg mem: 137.66205820542586, max mem: 153.984375, count: 55346"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.656100680022612, max cpu: 9.467456, count: 110692"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 152.10546875,
            "unit": "median mem",
            "extra": "avg mem: 135.3639227753022, max mem: 155.515625, count: 110692"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 28012,
            "unit": "median block_count",
            "extra": "avg block_count: 27605.775214107613, max block_count: 53802.0, count: 55346"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 30,
            "unit": "median segment_count",
            "extra": "avg segment_count: 30.23564485238319, max segment_count: 74.0, count: 55346"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.504760367386307, max cpu: 4.8484845, count: 55346"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 148.9296875,
            "unit": "median mem",
            "extra": "avg mem: 133.59327034699888, max mem: 155.71875, count: 55346"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6153846,
            "unit": "median cpu",
            "extra": "avg cpu: 2.9801620004360303, max cpu: 4.669261, count: 55346"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 145.73046875,
            "unit": "median mem",
            "extra": "avg mem: 125.85398419714161, max mem: 149.19140625, count: 55346"
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
          "id": "1046018b2db9614ef172bd802c98a3987da7513e",
          "message": "chore: small diff ported from enterprise for query cancel fix (#3186)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nSome small changes in enterprise that should be in community\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-09-18T21:53:42-04:00",
          "tree_id": "85ed1f4eb7261157deabdfba479dc61164775f99",
          "url": "https://github.com/paradedb/paradedb/commit/1046018b2db9614ef172bd802c98a3987da7513e"
        },
        "date": 1758247796717,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.911657370652558, max cpu: 14.738997, count: 55238"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 153.0390625,
            "unit": "median mem",
            "extra": "avg mem: 137.25299761090645, max mem: 153.79296875, count: 55238"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.608596999359459, max cpu: 9.266409, count: 55238"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 26.015625,
            "unit": "median mem",
            "extra": "avg mem: 26.122262485177774, max mem: 29.13671875, count: 55238"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.887613220596309, max cpu: 15.80247, count: 55238"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 153.078125,
            "unit": "median mem",
            "extra": "avg mem: 137.22588984259025, max mem: 153.453125, count: 55238"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.558720502118301, max cpu: 4.804805, count: 55238"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 154.82421875,
            "unit": "median mem",
            "extra": "avg mem: 138.37542224951574, max mem: 155.19921875, count: 55238"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.649006035678673, max cpu: 9.836065, count: 110476"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 152.01171875,
            "unit": "median mem",
            "extra": "avg mem: 135.20337678687, max mem: 154.66796875, count: 110476"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 28341,
            "unit": "median block_count",
            "extra": "avg block_count: 28266.40186103769, max block_count: 55717.0, count: 55238"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 30,
            "unit": "median segment_count",
            "extra": "avg segment_count: 30.178844273869437, max segment_count: 73.0, count: 55238"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.535507123575913, max cpu: 9.266409, count: 55238"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 152.078125,
            "unit": "median mem",
            "extra": "avg mem: 135.49053605251368, max mem: 156.22265625, count: 55238"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.469373217394965, max cpu: 4.7105007, count: 55238"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 147.2734375,
            "unit": "median mem",
            "extra": "avg mem: 127.78577516270502, max mem: 150.95703125, count: 55238"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "f052aabf25719cee68a756a379c6b66e39452759",
          "message": "feat: Port `ambulkdelete_epoch` to community (#3183)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nAdd `ambulkdelete_epoch` to the index metadata and make sure it's passed\ndown to all scans. In enterprise, this value is used to detect query\nconflicts with concurrent vacuums on the primary.\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-09-18T22:01:15-04:00",
          "tree_id": "48ffae94b2f43d5c2d62b5adb846d1dcc2992aee",
          "url": "https://github.com/paradedb/paradedb/commit/f052aabf25719cee68a756a379c6b66e39452759"
        },
        "date": 1758248250551,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.862731388364962, max cpu: 9.657948, count: 55384"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 152.359375,
            "unit": "median mem",
            "extra": "avg mem: 137.1669631290174, max mem: 152.359375, count: 55384"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.626897017029569, max cpu: 9.329447, count: 55384"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 25.92578125,
            "unit": "median mem",
            "extra": "avg mem: 26.35929762826358, max mem: 31.4375, count: 55384"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.885576679181895, max cpu: 14.173229, count: 55384"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 153.81640625,
            "unit": "median mem",
            "extra": "avg mem: 138.3269461566066, max mem: 153.81640625, count: 55384"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.455760973944881, max cpu: 4.7244096, count: 55384"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 152.78125,
            "unit": "median mem",
            "extra": "avg mem: 137.25973318106313, max mem: 153.21484375, count: 55384"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.70063211571687, max cpu: 9.476802, count: 110768"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 152.671875,
            "unit": "median mem",
            "extra": "avg mem: 136.45873794214032, max mem: 155.68359375, count: 110768"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 27454,
            "unit": "median block_count",
            "extra": "avg block_count: 27558.641358515095, max block_count: 53301.0, count: 55384"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 30,
            "unit": "median segment_count",
            "extra": "avg segment_count: 29.924996388848765, max segment_count: 76.0, count: 55384"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.586081229298855, max cpu: 9.458128, count: 55384"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 152.875,
            "unit": "median mem",
            "extra": "avg mem: 136.81642190772607, max mem: 155.875, count: 55384"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.673807,
            "unit": "median cpu",
            "extra": "avg cpu: 4.497942048103828, max cpu: 9.29332, count: 55384"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 147.6640625,
            "unit": "median mem",
            "extra": "avg mem: 129.86658446595587, max mem: 152.25, count: 55384"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "153f632ba06057571459a4b6e8767c135baf438c",
          "message": "chore: small diff ported from enterprise for query cancel fix (#3187)",
          "timestamp": "2025-09-18T22:31:35-04:00",
          "tree_id": "2c3b3f692c24ba8540a69da9d41f4d3a24d4ae6f",
          "url": "https://github.com/paradedb/paradedb/commit/153f632ba06057571459a4b6e8767c135baf438c"
        },
        "date": 1758250077396,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.83208013245289, max cpu: 14.486921, count: 55203"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 152.47265625,
            "unit": "median mem",
            "extra": "avg mem: 136.1875628362589, max mem: 152.47265625, count: 55203"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.6169126942210506, max cpu: 9.230769, count: 55203"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 25.2578125,
            "unit": "median mem",
            "extra": "avg mem: 25.19382643662935, max mem: 27.65234375, count: 55203"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.875025765298946, max cpu: 14.4723625, count: 55203"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 154.99609375,
            "unit": "median mem",
            "extra": "avg mem: 138.4478120810916, max mem: 154.99609375, count: 55203"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.614523724823468, max cpu: 9.302325, count: 55203"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 154.64453125,
            "unit": "median mem",
            "extra": "avg mem: 137.95316802302412, max mem: 155.01953125, count: 55203"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.642604329336817, max cpu: 9.486166, count: 110406"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 152.88671875,
            "unit": "median mem",
            "extra": "avg mem: 135.55110558557732, max mem: 155.13671875, count: 110406"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 26524,
            "unit": "median block_count",
            "extra": "avg block_count: 26402.713584406643, max block_count: 52977.0, count: 55203"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 30,
            "unit": "median segment_count",
            "extra": "avg segment_count: 30.077513903229896, max segment_count: 73.0, count: 55203"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.602053030500196, max cpu: 9.448819, count: 55203"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 149.80078125,
            "unit": "median mem",
            "extra": "avg mem: 134.46371029144703, max mem: 158.43359375, count: 55203"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 3.743026085620091, max cpu: 4.743083, count: 55203"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 142.26171875,
            "unit": "median mem",
            "extra": "avg mem: 125.24339044639784, max mem: 149.23828125, count: 55203"
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
          "id": "8101a67174703310a6a1655496fd5296e869901d",
          "message": "fix: Clone an Arc rather than a OnceLock. (#3185)\n\n## What\n\nInvert our use of `OnceLock` to ensure that we clone an\n`Arc<OnceLock<T>>` rather than a `OnceLock<Arc<T>>`.\n\n## Why\n\n`OnceLock` implements `Clone` by cloning its contents to create a\nseparate disconnected copy. If what is desired is \"exactly once\nbehavior\", then cloning the `OnceLock` before it has been computed the\nfirst time will defeat that.\n\nThis change has no impact on benchmarks in this case, but\n`Arc<OnceLock<T>>` matches the intent of this code, and sets a better\nexample for future us.\n\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>",
          "timestamp": "2025-09-19T15:01:21-07:00",
          "tree_id": "de6adf9a09b874a0e133e9cbfeca50d417e6c5bf",
          "url": "https://github.com/paradedb/paradedb/commit/8101a67174703310a6a1655496fd5296e869901d"
        },
        "date": 1758320254858,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.844728767592118, max cpu: 14.356929, count: 55367"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 153.7890625,
            "unit": "median mem",
            "extra": "avg mem: 137.39637307929362, max mem: 153.7890625, count: 55367"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.59670739879973, max cpu: 9.486166, count: 55367"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 25.265625,
            "unit": "median mem",
            "extra": "avg mem: 26.600508877128977, max mem: 30.49609375, count: 55367"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.877853308316837, max cpu: 14.117648, count: 55367"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 153.640625,
            "unit": "median mem",
            "extra": "avg mem: 137.5835198256633, max mem: 154.015625, count: 55367"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.519087783436111, max cpu: 4.7856426, count: 55367"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 154.7734375,
            "unit": "median mem",
            "extra": "avg mem: 137.8588537621688, max mem: 155.16015625, count: 55367"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.643637425553139, max cpu: 9.467456, count: 110734"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 151.8359375,
            "unit": "median mem",
            "extra": "avg mem: 135.54614429341936, max mem: 157.3359375, count: 110734"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 27939,
            "unit": "median block_count",
            "extra": "avg block_count: 27753.752325392383, max block_count: 54524.0, count: 55367"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 30,
            "unit": "median segment_count",
            "extra": "avg segment_count: 29.599707406939153, max segment_count: 71.0, count: 55367"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.552141807673459, max cpu: 4.902962, count: 55367"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 149.85546875,
            "unit": "median mem",
            "extra": "avg mem: 133.57387304542868, max mem: 157.3828125, count: 55367"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.037440554370437, max cpu: 9.619239, count: 55367"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 148.30859375,
            "unit": "median mem",
            "extra": "avg mem: 129.2380803180369, max mem: 151.4375, count: 55367"
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
          "id": "3163a5f3e48d3027585287ce8a63074f70ba1836",
          "message": "perf: Configurable Top N requeries more granularly (#3190)",
          "timestamp": "2025-09-19T21:06:04-04:00",
          "tree_id": "8c74bdf97c37281e4641be0e94b4d464daa5a3ea",
          "url": "https://github.com/paradedb/paradedb/commit/3163a5f3e48d3027585287ce8a63074f70ba1836"
        },
        "date": 1758331339178,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.858622597131398, max cpu: 13.967022, count: 55396"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 153.30859375,
            "unit": "median mem",
            "extra": "avg mem: 137.17199043308634, max mem: 153.30859375, count: 55396"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.611842680752763, max cpu: 9.347614, count: 55396"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 27.6328125,
            "unit": "median mem",
            "extra": "avg mem: 27.249629725634524, max mem: 30.90234375, count: 55396"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.888609239011618, max cpu: 14.4723625, count: 55396"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 154.859375,
            "unit": "median mem",
            "extra": "avg mem: 138.7453276748321, max mem: 154.859375, count: 55396"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.450533764569055, max cpu: 4.729064, count: 55396"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 154.26953125,
            "unit": "median mem",
            "extra": "avg mem: 137.86627334329194, max mem: 154.703125, count: 55396"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.6520739949067655, max cpu: 9.448819, count: 110792"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 153.8828125,
            "unit": "median mem",
            "extra": "avg mem: 136.62376165049147, max mem: 156.1328125, count: 110792"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 27273,
            "unit": "median block_count",
            "extra": "avg block_count: 27303.328850458518, max block_count: 53865.0, count: 55396"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 30,
            "unit": "median segment_count",
            "extra": "avg segment_count: 30.042349628131994, max segment_count: 74.0, count: 55396"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.620429584056166, max cpu: 9.338522, count: 55396"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 153.8359375,
            "unit": "median mem",
            "extra": "avg mem: 136.2688999995487, max mem: 157.5859375, count: 55396"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.013852031250563, max cpu: 9.302325, count: 55396"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 144.47265625,
            "unit": "median mem",
            "extra": "avg mem: 126.73401706576287, max mem: 150.359375, count: 55396"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "f573a31e6704d95d0a62271a23ba47658a1dae06",
          "message": "perf: Configurable Top N requeries more granularly (#3194)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nAllow the retry scale factor and max chunk size to be tuned, which is\nuseful for reducing Top N requeries.\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-09-20T09:26:21-04:00",
          "tree_id": "d4ee2092267660be53cb68f8b760756a5a07ab69",
          "url": "https://github.com/paradedb/paradedb/commit/f573a31e6704d95d0a62271a23ba47658a1dae06"
        },
        "date": 1758375750983,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.848438248403669, max cpu: 14.173229, count: 55345"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 154.04296875,
            "unit": "median mem",
            "extra": "avg mem: 137.44465751761678, max mem: 154.4453125, count: 55345"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.657353409846733, max cpu: 9.402546, count: 55345"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 26.83984375,
            "unit": "median mem",
            "extra": "avg mem: 26.890679064278615, max mem: 31.6015625, count: 55345"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.8717466426143625, max cpu: 14.501511, count: 55345"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 155.1875,
            "unit": "median mem",
            "extra": "avg mem: 138.32529488323246, max mem: 155.1875, count: 55345"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.583066762105478, max cpu: 4.7619047, count: 55345"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 153.59375,
            "unit": "median mem",
            "extra": "avg mem: 137.03999768497607, max mem: 153.98046875, count: 55345"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.741183980760839, max cpu: 9.504951, count: 110690"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 150.11328125,
            "unit": "median mem",
            "extra": "avg mem: 133.88828453196993, max mem: 155.74609375, count: 110690"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 26124,
            "unit": "median block_count",
            "extra": "avg block_count: 26053.45503658867, max block_count: 51902.0, count: 55345"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 30,
            "unit": "median segment_count",
            "extra": "avg segment_count: 30.026994308428947, max segment_count: 75.0, count: 55345"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.571310923777938, max cpu: 9.275363, count: 55345"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 149.21484375,
            "unit": "median mem",
            "extra": "avg mem: 132.9322335322748, max mem: 156.73046875, count: 55345"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.283484421160352, max cpu: 4.733728, count: 55345"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 147.87109375,
            "unit": "median mem",
            "extra": "avg mem: 128.4888438431656, max mem: 152.09765625, count: 55345"
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
          "id": "8b5e7226ce38eaa98ce91906a3f8eb0b72906e66",
          "message": "fix: update tantivy dependency revision (#3192)\n\n## What\n\nUpdate tantivy to\n\nhttps://github.com/paradedb/tantivy/commit/7c6c6fc6ac977382b19ae7fb9fd5b0c53b8f1b58\nwhich fixes a bug that disallowed a segment, during indexing, to real\nthe real memory limit of 4GB.\n\n## Why\n\nWe had a bug in our tantivy fork that wouldn't allow a segment, during\nindexing, to cross over 2GB to reach the actual limit of 4GB.\n\n## How\n\n## Tests",
          "timestamp": "2025-09-22T10:19:44-04:00",
          "tree_id": "89f8538f4216f95c908cb451c1405afaa80946e6",
          "url": "https://github.com/paradedb/paradedb/commit/8b5e7226ce38eaa98ce91906a3f8eb0b72906e66"
        },
        "date": 1758551860918,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.8763503597779065, max cpu: 14.145383, count: 55420"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 152.234375,
            "unit": "median mem",
            "extra": "avg mem: 136.60140968964274, max mem: 152.984375, count: 55420"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.5863997738978455, max cpu: 9.239654, count: 55420"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 25.66015625,
            "unit": "median mem",
            "extra": "avg mem: 26.049695366068207, max mem: 29.66015625, count: 55420"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.858573164980008, max cpu: 14.530776, count: 55420"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 153.4375,
            "unit": "median mem",
            "extra": "avg mem: 137.46606898175298, max mem: 153.4375, count: 55420"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.436778818644092, max cpu: 4.7477746, count: 55420"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 154.53125,
            "unit": "median mem",
            "extra": "avg mem: 138.08085246752074, max mem: 154.53125, count: 55420"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.644935911713358, max cpu: 9.60961, count: 110840"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 153.03515625,
            "unit": "median mem",
            "extra": "avg mem: 136.19103092013262, max mem: 157.4765625, count: 110840"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 26953,
            "unit": "median block_count",
            "extra": "avg block_count: 27058.69220498015, max block_count: 53025.0, count: 55420"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 30,
            "unit": "median segment_count",
            "extra": "avg segment_count: 29.975568386863948, max segment_count: 74.0, count: 55420"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.5778983224401335, max cpu: 4.843592, count: 55420"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 152.73046875,
            "unit": "median mem",
            "extra": "avg mem: 135.04039240120895, max mem: 156.85546875, count: 55420"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 3.9378586978135046, max cpu: 4.8096194, count: 55420"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 147.14453125,
            "unit": "median mem",
            "extra": "avg mem: 128.03643413366115, max mem: 151.38671875, count: 55420"
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
          "id": "7efcca559340008f06df6d1861f1f0301970a0dc",
          "message": "fix: use correct xids when returning to the fsm (#3191)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nThere's a few places where we need to use a future xid when returning\nblocks to the FSM.\n\nSpecically blocks from the segment meta entries list when it's garbage\ncollected.\n\n## Why\n\nTo address some community reports of what appear to be corrupt index\npages.\n\n## How\n\n## Tests",
          "timestamp": "2025-09-22T10:21:41-04:00",
          "tree_id": "a4cdecfcfa42fbe38c487b7c4f6c5cc31eef4f46",
          "url": "https://github.com/paradedb/paradedb/commit/7efcca559340008f06df6d1861f1f0301970a0dc"
        },
        "date": 1758551974444,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.9573809115158625, max cpu: 14.723927, count: 54856"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 155.3828125,
            "unit": "median mem",
            "extra": "avg mem: 138.7322456323146, max mem: 155.3828125, count: 54856"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.56181427169161, max cpu: 9.421001, count: 54856"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 25.3046875,
            "unit": "median mem",
            "extra": "avg mem: 25.94362178077603, max mem: 29.76953125, count: 54856"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.966166815059629, max cpu: 14.799589, count: 54856"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 154.13671875,
            "unit": "median mem",
            "extra": "avg mem: 137.8556089608475, max mem: 154.13671875, count: 54856"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.5975238576835356, max cpu: 9.302325, count: 54856"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 155.87109375,
            "unit": "median mem",
            "extra": "avg mem: 138.8414965147386, max mem: 155.87109375, count: 54856"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.641093304859274, max cpu: 9.687184, count: 109712"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 151.6328125,
            "unit": "median mem",
            "extra": "avg mem: 135.03091617143065, max mem: 157.6328125, count: 109712"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 27591,
            "unit": "median block_count",
            "extra": "avg block_count: 27240.194983228816, max block_count: 52655.0, count: 54856"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 30,
            "unit": "median segment_count",
            "extra": "avg segment_count: 30.00277089106023, max segment_count: 73.0, count: 54856"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.471852578224452, max cpu: 9.230769, count: 54856"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 152.796875,
            "unit": "median mem",
            "extra": "avg mem: 135.79438474426226, max mem: 158.0625, count: 54856"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6153846,
            "unit": "median cpu",
            "extra": "avg cpu: 3.7777341097302237, max cpu: 4.833837, count: 54856"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 146.578125,
            "unit": "median mem",
            "extra": "avg mem: 128.7476542276597, max mem: 150.58984375, count: 54856"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "5f4016019b33da6e3e77ef5d3f8663670394cff1",
          "message": "fix: update tantivy dependency revision (#3198)\n\n## What\n\nUpdate tantivy to\n\nhttps://github.com/paradedb/tantivy/commit/7c6c6fc6ac977382b19ae7fb9fd5b0c53b8f1b58\nwhich fixes a bug that disallowed a segment, during indexing, to real\nthe real memory limit of 4GB.\n\n## Why\n\nWe had a bug in our tantivy fork that wouldn't allow a segment, during\nindexing, to cross over 2GB to reach the actual limit of 4GB.\n\n## How\n\n## Tests\n\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>",
          "timestamp": "2025-09-22T10:36:38-04:00",
          "tree_id": "45c5f689aa0bacc2a9e85d97ac3ccc0047a14590",
          "url": "https://github.com/paradedb/paradedb/commit/5f4016019b33da6e3e77ef5d3f8663670394cff1"
        },
        "date": 1758552865551,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.870234418886268, max cpu: 14.243324, count: 55400"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 153.2109375,
            "unit": "median mem",
            "extra": "avg mem: 137.24727077222474, max mem: 153.5859375, count: 55400"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.606519563677009, max cpu: 9.347614, count: 55400"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 26.4375,
            "unit": "median mem",
            "extra": "avg mem: 26.852044787906138, max mem: 31.50390625, count: 55400"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.868537976821238, max cpu: 13.93998, count: 55400"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 153.08984375,
            "unit": "median mem",
            "extra": "avg mem: 137.31346478734207, max mem: 153.46484375, count: 55400"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.548968363204048, max cpu: 9.411765, count: 55400"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 155.6640625,
            "unit": "median mem",
            "extra": "avg mem: 139.4512591662906, max mem: 156.4140625, count: 55400"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.671291090560933, max cpu: 9.467456, count: 110800"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 153.6171875,
            "unit": "median mem",
            "extra": "avg mem: 136.83475310948782, max mem: 156.6328125, count: 110800"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 27490,
            "unit": "median block_count",
            "extra": "avg block_count: 27054.99774368231, max block_count: 51385.0, count: 55400"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 30,
            "unit": "median segment_count",
            "extra": "avg segment_count: 30.011841155234656, max segment_count: 75.0, count: 55400"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.601612919943056, max cpu: 9.266409, count: 55400"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 153.44921875,
            "unit": "median mem",
            "extra": "avg mem: 136.7786074994359, max mem: 158.36328125, count: 55400"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.004266828150818, max cpu: 4.8096194, count: 55400"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 146.671875,
            "unit": "median mem",
            "extra": "avg mem: 128.77548581340253, max mem: 152.125, count: 55400"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "414e16a1fd61ad461377aedc953534abc28d76a4",
          "message": "fix: use correct xids when returning to the fsm (#3199)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nThere's a few places where we need to use a future xid when returning\nblocks to the FSM.\n\nSpecically blocks from the segment meta entries list when it's garbage\ncollected.\n\n## Why\n\nTo address some community reports of what appear to be corrupt index\npages.\n\n## How\n\n## Tests\n\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>",
          "timestamp": "2025-09-22T10:38:03-04:00",
          "tree_id": "7f77ddfdfc23d53ffdb8a2b66caf5806c3a67939",
          "url": "https://github.com/paradedb/paradedb/commit/414e16a1fd61ad461377aedc953534abc28d76a4"
        },
        "date": 1758552984443,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.9443319911952655, max cpu: 14.45783, count: 55366"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 153.98046875,
            "unit": "median mem",
            "extra": "avg mem: 137.44737865127064, max mem: 153.98046875, count: 55366"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.544087840427325, max cpu: 9.29332, count: 55366"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 25.58203125,
            "unit": "median mem",
            "extra": "avg mem: 25.979833559291805, max mem: 28.3671875, count: 55366"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.925637657275549, max cpu: 15.559156, count: 55366"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 152.640625,
            "unit": "median mem",
            "extra": "avg mem: 136.6667111387178, max mem: 152.640625, count: 55366"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.613572531700935, max cpu: 4.738401, count: 55366"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 154.765625,
            "unit": "median mem",
            "extra": "avg mem: 137.69021538207113, max mem: 154.765625, count: 55366"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.6547553436312565, max cpu: 9.667674, count: 110732"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 152.62109375,
            "unit": "median mem",
            "extra": "avg mem: 135.14416017529936, max mem: 156.0, count: 110732"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 26701,
            "unit": "median block_count",
            "extra": "avg block_count: 27022.155420294042, max block_count: 53283.0, count: 55366"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 30,
            "unit": "median segment_count",
            "extra": "avg segment_count: 29.98087273778131, max segment_count: 73.0, count: 55366"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.557414816521631, max cpu: 4.833837, count: 55366"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 148.453125,
            "unit": "median mem",
            "extra": "avg mem: 133.92427310420655, max mem: 157.10546875, count: 55366"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 3.786946722312727, max cpu: 9.311348, count: 55366"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 146.7109375,
            "unit": "median mem",
            "extra": "avg mem: 127.46194408289836, max mem: 151.1328125, count: 55366"
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
          "id": "da997100cd0e2873fa8692ec6c2382761719ce58",
          "message": "chore: Upgrade to `0.18.2` (#3144) (#3145)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-09-10T17:44:33-04:00",
          "tree_id": "d0d9fd4cb9ebc554c1e7f3e029694e863f4247c9",
          "url": "https://github.com/paradedb/paradedb/commit/da997100cd0e2873fa8692ec6c2382761719ce58"
        },
        "date": 1757542436050,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.466630982022334,
            "unit": "median tps",
            "extra": "avg tps: 7.235960162567525, max tps: 11.266839553996212, count: 57932"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.316855698673045,
            "unit": "median tps",
            "extra": "avg tps: 4.812278058497864, max tps: 5.89220997396712, count: 57932"
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
          "id": "1cfaa7b311ca8b7ee91491411c8dfecd2ce5619c",
          "message": "fix: `GROUP BY` doesn't panic when Postgres eliminates group pathkeys (#3152)\n\n# Ticket(s) Closed\n\n- Closes #3050 \n\n## What\n\nIt's possible for Postgres to eliminate group pathkeys if it realizes\nthat one of the pathkeys is unique, making the other ones unnecessary.\n\nWe need to handle this case/not panic.\n\n## Why\n\nSee issue.\n\n## How\n\nInject the dropped group pathkeys back into our list of grouping\ncolumns.\n\n## Tests\n\nAdded regression test",
          "timestamp": "2025-09-14T17:56:19-04:00",
          "tree_id": "a41824569d62cfd5dbe40884e6ead540d3b1bd88",
          "url": "https://github.com/paradedb/paradedb/commit/1cfaa7b311ca8b7ee91491411c8dfecd2ce5619c"
        },
        "date": 1757888650715,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.566089790031889,
            "unit": "median tps",
            "extra": "avg tps: 7.325540152417709, max tps: 11.449210198558303, count: 57538"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.353641849881095,
            "unit": "median tps",
            "extra": "avg tps: 4.839541471600935, max tps: 5.9207229733359075, count: 57538"
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
          "id": "a521487756693e82c46bfe2f1a2f2fd3aded0136",
          "message": "fix: fixed `rt_fetch out-of-bounds` error (#3141)\n\n# Ticket(s) Closed\n\n- Closes #3135\n\n## What\n\nFixed `rt_fetch used out-of-bounds` and `Cannot open relation with\noid=0` errors that occurred in complex SQL queries with nested `OR\nEXISTS` clauses, multiple `JOIN`s.\n\n## Why\n\nThe issue occurred when PostgreSQL's query planner generated `Var` nodes\nreferencing Range Table Entries (RTEs) that were valid in outer planning\ncontexts but didn't exist in inner execution contexts. This happened\nspecifically with:\n- `OR EXISTS` subqueries (not `AND EXISTS`)  \n- Multiple `JOIN`s within the `EXISTS` clause\n- ParadeDB functions applied to joined tables\n\nWhen ParadeDB's custom scan tried to access these out-of-bounds RTEs\nusing `rt_fetch`, it caused crashes.\n\n## How\n\nImplemented bounds checking across the codebase:\n\n1. **Early detection**: Added bounds checking in `find_var_relation()`\nto detect invalid `varno` values and return `pg_sys::InvalidOid`. This\nwas the main fix for the issue.\n2. **Graceful handling**: Modified all functions that receive relation\nOIDs to check for `InvalidOid` before attempting to open relations\n3. **Safe fallbacks**: Updated query optimization logic to skip\noptimizations when relation information is unavailable rather than\ncrashing\n\n## Tests\n\nAdded regression test `or_exists_join_bug.sql` covering:\n- Simple queries (baseline functionality)\n- `AND EXISTS` with multiple `JOIN`s (should work)  \n- `OR EXISTS` with multiple `JOIN`s (the problematic case, now fixed)\n- Various edge cases and workarounds\n- Minimal reproduction cases\n\n---------\n\nSigned-off-by: Moe <mdashti@gmail.com>",
          "timestamp": "2025-09-15T02:47:52-07:00",
          "tree_id": "4a0b5db116e0263111295cc53d05810e093ce68c",
          "url": "https://github.com/paradedb/paradedb/commit/a521487756693e82c46bfe2f1a2f2fd3aded0136"
        },
        "date": 1757931340218,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.152844131748818,
            "unit": "median tps",
            "extra": "avg tps: 6.972836825219883, max tps: 10.829340533965318, count: 57898"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.581100423892432,
            "unit": "median tps",
            "extra": "avg tps: 5.035969500406348, max tps: 6.209944908958627, count: 57898"
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
          "id": "b37fc5af676e3438c051381414d81996ed0fb8f6",
          "message": "feat: push down `group by ... order by ... limit` (#3134)\n\n# Ticket(s) Closed\n\n- Closes #3131 \n- Opens #3156 #3155 \n\n## What\n\nPushes down `group by ... order by ... limit` to Tantivy\n\n## Why\n\nBy pushing down the sort/limit to Tantivy, we can significantly speed up\n`group by` queries over high cardinality columns.\n\n## How\n\n- Before we were hard-coding a bucket size and sorting the results\nourselves, now the bucket size is set to the limit and we push the sort\ndown to the Tantivy term agg\n\n## Tests",
          "timestamp": "2025-09-15T15:51:50-04:00",
          "tree_id": "e58df02d60abc13101aaae8ef6333a9afafbcd78",
          "url": "https://github.com/paradedb/paradedb/commit/b37fc5af676e3438c051381414d81996ed0fb8f6"
        },
        "date": 1757967581653,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.254349026155197,
            "unit": "median tps",
            "extra": "avg tps: 7.068501254146793, max tps: 10.986653416512787, count: 57321"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.356946186656411,
            "unit": "median tps",
            "extra": "avg tps: 4.842967001126219, max tps: 5.925605252964595, count: 57321"
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
          "id": "8211eef7a0dd34237afebfa91364fb66c65a4906",
          "message": "perf: remove `ExactBuffer` in favor of a regular rust `BufWriter` (#3158)\n\n# Ticket(s) Closed\n\n- Closes #2981  (/cc @yjhjstz)\n\nWhile #2981 wasn't the impetus for this, it addresses the complaint made\nthere just the same.\n\n## What\n\nIn profiling, our `ExactBuffer` was a large percentage of certain\nprofiles. This replaces it, and its complexity, with a standard Rust\n`BufWriter`.\n\n## Why\n\nImproves performance of (at least) our `wide-table.toml` test's \"Single\nUpdate\" job by quite a bit.\n\n<img width=\"720\" height=\"141\" alt=\"screenshot_2025-09-15_at_3 28\n33___pm_720\"\nsrc=\"https://github.com/user-attachments/assets/a373a7ae-df38-4691-980a-d6843f073d26\"\n/>\n\n\n## How\n\n## Tests\n\nExisting tests pass",
          "timestamp": "2025-09-15T15:55:52-04:00",
          "tree_id": "4ddf140542c5525034023441aadac4b634c90fc6",
          "url": "https://github.com/paradedb/paradedb/commit/8211eef7a0dd34237afebfa91364fb66c65a4906"
        },
        "date": 1757967820433,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.505780913594247,
            "unit": "median tps",
            "extra": "avg tps: 7.268743597415695, max tps: 11.349043037151878, count: 57546"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.376231436943641,
            "unit": "median tps",
            "extra": "avg tps: 4.866571434388452, max tps: 5.9533269334931, count: 57546"
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
          "id": "288a4bfa0c79838d86711b8a6231687c984ac0b5",
          "message": "chore: Upgrade to `0.18.3` (#3160)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-09-15T16:13:06-04:00",
          "tree_id": "ad59a6c86e8afe29cabad5b0bcc6a78bc448182e",
          "url": "https://github.com/paradedb/paradedb/commit/288a4bfa0c79838d86711b8a6231687c984ac0b5"
        },
        "date": 1757968847999,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.593806154937717,
            "unit": "median tps",
            "extra": "avg tps: 7.354015124297552, max tps: 11.471254655356777, count: 57465"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.374387141759855,
            "unit": "median tps",
            "extra": "avg tps: 4.860966127679511, max tps: 5.945750579439071, count: 57465"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "af5bea23effe976b411147e259e53afad947a393",
          "message": "perf: remove `ExactBuffer` in favor of a regular rust `BufWriter` (#3159)\n\n# Ticket(s) Closed\n\n- Closes #2981  (/cc @yjhjstz)\n\nWhile #2981 wasn't the impetus for this, it addresses the complaint made\nthere just the same.\n\n## What\n\nIn profiling, our `ExactBuffer` was a large percentage of certain\nprofiles. This replaces it, and its complexity, with a standard Rust\n`BufWriter`.\n\n## Why\n\nImproves performance of (at least) our `wide-table.toml` test's \"Single\nUpdate\" job by quite a bit.\n\n<img width=\"720\" height=\"141\" alt=\"screenshot_2025-09-15_at_3 28\n33___pm_720\"\nsrc=\"https://github.com/user-attachments/assets/a373a7ae-df38-4691-980a-d6843f073d26\"\n/>\n\n\n## How\n\n## Tests\n\nExisting tests pass\n\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>",
          "timestamp": "2025-09-15T16:26:19-04:00",
          "tree_id": "cbc00b9a93c129255360f60e5a70904e87f1e8c1",
          "url": "https://github.com/paradedb/paradedb/commit/af5bea23effe976b411147e259e53afad947a393"
        },
        "date": 1757969672335,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.339723531854597,
            "unit": "median tps",
            "extra": "avg tps: 7.143001956492063, max tps: 11.183058809935165, count: 57547"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.380226127682563,
            "unit": "median tps",
            "extra": "avg tps: 4.858418992247911, max tps: 5.949064416315608, count: 57547"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "7800d096e107acdbdec6297d0cb98ef030569e2b",
          "message": "chore: Upgrade to `0.18.3` (#3161)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-09-15T16:36:57-04:00",
          "tree_id": "c0962cc02d5690156721fd003c985f724ee9b20f",
          "url": "https://github.com/paradedb/paradedb/commit/7800d096e107acdbdec6297d0cb98ef030569e2b"
        },
        "date": 1757970326934,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.280278806494794,
            "unit": "median tps",
            "extra": "avg tps: 7.0654856651398354, max tps: 10.965445597358238, count: 57819"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.451946846433827,
            "unit": "median tps",
            "extra": "avg tps: 4.905883746005768, max tps: 6.100303018074733, count: 57819"
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
          "id": "f71a5572d645d23e58b949cc3f16645473c74735",
          "message": "chore: Sync `0.18.x` (#3162)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\n---------\n\nSigned-off-by: Moe <mdashti@gmail.com>\nCo-authored-by: Moe <mdashti@gmail.com>",
          "timestamp": "2025-09-15T17:11:39-04:00",
          "tree_id": "a75daf7f281149ef4317505338649d8b0d2ec8a4",
          "url": "https://github.com/paradedb/paradedb/commit/f71a5572d645d23e58b949cc3f16645473c74735"
        },
        "date": 1757972375069,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.377787265189449,
            "unit": "median tps",
            "extra": "avg tps: 7.150097709510416, max tps: 11.120328166346232, count: 57913"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.3691171322752025,
            "unit": "median tps",
            "extra": "avg tps: 4.833662049645503, max tps: 5.997957291802274, count: 57913"
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
          "id": "878c50feef96d61785ad711ebe46250c920bed70",
          "message": "fix: sequential scan segfault (#3163)\n\n# Ticket(s) Closed\n\n- Closes #3151 \n\n## What\n\nThe `@@@` return type should be `bool`, not `SearchQueryInput`.\n\n## Why\n\n## How\n\n## Tests\n\nAdded regression test.",
          "timestamp": "2025-09-16T10:27:13-04:00",
          "tree_id": "6859469869310b79c8c32af68b3ed77dfb787362",
          "url": "https://github.com/paradedb/paradedb/commit/878c50feef96d61785ad711ebe46250c920bed70"
        },
        "date": 1758034510639,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.416875082958079,
            "unit": "median tps",
            "extra": "avg tps: 7.184133378822356, max tps: 11.214125628925341, count: 57326"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.48267321443019,
            "unit": "median tps",
            "extra": "avg tps: 4.957614929086508, max tps: 6.0797712605609915, count: 57326"
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
          "id": "f2a0c9c43e4385628cc7b828a8ed12c30e55050e",
          "message": "chore: don't warn about `raw` tokenizer on UUID key fields (#3166)\n\n## What\n\nRemove the warning about using the `raw` tokenizer when the `key_field`\nis a UUID field.\n\nThe drama here is that we (pg_search) assign the `raw` tokenizer to UUID\nfields used as the key_field so there's nothing a user can do about it.\nWarning about our own bad decisions is not cool.\n\n## Why\n\nMany user and customer complaints.\n\n## How\n\n## Tests\n\nExisting tests pass but a couple of the regression test expected output\nis now different.",
          "timestamp": "2025-09-16T13:10:47-04:00",
          "tree_id": "2b24aea6e3a0645c584d8ebb8ce7465c8c90f904",
          "url": "https://github.com/paradedb/paradedb/commit/f2a0c9c43e4385628cc7b828a8ed12c30e55050e"
        },
        "date": 1758044325413,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.403449452706825,
            "unit": "median tps",
            "extra": "avg tps: 7.159291246498125, max tps: 11.125077637052089, count: 57782"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.423501363533868,
            "unit": "median tps",
            "extra": "avg tps: 4.910939270009515, max tps: 6.043305385902253, count: 57782"
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
          "id": "489eb48583040067612195f9e1406d5e31a1599f",
          "message": "perf: teach custom scan callback to exit early if it can (#3168)\n\n## What\n\nThis does two things.  \n\nOne, the first commit (62d752572b2d7bc5a02b7203ac2c83949e38e27e) simply\nreorders some code in the custom scan callback so it can decide to exit\nearly if we're not going to submit a path. Specifically, this is\nintended to avoid opening a Directory and Index and related structures.\n\nTwo, the second commit (5ac1dde23ef0809bea4b942d04fd14acc9d1c152) makes\na new decision to not evaluate possible pushdown predicates when the\nstatement type is not a SELECT statement. This cuts out the overhead of\nneeding to read/deserialize the index's schema at all on (at least)\nUPDATE statements.\n\nThis does mean that we won't consider doing pushdowns for UPDATE\nstatements, even if doing one would make the UPDATE scan faster.\n\n## Why\n\nTrying to reduce per-query overhead, targeting our stressgres benchmarks\nlike \"single-server.toml\" and \"wide-table.toml\".\n\n## How\n\n## Tests\n\nAll existing tests pass.",
          "timestamp": "2025-09-16T17:39:51-04:00",
          "tree_id": "0ebcd01c6225cbb43b199470f7f78bd694493ed7",
          "url": "https://github.com/paradedb/paradedb/commit/489eb48583040067612195f9e1406d5e31a1599f"
        },
        "date": 1758060461591,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.350612443072883,
            "unit": "median tps",
            "extra": "avg tps: 7.1372907535498324, max tps: 11.122249685434666, count: 57582"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.430722813933305,
            "unit": "median tps",
            "extra": "avg tps: 4.904515655349498, max tps: 6.033848801649281, count: 57582"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "63daa7f2bf568127e538f19f942d6363508ca615",
          "message": "chore: don't warn about `raw` tokenizer on UUID key fields (#3167)\n\n## What\n\nRemove the warning about using the `raw` tokenizer when the `key_field`\nis a UUID field.\n\nThe drama here is that we (pg_search) assign the `raw` tokenizer to UUID\nfields used as the key_field so there's nothing a user can do about it.\nWarning about our own bad decisions is not cool.\n\n## Why\n\nMany user and customer complaints.\n\n## How\n\n## Tests\n\nExisting tests pass but a couple of the regression test expected output\nis now different.\n\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>",
          "timestamp": "2025-09-17T10:06:31-04:00",
          "tree_id": "2c472616485a1c2a1ed61c7f2c030286882deb06",
          "url": "https://github.com/paradedb/paradedb/commit/63daa7f2bf568127e538f19f942d6363508ca615"
        },
        "date": 1758119671854,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.491470385346853,
            "unit": "median tps",
            "extra": "avg tps: 7.253411400493452, max tps: 11.318928396554517, count: 57466"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.33597122528871,
            "unit": "median tps",
            "extra": "avg tps: 4.820532029214345, max tps: 5.935444092020346, count: 57466"
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
          "id": "eb456f8c97d99e92e2795d88dd2c1082c13c83a6",
          "message": "perf: optimize `Timestamp` and `JsonB` datum decoding (#3171)\n\n## What\n\nOptimize `Timestamp` and `JsonB` to `TantivyValue` datum conversions.\n\nThese two show up quite high in profiles. The `JsonB` conversion in\nparticular has been bad due to how pgrx stupidly (I can say it) handles\n`JsonB` values by converting them to strings and then asking serde to\nparse the strings.\n\n## Why\n\nTrying to make things faster.\n\n## How\n\nFor the `Timestamp` conversion we memoize Postgres' understanding of the\ncurrent EPOCH and do the same math that it does to calculate a time\nvalue.\n\nFor the `JsonB` conversion we implement our own deserializer routine\nusing Postgres' internal `JsonbIteratorInit()` and `JsonbIteratorNext()`\nfunctions, building up a `serde_json::Value` structure as it goes.\n\n\n## Tests\n\nA new `#[pg_test]`-based proptest has been added to test our custom\njsonb deserializer against normal serde.\n\n---------\n\nSigned-off-by: Eric Ridge <eebbrr@gmail.com>\nCo-authored-by: Stu Hood <stuhood@paradedb.com>",
          "timestamp": "2025-09-17T15:26:06-04:00",
          "tree_id": "702cea735a514e9b33d6c1ee785606d39d4f705c",
          "url": "https://github.com/paradedb/paradedb/commit/eb456f8c97d99e92e2795d88dd2c1082c13c83a6"
        },
        "date": 1758138922249,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 7.997238472057587,
            "unit": "median tps",
            "extra": "avg tps: 6.833538823498622, max tps: 10.629087515885976, count: 57797"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.49438767999381,
            "unit": "median tps",
            "extra": "avg tps: 4.9479874971234565, max tps: 6.10650468212832, count: 57797"
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
          "id": "849076799ca599dfbf0f2415149b12495b24624c",
          "message": "fix: Always use a shared expression context to execute HeapFilters. (#3174)\n\n## What\n\nEvaluating heap filters was leaking a `CreateStandaloneExprContext` per\nexecution, which could eventually lead to OOMs.\n\n## Why\n\n`PgBox::from_pg` does not free the resulting memory: it's expected to be\nfreed by a memory context, since PG created it.\n\n## How\n\nAlways use a global expression context created by\n`ExecAssignExprContext` in `begin_custom_scan`.\n\n## Tests\n\nA repro uses significantly less memory, and [a benchmark query added for\nthis purpose](https://github.com/paradedb/paradedb/pull/3175) is faster.",
          "timestamp": "2025-09-17T16:44:32-07:00",
          "tree_id": "7eef1c518a935389aa23e91c6bc47bbc325b18e6",
          "url": "https://github.com/paradedb/paradedb/commit/849076799ca599dfbf0f2415149b12495b24624c"
        },
        "date": 1758154358109,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.34702636497707,
            "unit": "median tps",
            "extra": "avg tps: 7.129305693830275, max tps: 11.062926916051657, count: 57490"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.435481611094656,
            "unit": "median tps",
            "extra": "avg tps: 4.90806791865233, max tps: 6.054756369667337, count: 57490"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "dc0e87d6fdabdb573d6f37a24c6c33befa43e8b0",
          "message": "fix: Always use a shared expression context to execute HeapFilters. (#3176)\n\n## What\n\nEvaluating heap filters was leaking a `CreateStandaloneExprContext` per\nexecution, which could eventually lead to OOMs.\n\n## Why\n\n`PgBox::from_pg` does not free the resulting memory: it's expected to be\nfreed by a memory context, since PG created it.\n\n## How\n\nAlways use a global expression context created by\n`ExecAssignExprContext` in `begin_custom_scan`.\n\n## Tests\n\nA repro uses significantly less memory, and [a benchmark query added for\nthis purpose](https://github.com/paradedb/paradedb/pull/3175) is faster.\n\nCo-authored-by: Stu Hood <stuhood@paradedb.com>",
          "timestamp": "2025-09-17T17:11:43-07:00",
          "tree_id": "0c30f446ad8404b4f66727777f1b6e6a5bc8958e",
          "url": "https://github.com/paradedb/paradedb/commit/dc0e87d6fdabdb573d6f37a24c6c33befa43e8b0"
        },
        "date": 1758155993850,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.260655979932775,
            "unit": "median tps",
            "extra": "avg tps: 7.078101988113657, max tps: 11.018866045157377, count: 57762"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.441603860708489,
            "unit": "median tps",
            "extra": "avg tps: 4.920656528954137, max tps: 6.0600331384635195, count: 57762"
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
          "id": "3bcb1451087be74b7bd73bfc7d6546423046a0ce",
          "message": "fix: write all delete files atomically (#3178)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nIn `ambulkdelete`, write all delete files to the meta list atomically,\ninstead of one at a time.\n\n## Why\n\nReduces the number of writes to the metas list.\n\n## How\n\n## Tests",
          "timestamp": "2025-09-18T16:03:10-04:00",
          "tree_id": "ad9609f0419a34b8f0cf543e911c1dc3c25d4563",
          "url": "https://github.com/paradedb/paradedb/commit/3bcb1451087be74b7bd73bfc7d6546423046a0ce"
        },
        "date": 1758227482974,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 7.9710120537427755,
            "unit": "median tps",
            "extra": "avg tps: 6.821520368275454, max tps: 10.653046011324387, count: 57882"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.499582999104259,
            "unit": "median tps",
            "extra": "avg tps: 4.970809495910068, max tps: 6.100146880844462, count: 57882"
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
          "id": "6e11875ba052ccd6937ca0c535b3803309c8b6eb",
          "message": "feat: removed aggregation limitations re mix of aggregate functions and aggregation on group-by column. (#3179)\n\n# Ticket(s) Closed\n\n- Closes #2963\n\n## What\n\nRemoves aggregate limitations that prevented queries where the same\nfield is used in both `GROUP BY` and aggregate functions (e.g., `SELECT\nrating, AVG(rating) FROM table GROUP BY rating`).\n\n## Why\n\nPrevious safety checks blocked these queries due to Tantivy's\n\"incompatible fruit types\" errors, but testing shows the underlying\nissue is resolved. The limitations were overly restrictive and caused\nunnecessary fallbacks to slower PostgreSQL aggregation.\n\n## How\n\n- Removed `has_search_field_conflicts` function and field conflict\nvalidation\n- Eliminated ~35 lines of restrictive code in\n`extract_and_validate_aggregates`\n- Previously blocked queries now use faster `AggregateScan` instead of\n`GroupAggregate`\n\n## Tests\n\n- **`aggregate-groupby-conflict.sql`** - Tests `GROUP BY field` with\naggregates on same field\n- **`test-fruit-types-issue.sql`** - Validates #2963 issue resolution  \n- **`groupby_aggregate.out`** - Updated expectations showing\n`AggregateScan` usage",
          "timestamp": "2025-09-18T16:00:25-07:00",
          "tree_id": "f85924512f419186b824a986dd35eaa96d973884",
          "url": "https://github.com/paradedb/paradedb/commit/6e11875ba052ccd6937ca0c535b3803309c8b6eb"
        },
        "date": 1758238111551,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.26112343217718,
            "unit": "median tps",
            "extra": "avg tps: 7.055789437238797, max tps: 10.959271372995877, count: 57965"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.433154161699186,
            "unit": "median tps",
            "extra": "avg tps: 4.906473493725666, max tps: 6.030736684806794, count: 57965"
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
          "id": "020f92b742187fe6fdc75a19390692e6d2e9a373",
          "message": "feat: Port `ambulkdelete_epoch` to community (#3180)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nAdd `ambulkdelete_epoch` to the index metadata and make sure it's passed\ndown to all scans. In enterprise, this value is used to detect query\nconflicts with concurrent vacuums on the primary.\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-09-18T19:16:09-04:00",
          "tree_id": "3642b293b38caa7676318f888b910c3f934e1976",
          "url": "https://github.com/paradedb/paradedb/commit/020f92b742187fe6fdc75a19390692e6d2e9a373"
        },
        "date": 1758239071148,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.455449694573945,
            "unit": "median tps",
            "extra": "avg tps: 7.220979651508518, max tps: 11.20815972663723, count: 57329"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.325307900647886,
            "unit": "median tps",
            "extra": "avg tps: 4.815375490773559, max tps: 5.910384785730325, count: 57329"
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
          "id": "c0dc0e674f3524c2f65a7b387ccbf594b23e5e0e",
          "message": "chore: Upgrade to `0.18.4` (#3181)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\n---------\n\nSigned-off-by: Ming <ming.ying.nyc@gmail.com>\nCo-authored-by: Stu Hood <stuhood@paradedb.com>",
          "timestamp": "2025-09-18T19:18:34-04:00",
          "tree_id": "b67f22553ed7786ef556afbfad2b7f8ddc6b139e",
          "url": "https://github.com/paradedb/paradedb/commit/c0dc0e674f3524c2f65a7b387ccbf594b23e5e0e"
        },
        "date": 1758239312809,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.486775140245504,
            "unit": "median tps",
            "extra": "avg tps: 7.236022410995005, max tps: 11.21681025366527, count: 57781"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.290319500321394,
            "unit": "median tps",
            "extra": "avg tps: 4.79655257054257, max tps: 5.8850429158935, count: 57781"
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
          "id": "a97c525d0c11314fe01c8bbb250ea5fdf73ec8ce",
          "message": "fix: write all delete files atomically (#3178) (#3182)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nIn `ambulkdelete`, write all delete files to the meta list atomically,\ninstead of one at a time.\n\n## Why\n\nReduces the number of writes to the metas list.\n\n## How\n\n## Tests\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-09-18T21:50:00-04:00",
          "tree_id": "ba5917ed034f24a8e2ad95a64751e5faef3d55d5",
          "url": "https://github.com/paradedb/paradedb/commit/a97c525d0c11314fe01c8bbb250ea5fdf73ec8ce"
        },
        "date": 1758248286407,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.47896006161148,
            "unit": "median tps",
            "extra": "avg tps: 7.247518027409807, max tps: 11.324757579349736, count: 57766"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.204393574686115,
            "unit": "median tps",
            "extra": "avg tps: 4.709720480629928, max tps: 5.729398851381262, count: 57766"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "e15e51abfc4b7834faea068d861d91d5d873580f",
          "message": "chore: Upgrade to `0.18.4` (#3184)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\nSigned-off-by: Ming <ming.ying.nyc@gmail.com>\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>\nCo-authored-by: Stu Hood <stuhood@paradedb.com>",
          "timestamp": "2025-09-18T21:52:13-04:00",
          "tree_id": "3d203e3468a4e7504d03af9c39ac9a0869033086",
          "url": "https://github.com/paradedb/paradedb/commit/e15e51abfc4b7834faea068d861d91d5d873580f"
        },
        "date": 1758248475802,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.592648592891226,
            "unit": "median tps",
            "extra": "avg tps: 7.3409674014039155, max tps: 11.450471250469944, count: 57790"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.272805790729678,
            "unit": "median tps",
            "extra": "avg tps: 4.777170192342345, max tps: 5.82993025778322, count: 57790"
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
          "id": "1046018b2db9614ef172bd802c98a3987da7513e",
          "message": "chore: small diff ported from enterprise for query cancel fix (#3186)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nSome small changes in enterprise that should be in community\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-09-18T21:53:42-04:00",
          "tree_id": "85ed1f4eb7261157deabdfba479dc61164775f99",
          "url": "https://github.com/paradedb/paradedb/commit/1046018b2db9614ef172bd802c98a3987da7513e"
        },
        "date": 1758248509577,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.559399004009657,
            "unit": "median tps",
            "extra": "avg tps: 7.293085470774937, max tps: 11.363488230973717, count: 57331"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.355068383367619,
            "unit": "median tps",
            "extra": "avg tps: 4.858077769581307, max tps: 5.947132276845599, count: 57331"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "f052aabf25719cee68a756a379c6b66e39452759",
          "message": "feat: Port `ambulkdelete_epoch` to community (#3183)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nAdd `ambulkdelete_epoch` to the index metadata and make sure it's passed\ndown to all scans. In enterprise, this value is used to detect query\nconflicts with concurrent vacuums on the primary.\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-09-18T22:01:15-04:00",
          "tree_id": "48ffae94b2f43d5c2d62b5adb846d1dcc2992aee",
          "url": "https://github.com/paradedb/paradedb/commit/f052aabf25719cee68a756a379c6b66e39452759"
        },
        "date": 1758248961628,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.481172057453366,
            "unit": "median tps",
            "extra": "avg tps: 7.245129630860569, max tps: 11.306546885863673, count: 57551"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.300693169837988,
            "unit": "median tps",
            "extra": "avg tps: 4.80102882445662, max tps: 5.8623228557837646, count: 57551"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "153f632ba06057571459a4b6e8767c135baf438c",
          "message": "chore: small diff ported from enterprise for query cancel fix (#3187)",
          "timestamp": "2025-09-18T22:31:35-04:00",
          "tree_id": "2c3b3f692c24ba8540a69da9d41f4d3a24d4ae6f",
          "url": "https://github.com/paradedb/paradedb/commit/153f632ba06057571459a4b6e8767c135baf438c"
        },
        "date": 1758250788253,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.368137352886034,
            "unit": "median tps",
            "extra": "avg tps: 7.147871379230123, max tps: 11.153937559865112, count: 57320"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.365049283616284,
            "unit": "median tps",
            "extra": "avg tps: 4.856610334783065, max tps: 5.9559297022817494, count: 57320"
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
          "id": "8101a67174703310a6a1655496fd5296e869901d",
          "message": "fix: Clone an Arc rather than a OnceLock. (#3185)\n\n## What\n\nInvert our use of `OnceLock` to ensure that we clone an\n`Arc<OnceLock<T>>` rather than a `OnceLock<Arc<T>>`.\n\n## Why\n\n`OnceLock` implements `Clone` by cloning its contents to create a\nseparate disconnected copy. If what is desired is \"exactly once\nbehavior\", then cloning the `OnceLock` before it has been computed the\nfirst time will defeat that.\n\nThis change has no impact on benchmarks in this case, but\n`Arc<OnceLock<T>>` matches the intent of this code, and sets a better\nexample for future us.\n\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>",
          "timestamp": "2025-09-19T15:01:21-07:00",
          "tree_id": "de6adf9a09b874a0e133e9cbfeca50d417e6c5bf",
          "url": "https://github.com/paradedb/paradedb/commit/8101a67174703310a6a1655496fd5296e869901d"
        },
        "date": 1758320978441,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.572866843560712,
            "unit": "median tps",
            "extra": "avg tps: 7.323050234414194, max tps: 11.426028912953587, count: 57801"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.4184939753733925,
            "unit": "median tps",
            "extra": "avg tps: 4.901685016159776, max tps: 5.9881136085025455, count: 57801"
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
          "id": "3163a5f3e48d3027585287ce8a63074f70ba1836",
          "message": "perf: Configurable Top N requeries more granularly (#3190)",
          "timestamp": "2025-09-19T21:06:04-04:00",
          "tree_id": "8c74bdf97c37281e4641be0e94b4d464daa5a3ea",
          "url": "https://github.com/paradedb/paradedb/commit/3163a5f3e48d3027585287ce8a63074f70ba1836"
        },
        "date": 1758332062127,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.454378313024492,
            "unit": "median tps",
            "extra": "avg tps: 7.230273498099597, max tps: 11.27148989391017, count: 57910"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.39295219475513,
            "unit": "median tps",
            "extra": "avg tps: 4.882237916701781, max tps: 5.973633924552263, count: 57910"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "f573a31e6704d95d0a62271a23ba47658a1dae06",
          "message": "perf: Configurable Top N requeries more granularly (#3194)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nAllow the retry scale factor and max chunk size to be tuned, which is\nuseful for reducing Top N requeries.\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-09-20T09:26:21-04:00",
          "tree_id": "d4ee2092267660be53cb68f8b760756a5a07ab69",
          "url": "https://github.com/paradedb/paradedb/commit/f573a31e6704d95d0a62271a23ba47658a1dae06"
        },
        "date": 1758376468417,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.373370368941451,
            "unit": "median tps",
            "extra": "avg tps: 7.157455886491669, max tps: 11.160219415371383, count: 57946"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.387271665451007,
            "unit": "median tps",
            "extra": "avg tps: 4.868567970757335, max tps: 5.977213272629829, count: 57946"
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
          "id": "8b5e7226ce38eaa98ce91906a3f8eb0b72906e66",
          "message": "fix: update tantivy dependency revision (#3192)\n\n## What\n\nUpdate tantivy to\n\nhttps://github.com/paradedb/tantivy/commit/7c6c6fc6ac977382b19ae7fb9fd5b0c53b8f1b58\nwhich fixes a bug that disallowed a segment, during indexing, to real\nthe real memory limit of 4GB.\n\n## Why\n\nWe had a bug in our tantivy fork that wouldn't allow a segment, during\nindexing, to cross over 2GB to reach the actual limit of 4GB.\n\n## How\n\n## Tests",
          "timestamp": "2025-09-22T10:19:44-04:00",
          "tree_id": "89f8538f4216f95c908cb451c1405afaa80946e6",
          "url": "https://github.com/paradedb/paradedb/commit/8b5e7226ce38eaa98ce91906a3f8eb0b72906e66"
        },
        "date": 1758552581554,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.479330950606695,
            "unit": "median tps",
            "extra": "avg tps: 7.231219546657223, max tps: 11.256405356574138, count: 57344"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.374762165762079,
            "unit": "median tps",
            "extra": "avg tps: 4.863768955714802, max tps: 5.953715152993979, count: 57344"
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
          "id": "7efcca559340008f06df6d1861f1f0301970a0dc",
          "message": "fix: use correct xids when returning to the fsm (#3191)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nThere's a few places where we need to use a future xid when returning\nblocks to the FSM.\n\nSpecically blocks from the segment meta entries list when it's garbage\ncollected.\n\n## Why\n\nTo address some community reports of what appear to be corrupt index\npages.\n\n## How\n\n## Tests",
          "timestamp": "2025-09-22T10:21:41-04:00",
          "tree_id": "a4cdecfcfa42fbe38c487b7c4f6c5cc31eef4f46",
          "url": "https://github.com/paradedb/paradedb/commit/7efcca559340008f06df6d1861f1f0301970a0dc"
        },
        "date": 1758552702658,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.256143837132669,
            "unit": "median tps",
            "extra": "avg tps: 7.06516892048871, max tps: 11.036185768984621, count: 57588"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.41567443207037,
            "unit": "median tps",
            "extra": "avg tps: 4.890071032285965, max tps: 6.00005372655396, count: 57588"
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
          "id": "da997100cd0e2873fa8692ec6c2382761719ce58",
          "message": "chore: Upgrade to `0.18.2` (#3144) (#3145)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-09-10T17:44:33-04:00",
          "tree_id": "d0d9fd4cb9ebc554c1e7f3e029694e863f4247c9",
          "url": "https://github.com/paradedb/paradedb/commit/da997100cd0e2873fa8692ec6c2382761719ce58"
        },
        "date": 1757542438538,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.731707,
            "unit": "median cpu",
            "extra": "avg cpu: 19.433778711504154, max cpu: 42.105263, count: 57932"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 230.0625,
            "unit": "median mem",
            "extra": "avg mem: 230.3217531033151, max mem: 231.3515625, count: 57932"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.30097,
            "unit": "median cpu",
            "extra": "avg cpu: 22.359054719525734, max cpu: 33.20158, count: 57932"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 161.36328125,
            "unit": "median mem",
            "extra": "avg mem: 161.48835312845233, max mem: 162.671875, count: 57932"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 24316,
            "unit": "median block_count",
            "extra": "avg block_count: 23124.452271628805, max block_count: 26078.0, count: 57932"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 71,
            "unit": "median segment_count",
            "extra": "avg segment_count: 73.08663605606573, max segment_count: 108.0, count: 57932"
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
          "id": "1cfaa7b311ca8b7ee91491411c8dfecd2ce5619c",
          "message": "fix: `GROUP BY` doesn't panic when Postgres eliminates group pathkeys (#3152)\n\n# Ticket(s) Closed\n\n- Closes #3050 \n\n## What\n\nIt's possible for Postgres to eliminate group pathkeys if it realizes\nthat one of the pathkeys is unique, making the other ones unnecessary.\n\nWe need to handle this case/not panic.\n\n## Why\n\nSee issue.\n\n## How\n\nInject the dropped group pathkeys back into our list of grouping\ncolumns.\n\n## Tests\n\nAdded regression test",
          "timestamp": "2025-09-14T17:56:19-04:00",
          "tree_id": "a41824569d62cfd5dbe40884e6ead540d3b1bd88",
          "url": "https://github.com/paradedb/paradedb/commit/1cfaa7b311ca8b7ee91491411c8dfecd2ce5619c"
        },
        "date": 1757888653201,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.622696,
            "unit": "median cpu",
            "extra": "avg cpu: 19.044698999680225, max cpu: 42.436146, count: 57538"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 230.6640625,
            "unit": "median mem",
            "extra": "avg mem: 230.3417119447148, max mem: 233.1875, count: 57538"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.27837,
            "unit": "median cpu",
            "extra": "avg cpu: 22.316380309836063, max cpu: 33.333336, count: 57538"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 160.5234375,
            "unit": "median mem",
            "extra": "avg mem: 160.78393979848101, max mem: 163.3984375, count: 57538"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 24407,
            "unit": "median block_count",
            "extra": "avg block_count: 23127.570753241336, max block_count: 26169.0, count: 57538"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 71,
            "unit": "median segment_count",
            "extra": "avg segment_count: 73.34844798220307, max segment_count: 108.0, count: 57538"
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
          "id": "a521487756693e82c46bfe2f1a2f2fd3aded0136",
          "message": "fix: fixed `rt_fetch out-of-bounds` error (#3141)\n\n# Ticket(s) Closed\n\n- Closes #3135\n\n## What\n\nFixed `rt_fetch used out-of-bounds` and `Cannot open relation with\noid=0` errors that occurred in complex SQL queries with nested `OR\nEXISTS` clauses, multiple `JOIN`s.\n\n## Why\n\nThe issue occurred when PostgreSQL's query planner generated `Var` nodes\nreferencing Range Table Entries (RTEs) that were valid in outer planning\ncontexts but didn't exist in inner execution contexts. This happened\nspecifically with:\n- `OR EXISTS` subqueries (not `AND EXISTS`)  \n- Multiple `JOIN`s within the `EXISTS` clause\n- ParadeDB functions applied to joined tables\n\nWhen ParadeDB's custom scan tried to access these out-of-bounds RTEs\nusing `rt_fetch`, it caused crashes.\n\n## How\n\nImplemented bounds checking across the codebase:\n\n1. **Early detection**: Added bounds checking in `find_var_relation()`\nto detect invalid `varno` values and return `pg_sys::InvalidOid`. This\nwas the main fix for the issue.\n2. **Graceful handling**: Modified all functions that receive relation\nOIDs to check for `InvalidOid` before attempting to open relations\n3. **Safe fallbacks**: Updated query optimization logic to skip\noptimizations when relation information is unavailable rather than\ncrashing\n\n## Tests\n\nAdded regression test `or_exists_join_bug.sql` covering:\n- Simple queries (baseline functionality)\n- `AND EXISTS` with multiple `JOIN`s (should work)  \n- `OR EXISTS` with multiple `JOIN`s (the problematic case, now fixed)\n- Various edge cases and workarounds\n- Minimal reproduction cases\n\n---------\n\nSigned-off-by: Moe <mdashti@gmail.com>",
          "timestamp": "2025-09-15T02:47:52-07:00",
          "tree_id": "4a0b5db116e0263111295cc53d05810e093ce68c",
          "url": "https://github.com/paradedb/paradedb/commit/a521487756693e82c46bfe2f1a2f2fd3aded0136"
        },
        "date": 1757931342738,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 19.028742,
            "unit": "median cpu",
            "extra": "avg cpu: 19.873728535788707, max cpu: 42.72997, count: 57898"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 230.4765625,
            "unit": "median mem",
            "extra": "avg mem: 230.2874957630229, max mem: 232.7265625, count: 57898"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.323614,
            "unit": "median cpu",
            "extra": "avg cpu: 22.396449032213006, max cpu: 33.267326, count: 57898"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 162.42578125,
            "unit": "median mem",
            "extra": "avg mem: 162.3680008241865, max mem: 163.92578125, count: 57898"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 23848,
            "unit": "median block_count",
            "extra": "avg block_count: 22826.815537669696, max block_count: 25645.0, count: 57898"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 70,
            "unit": "median segment_count",
            "extra": "avg segment_count: 72.28013057445852, max segment_count: 107.0, count: 57898"
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
          "id": "b37fc5af676e3438c051381414d81996ed0fb8f6",
          "message": "feat: push down `group by ... order by ... limit` (#3134)\n\n# Ticket(s) Closed\n\n- Closes #3131 \n- Opens #3156 #3155 \n\n## What\n\nPushes down `group by ... order by ... limit` to Tantivy\n\n## Why\n\nBy pushing down the sort/limit to Tantivy, we can significantly speed up\n`group by` queries over high cardinality columns.\n\n## How\n\n- Before we were hard-coding a bucket size and sorting the results\nourselves, now the bucket size is set to the limit and we push the sort\ndown to the Tantivy term agg\n\n## Tests",
          "timestamp": "2025-09-15T15:51:50-04:00",
          "tree_id": "e58df02d60abc13101aaae8ef6333a9afafbcd78",
          "url": "https://github.com/paradedb/paradedb/commit/b37fc5af676e3438c051381414d81996ed0fb8f6"
        },
        "date": 1757967584344,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.86051,
            "unit": "median cpu",
            "extra": "avg cpu: 19.63327443584652, max cpu: 42.857143, count: 57321"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 231.51953125,
            "unit": "median mem",
            "extra": "avg mem: 231.3231928661616, max mem: 233.0078125, count: 57321"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.27837,
            "unit": "median cpu",
            "extra": "avg cpu: 22.4776531136998, max cpu: 33.432835, count: 57321"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 158.6796875,
            "unit": "median mem",
            "extra": "avg mem: 158.42077081534254, max mem: 161.98046875, count: 57321"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 24010,
            "unit": "median block_count",
            "extra": "avg block_count: 22908.10488302716, max block_count: 25942.0, count: 57321"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 71,
            "unit": "median segment_count",
            "extra": "avg segment_count: 72.61799340555817, max segment_count: 107.0, count: 57321"
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
          "id": "8211eef7a0dd34237afebfa91364fb66c65a4906",
          "message": "perf: remove `ExactBuffer` in favor of a regular rust `BufWriter` (#3158)\n\n# Ticket(s) Closed\n\n- Closes #2981  (/cc @yjhjstz)\n\nWhile #2981 wasn't the impetus for this, it addresses the complaint made\nthere just the same.\n\n## What\n\nIn profiling, our `ExactBuffer` was a large percentage of certain\nprofiles. This replaces it, and its complexity, with a standard Rust\n`BufWriter`.\n\n## Why\n\nImproves performance of (at least) our `wide-table.toml` test's \"Single\nUpdate\" job by quite a bit.\n\n<img width=\"720\" height=\"141\" alt=\"screenshot_2025-09-15_at_3 28\n33___pm_720\"\nsrc=\"https://github.com/user-attachments/assets/a373a7ae-df38-4691-980a-d6843f073d26\"\n/>\n\n\n## How\n\n## Tests\n\nExisting tests pass",
          "timestamp": "2025-09-15T15:55:52-04:00",
          "tree_id": "4ddf140542c5525034023441aadac4b634c90fc6",
          "url": "https://github.com/paradedb/paradedb/commit/8211eef7a0dd34237afebfa91364fb66c65a4906"
        },
        "date": 1757967823031,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.677044,
            "unit": "median cpu",
            "extra": "avg cpu: 19.159818446081964, max cpu: 42.687748, count: 57546"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 233.01953125,
            "unit": "median mem",
            "extra": "avg mem: 230.8696391392321, max mem: 236.46484375, count: 57546"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.30097,
            "unit": "median cpu",
            "extra": "avg cpu: 22.48369651186787, max cpu: 33.300297, count: 57546"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 164.31640625,
            "unit": "median mem",
            "extra": "avg mem: 162.4528762180473, max mem: 167.796875, count: 57546"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 24294,
            "unit": "median block_count",
            "extra": "avg block_count: 23129.672227435443, max block_count: 26114.0, count: 57546"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 71,
            "unit": "median segment_count",
            "extra": "avg segment_count: 72.98224029472074, max segment_count: 109.0, count: 57546"
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
          "id": "288a4bfa0c79838d86711b8a6231687c984ac0b5",
          "message": "chore: Upgrade to `0.18.3` (#3160)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-09-15T16:13:06-04:00",
          "tree_id": "ad59a6c86e8afe29cabad5b0bcc6a78bc448182e",
          "url": "https://github.com/paradedb/paradedb/commit/288a4bfa0c79838d86711b8a6231687c984ac0b5"
        },
        "date": 1757968850612,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.640776,
            "unit": "median cpu",
            "extra": "avg cpu: 18.99675884622466, max cpu: 42.899704, count: 57465"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 227.5859375,
            "unit": "median mem",
            "extra": "avg mem: 227.34934205875317, max mem: 228.09375, count: 57465"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.30097,
            "unit": "median cpu",
            "extra": "avg cpu: 22.463291343281753, max cpu: 33.267326, count: 57465"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 163.12890625,
            "unit": "median mem",
            "extra": "avg mem: 162.65236665796573, max mem: 167.703125, count: 57465"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 24337,
            "unit": "median block_count",
            "extra": "avg block_count: 23091.912485860958, max block_count: 26146.0, count: 57465"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 71,
            "unit": "median segment_count",
            "extra": "avg segment_count: 73.26847646393458, max segment_count: 108.0, count: 57465"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "af5bea23effe976b411147e259e53afad947a393",
          "message": "perf: remove `ExactBuffer` in favor of a regular rust `BufWriter` (#3159)\n\n# Ticket(s) Closed\n\n- Closes #2981  (/cc @yjhjstz)\n\nWhile #2981 wasn't the impetus for this, it addresses the complaint made\nthere just the same.\n\n## What\n\nIn profiling, our `ExactBuffer` was a large percentage of certain\nprofiles. This replaces it, and its complexity, with a standard Rust\n`BufWriter`.\n\n## Why\n\nImproves performance of (at least) our `wide-table.toml` test's \"Single\nUpdate\" job by quite a bit.\n\n<img width=\"720\" height=\"141\" alt=\"screenshot_2025-09-15_at_3 28\n33___pm_720\"\nsrc=\"https://github.com/user-attachments/assets/a373a7ae-df38-4691-980a-d6843f073d26\"\n/>\n\n\n## How\n\n## Tests\n\nExisting tests pass\n\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>",
          "timestamp": "2025-09-15T16:26:19-04:00",
          "tree_id": "cbc00b9a93c129255360f60e5a70904e87f1e8c1",
          "url": "https://github.com/paradedb/paradedb/commit/af5bea23effe976b411147e259e53afad947a393"
        },
        "date": 1757969674952,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.640776,
            "unit": "median cpu",
            "extra": "avg cpu: 19.146202775262218, max cpu: 42.064266, count: 57547"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 229.37109375,
            "unit": "median mem",
            "extra": "avg mem: 228.70742549296662, max mem: 232.33984375, count: 57547"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.27837,
            "unit": "median cpu",
            "extra": "avg cpu: 22.487249513209278, max cpu: 33.23442, count: 57547"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 162.19140625,
            "unit": "median mem",
            "extra": "avg mem: 162.07629259833266, max mem: 163.7421875, count: 57547"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 24248,
            "unit": "median block_count",
            "extra": "avg block_count: 23008.704589292232, max block_count: 26061.0, count: 57547"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 71,
            "unit": "median segment_count",
            "extra": "avg segment_count: 72.7096807826646, max segment_count: 106.0, count: 57547"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "7800d096e107acdbdec6297d0cb98ef030569e2b",
          "message": "chore: Upgrade to `0.18.3` (#3161)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-09-15T16:36:57-04:00",
          "tree_id": "c0962cc02d5690156721fd003c985f724ee9b20f",
          "url": "https://github.com/paradedb/paradedb/commit/7800d096e107acdbdec6297d0cb98ef030569e2b"
        },
        "date": 1757970329571,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.897638,
            "unit": "median cpu",
            "extra": "avg cpu: 19.550922514557215, max cpu: 43.59233, count: 57819"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 226.83203125,
            "unit": "median mem",
            "extra": "avg mem: 226.58179006090126, max mem: 228.32421875, count: 57819"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.323614,
            "unit": "median cpu",
            "extra": "avg cpu: 22.39025136207082, max cpu: 33.3996, count: 57819"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 161.609375,
            "unit": "median mem",
            "extra": "avg mem: 161.5885537599016, max mem: 163.3515625, count: 57819"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 24057,
            "unit": "median block_count",
            "extra": "avg block_count: 22933.06553209153, max block_count: 25760.0, count: 57819"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 71,
            "unit": "median segment_count",
            "extra": "avg segment_count: 72.46149189712725, max segment_count: 106.0, count: 57819"
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
          "id": "f71a5572d645d23e58b949cc3f16645473c74735",
          "message": "chore: Sync `0.18.x` (#3162)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\n---------\n\nSigned-off-by: Moe <mdashti@gmail.com>\nCo-authored-by: Moe <mdashti@gmail.com>",
          "timestamp": "2025-09-15T17:11:39-04:00",
          "tree_id": "a75daf7f281149ef4317505338649d8b0d2ec8a4",
          "url": "https://github.com/paradedb/paradedb/commit/f71a5572d645d23e58b949cc3f16645473c74735"
        },
        "date": 1757972377818,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.82353,
            "unit": "median cpu",
            "extra": "avg cpu: 19.58987414998271, max cpu: 42.519684, count: 57913"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 226.9375,
            "unit": "median mem",
            "extra": "avg mem: 226.75315600016404, max mem: 228.44921875, count: 57913"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.233301,
            "unit": "median cpu",
            "extra": "avg cpu: 22.339039880359813, max cpu: 33.23442, count: 57913"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 161.40625,
            "unit": "median mem",
            "extra": "avg mem: 161.69270041916323, max mem: 164.3046875, count: 57913"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 24187,
            "unit": "median block_count",
            "extra": "avg block_count: 23046.988983475214, max block_count: 25972.0, count: 57913"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 71,
            "unit": "median segment_count",
            "extra": "avg segment_count: 72.69386838879008, max segment_count: 106.0, count: 57913"
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
          "id": "878c50feef96d61785ad711ebe46250c920bed70",
          "message": "fix: sequential scan segfault (#3163)\n\n# Ticket(s) Closed\n\n- Closes #3151 \n\n## What\n\nThe `@@@` return type should be `bool`, not `SearchQueryInput`.\n\n## Why\n\n## How\n\n## Tests\n\nAdded regression test.",
          "timestamp": "2025-09-16T10:27:13-04:00",
          "tree_id": "6859469869310b79c8c32af68b3ed77dfb787362",
          "url": "https://github.com/paradedb/paradedb/commit/878c50feef96d61785ad711ebe46250c920bed70"
        },
        "date": 1758034513730,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.79405,
            "unit": "median cpu",
            "extra": "avg cpu: 19.43965205984045, max cpu: 42.772278, count: 57326"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 226.41796875,
            "unit": "median mem",
            "extra": "avg mem: 225.86540036643495, max mem: 227.93359375, count: 57326"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.30097,
            "unit": "median cpu",
            "extra": "avg cpu: 22.36221795438288, max cpu: 33.20158, count: 57326"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 161.8125,
            "unit": "median mem",
            "extra": "avg mem: 161.75509258451663, max mem: 163.31640625, count: 57326"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 24200,
            "unit": "median block_count",
            "extra": "avg block_count: 23084.727645396506, max block_count: 26111.0, count: 57326"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 71,
            "unit": "median segment_count",
            "extra": "avg segment_count: 72.89985346962983, max segment_count: 108.0, count: 57326"
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
          "id": "f2a0c9c43e4385628cc7b828a8ed12c30e55050e",
          "message": "chore: don't warn about `raw` tokenizer on UUID key fields (#3166)\n\n## What\n\nRemove the warning about using the `raw` tokenizer when the `key_field`\nis a UUID field.\n\nThe drama here is that we (pg_search) assign the `raw` tokenizer to UUID\nfields used as the key_field so there's nothing a user can do about it.\nWarning about our own bad decisions is not cool.\n\n## Why\n\nMany user and customer complaints.\n\n## How\n\n## Tests\n\nExisting tests pass but a couple of the regression test expected output\nis now different.",
          "timestamp": "2025-09-16T13:10:47-04:00",
          "tree_id": "2b24aea6e3a0645c584d8ebb8ce7465c8c90f904",
          "url": "https://github.com/paradedb/paradedb/commit/f2a0c9c43e4385628cc7b828a8ed12c30e55050e"
        },
        "date": 1758044328039,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.786694,
            "unit": "median cpu",
            "extra": "avg cpu: 19.295756604792555, max cpu: 42.64561, count: 57782"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 227.8359375,
            "unit": "median mem",
            "extra": "avg mem: 227.22508293564604, max mem: 229.9765625, count: 57782"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.323614,
            "unit": "median cpu",
            "extra": "avg cpu: 22.305023630372585, max cpu: 33.366436, count: 57782"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 162.328125,
            "unit": "median mem",
            "extra": "avg mem: 162.12171779544235, max mem: 163.48046875, count: 57782"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 24421,
            "unit": "median block_count",
            "extra": "avg block_count: 23072.45368799972, max block_count: 25971.0, count: 57782"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 71,
            "unit": "median segment_count",
            "extra": "avg segment_count: 72.63817451801599, max segment_count: 107.0, count: 57782"
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
          "id": "489eb48583040067612195f9e1406d5e31a1599f",
          "message": "perf: teach custom scan callback to exit early if it can (#3168)\n\n## What\n\nThis does two things.  \n\nOne, the first commit (62d752572b2d7bc5a02b7203ac2c83949e38e27e) simply\nreorders some code in the custom scan callback so it can decide to exit\nearly if we're not going to submit a path. Specifically, this is\nintended to avoid opening a Directory and Index and related structures.\n\nTwo, the second commit (5ac1dde23ef0809bea4b942d04fd14acc9d1c152) makes\na new decision to not evaluate possible pushdown predicates when the\nstatement type is not a SELECT statement. This cuts out the overhead of\nneeding to read/deserialize the index's schema at all on (at least)\nUPDATE statements.\n\nThis does mean that we won't consider doing pushdowns for UPDATE\nstatements, even if doing one would make the UPDATE scan faster.\n\n## Why\n\nTrying to reduce per-query overhead, targeting our stressgres benchmarks\nlike \"single-server.toml\" and \"wide-table.toml\".\n\n## How\n\n## Tests\n\nAll existing tests pass.",
          "timestamp": "2025-09-16T17:39:51-04:00",
          "tree_id": "0ebcd01c6225cbb43b199470f7f78bd694493ed7",
          "url": "https://github.com/paradedb/paradedb/commit/489eb48583040067612195f9e1406d5e31a1599f"
        },
        "date": 1758060464209,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.842003,
            "unit": "median cpu",
            "extra": "avg cpu: 19.44328226473803, max cpu: 42.857143, count: 57582"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 225.8046875,
            "unit": "median mem",
            "extra": "avg mem: 225.42321941427008, max mem: 229.37109375, count: 57582"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.30097,
            "unit": "median cpu",
            "extra": "avg cpu: 22.339219698783868, max cpu: 33.300297, count: 57582"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 162.234375,
            "unit": "median mem",
            "extra": "avg mem: 162.3148476710387, max mem: 163.8515625, count: 57582"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 24144,
            "unit": "median block_count",
            "extra": "avg block_count: 22986.621148970164, max block_count: 25794.0, count: 57582"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 71,
            "unit": "median segment_count",
            "extra": "avg segment_count: 72.54484040151436, max segment_count: 105.0, count: 57582"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "63daa7f2bf568127e538f19f942d6363508ca615",
          "message": "chore: don't warn about `raw` tokenizer on UUID key fields (#3167)\n\n## What\n\nRemove the warning about using the `raw` tokenizer when the `key_field`\nis a UUID field.\n\nThe drama here is that we (pg_search) assign the `raw` tokenizer to UUID\nfields used as the key_field so there's nothing a user can do about it.\nWarning about our own bad decisions is not cool.\n\n## Why\n\nMany user and customer complaints.\n\n## How\n\n## Tests\n\nExisting tests pass but a couple of the regression test expected output\nis now different.\n\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>",
          "timestamp": "2025-09-17T10:06:31-04:00",
          "tree_id": "2c472616485a1c2a1ed61c7f2c030286882deb06",
          "url": "https://github.com/paradedb/paradedb/commit/63daa7f2bf568127e538f19f942d6363508ca615"
        },
        "date": 1758119674526,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.640776,
            "unit": "median cpu",
            "extra": "avg cpu: 19.10898600702473, max cpu: 42.64561, count: 57466"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 226.37109375,
            "unit": "median mem",
            "extra": "avg mem: 226.00273442937998, max mem: 228.2265625, count: 57466"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.255816,
            "unit": "median cpu",
            "extra": "avg cpu: 22.418841541946243, max cpu: 33.300297, count: 57466"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 159.12109375,
            "unit": "median mem",
            "extra": "avg mem: 158.627745917151, max mem: 162.52734375, count: 57466"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 24190,
            "unit": "median block_count",
            "extra": "avg block_count: 23050.875630807783, max block_count: 26008.0, count: 57466"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 71,
            "unit": "median segment_count",
            "extra": "avg segment_count: 72.84333344934396, max segment_count: 106.0, count: 57466"
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
          "id": "eb456f8c97d99e92e2795d88dd2c1082c13c83a6",
          "message": "perf: optimize `Timestamp` and `JsonB` datum decoding (#3171)\n\n## What\n\nOptimize `Timestamp` and `JsonB` to `TantivyValue` datum conversions.\n\nThese two show up quite high in profiles. The `JsonB` conversion in\nparticular has been bad due to how pgrx stupidly (I can say it) handles\n`JsonB` values by converting them to strings and then asking serde to\nparse the strings.\n\n## Why\n\nTrying to make things faster.\n\n## How\n\nFor the `Timestamp` conversion we memoize Postgres' understanding of the\ncurrent EPOCH and do the same math that it does to calculate a time\nvalue.\n\nFor the `JsonB` conversion we implement our own deserializer routine\nusing Postgres' internal `JsonbIteratorInit()` and `JsonbIteratorNext()`\nfunctions, building up a `serde_json::Value` structure as it goes.\n\n\n## Tests\n\nA new `#[pg_test]`-based proptest has been added to test our custom\njsonb deserializer against normal serde.\n\n---------\n\nSigned-off-by: Eric Ridge <eebbrr@gmail.com>\nCo-authored-by: Stu Hood <stuhood@paradedb.com>",
          "timestamp": "2025-09-17T15:26:06-04:00",
          "tree_id": "702cea735a514e9b33d6c1ee785606d39d4f705c",
          "url": "https://github.com/paradedb/paradedb/commit/eb456f8c97d99e92e2795d88dd2c1082c13c83a6"
        },
        "date": 1758138924978,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.010548,
            "unit": "median cpu",
            "extra": "avg cpu: 19.919785698209218, max cpu: 42.814667, count: 57797"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 225.87109375,
            "unit": "median mem",
            "extra": "avg mem: 225.27184396465213, max mem: 227.421875, count: 57797"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.27837,
            "unit": "median cpu",
            "extra": "avg cpu: 22.457745470786772, max cpu: 33.267326, count: 57797"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 162.3359375,
            "unit": "median mem",
            "extra": "avg mem: 162.29050376479316, max mem: 164.51953125, count: 57797"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 23768,
            "unit": "median block_count",
            "extra": "avg block_count: 22713.127117324428, max block_count: 25407.0, count: 57797"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 70,
            "unit": "median segment_count",
            "extra": "avg segment_count: 71.80703150682561, max segment_count: 104.0, count: 57797"
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
          "id": "849076799ca599dfbf0f2415149b12495b24624c",
          "message": "fix: Always use a shared expression context to execute HeapFilters. (#3174)\n\n## What\n\nEvaluating heap filters was leaking a `CreateStandaloneExprContext` per\nexecution, which could eventually lead to OOMs.\n\n## Why\n\n`PgBox::from_pg` does not free the resulting memory: it's expected to be\nfreed by a memory context, since PG created it.\n\n## How\n\nAlways use a global expression context created by\n`ExecAssignExprContext` in `begin_custom_scan`.\n\n## Tests\n\nA repro uses significantly less memory, and [a benchmark query added for\nthis purpose](https://github.com/paradedb/paradedb/pull/3175) is faster.",
          "timestamp": "2025-09-17T16:44:32-07:00",
          "tree_id": "7eef1c518a935389aa23e91c6bc47bbc325b18e6",
          "url": "https://github.com/paradedb/paradedb/commit/849076799ca599dfbf0f2415149b12495b24624c"
        },
        "date": 1758154360681,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.75,
            "unit": "median cpu",
            "extra": "avg cpu: 19.34121978197039, max cpu: 42.60355, count: 57490"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 226.3046875,
            "unit": "median mem",
            "extra": "avg mem: 225.613440177096, max mem: 227.86328125, count: 57490"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.27837,
            "unit": "median cpu",
            "extra": "avg cpu: 22.42017493662807, max cpu: 33.267326, count: 57490"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 161.63671875,
            "unit": "median mem",
            "extra": "avg mem: 161.62385476006696, max mem: 163.13671875, count: 57490"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 24021,
            "unit": "median block_count",
            "extra": "avg block_count: 22915.575908853712, max block_count: 25753.0, count: 57490"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 71,
            "unit": "median segment_count",
            "extra": "avg segment_count: 72.51902939641677, max segment_count: 105.0, count: 57490"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "dc0e87d6fdabdb573d6f37a24c6c33befa43e8b0",
          "message": "fix: Always use a shared expression context to execute HeapFilters. (#3176)\n\n## What\n\nEvaluating heap filters was leaking a `CreateStandaloneExprContext` per\nexecution, which could eventually lead to OOMs.\n\n## Why\n\n`PgBox::from_pg` does not free the resulting memory: it's expected to be\nfreed by a memory context, since PG created it.\n\n## How\n\nAlways use a global expression context created by\n`ExecAssignExprContext` in `begin_custom_scan`.\n\n## Tests\n\nA repro uses significantly less memory, and [a benchmark query added for\nthis purpose](https://github.com/paradedb/paradedb/pull/3175) is faster.\n\nCo-authored-by: Stu Hood <stuhood@paradedb.com>",
          "timestamp": "2025-09-17T17:11:43-07:00",
          "tree_id": "0c30f446ad8404b4f66727777f1b6e6a5bc8958e",
          "url": "https://github.com/paradedb/paradedb/commit/dc0e87d6fdabdb573d6f37a24c6c33befa43e8b0"
        },
        "date": 1758155996488,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.897638,
            "unit": "median cpu",
            "extra": "avg cpu: 19.570101903166552, max cpu: 42.899704, count: 57762"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 230.60546875,
            "unit": "median mem",
            "extra": "avg mem: 229.65830774341262, max mem: 232.51953125, count: 57762"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.27837,
            "unit": "median cpu",
            "extra": "avg cpu: 22.43874385948832, max cpu: 33.20158, count: 57762"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 159.20703125,
            "unit": "median mem",
            "extra": "avg mem: 158.91773895467608, max mem: 162.01171875, count: 57762"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 24216,
            "unit": "median block_count",
            "extra": "avg block_count: 22973.451490599356, max block_count: 25726.0, count: 57762"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 70,
            "unit": "median segment_count",
            "extra": "avg segment_count: 72.39067206814168, max segment_count: 107.0, count: 57762"
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
          "id": "3bcb1451087be74b7bd73bfc7d6546423046a0ce",
          "message": "fix: write all delete files atomically (#3178)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nIn `ambulkdelete`, write all delete files to the meta list atomically,\ninstead of one at a time.\n\n## Why\n\nReduces the number of writes to the metas list.\n\n## How\n\n## Tests",
          "timestamp": "2025-09-18T16:03:10-04:00",
          "tree_id": "ad9609f0419a34b8f0cf543e911c1dc3c25d4563",
          "url": "https://github.com/paradedb/paradedb/commit/3bcb1451087be74b7bd73bfc7d6546423046a0ce"
        },
        "date": 1758227485803,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 22.623722,
            "unit": "median cpu",
            "extra": "avg cpu: 20.0225459708206, max cpu: 42.942345, count: 57882"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 227.390625,
            "unit": "median mem",
            "extra": "avg mem: 226.62063916135415, max mem: 229.00390625, count: 57882"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.30097,
            "unit": "median cpu",
            "extra": "avg cpu: 22.326251388743636, max cpu: 33.168808, count: 57882"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 160.5859375,
            "unit": "median mem",
            "extra": "avg mem: 160.85732948606648, max mem: 162.65625, count: 57882"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 23874,
            "unit": "median block_count",
            "extra": "avg block_count: 22787.244134618708, max block_count: 25615.0, count: 57882"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 70,
            "unit": "median segment_count",
            "extra": "avg segment_count: 72.0050447462078, max segment_count: 105.0, count: 57882"
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
          "id": "6e11875ba052ccd6937ca0c535b3803309c8b6eb",
          "message": "feat: removed aggregation limitations re mix of aggregate functions and aggregation on group-by column. (#3179)\n\n# Ticket(s) Closed\n\n- Closes #2963\n\n## What\n\nRemoves aggregate limitations that prevented queries where the same\nfield is used in both `GROUP BY` and aggregate functions (e.g., `SELECT\nrating, AVG(rating) FROM table GROUP BY rating`).\n\n## Why\n\nPrevious safety checks blocked these queries due to Tantivy's\n\"incompatible fruit types\" errors, but testing shows the underlying\nissue is resolved. The limitations were overly restrictive and caused\nunnecessary fallbacks to slower PostgreSQL aggregation.\n\n## How\n\n- Removed `has_search_field_conflicts` function and field conflict\nvalidation\n- Eliminated ~35 lines of restrictive code in\n`extract_and_validate_aggregates`\n- Previously blocked queries now use faster `AggregateScan` instead of\n`GroupAggregate`\n\n## Tests\n\n- **`aggregate-groupby-conflict.sql`** - Tests `GROUP BY field` with\naggregates on same field\n- **`test-fruit-types-issue.sql`** - Validates #2963 issue resolution  \n- **`groupby_aggregate.out`** - Updated expectations showing\n`AggregateScan` usage",
          "timestamp": "2025-09-18T16:00:25-07:00",
          "tree_id": "f85924512f419186b824a986dd35eaa96d973884",
          "url": "https://github.com/paradedb/paradedb/commit/6e11875ba052ccd6937ca0c535b3803309c8b6eb"
        },
        "date": 1758238114223,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.897638,
            "unit": "median cpu",
            "extra": "avg cpu: 19.76614080738189, max cpu: 42.772278, count: 57965"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 225.08984375,
            "unit": "median mem",
            "extra": "avg mem: 224.62886244662727, max mem: 226.64453125, count: 57965"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.255816,
            "unit": "median cpu",
            "extra": "avg cpu: 22.28088770180644, max cpu: 33.7011, count: 57965"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 163.87109375,
            "unit": "median mem",
            "extra": "avg mem: 162.77770846362029, max mem: 165.26171875, count: 57965"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 23970,
            "unit": "median block_count",
            "extra": "avg block_count: 22921.125575778486, max block_count: 25736.0, count: 57965"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 71,
            "unit": "median segment_count",
            "extra": "avg segment_count: 72.51565599930993, max segment_count: 105.0, count: 57965"
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
          "id": "020f92b742187fe6fdc75a19390692e6d2e9a373",
          "message": "feat: Port `ambulkdelete_epoch` to community (#3180)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nAdd `ambulkdelete_epoch` to the index metadata and make sure it's passed\ndown to all scans. In enterprise, this value is used to detect query\nconflicts with concurrent vacuums on the primary.\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-09-18T19:16:09-04:00",
          "tree_id": "3642b293b38caa7676318f888b910c3f934e1976",
          "url": "https://github.com/paradedb/paradedb/commit/020f92b742187fe6fdc75a19390692e6d2e9a373"
        },
        "date": 1758239073934,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.75,
            "unit": "median cpu",
            "extra": "avg cpu: 19.43178195500857, max cpu: 42.687748, count: 57329"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 225.78125,
            "unit": "median mem",
            "extra": "avg mem: 225.3670505437911, max mem: 227.4375, count: 57329"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.27837,
            "unit": "median cpu",
            "extra": "avg cpu: 22.369155595593817, max cpu: 33.267326, count: 57329"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 161.828125,
            "unit": "median mem",
            "extra": "avg mem: 161.689824780543, max mem: 163.23828125, count: 57329"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 24284,
            "unit": "median block_count",
            "extra": "avg block_count: 23072.730206352804, max block_count: 25965.0, count: 57329"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 71,
            "unit": "median segment_count",
            "extra": "avg segment_count: 72.88621814439463, max segment_count: 108.0, count: 57329"
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
          "id": "c0dc0e674f3524c2f65a7b387ccbf594b23e5e0e",
          "message": "chore: Upgrade to `0.18.4` (#3181)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\n---------\n\nSigned-off-by: Ming <ming.ying.nyc@gmail.com>\nCo-authored-by: Stu Hood <stuhood@paradedb.com>",
          "timestamp": "2025-09-18T19:18:34-04:00",
          "tree_id": "b67f22553ed7786ef556afbfad2b7f8ddc6b139e",
          "url": "https://github.com/paradedb/paradedb/commit/c0dc0e674f3524c2f65a7b387ccbf594b23e5e0e"
        },
        "date": 1758239315528,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.677044,
            "unit": "median cpu",
            "extra": "avg cpu: 19.32787712126821, max cpu: 42.64561, count: 57781"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 226.375,
            "unit": "median mem",
            "extra": "avg mem: 225.81550805790397, max mem: 228.046875, count: 57781"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.27837,
            "unit": "median cpu",
            "extra": "avg cpu: 22.433499849915933, max cpu: 33.168808, count: 57781"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 164.3125,
            "unit": "median mem",
            "extra": "avg mem: 163.7188191593041, max mem: 165.3984375, count: 57781"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 24276,
            "unit": "median block_count",
            "extra": "avg block_count: 23073.27623267164, max block_count: 25666.0, count: 57781"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 71,
            "unit": "median segment_count",
            "extra": "avg segment_count: 72.831553624894, max segment_count: 106.0, count: 57781"
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
          "id": "a97c525d0c11314fe01c8bbb250ea5fdf73ec8ce",
          "message": "fix: write all delete files atomically (#3178) (#3182)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nIn `ambulkdelete`, write all delete files to the meta list atomically,\ninstead of one at a time.\n\n## Why\n\nReduces the number of writes to the metas list.\n\n## How\n\n## Tests\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-09-18T21:50:00-04:00",
          "tree_id": "ba5917ed034f24a8e2ad95a64751e5faef3d55d5",
          "url": "https://github.com/paradedb/paradedb/commit/a97c525d0c11314fe01c8bbb250ea5fdf73ec8ce"
        },
        "date": 1758248289304,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.622696,
            "unit": "median cpu",
            "extra": "avg cpu: 19.005443835201508, max cpu: 42.72997, count: 57766"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 225.85546875,
            "unit": "median mem",
            "extra": "avg mem: 225.36644676907696, max mem: 227.37890625, count: 57766"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.255816,
            "unit": "median cpu",
            "extra": "avg cpu: 22.471422613707357, max cpu: 33.103447, count: 57766"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 159.359375,
            "unit": "median mem",
            "extra": "avg mem: 159.31562771840268, max mem: 161.234375, count: 57766"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 24260,
            "unit": "median block_count",
            "extra": "avg block_count: 23070.376415192328, max block_count: 25914.0, count: 57766"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 71,
            "unit": "median segment_count",
            "extra": "avg segment_count: 72.90601738046601, max segment_count: 106.0, count: 57766"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "e15e51abfc4b7834faea068d861d91d5d873580f",
          "message": "chore: Upgrade to `0.18.4` (#3184)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\nSigned-off-by: Ming <ming.ying.nyc@gmail.com>\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>\nCo-authored-by: Stu Hood <stuhood@paradedb.com>",
          "timestamp": "2025-09-18T21:52:13-04:00",
          "tree_id": "3d203e3468a4e7504d03af9c39ac9a0869033086",
          "url": "https://github.com/paradedb/paradedb/commit/e15e51abfc4b7834faea068d861d91d5d873580f"
        },
        "date": 1758248478769,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.640776,
            "unit": "median cpu",
            "extra": "avg cpu: 18.90207924373229, max cpu: 42.814667, count: 57790"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 233.87890625,
            "unit": "median mem",
            "extra": "avg mem: 231.61262754964093, max mem: 236.13671875, count: 57790"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.30097,
            "unit": "median cpu",
            "extra": "avg cpu: 22.50733221606574, max cpu: 33.333336, count: 57790"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 158.6640625,
            "unit": "median mem",
            "extra": "avg mem: 158.25980665448174, max mem: 162.48046875, count: 57790"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 24288,
            "unit": "median block_count",
            "extra": "avg block_count: 23091.846046028724, max block_count: 25929.0, count: 57790"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 71,
            "unit": "median segment_count",
            "extra": "avg segment_count: 73.06618792178578, max segment_count: 106.0, count: 57790"
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
          "id": "1046018b2db9614ef172bd802c98a3987da7513e",
          "message": "chore: small diff ported from enterprise for query cancel fix (#3186)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nSome small changes in enterprise that should be in community\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-09-18T21:53:42-04:00",
          "tree_id": "85ed1f4eb7261157deabdfba479dc61164775f99",
          "url": "https://github.com/paradedb/paradedb/commit/1046018b2db9614ef172bd802c98a3987da7513e"
        },
        "date": 1758248512223,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.677044,
            "unit": "median cpu",
            "extra": "avg cpu: 19.225564935792264, max cpu: 42.857143, count: 57331"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 226.671875,
            "unit": "median mem",
            "extra": "avg mem: 226.09672593415866, max mem: 229.59375, count: 57331"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.323614,
            "unit": "median cpu",
            "extra": "avg cpu: 22.457442799168003, max cpu: 33.366436, count: 57331"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 161.28515625,
            "unit": "median mem",
            "extra": "avg mem: 161.34388994937294, max mem: 162.78515625, count: 57331"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 24471,
            "unit": "median block_count",
            "extra": "avg block_count: 23132.183443512236, max block_count: 26178.0, count: 57331"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 71,
            "unit": "median segment_count",
            "extra": "avg segment_count: 73.10163785735466, max segment_count: 109.0, count: 57331"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "f052aabf25719cee68a756a379c6b66e39452759",
          "message": "feat: Port `ambulkdelete_epoch` to community (#3183)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nAdd `ambulkdelete_epoch` to the index metadata and make sure it's passed\ndown to all scans. In enterprise, this value is used to detect query\nconflicts with concurrent vacuums on the primary.\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-09-18T22:01:15-04:00",
          "tree_id": "48ffae94b2f43d5c2d62b5adb846d1dcc2992aee",
          "url": "https://github.com/paradedb/paradedb/commit/f052aabf25719cee68a756a379c6b66e39452759"
        },
        "date": 1758248964323,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.622696,
            "unit": "median cpu",
            "extra": "avg cpu: 19.03452936818916, max cpu: 42.60355, count: 57551"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 226.5625,
            "unit": "median mem",
            "extra": "avg mem: 226.04120740462807, max mem: 228.265625, count: 57551"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.233301,
            "unit": "median cpu",
            "extra": "avg cpu: 22.436868031031953, max cpu: 33.136093, count: 57551"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 161.828125,
            "unit": "median mem",
            "extra": "avg mem: 162.083685557158, max mem: 164.203125, count: 57551"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 24430,
            "unit": "median block_count",
            "extra": "avg block_count: 23080.372469635626, max block_count: 26056.0, count: 57551"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 71,
            "unit": "median segment_count",
            "extra": "avg segment_count: 72.8978992545742, max segment_count: 108.0, count: 57551"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "153f632ba06057571459a4b6e8767c135baf438c",
          "message": "chore: small diff ported from enterprise for query cancel fix (#3187)",
          "timestamp": "2025-09-18T22:31:35-04:00",
          "tree_id": "2c3b3f692c24ba8540a69da9d41f4d3a24d4ae6f",
          "url": "https://github.com/paradedb/paradedb/commit/153f632ba06057571459a4b6e8767c135baf438c"
        },
        "date": 1758250790918,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.786694,
            "unit": "median cpu",
            "extra": "avg cpu: 19.53658323337898, max cpu: 42.72997, count: 57320"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 225.87109375,
            "unit": "median mem",
            "extra": "avg mem: 225.33297087622122, max mem: 227.53515625, count: 57320"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.30097,
            "unit": "median cpu",
            "extra": "avg cpu: 22.459980709136364, max cpu: 33.300297, count: 57320"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 158.5859375,
            "unit": "median mem",
            "extra": "avg mem: 158.38449807549722, max mem: 162.29296875, count: 57320"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 24438,
            "unit": "median block_count",
            "extra": "avg block_count: 23039.90202372645, max block_count: 25992.0, count: 57320"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 71,
            "unit": "median segment_count",
            "extra": "avg segment_count: 72.74152128401954, max segment_count: 107.0, count: 57320"
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
          "id": "8101a67174703310a6a1655496fd5296e869901d",
          "message": "fix: Clone an Arc rather than a OnceLock. (#3185)\n\n## What\n\nInvert our use of `OnceLock` to ensure that we clone an\n`Arc<OnceLock<T>>` rather than a `OnceLock<Arc<T>>`.\n\n## Why\n\n`OnceLock` implements `Clone` by cloning its contents to create a\nseparate disconnected copy. If what is desired is \"exactly once\nbehavior\", then cloning the `OnceLock` before it has been computed the\nfirst time will defeat that.\n\nThis change has no impact on benchmarks in this case, but\n`Arc<OnceLock<T>>` matches the intent of this code, and sets a better\nexample for future us.\n\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>",
          "timestamp": "2025-09-19T15:01:21-07:00",
          "tree_id": "de6adf9a09b874a0e133e9cbfeca50d417e6c5bf",
          "url": "https://github.com/paradedb/paradedb/commit/8101a67174703310a6a1655496fd5296e869901d"
        },
        "date": 1758320981185,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.640776,
            "unit": "median cpu",
            "extra": "avg cpu: 18.991999041581842, max cpu: 42.72997, count: 57801"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 226.6875,
            "unit": "median mem",
            "extra": "avg mem: 226.20222880821697, max mem: 228.41015625, count: 57801"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.30097,
            "unit": "median cpu",
            "extra": "avg cpu: 22.39189182603578, max cpu: 33.300297, count: 57801"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 165.36328125,
            "unit": "median mem",
            "extra": "avg mem: 164.66601525330444, max mem: 167.41796875, count: 57801"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 24321,
            "unit": "median block_count",
            "extra": "avg block_count: 23030.83363609626, max block_count: 26004.0, count: 57801"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 71,
            "unit": "median segment_count",
            "extra": "avg segment_count: 72.84417224615491, max segment_count: 107.0, count: 57801"
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
          "id": "3163a5f3e48d3027585287ce8a63074f70ba1836",
          "message": "perf: Configurable Top N requeries more granularly (#3190)",
          "timestamp": "2025-09-19T21:06:04-04:00",
          "tree_id": "8c74bdf97c37281e4641be0e94b4d464daa5a3ea",
          "url": "https://github.com/paradedb/paradedb/commit/3163a5f3e48d3027585287ce8a63074f70ba1836"
        },
        "date": 1758332064804,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.713451,
            "unit": "median cpu",
            "extra": "avg cpu: 19.382019738133756, max cpu: 42.857143, count: 57910"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 226.46484375,
            "unit": "median mem",
            "extra": "avg mem: 226.0704706791789, max mem: 228.50390625, count: 57910"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.30097,
            "unit": "median cpu",
            "extra": "avg cpu: 22.406574856519356, max cpu: 33.300297, count: 57910"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 162.8203125,
            "unit": "median mem",
            "extra": "avg mem: 161.88291543451044, max mem: 166.97265625, count: 57910"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 24170,
            "unit": "median block_count",
            "extra": "avg block_count: 23019.354291141426, max block_count: 25952.0, count: 57910"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 71,
            "unit": "median segment_count",
            "extra": "avg segment_count: 72.89350716629252, max segment_count: 107.0, count: 57910"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "f573a31e6704d95d0a62271a23ba47658a1dae06",
          "message": "perf: Configurable Top N requeries more granularly (#3194)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nAllow the retry scale factor and max chunk size to be tuned, which is\nuseful for reducing Top N requeries.\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-09-20T09:26:21-04:00",
          "tree_id": "d4ee2092267660be53cb68f8b760756a5a07ab69",
          "url": "https://github.com/paradedb/paradedb/commit/f573a31e6704d95d0a62271a23ba47658a1dae06"
        },
        "date": 1758376471082,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.713451,
            "unit": "median cpu",
            "extra": "avg cpu: 19.48057349751462, max cpu: 42.64561, count: 57946"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 226.1953125,
            "unit": "median mem",
            "extra": "avg mem: 225.6010676966486, max mem: 227.9609375, count: 57946"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.255816,
            "unit": "median cpu",
            "extra": "avg cpu: 22.325163689497085, max cpu: 33.136093, count: 57946"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 161.8671875,
            "unit": "median mem",
            "extra": "avg mem: 160.92321959777206, max mem: 162.546875, count: 57946"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 24268,
            "unit": "median block_count",
            "extra": "avg block_count: 23056.306595796086, max block_count: 25875.0, count: 57946"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 71,
            "unit": "median segment_count",
            "extra": "avg segment_count: 72.7900804197011, max segment_count: 106.0, count: 57946"
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
          "id": "8b5e7226ce38eaa98ce91906a3f8eb0b72906e66",
          "message": "fix: update tantivy dependency revision (#3192)\n\n## What\n\nUpdate tantivy to\n\nhttps://github.com/paradedb/tantivy/commit/7c6c6fc6ac977382b19ae7fb9fd5b0c53b8f1b58\nwhich fixes a bug that disallowed a segment, during indexing, to real\nthe real memory limit of 4GB.\n\n## Why\n\nWe had a bug in our tantivy fork that wouldn't allow a segment, during\nindexing, to cross over 2GB to reach the actual limit of 4GB.\n\n## How\n\n## Tests",
          "timestamp": "2025-09-22T10:19:44-04:00",
          "tree_id": "89f8538f4216f95c908cb451c1405afaa80946e6",
          "url": "https://github.com/paradedb/paradedb/commit/8b5e7226ce38eaa98ce91906a3f8eb0b72906e66"
        },
        "date": 1758552584467,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.75,
            "unit": "median cpu",
            "extra": "avg cpu: 19.491344967273704, max cpu: 42.72997, count: 57344"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 225.3046875,
            "unit": "median mem",
            "extra": "avg mem: 224.82611744744437, max mem: 227.05078125, count: 57344"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.30097,
            "unit": "median cpu",
            "extra": "avg cpu: 22.40848872511472, max cpu: 33.23442, count: 57344"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 163.3125,
            "unit": "median mem",
            "extra": "avg mem: 162.99376930509294, max mem: 168.67578125, count: 57344"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 24251,
            "unit": "median block_count",
            "extra": "avg block_count: 23073.469273158484, max block_count: 25982.0, count: 57344"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 71,
            "unit": "median segment_count",
            "extra": "avg segment_count: 72.93221609933036, max segment_count: 107.0, count: 57344"
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
          "id": "7efcca559340008f06df6d1861f1f0301970a0dc",
          "message": "fix: use correct xids when returning to the fsm (#3191)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nThere's a few places where we need to use a future xid when returning\nblocks to the FSM.\n\nSpecically blocks from the segment meta entries list when it's garbage\ncollected.\n\n## Why\n\nTo address some community reports of what appear to be corrupt index\npages.\n\n## How\n\n## Tests",
          "timestamp": "2025-09-22T10:21:41-04:00",
          "tree_id": "a4cdecfcfa42fbe38c487b7c4f6c5cc31eef4f46",
          "url": "https://github.com/paradedb/paradedb/commit/7efcca559340008f06df6d1861f1f0301970a0dc"
        },
        "date": 1758552705617,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.842003,
            "unit": "median cpu",
            "extra": "avg cpu: 19.418180620310526, max cpu: 42.72997, count: 57588"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 226.44921875,
            "unit": "median mem",
            "extra": "avg mem: 225.75320928676547, max mem: 228.2109375, count: 57588"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.27837,
            "unit": "median cpu",
            "extra": "avg cpu: 22.446796378001565, max cpu: 33.23442, count: 57588"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 158.453125,
            "unit": "median mem",
            "extra": "avg mem: 158.06027385533878, max mem: 161.828125, count: 57588"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 23850,
            "unit": "median block_count",
            "extra": "avg block_count: 22879.202108078072, max block_count: 25970.0, count: 57588"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 71,
            "unit": "median segment_count",
            "extra": "avg segment_count: 72.51179065083004, max segment_count: 106.0, count: 57588"
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
          "id": "da997100cd0e2873fa8692ec6c2382761719ce58",
          "message": "chore: Upgrade to `0.18.2` (#3144) (#3145)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-09-10T17:44:33-04:00",
          "tree_id": "d0d9fd4cb9ebc554c1e7f3e029694e863f4247c9",
          "url": "https://github.com/paradedb/paradedb/commit/da997100cd0e2873fa8692ec6c2382761719ce58"
        },
        "date": 1757543110225,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 34.32782262329838,
            "unit": "median tps",
            "extra": "avg tps: 34.2596571422541, max tps: 34.51292914402329, count: 57400"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 390.97428051202667,
            "unit": "median tps",
            "extra": "avg tps: 390.9001937610643, max tps: 423.327598604861, count: 57400"
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
          "id": "1cfaa7b311ca8b7ee91491411c8dfecd2ce5619c",
          "message": "fix: `GROUP BY` doesn't panic when Postgres eliminates group pathkeys (#3152)\n\n# Ticket(s) Closed\n\n- Closes #3050 \n\n## What\n\nIt's possible for Postgres to eliminate group pathkeys if it realizes\nthat one of the pathkeys is unique, making the other ones unnecessary.\n\nWe need to handle this case/not panic.\n\n## Why\n\nSee issue.\n\n## How\n\nInject the dropped group pathkeys back into our list of grouping\ncolumns.\n\n## Tests\n\nAdded regression test",
          "timestamp": "2025-09-14T17:56:19-04:00",
          "tree_id": "a41824569d62cfd5dbe40884e6ead540d3b1bd88",
          "url": "https://github.com/paradedb/paradedb/commit/1cfaa7b311ca8b7ee91491411c8dfecd2ce5619c"
        },
        "date": 1757889329985,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 34.8655053725746,
            "unit": "median tps",
            "extra": "avg tps: 34.791768462646964, max tps: 35.15468458924473, count: 57464"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 406.1184868155923,
            "unit": "median tps",
            "extra": "avg tps: 405.3093430096764, max tps: 441.4728913484981, count: 57464"
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
          "id": "a521487756693e82c46bfe2f1a2f2fd3aded0136",
          "message": "fix: fixed `rt_fetch out-of-bounds` error (#3141)\n\n# Ticket(s) Closed\n\n- Closes #3135\n\n## What\n\nFixed `rt_fetch used out-of-bounds` and `Cannot open relation with\noid=0` errors that occurred in complex SQL queries with nested `OR\nEXISTS` clauses, multiple `JOIN`s.\n\n## Why\n\nThe issue occurred when PostgreSQL's query planner generated `Var` nodes\nreferencing Range Table Entries (RTEs) that were valid in outer planning\ncontexts but didn't exist in inner execution contexts. This happened\nspecifically with:\n- `OR EXISTS` subqueries (not `AND EXISTS`)  \n- Multiple `JOIN`s within the `EXISTS` clause\n- ParadeDB functions applied to joined tables\n\nWhen ParadeDB's custom scan tried to access these out-of-bounds RTEs\nusing `rt_fetch`, it caused crashes.\n\n## How\n\nImplemented bounds checking across the codebase:\n\n1. **Early detection**: Added bounds checking in `find_var_relation()`\nto detect invalid `varno` values and return `pg_sys::InvalidOid`. This\nwas the main fix for the issue.\n2. **Graceful handling**: Modified all functions that receive relation\nOIDs to check for `InvalidOid` before attempting to open relations\n3. **Safe fallbacks**: Updated query optimization logic to skip\noptimizations when relation information is unavailable rather than\ncrashing\n\n## Tests\n\nAdded regression test `or_exists_join_bug.sql` covering:\n- Simple queries (baseline functionality)\n- `AND EXISTS` with multiple `JOIN`s (should work)  \n- `OR EXISTS` with multiple `JOIN`s (the problematic case, now fixed)\n- Various edge cases and workarounds\n- Minimal reproduction cases\n\n---------\n\nSigned-off-by: Moe <mdashti@gmail.com>",
          "timestamp": "2025-09-15T02:47:52-07:00",
          "tree_id": "4a0b5db116e0263111295cc53d05810e093ce68c",
          "url": "https://github.com/paradedb/paradedb/commit/a521487756693e82c46bfe2f1a2f2fd3aded0136"
        },
        "date": 1757932023134,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 34.19499544738431,
            "unit": "median tps",
            "extra": "avg tps: 34.07872572029445, max tps: 34.35403468802994, count: 57769"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 396.7440307321041,
            "unit": "median tps",
            "extra": "avg tps: 395.06342950604534, max tps: 427.9865109573478, count: 57769"
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
          "id": "b37fc5af676e3438c051381414d81996ed0fb8f6",
          "message": "feat: push down `group by ... order by ... limit` (#3134)\n\n# Ticket(s) Closed\n\n- Closes #3131 \n- Opens #3156 #3155 \n\n## What\n\nPushes down `group by ... order by ... limit` to Tantivy\n\n## Why\n\nBy pushing down the sort/limit to Tantivy, we can significantly speed up\n`group by` queries over high cardinality columns.\n\n## How\n\n- Before we were hard-coding a bucket size and sorting the results\nourselves, now the bucket size is set to the limit and we push the sort\ndown to the Tantivy term agg\n\n## Tests",
          "timestamp": "2025-09-15T15:51:50-04:00",
          "tree_id": "e58df02d60abc13101aaae8ef6333a9afafbcd78",
          "url": "https://github.com/paradedb/paradedb/commit/b37fc5af676e3438c051381414d81996ed0fb8f6"
        },
        "date": 1757968256795,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 34.00151252712465,
            "unit": "median tps",
            "extra": "avg tps: 33.88361160558337, max tps: 34.261670837666756, count: 57653"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 388.65279563943085,
            "unit": "median tps",
            "extra": "avg tps: 388.7951628024147, max tps: 424.3148705257172, count: 57653"
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
          "id": "8211eef7a0dd34237afebfa91364fb66c65a4906",
          "message": "perf: remove `ExactBuffer` in favor of a regular rust `BufWriter` (#3158)\n\n# Ticket(s) Closed\n\n- Closes #2981  (/cc @yjhjstz)\n\nWhile #2981 wasn't the impetus for this, it addresses the complaint made\nthere just the same.\n\n## What\n\nIn profiling, our `ExactBuffer` was a large percentage of certain\nprofiles. This replaces it, and its complexity, with a standard Rust\n`BufWriter`.\n\n## Why\n\nImproves performance of (at least) our `wide-table.toml` test's \"Single\nUpdate\" job by quite a bit.\n\n<img width=\"720\" height=\"141\" alt=\"screenshot_2025-09-15_at_3 28\n33___pm_720\"\nsrc=\"https://github.com/user-attachments/assets/a373a7ae-df38-4691-980a-d6843f073d26\"\n/>\n\n\n## How\n\n## Tests\n\nExisting tests pass",
          "timestamp": "2025-09-15T15:55:52-04:00",
          "tree_id": "4ddf140542c5525034023441aadac4b634c90fc6",
          "url": "https://github.com/paradedb/paradedb/commit/8211eef7a0dd34237afebfa91364fb66c65a4906"
        },
        "date": 1757968500169,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 36.03984913941636,
            "unit": "median tps",
            "extra": "avg tps: 35.95567342848722, max tps: 36.27277759301276, count: 57572"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 583.7708514245383,
            "unit": "median tps",
            "extra": "avg tps: 583.583785859464, max tps: 678.4226970576277, count: 57572"
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
          "id": "288a4bfa0c79838d86711b8a6231687c984ac0b5",
          "message": "chore: Upgrade to `0.18.3` (#3160)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-09-15T16:13:06-04:00",
          "tree_id": "ad59a6c86e8afe29cabad5b0bcc6a78bc448182e",
          "url": "https://github.com/paradedb/paradedb/commit/288a4bfa0c79838d86711b8a6231687c984ac0b5"
        },
        "date": 1757969529085,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 35.33732279778155,
            "unit": "median tps",
            "extra": "avg tps: 35.33886224593572, max tps: 35.79275582318993, count: 57386"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 596.626991647523,
            "unit": "median tps",
            "extra": "avg tps: 595.3690060155251, max tps: 676.5761235998564, count: 57386"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "af5bea23effe976b411147e259e53afad947a393",
          "message": "perf: remove `ExactBuffer` in favor of a regular rust `BufWriter` (#3159)\n\n# Ticket(s) Closed\n\n- Closes #2981  (/cc @yjhjstz)\n\nWhile #2981 wasn't the impetus for this, it addresses the complaint made\nthere just the same.\n\n## What\n\nIn profiling, our `ExactBuffer` was a large percentage of certain\nprofiles. This replaces it, and its complexity, with a standard Rust\n`BufWriter`.\n\n## Why\n\nImproves performance of (at least) our `wide-table.toml` test's \"Single\nUpdate\" job by quite a bit.\n\n<img width=\"720\" height=\"141\" alt=\"screenshot_2025-09-15_at_3 28\n33___pm_720\"\nsrc=\"https://github.com/user-attachments/assets/a373a7ae-df38-4691-980a-d6843f073d26\"\n/>\n\n\n## How\n\n## Tests\n\nExisting tests pass\n\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>",
          "timestamp": "2025-09-15T16:26:19-04:00",
          "tree_id": "cbc00b9a93c129255360f60e5a70904e87f1e8c1",
          "url": "https://github.com/paradedb/paradedb/commit/af5bea23effe976b411147e259e53afad947a393"
        },
        "date": 1757970390026,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 35.95522645010718,
            "unit": "median tps",
            "extra": "avg tps: 35.82258573832357, max tps: 36.15931436402156, count: 57694"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 586.8842735930652,
            "unit": "median tps",
            "extra": "avg tps: 585.5378976855646, max tps: 666.6814121580074, count: 57694"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "7800d096e107acdbdec6297d0cb98ef030569e2b",
          "message": "chore: Upgrade to `0.18.3` (#3161)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-09-15T16:36:57-04:00",
          "tree_id": "c0962cc02d5690156721fd003c985f724ee9b20f",
          "url": "https://github.com/paradedb/paradedb/commit/7800d096e107acdbdec6297d0cb98ef030569e2b"
        },
        "date": 1757971006554,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 35.94392169293443,
            "unit": "median tps",
            "extra": "avg tps: 35.8652102143425, max tps: 36.14062374110759, count: 57840"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 586.3989955524523,
            "unit": "median tps",
            "extra": "avg tps: 590.1201795191067, max tps: 723.0813132213349, count: 57840"
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
          "id": "f71a5572d645d23e58b949cc3f16645473c74735",
          "message": "chore: Sync `0.18.x` (#3162)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\n---------\n\nSigned-off-by: Moe <mdashti@gmail.com>\nCo-authored-by: Moe <mdashti@gmail.com>",
          "timestamp": "2025-09-15T17:11:39-04:00",
          "tree_id": "a75daf7f281149ef4317505338649d8b0d2ec8a4",
          "url": "https://github.com/paradedb/paradedb/commit/f71a5572d645d23e58b949cc3f16645473c74735"
        },
        "date": 1757973054646,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 35.80187323849048,
            "unit": "median tps",
            "extra": "avg tps: 35.700034853751795, max tps: 36.086024961596124, count: 57364"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 605.1762258618348,
            "unit": "median tps",
            "extra": "avg tps: 610.104344592047, max tps: 744.8357525616824, count: 57364"
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
          "id": "878c50feef96d61785ad711ebe46250c920bed70",
          "message": "fix: sequential scan segfault (#3163)\n\n# Ticket(s) Closed\n\n- Closes #3151 \n\n## What\n\nThe `@@@` return type should be `bool`, not `SearchQueryInput`.\n\n## Why\n\n## How\n\n## Tests\n\nAdded regression test.",
          "timestamp": "2025-09-16T10:27:13-04:00",
          "tree_id": "6859469869310b79c8c32af68b3ed77dfb787362",
          "url": "https://github.com/paradedb/paradedb/commit/878c50feef96d61785ad711ebe46250c920bed70"
        },
        "date": 1758035199614,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 36.186798585532955,
            "unit": "median tps",
            "extra": "avg tps: 36.090589625470614, max tps: 36.41734847805764, count: 57647"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 595.1052289756238,
            "unit": "median tps",
            "extra": "avg tps: 596.0051258480202, max tps: 705.350721282946, count: 57647"
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
          "id": "f2a0c9c43e4385628cc7b828a8ed12c30e55050e",
          "message": "chore: don't warn about `raw` tokenizer on UUID key fields (#3166)\n\n## What\n\nRemove the warning about using the `raw` tokenizer when the `key_field`\nis a UUID field.\n\nThe drama here is that we (pg_search) assign the `raw` tokenizer to UUID\nfields used as the key_field so there's nothing a user can do about it.\nWarning about our own bad decisions is not cool.\n\n## Why\n\nMany user and customer complaints.\n\n## How\n\n## Tests\n\nExisting tests pass but a couple of the regression test expected output\nis now different.",
          "timestamp": "2025-09-16T13:10:47-04:00",
          "tree_id": "2b24aea6e3a0645c584d8ebb8ce7465c8c90f904",
          "url": "https://github.com/paradedb/paradedb/commit/f2a0c9c43e4385628cc7b828a8ed12c30e55050e"
        },
        "date": 1758045007161,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 35.28271431348583,
            "unit": "median tps",
            "extra": "avg tps: 35.24945151087628, max tps: 35.87316447419095, count: 58021"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 612.0884249735694,
            "unit": "median tps",
            "extra": "avg tps: 611.7917991724885, max tps: 702.579747404793, count: 58021"
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
          "id": "489eb48583040067612195f9e1406d5e31a1599f",
          "message": "perf: teach custom scan callback to exit early if it can (#3168)\n\n## What\n\nThis does two things.  \n\nOne, the first commit (62d752572b2d7bc5a02b7203ac2c83949e38e27e) simply\nreorders some code in the custom scan callback so it can decide to exit\nearly if we're not going to submit a path. Specifically, this is\nintended to avoid opening a Directory and Index and related structures.\n\nTwo, the second commit (5ac1dde23ef0809bea4b942d04fd14acc9d1c152) makes\na new decision to not evaluate possible pushdown predicates when the\nstatement type is not a SELECT statement. This cuts out the overhead of\nneeding to read/deserialize the index's schema at all on (at least)\nUPDATE statements.\n\nThis does mean that we won't consider doing pushdowns for UPDATE\nstatements, even if doing one would make the UPDATE scan faster.\n\n## Why\n\nTrying to reduce per-query overhead, targeting our stressgres benchmarks\nlike \"single-server.toml\" and \"wide-table.toml\".\n\n## How\n\n## Tests\n\nAll existing tests pass.",
          "timestamp": "2025-09-16T17:39:51-04:00",
          "tree_id": "0ebcd01c6225cbb43b199470f7f78bd694493ed7",
          "url": "https://github.com/paradedb/paradedb/commit/489eb48583040067612195f9e1406d5e31a1599f"
        },
        "date": 1758061144490,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 36.86853287577319,
            "unit": "median tps",
            "extra": "avg tps: 36.701107250783565, max tps: 37.00631448569973, count: 57664"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 619.3069693817968,
            "unit": "median tps",
            "extra": "avg tps: 619.6140069746084, max tps: 721.1585383019876, count: 57664"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "63daa7f2bf568127e538f19f942d6363508ca615",
          "message": "chore: don't warn about `raw` tokenizer on UUID key fields (#3167)\n\n## What\n\nRemove the warning about using the `raw` tokenizer when the `key_field`\nis a UUID field.\n\nThe drama here is that we (pg_search) assign the `raw` tokenizer to UUID\nfields used as the key_field so there's nothing a user can do about it.\nWarning about our own bad decisions is not cool.\n\n## Why\n\nMany user and customer complaints.\n\n## How\n\n## Tests\n\nExisting tests pass but a couple of the regression test expected output\nis now different.\n\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>",
          "timestamp": "2025-09-17T10:06:31-04:00",
          "tree_id": "2c472616485a1c2a1ed61c7f2c030286882deb06",
          "url": "https://github.com/paradedb/paradedb/commit/63daa7f2bf568127e538f19f942d6363508ca615"
        },
        "date": 1758120357718,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 36.05257524707471,
            "unit": "median tps",
            "extra": "avg tps: 35.940042482311675, max tps: 36.45464933327694, count: 56889"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 642.8602759024426,
            "unit": "median tps",
            "extra": "avg tps: 643.0030063266685, max tps: 767.9910179815552, count: 56889"
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
          "id": "eb456f8c97d99e92e2795d88dd2c1082c13c83a6",
          "message": "perf: optimize `Timestamp` and `JsonB` datum decoding (#3171)\n\n## What\n\nOptimize `Timestamp` and `JsonB` to `TantivyValue` datum conversions.\n\nThese two show up quite high in profiles. The `JsonB` conversion in\nparticular has been bad due to how pgrx stupidly (I can say it) handles\n`JsonB` values by converting them to strings and then asking serde to\nparse the strings.\n\n## Why\n\nTrying to make things faster.\n\n## How\n\nFor the `Timestamp` conversion we memoize Postgres' understanding of the\ncurrent EPOCH and do the same math that it does to calculate a time\nvalue.\n\nFor the `JsonB` conversion we implement our own deserializer routine\nusing Postgres' internal `JsonbIteratorInit()` and `JsonbIteratorNext()`\nfunctions, building up a `serde_json::Value` structure as it goes.\n\n\n## Tests\n\nA new `#[pg_test]`-based proptest has been added to test our custom\njsonb deserializer against normal serde.\n\n---------\n\nSigned-off-by: Eric Ridge <eebbrr@gmail.com>\nCo-authored-by: Stu Hood <stuhood@paradedb.com>",
          "timestamp": "2025-09-17T15:26:06-04:00",
          "tree_id": "702cea735a514e9b33d6c1ee785606d39d4f705c",
          "url": "https://github.com/paradedb/paradedb/commit/eb456f8c97d99e92e2795d88dd2c1082c13c83a6"
        },
        "date": 1758139610277,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 46.658012563405215,
            "unit": "median tps",
            "extra": "avg tps: 46.52160190762914, max tps: 47.9320351582405, count: 57411"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 574.6651352524582,
            "unit": "median tps",
            "extra": "avg tps: 578.2519836690008, max tps: 688.5202054461437, count: 57411"
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
          "id": "849076799ca599dfbf0f2415149b12495b24624c",
          "message": "fix: Always use a shared expression context to execute HeapFilters. (#3174)\n\n## What\n\nEvaluating heap filters was leaking a `CreateStandaloneExprContext` per\nexecution, which could eventually lead to OOMs.\n\n## Why\n\n`PgBox::from_pg` does not free the resulting memory: it's expected to be\nfreed by a memory context, since PG created it.\n\n## How\n\nAlways use a global expression context created by\n`ExecAssignExprContext` in `begin_custom_scan`.\n\n## Tests\n\nA repro uses significantly less memory, and [a benchmark query added for\nthis purpose](https://github.com/paradedb/paradedb/pull/3175) is faster.",
          "timestamp": "2025-09-17T16:44:32-07:00",
          "tree_id": "7eef1c518a935389aa23e91c6bc47bbc325b18e6",
          "url": "https://github.com/paradedb/paradedb/commit/849076799ca599dfbf0f2415149b12495b24624c"
        },
        "date": 1758155042072,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 46.43032740364295,
            "unit": "median tps",
            "extra": "avg tps: 46.54268708555075, max tps: 48.475975155175014, count: 56558"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 595.3755371729856,
            "unit": "median tps",
            "extra": "avg tps: 598.0198865304727, max tps: 706.1907831422631, count: 56558"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "dc0e87d6fdabdb573d6f37a24c6c33befa43e8b0",
          "message": "fix: Always use a shared expression context to execute HeapFilters. (#3176)\n\n## What\n\nEvaluating heap filters was leaking a `CreateStandaloneExprContext` per\nexecution, which could eventually lead to OOMs.\n\n## Why\n\n`PgBox::from_pg` does not free the resulting memory: it's expected to be\nfreed by a memory context, since PG created it.\n\n## How\n\nAlways use a global expression context created by\n`ExecAssignExprContext` in `begin_custom_scan`.\n\n## Tests\n\nA repro uses significantly less memory, and [a benchmark query added for\nthis purpose](https://github.com/paradedb/paradedb/pull/3175) is faster.\n\nCo-authored-by: Stu Hood <stuhood@paradedb.com>",
          "timestamp": "2025-09-17T17:11:43-07:00",
          "tree_id": "0c30f446ad8404b4f66727777f1b6e6a5bc8958e",
          "url": "https://github.com/paradedb/paradedb/commit/dc0e87d6fdabdb573d6f37a24c6c33befa43e8b0"
        },
        "date": 1758156685552,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 46.85771592154582,
            "unit": "median tps",
            "extra": "avg tps: 46.858574732235176, max tps: 48.563356906830826, count: 57745"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 582.6206868262547,
            "unit": "median tps",
            "extra": "avg tps: 587.2620465421335, max tps: 703.4971293993217, count: 57745"
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
          "id": "3bcb1451087be74b7bd73bfc7d6546423046a0ce",
          "message": "fix: write all delete files atomically (#3178)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nIn `ambulkdelete`, write all delete files to the meta list atomically,\ninstead of one at a time.\n\n## Why\n\nReduces the number of writes to the metas list.\n\n## How\n\n## Tests",
          "timestamp": "2025-09-18T16:03:10-04:00",
          "tree_id": "ad9609f0419a34b8f0cf543e911c1dc3c25d4563",
          "url": "https://github.com/paradedb/paradedb/commit/3bcb1451087be74b7bd73bfc7d6546423046a0ce"
        },
        "date": 1758228171539,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 46.64891486008565,
            "unit": "median tps",
            "extra": "avg tps: 46.7732229727862, max tps: 48.900454305430735, count: 56987"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 611.9517943625154,
            "unit": "median tps",
            "extra": "avg tps: 615.3194737249194, max tps: 754.8805749393778, count: 56987"
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
          "id": "6e11875ba052ccd6937ca0c535b3803309c8b6eb",
          "message": "feat: removed aggregation limitations re mix of aggregate functions and aggregation on group-by column. (#3179)\n\n# Ticket(s) Closed\n\n- Closes #2963\n\n## What\n\nRemoves aggregate limitations that prevented queries where the same\nfield is used in both `GROUP BY` and aggregate functions (e.g., `SELECT\nrating, AVG(rating) FROM table GROUP BY rating`).\n\n## Why\n\nPrevious safety checks blocked these queries due to Tantivy's\n\"incompatible fruit types\" errors, but testing shows the underlying\nissue is resolved. The limitations were overly restrictive and caused\nunnecessary fallbacks to slower PostgreSQL aggregation.\n\n## How\n\n- Removed `has_search_field_conflicts` function and field conflict\nvalidation\n- Eliminated ~35 lines of restrictive code in\n`extract_and_validate_aggregates`\n- Previously blocked queries now use faster `AggregateScan` instead of\n`GroupAggregate`\n\n## Tests\n\n- **`aggregate-groupby-conflict.sql`** - Tests `GROUP BY field` with\naggregates on same field\n- **`test-fruit-types-issue.sql`** - Validates #2963 issue resolution  \n- **`groupby_aggregate.out`** - Updated expectations showing\n`AggregateScan` usage",
          "timestamp": "2025-09-18T16:00:25-07:00",
          "tree_id": "f85924512f419186b824a986dd35eaa96d973884",
          "url": "https://github.com/paradedb/paradedb/commit/6e11875ba052ccd6937ca0c535b3803309c8b6eb"
        },
        "date": 1758238801473,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 46.628693894989325,
            "unit": "median tps",
            "extra": "avg tps: 46.60137773993554, max tps: 47.961292091434025, count: 58022"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 604.6232560024505,
            "unit": "median tps",
            "extra": "avg tps: 606.4378021145862, max tps: 718.0308349467156, count: 58022"
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
          "id": "020f92b742187fe6fdc75a19390692e6d2e9a373",
          "message": "feat: Port `ambulkdelete_epoch` to community (#3180)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nAdd `ambulkdelete_epoch` to the index metadata and make sure it's passed\ndown to all scans. In enterprise, this value is used to detect query\nconflicts with concurrent vacuums on the primary.\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-09-18T19:16:09-04:00",
          "tree_id": "3642b293b38caa7676318f888b910c3f934e1976",
          "url": "https://github.com/paradedb/paradedb/commit/020f92b742187fe6fdc75a19390692e6d2e9a373"
        },
        "date": 1758239763650,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 47.5783201824845,
            "unit": "median tps",
            "extra": "avg tps: 47.53280297775848, max tps: 48.99977065580093, count: 57612"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 576.354450095338,
            "unit": "median tps",
            "extra": "avg tps: 579.5650524332784, max tps: 724.9783691108424, count: 57612"
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
          "id": "c0dc0e674f3524c2f65a7b387ccbf594b23e5e0e",
          "message": "chore: Upgrade to `0.18.4` (#3181)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\n---------\n\nSigned-off-by: Ming <ming.ying.nyc@gmail.com>\nCo-authored-by: Stu Hood <stuhood@paradedb.com>",
          "timestamp": "2025-09-18T19:18:34-04:00",
          "tree_id": "b67f22553ed7786ef556afbfad2b7f8ddc6b139e",
          "url": "https://github.com/paradedb/paradedb/commit/c0dc0e674f3524c2f65a7b387ccbf594b23e5e0e"
        },
        "date": 1758239998367,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 47.95577948564976,
            "unit": "median tps",
            "extra": "avg tps: 47.718196269514046, max tps: 49.17910412976889, count: 57625"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 587.3155887919804,
            "unit": "median tps",
            "extra": "avg tps: 592.846701523783, max tps: 806.6144830950872, count: 57625"
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
          "id": "a97c525d0c11314fe01c8bbb250ea5fdf73ec8ce",
          "message": "fix: write all delete files atomically (#3178) (#3182)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nIn `ambulkdelete`, write all delete files to the meta list atomically,\ninstead of one at a time.\n\n## Why\n\nReduces the number of writes to the metas list.\n\n## How\n\n## Tests\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-09-18T21:50:00-04:00",
          "tree_id": "ba5917ed034f24a8e2ad95a64751e5faef3d55d5",
          "url": "https://github.com/paradedb/paradedb/commit/a97c525d0c11314fe01c8bbb250ea5fdf73ec8ce"
        },
        "date": 1758249000120,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 46.75637345075127,
            "unit": "median tps",
            "extra": "avg tps: 46.71293138436812, max tps: 48.24089029479664, count: 57774"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 573.2073319848355,
            "unit": "median tps",
            "extra": "avg tps: 578.6664274940713, max tps: 721.6169178756694, count: 57774"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "e15e51abfc4b7834faea068d861d91d5d873580f",
          "message": "chore: Upgrade to `0.18.4` (#3184)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\nSigned-off-by: Ming <ming.ying.nyc@gmail.com>\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>\nCo-authored-by: Stu Hood <stuhood@paradedb.com>",
          "timestamp": "2025-09-18T21:52:13-04:00",
          "tree_id": "3d203e3468a4e7504d03af9c39ac9a0869033086",
          "url": "https://github.com/paradedb/paradedb/commit/e15e51abfc4b7834faea068d861d91d5d873580f"
        },
        "date": 1758249173629,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 47.17523002823842,
            "unit": "median tps",
            "extra": "avg tps: 47.09566666827146, max tps: 48.354386773705095, count: 57685"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 575.401080626544,
            "unit": "median tps",
            "extra": "avg tps: 577.9796922971565, max tps: 687.5544063460793, count: 57685"
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
          "id": "1046018b2db9614ef172bd802c98a3987da7513e",
          "message": "chore: small diff ported from enterprise for query cancel fix (#3186)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nSome small changes in enterprise that should be in community\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-09-18T21:53:42-04:00",
          "tree_id": "85ed1f4eb7261157deabdfba479dc61164775f99",
          "url": "https://github.com/paradedb/paradedb/commit/1046018b2db9614ef172bd802c98a3987da7513e"
        },
        "date": 1758249249486,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 48.27854629579923,
            "unit": "median tps",
            "extra": "avg tps: 48.30576778002581, max tps: 50.149624359408904, count: 57780"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 591.4544078569563,
            "unit": "median tps",
            "extra": "avg tps: 597.3079820001481, max tps: 772.7968073697307, count: 57780"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "f052aabf25719cee68a756a379c6b66e39452759",
          "message": "feat: Port `ambulkdelete_epoch` to community (#3183)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nAdd `ambulkdelete_epoch` to the index metadata and make sure it's passed\ndown to all scans. In enterprise, this value is used to detect query\nconflicts with concurrent vacuums on the primary.\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-09-18T22:01:15-04:00",
          "tree_id": "48ffae94b2f43d5c2d62b5adb846d1dcc2992aee",
          "url": "https://github.com/paradedb/paradedb/commit/f052aabf25719cee68a756a379c6b66e39452759"
        },
        "date": 1758249660595,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 48.68572861923553,
            "unit": "median tps",
            "extra": "avg tps: 48.60431541640026, max tps: 49.997074174978465, count: 57661"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 576.6581997570621,
            "unit": "median tps",
            "extra": "avg tps: 582.0382795065016, max tps: 704.3792710887585, count: 57661"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "153f632ba06057571459a4b6e8767c135baf438c",
          "message": "chore: small diff ported from enterprise for query cancel fix (#3187)",
          "timestamp": "2025-09-18T22:31:35-04:00",
          "tree_id": "2c3b3f692c24ba8540a69da9d41f4d3a24d4ae6f",
          "url": "https://github.com/paradedb/paradedb/commit/153f632ba06057571459a4b6e8767c135baf438c"
        },
        "date": 1758251480829,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 46.52564243061721,
            "unit": "median tps",
            "extra": "avg tps: 46.404817363645044, max tps: 48.5951577325077, count: 56981"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 607.756141229271,
            "unit": "median tps",
            "extra": "avg tps: 608.6908333872653, max tps: 726.0105977491229, count: 56981"
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
          "id": "8101a67174703310a6a1655496fd5296e869901d",
          "message": "fix: Clone an Arc rather than a OnceLock. (#3185)\n\n## What\n\nInvert our use of `OnceLock` to ensure that we clone an\n`Arc<OnceLock<T>>` rather than a `OnceLock<Arc<T>>`.\n\n## Why\n\n`OnceLock` implements `Clone` by cloning its contents to create a\nseparate disconnected copy. If what is desired is \"exactly once\nbehavior\", then cloning the `OnceLock` before it has been computed the\nfirst time will defeat that.\n\nThis change has no impact on benchmarks in this case, but\n`Arc<OnceLock<T>>` matches the intent of this code, and sets a better\nexample for future us.\n\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>",
          "timestamp": "2025-09-19T15:01:21-07:00",
          "tree_id": "de6adf9a09b874a0e133e9cbfeca50d417e6c5bf",
          "url": "https://github.com/paradedb/paradedb/commit/8101a67174703310a6a1655496fd5296e869901d"
        },
        "date": 1758321672664,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 47.38957510699042,
            "unit": "median tps",
            "extra": "avg tps: 47.17322199951869, max tps: 49.019941221539014, count: 57619"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 588.8308962890161,
            "unit": "median tps",
            "extra": "avg tps: 592.541047281289, max tps: 718.4338518893918, count: 57619"
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
          "id": "3163a5f3e48d3027585287ce8a63074f70ba1836",
          "message": "perf: Configurable Top N requeries more granularly (#3190)",
          "timestamp": "2025-09-19T21:06:04-04:00",
          "tree_id": "8c74bdf97c37281e4641be0e94b4d464daa5a3ea",
          "url": "https://github.com/paradedb/paradedb/commit/3163a5f3e48d3027585287ce8a63074f70ba1836"
        },
        "date": 1758332757619,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 46.43386856128854,
            "unit": "median tps",
            "extra": "avg tps: 46.446131464419345, max tps: 48.166804975187475, count: 57763"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 575.4340397646366,
            "unit": "median tps",
            "extra": "avg tps: 580.7974890099725, max tps: 722.9219540403342, count: 57763"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "f573a31e6704d95d0a62271a23ba47658a1dae06",
          "message": "perf: Configurable Top N requeries more granularly (#3194)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nAllow the retry scale factor and max chunk size to be tuned, which is\nuseful for reducing Top N requeries.\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-09-20T09:26:21-04:00",
          "tree_id": "d4ee2092267660be53cb68f8b760756a5a07ab69",
          "url": "https://github.com/paradedb/paradedb/commit/f573a31e6704d95d0a62271a23ba47658a1dae06"
        },
        "date": 1758377172275,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 46.55440813786506,
            "unit": "median tps",
            "extra": "avg tps: 46.54210289113978, max tps: 48.390969317988734, count: 57979"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 601.1234849488346,
            "unit": "median tps",
            "extra": "avg tps: 607.6391042886095, max tps: 769.4370595257326, count: 57979"
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
        "date": 1757515076670,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.677044,
            "unit": "median cpu",
            "extra": "avg cpu: 20.46092039409077, max cpu: 51.51219, count: 56690"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 170.2734375,
            "unit": "median mem",
            "extra": "avg mem: 169.4291618892221, max mem: 173.828125, count: 56690"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 21719,
            "unit": "median block_count",
            "extra": "avg block_count: 19605.89156817781, max block_count: 23219.0, count: 56690"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 49,
            "unit": "median segment_count",
            "extra": "avg segment_count: 53.194425824660435, max segment_count: 146.0, count: 56690"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 5.672061825225562, max cpu: 28.486649, count: 56690"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 165.0546875,
            "unit": "median mem",
            "extra": "avg mem: 154.65616317913214, max mem: 167.32421875, count: 56690"
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
          "id": "da997100cd0e2873fa8692ec6c2382761719ce58",
          "message": "chore: Upgrade to `0.18.2` (#3144) (#3145)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-09-10T17:44:33-04:00",
          "tree_id": "d0d9fd4cb9ebc554c1e7f3e029694e863f4247c9",
          "url": "https://github.com/paradedb/paradedb/commit/da997100cd0e2873fa8692ec6c2382761719ce58"
        },
        "date": 1757543112697,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.677044,
            "unit": "median cpu",
            "extra": "avg cpu: 20.001007593548678, max cpu: 60.348164, count: 57400"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 169.13671875,
            "unit": "median mem",
            "extra": "avg mem: 168.47533216735627, max mem: 172.4140625, count: 57400"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 21615,
            "unit": "median block_count",
            "extra": "avg block_count: 19481.32425087108, max block_count: 23259.0, count: 57400"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 49,
            "unit": "median segment_count",
            "extra": "avg segment_count: 53.788536585365854, max segment_count: 131.0, count: 57400"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.669261,
            "unit": "median cpu",
            "extra": "avg cpu: 5.683029138946453, max cpu: 27.961164, count: 57400"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 169.71484375,
            "unit": "median mem",
            "extra": "avg mem: 159.38507424597125, max mem: 171.26171875, count: 57400"
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
          "id": "1cfaa7b311ca8b7ee91491411c8dfecd2ce5619c",
          "message": "fix: `GROUP BY` doesn't panic when Postgres eliminates group pathkeys (#3152)\n\n# Ticket(s) Closed\n\n- Closes #3050 \n\n## What\n\nIt's possible for Postgres to eliminate group pathkeys if it realizes\nthat one of the pathkeys is unique, making the other ones unnecessary.\n\nWe need to handle this case/not panic.\n\n## Why\n\nSee issue.\n\n## How\n\nInject the dropped group pathkeys back into our list of grouping\ncolumns.\n\n## Tests\n\nAdded regression test",
          "timestamp": "2025-09-14T17:56:19-04:00",
          "tree_id": "a41824569d62cfd5dbe40884e6ead540d3b1bd88",
          "url": "https://github.com/paradedb/paradedb/commit/1cfaa7b311ca8b7ee91491411c8dfecd2ce5619c"
        },
        "date": 1757889332454,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.658894,
            "unit": "median cpu",
            "extra": "avg cpu: 20.05024009985133, max cpu: 47.43083, count: 57464"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 166.953125,
            "unit": "median mem",
            "extra": "avg mem: 166.0429221855205, max mem: 171.6171875, count: 57464"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 21892,
            "unit": "median block_count",
            "extra": "avg block_count: 19717.717666713073, max block_count: 23226.0, count: 57464"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 49,
            "unit": "median segment_count",
            "extra": "avg segment_count: 53.74065501879438, max segment_count: 131.0, count: 57464"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 5.691154967253198, max cpu: 28.57143, count: 57464"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 166.41015625,
            "unit": "median mem",
            "extra": "avg mem: 156.16103682849263, max mem: 169.59375, count: 57464"
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
          "id": "a521487756693e82c46bfe2f1a2f2fd3aded0136",
          "message": "fix: fixed `rt_fetch out-of-bounds` error (#3141)\n\n# Ticket(s) Closed\n\n- Closes #3135\n\n## What\n\nFixed `rt_fetch used out-of-bounds` and `Cannot open relation with\noid=0` errors that occurred in complex SQL queries with nested `OR\nEXISTS` clauses, multiple `JOIN`s.\n\n## Why\n\nThe issue occurred when PostgreSQL's query planner generated `Var` nodes\nreferencing Range Table Entries (RTEs) that were valid in outer planning\ncontexts but didn't exist in inner execution contexts. This happened\nspecifically with:\n- `OR EXISTS` subqueries (not `AND EXISTS`)  \n- Multiple `JOIN`s within the `EXISTS` clause\n- ParadeDB functions applied to joined tables\n\nWhen ParadeDB's custom scan tried to access these out-of-bounds RTEs\nusing `rt_fetch`, it caused crashes.\n\n## How\n\nImplemented bounds checking across the codebase:\n\n1. **Early detection**: Added bounds checking in `find_var_relation()`\nto detect invalid `varno` values and return `pg_sys::InvalidOid`. This\nwas the main fix for the issue.\n2. **Graceful handling**: Modified all functions that receive relation\nOIDs to check for `InvalidOid` before attempting to open relations\n3. **Safe fallbacks**: Updated query optimization logic to skip\noptimizations when relation information is unavailable rather than\ncrashing\n\n## Tests\n\nAdded regression test `or_exists_join_bug.sql` covering:\n- Simple queries (baseline functionality)\n- `AND EXISTS` with multiple `JOIN`s (should work)  \n- `OR EXISTS` with multiple `JOIN`s (the problematic case, now fixed)\n- Various edge cases and workarounds\n- Minimal reproduction cases\n\n---------\n\nSigned-off-by: Moe <mdashti@gmail.com>",
          "timestamp": "2025-09-15T02:47:52-07:00",
          "tree_id": "4a0b5db116e0263111295cc53d05810e093ce68c",
          "url": "https://github.com/paradedb/paradedb/commit/a521487756693e82c46bfe2f1a2f2fd3aded0136"
        },
        "date": 1757932025630,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.622696,
            "unit": "median cpu",
            "extra": "avg cpu: 20.245334879546977, max cpu: 55.437916, count: 57769"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 170.8046875,
            "unit": "median mem",
            "extra": "avg mem: 169.9413648675111, max mem: 173.94140625, count: 57769"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 22069,
            "unit": "median block_count",
            "extra": "avg block_count: 19850.683809655697, max block_count: 23430.0, count: 57769"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 49,
            "unit": "median segment_count",
            "extra": "avg segment_count: 53.58254427114888, max segment_count: 133.0, count: 57769"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 5.611056832657908, max cpu: 32.495163, count: 57769"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 165.76953125,
            "unit": "median mem",
            "extra": "avg mem: 154.96212454181742, max mem: 169.19921875, count: 57769"
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
          "id": "b37fc5af676e3438c051381414d81996ed0fb8f6",
          "message": "feat: push down `group by ... order by ... limit` (#3134)\n\n# Ticket(s) Closed\n\n- Closes #3131 \n- Opens #3156 #3155 \n\n## What\n\nPushes down `group by ... order by ... limit` to Tantivy\n\n## Why\n\nBy pushing down the sort/limit to Tantivy, we can significantly speed up\n`group by` queries over high cardinality columns.\n\n## How\n\n- Before we were hard-coding a bucket size and sorting the results\nourselves, now the bucket size is set to the limit and we push the sort\ndown to the Tantivy term agg\n\n## Tests",
          "timestamp": "2025-09-15T15:51:50-04:00",
          "tree_id": "e58df02d60abc13101aaae8ef6333a9afafbcd78",
          "url": "https://github.com/paradedb/paradedb/commit/b37fc5af676e3438c051381414d81996ed0fb8f6"
        },
        "date": 1757968259364,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.640776,
            "unit": "median cpu",
            "extra": "avg cpu: 20.42680839529542, max cpu: 46.64723, count: 57653"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 166.8984375,
            "unit": "median mem",
            "extra": "avg mem: 166.4668376970843, max mem: 171.28515625, count: 57653"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 21713,
            "unit": "median block_count",
            "extra": "avg block_count: 19583.645430419925, max block_count: 23039.0, count: 57653"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 49,
            "unit": "median segment_count",
            "extra": "avg segment_count: 53.50299203857561, max segment_count: 132.0, count: 57653"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 5.740415907011352, max cpu: 28.458496, count: 57653"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 166.74609375,
            "unit": "median mem",
            "extra": "avg mem: 157.20094377948675, max mem: 169.6171875, count: 57653"
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
          "id": "8211eef7a0dd34237afebfa91364fb66c65a4906",
          "message": "perf: remove `ExactBuffer` in favor of a regular rust `BufWriter` (#3158)\n\n# Ticket(s) Closed\n\n- Closes #2981  (/cc @yjhjstz)\n\nWhile #2981 wasn't the impetus for this, it addresses the complaint made\nthere just the same.\n\n## What\n\nIn profiling, our `ExactBuffer` was a large percentage of certain\nprofiles. This replaces it, and its complexity, with a standard Rust\n`BufWriter`.\n\n## Why\n\nImproves performance of (at least) our `wide-table.toml` test's \"Single\nUpdate\" job by quite a bit.\n\n<img width=\"720\" height=\"141\" alt=\"screenshot_2025-09-15_at_3 28\n33___pm_720\"\nsrc=\"https://github.com/user-attachments/assets/a373a7ae-df38-4691-980a-d6843f073d26\"\n/>\n\n\n## How\n\n## Tests\n\nExisting tests pass",
          "timestamp": "2025-09-15T15:55:52-04:00",
          "tree_id": "4ddf140542c5525034023441aadac4b634c90fc6",
          "url": "https://github.com/paradedb/paradedb/commit/8211eef7a0dd34237afebfa91364fb66c65a4906"
        },
        "date": 1757968503172,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.677044,
            "unit": "median cpu",
            "extra": "avg cpu: 19.700043565182213, max cpu: 53.118713, count: 57572"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 163.4296875,
            "unit": "median mem",
            "extra": "avg mem: 162.6317740584616, max mem: 166.2265625, count: 57572"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 22483,
            "unit": "median block_count",
            "extra": "avg block_count: 20250.06720280692, max block_count: 24632.0, count: 57572"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 50,
            "unit": "median segment_count",
            "extra": "avg segment_count: 54.59125964010283, max segment_count: 136.0, count: 57572"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.673807,
            "unit": "median cpu",
            "extra": "avg cpu: 5.4410108164948525, max cpu: 27.799229, count: 57572"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 160.90625,
            "unit": "median mem",
            "extra": "avg mem: 149.96735894293232, max mem: 162.04296875, count: 57572"
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
          "id": "288a4bfa0c79838d86711b8a6231687c984ac0b5",
          "message": "chore: Upgrade to `0.18.3` (#3160)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-09-15T16:13:06-04:00",
          "tree_id": "ad59a6c86e8afe29cabad5b0bcc6a78bc448182e",
          "url": "https://github.com/paradedb/paradedb/commit/288a4bfa0c79838d86711b8a6231687c984ac0b5"
        },
        "date": 1757969531677,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.658894,
            "unit": "median cpu",
            "extra": "avg cpu: 20.012480540045868, max cpu: 55.652172, count: 57386"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 168.37109375,
            "unit": "median mem",
            "extra": "avg mem: 167.44835065662792, max mem: 168.74609375, count: 57386"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 22963,
            "unit": "median block_count",
            "extra": "avg block_count: 20619.963841355035, max block_count: 24247.0, count: 57386"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 49,
            "unit": "median segment_count",
            "extra": "avg segment_count: 54.373697417488586, max segment_count: 138.0, count: 57386"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 5.416633996328867, max cpu: 27.934044, count: 57386"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 163.09765625,
            "unit": "median mem",
            "extra": "avg mem: 152.91570495471456, max mem: 165.46484375, count: 57386"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "af5bea23effe976b411147e259e53afad947a393",
          "message": "perf: remove `ExactBuffer` in favor of a regular rust `BufWriter` (#3159)\n\n# Ticket(s) Closed\n\n- Closes #2981  (/cc @yjhjstz)\n\nWhile #2981 wasn't the impetus for this, it addresses the complaint made\nthere just the same.\n\n## What\n\nIn profiling, our `ExactBuffer` was a large percentage of certain\nprofiles. This replaces it, and its complexity, with a standard Rust\n`BufWriter`.\n\n## Why\n\nImproves performance of (at least) our `wide-table.toml` test's \"Single\nUpdate\" job by quite a bit.\n\n<img width=\"720\" height=\"141\" alt=\"screenshot_2025-09-15_at_3 28\n33___pm_720\"\nsrc=\"https://github.com/user-attachments/assets/a373a7ae-df38-4691-980a-d6843f073d26\"\n/>\n\n\n## How\n\n## Tests\n\nExisting tests pass\n\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>",
          "timestamp": "2025-09-15T16:26:19-04:00",
          "tree_id": "cbc00b9a93c129255360f60e5a70904e87f1e8c1",
          "url": "https://github.com/paradedb/paradedb/commit/af5bea23effe976b411147e259e53afad947a393"
        },
        "date": 1757970392551,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.640776,
            "unit": "median cpu",
            "extra": "avg cpu: 19.676743279234536, max cpu: 51.014496, count: 57694"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 164.9140625,
            "unit": "median mem",
            "extra": "avg mem: 163.86741214966028, max mem: 168.125, count: 57694"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 22429,
            "unit": "median block_count",
            "extra": "avg block_count: 20239.711633792074, max block_count: 24713.0, count: 57694"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 49,
            "unit": "median segment_count",
            "extra": "avg segment_count: 54.10843415259819, max segment_count: 136.0, count: 57694"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 5.46851229381257, max cpu: 28.09756, count: 57694"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 165.1171875,
            "unit": "median mem",
            "extra": "avg mem: 154.93150934239262, max mem: 165.8671875, count: 57694"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "7800d096e107acdbdec6297d0cb98ef030569e2b",
          "message": "chore: Upgrade to `0.18.3` (#3161)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-09-15T16:36:57-04:00",
          "tree_id": "c0962cc02d5690156721fd003c985f724ee9b20f",
          "url": "https://github.com/paradedb/paradedb/commit/7800d096e107acdbdec6297d0cb98ef030569e2b"
        },
        "date": 1757971009046,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.677044,
            "unit": "median cpu",
            "extra": "avg cpu: 19.946495000819485, max cpu: 47.151276, count: 57840"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 169.56640625,
            "unit": "median mem",
            "extra": "avg mem: 168.86389123011756, max mem: 169.56640625, count: 57840"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 22720,
            "unit": "median block_count",
            "extra": "avg block_count: 20465.07237206086, max block_count: 24236.0, count: 57840"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 49,
            "unit": "median segment_count",
            "extra": "avg segment_count: 54.2524550484094, max segment_count: 140.0, count: 57840"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.673807,
            "unit": "median cpu",
            "extra": "avg cpu: 5.469076277596052, max cpu: 23.483368, count: 57840"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 164.2109375,
            "unit": "median mem",
            "extra": "avg mem: 152.4190586369727, max mem: 164.5859375, count: 57840"
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
          "id": "f71a5572d645d23e58b949cc3f16645473c74735",
          "message": "chore: Sync `0.18.x` (#3162)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\n---------\n\nSigned-off-by: Moe <mdashti@gmail.com>\nCo-authored-by: Moe <mdashti@gmail.com>",
          "timestamp": "2025-09-15T17:11:39-04:00",
          "tree_id": "a75daf7f281149ef4317505338649d8b0d2ec8a4",
          "url": "https://github.com/paradedb/paradedb/commit/f71a5572d645d23e58b949cc3f16645473c74735"
        },
        "date": 1757973057225,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.713451,
            "unit": "median cpu",
            "extra": "avg cpu: 20.133039007548106, max cpu: 55.813957, count: 57364"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 168.3828125,
            "unit": "median mem",
            "extra": "avg mem: 167.59327973010076, max mem: 168.3828125, count: 57364"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 22994,
            "unit": "median block_count",
            "extra": "avg block_count: 20663.317585942405, max block_count: 24398.0, count: 57364"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 49,
            "unit": "median segment_count",
            "extra": "avg segment_count: 54.31559863328917, max segment_count: 151.0, count: 57364"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.678363,
            "unit": "median cpu",
            "extra": "avg cpu: 5.452535654149671, max cpu: 28.070175, count: 57364"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 163.98046875,
            "unit": "median mem",
            "extra": "avg mem: 152.79084885827783, max mem: 165.140625, count: 57364"
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
          "id": "878c50feef96d61785ad711ebe46250c920bed70",
          "message": "fix: sequential scan segfault (#3163)\n\n# Ticket(s) Closed\n\n- Closes #3151 \n\n## What\n\nThe `@@@` return type should be `bool`, not `SearchQueryInput`.\n\n## Why\n\n## How\n\n## Tests\n\nAdded regression test.",
          "timestamp": "2025-09-16T10:27:13-04:00",
          "tree_id": "6859469869310b79c8c32af68b3ed77dfb787362",
          "url": "https://github.com/paradedb/paradedb/commit/878c50feef96d61785ad711ebe46250c920bed70"
        },
        "date": 1758035202339,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.640776,
            "unit": "median cpu",
            "extra": "avg cpu: 19.668706285312755, max cpu: 52.64207, count: 57647"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 167.68359375,
            "unit": "median mem",
            "extra": "avg mem: 166.42489343818846, max mem: 168.97265625, count: 57647"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 22433,
            "unit": "median block_count",
            "extra": "avg block_count: 20241.412545318923, max block_count: 24227.0, count: 57647"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 49,
            "unit": "median segment_count",
            "extra": "avg segment_count: 54.189688969070374, max segment_count: 137.0, count: 57647"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 5.419130669108517, max cpu: 27.988338, count: 57647"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 161.3515625,
            "unit": "median mem",
            "extra": "avg mem: 150.91435556869828, max mem: 164.0546875, count: 57647"
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
          "id": "f2a0c9c43e4385628cc7b828a8ed12c30e55050e",
          "message": "chore: don't warn about `raw` tokenizer on UUID key fields (#3166)\n\n## What\n\nRemove the warning about using the `raw` tokenizer when the `key_field`\nis a UUID field.\n\nThe drama here is that we (pg_search) assign the `raw` tokenizer to UUID\nfields used as the key_field so there's nothing a user can do about it.\nWarning about our own bad decisions is not cool.\n\n## Why\n\nMany user and customer complaints.\n\n## How\n\n## Tests\n\nExisting tests pass but a couple of the regression test expected output\nis now different.",
          "timestamp": "2025-09-16T13:10:47-04:00",
          "tree_id": "2b24aea6e3a0645c584d8ebb8ce7465c8c90f904",
          "url": "https://github.com/paradedb/paradedb/commit/f2a0c9c43e4385628cc7b828a8ed12c30e55050e"
        },
        "date": 1758045009854,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.677044,
            "unit": "median cpu",
            "extra": "avg cpu: 20.073538649100946, max cpu: 72.14429, count: 58021"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 168.87890625,
            "unit": "median mem",
            "extra": "avg mem: 167.66160750256373, max mem: 168.87890625, count: 58021"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 22594,
            "unit": "median block_count",
            "extra": "avg block_count: 20365.76579169611, max block_count: 25239.0, count: 58021"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 50,
            "unit": "median segment_count",
            "extra": "avg segment_count: 54.68828527602075, max segment_count: 135.0, count: 58021"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.669261,
            "unit": "median cpu",
            "extra": "avg cpu: 5.43765187421511, max cpu: 23.346306, count: 58021"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 165.20703125,
            "unit": "median mem",
            "extra": "avg mem: 154.71895042581565, max mem: 165.5859375, count: 58021"
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
          "id": "489eb48583040067612195f9e1406d5e31a1599f",
          "message": "perf: teach custom scan callback to exit early if it can (#3168)\n\n## What\n\nThis does two things.  \n\nOne, the first commit (62d752572b2d7bc5a02b7203ac2c83949e38e27e) simply\nreorders some code in the custom scan callback so it can decide to exit\nearly if we're not going to submit a path. Specifically, this is\nintended to avoid opening a Directory and Index and related structures.\n\nTwo, the second commit (5ac1dde23ef0809bea4b942d04fd14acc9d1c152) makes\na new decision to not evaluate possible pushdown predicates when the\nstatement type is not a SELECT statement. This cuts out the overhead of\nneeding to read/deserialize the index's schema at all on (at least)\nUPDATE statements.\n\nThis does mean that we won't consider doing pushdowns for UPDATE\nstatements, even if doing one would make the UPDATE scan faster.\n\n## Why\n\nTrying to reduce per-query overhead, targeting our stressgres benchmarks\nlike \"single-server.toml\" and \"wide-table.toml\".\n\n## How\n\n## Tests\n\nAll existing tests pass.",
          "timestamp": "2025-09-16T17:39:51-04:00",
          "tree_id": "0ebcd01c6225cbb43b199470f7f78bd694493ed7",
          "url": "https://github.com/paradedb/paradedb/commit/489eb48583040067612195f9e1406d5e31a1599f"
        },
        "date": 1758061147177,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.658894,
            "unit": "median cpu",
            "extra": "avg cpu: 19.424036583037342, max cpu: 56.916992, count: 57664"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 161.4921875,
            "unit": "median mem",
            "extra": "avg mem: 160.512625538681, max mem: 165.48828125, count: 57664"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 23143,
            "unit": "median block_count",
            "extra": "avg block_count: 20830.29722183685, max block_count: 25186.0, count: 57664"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 50,
            "unit": "median segment_count",
            "extra": "avg segment_count: 55.046875, max segment_count: 155.0, count: 57664"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.669261,
            "unit": "median cpu",
            "extra": "avg cpu: 5.393057604202729, max cpu: 27.906979, count: 57664"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 160.69921875,
            "unit": "median mem",
            "extra": "avg mem: 150.71746805849403, max mem: 162.2578125, count: 57664"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "63daa7f2bf568127e538f19f942d6363508ca615",
          "message": "chore: don't warn about `raw` tokenizer on UUID key fields (#3167)\n\n## What\n\nRemove the warning about using the `raw` tokenizer when the `key_field`\nis a UUID field.\n\nThe drama here is that we (pg_search) assign the `raw` tokenizer to UUID\nfields used as the key_field so there's nothing a user can do about it.\nWarning about our own bad decisions is not cool.\n\n## Why\n\nMany user and customer complaints.\n\n## How\n\n## Tests\n\nExisting tests pass but a couple of the regression test expected output\nis now different.\n\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>",
          "timestamp": "2025-09-17T10:06:31-04:00",
          "tree_id": "2c472616485a1c2a1ed61c7f2c030286882deb06",
          "url": "https://github.com/paradedb/paradedb/commit/63daa7f2bf568127e538f19f942d6363508ca615"
        },
        "date": 1758120361123,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.640776,
            "unit": "median cpu",
            "extra": "avg cpu: 19.623204315235817, max cpu: 43.59233, count: 56889"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 171.85546875,
            "unit": "median mem",
            "extra": "avg mem: 170.78271900653027, max mem: 172.234375, count: 56889"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 22930,
            "unit": "median block_count",
            "extra": "avg block_count: 20644.549385645732, max block_count: 24851.0, count: 56889"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 50,
            "unit": "median segment_count",
            "extra": "avg segment_count: 54.682311167361, max segment_count: 135.0, count: 56889"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 5.290000936713864, max cpu: 28.290766, count: 56889"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 166.6328125,
            "unit": "median mem",
            "extra": "avg mem: 155.97824299293362, max mem: 166.6328125, count: 56889"
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
          "id": "eb456f8c97d99e92e2795d88dd2c1082c13c83a6",
          "message": "perf: optimize `Timestamp` and `JsonB` datum decoding (#3171)\n\n## What\n\nOptimize `Timestamp` and `JsonB` to `TantivyValue` datum conversions.\n\nThese two show up quite high in profiles. The `JsonB` conversion in\nparticular has been bad due to how pgrx stupidly (I can say it) handles\n`JsonB` values by converting them to strings and then asking serde to\nparse the strings.\n\n## Why\n\nTrying to make things faster.\n\n## How\n\nFor the `Timestamp` conversion we memoize Postgres' understanding of the\ncurrent EPOCH and do the same math that it does to calculate a time\nvalue.\n\nFor the `JsonB` conversion we implement our own deserializer routine\nusing Postgres' internal `JsonbIteratorInit()` and `JsonbIteratorNext()`\nfunctions, building up a `serde_json::Value` structure as it goes.\n\n\n## Tests\n\nA new `#[pg_test]`-based proptest has been added to test our custom\njsonb deserializer against normal serde.\n\n---------\n\nSigned-off-by: Eric Ridge <eebbrr@gmail.com>\nCo-authored-by: Stu Hood <stuhood@paradedb.com>",
          "timestamp": "2025-09-17T15:26:06-04:00",
          "tree_id": "702cea735a514e9b33d6c1ee785606d39d4f705c",
          "url": "https://github.com/paradedb/paradedb/commit/eb456f8c97d99e92e2795d88dd2c1082c13c83a6"
        },
        "date": 1758139613327,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.622696,
            "unit": "median cpu",
            "extra": "avg cpu: 17.746835301100056, max cpu: 57.657658, count: 57411"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 166.4375,
            "unit": "median mem",
            "extra": "avg mem: 164.5500956779842, max mem: 166.4375, count: 57411"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 41220,
            "unit": "median block_count",
            "extra": "avg block_count: 36061.696939610876, max block_count: 42784.0, count: 57411"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 52,
            "unit": "median segment_count",
            "extra": "avg segment_count: 58.27141140199613, max segment_count: 158.0, count: 57411"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.678363,
            "unit": "median cpu",
            "extra": "avg cpu: 5.41323945327829, max cpu: 28.015566, count: 57411"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 160.875,
            "unit": "median mem",
            "extra": "avg mem: 151.6666939507455, max mem: 162.984375, count: 57411"
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
          "id": "849076799ca599dfbf0f2415149b12495b24624c",
          "message": "fix: Always use a shared expression context to execute HeapFilters. (#3174)\n\n## What\n\nEvaluating heap filters was leaking a `CreateStandaloneExprContext` per\nexecution, which could eventually lead to OOMs.\n\n## Why\n\n`PgBox::from_pg` does not free the resulting memory: it's expected to be\nfreed by a memory context, since PG created it.\n\n## How\n\nAlways use a global expression context created by\n`ExecAssignExprContext` in `begin_custom_scan`.\n\n## Tests\n\nA repro uses significantly less memory, and [a benchmark query added for\nthis purpose](https://github.com/paradedb/paradedb/pull/3175) is faster.",
          "timestamp": "2025-09-17T16:44:32-07:00",
          "tree_id": "7eef1c518a935389aa23e91c6bc47bbc325b18e6",
          "url": "https://github.com/paradedb/paradedb/commit/849076799ca599dfbf0f2415149b12495b24624c"
        },
        "date": 1758155044702,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.60465,
            "unit": "median cpu",
            "extra": "avg cpu: 18.04943002684695, max cpu: 55.598457, count: 56558"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 163.875,
            "unit": "median mem",
            "extra": "avg mem: 163.33486179287192, max mem: 167.84375, count: 56558"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 41174,
            "unit": "median block_count",
            "extra": "avg block_count: 36208.008416139186, max block_count: 42876.0, count: 56558"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 52,
            "unit": "median segment_count",
            "extra": "avg segment_count: 58.566427384278086, max segment_count: 164.0, count: 56558"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.669261,
            "unit": "median cpu",
            "extra": "avg cpu: 5.458769467761314, max cpu: 28.458496, count: 56558"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 163.98828125,
            "unit": "median mem",
            "extra": "avg mem: 153.87440361277362, max mem: 165.13671875, count: 56558"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "dc0e87d6fdabdb573d6f37a24c6c33befa43e8b0",
          "message": "fix: Always use a shared expression context to execute HeapFilters. (#3176)\n\n## What\n\nEvaluating heap filters was leaking a `CreateStandaloneExprContext` per\nexecution, which could eventually lead to OOMs.\n\n## Why\n\n`PgBox::from_pg` does not free the resulting memory: it's expected to be\nfreed by a memory context, since PG created it.\n\n## How\n\nAlways use a global expression context created by\n`ExecAssignExprContext` in `begin_custom_scan`.\n\n## Tests\n\nA repro uses significantly less memory, and [a benchmark query added for\nthis purpose](https://github.com/paradedb/paradedb/pull/3175) is faster.\n\nCo-authored-by: Stu Hood <stuhood@paradedb.com>",
          "timestamp": "2025-09-17T17:11:43-07:00",
          "tree_id": "0c30f446ad8404b4f66727777f1b6e6a5bc8958e",
          "url": "https://github.com/paradedb/paradedb/commit/dc0e87d6fdabdb573d6f37a24c6c33befa43e8b0"
        },
        "date": 1758156688166,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.60465,
            "unit": "median cpu",
            "extra": "avg cpu: 17.729854547438848, max cpu: 57.715435, count: 57745"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 163.484375,
            "unit": "median mem",
            "extra": "avg mem: 162.73732459249717, max mem: 166.65625, count: 57745"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 41005,
            "unit": "median block_count",
            "extra": "avg block_count: 36038.62117932289, max block_count: 42653.0, count: 57745"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 52,
            "unit": "median segment_count",
            "extra": "avg segment_count: 58.23621092735302, max segment_count: 166.0, count: 57745"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.673807,
            "unit": "median cpu",
            "extra": "avg cpu: 5.382849816154208, max cpu: 28.290766, count: 57745"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 161.69140625,
            "unit": "median mem",
            "extra": "avg mem: 152.13436999415535, max mem: 163.24609375, count: 57745"
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
          "id": "3bcb1451087be74b7bd73bfc7d6546423046a0ce",
          "message": "fix: write all delete files atomically (#3178)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nIn `ambulkdelete`, write all delete files to the meta list atomically,\ninstead of one at a time.\n\n## Why\n\nReduces the number of writes to the metas list.\n\n## How\n\n## Tests",
          "timestamp": "2025-09-18T16:03:10-04:00",
          "tree_id": "ad9609f0419a34b8f0cf543e911c1dc3c25d4563",
          "url": "https://github.com/paradedb/paradedb/commit/3bcb1451087be74b7bd73bfc7d6546423046a0ce"
        },
        "date": 1758228174190,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.58664,
            "unit": "median cpu",
            "extra": "avg cpu: 17.86309858654299, max cpu: 58.181816, count: 56987"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 163.85546875,
            "unit": "median mem",
            "extra": "avg mem: 162.82764078868865, max mem: 166.69140625, count: 56987"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 40746,
            "unit": "median block_count",
            "extra": "avg block_count: 35662.05443346728, max block_count: 41631.0, count: 56987"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 52,
            "unit": "median segment_count",
            "extra": "avg segment_count: 58.127976556056645, max segment_count: 152.0, count: 56987"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 5.396999901402886, max cpu: 23.483368, count: 56987"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 159.39453125,
            "unit": "median mem",
            "extra": "avg mem: 149.88235481612034, max mem: 161.0390625, count: 56987"
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
          "id": "6e11875ba052ccd6937ca0c535b3803309c8b6eb",
          "message": "feat: removed aggregation limitations re mix of aggregate functions and aggregation on group-by column. (#3179)\n\n# Ticket(s) Closed\n\n- Closes #2963\n\n## What\n\nRemoves aggregate limitations that prevented queries where the same\nfield is used in both `GROUP BY` and aggregate functions (e.g., `SELECT\nrating, AVG(rating) FROM table GROUP BY rating`).\n\n## Why\n\nPrevious safety checks blocked these queries due to Tantivy's\n\"incompatible fruit types\" errors, but testing shows the underlying\nissue is resolved. The limitations were overly restrictive and caused\nunnecessary fallbacks to slower PostgreSQL aggregation.\n\n## How\n\n- Removed `has_search_field_conflicts` function and field conflict\nvalidation\n- Eliminated ~35 lines of restrictive code in\n`extract_and_validate_aggregates`\n- Previously blocked queries now use faster `AggregateScan` instead of\n`GroupAggregate`\n\n## Tests\n\n- **`aggregate-groupby-conflict.sql`** - Tests `GROUP BY field` with\naggregates on same field\n- **`test-fruit-types-issue.sql`** - Validates #2963 issue resolution  \n- **`groupby_aggregate.out`** - Updated expectations showing\n`AggregateScan` usage",
          "timestamp": "2025-09-18T16:00:25-07:00",
          "tree_id": "f85924512f419186b824a986dd35eaa96d973884",
          "url": "https://github.com/paradedb/paradedb/commit/6e11875ba052ccd6937ca0c535b3803309c8b6eb"
        },
        "date": 1758238806569,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.58664,
            "unit": "median cpu",
            "extra": "avg cpu: 18.093722797306825, max cpu: 57.773315, count: 58022"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 165.59375,
            "unit": "median mem",
            "extra": "avg mem: 164.92298913827943, max mem: 168.71484375, count: 58022"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 41034.5,
            "unit": "median block_count",
            "extra": "avg block_count: 35833.66698838372, max block_count: 41843.0, count: 58022"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 52,
            "unit": "median segment_count",
            "extra": "avg segment_count: 58.40060666643687, max segment_count: 161.0, count: 58022"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 5.51789848435675, max cpu: 28.042841, count: 58022"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 165.921875,
            "unit": "median mem",
            "extra": "avg mem: 155.96936042309383, max mem: 165.921875, count: 58022"
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
          "id": "020f92b742187fe6fdc75a19390692e6d2e9a373",
          "message": "feat: Port `ambulkdelete_epoch` to community (#3180)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nAdd `ambulkdelete_epoch` to the index metadata and make sure it's passed\ndown to all scans. In enterprise, this value is used to detect query\nconflicts with concurrent vacuums on the primary.\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-09-18T19:16:09-04:00",
          "tree_id": "3642b293b38caa7676318f888b910c3f934e1976",
          "url": "https://github.com/paradedb/paradedb/commit/020f92b742187fe6fdc75a19390692e6d2e9a373"
        },
        "date": 1758239766616,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.532818,
            "unit": "median cpu",
            "extra": "avg cpu: 17.388314461055106, max cpu: 53.118713, count: 57612"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 164.35546875,
            "unit": "median mem",
            "extra": "avg mem: 163.53101011399534, max mem: 167.73046875, count: 57612"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 40873.5,
            "unit": "median block_count",
            "extra": "avg block_count: 35686.718096924254, max block_count: 41661.0, count: 57612"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 52,
            "unit": "median segment_count",
            "extra": "avg segment_count: 58.17178712768173, max segment_count: 165.0, count: 57612"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.673807,
            "unit": "median cpu",
            "extra": "avg cpu: 5.418948135930527, max cpu: 28.57143, count: 57612"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 164.72265625,
            "unit": "median mem",
            "extra": "avg mem: 154.3793956499948, max mem: 164.72265625, count: 57612"
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
          "id": "c0dc0e674f3524c2f65a7b387ccbf594b23e5e0e",
          "message": "chore: Upgrade to `0.18.4` (#3181)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\n---------\n\nSigned-off-by: Ming <ming.ying.nyc@gmail.com>\nCo-authored-by: Stu Hood <stuhood@paradedb.com>",
          "timestamp": "2025-09-18T19:18:34-04:00",
          "tree_id": "b67f22553ed7786ef556afbfad2b7f8ddc6b139e",
          "url": "https://github.com/paradedb/paradedb/commit/c0dc0e674f3524c2f65a7b387ccbf594b23e5e0e"
        },
        "date": 1758240001017,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.568666,
            "unit": "median cpu",
            "extra": "avg cpu: 17.456832708458688, max cpu: 53.49544, count: 57625"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 162.66796875,
            "unit": "median mem",
            "extra": "avg mem: 162.37941370661605, max mem: 167.21484375, count: 57625"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 40867,
            "unit": "median block_count",
            "extra": "avg block_count: 35718.887618221255, max block_count: 41701.0, count: 57625"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 52,
            "unit": "median segment_count",
            "extra": "avg segment_count: 58.38127548806941, max segment_count: 172.0, count: 57625"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.678363,
            "unit": "median cpu",
            "extra": "avg cpu: 5.483746947937417, max cpu: 27.906979, count: 57625"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 160.484375,
            "unit": "median mem",
            "extra": "avg mem: 151.18655809381778, max mem: 162.1484375, count: 57625"
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
          "id": "a97c525d0c11314fe01c8bbb250ea5fdf73ec8ce",
          "message": "fix: write all delete files atomically (#3178) (#3182)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nIn `ambulkdelete`, write all delete files to the meta list atomically,\ninstead of one at a time.\n\n## Why\n\nReduces the number of writes to the metas list.\n\n## How\n\n## Tests\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-09-18T21:50:00-04:00",
          "tree_id": "ba5917ed034f24a8e2ad95a64751e5faef3d55d5",
          "url": "https://github.com/paradedb/paradedb/commit/a97c525d0c11314fe01c8bbb250ea5fdf73ec8ce"
        },
        "date": 1758249003174,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.60465,
            "unit": "median cpu",
            "extra": "avg cpu: 17.741109929702752, max cpu: 56.085682, count: 57774"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 167.56640625,
            "unit": "median mem",
            "extra": "avg mem: 165.64703717556773, max mem: 168.31640625, count: 57774"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 40637,
            "unit": "median block_count",
            "extra": "avg block_count: 35523.15069062208, max block_count: 41528.0, count: 57774"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 52,
            "unit": "median segment_count",
            "extra": "avg segment_count: 58.1435247689272, max segment_count: 173.0, count: 57774"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.678363,
            "unit": "median cpu",
            "extra": "avg cpu: 5.449814496031826, max cpu: 28.514853, count: 57774"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 160.89453125,
            "unit": "median mem",
            "extra": "avg mem: 150.61594552426266, max mem: 163.1171875, count: 57774"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "e15e51abfc4b7834faea068d861d91d5d873580f",
          "message": "chore: Upgrade to `0.18.4` (#3184)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\nSigned-off-by: Ming <ming.ying.nyc@gmail.com>\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>\nCo-authored-by: Stu Hood <stuhood@paradedb.com>",
          "timestamp": "2025-09-18T21:52:13-04:00",
          "tree_id": "3d203e3468a4e7504d03af9c39ac9a0869033086",
          "url": "https://github.com/paradedb/paradedb/commit/e15e51abfc4b7834faea068d861d91d5d873580f"
        },
        "date": 1758249176436,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.550726,
            "unit": "median cpu",
            "extra": "avg cpu: 17.51975894975282, max cpu: 55.868088, count: 57685"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 167.84375,
            "unit": "median mem",
            "extra": "avg mem: 166.975619542017, max mem: 167.84375, count: 57685"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 40598,
            "unit": "median block_count",
            "extra": "avg block_count: 35447.299748634825, max block_count: 41429.0, count: 57685"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 52,
            "unit": "median segment_count",
            "extra": "avg segment_count: 58.18060154286209, max segment_count: 167.0, count: 57685"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.673807,
            "unit": "median cpu",
            "extra": "avg cpu: 5.501378008401619, max cpu: 28.042841, count: 57685"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 160.76953125,
            "unit": "median mem",
            "extra": "avg mem: 151.687679923832, max mem: 162.71484375, count: 57685"
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
          "id": "1046018b2db9614ef172bd802c98a3987da7513e",
          "message": "chore: small diff ported from enterprise for query cancel fix (#3186)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nSome small changes in enterprise that should be in community\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-09-18T21:53:42-04:00",
          "tree_id": "85ed1f4eb7261157deabdfba479dc61164775f99",
          "url": "https://github.com/paradedb/paradedb/commit/1046018b2db9614ef172bd802c98a3987da7513e"
        },
        "date": 1758249252261,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.58664,
            "unit": "median cpu",
            "extra": "avg cpu: 17.57618804491097, max cpu: 57.88945, count: 57780"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 164.68359375,
            "unit": "median mem",
            "extra": "avg mem: 164.06713449723088, max mem: 168.078125, count: 57780"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 40601,
            "unit": "median block_count",
            "extra": "avg block_count: 35645.68508134303, max block_count: 41504.0, count: 57780"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 52,
            "unit": "median segment_count",
            "extra": "avg segment_count: 58.25867082035306, max segment_count: 146.0, count: 57780"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.678363,
            "unit": "median cpu",
            "extra": "avg cpu: 5.487700680965582, max cpu: 28.09756, count: 57780"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 162.92578125,
            "unit": "median mem",
            "extra": "avg mem: 153.17025462962962, max mem: 162.92578125, count: 57780"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "f052aabf25719cee68a756a379c6b66e39452759",
          "message": "feat: Port `ambulkdelete_epoch` to community (#3183)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nAdd `ambulkdelete_epoch` to the index metadata and make sure it's passed\ndown to all scans. In enterprise, this value is used to detect query\nconflicts with concurrent vacuums on the primary.\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-09-18T22:01:15-04:00",
          "tree_id": "48ffae94b2f43d5c2d62b5adb846d1dcc2992aee",
          "url": "https://github.com/paradedb/paradedb/commit/f052aabf25719cee68a756a379c6b66e39452759"
        },
        "date": 1758249663534,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.497108,
            "unit": "median cpu",
            "extra": "avg cpu: 17.249189026104606, max cpu: 53.118713, count: 57661"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 168.07421875,
            "unit": "median mem",
            "extra": "avg mem: 167.3892079758199, max mem: 168.07421875, count: 57661"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 40651,
            "unit": "median block_count",
            "extra": "avg block_count: 35631.1738436725, max block_count: 41548.0, count: 57661"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 52,
            "unit": "median segment_count",
            "extra": "avg segment_count: 58.26928079637884, max segment_count: 154.0, count: 57661"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.673807,
            "unit": "median cpu",
            "extra": "avg cpu: 5.498468074949073, max cpu: 23.346306, count: 57661"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 161.234375,
            "unit": "median mem",
            "extra": "avg mem: 152.08121932502038, max mem: 163.85546875, count: 57661"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "153f632ba06057571459a4b6e8767c135baf438c",
          "message": "chore: small diff ported from enterprise for query cancel fix (#3187)",
          "timestamp": "2025-09-18T22:31:35-04:00",
          "tree_id": "2c3b3f692c24ba8540a69da9d41f4d3a24d4ae6f",
          "url": "https://github.com/paradedb/paradedb/commit/153f632ba06057571459a4b6e8767c135baf438c"
        },
        "date": 1758251483557,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.568666,
            "unit": "median cpu",
            "extra": "avg cpu: 17.91094392803629, max cpu: 58.006042, count: 56981"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 213.44140625,
            "unit": "median mem",
            "extra": "avg mem: 204.36049749587582, max mem: 243.6796875, count: 56981"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 39876,
            "unit": "median block_count",
            "extra": "avg block_count: 34955.63168424563, max block_count: 40770.0, count: 56981"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 52,
            "unit": "median segment_count",
            "extra": "avg segment_count: 58.33386567452309, max segment_count: 153.0, count: 56981"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 5.469018114341952, max cpu: 28.235296, count: 56981"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 160.62890625,
            "unit": "median mem",
            "extra": "avg mem: 151.06046437079902, max mem: 163.390625, count: 56981"
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
          "id": "8101a67174703310a6a1655496fd5296e869901d",
          "message": "fix: Clone an Arc rather than a OnceLock. (#3185)\n\n## What\n\nInvert our use of `OnceLock` to ensure that we clone an\n`Arc<OnceLock<T>>` rather than a `OnceLock<Arc<T>>`.\n\n## Why\n\n`OnceLock` implements `Clone` by cloning its contents to create a\nseparate disconnected copy. If what is desired is \"exactly once\nbehavior\", then cloning the `OnceLock` before it has been computed the\nfirst time will defeat that.\n\nThis change has no impact on benchmarks in this case, but\n`Arc<OnceLock<T>>` matches the intent of this code, and sets a better\nexample for future us.\n\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>",
          "timestamp": "2025-09-19T15:01:21-07:00",
          "tree_id": "de6adf9a09b874a0e133e9cbfeca50d417e6c5bf",
          "url": "https://github.com/paradedb/paradedb/commit/8101a67174703310a6a1655496fd5296e869901d"
        },
        "date": 1758321675539,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.550726,
            "unit": "median cpu",
            "extra": "avg cpu: 17.380043885290004, max cpu: 53.22581, count: 57619"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 189.08984375,
            "unit": "median mem",
            "extra": "avg mem: 184.58324619483156, max mem: 242.66015625, count: 57619"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 39750,
            "unit": "median block_count",
            "extra": "avg block_count: 34807.172286919245, max block_count: 40626.0, count: 57619"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 52,
            "unit": "median segment_count",
            "extra": "avg segment_count: 58.20508859924678, max segment_count: 144.0, count: 57619"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.678363,
            "unit": "median cpu",
            "extra": "avg cpu: 5.458941732017832, max cpu: 23.809525, count: 57619"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 160.57421875,
            "unit": "median mem",
            "extra": "avg mem: 151.54639318464828, max mem: 161.38671875, count: 57619"
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
          "id": "3163a5f3e48d3027585287ce8a63074f70ba1836",
          "message": "perf: Configurable Top N requeries more granularly (#3190)",
          "timestamp": "2025-09-19T21:06:04-04:00",
          "tree_id": "8c74bdf97c37281e4641be0e94b4d464daa5a3ea",
          "url": "https://github.com/paradedb/paradedb/commit/3163a5f3e48d3027585287ce8a63074f70ba1836"
        },
        "date": 1758332760211,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.60465,
            "unit": "median cpu",
            "extra": "avg cpu: 17.72951450196891, max cpu: 56.14035, count: 57763"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 167.953125,
            "unit": "median mem",
            "extra": "avg mem: 167.16273879905822, max mem: 167.953125, count: 57763"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 40755,
            "unit": "median block_count",
            "extra": "avg block_count: 35618.45991378564, max block_count: 41622.0, count: 57763"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 52,
            "unit": "median segment_count",
            "extra": "avg segment_count: 58.07084119592126, max segment_count: 172.0, count: 57763"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.673807,
            "unit": "median cpu",
            "extra": "avg cpu: 5.447782569254149, max cpu: 28.015566, count: 57763"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 158.80859375,
            "unit": "median mem",
            "extra": "avg mem: 149.77536550213804, max mem: 161.06640625, count: 57763"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "f573a31e6704d95d0a62271a23ba47658a1dae06",
          "message": "perf: Configurable Top N requeries more granularly (#3194)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nAllow the retry scale factor and max chunk size to be tuned, which is\nuseful for reducing Top N requeries.\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-09-20T09:26:21-04:00",
          "tree_id": "d4ee2092267660be53cb68f8b760756a5a07ab69",
          "url": "https://github.com/paradedb/paradedb/commit/f573a31e6704d95d0a62271a23ba47658a1dae06"
        },
        "date": 1758377174944,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.60465,
            "unit": "median cpu",
            "extra": "avg cpu: 18.051949449300416, max cpu: 58.06452, count: 57979"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 164.2421875,
            "unit": "median mem",
            "extra": "avg mem: 163.71157903076977, max mem: 169.5078125, count: 57979"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 40953,
            "unit": "median block_count",
            "extra": "avg block_count: 35870.83073181669, max block_count: 41832.0, count: 57979"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 52,
            "unit": "median segment_count",
            "extra": "avg segment_count: 58.285913865365046, max segment_count: 170.0, count: 57979"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.673807,
            "unit": "median cpu",
            "extra": "avg cpu: 5.516299398329174, max cpu: 28.486649, count: 57979"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 165.84765625,
            "unit": "median mem",
            "extra": "avg mem: 156.83373014097776, max mem: 166.59765625, count: 57979"
          }
        ]
      }
    ]
  }
}