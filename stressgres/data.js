window.BENCHMARK_DATA = {
  "lastUpdate": 1752108378241,
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
      }
    ]
  }
}