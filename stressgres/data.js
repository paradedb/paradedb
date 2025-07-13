window.BENCHMARK_DATA = {
  "lastUpdate": 1752441066240,
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
        "date": 1752440998850,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 271.9593434748166,
            "unit": "median tps",
            "extra": "avg tps: 272.28677116029274, max tps: 475.5747474993365, count: 55130"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2019.6208111655794,
            "unit": "median tps",
            "extra": "avg tps: 2018.8954776980363, max tps: 2408.5971419167427, count: 55130"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 269.81723373540353,
            "unit": "median tps",
            "extra": "avg tps: 270.34188692359044, max tps: 443.5282070858116, count: 55130"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 224.698678800447,
            "unit": "median tps",
            "extra": "avg tps: 225.53739780980712, max tps: 362.169458800037, count: 55130"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 135.49123934417017,
            "unit": "median tps",
            "extra": "avg tps: 139.12872467955398, max tps: 153.29109269682283, count: 110260"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 130.71653115941604,
            "unit": "median tps",
            "extra": "avg tps: 133.08391895358787, max tps: 148.70270170304914, count: 55130"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 4.3696814096749925,
            "unit": "median tps",
            "extra": "avg tps: 8.5874659065905, max tps: 1163.9817487661794, count: 55130"
          }
        ]
      },
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
          "id": "4fd1b2b6b6664d03946be0f4836732f0f40df0cc",
          "message": "chore: Rename datasets and add string paging queries (#2834)\n\n## What\n\nAdd a high-cardinality paging/top-n query to the benchmarks, and rename\ndatasets to match their content. Additionally, improve the generation\nscript for the `docs` dataset to avoid joins and allow for deterministic\nrelative-position queries.\n\n## Why\n\nWe don't currently have a high-cardinality string paging/top-n query in\nthe benchmark. We have top-n on a string column, but only for\nlow-cardinality values (`top_n-string.sql`). The top-n case represented\nan important gap that a user encountered, which #2828 addresses.\n\nThe names of the `benchmark` datasets don't currently describe their\nshape / schema, and for the `join` dataset in particular, that would\ndiscourage using it for other types of queries. We rename it to `docs`\nhere, and then use the `pages` table as the dataset for top-n.\n\n## Tests\n\nTested locally that the new query demonstrates a speedup for #2828.",
          "timestamp": "2025-07-13T18:04:27Z",
          "url": "https://github.com/paradedb/paradedb/commit/4fd1b2b6b6664d03946be0f4836732f0f40df0cc"
        },
        "date": 1752441065373,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 287.4410162524614,
            "unit": "median tps",
            "extra": "avg tps: 290.79458171237866, max tps: 532.1087582874854, count: 54617"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2309.093793540984,
            "unit": "median tps",
            "extra": "avg tps: 2307.4051934213358, max tps: 2514.506580333097, count: 54617"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 287.910233299443,
            "unit": "median tps",
            "extra": "avg tps: 291.8019135736587, max tps: 537.4094673147708, count: 54617"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 257.8723536897191,
            "unit": "median tps",
            "extra": "avg tps: 259.8010282477663, max tps: 447.463310357733, count: 54617"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 139.78571655359298,
            "unit": "median tps",
            "extra": "avg tps: 139.88588374700302, max tps: 159.86662399337084, count: 109234"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 152.4945316017401,
            "unit": "median tps",
            "extra": "avg tps: 150.75898831967797, max tps: 153.2469948209186, count: 54617"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 4.815295851075635,
            "unit": "median tps",
            "extra": "avg tps: 9.03018688698432, max tps: 1067.3486312854827, count: 54617"
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
        "date": 1752441000636,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 9.142857,
            "unit": "median cpu",
            "extra": "avg cpu: 7.372498752266969, max cpu: 23.143684, count: 55117"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 99.9921875,
            "unit": "median mem",
            "extra": "avg mem: 106.5030090109449, max mem: 139.4140625, count: 55117"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.619827,
            "unit": "median cpu",
            "extra": "avg cpu: 4.698644794060423, max cpu: 9.402546, count: 55117"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 86.29296875,
            "unit": "median mem",
            "extra": "avg mem: 87.64859908807627, max mem: 99.79296875, count: 55117"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 9.213051,
            "unit": "median cpu",
            "extra": "avg cpu: 8.294642715711806, max cpu: 24.0, count: 55117"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 104.26953125,
            "unit": "median mem",
            "extra": "avg mem: 110.44533921873015, max mem: 143.88671875, count: 55117"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.610951,
            "unit": "median cpu",
            "extra": "avg cpu: 4.561810237250614, max cpu: 9.257474, count: 55117"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 99.41796875,
            "unit": "median mem",
            "extra": "avg mem: 105.16435548469619, max mem: 132.04296875, count: 55117"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.221902,
            "unit": "median cpu",
            "extra": "avg cpu: 8.626394826952847, max cpu: 28.8, count: 110234"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 112.3984375,
            "unit": "median mem",
            "extra": "avg mem: 117.72671944596269, max mem: 162.28515625, count: 110234"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 8125,
            "unit": "median block_count",
            "extra": "avg block_count: 9345.314549050203, max block_count: 14412.0, count: 55117"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 120,
            "unit": "median segment_count",
            "extra": "avg segment_count: 119.52584502059256, max segment_count: 429.0, count: 55117"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 5.995498716428834, max cpu: 23.099133, count: 55117"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 117.640625,
            "unit": "median mem",
            "extra": "avg mem: 122.2538302043834, max mem: 152.56640625, count: 55117"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 18.426102,
            "unit": "median cpu",
            "extra": "avg cpu: 15.938639467034454, max cpu: 28.180038, count: 55117"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 94.78515625,
            "unit": "median mem",
            "extra": "avg mem: 93.84444029292233, max mem: 98.8125, count: 55117"
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
        "date": 1752441002300,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 9.186603,
            "unit": "median cpu",
            "extra": "avg cpu: 7.806153008008782, max cpu: 36.994217, count: 55130"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 103.1875,
            "unit": "median mem",
            "extra": "avg mem: 102.25979814756032, max mem: 106.32421875, count: 55130"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6153846,
            "unit": "median cpu",
            "extra": "avg cpu: 4.608452600735903, max cpu: 9.275363, count: 55130"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 87.04296875,
            "unit": "median mem",
            "extra": "avg mem: 85.86739042943951, max mem: 87.41796875, count: 55130"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 9.186603,
            "unit": "median cpu",
            "extra": "avg cpu: 7.76685727779277, max cpu: 32.36994, count: 55130"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 103.84765625,
            "unit": "median mem",
            "extra": "avg mem: 102.48738776528207, max mem: 106.01171875, count: 55130"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6153846,
            "unit": "median cpu",
            "extra": "avg cpu: 4.628196297955122, max cpu: 9.239654, count: 55130"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 101.66796875,
            "unit": "median mem",
            "extra": "avg mem: 101.02651956171776, max mem: 104.8125, count: 55130"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.204219,
            "unit": "median cpu",
            "extra": "avg cpu: 8.278521581700955, max cpu: 28.374382, count: 110260"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 114.189453125,
            "unit": "median mem",
            "extra": "avg mem: 113.88865171668103, max mem: 124.03515625, count: 110260"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 8535,
            "unit": "median block_count",
            "extra": "avg block_count: 8517.909341556322, max block_count: 8535.0, count: 55130"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 120,
            "unit": "median segment_count",
            "extra": "avg segment_count: 119.3121349537457, max segment_count: 355.0, count: 55130"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 6.113521859631946, max cpu: 18.497108, count: 55130"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 112.36328125,
            "unit": "median mem",
            "extra": "avg mem: 113.4691090944132, max mem: 122.44140625, count: 55130"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 18.497108,
            "unit": "median cpu",
            "extra": "avg cpu: 17.34055356965306, max cpu: 32.40116, count: 55130"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 95.3984375,
            "unit": "median mem",
            "extra": "avg mem: 94.00955872766643, max mem: 96.5625, count: 55130"
          }
        ]
      }
    ]
  }
}