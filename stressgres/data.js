window.BENCHMARK_DATA = {
  "lastUpdate": 1752108374795,
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
      }
    ]
  }
}