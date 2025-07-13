window.BENCHMARK_DATA = {
  "lastUpdate": 1752440999181,
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
        "date": 1752440998308,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 285.03547980341597,
            "unit": "median tps",
            "extra": "avg tps: 284.5664475907636, max tps: 443.99798977665193, count: 55117"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2280.7349466269866,
            "unit": "median tps",
            "extra": "avg tps: 2266.1489210943914, max tps: 2296.1671773055514, count: 55117"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 237.356420152653,
            "unit": "median tps",
            "extra": "avg tps: 239.7133171667209, max tps: 446.9448088244301, count: 55117"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 233.32858450425005,
            "unit": "median tps",
            "extra": "avg tps: 233.01200561653596, max tps: 356.4562333234179, count: 55117"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 133.1109599699799,
            "unit": "median tps",
            "extra": "avg tps: 134.161937956422, max tps: 145.3352230343854, count: 110234"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 135.0507020088833,
            "unit": "median tps",
            "extra": "avg tps: 138.08492580934154, max tps: 157.536776469744, count: 55117"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 3.94053259686009,
            "unit": "median tps",
            "extra": "avg tps: 8.313916343810918, max tps: 1121.39052425007, count: 55117"
          }
        ]
      }
    ],
    "pg_search single-server.toml Performance - Other Metrics": [
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
        "date": 1752440987843,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 9.160305,
            "unit": "median cpu",
            "extra": "avg cpu: 7.4726942799140685, max cpu: 23.506365, count: 55107"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 103.12890625,
            "unit": "median mem",
            "extra": "avg mem: 100.72339756349012, max mem: 105.32421875, count: 55107"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.619827,
            "unit": "median cpu",
            "extra": "avg cpu: 4.587054184366673, max cpu: 9.221902, count: 55107"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 86.71484375,
            "unit": "median mem",
            "extra": "avg mem: 84.90314311816103, max mem: 86.71484375, count: 55107"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 9.142857,
            "unit": "median cpu",
            "extra": "avg cpu: 7.3917408358814, max cpu: 23.210833, count: 55107"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 102.515625,
            "unit": "median mem",
            "extra": "avg mem: 101.6168992114205, max mem: 106.18359375, count: 55107"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.619827,
            "unit": "median cpu",
            "extra": "avg cpu: 4.5210036859302285, max cpu: 9.230769, count: 55107"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 99.08984375,
            "unit": "median mem",
            "extra": "avg mem: 98.27602266953382, max mem: 101.33984375, count: 55107"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.17782,
            "unit": "median cpu",
            "extra": "avg cpu: 7.6169880760372815, max cpu: 24.048098, count: 110214"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 112.9609375,
            "unit": "median mem",
            "extra": "avg mem: 112.4219112221451, max mem: 119.33203125, count: 110214"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 8535,
            "unit": "median block_count",
            "extra": "avg block_count: 8463.170559094126, max block_count: 8535.0, count: 55107"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 119,
            "unit": "median segment_count",
            "extra": "avg segment_count: 118.37008002613098, max segment_count: 270.0, count: 55107"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 6.235926841931639, max cpu: 19.238478, count: 55107"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 112.75390625,
            "unit": "median mem",
            "extra": "avg mem: 112.79075827764622, max mem: 118.375, count: 55107"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 18.461538,
            "unit": "median cpu",
            "extra": "avg cpu: 17.207486968736323, max cpu: 28.402367, count: 55107"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 97.140625,
            "unit": "median mem",
            "extra": "avg mem: 94.69297365522982, max mem: 99.703125, count: 55107"
          }
        ]
      }
    ]
  }
}