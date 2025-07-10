window.BENCHMARK_DATA = {
  "lastUpdate": 1752105864810,
  "repoUrl": "https://github.com/paradedb/paradedb",
  "entries": {
    "pg_search wide-table.toml Performance": [
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
          "id": "148c36c366eceb9a4ef2b5ec8e86687a04648ccb",
          "message": "ci: disable using fsm_info() in stressgres suite (#2803)\n\nFor now disable using fsm_info() in stressgres `bulkd-updates.toml`\nsuite.\n\nThis is because the benchmark workflows use the latest suite files from\n`main` to run against prior branches during a backfill and this is a\nrelatively new function so old branches don't have it.\n\nI am not sure how to handle this going forward. Perhaps stressgres can\nbe taught how to ignore certain errors -- I am not sure.\n\n---------\n\nSigned-off-by: Eric Ridge <eebbrr@gmail.com>",
          "timestamp": "2025-07-09T23:46:43Z",
          "url": "https://github.com/paradedb/paradedb/commit/148c36c366eceb9a4ef2b5ec8e86687a04648ccb"
        },
        "date": 1752105863940,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 19.53314451763355,
            "unit": "avg cpu",
            "extra": "max cpu: 49.382717, count: 59103"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 181.99721103941425,
            "unit": "avg mem",
            "extra": "max mem: 182.91796875, count: 59103"
          },
          {
            "name": "Bulk Update - Primary - tps",
            "value": 39.21620188297337,
            "unit": "avg tps",
            "extra": "max tps: 40.084065935510075, count: 59103"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 22582.628411417358,
            "unit": "avg block_count",
            "extra": "max block_count: 29346.0, count: 59103"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 54.417982166725885,
            "unit": "avg segment_count",
            "extra": "max segment_count: 169.0, count: 59103"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 8.99366583573179,
            "unit": "avg cpu",
            "extra": "max cpu: 34.5679, count: 59103"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 159.98320602644958,
            "unit": "avg mem",
            "extra": "max mem: 175.0859375, count: 59103"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 207.40515755411022,
            "unit": "avg tps",
            "extra": "max tps: 223.65606795132123, count: 59103"
          }
        ]
      }
    ]
  }
}