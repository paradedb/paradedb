window.BENCHMARK_DATA = {
  "lastUpdate": 1752037887891,
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
    ],
    "pg_search single-server.toml Performance": [
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
        "date": 1752037854858,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 7.664339360247371,
            "unit": "avg cpu",
            "extra": "max cpu: 23.809525, count: 56294"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 113.93638219381728,
            "unit": "avg mem",
            "extra": "max mem: 116.83203125, count: 56294"
          },
          {
            "name": "Custom Scan - Primary - tps",
            "value": 250.16141484850002,
            "unit": "avg tps",
            "extra": "max tps: 460.40873779211086, count: 56294"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.85019531675967,
            "unit": "avg cpu",
            "extra": "max cpu: 9.580839, count: 56294"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 95.32131692265162,
            "unit": "avg mem",
            "extra": "max mem: 97.54296875, count: 56294"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 306.55325947413377,
            "unit": "avg tps",
            "extra": "max tps: 434.83504784932944, count: 56294"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 7.7099785378598,
            "unit": "avg cpu",
            "extra": "max cpu: 19.512194, count: 56294"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 114.28242144478097,
            "unit": "avg mem",
            "extra": "max mem: 117.0078125, count: 56294"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 249.30898622358418,
            "unit": "avg tps",
            "extra": "max tps: 572.8652649752214, count: 56294"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.801507635631248,
            "unit": "avg cpu",
            "extra": "max cpu: 9.756097, count: 56294"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 110.31271698304882,
            "unit": "avg mem",
            "extra": "max mem: 112.0, count: 56294"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 235.9100078160583,
            "unit": "avg tps",
            "extra": "max tps: 480.87223578791946, count: 56294"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 16.551882661519773,
            "unit": "avg cpu",
            "extra": "max cpu: 48.484848, count: 112588"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 149.90494289744467,
            "unit": "avg mem",
            "extra": "max mem: 178.265625, count: 112588"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 68.60552265282757,
            "unit": "avg tps",
            "extra": "max tps: 77.28253636671721, count: 112588"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 9107.04602621949,
            "unit": "avg block_count",
            "extra": "max block_count: 9149.0, count: 56294"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 115.05977546452553,
            "unit": "avg segment_count",
            "extra": "max segment_count: 246.0, count: 56294"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 11.994156394399573,
            "unit": "avg cpu",
            "extra": "max cpu: 33.939396, count: 56294"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 163.41880080747947,
            "unit": "avg mem",
            "extra": "max mem: 186.20703125, count: 56294"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 66.39709580772534,
            "unit": "avg tps",
            "extra": "max tps: 76.47667554747053, count: 56294"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 12.140320713295937,
            "unit": "avg cpu",
            "extra": "max cpu: 29.090908, count: 56294"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 96.05265860093883,
            "unit": "avg mem",
            "extra": "max mem: 105.875, count: 56294"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 4.416749646988359,
            "unit": "avg tps",
            "extra": "max tps: 213.96566749692477, count: 56294"
          }
        ]
      }
    ],
    "pg_search bulk-updates.toml Performance": [
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
        "date": 1752037886437,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 27.51390401387334,
            "unit": "avg cpu",
            "extra": "max cpu: 60.818714, count: 56964"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 414.7300640947572,
            "unit": "avg mem",
            "extra": "max mem: 498.3125, count: 56964"
          },
          {
            "name": "Bulk Update - Primary - tps",
            "value": 5.682229676248961,
            "unit": "avg tps",
            "extra": "max tps: 8.133678872341175, count: 56964"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 18.339099408591416,
            "unit": "avg cpu",
            "extra": "max cpu: 33.73494, count: 56964"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 175.16254036266326,
            "unit": "avg mem",
            "extra": "max mem: 215.0625, count: 56964"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 3.9799371934672725,
            "unit": "avg tps",
            "extra": "max tps: 4.544042033135933, count: 56964"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 32330.87787023383,
            "unit": "avg block_count",
            "extra": "max block_count: 36743.0, count: 56964"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 22.407924303068604,
            "unit": "avg segment_count",
            "extra": "max segment_count: 48.0, count: 56964"
          }
        ]
      }
    ]
  }
}