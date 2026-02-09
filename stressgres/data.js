window.BENCHMARK_DATA = {
  "lastUpdate": 1770611040507,
  "repoUrl": "https://github.com/paradedb/paradedb",
  "entries": {
    "pg_search single-server.toml Performance - TPS": [
      {
        "commit": {
          "author": {
            "name": "Stu Hood",
            "username": "stuhood",
            "email": "stuhood@paradedb.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "05025da0ce66b0606497b092203538a732534d16",
          "message": "chore: Fix benchmark data destination. (#4135)\n\n## What\n\nSend all query benchmark data to the same directory.\n\n## Why\n\nIn #4080, we accidentally introduced subdirectories of our `benchmarks`\ndataset, which resulted in separate datasets and pages to render them,\nrather than a single dataset and page.\n\n<img width=\"145\" height=\"413\" alt=\"Screenshot 2026-02-08 at 5 04 35 PM\"\nsrc=\"https://github.com/user-attachments/assets/5afbcaf0-d823-4507-b0ab-36494b839661\"\n/>\n\nEach subdirectory has its own `data.js` and `index.html`, but we want it\nto be merged into the parent directory's data.",
          "timestamp": "2026-02-09T01:08:28Z",
          "url": "https://github.com/paradedb/paradedb/commit/05025da0ce66b0606497b092203538a732534d16"
        },
        "date": 1770610103055,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - tps",
            "value": 128.42890343575283,
            "unit": "median tps",
            "extra": "avg tps: 128.67567885195652, max tps: 145.51349215130887, count: 29932"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2944.7990361604075,
            "unit": "median tps",
            "extra": "avg tps: 2910.3640041353437, max tps: 2963.441071251417, count: 29932"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 482.40471811577345,
            "unit": "median tps",
            "extra": "avg tps: 480.7129173347383, max tps: 577.8264069144359, count: 29932"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 2988.746211238815,
            "unit": "median tps",
            "extra": "avg tps: 2967.6152251448634, max tps: 3023.9039985550585, count: 59864"
          },
          {
            "name": "Mixed Fast Field Scan - Primary - tps",
            "value": 505.5183063997111,
            "unit": "median tps",
            "extra": "avg tps: 506.94530566253354, max tps: 575.0349482203181, count: 29932"
          },
          {
            "name": "Normal Scan - Primary - tps",
            "value": 506.0880704253664,
            "unit": "median tps",
            "extra": "avg tps: 508.7508724190859, max tps: 606.6180980372613, count: 29932"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1892.5344427645787,
            "unit": "median tps",
            "extra": "avg tps: 1869.1192890268378, max tps: 1915.2173819296436, count: 29932"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 56.884004666833754,
            "unit": "median tps",
            "extra": "avg tps: 75.57458274653905, max tps: 676.1338934426507, count: 29932"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "paradedb[bot]",
            "username": "paradedb-bot",
            "email": "developers@paradedb.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "b91006e07d5c6fc93a8bb38ca19094ef5f3081f7",
          "message": "fix: Early return in fieldname extraction causes pushdown to not happen (#4075)\n\n# Description\nBackport of #4071 to `0.21.x`.\n\n---------\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2026-02-03T20:28:51Z",
          "url": "https://github.com/paradedb/paradedb/commit/b91006e07d5c6fc93a8bb38ca19094ef5f3081f7"
        },
        "date": 1770610373414,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 576.9271273931454,
            "unit": "median tps",
            "extra": "avg tps: 576.9830043436718, max tps: 635.5297465679338, count: 55286"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3204.9024242604264,
            "unit": "median tps",
            "extra": "avg tps: 3194.961836768077, max tps: 3219.601327688294, count: 55286"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 550.1662553448517,
            "unit": "median tps",
            "extra": "avg tps: 550.3130535478572, max tps: 663.0059450013507, count: 55286"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 471.6853506083667,
            "unit": "median tps",
            "extra": "avg tps: 473.5374473483095, max tps: 524.6292800272076, count: 55286"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 3278.503369482497,
            "unit": "median tps",
            "extra": "avg tps: 3260.6341072329715, max tps: 3300.321925523087, count: 110572"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 2177.4200905084954,
            "unit": "median tps",
            "extra": "avg tps: 2164.3130756931096, max tps: 2187.8838007956565, count: 55286"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 135.1278900207408,
            "unit": "median tps",
            "extra": "avg tps: 131.43541511759994, max tps: 211.8584819238106, count: 55286"
          }
        ]
      }
    ],
    "pg_search single-server.toml Performance - Other Metrics": [
      {
        "commit": {
          "author": {
            "name": "Stu Hood",
            "username": "stuhood",
            "email": "stuhood@paradedb.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "05025da0ce66b0606497b092203538a732534d16",
          "message": "chore: Fix benchmark data destination. (#4135)\n\n## What\n\nSend all query benchmark data to the same directory.\n\n## Why\n\nIn #4080, we accidentally introduced subdirectories of our `benchmarks`\ndataset, which resulted in separate datasets and pages to render them,\nrather than a single dataset and page.\n\n<img width=\"145\" height=\"413\" alt=\"Screenshot 2026-02-08 at 5 04 35 PM\"\nsrc=\"https://github.com/user-attachments/assets/5afbcaf0-d823-4507-b0ab-36494b839661\"\n/>\n\nEach subdirectory has its own `data.js` and `index.html`, but we want it\nto be merged into the parent directory's data.",
          "timestamp": "2026-02-09T01:08:28Z",
          "url": "https://github.com/paradedb/paradedb/commit/05025da0ce66b0606497b092203538a732534d16"
        },
        "date": 1770610108861,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - cpu",
            "value": 9.239654,
            "unit": "median cpu",
            "extra": "avg cpu: 7.936195449402945, max cpu: 22.89348, count: 29932"
          },
          {
            "name": "Aggregate Custom Scan - Primary - mem",
            "value": 58.06640625,
            "unit": "median mem",
            "extra": "avg mem: 57.77868115645463, max mem: 63.98828125, count: 29932"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.60582090797109, max cpu: 9.384164, count: 29932"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 34.66015625,
            "unit": "median mem",
            "extra": "avg mem: 34.48554692720166, max mem: 35.46875, count: 29932"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.625412881184036, max cpu: 4.738401, count: 29932"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 55.98828125,
            "unit": "median mem",
            "extra": "avg mem: 55.379200275833554, max mem: 62.37890625, count: 29932"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.653208168648449, max cpu: 9.467456, count: 59864"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 44.48046875,
            "unit": "median mem",
            "extra": "avg mem: 44.164418319545135, max mem: 50.3515625, count: 59864"
          },
          {
            "name": "Mixed Fast Field Scan - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 5.4355368070757395, max cpu: 15.130024, count: 29932"
          },
          {
            "name": "Mixed Fast Field Scan - Primary - mem",
            "value": 57.2734375,
            "unit": "median mem",
            "extra": "avg mem: 56.9986648121158, max mem: 63.36328125, count: 29932"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 1142,
            "unit": "median block_count",
            "extra": "avg block_count: 1144.8037885874649, max block_count: 1874.0, count: 29932"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 12,
            "unit": "median segment_count",
            "extra": "avg segment_count: 11.309835627422157, max segment_count: 17.0, count: 29932"
          },
          {
            "name": "Normal Scan - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 5.367413275353657, max cpu: 18.934912, count: 29932"
          },
          {
            "name": "Normal Scan - Primary - mem",
            "value": 57.11328125,
            "unit": "median mem",
            "extra": "avg mem: 56.86202593065114, max mem: 63.125, count: 29932"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.48981890434958, max cpu: 4.7244096, count: 29932"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 47.04296875,
            "unit": "median mem",
            "extra": "avg mem: 46.85442275929607, max mem: 52.91015625, count: 29932"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 0,
            "unit": "median cpu",
            "extra": "avg cpu: 0.0, max cpu: 0.0, count: 29932"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 48.6640625,
            "unit": "median mem",
            "extra": "avg mem: 47.811727676483365, max mem: 55.58984375, count: 29932"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "paradedb[bot]",
            "username": "paradedb-bot",
            "email": "developers@paradedb.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "b91006e07d5c6fc93a8bb38ca19094ef5f3081f7",
          "message": "fix: Early return in fieldname extraction causes pushdown to not happen (#4075)\n\n# Description\nBackport of #4071 to `0.21.x`.\n\n---------\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2026-02-03T20:28:51Z",
          "url": "https://github.com/paradedb/paradedb/commit/b91006e07d5c6fc93a8bb38ca19094ef5f3081f7"
        },
        "date": 1770610383449,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 5.440482351599542, max cpu: 18.953604, count: 55286"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 57.6015625,
            "unit": "median mem",
            "extra": "avg mem: 57.35723760592646, max mem: 68.1796875, count: 55286"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.595994830631172, max cpu: 9.486166, count: 55286"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 33.328125,
            "unit": "median mem",
            "extra": "avg mem: 33.048295666511414, max mem: 35.33984375, count: 55286"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 5.480735821157791, max cpu: 15.2623205, count: 55286"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 57.58984375,
            "unit": "median mem",
            "extra": "avg mem: 57.32359578204699, max mem: 68.1015625, count: 55286"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.627676854264966, max cpu: 9.248554, count: 55286"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 56.91015625,
            "unit": "median mem",
            "extra": "avg mem: 56.23966199625583, max mem: 67.46484375, count: 55286"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.585491465516918, max cpu: 9.657948, count: 110572"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 46.3828125,
            "unit": "median mem",
            "extra": "avg mem: 46.10013755884175, max mem: 56.640625, count: 110572"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 1743,
            "unit": "median block_count",
            "extra": "avg block_count: 1738.4084216619035, max block_count: 3053.0, count: 55286"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 11,
            "unit": "median segment_count",
            "extra": "avg segment_count: 11.301577252830734, max segment_count: 25.0, count: 55286"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.491054652340373, max cpu: 7.5235105, count: 55286"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 48.33984375,
            "unit": "median mem",
            "extra": "avg mem: 47.19514193805846, max mem: 58.4609375, count: 55286"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 3.337313731064648, max cpu: 4.7197638, count: 55286"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 51.34375,
            "unit": "median mem",
            "extra": "avg mem: 50.72060774031853, max mem: 63.296875, count: 55286"
          }
        ]
      }
    ],
    "pg_search bulk-updates.toml Performance - TPS": [
      {
        "commit": {
          "author": {
            "name": "Stu Hood",
            "username": "stuhood",
            "email": "stuhood@paradedb.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "05025da0ce66b0606497b092203538a732534d16",
          "message": "chore: Fix benchmark data destination. (#4135)\n\n## What\n\nSend all query benchmark data to the same directory.\n\n## Why\n\nIn #4080, we accidentally introduced subdirectories of our `benchmarks`\ndataset, which resulted in separate datasets and pages to render them,\nrather than a single dataset and page.\n\n<img width=\"145\" height=\"413\" alt=\"Screenshot 2026-02-08 at 5 04 35 PM\"\nsrc=\"https://github.com/user-attachments/assets/5afbcaf0-d823-4507-b0ab-36494b839661\"\n/>\n\nEach subdirectory has its own `data.js` and `index.html`, but we want it\nto be merged into the parent directory's data.",
          "timestamp": "2026-02-09T01:08:28Z",
          "url": "https://github.com/paradedb/paradedb/commit/05025da0ce66b0606497b092203538a732534d16"
        },
        "date": 1770611026125,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 7.734170460752499,
            "unit": "median tps",
            "extra": "avg tps: 6.639922827873307, max tps: 10.014262572041444, count: 57787"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.323796506604033,
            "unit": "median tps",
            "extra": "avg tps: 4.772389363788728, max tps: 5.9722371440075, count: 57787"
          }
        ]
      }
    ],
    "pg_search bulk-updates.toml Performance - Other Metrics": [
      {
        "commit": {
          "author": {
            "name": "Stu Hood",
            "username": "stuhood",
            "email": "stuhood@paradedb.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "05025da0ce66b0606497b092203538a732534d16",
          "message": "chore: Fix benchmark data destination. (#4135)\n\n## What\n\nSend all query benchmark data to the same directory.\n\n## Why\n\nIn #4080, we accidentally introduced subdirectories of our `benchmarks`\ndataset, which resulted in separate datasets and pages to render them,\nrather than a single dataset and page.\n\n<img width=\"145\" height=\"413\" alt=\"Screenshot 2026-02-08 at 5 04 35 PM\"\nsrc=\"https://github.com/user-attachments/assets/5afbcaf0-d823-4507-b0ab-36494b839661\"\n/>\n\nEach subdirectory has its own `data.js` and `index.html`, but we want it\nto be merged into the parent directory's data.",
          "timestamp": "2026-02-09T01:08:28Z",
          "url": "https://github.com/paradedb/paradedb/commit/05025da0ce66b0606497b092203538a732534d16"
        },
        "date": 1770611036166,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.210833,
            "unit": "median cpu",
            "extra": "avg cpu: 21.04075997203024, max cpu: 42.772278, count: 57787"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 236.03515625,
            "unit": "median mem",
            "extra": "avg mem: 235.90127861802827, max mem: 237.5078125, count: 57787"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.323614,
            "unit": "median cpu",
            "extra": "avg cpu: 22.470302276083196, max cpu: 33.366436, count: 57787"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 175.40234375,
            "unit": "median mem",
            "extra": "avg mem: 175.15582075661482, max mem: 175.8515625, count: 57787"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 34256,
            "unit": "median block_count",
            "extra": "avg block_count: 33530.154359977154, max block_count: 36247.0, count: 57787"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 79,
            "unit": "median segment_count",
            "extra": "avg segment_count: 81.58656791319846, max segment_count: 130.0, count: 57787"
          }
        ]
      }
    ]
  }
}