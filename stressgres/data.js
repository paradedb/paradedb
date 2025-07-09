window.BENCHMARK_DATA = {
  "lastUpdate": 1752037849688,
  "repoUrl": "https://github.com/paradedb/paradedb",
  "entries": {
    "pg_search wide-table.toml Performance": [
      {
        "commit": {
          "author": {
            "name": "Stu Hood",
            "username": "stuhood",
            "email": "stuhood@gmail.com"
          },
          "committer": {
            "name": "Stu Hood",
            "username": "stuhood",
            "email": "stuhood@gmail.com"
          },
          "id": "35abeff494503653aeca1073004f5e0cfd89e115",
          "message": "Backfill for 0.15.26",
          "timestamp": "2025-07-09T04:35:02Z",
          "url": "https://github.com/paradedb/paradedb/commit/35abeff494503653aeca1073004f5e0cfd89e115"
        },
        "date": 1752037848185,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 24.989853051558985,
            "unit": "avg cpu",
            "extra": "max cpu: 73.61964, count: 57009"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 238.88831115207685,
            "unit": "avg mem",
            "extra": "max mem: 265.6328125, count: 57009"
          },
          {
            "name": "Bulk Update - Primary - tps",
            "value": 20.749467659298276,
            "unit": "avg tps",
            "extra": "max tps: 22.04165441628764, count: 57009"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 10514.873160378185,
            "unit": "avg block_count",
            "extra": "max block_count: 11631.0, count: 57009"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 38.182865863284746,
            "unit": "avg segment_count",
            "extra": "max segment_count: 107.0, count: 57009"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 15.39623538093132,
            "unit": "avg cpu",
            "extra": "max cpu: 59.428574, count: 57009"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 223.2905662373485,
            "unit": "avg mem",
            "extra": "max mem: 271.1640625, count: 57009"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 65.27144719743694,
            "unit": "avg tps",
            "extra": "max tps: 83.60156536738894, count: 57009"
          }
        ]
      }
    ]
  }
}