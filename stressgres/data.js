window.BENCHMARK_DATA = {
  "lastUpdate": 1752440986748,
  "repoUrl": "https://github.com/paradedb/paradedb",
  "entries": {
    "pg_search single-server.toml Performance - TPS": [
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
        "date": 1752440985886,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 306.3111752901644,
            "unit": "median tps",
            "extra": "avg tps: 307.6931278290426, max tps: 520.3365980533484, count: 55107"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2536.7213297437415,
            "unit": "median tps",
            "extra": "avg tps: 2524.1791177870427, max tps: 2577.5381654331127, count: 55107"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 305.2038031331571,
            "unit": "median tps",
            "extra": "avg tps: 306.3767194694763, max tps: 484.06616612217107, count: 55107"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 267.7627374966694,
            "unit": "median tps",
            "extra": "avg tps: 267.2598482688497, max tps: 430.7737903441194, count: 55107"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 155.08945061007833,
            "unit": "median tps",
            "extra": "avg tps: 154.11631341151, max tps: 163.90287680657562, count: 110214"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 135.65325779836144,
            "unit": "median tps",
            "extra": "avg tps: 134.99656233652175, max tps: 147.78242179006236, count: 55107"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 5.390145064605325,
            "unit": "median tps",
            "extra": "avg tps: 8.90633164011802, max tps: 940.1747972983138, count: 55107"
          }
        ]
      }
    ]
  }
}