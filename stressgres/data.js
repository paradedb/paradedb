window.BENCHMARK_DATA = {
  "lastUpdate": 1778265040083,
  "repoUrl": "https://github.com/paradedb/paradedb",
  "entries": {
    "pg_search single-server.toml Performance - TPS": [
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
          "id": "5ce8f7cabc2743985d08edbeaffb38b3c62f6826",
          "message": "chore: Prepare `0.21.16`. (#4436)\n\n# Description\nBackport of #4434 to `0.21.x`.\n\n---------\n\nCo-authored-by: Stu Hood <stuhood@paradedb.com>\nCo-authored-by: Stu Hood <stuhood@gmail.com>",
          "timestamp": "2026-03-20T02:44:33Z",
          "url": "https://github.com/paradedb/paradedb/commit/5ce8f7cabc2743985d08edbeaffb38b3c62f6826"
        },
        "date": 1778264595193,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 469.5038535362138,
            "unit": "median tps",
            "extra": "avg tps: 469.57655091860175, max tps: 565.4828642755789, count: 55529"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3056.0037107653116,
            "unit": "median tps",
            "extra": "avg tps: 3046.9509948172113, max tps: 3073.463935718692, count: 55529"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 469.41730571191096,
            "unit": "median tps",
            "extra": "avg tps: 468.03376846877063, max tps: 591.5847382346509, count: 55529"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 393.4925510721222,
            "unit": "median tps",
            "extra": "avg tps: 392.79560334475957, max tps: 429.0520562602851, count: 55529"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 3025.744137907696,
            "unit": "median tps",
            "extra": "avg tps: 3032.77067236461, max tps: 3372.381008908482, count: 111058"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 2030.8355824308394,
            "unit": "median tps",
            "extra": "avg tps: 2024.786751386968, max tps: 2039.925185247479, count: 55529"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 94.36971552254222,
            "unit": "median tps",
            "extra": "avg tps: 95.96977092566719, max tps: 853.6232465511487, count: 55529"
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
          "id": "0b5f5714895d3226ae9025f04f5867cf6e63215d",
          "message": "chore: Upgrade to 0.22.6 (#4694)\n\n## Summary\n- Bump version from 0.22.5 to 0.22.6\n- Add changelog entry for 0.22.6\n- Update version references in docs\n\n## Changes since 0.22.5\n- feat: Support expressions in JoinScan DISTINCT target lists (#4682)\n- fix: JoinScan `DISTINCT` planning for deferred keys (#4670)\n- fix: JoinScan pushdown with outer-only `ORDER BY` pathkeys (#4680)\n- fix: `pdb.score()` with `SELECT` subquery in `WHERE` clause (#4653)\n- fix: Handle `IN (SELECT ...) OR IS NULL` via LeftMark JoinScan (#4651)\n- fix: Score filter for joins and cases without other quals (#4650)\n- fix: Handle pruned columns in nested semi/anti join keys (#4668)\n- fix: Handle aliased indexed expressions in search resolution and top-k\norder by (#4671)\n- fix: Lower x86_64 target-cpu from x86-64-v3 to x86-64-v2 (#4673)\n- fix: Prevent DSM buffer overflow in PG18 parallel index scans (#4683)\n- fix: `PlaceHolderVar found where not expected` error (#4689)\n- fix: Restore `pdb.agg(jsonb)` if accidentally removed by an upgrade\nscript (#4688)\n\n## Test plan\n- [ ] CI passes on 0.22.x\n\n---------\n\nCo-authored-by: github-actions[bot] <github-actions[bot]@users.noreply.github.com>",
          "timestamp": "2026-04-08T18:52:29Z",
          "url": "https://github.com/paradedb/paradedb/commit/0b5f5714895d3226ae9025f04f5867cf6e63215d"
        },
        "date": 1778264660427,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - tps",
            "value": 130.64620327425928,
            "unit": "median tps",
            "extra": "avg tps: 130.5537807524007, max tps: 151.66131702434936, count: 55240"
          },
          {
            "name": "Columnar Scan - Primary - tps",
            "value": 455.69339720439126,
            "unit": "median tps",
            "extra": "avg tps: 453.47753139623177, max tps: 560.6790196714235, count: 55240"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2903.686482163258,
            "unit": "median tps",
            "extra": "avg tps: 2897.056677565254, max tps: 3038.044097083484, count: 55240"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 407.6685137589609,
            "unit": "median tps",
            "extra": "avg tps: 406.7680221010021, max tps: 624.3667880158206, count: 55240"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 3013.330659465104,
            "unit": "median tps",
            "extra": "avg tps: 3003.057001642128, max tps: 3035.2340605570903, count: 110480"
          },
          {
            "name": "Normal Scan - Primary - tps",
            "value": 457.5955468873864,
            "unit": "median tps",
            "extra": "avg tps: 455.87021032317995, max tps: 581.3998276440211, count: 55240"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1835.5105830943826,
            "unit": "median tps",
            "extra": "avg tps: 1819.7744701894508, max tps: 1840.1007474585967, count: 55240"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 47.574328903972116,
            "unit": "median tps",
            "extra": "avg tps: 45.60380958140924, max tps: 180.48977705902738, count: 55240"
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
          "id": "c07921a78f3d24cbb0251b31a1150a7db600af5a",
          "message": "chore: Prepare 0.23.4. (#4997)\n\n# Description\nBackport of #4994 to `0.23.x`.\n\n---------\n\nCo-authored-by: Stu Hood <stuhood@paradedb.com>\nCo-authored-by: github-actions[bot] <github-actions[bot]@users.noreply.github.com>",
          "timestamp": "2026-05-06T00:08:47Z",
          "url": "https://github.com/paradedb/paradedb/commit/c07921a78f3d24cbb0251b31a1150a7db600af5a"
        },
        "date": 1778265000675,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - tps",
            "value": 131.5152309296288,
            "unit": "median tps",
            "extra": "avg tps: 131.39283791408823, max tps: 144.12808049313955, count: 55064"
          },
          {
            "name": "Columnar Scan - Primary - tps",
            "value": 480.49816682925507,
            "unit": "median tps",
            "extra": "avg tps: 480.87560393631657, max tps: 602.2349944368542, count: 55064"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3334.579992780429,
            "unit": "median tps",
            "extra": "avg tps: 3316.4504064554453, max tps: 3343.0166272752895, count: 55064"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 425.95851394384323,
            "unit": "median tps",
            "extra": "avg tps: 425.7857098752654, max tps: 588.3344803894586, count: 55064"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 3043.7689057108482,
            "unit": "median tps",
            "extra": "avg tps: 3049.1213012130947, max tps: 3112.202122182038, count: 110128"
          },
          {
            "name": "Normal Scan - Primary - tps",
            "value": 491.906387866289,
            "unit": "median tps",
            "extra": "avg tps: 493.77680618256886, max tps: 624.7935625269996, count: 55064"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 2074.333171113102,
            "unit": "median tps",
            "extra": "avg tps: 2067.97026280735, max tps: 2086.8569902636254, count: 55064"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 44.005912621184926,
            "unit": "median tps",
            "extra": "avg tps: 46.706407894632314, max tps: 282.09206243712873, count: 55064"
          }
        ]
      }
    ],
    "pg_search single-server.toml Performance - Other Metrics": [
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
          "id": "5ce8f7cabc2743985d08edbeaffb38b3c62f6826",
          "message": "chore: Prepare `0.21.16`. (#4436)\n\n# Description\nBackport of #4434 to `0.21.x`.\n\n---------\n\nCo-authored-by: Stu Hood <stuhood@paradedb.com>\nCo-authored-by: Stu Hood <stuhood@gmail.com>",
          "timestamp": "2026-03-20T02:44:33Z",
          "url": "https://github.com/paradedb/paradedb/commit/5ce8f7cabc2743985d08edbeaffb38b3c62f6826"
        },
        "date": 1778264633264,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.7058825,
            "unit": "median cpu",
            "extra": "avg cpu: 6.21519925094482, max cpu: 23.575638, count: 55529"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 57.10546875,
            "unit": "median mem",
            "extra": "avg mem: 56.920296292252694, max mem: 67.39453125, count: 55529"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.51732564154388, max cpu: 9.248554, count: 55529"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 33.87109375,
            "unit": "median mem",
            "extra": "avg mem: 33.52545622276198, max mem: 35.87109375, count: 55529"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.6966734,
            "unit": "median cpu",
            "extra": "avg cpu: 6.138716325749014, max cpu: 19.692308, count: 55529"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 57.375,
            "unit": "median mem",
            "extra": "avg mem: 57.23053459397342, max mem: 67.68359375, count: 55529"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.59872318222867, max cpu: 9.375, count: 55529"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 56.5859375,
            "unit": "median mem",
            "extra": "avg mem: 56.036684730613736, max mem: 66.97265625, count: 55529"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.577734159712496, max cpu: 9.706775, count: 111058"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 45.33984375,
            "unit": "median mem",
            "extra": "avg mem: 45.13387092780349, max mem: 55.63671875, count: 111058"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 1662,
            "unit": "median block_count",
            "extra": "avg block_count: 1667.3625132813486, max block_count: 2961.0, count: 55529"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 8,
            "unit": "median segment_count",
            "extra": "avg segment_count: 9.201354247330224, max segment_count: 17.0, count: 55529"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.5479995600119825, max cpu: 9.257474, count: 55529"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 48.13671875,
            "unit": "median mem",
            "extra": "avg mem: 47.9234766405842, max mem: 58.25390625, count: 55529"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.833837,
            "unit": "median cpu",
            "extra": "avg cpu: 3.7113656248965303, max cpu: 4.833837, count: 55529"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 49.5625,
            "unit": "median mem",
            "extra": "avg mem: 49.8654714062697, max mem: 61.2734375, count: 55529"
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
          "id": "0b5f5714895d3226ae9025f04f5867cf6e63215d",
          "message": "chore: Upgrade to 0.22.6 (#4694)\n\n## Summary\n- Bump version from 0.22.5 to 0.22.6\n- Add changelog entry for 0.22.6\n- Update version references in docs\n\n## Changes since 0.22.5\n- feat: Support expressions in JoinScan DISTINCT target lists (#4682)\n- fix: JoinScan `DISTINCT` planning for deferred keys (#4670)\n- fix: JoinScan pushdown with outer-only `ORDER BY` pathkeys (#4680)\n- fix: `pdb.score()` with `SELECT` subquery in `WHERE` clause (#4653)\n- fix: Handle `IN (SELECT ...) OR IS NULL` via LeftMark JoinScan (#4651)\n- fix: Score filter for joins and cases without other quals (#4650)\n- fix: Handle pruned columns in nested semi/anti join keys (#4668)\n- fix: Handle aliased indexed expressions in search resolution and top-k\norder by (#4671)\n- fix: Lower x86_64 target-cpu from x86-64-v3 to x86-64-v2 (#4673)\n- fix: Prevent DSM buffer overflow in PG18 parallel index scans (#4683)\n- fix: `PlaceHolderVar found where not expected` error (#4689)\n- fix: Restore `pdb.agg(jsonb)` if accidentally removed by an upgrade\nscript (#4688)\n\n## Test plan\n- [ ] CI passes on 0.22.x\n\n---------\n\nCo-authored-by: github-actions[bot] <github-actions[bot]@users.noreply.github.com>",
          "timestamp": "2026-04-08T18:52:29Z",
          "url": "https://github.com/paradedb/paradedb/commit/0b5f5714895d3226ae9025f04f5867cf6e63215d"
        },
        "date": 1778264738010,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - cpu",
            "value": 9.257474,
            "unit": "median cpu",
            "extra": "avg cpu: 8.56183403683842, max cpu: 23.30097, count: 55240"
          },
          {
            "name": "Aggregate Custom Scan - Primary - mem",
            "value": 63.64453125,
            "unit": "median mem",
            "extra": "avg mem: 63.51424501323769, max mem: 75.18359375, count: 55240"
          },
          {
            "name": "Columnar Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 5.755449438390889, max cpu: 18.443804, count: 55240"
          },
          {
            "name": "Columnar Scan - Primary - mem",
            "value": 62.787109375,
            "unit": "median mem",
            "extra": "avg mem: 62.64326758802498, max mem: 74.24609375, count: 55240"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.613288762845364, max cpu: 9.213051, count: 55240"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 35.6796875,
            "unit": "median mem",
            "extra": "avg mem: 35.564563439084, max mem: 37.57421875, count: 55240"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.58092531493284, max cpu: 4.733728, count: 55240"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 61.8984375,
            "unit": "median mem",
            "extra": "avg mem: 61.516383904326574, max mem: 73.33203125, count: 55240"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.640108300893583, max cpu: 9.402546, count: 110480"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 49.65625,
            "unit": "median mem",
            "extra": "avg mem: 49.387381447716784, max mem: 60.8046875, count: 110480"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 1813,
            "unit": "median block_count",
            "extra": "avg block_count: 1805.3439717595945, max block_count: 3214.0, count: 55240"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 17,
            "unit": "median segment_count",
            "extra": "avg segment_count: 17.660264301230992, max segment_count: 30.0, count: 55240"
          },
          {
            "name": "Normal Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 5.804137992870516, max cpu: 18.532818, count: 55240"
          },
          {
            "name": "Normal Scan - Primary - mem",
            "value": 62.76171875,
            "unit": "median mem",
            "extra": "avg mem: 62.615605907177766, max mem: 74.17578125, count: 55240"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.570229992333264, max cpu: 9.347614, count: 55240"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 52.390625,
            "unit": "median mem",
            "extra": "avg mem: 52.154499188201484, max mem: 63.24609375, count: 55240"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 3.3717161874335453, max cpu: 4.660194, count: 55240"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 54.53125,
            "unit": "median mem",
            "extra": "avg mem: 54.39136594293085, max mem: 67.37890625, count: 55240"
          }
        ]
      }
    ]
  }
}