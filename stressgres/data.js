window.BENCHMARK_DATA = {
  "lastUpdate": 1752105917232,
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
      },
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
        "date": 1752105868502,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 19.743467254206806,
            "unit": "avg cpu",
            "extra": "max cpu: 54.658382, count: 59105"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 173.33258224240757,
            "unit": "avg mem",
            "extra": "max mem: 182.64453125, count: 59105"
          },
          {
            "name": "Bulk Update - Primary - tps",
            "value": 38.32710195743882,
            "unit": "avg tps",
            "extra": "max tps: 39.056620621099185, count: 59105"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 17999.3765671263,
            "unit": "avg block_count",
            "extra": "max block_count: 19992.0, count: 59105"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 53.57189747060316,
            "unit": "avg segment_count",
            "extra": "max segment_count: 162.0, count: 59105"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 8.900877412833852,
            "unit": "avg cpu",
            "extra": "max cpu: 34.146343, count: 59105"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 159.49692694674732,
            "unit": "avg mem",
            "extra": "max mem: 176.5, count: 59105"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 199.42117450370375,
            "unit": "avg tps",
            "extra": "max tps: 217.38683911004816, count: 59105"
          }
        ]
      },
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
        "date": 1752105904947,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 26.675825635791888,
            "unit": "avg cpu",
            "extra": "max cpu: 60.377357, count: 59128"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 240.90285938187068,
            "unit": "avg mem",
            "extra": "max mem: 264.6328125, count: 59128"
          },
          {
            "name": "Bulk Update - Primary - tps",
            "value": 38.16449538012722,
            "unit": "avg tps",
            "extra": "max tps: 38.90653035314374, count: 59128"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 18145.211963874983,
            "unit": "avg block_count",
            "extra": "max block_count: 20196.0, count: 59128"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 54.60749560276011,
            "unit": "avg segment_count",
            "extra": "max segment_count: 149.0, count: 59128"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 14.5748977430647,
            "unit": "avg cpu",
            "extra": "max cpu: 44.720497, count: 59128"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 219.8370487676101,
            "unit": "avg mem",
            "extra": "max mem: 263.40234375, count: 59128"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 184.1815248374572,
            "unit": "avg tps",
            "extra": "max tps: 198.81901305598487, count: 59128"
          }
        ]
      }
    ],
    "pg_search single-server.toml Performance": [
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
        "date": 1752105864762,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 6.766805918805399,
            "unit": "avg cpu",
            "extra": "max cpu: 24.539877, count: 58439"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 90.4298917728315,
            "unit": "avg mem",
            "extra": "max mem: 95.80078125, count: 58439"
          },
          {
            "name": "Custom Scan - Primary - tps",
            "value": 496.6897017775477,
            "unit": "avg tps",
            "extra": "max tps: 696.2750594261913, count: 58439"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.810678557191289,
            "unit": "avg cpu",
            "extra": "max cpu: 9.876543, count: 58439"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 80.17335751542207,
            "unit": "avg mem",
            "extra": "max mem: 83.94921875, count: 58439"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3049.4191378286787,
            "unit": "avg tps",
            "extra": "max tps: 3444.850413686345, count: 58439"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 6.753622120208729,
            "unit": "avg cpu",
            "extra": "max cpu: 24.691359, count: 58439"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 90.490446836124,
            "unit": "avg mem",
            "extra": "max mem: 95.953125, count: 58439"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 496.8473321466375,
            "unit": "avg tps",
            "extra": "max tps: 685.3906605079011, count: 58439"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.687684407885299,
            "unit": "avg cpu",
            "extra": "max cpu: 5.0, count: 58439"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 86.92706738008864,
            "unit": "avg mem",
            "extra": "max mem: 90.90625, count: 58439"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 451.4702087760443,
            "unit": "avg tps",
            "extra": "max tps: 563.9078452776722, count: 58439"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 6.982132359691001,
            "unit": "avg cpu",
            "extra": "max cpu: 20.0, count: 116878"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 95.01356342190147,
            "unit": "avg mem",
            "extra": "max mem: 103.0859375, count: 116878"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 321.7790409244092,
            "unit": "avg tps",
            "extra": "max tps: 326.31766650702406, count: 116878"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 6583.019815534147,
            "unit": "avg block_count",
            "extra": "max block_count: 6937.0, count: 58439"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 117.13396875374322,
            "unit": "avg segment_count",
            "extra": "max segment_count: 381.0, count: 58439"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 5.853865057882277,
            "unit": "avg cpu",
            "extra": "max cpu: 14.814815, count: 58439"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 97.29943041568987,
            "unit": "avg mem",
            "extra": "max mem: 104.96484375, count: 58439"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 290.1107886997403,
            "unit": "avg tps",
            "extra": "max tps: 302.368153662459, count: 58439"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 14.535234414608828,
            "unit": "avg cpu",
            "extra": "max cpu: 29.447853, count: 58439"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 80.32220396214429,
            "unit": "avg mem",
            "extra": "max mem: 85.73046875, count: 58439"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 20.43400682881216,
            "unit": "avg tps",
            "extra": "max tps: 1414.8934514486386, count: 58439"
          }
        ]
      },
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
        "date": 1752105872542,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 6.885755959301093,
            "unit": "avg cpu",
            "extra": "max cpu: 29.813665, count: 58489"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 89.86421772416608,
            "unit": "avg mem",
            "extra": "max mem: 108.34765625, count: 58489"
          },
          {
            "name": "Custom Scan - Primary - tps",
            "value": 489.2658563209803,
            "unit": "avg tps",
            "extra": "max tps: 672.6666521772723, count: 58489"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.694372083706573,
            "unit": "avg cpu",
            "extra": "max cpu: 9.937888, count: 58489"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 74.48629162641693,
            "unit": "avg mem",
            "extra": "max mem: 86.42578125, count: 58489"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3022.0439818683562,
            "unit": "avg tps",
            "extra": "max tps: 3271.237678986277, count: 58489"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 6.895550625832995,
            "unit": "avg cpu",
            "extra": "max cpu: 34.782608, count: 58489"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 90.47303946842568,
            "unit": "avg mem",
            "extra": "max mem: 108.81640625, count: 58489"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 489.1040362671156,
            "unit": "avg tps",
            "extra": "max tps: 670.4270379802928, count: 58489"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.809780352651678,
            "unit": "avg cpu",
            "extra": "max cpu: 5.0, count: 58489"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 85.74401803918258,
            "unit": "avg mem",
            "extra": "max mem: 101.73828125, count: 58489"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 450.04449268448553,
            "unit": "avg tps",
            "extra": "max tps: 583.3876515852596, count: 58489"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 7.045980787784953,
            "unit": "avg cpu",
            "extra": "max cpu: 25.157234, count: 116978"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 95.06609552784498,
            "unit": "avg mem",
            "extra": "max mem: 116.3046875, count: 116978"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 311.84551867296796,
            "unit": "avg tps",
            "extra": "max tps: 327.4917567256, count: 116978"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 6976.661662876781,
            "unit": "avg block_count",
            "extra": "max block_count: 8764.0, count: 58489"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 116.91680486929167,
            "unit": "avg segment_count",
            "extra": "max segment_count: 251.0, count: 58489"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 5.735392419565638,
            "unit": "avg cpu",
            "extra": "max cpu: 15.6862755, count: 58489"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 93.47078403631453,
            "unit": "avg mem",
            "extra": "max mem: 109.31640625, count: 58489"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 277.6961375254862,
            "unit": "avg tps",
            "extra": "max tps: 281.1107471033003, count: 58489"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.596157099722275,
            "unit": "avg cpu",
            "extra": "max cpu: 29.62963, count: 58489"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 82.98060572821385,
            "unit": "avg mem",
            "extra": "max mem: 98.375, count: 58489"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 22.2217393579684,
            "unit": "avg tps",
            "extra": "max tps: 1680.209152435295, count: 58489"
          }
        ]
      },
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
        "date": 1752105914930,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 8.43129929214287,
            "unit": "avg cpu",
            "extra": "max cpu: 29.268291, count: 58360"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 106.0109326807745,
            "unit": "avg mem",
            "extra": "max mem: 109.24609375, count: 58360"
          },
          {
            "name": "Custom Scan - Primary - tps",
            "value": 334.2975476485161,
            "unit": "avg tps",
            "extra": "max tps: 538.7237803917714, count: 58360"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.913033166483185,
            "unit": "avg cpu",
            "extra": "max cpu: 9.876543, count: 58360"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 90.5340169047721,
            "unit": "avg mem",
            "extra": "max mem: 92.5859375, count: 58360"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2181.8640162335137,
            "unit": "avg tps",
            "extra": "max tps: 2690.3143504528575, count: 58360"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 8.448712169105734,
            "unit": "avg cpu",
            "extra": "max cpu: 29.447853, count: 58360"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 106.68236605230466,
            "unit": "avg mem",
            "extra": "max mem: 109.6875, count: 58360"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 333.86493947809066,
            "unit": "avg tps",
            "extra": "max tps: 528.7836710733657, count: 58360"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.775688804025685,
            "unit": "avg cpu",
            "extra": "max cpu: 5.0314465, count: 58360"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 103.45027228624058,
            "unit": "avg mem",
            "extra": "max mem: 105.40234375, count: 58360"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 284.4122660925113,
            "unit": "avg tps",
            "extra": "max tps: 403.8615627180853, count: 58360"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 14.36636369343981,
            "unit": "avg cpu",
            "extra": "max cpu: 54.320984, count: 116720"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 146.55824355294723,
            "unit": "avg mem",
            "extra": "max mem: 171.1171875, count: 116720"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 154.92466528085237,
            "unit": "avg tps",
            "extra": "max tps: 226.49921419020208, count: 116720"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 7864.868231665524,
            "unit": "avg block_count",
            "extra": "max block_count: 7938.0, count: 58360"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 117.05106237148732,
            "unit": "avg segment_count",
            "extra": "max segment_count: 252.0, count: 58360"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.935849780397286,
            "unit": "avg cpu",
            "extra": "max cpu: 34.5679, count: 58360"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 143.42865899695855,
            "unit": "avg mem",
            "extra": "max mem: 171.015625, count: 58360"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 136.6375318073206,
            "unit": "avg tps",
            "extra": "max tps: 162.12389002323548, count: 58360"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 7.688651599028973,
            "unit": "avg cpu",
            "extra": "max cpu: 19.161678, count: 58360"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 91.19781695778359,
            "unit": "avg mem",
            "extra": "max mem: 101.13671875, count: 58360"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 13.662693646200207,
            "unit": "avg tps",
            "extra": "max tps: 1755.6364708897918, count: 58360"
          }
        ]
      }
    ],
    "pg_search bulk-updates.toml Performance": [
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
        "date": 1752105891833,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 20.23394578900666,
            "unit": "avg cpu",
            "extra": "max cpu: 44.720497, count: 59034"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 228.7284296046558,
            "unit": "avg mem",
            "extra": "max mem: 231.9140625, count: 59034"
          },
          {
            "name": "Bulk Update - Primary - tps",
            "value": 7.499246010648971,
            "unit": "avg tps",
            "extra": "max tps: 11.046118439445621, count: 59034"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 21.5508983225605,
            "unit": "avg cpu",
            "extra": "max cpu: 34.146343, count: 59034"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 162.08923949016668,
            "unit": "avg mem",
            "extra": "max mem: 163.75, count: 59034"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 7.050953977556769,
            "unit": "avg tps",
            "extra": "max tps: 8.51130591270949, count: 59034"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 22468.95611003828,
            "unit": "avg block_count",
            "extra": "max block_count: 25211.0, count: 59034"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 32.52871226750686,
            "unit": "avg segment_count",
            "extra": "max segment_count: 65.0, count: 59034"
          }
        ]
      },
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
        "date": 1752105895250,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 20.340241378830413,
            "unit": "avg cpu",
            "extra": "max cpu: 44.17178, count: 59016"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 228.70616509188187,
            "unit": "avg mem",
            "extra": "max mem: 237.46875, count: 59016"
          },
          {
            "name": "Bulk Update - Primary - tps",
            "value": 7.384152344813479,
            "unit": "avg tps",
            "extra": "max tps: 10.84544546726363, count: 59016"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 21.629960672688743,
            "unit": "avg cpu",
            "extra": "max cpu: 34.782608, count: 59016"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 160.2383201033406,
            "unit": "avg mem",
            "extra": "max mem: 163.28515625, count: 59016"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 7.104237608398653,
            "unit": "avg tps",
            "extra": "max tps: 8.500296742074495, count: 59016"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 21853.173325877728,
            "unit": "avg block_count",
            "extra": "max block_count: 22689.0, count: 59016"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 32.19535380235868,
            "unit": "avg segment_count",
            "extra": "max segment_count: 63.0, count: 59016"
          }
        ]
      },
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
        "date": 1752105915313,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.38682418774135,
            "unit": "avg cpu",
            "extra": "max cpu: 58.895706, count: 59066"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 401.67909514092287,
            "unit": "avg mem",
            "extra": "max mem: 461.828125, count: 59066"
          },
          {
            "name": "Bulk Update - Primary - tps",
            "value": 7.485103252937739,
            "unit": "avg tps",
            "extra": "max tps: 11.375819303503253, count: 59066"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 18.754631014232448,
            "unit": "avg cpu",
            "extra": "max cpu: 34.146343, count: 59066"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 176.81340556802982,
            "unit": "avg mem",
            "extra": "max mem: 212.16796875, count: 59066"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 4.394599725784584,
            "unit": "avg tps",
            "extra": "max tps: 4.713354506699825, count: 59066"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 31430.221718078083,
            "unit": "avg block_count",
            "extra": "max block_count: 35152.0, count: 59066"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 28.334354789557445,
            "unit": "avg segment_count",
            "extra": "max segment_count: 62.0, count: 59066"
          }
        ]
      }
    ]
  }
}