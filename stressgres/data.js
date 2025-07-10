window.BENCHMARK_DATA = {
  "lastUpdate": 1752108391675,
  "repoUrl": "https://github.com/paradedb/paradedb",
  "entries": {
    "pg_search single-server.toml Performance": [
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
        "date": 1752108373966,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 7.544989098607985,
            "unit": "avg cpu",
            "extra": "max cpu: 35.44304, count: 58450"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 102.83543600299402,
            "unit": "avg mem",
            "extra": "max mem: 108.53515625, count: 58450"
          },
          {
            "name": "Custom Scan - Primary - tps",
            "value": 432.86038005186373,
            "unit": "avg tps",
            "extra": "max tps: 612.0542966251891, count: 58450"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.737457370166125,
            "unit": "avg cpu",
            "extra": "max cpu: 9.937888, count: 58450"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 85.85503247968349,
            "unit": "avg mem",
            "extra": "max mem: 92.7890625, count: 58450"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2685.718670159745,
            "unit": "avg tps",
            "extra": "max tps: 2962.4520178778494, count: 58450"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 7.491795135544181,
            "unit": "avg cpu",
            "extra": "max cpu: 35.44304, count: 58450"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 102.83261067151412,
            "unit": "avg mem",
            "extra": "max mem: 109.1640625, count: 58450"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 432.24354994712394,
            "unit": "avg tps",
            "extra": "max tps: 611.7218954503426, count: 58450"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.69910799110644,
            "unit": "avg cpu",
            "extra": "max cpu: 4.968944, count: 58450"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 99.65980151304534,
            "unit": "avg mem",
            "extra": "max mem: 103.68359375, count: 58450"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 403.1011803377769,
            "unit": "avg tps",
            "extra": "max tps: 502.1029326074933, count: 58450"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 7.232880903788376,
            "unit": "avg cpu",
            "extra": "max cpu: 25.0, count: 116900"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 111.83432404164884,
            "unit": "avg mem",
            "extra": "max mem: 118.5703125, count: 116900"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 295.5300470667846,
            "unit": "avg tps",
            "extra": "max tps: 333.84946173576714, count: 116900"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 7652.788331907614,
            "unit": "avg block_count",
            "extra": "max block_count: 7918.0, count: 58450"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 117.13611633875107,
            "unit": "avg segment_count",
            "extra": "max segment_count: 294.0, count: 58450"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 5.639073638550277,
            "unit": "avg cpu",
            "extra": "max cpu: 14.814815, count: 58450"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 114.1335153576775,
            "unit": "avg mem",
            "extra": "max mem: 122.26171875, count: 58450"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 271.7192765059225,
            "unit": "avg tps",
            "extra": "max tps: 299.6863301193838, count: 58450"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 15.52343231830596,
            "unit": "avg cpu",
            "extra": "max cpu: 29.447853, count: 58450"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 82.96726502352438,
            "unit": "avg mem",
            "extra": "max mem: 89.59375, count: 58450"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 25.68619399829861,
            "unit": "avg tps",
            "extra": "max tps: 1918.5609258207603, count: 58450"
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
        "date": 1752108377420,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 7.3428467823705095,
            "unit": "avg cpu",
            "extra": "max cpu: 34.146343, count: 58478"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 105.89344136363248,
            "unit": "avg mem",
            "extra": "max mem: 111.41015625, count: 58478"
          },
          {
            "name": "Custom Scan - Primary - tps",
            "value": 446.8603884997285,
            "unit": "avg tps",
            "extra": "max tps: 630.7002697173392, count: 58478"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.761311465525807,
            "unit": "avg cpu",
            "extra": "max cpu: 9.756097, count: 58478"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 88.51334950483515,
            "unit": "avg mem",
            "extra": "max mem: 94.92578125, count: 58478"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2727.38699867502,
            "unit": "avg tps",
            "extra": "max tps: 3046.948138657469, count: 58478"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 7.270526742471582,
            "unit": "avg cpu",
            "extra": "max cpu: 35.0, count: 58478"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 106.9639764366514,
            "unit": "avg mem",
            "extra": "max mem: 112.6015625, count: 58478"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 446.71138133975114,
            "unit": "avg tps",
            "extra": "max tps: 622.2290290740376, count: 58478"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.794267770430282,
            "unit": "avg cpu",
            "extra": "max cpu: 4.968944, count: 58478"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 102.29601723885479,
            "unit": "avg mem",
            "extra": "max mem: 105.921875, count: 58478"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 396.2264454357822,
            "unit": "avg tps",
            "extra": "max tps: 497.3610767310267, count: 58478"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 7.043609974563129,
            "unit": "avg cpu",
            "extra": "max cpu: 25.157234, count: 116956"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 117.51247060192509,
            "unit": "avg mem",
            "extra": "max mem: 127.0390625, count: 116956"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 302.68882921977246,
            "unit": "avg tps",
            "extra": "max tps: 338.88401502551477, count: 116956"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 8661.441636170868,
            "unit": "avg block_count",
            "extra": "max block_count: 8985.0, count: 58478"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 117.41856766647287,
            "unit": "avg segment_count",
            "extra": "max segment_count: 252.0, count: 58478"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 5.704731440017599,
            "unit": "avg cpu",
            "extra": "max cpu: 14.906833, count: 58478"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 116.14647228444885,
            "unit": "avg mem",
            "extra": "max mem: 124.68359375, count: 58478"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 257.4457808726061,
            "unit": "avg tps",
            "extra": "max tps: 283.5604414361397, count: 58478"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 17.128625843358563,
            "unit": "avg cpu",
            "extra": "max cpu: 29.447853, count: 58478"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 98.64008179358049,
            "unit": "avg mem",
            "extra": "max mem: 103.65625, count: 58478"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 22.56365802262811,
            "unit": "avg tps",
            "extra": "max tps: 1791.5913451805297, count: 58478"
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
        "date": 1752108378654,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 7.0477671566769065,
            "unit": "avg cpu",
            "extra": "max cpu: 29.813665, count: 58450"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 92.2636349844953,
            "unit": "avg mem",
            "extra": "max mem: 106.171875, count: 58450"
          },
          {
            "name": "Custom Scan - Primary - tps",
            "value": 479.6491837147133,
            "unit": "avg tps",
            "extra": "max tps: 662.2202107582043, count: 58450"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.749217555069975,
            "unit": "avg cpu",
            "extra": "max cpu: 9.81595, count: 58450"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 80.87191857356716,
            "unit": "avg mem",
            "extra": "max mem: 93.57421875, count: 58450"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2973.7602015180537,
            "unit": "avg tps",
            "extra": "max tps: 3269.6823796435433, count: 58450"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 7.067524037727375,
            "unit": "avg cpu",
            "extra": "max cpu: 30.000002, count: 58450"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 93.54846657132164,
            "unit": "avg mem",
            "extra": "max mem: 107.49609375, count: 58450"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 479.5352310095232,
            "unit": "avg tps",
            "extra": "max tps: 666.0520619081843, count: 58450"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.690736231356621,
            "unit": "avg cpu",
            "extra": "max cpu: 5.0, count: 58450"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 88.85897735778443,
            "unit": "avg mem",
            "extra": "max mem: 101.9140625, count: 58450"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 443.3550971669251,
            "unit": "avg tps",
            "extra": "max tps: 585.1763706951865, count: 58450"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 7.216104813586705,
            "unit": "avg cpu",
            "extra": "max cpu: 25.157234, count: 116900"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 96.83707816509838,
            "unit": "avg mem",
            "extra": "max mem: 113.875, count: 116900"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 310.5146259137499,
            "unit": "avg tps",
            "extra": "max tps: 313.4439859800386, count: 116900"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 6905.016732249786,
            "unit": "avg block_count",
            "extra": "max block_count: 8378.0, count: 58450"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 116.81213002566297,
            "unit": "avg segment_count",
            "extra": "max segment_count: 398.0, count: 58450"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 5.594070642372758,
            "unit": "avg cpu",
            "extra": "max cpu: 14.906833, count: 58450"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 97.96280962628315,
            "unit": "avg mem",
            "extra": "max mem: 113.65625, count: 58450"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 281.8817514971659,
            "unit": "avg tps",
            "extra": "max tps: 303.64200085466945, count: 58450"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 16.713136134462864,
            "unit": "avg cpu",
            "extra": "max cpu: 29.447853, count: 58450"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 75.25069129597946,
            "unit": "avg mem",
            "extra": "max mem: 91.77734375, count: 58450"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 25.81401332289301,
            "unit": "avg tps",
            "extra": "max tps: 505.2178903717595, count: 58450"
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
        "date": 1752108379898,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 6.867966618418908,
            "unit": "avg cpu",
            "extra": "max cpu: 29.813665, count: 58482"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 88.39754974019783,
            "unit": "avg mem",
            "extra": "max mem: 101.8046875, count: 58482"
          },
          {
            "name": "Custom Scan - Primary - tps",
            "value": 494.8240312467027,
            "unit": "avg tps",
            "extra": "max tps: 681.6914888988114, count: 58482"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.7645907658733835,
            "unit": "avg cpu",
            "extra": "max cpu: 9.937888, count: 58482"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 73.84790378877261,
            "unit": "avg mem",
            "extra": "max mem: 83.42578125, count: 58482"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2966.040566331377,
            "unit": "avg tps",
            "extra": "max tps: 3271.851626307856, count: 58482"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 6.8566792511143895,
            "unit": "avg cpu",
            "extra": "max cpu: 24.84472, count: 58482"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 89.03011757356964,
            "unit": "avg mem",
            "extra": "max mem: 101.7421875, count: 58482"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 494.5673607123992,
            "unit": "avg tps",
            "extra": "max tps: 687.0097414810527, count: 58482"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.70823319313717,
            "unit": "avg cpu",
            "extra": "max cpu: 4.968944, count: 58482"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 84.93762450412093,
            "unit": "avg mem",
            "extra": "max mem: 97.171875, count: 58482"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 458.52697838542326,
            "unit": "avg tps",
            "extra": "max tps: 585.9814371628372, count: 58482"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 6.865481468347383,
            "unit": "avg cpu",
            "extra": "max cpu: 20.125786, count: 116964"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 91.46677289593379,
            "unit": "avg mem",
            "extra": "max mem: 109.25390625, count: 116964"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 326.84408612035367,
            "unit": "avg tps",
            "extra": "max tps: 352.0698636201697, count: 116964"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 6374.67352347731,
            "unit": "avg block_count",
            "extra": "max block_count: 7751.0, count: 58482"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 117.5101398720974,
            "unit": "avg segment_count",
            "extra": "max segment_count: 410.0, count: 58482"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 5.743097954200476,
            "unit": "avg cpu",
            "extra": "max cpu: 15.000001, count: 58482"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 95.31351172957064,
            "unit": "avg mem",
            "extra": "max mem: 107.40625, count: 58482"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 279.8351887140179,
            "unit": "avg tps",
            "extra": "max tps: 288.54053253051694, count: 58482"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.540612301518182,
            "unit": "avg cpu",
            "extra": "max cpu: 29.813665, count: 58482"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 70.52644082794706,
            "unit": "avg mem",
            "extra": "max mem: 76.48828125, count: 58482"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 21.051536508891143,
            "unit": "avg tps",
            "extra": "max tps: 1521.574403466755, count: 58482"
          }
        ]
      }
    ],
    "pg_search wide-table.toml Performance": [
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
        "date": 1752108379500,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 21.451938394237807,
            "unit": "avg cpu",
            "extra": "max cpu: 62.650604, count: 59078"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 174.93739566230238,
            "unit": "avg mem",
            "extra": "max mem: 179.12890625, count: 59078"
          },
          {
            "name": "Bulk Update - Primary - tps",
            "value": 19.788703323615305,
            "unit": "avg tps",
            "extra": "max tps: 31.156245987659695, count: 59078"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 8019.1988726768,
            "unit": "avg block_count",
            "extra": "max block_count: 9434.0, count: 59078"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 42.8015166390196,
            "unit": "avg segment_count",
            "extra": "max segment_count: 96.0, count: 59078"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 13.257987316467045,
            "unit": "avg cpu",
            "extra": "max cpu: 34.355827, count: 59078"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 163.86239153640102,
            "unit": "avg mem",
            "extra": "max mem: 177.02734375, count: 59078"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 112.6134309941091,
            "unit": "avg tps",
            "extra": "max tps: 116.57350174957284, count: 59078"
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
        "date": 1752108387205,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 21.521147951918458,
            "unit": "avg cpu",
            "extra": "max cpu: 59.62733, count: 59093"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 174.76250789274957,
            "unit": "avg mem",
            "extra": "max mem: 178.671875, count: 59093"
          },
          {
            "name": "Bulk Update - Primary - tps",
            "value": 19.974580976751867,
            "unit": "avg tps",
            "extra": "max tps: 31.58866735432163, count: 59093"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 7956.5599817237235,
            "unit": "avg block_count",
            "extra": "max block_count: 9244.0, count: 59093"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 42.77963548982113,
            "unit": "avg segment_count",
            "extra": "max segment_count: 86.0, count: 59093"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 13.432761241527672,
            "unit": "avg cpu",
            "extra": "max cpu: 39.024387, count: 59093"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 164.38808861138375,
            "unit": "avg mem",
            "extra": "max mem: 177.10546875, count: 59093"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 114.75780581178766,
            "unit": "avg tps",
            "extra": "max tps: 117.48392795856309, count: 59093"
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
        "date": 1752108387640,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 19.629033578116807,
            "unit": "avg cpu",
            "extra": "max cpu: 49.079754, count: 59123"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 176.88601688957766,
            "unit": "avg mem",
            "extra": "max mem: 182.73828125, count: 59123"
          },
          {
            "name": "Bulk Update - Primary - tps",
            "value": 38.26211661797697,
            "unit": "avg tps",
            "extra": "max tps: 39.093154026360075, count: 59123"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 17673.381289853358,
            "unit": "avg block_count",
            "extra": "max block_count: 19600.0, count: 59123"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 53.675473166111324,
            "unit": "avg segment_count",
            "extra": "max segment_count: 145.0, count: 59123"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 8.934089462996475,
            "unit": "avg cpu",
            "extra": "max cpu: 34.5679, count: 59123"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 159.20108231726655,
            "unit": "avg mem",
            "extra": "max mem: 175.765625, count: 59123"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 193.2461424668645,
            "unit": "avg tps",
            "extra": "max tps: 217.64761217575733, count: 59123"
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
        "date": 1752108389791,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 19.844605751705508,
            "unit": "avg cpu",
            "extra": "max cpu: 49.382717, count: 59100"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 173.12901775327833,
            "unit": "avg mem",
            "extra": "max mem: 182.5390625, count: 59100"
          },
          {
            "name": "Bulk Update - Primary - tps",
            "value": 38.22904648733755,
            "unit": "avg tps",
            "extra": "max tps: 39.04931818142502, count: 59100"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 22018.948866328257,
            "unit": "avg block_count",
            "extra": "max block_count: 29413.0, count: 59100"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 53.96186125211506,
            "unit": "avg segment_count",
            "extra": "max segment_count: 147.0, count: 59100"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 9.056908910806028,
            "unit": "avg cpu",
            "extra": "max cpu: 34.355827, count: 59100"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 160.2127458756345,
            "unit": "avg mem",
            "extra": "max mem: 175.5078125, count: 59100"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 199.69282709126932,
            "unit": "avg tps",
            "extra": "max tps: 221.23540435269956, count: 59100"
          }
        ]
      }
    ]
  }
}