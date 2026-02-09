window.BENCHMARK_DATA = {
  "lastUpdate": 1770610107417,
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
      }
    ]
  }
}