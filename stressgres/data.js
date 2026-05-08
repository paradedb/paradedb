window.BENCHMARK_DATA = {
  "lastUpdate": 1778278243280,
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
      },
      {
        "commit": {
          "author": {
            "name": "JLockerman",
            "username": "JLockerman",
            "email": "lockerman@paradedb.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "9a827ea3a0452e482e852785e743deb2f4630222",
          "message": "fix: Issues with text casts and memory layout for tokenizer types (#4900)\n\n# Ticket(s) Closed\n\n- fixes https://github.com/paradedb/paradedb/issues/5033\n\n## What\n\nChanges the tokenizer and alias types to function as regular SQL types\n(writable to disk, reallocatable in memory contexts etc).\n\n## Why\n\nWhen used incorrectly (eg. within a non-optimized function call) the\nprevious versions would access freed memory.\n\n## How\n\nThe tokenizer format is changed from `(header, magic_num, Oid, padding\nDatum)` to `(header, magic_num, metadata, padding, Oid, data_bytes)`\nwhere the `data_bytes` are the bytes from the original value (the\n`Datum` for by-value types, and the bytes pointed-at by the `Datum` for\nby-ref types). This lets us create a new `Datum` for that type (pointing\nat the inner bytes if needed).\n\nNOTE: Since the old version of the type was storing `Datum`s directly,\nany values stored to disk with the old code is broken unless they were\nin text format (the others store dangling pointers). In the updated\nversion such values will be output as meaningless text instead.\n\n## Tests\n\n- in\n`pg_search/tests/pg_regress/sql/tokenizer-types-inline-tokenization.sql`\n\n---------\n\nCo-authored-by: Mithun Chicklore Yogendra <mithun.cy@gmail.com>",
          "timestamp": "2026-05-08T18:32:32Z",
          "url": "https://github.com/paradedb/paradedb/commit/9a827ea3a0452e482e852785e743deb2f4630222"
        },
        "date": 1778265158970,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - tps",
            "value": 136.0293526728978,
            "unit": "median tps",
            "extra": "avg tps: 135.99389372208208, max tps: 148.79856320107373, count: 55176"
          },
          {
            "name": "Columnar Scan - Primary - tps",
            "value": 526.8343432771351,
            "unit": "median tps",
            "extra": "avg tps: 527.6718357790961, max tps: 644.5323371708233, count: 55176"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3327.9272673863798,
            "unit": "median tps",
            "extra": "avg tps: 3313.518666777155, max tps: 3365.96427926508, count: 55176"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 438.24118128065373,
            "unit": "median tps",
            "extra": "avg tps: 444.2537095393203, max tps: 511.49403828123684, count: 55176"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 3052.521187885589,
            "unit": "median tps",
            "extra": "avg tps: 3031.3386182147588, max tps: 3082.5632875987058, count: 110352"
          },
          {
            "name": "Normal Scan - Primary - tps",
            "value": 520.4251913238885,
            "unit": "median tps",
            "extra": "avg tps: 522.2922015720238, max tps: 628.5028559169773, count: 55176"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 2134.2074127467054,
            "unit": "median tps",
            "extra": "avg tps: 2122.2540331822524, max tps: 2140.773692959165, count: 55176"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 132.73487386892947,
            "unit": "median tps",
            "extra": "avg tps: 173.85248264530202, max tps: 389.5233936435495, count: 55176"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "lockerman@paradedb.com",
            "name": "JLockerman",
            "username": "JLockerman"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "9a827ea3a0452e482e852785e743deb2f4630222",
          "message": "fix: Issues with text casts and memory layout for tokenizer types (#4900)\n\n# Ticket(s) Closed\n\n- fixes https://github.com/paradedb/paradedb/issues/5033\n\n## What\n\nChanges the tokenizer and alias types to function as regular SQL types\n(writable to disk, reallocatable in memory contexts etc).\n\n## Why\n\nWhen used incorrectly (eg. within a non-optimized function call) the\nprevious versions would access freed memory.\n\n## How\n\nThe tokenizer format is changed from `(header, magic_num, Oid, padding\nDatum)` to `(header, magic_num, metadata, padding, Oid, data_bytes)`\nwhere the `data_bytes` are the bytes from the original value (the\n`Datum` for by-value types, and the bytes pointed-at by the `Datum` for\nby-ref types). This lets us create a new `Datum` for that type (pointing\nat the inner bytes if needed).\n\nNOTE: Since the old version of the type was storing `Datum`s directly,\nany values stored to disk with the old code is broken unless they were\nin text format (the others store dangling pointers). In the updated\nversion such values will be output as meaningless text instead.\n\n## Tests\n\n- in\n`pg_search/tests/pg_regress/sql/tokenizer-types-inline-tokenization.sql`\n\n---------\n\nCo-authored-by: Mithun Chicklore Yogendra <mithun.cy@gmail.com>",
          "timestamp": "2026-05-08T14:32:32-04:00",
          "tree_id": "9cf77ffd18186494bb164cc15f9f703749357d03",
          "url": "https://github.com/paradedb/paradedb/commit/9a827ea3a0452e482e852785e743deb2f4630222"
        },
        "date": 1778266900807,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - tps",
            "value": 133.41655513726474,
            "unit": "median tps",
            "extra": "avg tps: 133.02022440266626, max tps: 139.72623880497363, count: 55180"
          },
          {
            "name": "Columnar Scan - Primary - tps",
            "value": 500.6837660823121,
            "unit": "median tps",
            "extra": "avg tps: 496.55275793827235, max tps: 604.2419311271138, count: 55180"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3255.5352136142005,
            "unit": "median tps",
            "extra": "avg tps: 3243.725249721801, max tps: 3269.173651708627, count: 55180"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 432.2600057519913,
            "unit": "median tps",
            "extra": "avg tps: 429.9304950025729, max tps: 573.8180042475922, count: 55180"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 2978.6831580916514,
            "unit": "median tps",
            "extra": "avg tps: 2972.5726179973954, max tps: 3014.763784814213, count: 110360"
          },
          {
            "name": "Normal Scan - Primary - tps",
            "value": 500.17195731929496,
            "unit": "median tps",
            "extra": "avg tps: 496.452438461649, max tps: 611.6167084868247, count: 55180"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1969.0311450225408,
            "unit": "median tps",
            "extra": "avg tps: 1966.203541263929, max tps: 1991.3861945059707, count: 55180"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 72.40809791751174,
            "unit": "median tps",
            "extra": "avg tps: 100.34893044469456, max tps: 304.25746162085517, count: 55180"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "rjhallsted@gmail.com",
            "name": "RJ Barman",
            "username": "barbarj"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "c5464b3e24db03ac3cb8bab3f1a344c417055c5b",
          "message": "chore: Refactor is_array branch in row_to_search_document (#5045)\n\n## What\nRefactor this block to separate the array conversion and then adding the\nelements of that array to the document into distinct steps.\n\n## Why\nIt's cleaner",
          "timestamp": "2026-05-08T15:16:13-06:00",
          "tree_id": "f8f52e91215e0a49ed69cefc3d821224ee7ddf3e",
          "url": "https://github.com/paradedb/paradedb/commit/c5464b3e24db03ac3cb8bab3f1a344c417055c5b"
        },
        "date": 1778276720398,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - tps",
            "value": 129.46361032140305,
            "unit": "median tps",
            "extra": "avg tps: 129.6304212369127, max tps: 145.1790517063132, count: 55060"
          },
          {
            "name": "Columnar Scan - Primary - tps",
            "value": 485.7165339982324,
            "unit": "median tps",
            "extra": "avg tps: 485.92342313993174, max tps: 586.1380737947835, count: 55060"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3187.146411172689,
            "unit": "median tps",
            "extra": "avg tps: 3177.531457303003, max tps: 3205.4678583491896, count: 55060"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 420.5480282954408,
            "unit": "median tps",
            "extra": "avg tps: 419.7012328960215, max tps: 513.7489933730661, count: 55060"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 2996.975584284241,
            "unit": "median tps",
            "extra": "avg tps: 3029.558369638343, max tps: 3130.6847490919067, count: 110120"
          },
          {
            "name": "Normal Scan - Primary - tps",
            "value": 488.28635762421993,
            "unit": "median tps",
            "extra": "avg tps: 489.19963565671827, max tps: 622.5922110947315, count: 55060"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1978.7236552945715,
            "unit": "median tps",
            "extra": "avg tps: 1972.1136780942632, max tps: 1992.2280487197522, count: 55060"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 72.53586596339115,
            "unit": "median tps",
            "extra": "avg tps: 85.76626134238369, max tps: 317.97044973422436, count: 55060"
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
        "date": 1778265041798,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - cpu",
            "value": 9.257474,
            "unit": "median cpu",
            "extra": "avg cpu: 8.444106413333927, max cpu: 28.857718, count: 55064"
          },
          {
            "name": "Aggregate Custom Scan - Primary - mem",
            "value": 65.1640625,
            "unit": "median mem",
            "extra": "avg mem: 65.07382027901623, max mem: 76.4921875, count: 55064"
          },
          {
            "name": "Columnar Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 5.635033606861062, max cpu: 15.047021, count: 55064"
          },
          {
            "name": "Columnar Scan - Primary - mem",
            "value": 64.1484375,
            "unit": "median mem",
            "extra": "avg mem: 63.996632185956344, max mem: 75.390625, count: 55064"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.612254281353617, max cpu: 9.311348, count: 55064"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 36.23046875,
            "unit": "median mem",
            "extra": "avg mem: 35.8498202092111, max mem: 37.6328125, count: 55064"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.6006067618221, max cpu: 9.320388, count: 55064"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 62.78515625,
            "unit": "median mem",
            "extra": "avg mem: 62.44837314306988, max mem: 74.203125, count: 55064"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.682727196032935, max cpu: 9.320388, count: 110128"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 54.76171875,
            "unit": "median mem",
            "extra": "avg mem: 54.690958227301415, max mem: 70.078125, count: 110128"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 1783,
            "unit": "median block_count",
            "extra": "avg block_count: 1785.2058513729478, max block_count: 3181.0, count: 55064"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 14,
            "unit": "median segment_count",
            "extra": "avg segment_count: 14.505775098067703, max segment_count: 30.0, count: 55064"
          },
          {
            "name": "Normal Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 5.571888885242803, max cpu: 14.229248, count: 55064"
          },
          {
            "name": "Normal Scan - Primary - mem",
            "value": 64.0234375,
            "unit": "median mem",
            "extra": "avg mem: 63.876491730077724, max mem: 75.2578125, count: 55064"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.61438713458175, max cpu: 9.320388, count: 55064"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 53.80859375,
            "unit": "median mem",
            "extra": "avg mem: 53.73335991698296, max mem: 64.890625, count: 55064"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.436851710992277, max cpu: 4.701273, count: 55064"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 56.0703125,
            "unit": "median mem",
            "extra": "avg mem: 55.1871909136414, max mem: 68.11328125, count: 55064"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "JLockerman",
            "username": "JLockerman",
            "email": "lockerman@paradedb.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "9a827ea3a0452e482e852785e743deb2f4630222",
          "message": "fix: Issues with text casts and memory layout for tokenizer types (#4900)\n\n# Ticket(s) Closed\n\n- fixes https://github.com/paradedb/paradedb/issues/5033\n\n## What\n\nChanges the tokenizer and alias types to function as regular SQL types\n(writable to disk, reallocatable in memory contexts etc).\n\n## Why\n\nWhen used incorrectly (eg. within a non-optimized function call) the\nprevious versions would access freed memory.\n\n## How\n\nThe tokenizer format is changed from `(header, magic_num, Oid, padding\nDatum)` to `(header, magic_num, metadata, padding, Oid, data_bytes)`\nwhere the `data_bytes` are the bytes from the original value (the\n`Datum` for by-value types, and the bytes pointed-at by the `Datum` for\nby-ref types). This lets us create a new `Datum` for that type (pointing\nat the inner bytes if needed).\n\nNOTE: Since the old version of the type was storing `Datum`s directly,\nany values stored to disk with the old code is broken unless they were\nin text format (the others store dangling pointers). In the updated\nversion such values will be output as meaningless text instead.\n\n## Tests\n\n- in\n`pg_search/tests/pg_regress/sql/tokenizer-types-inline-tokenization.sql`\n\n---------\n\nCo-authored-by: Mithun Chicklore Yogendra <mithun.cy@gmail.com>",
          "timestamp": "2026-05-08T18:32:32Z",
          "url": "https://github.com/paradedb/paradedb/commit/9a827ea3a0452e482e852785e743deb2f4630222"
        },
        "date": 1778265197043,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - cpu",
            "value": 9.257474,
            "unit": "median cpu",
            "extra": "avg cpu: 8.272619692367599, max cpu: 19.393938, count: 55176"
          },
          {
            "name": "Aggregate Custom Scan - Primary - mem",
            "value": 66.70703125,
            "unit": "median mem",
            "extra": "avg mem: 66.67085985358217, max mem: 78.12109375, count: 55176"
          },
          {
            "name": "Columnar Scan - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 5.5707800372282845, max cpu: 18.82353, count: 55176"
          },
          {
            "name": "Columnar Scan - Primary - mem",
            "value": 65.21875,
            "unit": "median mem",
            "extra": "avg mem: 65.18758261914148, max mem: 76.69921875, count: 55176"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.582990296092643, max cpu: 4.7524753, count: 55176"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 35.45703125,
            "unit": "median mem",
            "extra": "avg mem: 35.242661763606456, max mem: 36.78515625, count: 55176"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.806225831003867, max cpu: 9.356726, count: 55176"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 63.53515625,
            "unit": "median mem",
            "extra": "avg mem: 63.242651356568075, max mem: 75.17578125, count: 55176"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.619845763621786, max cpu: 9.329447, count: 110352"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 50.734375,
            "unit": "median mem",
            "extra": "avg mem: 50.71786186192819, max mem: 70.23046875, count: 110352"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 1799,
            "unit": "median block_count",
            "extra": "avg block_count: 1807.300492967957, max block_count: 3200.0, count: 55176"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 14,
            "unit": "median segment_count",
            "extra": "avg segment_count: 13.817239379440336, max segment_count: 30.0, count: 55176"
          },
          {
            "name": "Normal Scan - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 5.548532701831689, max cpu: 14.299901, count: 55176"
          },
          {
            "name": "Normal Scan - Primary - mem",
            "value": 64.9140625,
            "unit": "median mem",
            "extra": "avg mem: 64.93621342102998, max mem: 76.4609375, count: 55176"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.594269519807903, max cpu: 4.7619047, count: 55176"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 53.859375,
            "unit": "median mem",
            "extra": "avg mem: 53.7002999492533, max mem: 64.72265625, count: 55176"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 3.560432512738818, max cpu: 4.628737, count: 55176"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 56.6171875,
            "unit": "median mem",
            "extra": "avg mem: 51.212382804004456, max mem: 68.6015625, count: 55176"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "lockerman@paradedb.com",
            "name": "JLockerman",
            "username": "JLockerman"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "9a827ea3a0452e482e852785e743deb2f4630222",
          "message": "fix: Issues with text casts and memory layout for tokenizer types (#4900)\n\n# Ticket(s) Closed\n\n- fixes https://github.com/paradedb/paradedb/issues/5033\n\n## What\n\nChanges the tokenizer and alias types to function as regular SQL types\n(writable to disk, reallocatable in memory contexts etc).\n\n## Why\n\nWhen used incorrectly (eg. within a non-optimized function call) the\nprevious versions would access freed memory.\n\n## How\n\nThe tokenizer format is changed from `(header, magic_num, Oid, padding\nDatum)` to `(header, magic_num, metadata, padding, Oid, data_bytes)`\nwhere the `data_bytes` are the bytes from the original value (the\n`Datum` for by-value types, and the bytes pointed-at by the `Datum` for\nby-ref types). This lets us create a new `Datum` for that type (pointing\nat the inner bytes if needed).\n\nNOTE: Since the old version of the type was storing `Datum`s directly,\nany values stored to disk with the old code is broken unless they were\nin text format (the others store dangling pointers). In the updated\nversion such values will be output as meaningless text instead.\n\n## Tests\n\n- in\n`pg_search/tests/pg_regress/sql/tokenizer-types-inline-tokenization.sql`\n\n---------\n\nCo-authored-by: Mithun Chicklore Yogendra <mithun.cy@gmail.com>",
          "timestamp": "2026-05-08T14:32:32-04:00",
          "tree_id": "9cf77ffd18186494bb164cc15f9f703749357d03",
          "url": "https://github.com/paradedb/paradedb/commit/9a827ea3a0452e482e852785e743deb2f4630222"
        },
        "date": 1778266941959,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - cpu",
            "value": 9.257474,
            "unit": "median cpu",
            "extra": "avg cpu: 8.508859233510707, max cpu: 23.529411, count: 55180"
          },
          {
            "name": "Aggregate Custom Scan - Primary - mem",
            "value": 66.3046875,
            "unit": "median mem",
            "extra": "avg mem: 66.16462019187206, max mem: 77.41796875, count: 55180"
          },
          {
            "name": "Columnar Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 5.605055380833783, max cpu: 18.622696, count: 55180"
          },
          {
            "name": "Columnar Scan - Primary - mem",
            "value": 65.22265625,
            "unit": "median mem",
            "extra": "avg mem: 65.09326741742932, max mem: 76.375, count: 55180"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.642277349010782, max cpu: 9.248554, count: 55180"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 36.4453125,
            "unit": "median mem",
            "extra": "avg mem: 35.99124031578471, max mem: 37.2265625, count: 55180"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.814003898928115, max cpu: 9.320388, count: 55180"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 63.21484375,
            "unit": "median mem",
            "extra": "avg mem: 62.864474716269484, max mem: 74.5859375, count: 55180"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.645606984697102, max cpu: 9.338522, count: 110360"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 62.14453125,
            "unit": "median mem",
            "extra": "avg mem: 61.282312892182404, max mem: 73.72265625, count: 110360"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 1772,
            "unit": "median block_count",
            "extra": "avg block_count: 1768.2072671257702, max block_count: 3138.0, count: 55180"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 14,
            "unit": "median segment_count",
            "extra": "avg segment_count: 14.225607104023197, max segment_count: 30.0, count: 55180"
          },
          {
            "name": "Normal Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 5.641663908091566, max cpu: 19.277107, count: 55180"
          },
          {
            "name": "Normal Scan - Primary - mem",
            "value": 64.89453125,
            "unit": "median mem",
            "extra": "avg mem: 64.77735996114987, max mem: 76.06640625, count: 55180"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.6664619048272495, max cpu: 9.204219, count: 55180"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 53.70703125,
            "unit": "median mem",
            "extra": "avg mem: 53.536278217311526, max mem: 64.59375, count: 55180"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 3.004639710572733, max cpu: 4.64666, count: 55180"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 57.64453125,
            "unit": "median mem",
            "extra": "avg mem: 56.293996777591516, max mem: 68.9140625, count: 55180"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "rjhallsted@gmail.com",
            "name": "RJ Barman",
            "username": "barbarj"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "c5464b3e24db03ac3cb8bab3f1a344c417055c5b",
          "message": "chore: Refactor is_array branch in row_to_search_document (#5045)\n\n## What\nRefactor this block to separate the array conversion and then adding the\nelements of that array to the document into distinct steps.\n\n## Why\nIt's cleaner",
          "timestamp": "2026-05-08T15:16:13-06:00",
          "tree_id": "f8f52e91215e0a49ed69cefc3d821224ee7ddf3e",
          "url": "https://github.com/paradedb/paradedb/commit/c5464b3e24db03ac3cb8bab3f1a344c417055c5b"
        },
        "date": 1778276758046,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - cpu",
            "value": 9.284333,
            "unit": "median cpu",
            "extra": "avg cpu: 8.558457687737423, max cpu: 23.645319, count: 55060"
          },
          {
            "name": "Aggregate Custom Scan - Primary - mem",
            "value": 66.78125,
            "unit": "median mem",
            "extra": "avg mem: 66.42559523133853, max mem: 77.53125, count: 55060"
          },
          {
            "name": "Columnar Scan - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 5.703450481765656, max cpu: 18.786694, count: 55060"
          },
          {
            "name": "Columnar Scan - Primary - mem",
            "value": 65.73828125,
            "unit": "median mem",
            "extra": "avg mem: 65.31757982768798, max mem: 76.390625, count: 55060"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.5881057644796925, max cpu: 4.824121, count: 55060"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 36.203125,
            "unit": "median mem",
            "extra": "avg mem: 36.05167728954777, max mem: 38.02734375, count: 55060"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.71375927434788, max cpu: 9.320388, count: 55060"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 63.84375,
            "unit": "median mem",
            "extra": "avg mem: 63.19594611276335, max mem: 74.6171875, count: 55060"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.643983613801625, max cpu: 9.311348, count: 110120"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 55.2421875,
            "unit": "median mem",
            "extra": "avg mem: 54.38170224102116, max mem: 69.95703125, count: 110120"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 1785,
            "unit": "median block_count",
            "extra": "avg block_count: 1776.9277697057755, max block_count: 3126.0, count: 55060"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 19,
            "unit": "median segment_count",
            "extra": "avg segment_count: 18.238394478750454, max segment_count: 30.0, count: 55060"
          },
          {
            "name": "Normal Scan - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 5.674876197103501, max cpu: 18.972332, count: 55060"
          },
          {
            "name": "Normal Scan - Primary - mem",
            "value": 65.27734375,
            "unit": "median mem",
            "extra": "avg mem: 64.90551209759808, max mem: 76.02734375, count: 55060"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.599389251918843, max cpu: 9.311348, count: 55060"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 54.06640625,
            "unit": "median mem",
            "extra": "avg mem: 53.4879441177806, max mem: 64.50390625, count: 55060"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.223434887559921, max cpu: 4.7477746, count: 55060"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 56.96484375,
            "unit": "median mem",
            "extra": "avg mem: 56.36784480850436, max mem: 68.9609375, count: 55060"
          }
        ]
      }
    ],
    "pg_search bulk-updates.toml Performance - TPS": [
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
        "date": 1778265307653,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 7.761464733610281,
            "unit": "median tps",
            "extra": "avg tps: 6.64796981624041, max tps: 10.124409682416042, count: 57806"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.547733949959475,
            "unit": "median tps",
            "extra": "avg tps: 4.970588785556358, max tps: 6.237019264823482, count: 57806"
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
        "date": 1778265493662,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 7.822520793066646,
            "unit": "median tps",
            "extra": "avg tps: 6.6797761149314745, max tps: 10.111696132649264, count: 57971"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.407919002468278,
            "unit": "median tps",
            "extra": "avg tps: 4.8551408554635875, max tps: 6.078631217475262, count: 57971"
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
        "date": 1778265716503,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 7.724239389222684,
            "unit": "median tps",
            "extra": "avg tps: 6.602240857190439, max tps: 9.990579572955813, count: 57254"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.477587463778522,
            "unit": "median tps",
            "extra": "avg tps: 4.921821036670575, max tps: 6.164970557989403, count: 57254"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "JLockerman",
            "username": "JLockerman",
            "email": "lockerman@paradedb.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "9a827ea3a0452e482e852785e743deb2f4630222",
          "message": "fix: Issues with text casts and memory layout for tokenizer types (#4900)\n\n# Ticket(s) Closed\n\n- fixes https://github.com/paradedb/paradedb/issues/5033\n\n## What\n\nChanges the tokenizer and alias types to function as regular SQL types\n(writable to disk, reallocatable in memory contexts etc).\n\n## Why\n\nWhen used incorrectly (eg. within a non-optimized function call) the\nprevious versions would access freed memory.\n\n## How\n\nThe tokenizer format is changed from `(header, magic_num, Oid, padding\nDatum)` to `(header, magic_num, metadata, padding, Oid, data_bytes)`\nwhere the `data_bytes` are the bytes from the original value (the\n`Datum` for by-value types, and the bytes pointed-at by the `Datum` for\nby-ref types). This lets us create a new `Datum` for that type (pointing\nat the inner bytes if needed).\n\nNOTE: Since the old version of the type was storing `Datum`s directly,\nany values stored to disk with the old code is broken unless they were\nin text format (the others store dangling pointers). In the updated\nversion such values will be output as meaningless text instead.\n\n## Tests\n\n- in\n`pg_search/tests/pg_regress/sql/tokenizer-types-inline-tokenization.sql`\n\n---------\n\nCo-authored-by: Mithun Chicklore Yogendra <mithun.cy@gmail.com>",
          "timestamp": "2026-05-08T18:32:32Z",
          "url": "https://github.com/paradedb/paradedb/commit/9a827ea3a0452e482e852785e743deb2f4630222"
        },
        "date": 1778265871140,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 7.941954642660017,
            "unit": "median tps",
            "extra": "avg tps: 6.757098818499271, max tps: 10.233467055515499, count: 57761"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.497927215315403,
            "unit": "median tps",
            "extra": "avg tps: 4.9388918024514235, max tps: 6.165032153296289, count: 57761"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "lockerman@paradedb.com",
            "name": "JLockerman",
            "username": "JLockerman"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "9a827ea3a0452e482e852785e743deb2f4630222",
          "message": "fix: Issues with text casts and memory layout for tokenizer types (#4900)\n\n# Ticket(s) Closed\n\n- fixes https://github.com/paradedb/paradedb/issues/5033\n\n## What\n\nChanges the tokenizer and alias types to function as regular SQL types\n(writable to disk, reallocatable in memory contexts etc).\n\n## Why\n\nWhen used incorrectly (eg. within a non-optimized function call) the\nprevious versions would access freed memory.\n\n## How\n\nThe tokenizer format is changed from `(header, magic_num, Oid, padding\nDatum)` to `(header, magic_num, metadata, padding, Oid, data_bytes)`\nwhere the `data_bytes` are the bytes from the original value (the\n`Datum` for by-value types, and the bytes pointed-at by the `Datum` for\nby-ref types). This lets us create a new `Datum` for that type (pointing\nat the inner bytes if needed).\n\nNOTE: Since the old version of the type was storing `Datum`s directly,\nany values stored to disk with the old code is broken unless they were\nin text format (the others store dangling pointers). In the updated\nversion such values will be output as meaningless text instead.\n\n## Tests\n\n- in\n`pg_search/tests/pg_regress/sql/tokenizer-types-inline-tokenization.sql`\n\n---------\n\nCo-authored-by: Mithun Chicklore Yogendra <mithun.cy@gmail.com>",
          "timestamp": "2026-05-08T14:32:32-04:00",
          "tree_id": "9cf77ffd18186494bb164cc15f9f703749357d03",
          "url": "https://github.com/paradedb/paradedb/commit/9a827ea3a0452e482e852785e743deb2f4630222"
        },
        "date": 1778267616920,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 7.840576260888336,
            "unit": "median tps",
            "extra": "avg tps: 6.6947529729047615, max tps: 10.059681551909978, count: 57241"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.483911828830073,
            "unit": "median tps",
            "extra": "avg tps: 4.917001170005764, max tps: 6.1740980972730535, count: 57241"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "rjhallsted@gmail.com",
            "name": "RJ Barman",
            "username": "barbarj"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "c5464b3e24db03ac3cb8bab3f1a344c417055c5b",
          "message": "chore: Refactor is_array branch in row_to_search_document (#5045)\n\n## What\nRefactor this block to separate the array conversion and then adding the\nelements of that array to the document into distinct steps.\n\n## Why\nIt's cleaner",
          "timestamp": "2026-05-08T15:16:13-06:00",
          "tree_id": "f8f52e91215e0a49ed69cefc3d821224ee7ddf3e",
          "url": "https://github.com/paradedb/paradedb/commit/c5464b3e24db03ac3cb8bab3f1a344c417055c5b"
        },
        "date": 1778277437003,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 7.57750625634313,
            "unit": "median tps",
            "extra": "avg tps: 6.479428874458487, max tps: 9.838173578521856, count: 57765"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.624266082575761,
            "unit": "median tps",
            "extra": "avg tps: 5.039313199624603, max tps: 6.318468538763891, count: 57765"
          }
        ]
      }
    ],
    "pg_search bulk-updates.toml Performance - Other Metrics": [
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
        "date": 1778265346089,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.27837,
            "unit": "median cpu",
            "extra": "avg cpu: 21.106170660423142, max cpu: 42.942345, count: 57806"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 230.19921875,
            "unit": "median mem",
            "extra": "avg mem: 230.10951420484034, max mem: 231.6875, count: 57806"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.391813,
            "unit": "median cpu",
            "extra": "avg cpu: 22.345296363077562, max cpu: 33.333336, count: 57806"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 172.1953125,
            "unit": "median mem",
            "extra": "avg mem: 172.13933113831177, max mem: 172.89453125, count: 57806"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 34293,
            "unit": "median block_count",
            "extra": "avg block_count: 33728.08724007889, max block_count: 36656.0, count: 57806"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 79,
            "unit": "median segment_count",
            "extra": "avg segment_count: 81.8108154862817, max segment_count: 132.0, count: 57806"
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
        "date": 1778265534966,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.233301,
            "unit": "median cpu",
            "extra": "avg cpu: 21.054717038693386, max cpu: 42.942345, count: 57971"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 233.5234375,
            "unit": "median mem",
            "extra": "avg mem: 233.38646586224147, max mem: 235.0, count: 57971"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.346306,
            "unit": "median cpu",
            "extra": "avg cpu: 22.481455249580964, max cpu: 33.366436, count: 57971"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 175.71484375,
            "unit": "median mem",
            "extra": "avg mem: 175.5281981642761, max mem: 176.27734375, count: 57971"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 34377,
            "unit": "median block_count",
            "extra": "avg block_count: 33698.522899380725, max block_count: 36374.0, count: 57971"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 79,
            "unit": "median segment_count",
            "extra": "avg segment_count: 81.70348967587242, max segment_count: 130.0, count: 57971"
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
        "date": 1778265761080,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.210833,
            "unit": "median cpu",
            "extra": "avg cpu: 21.02514697506792, max cpu: 42.814667, count: 57254"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 234.21875,
            "unit": "median mem",
            "extra": "avg mem: 234.1082588798381, max mem: 235.6953125, count: 57254"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.30097,
            "unit": "median cpu",
            "extra": "avg cpu: 22.36709697737051, max cpu: 33.20158, count: 57254"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 176.90234375,
            "unit": "median mem",
            "extra": "avg mem: 176.97981213648828, max mem: 177.58203125, count: 57254"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 34439,
            "unit": "median block_count",
            "extra": "avg block_count: 33721.75820030041, max block_count: 36483.0, count: 57254"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 78,
            "unit": "median segment_count",
            "extra": "avg segment_count: 81.29395326090753, max segment_count: 128.0, count: 57254"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "JLockerman",
            "username": "JLockerman",
            "email": "lockerman@paradedb.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "9a827ea3a0452e482e852785e743deb2f4630222",
          "message": "fix: Issues with text casts and memory layout for tokenizer types (#4900)\n\n# Ticket(s) Closed\n\n- fixes https://github.com/paradedb/paradedb/issues/5033\n\n## What\n\nChanges the tokenizer and alias types to function as regular SQL types\n(writable to disk, reallocatable in memory contexts etc).\n\n## Why\n\nWhen used incorrectly (eg. within a non-optimized function call) the\nprevious versions would access freed memory.\n\n## How\n\nThe tokenizer format is changed from `(header, magic_num, Oid, padding\nDatum)` to `(header, magic_num, metadata, padding, Oid, data_bytes)`\nwhere the `data_bytes` are the bytes from the original value (the\n`Datum` for by-value types, and the bytes pointed-at by the `Datum` for\nby-ref types). This lets us create a new `Datum` for that type (pointing\nat the inner bytes if needed).\n\nNOTE: Since the old version of the type was storing `Datum`s directly,\nany values stored to disk with the old code is broken unless they were\nin text format (the others store dangling pointers). In the updated\nversion such values will be output as meaningless text instead.\n\n## Tests\n\n- in\n`pg_search/tests/pg_regress/sql/tokenizer-types-inline-tokenization.sql`\n\n---------\n\nCo-authored-by: Mithun Chicklore Yogendra <mithun.cy@gmail.com>",
          "timestamp": "2026-05-08T18:32:32Z",
          "url": "https://github.com/paradedb/paradedb/commit/9a827ea3a0452e482e852785e743deb2f4630222"
        },
        "date": 1778265907372,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.233301,
            "unit": "median cpu",
            "extra": "avg cpu: 20.66963584979329, max cpu: 43.59233, count: 57761"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 235.37109375,
            "unit": "median mem",
            "extra": "avg mem: 235.20088401722182, max mem: 236.84765625, count: 57761"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.346306,
            "unit": "median cpu",
            "extra": "avg cpu: 22.426088353568023, max cpu: 33.366436, count: 57761"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 177.1796875,
            "unit": "median mem",
            "extra": "avg mem: 177.0406366725602, max mem: 177.90625, count: 57761"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 34504,
            "unit": "median block_count",
            "extra": "avg block_count: 33770.8826716989, max block_count: 36658.0, count: 57761"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 79,
            "unit": "median segment_count",
            "extra": "avg segment_count: 81.95896885441734, max segment_count: 131.0, count: 57761"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "lockerman@paradedb.com",
            "name": "JLockerman",
            "username": "JLockerman"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "9a827ea3a0452e482e852785e743deb2f4630222",
          "message": "fix: Issues with text casts and memory layout for tokenizer types (#4900)\n\n# Ticket(s) Closed\n\n- fixes https://github.com/paradedb/paradedb/issues/5033\n\n## What\n\nChanges the tokenizer and alias types to function as regular SQL types\n(writable to disk, reallocatable in memory contexts etc).\n\n## Why\n\nWhen used incorrectly (eg. within a non-optimized function call) the\nprevious versions would access freed memory.\n\n## How\n\nThe tokenizer format is changed from `(header, magic_num, Oid, padding\nDatum)` to `(header, magic_num, metadata, padding, Oid, data_bytes)`\nwhere the `data_bytes` are the bytes from the original value (the\n`Datum` for by-value types, and the bytes pointed-at by the `Datum` for\nby-ref types). This lets us create a new `Datum` for that type (pointing\nat the inner bytes if needed).\n\nNOTE: Since the old version of the type was storing `Datum`s directly,\nany values stored to disk with the old code is broken unless they were\nin text format (the others store dangling pointers). In the updated\nversion such values will be output as meaningless text instead.\n\n## Tests\n\n- in\n`pg_search/tests/pg_regress/sql/tokenizer-types-inline-tokenization.sql`\n\n---------\n\nCo-authored-by: Mithun Chicklore Yogendra <mithun.cy@gmail.com>",
          "timestamp": "2026-05-08T14:32:32-04:00",
          "tree_id": "9cf77ffd18186494bb164cc15f9f703749357d03",
          "url": "https://github.com/paradedb/paradedb/commit/9a827ea3a0452e482e852785e743deb2f4630222"
        },
        "date": 1778267658953,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.210833,
            "unit": "median cpu",
            "extra": "avg cpu: 20.95453969873176, max cpu: 42.857143, count: 57241"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 235.19140625,
            "unit": "median mem",
            "extra": "avg mem: 235.07082342911986, max mem: 236.66015625, count: 57241"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.323614,
            "unit": "median cpu",
            "extra": "avg cpu: 22.417028400660744, max cpu: 33.333336, count: 57241"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 177.25,
            "unit": "median mem",
            "extra": "avg mem: 177.15841839492234, max mem: 177.8671875, count: 57241"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 34572,
            "unit": "median block_count",
            "extra": "avg block_count: 33796.012176586715, max block_count: 36298.0, count: 57241"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 79,
            "unit": "median segment_count",
            "extra": "avg segment_count: 81.69427508254573, max segment_count: 129.0, count: 57241"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "rjhallsted@gmail.com",
            "name": "RJ Barman",
            "username": "barbarj"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "c5464b3e24db03ac3cb8bab3f1a344c417055c5b",
          "message": "chore: Refactor is_array branch in row_to_search_document (#5045)\n\n## What\nRefactor this block to separate the array conversion and then adding the\nelements of that array to the document into distinct steps.\n\n## Why\nIt's cleaner",
          "timestamp": "2026-05-08T15:16:13-06:00",
          "tree_id": "f8f52e91215e0a49ed69cefc3d821224ee7ddf3e",
          "url": "https://github.com/paradedb/paradedb/commit/c5464b3e24db03ac3cb8bab3f1a344c417055c5b"
        },
        "date": 1778277474656,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.27837,
            "unit": "median cpu",
            "extra": "avg cpu: 21.036268896632222, max cpu: 43.02789, count: 57765"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 235.26953125,
            "unit": "median mem",
            "extra": "avg mem: 235.17632399322687, max mem: 236.7421875, count: 57765"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.346306,
            "unit": "median cpu",
            "extra": "avg cpu: 22.300396443939874, max cpu: 33.23442, count: 57765"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 177.53515625,
            "unit": "median mem",
            "extra": "avg mem: 177.22341247944257, max mem: 177.88671875, count: 57765"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 33896,
            "unit": "median block_count",
            "extra": "avg block_count: 33524.72408898122, max block_count: 35953.0, count: 57765"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 78,
            "unit": "median segment_count",
            "extra": "avg segment_count: 80.60884618713754, max segment_count: 125.0, count: 57765"
          }
        ]
      }
    ],
    "pg_search wide-table.toml Performance - TPS": [
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
        "date": 1778266039282,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 1135.3686704270908,
            "unit": "median tps",
            "extra": "avg tps: 1135.9031528383039, max tps: 1182.3043000801704, count: 56443"
          },
          {
            "name": "Single Insert - Primary - tps",
            "value": 1255.5500959355659,
            "unit": "median tps",
            "extra": "avg tps: 1243.120362700185, max tps: 1266.6779545024124, count: 56443"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 1912.3350734199855,
            "unit": "median tps",
            "extra": "avg tps: 1877.002157927372, max tps: 2067.3487022415707, count: 56443"
          },
          {
            "name": "Top N - Primary - tps",
            "value": 5.1753282548798225,
            "unit": "median tps",
            "extra": "avg tps: 5.224267981626942, max tps: 7.104795585688194, count: 56443"
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
        "date": 1778266228642,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 1113.370537496831,
            "unit": "median tps",
            "extra": "avg tps: 1116.2163682983348, max tps: 1172.6005309026839, count: 56191"
          },
          {
            "name": "Single Insert - Primary - tps",
            "value": 627.8088597145339,
            "unit": "median tps",
            "extra": "avg tps: 577.1291306952212, max tps: 1219.3836612373154, count: 56191"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 1823.572446252476,
            "unit": "median tps",
            "extra": "avg tps: 1794.0822625409473, max tps: 1980.9591918970427, count: 56191"
          },
          {
            "name": "Top K - Primary - tps",
            "value": 5.299997267573098,
            "unit": "median tps",
            "extra": "avg tps: 5.341394309597988, max tps: 7.197439697960883, count: 56191"
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
        "date": 1778266455380,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 1154.4737587749366,
            "unit": "median tps",
            "extra": "avg tps: 1158.3088484419575, max tps: 1214.3944275146157, count: 56341"
          },
          {
            "name": "Single Insert - Primary - tps",
            "value": 1283.9235209979818,
            "unit": "median tps",
            "extra": "avg tps: 1272.7086164973439, max tps: 1291.1778838049884, count: 56341"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 1098.0039760382156,
            "unit": "median tps",
            "extra": "avg tps: 1008.9320985398803, max tps: 1593.4576171257743, count: 56341"
          },
          {
            "name": "Top K - Primary - tps",
            "value": 5.306491417782708,
            "unit": "median tps",
            "extra": "avg tps: 5.332639821621765, max tps: 6.875938703151754, count: 56341"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "JLockerman",
            "username": "JLockerman",
            "email": "lockerman@paradedb.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "9a827ea3a0452e482e852785e743deb2f4630222",
          "message": "fix: Issues with text casts and memory layout for tokenizer types (#4900)\n\n# Ticket(s) Closed\n\n- fixes https://github.com/paradedb/paradedb/issues/5033\n\n## What\n\nChanges the tokenizer and alias types to function as regular SQL types\n(writable to disk, reallocatable in memory contexts etc).\n\n## Why\n\nWhen used incorrectly (eg. within a non-optimized function call) the\nprevious versions would access freed memory.\n\n## How\n\nThe tokenizer format is changed from `(header, magic_num, Oid, padding\nDatum)` to `(header, magic_num, metadata, padding, Oid, data_bytes)`\nwhere the `data_bytes` are the bytes from the original value (the\n`Datum` for by-value types, and the bytes pointed-at by the `Datum` for\nby-ref types). This lets us create a new `Datum` for that type (pointing\nat the inner bytes if needed).\n\nNOTE: Since the old version of the type was storing `Datum`s directly,\nany values stored to disk with the old code is broken unless they were\nin text format (the others store dangling pointers). In the updated\nversion such values will be output as meaningless text instead.\n\n## Tests\n\n- in\n`pg_search/tests/pg_regress/sql/tokenizer-types-inline-tokenization.sql`\n\n---------\n\nCo-authored-by: Mithun Chicklore Yogendra <mithun.cy@gmail.com>",
          "timestamp": "2026-05-08T18:32:32Z",
          "url": "https://github.com/paradedb/paradedb/commit/9a827ea3a0452e482e852785e743deb2f4630222"
        },
        "date": 1778266600925,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 1156.8812119617207,
            "unit": "median tps",
            "extra": "avg tps: 1157.6496253121984, max tps: 1205.725189751743, count: 56051"
          },
          {
            "name": "Single Insert - Primary - tps",
            "value": 1338.9917021249942,
            "unit": "median tps",
            "extra": "avg tps: 1328.3109233279947, max tps: 1345.9203607235065, count: 56051"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 1803.927295621786,
            "unit": "median tps",
            "extra": "avg tps: 1783.8213090958986, max tps: 1928.8615511816781, count: 56051"
          },
          {
            "name": "Top K - Primary - tps",
            "value": 5.310445813093201,
            "unit": "median tps",
            "extra": "avg tps: 5.35545347113524, max tps: 7.617161696864752, count: 56051"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "lockerman@paradedb.com",
            "name": "JLockerman",
            "username": "JLockerman"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "9a827ea3a0452e482e852785e743deb2f4630222",
          "message": "fix: Issues with text casts and memory layout for tokenizer types (#4900)\n\n# Ticket(s) Closed\n\n- fixes https://github.com/paradedb/paradedb/issues/5033\n\n## What\n\nChanges the tokenizer and alias types to function as regular SQL types\n(writable to disk, reallocatable in memory contexts etc).\n\n## Why\n\nWhen used incorrectly (eg. within a non-optimized function call) the\nprevious versions would access freed memory.\n\n## How\n\nThe tokenizer format is changed from `(header, magic_num, Oid, padding\nDatum)` to `(header, magic_num, metadata, padding, Oid, data_bytes)`\nwhere the `data_bytes` are the bytes from the original value (the\n`Datum` for by-value types, and the bytes pointed-at by the `Datum` for\nby-ref types). This lets us create a new `Datum` for that type (pointing\nat the inner bytes if needed).\n\nNOTE: Since the old version of the type was storing `Datum`s directly,\nany values stored to disk with the old code is broken unless they were\nin text format (the others store dangling pointers). In the updated\nversion such values will be output as meaningless text instead.\n\n## Tests\n\n- in\n`pg_search/tests/pg_regress/sql/tokenizer-types-inline-tokenization.sql`\n\n---------\n\nCo-authored-by: Mithun Chicklore Yogendra <mithun.cy@gmail.com>",
          "timestamp": "2026-05-08T14:32:32-04:00",
          "tree_id": "9cf77ffd18186494bb164cc15f9f703749357d03",
          "url": "https://github.com/paradedb/paradedb/commit/9a827ea3a0452e482e852785e743deb2f4630222"
        },
        "date": 1778268353406,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 1131.979648907775,
            "unit": "median tps",
            "extra": "avg tps: 1130.8868772624676, max tps: 1183.9216770259645, count: 56276"
          },
          {
            "name": "Single Insert - Primary - tps",
            "value": 1238.488050880942,
            "unit": "median tps",
            "extra": "avg tps: 1235.249798977946, max tps: 1265.2186235801046, count: 56276"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 1899.3142369235095,
            "unit": "median tps",
            "extra": "avg tps: 1857.649696140123, max tps: 2039.720718483873, count: 56276"
          },
          {
            "name": "Top K - Primary - tps",
            "value": 5.595694663094926,
            "unit": "median tps",
            "extra": "avg tps: 5.578274074355892, max tps: 6.608855985318849, count: 56276"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "rjhallsted@gmail.com",
            "name": "RJ Barman",
            "username": "barbarj"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "c5464b3e24db03ac3cb8bab3f1a344c417055c5b",
          "message": "chore: Refactor is_array branch in row_to_search_document (#5045)\n\n## What\nRefactor this block to separate the array conversion and then adding the\nelements of that array to the document into distinct steps.\n\n## Why\nIt's cleaner",
          "timestamp": "2026-05-08T15:16:13-06:00",
          "tree_id": "f8f52e91215e0a49ed69cefc3d821224ee7ddf3e",
          "url": "https://github.com/paradedb/paradedb/commit/c5464b3e24db03ac3cb8bab3f1a344c417055c5b"
        },
        "date": 1778278168315,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 1104.2007050471018,
            "unit": "median tps",
            "extra": "avg tps: 1107.4149508768173, max tps: 1153.9193411173896, count: 56109"
          },
          {
            "name": "Single Insert - Primary - tps",
            "value": 1234.2659113043585,
            "unit": "median tps",
            "extra": "avg tps: 1232.6918855667345, max tps: 1278.5001615653127, count: 56109"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 1131.2263968461682,
            "unit": "median tps",
            "extra": "avg tps: 1035.973834253223, max tps: 1523.6557583482863, count: 56109"
          },
          {
            "name": "Top K - Primary - tps",
            "value": 5.294029514586228,
            "unit": "median tps",
            "extra": "avg tps: 5.353475948286726, max tps: 6.690312267046811, count: 56109"
          }
        ]
      }
    ],
    "pg_search wide-table.toml Performance - Other Metrics": [
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
        "date": 1778266078324,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Background Merger - Primary - background_merging",
            "value": 0,
            "unit": "median background_merging",
            "extra": "avg background_merging: 0.0787874492851195, max background_merging: 2.0, count: 56443"
          },
          {
            "name": "Background Merger - Primary - cpu",
            "value": 4.673807,
            "unit": "median cpu",
            "extra": "avg cpu: 4.8313681732779745, max cpu: 9.657948, count: 56443"
          },
          {
            "name": "Background Merger - Primary - mem",
            "value": 27.35546875,
            "unit": "median mem",
            "extra": "avg mem: 27.40826565351328, max mem: 27.4765625, count: 56443"
          },
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 4.673807,
            "unit": "median cpu",
            "extra": "avg cpu: 4.948131915591215, max cpu: 9.825998, count: 56443"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 166,
            "unit": "median mem",
            "extra": "avg mem: 164.54179880587495, max mem: 166.1171875, count: 56443"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 51470,
            "unit": "median block_count",
            "extra": "avg block_count: 51330.59805467463, max block_count: 51470.0, count: 56443"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 45,
            "unit": "median segment_count",
            "extra": "avg segment_count: 42.61524015378346, max segment_count: 56.0, count: 56443"
          },
          {
            "name": "Single Insert - Primary - cpu",
            "value": 4.673807,
            "unit": "median cpu",
            "extra": "avg cpu: 4.6283668938634035, max cpu: 9.81595, count: 56443"
          },
          {
            "name": "Single Insert - Primary - mem",
            "value": 123.40625,
            "unit": "median mem",
            "extra": "avg mem: 112.02581593654217, max mem: 138.5546875, count: 56443"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.669261,
            "unit": "median cpu",
            "extra": "avg cpu: 4.841313802272757, max cpu: 9.657948, count: 56443"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 165.59765625,
            "unit": "median mem",
            "extra": "avg mem: 161.67931675816754, max mem: 165.8125, count: 56443"
          },
          {
            "name": "Top N - Primary - cpu",
            "value": 23.391813,
            "unit": "median cpu",
            "extra": "avg cpu: 23.817395711231907, max cpu: 33.432835, count: 56443"
          },
          {
            "name": "Top N - Primary - mem",
            "value": 160.22265625,
            "unit": "median mem",
            "extra": "avg mem: 177.7709504776943, max mem: 218.625, count: 56443"
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
        "date": 1778266268264,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Background Merger - Primary - background_merging",
            "value": 0,
            "unit": "median background_merging",
            "extra": "avg background_merging: 0.059226566532006905, max background_merging: 2.0, count: 56191"
          },
          {
            "name": "Background Merger - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 4.725138828859841, max cpu: 9.67742, count: 56191"
          },
          {
            "name": "Background Merger - Primary - mem",
            "value": 27.30078125,
            "unit": "median mem",
            "extra": "avg mem: 27.28764615329857, max mem: 27.3046875, count: 56191"
          },
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 4.669261,
            "unit": "median cpu",
            "extra": "avg cpu: 4.9486920579636955, max cpu: 27.906979, count: 56191"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 187.78515625,
            "unit": "median mem",
            "extra": "avg mem: 184.43455461895587, max mem: 188.1015625, count: 56191"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 62588,
            "unit": "median block_count",
            "extra": "avg block_count: 62332.43845099749, max block_count: 62588.0, count: 56191"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 45,
            "unit": "median segment_count",
            "extra": "avg segment_count: 41.90829492267445, max segment_count: 57.0, count: 56191"
          },
          {
            "name": "Single Insert - Primary - cpu",
            "value": 4.669261,
            "unit": "median cpu",
            "extra": "avg cpu: 5.105442792325327, max cpu: 32.621357, count: 56191"
          },
          {
            "name": "Single Insert - Primary - mem",
            "value": 221.22265625,
            "unit": "median mem",
            "extra": "avg mem: 219.44726607686283, max mem: 222.265625, count: 56191"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 4.834019465286621, max cpu: 28.514853, count: 56191"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 187.69140625,
            "unit": "median mem",
            "extra": "avg mem: 182.30792318554128, max mem: 187.77734375, count: 56191"
          },
          {
            "name": "Top K - Primary - cpu",
            "value": 23.391813,
            "unit": "median cpu",
            "extra": "avg cpu: 23.920158787907425, max cpu: 33.20158, count: 56191"
          },
          {
            "name": "Top K - Primary - mem",
            "value": 162.97265625,
            "unit": "median mem",
            "extra": "avg mem: 180.4371335044758, max mem: 221.2890625, count: 56191"
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
        "date": 1778266494076,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Background Merger - Primary - background_merging",
            "value": 0,
            "unit": "median background_merging",
            "extra": "avg background_merging: 0.05443637848103512, max background_merging: 2.0, count: 56341"
          },
          {
            "name": "Background Merger - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.8095906463401015, max cpu: 9.552238, count: 56341"
          },
          {
            "name": "Background Merger - Primary - mem",
            "value": 25.0859375,
            "unit": "median mem",
            "extra": "avg mem: 25.13258529811771, max mem: 25.20703125, count: 56341"
          },
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 5.007905945727295, max cpu: 28.543112, count: 56341"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 189.51953125,
            "unit": "median mem",
            "extra": "avg mem: 183.6381027609778, max mem: 189.60546875, count: 56341"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 53422,
            "unit": "median block_count",
            "extra": "avg block_count: 53288.11320352851, max block_count: 53422.0, count: 56341"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 45,
            "unit": "median segment_count",
            "extra": "avg segment_count: 41.9457233630926, max segment_count: 56.0, count: 56341"
          },
          {
            "name": "Single Insert - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.623210188765821, max cpu: 9.514371, count: 56341"
          },
          {
            "name": "Single Insert - Primary - mem",
            "value": 126.95703125,
            "unit": "median mem",
            "extra": "avg mem: 114.93585252635737, max mem: 139.29296875, count: 56341"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 5.204977033691848, max cpu: 28.152493, count: 56341"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 197.24609375,
            "unit": "median mem",
            "extra": "avg mem: 195.43410264893683, max mem: 231.18359375, count: 56341"
          },
          {
            "name": "Top K - Primary - cpu",
            "value": 23.369036,
            "unit": "median cpu",
            "extra": "avg cpu: 23.943496535923423, max cpu: 33.83686, count: 56341"
          },
          {
            "name": "Top K - Primary - mem",
            "value": 163.3359375,
            "unit": "median mem",
            "extra": "avg mem: 182.01405725847962, max mem: 221.76953125, count: 56341"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "JLockerman",
            "username": "JLockerman",
            "email": "lockerman@paradedb.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "9a827ea3a0452e482e852785e743deb2f4630222",
          "message": "fix: Issues with text casts and memory layout for tokenizer types (#4900)\n\n# Ticket(s) Closed\n\n- fixes https://github.com/paradedb/paradedb/issues/5033\n\n## What\n\nChanges the tokenizer and alias types to function as regular SQL types\n(writable to disk, reallocatable in memory contexts etc).\n\n## Why\n\nWhen used incorrectly (eg. within a non-optimized function call) the\nprevious versions would access freed memory.\n\n## How\n\nThe tokenizer format is changed from `(header, magic_num, Oid, padding\nDatum)` to `(header, magic_num, metadata, padding, Oid, data_bytes)`\nwhere the `data_bytes` are the bytes from the original value (the\n`Datum` for by-value types, and the bytes pointed-at by the `Datum` for\nby-ref types). This lets us create a new `Datum` for that type (pointing\nat the inner bytes if needed).\n\nNOTE: Since the old version of the type was storing `Datum`s directly,\nany values stored to disk with the old code is broken unless they were\nin text format (the others store dangling pointers). In the updated\nversion such values will be output as meaningless text instead.\n\n## Tests\n\n- in\n`pg_search/tests/pg_regress/sql/tokenizer-types-inline-tokenization.sql`\n\n---------\n\nCo-authored-by: Mithun Chicklore Yogendra <mithun.cy@gmail.com>",
          "timestamp": "2026-05-08T18:32:32Z",
          "url": "https://github.com/paradedb/paradedb/commit/9a827ea3a0452e482e852785e743deb2f4630222"
        },
        "date": 1778266673929,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Background Merger - Primary - background_merging",
            "value": 0,
            "unit": "median background_merging",
            "extra": "avg background_merging: 0.07304062371768567, max background_merging: 2.0, count: 56051"
          },
          {
            "name": "Background Merger - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 4.734929566446835, max cpu: 9.523809, count: 56051"
          },
          {
            "name": "Background Merger - Primary - mem",
            "value": 26.015625,
            "unit": "median mem",
            "extra": "avg mem: 26.00912039091631, max mem: 26.015625, count: 56051"
          },
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 4.938646688234482, max cpu: 27.906979, count: 56051"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 186.71484375,
            "unit": "median mem",
            "extra": "avg mem: 178.79204388079606, max mem: 188.39453125, count: 56051"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 51555,
            "unit": "median block_count",
            "extra": "avg block_count: 51428.05207757221, max block_count: 51555.0, count: 56051"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 45,
            "unit": "median segment_count",
            "extra": "avg segment_count: 43.06569017501918, max segment_count: 56.0, count: 56051"
          },
          {
            "name": "Single Insert - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.678604359714064, max cpu: 9.448819, count: 56051"
          },
          {
            "name": "Single Insert - Primary - mem",
            "value": 128.4609375,
            "unit": "median mem",
            "extra": "avg mem: 116.88266001610141, max mem: 143.28125, count: 56051"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.694043537817269, max cpu: 23.166023, count: 56051"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 184.00390625,
            "unit": "median mem",
            "extra": "avg mem: 174.41203504955308, max mem: 185.7734375, count: 56051"
          },
          {
            "name": "Top K - Primary - cpu",
            "value": 23.369036,
            "unit": "median cpu",
            "extra": "avg cpu: 23.89121007097954, max cpu: 33.870968, count: 56051"
          },
          {
            "name": "Top K - Primary - mem",
            "value": 164.4375,
            "unit": "median mem",
            "extra": "avg mem: 182.7609780044067, max mem: 222.77734375, count: 56051"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "lockerman@paradedb.com",
            "name": "JLockerman",
            "username": "JLockerman"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "9a827ea3a0452e482e852785e743deb2f4630222",
          "message": "fix: Issues with text casts and memory layout for tokenizer types (#4900)\n\n# Ticket(s) Closed\n\n- fixes https://github.com/paradedb/paradedb/issues/5033\n\n## What\n\nChanges the tokenizer and alias types to function as regular SQL types\n(writable to disk, reallocatable in memory contexts etc).\n\n## Why\n\nWhen used incorrectly (eg. within a non-optimized function call) the\nprevious versions would access freed memory.\n\n## How\n\nThe tokenizer format is changed from `(header, magic_num, Oid, padding\nDatum)` to `(header, magic_num, metadata, padding, Oid, data_bytes)`\nwhere the `data_bytes` are the bytes from the original value (the\n`Datum` for by-value types, and the bytes pointed-at by the `Datum` for\nby-ref types). This lets us create a new `Datum` for that type (pointing\nat the inner bytes if needed).\n\nNOTE: Since the old version of the type was storing `Datum`s directly,\nany values stored to disk with the old code is broken unless they were\nin text format (the others store dangling pointers). In the updated\nversion such values will be output as meaningless text instead.\n\n## Tests\n\n- in\n`pg_search/tests/pg_regress/sql/tokenizer-types-inline-tokenization.sql`\n\n---------\n\nCo-authored-by: Mithun Chicklore Yogendra <mithun.cy@gmail.com>",
          "timestamp": "2026-05-08T14:32:32-04:00",
          "tree_id": "9cf77ffd18186494bb164cc15f9f703749357d03",
          "url": "https://github.com/paradedb/paradedb/commit/9a827ea3a0452e482e852785e743deb2f4630222"
        },
        "date": 1778268391672,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Background Merger - Primary - background_merging",
            "value": 0,
            "unit": "median background_merging",
            "extra": "avg background_merging: 0.07276636576871136, max background_merging: 2.0, count: 56276"
          },
          {
            "name": "Background Merger - Primary - cpu",
            "value": 4.669261,
            "unit": "median cpu",
            "extra": "avg cpu: 4.860341993882093, max cpu: 9.667674, count: 56276"
          },
          {
            "name": "Background Merger - Primary - mem",
            "value": 28.8125,
            "unit": "median mem",
            "extra": "avg mem: 28.864540667869075, max mem: 28.93359375, count: 56276"
          },
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 4.669261,
            "unit": "median cpu",
            "extra": "avg cpu: 5.027205657596049, max cpu: 23.66864, count: 56276"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 184.171875,
            "unit": "median mem",
            "extra": "avg mem: 182.45338140925972, max mem: 184.34765625, count: 56276"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 51431,
            "unit": "median block_count",
            "extra": "avg block_count: 51295.68624280333, max block_count: 51431.0, count: 56276"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 45,
            "unit": "median segment_count",
            "extra": "avg segment_count: 42.47519368825076, max segment_count: 56.0, count: 56276"
          },
          {
            "name": "Single Insert - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.7195914093543445, max cpu: 27.934044, count: 56276"
          },
          {
            "name": "Single Insert - Primary - mem",
            "value": 160.15234375,
            "unit": "median mem",
            "extra": "avg mem: 141.2139116808231, max mem: 169.65625, count: 56276"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 4.786772623304549, max cpu: 27.906979, count: 56276"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 186.125,
            "unit": "median mem",
            "extra": "avg mem: 181.9123938962657, max mem: 190.18359375, count: 56276"
          },
          {
            "name": "Top K - Primary - cpu",
            "value": 23.506365,
            "unit": "median cpu",
            "extra": "avg cpu: 23.879563884989647, max cpu: 33.20158, count: 56276"
          },
          {
            "name": "Top K - Primary - mem",
            "value": 163.6640625,
            "unit": "median mem",
            "extra": "avg mem: 182.22712876603703, max mem: 222.140625, count: 56276"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "rjhallsted@gmail.com",
            "name": "RJ Barman",
            "username": "barbarj"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "c5464b3e24db03ac3cb8bab3f1a344c417055c5b",
          "message": "chore: Refactor is_array branch in row_to_search_document (#5045)\n\n## What\nRefactor this block to separate the array conversion and then adding the\nelements of that array to the document into distinct steps.\n\n## Why\nIt's cleaner",
          "timestamp": "2026-05-08T15:16:13-06:00",
          "tree_id": "f8f52e91215e0a49ed69cefc3d821224ee7ddf3e",
          "url": "https://github.com/paradedb/paradedb/commit/c5464b3e24db03ac3cb8bab3f1a344c417055c5b"
        },
        "date": 1778278206777,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Background Merger - Primary - background_merging",
            "value": 0,
            "unit": "median background_merging",
            "extra": "avg background_merging: 0.05312873157603949, max background_merging: 2.0, count: 56109"
          },
          {
            "name": "Background Merger - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 4.785329918111452, max cpu: 9.657948, count: 56109"
          },
          {
            "name": "Background Merger - Primary - mem",
            "value": 25.6015625,
            "unit": "median mem",
            "extra": "avg mem: 25.653496361434886, max mem: 25.72265625, count: 56109"
          },
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 4.669261,
            "unit": "median cpu",
            "extra": "avg cpu: 5.013624770001632, max cpu: 23.645319, count: 56109"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 185.86328125,
            "unit": "median mem",
            "extra": "avg mem: 177.49304959153167, max mem: 187.33984375, count: 56109"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 57954,
            "unit": "median block_count",
            "extra": "avg block_count: 57742.43265786237, max block_count: 57954.0, count: 56109"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 45,
            "unit": "median segment_count",
            "extra": "avg segment_count: 42.04988504517992, max segment_count: 59.0, count: 56109"
          },
          {
            "name": "Single Insert - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 4.763540367658041, max cpu: 28.543112, count: 56109"
          },
          {
            "name": "Single Insert - Primary - mem",
            "value": 150.75,
            "unit": "median mem",
            "extra": "avg mem: 135.44787997747687, max mem: 164.47265625, count: 56109"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 5.138905801454459, max cpu: 32.40116, count: 56109"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 197.5078125,
            "unit": "median mem",
            "extra": "avg mem: 195.93620091028177, max mem: 224.1015625, count: 56109"
          },
          {
            "name": "Top K - Primary - cpu",
            "value": 23.391813,
            "unit": "median cpu",
            "extra": "avg cpu: 24.031368747413346, max cpu: 33.870968, count: 56109"
          },
          {
            "name": "Top K - Primary - mem",
            "value": 163.44140625,
            "unit": "median mem",
            "extra": "avg mem: 181.4879536928122, max mem: 221.9921875, count: 56109"
          }
        ]
      }
    ],
    "pg_search background-merge.toml Performance - TPS": [
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
        "date": 1778266730994,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 30.161878616708442,
            "unit": "median tps",
            "extra": "avg tps: 29.831884815930408, max tps: 34.14423983270979, count: 55496"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 245.3121658574443,
            "unit": "median tps",
            "extra": "avg tps: 269.51594951027624, max tps: 2749.9441417596204, count: 55496"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 2112.592031408757,
            "unit": "median tps",
            "extra": "avg tps: 2101.9808218298695, max tps: 2508.809548710502, count: 55496"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 163.26761909357828,
            "unit": "median tps",
            "extra": "avg tps: 201.33504410488703, max tps: 1761.3704166389618, count: 110992"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 13.430466967713933,
            "unit": "median tps",
            "extra": "avg tps: 13.147687635753782, max tps: 20.702362425245322, count: 55496"
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
        "date": 1778266918809,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 30.742425328080277,
            "unit": "median tps",
            "extra": "avg tps: 30.576156903141204, max tps: 34.19935051927931, count: 55612"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 244.83719218263354,
            "unit": "median tps",
            "extra": "avg tps: 272.3602934223051, max tps: 2835.958472429875, count: 55612"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 616.5547181246752,
            "unit": "median tps",
            "extra": "avg tps: 602.403775403539, max tps: 1394.9159561551285, count: 55612"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 165.92435974278087,
            "unit": "median tps",
            "extra": "avg tps: 179.5275138601217, max tps: 1120.1204164819942, count: 111224"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 15.714348394065299,
            "unit": "median tps",
            "extra": "avg tps: 15.585327624741359, max tps: 19.45550765674422, count: 55612"
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
        "date": 1778267144551,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 30.99153715758633,
            "unit": "median tps",
            "extra": "avg tps: 30.706289396116198, max tps: 32.935812296210386, count: 55513"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 248.69792028784912,
            "unit": "median tps",
            "extra": "avg tps: 281.6435877153544, max tps: 3232.457469368987, count: 55513"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 644.2505523558966,
            "unit": "median tps",
            "extra": "avg tps: 626.9993097670035, max tps: 830.4538567107559, count: 55513"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 168.01462882512897,
            "unit": "median tps",
            "extra": "avg tps: 182.57307614065752, max tps: 1107.1414204950615, count: 111026"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 16.593463773307892,
            "unit": "median tps",
            "extra": "avg tps: 16.54375242729632, max tps: 21.60321708195755, count: 55513"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "JLockerman",
            "username": "JLockerman",
            "email": "lockerman@paradedb.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "9a827ea3a0452e482e852785e743deb2f4630222",
          "message": "fix: Issues with text casts and memory layout for tokenizer types (#4900)\n\n# Ticket(s) Closed\n\n- fixes https://github.com/paradedb/paradedb/issues/5033\n\n## What\n\nChanges the tokenizer and alias types to function as regular SQL types\n(writable to disk, reallocatable in memory contexts etc).\n\n## Why\n\nWhen used incorrectly (eg. within a non-optimized function call) the\nprevious versions would access freed memory.\n\n## How\n\nThe tokenizer format is changed from `(header, magic_num, Oid, padding\nDatum)` to `(header, magic_num, metadata, padding, Oid, data_bytes)`\nwhere the `data_bytes` are the bytes from the original value (the\n`Datum` for by-value types, and the bytes pointed-at by the `Datum` for\nby-ref types). This lets us create a new `Datum` for that type (pointing\nat the inner bytes if needed).\n\nNOTE: Since the old version of the type was storing `Datum`s directly,\nany values stored to disk with the old code is broken unless they were\nin text format (the others store dangling pointers). In the updated\nversion such values will be output as meaningless text instead.\n\n## Tests\n\n- in\n`pg_search/tests/pg_regress/sql/tokenizer-types-inline-tokenization.sql`\n\n---------\n\nCo-authored-by: Mithun Chicklore Yogendra <mithun.cy@gmail.com>",
          "timestamp": "2026-05-08T18:32:32Z",
          "url": "https://github.com/paradedb/paradedb/commit/9a827ea3a0452e482e852785e743deb2f4630222"
        },
        "date": 1778267328719,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 31.472506278673567,
            "unit": "median tps",
            "extra": "avg tps: 31.357784265565275, max tps: 32.601604185921886, count: 55584"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 245.2253620020487,
            "unit": "median tps",
            "extra": "avg tps: 276.22564851768374, max tps: 3123.4270929791232, count: 55584"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 653.6185768210875,
            "unit": "median tps",
            "extra": "avg tps: 633.6327662369396, max tps: 1264.5936877869306, count: 55584"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 162.46301769269286,
            "unit": "median tps",
            "extra": "avg tps: 181.48354473514928, max tps: 1118.7419600065953, count: 111168"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 16.061410762201195,
            "unit": "median tps",
            "extra": "avg tps: 16.10753326047288, max tps: 21.65379002389106, count: 55584"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "lockerman@paradedb.com",
            "name": "JLockerman",
            "username": "JLockerman"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "9a827ea3a0452e482e852785e743deb2f4630222",
          "message": "fix: Issues with text casts and memory layout for tokenizer types (#4900)\n\n# Ticket(s) Closed\n\n- fixes https://github.com/paradedb/paradedb/issues/5033\n\n## What\n\nChanges the tokenizer and alias types to function as regular SQL types\n(writable to disk, reallocatable in memory contexts etc).\n\n## Why\n\nWhen used incorrectly (eg. within a non-optimized function call) the\nprevious versions would access freed memory.\n\n## How\n\nThe tokenizer format is changed from `(header, magic_num, Oid, padding\nDatum)` to `(header, magic_num, metadata, padding, Oid, data_bytes)`\nwhere the `data_bytes` are the bytes from the original value (the\n`Datum` for by-value types, and the bytes pointed-at by the `Datum` for\nby-ref types). This lets us create a new `Datum` for that type (pointing\nat the inner bytes if needed).\n\nNOTE: Since the old version of the type was storing `Datum`s directly,\nany values stored to disk with the old code is broken unless they were\nin text format (the others store dangling pointers). In the updated\nversion such values will be output as meaningless text instead.\n\n## Tests\n\n- in\n`pg_search/tests/pg_regress/sql/tokenizer-types-inline-tokenization.sql`\n\n---------\n\nCo-authored-by: Mithun Chicklore Yogendra <mithun.cy@gmail.com>",
          "timestamp": "2026-05-08T14:32:32-04:00",
          "tree_id": "9cf77ffd18186494bb164cc15f9f703749357d03",
          "url": "https://github.com/paradedb/paradedb/commit/9a827ea3a0452e482e852785e743deb2f4630222"
        },
        "date": 1778269042674,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 30.403193210343296,
            "unit": "median tps",
            "extra": "avg tps: 30.144488407809437, max tps: 32.41134945483628, count: 55504"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 242.66325193776214,
            "unit": "median tps",
            "extra": "avg tps: 270.9234271543644, max tps: 2959.0588706965877, count: 55504"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 629.5296110689194,
            "unit": "median tps",
            "extra": "avg tps: 617.3717751346164, max tps: 982.6791925045301, count: 55504"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 165.03378242086194,
            "unit": "median tps",
            "extra": "avg tps: 181.50554824231364, max tps: 1483.2172973987797, count: 111008"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 17.079864218279255,
            "unit": "median tps",
            "extra": "avg tps: 16.99631405208253, max tps: 20.037490144059536, count: 55504"
          }
        ]
      }
    ],
    "pg_search background-merge.toml Performance - Other Metrics": [
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
        "date": 1778266773334,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.58664,
            "unit": "median cpu",
            "extra": "avg cpu: 19.894724417372704, max cpu: 42.561577, count: 55496"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 161.24609375,
            "unit": "median mem",
            "extra": "avg mem: 148.21956488136533, max mem: 173.703125, count: 55496"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 7.695167383066763, max cpu: 28.015566, count: 55496"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 119.375,
            "unit": "median mem",
            "extra": "avg mem: 118.00174034220935, max mem: 119.4375, count: 55496"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.755556035975289, max cpu: 9.320388, count: 55496"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 109.8046875,
            "unit": "median mem",
            "extra": "avg mem: 107.03429216745351, max mem: 156.39453125, count: 55496"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 13052,
            "unit": "median block_count",
            "extra": "avg block_count: 13074.812202681274, max block_count: 22681.0, count: 55496"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.266793033002365, max cpu: 4.7151275, count: 55496"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 86.8515625,
            "unit": "median mem",
            "extra": "avg mem: 84.95593249400858, max mem: 130.86328125, count: 55496"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 24,
            "unit": "median segment_count",
            "extra": "avg segment_count: 24.471223151218105, max segment_count: 39.0, count: 55496"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.230769,
            "unit": "median cpu",
            "extra": "avg cpu: 8.836353610901083, max cpu: 28.042841, count: 110992"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 149.94921875,
            "unit": "median mem",
            "extra": "avg mem: 134.502770153198, max mem: 161.7265625, count: 110992"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.859479,
            "unit": "median cpu",
            "extra": "avg cpu: 12.561721557675387, max cpu: 27.853, count: 55496"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 168.48046875,
            "unit": "median mem",
            "extra": "avg mem: 165.58013950329754, max mem: 170.29296875, count: 55496"
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
        "date": 1778267036187,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.60465,
            "unit": "median cpu",
            "extra": "avg cpu: 19.92496345237475, max cpu: 46.242775, count: 55612"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 170.59765625,
            "unit": "median mem",
            "extra": "avg mem: 153.19251184546502, max mem: 177.07421875, count: 55612"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 7.669234721509328, max cpu: 28.290766, count: 55612"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 120.5390625,
            "unit": "median mem",
            "extra": "avg mem: 119.31747637369183, max mem: 120.703125, count: 55612"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 6.1726741972495685, max cpu: 18.731707, count: 55612"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 163.53515625,
            "unit": "median mem",
            "extra": "avg mem: 142.07444984333418, max mem: 177.0625, count: 55612"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 16565,
            "unit": "median block_count",
            "extra": "avg block_count: 16844.5784902539, max block_count: 31515.0, count: 55612"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.432406449865396, max cpu: 4.673807, count: 55612"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 106.75390625,
            "unit": "median mem",
            "extra": "avg mem: 95.92363095813404, max mem: 138.14453125, count: 55612"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 26,
            "unit": "median segment_count",
            "extra": "avg segment_count: 25.74169244048047, max segment_count: 41.0, count: 55612"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.266409,
            "unit": "median cpu",
            "extra": "avg cpu: 9.259262109841389, max cpu: 28.486649, count: 111224"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 176.3984375,
            "unit": "median mem",
            "extra": "avg mem: 160.10152709846795, max mem: 181.24609375, count: 111224"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.88621,
            "unit": "median cpu",
            "extra": "avg cpu: 12.73508389350431, max cpu: 28.486649, count: 55612"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 170.890625,
            "unit": "median mem",
            "extra": "avg mem: 168.53227481815975, max mem: 172.015625, count: 55612"
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
        "date": 1778267183299,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.622696,
            "unit": "median cpu",
            "extra": "avg cpu: 19.81404018234576, max cpu: 42.72997, count: 55513"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 174.6640625,
            "unit": "median mem",
            "extra": "avg mem: 164.08927406362022, max mem: 178.46484375, count: 55513"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 7.656577325841496, max cpu: 27.934044, count: 55513"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 120.40625,
            "unit": "median mem",
            "extra": "avg mem: 119.22846872579396, max mem: 120.51953125, count: 55513"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 6.171069876495928, max cpu: 19.047619, count: 55513"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 163.671875,
            "unit": "median mem",
            "extra": "avg mem: 143.54280261492804, max mem: 178.57421875, count: 55513"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 16602,
            "unit": "median block_count",
            "extra": "avg block_count: 16876.180264082286, max block_count: 31248.0, count: 55513"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.448441144161088, max cpu: 4.673807, count: 55513"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 104.20703125,
            "unit": "median mem",
            "extra": "avg mem: 94.78674765651739, max mem: 137.41015625, count: 55513"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 26,
            "unit": "median segment_count",
            "extra": "avg segment_count: 25.60151676183957, max segment_count: 36.0, count: 55513"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.266409,
            "unit": "median cpu",
            "extra": "avg cpu: 9.223023157325445, max cpu: 27.988338, count: 111026"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 178.0390625,
            "unit": "median mem",
            "extra": "avg mem: 161.0062310855115, max mem: 182.8828125, count: 111026"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.859479,
            "unit": "median cpu",
            "extra": "avg cpu: 11.958286128844422, max cpu: 27.77242, count: 55513"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 172.83203125,
            "unit": "median mem",
            "extra": "avg mem: 170.1710987178679, max mem: 173.47265625, count: 55513"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "JLockerman",
            "username": "JLockerman",
            "email": "lockerman@paradedb.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "9a827ea3a0452e482e852785e743deb2f4630222",
          "message": "fix: Issues with text casts and memory layout for tokenizer types (#4900)\n\n# Ticket(s) Closed\n\n- fixes https://github.com/paradedb/paradedb/issues/5033\n\n## What\n\nChanges the tokenizer and alias types to function as regular SQL types\n(writable to disk, reallocatable in memory contexts etc).\n\n## Why\n\nWhen used incorrectly (eg. within a non-optimized function call) the\nprevious versions would access freed memory.\n\n## How\n\nThe tokenizer format is changed from `(header, magic_num, Oid, padding\nDatum)` to `(header, magic_num, metadata, padding, Oid, data_bytes)`\nwhere the `data_bytes` are the bytes from the original value (the\n`Datum` for by-value types, and the bytes pointed-at by the `Datum` for\nby-ref types). This lets us create a new `Datum` for that type (pointing\nat the inner bytes if needed).\n\nNOTE: Since the old version of the type was storing `Datum`s directly,\nany values stored to disk with the old code is broken unless they were\nin text format (the others store dangling pointers). In the updated\nversion such values will be output as meaningless text instead.\n\n## Tests\n\n- in\n`pg_search/tests/pg_regress/sql/tokenizer-types-inline-tokenization.sql`\n\n---------\n\nCo-authored-by: Mithun Chicklore Yogendra <mithun.cy@gmail.com>",
          "timestamp": "2026-05-08T18:32:32Z",
          "url": "https://github.com/paradedb/paradedb/commit/9a827ea3a0452e482e852785e743deb2f4630222"
        },
        "date": 1778267373285,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.640776,
            "unit": "median cpu",
            "extra": "avg cpu: 19.66118773248524, max cpu: 42.772278, count: 55584"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 178.73046875,
            "unit": "median mem",
            "extra": "avg mem: 176.83196230873543, max mem: 179.12890625, count: 55584"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 7.6366811335858795, max cpu: 28.042841, count: 55584"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 120.71875,
            "unit": "median mem",
            "extra": "avg mem: 119.5736301841582, max mem: 120.81640625, count: 55584"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 6.144666321272975, max cpu: 18.550726, count: 55584"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 165.640625,
            "unit": "median mem",
            "extra": "avg mem: 145.35265797036917, max mem: 180.54296875, count: 55584"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 16926,
            "unit": "median block_count",
            "extra": "avg block_count: 16995.060035261948, max block_count: 31582.0, count: 55584"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 3.77285283860913, max cpu: 4.692082, count: 55584"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 106.21484375,
            "unit": "median mem",
            "extra": "avg mem: 95.40399524818294, max mem: 137.140625, count: 55584"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 25,
            "unit": "median segment_count",
            "extra": "avg segment_count: 25.47636010362694, max segment_count: 40.0, count: 55584"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.275363,
            "unit": "median cpu",
            "extra": "avg cpu: 9.17293064861752, max cpu: 28.09756, count: 111168"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 180.46484375,
            "unit": "median mem",
            "extra": "avg mem: 162.4621316737393, max mem: 183.08984375, count: 111168"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.913043,
            "unit": "median cpu",
            "extra": "avg cpu: 12.698609841438476, max cpu: 28.070175, count: 55584"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 173.87890625,
            "unit": "median mem",
            "extra": "avg mem: 171.28223520651176, max mem: 174.60546875, count: 55584"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "lockerman@paradedb.com",
            "name": "JLockerman",
            "username": "JLockerman"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "9a827ea3a0452e482e852785e743deb2f4630222",
          "message": "fix: Issues with text casts and memory layout for tokenizer types (#4900)\n\n# Ticket(s) Closed\n\n- fixes https://github.com/paradedb/paradedb/issues/5033\n\n## What\n\nChanges the tokenizer and alias types to function as regular SQL types\n(writable to disk, reallocatable in memory contexts etc).\n\n## Why\n\nWhen used incorrectly (eg. within a non-optimized function call) the\nprevious versions would access freed memory.\n\n## How\n\nThe tokenizer format is changed from `(header, magic_num, Oid, padding\nDatum)` to `(header, magic_num, metadata, padding, Oid, data_bytes)`\nwhere the `data_bytes` are the bytes from the original value (the\n`Datum` for by-value types, and the bytes pointed-at by the `Datum` for\nby-ref types). This lets us create a new `Datum` for that type (pointing\nat the inner bytes if needed).\n\nNOTE: Since the old version of the type was storing `Datum`s directly,\nany values stored to disk with the old code is broken unless they were\nin text format (the others store dangling pointers). In the updated\nversion such values will be output as meaningless text instead.\n\n## Tests\n\n- in\n`pg_search/tests/pg_regress/sql/tokenizer-types-inline-tokenization.sql`\n\n---------\n\nCo-authored-by: Mithun Chicklore Yogendra <mithun.cy@gmail.com>",
          "timestamp": "2026-05-08T14:32:32-04:00",
          "tree_id": "9cf77ffd18186494bb164cc15f9f703749357d03",
          "url": "https://github.com/paradedb/paradedb/commit/9a827ea3a0452e482e852785e743deb2f4630222"
        },
        "date": 1778269081444,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.622696,
            "unit": "median cpu",
            "extra": "avg cpu: 20.030638984775354, max cpu: 55.813957, count: 55504"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 178.56640625,
            "unit": "median mem",
            "extra": "avg mem: 176.41448082570625, max mem: 178.95703125, count: 55504"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 7.644825336898069, max cpu: 30.04695, count: 55504"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 120.140625,
            "unit": "median mem",
            "extra": "avg mem: 118.91140967880693, max mem: 120.30859375, count: 55504"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 6.200529616276641, max cpu: 18.640776, count: 55504"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 174.32421875,
            "unit": "median mem",
            "extra": "avg mem: 145.9664473787925, max mem: 177.91796875, count: 55504"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 16592,
            "unit": "median block_count",
            "extra": "avg block_count: 17003.803383539926, max block_count: 31691.0, count: 55504"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.619827,
            "unit": "median cpu",
            "extra": "avg cpu: 4.428563894756317, max cpu: 4.678363, count: 55504"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 113.48828125,
            "unit": "median mem",
            "extra": "avg mem: 98.41345155022611, max mem: 138.98828125, count: 55504"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 25,
            "unit": "median segment_count",
            "extra": "avg segment_count: 24.568625684635343, max segment_count: 38.0, count: 55504"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.257474,
            "unit": "median cpu",
            "extra": "avg cpu: 9.141051741062032, max cpu: 32.74854, count: 111008"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 179.359375,
            "unit": "median mem",
            "extra": "avg mem: 163.27744301790187, max mem: 183.5390625, count: 111008"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.846154,
            "unit": "median cpu",
            "extra": "avg cpu: 11.999422550054245, max cpu: 27.906979, count: 55504"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 173.4296875,
            "unit": "median mem",
            "extra": "avg mem: 170.7500297698139, max mem: 174.32421875, count: 55504"
          }
        ]
      }
    ],
    "pg_search logical-replication.toml Performance - TPS": [
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
        "date": 1778267433551,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - tps",
            "value": 504.1809064695106,
            "unit": "median tps",
            "extra": "avg tps: 506.898100078917, max tps: 671.3899126055779, count: 53850"
          },
          {
            "name": "Index Only Scan - Subscriber - tps",
            "value": 574.4695251974631,
            "unit": "median tps",
            "extra": "avg tps: 577.1129999871116, max tps: 771.2777185013248, count: 53850"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - tps",
            "value": 93.88527356304053,
            "unit": "median tps",
            "extra": "avg tps: 94.0121679616389, max tps: 102.02090008715777, count: 53850"
          },
          {
            "name": "Top N - Subscriber - tps",
            "value": 125.25697899320559,
            "unit": "median tps",
            "extra": "avg tps: 121.11099050529903, max tps: 172.95204549753169, count: 107700"
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
        "date": 1778267691532,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - tps",
            "value": 532.5995098294185,
            "unit": "median tps",
            "extra": "avg tps: 532.044307921473, max tps: 730.2105829074591, count: 53899"
          },
          {
            "name": "Index Only Scan - Subscriber - tps",
            "value": 557.5968672042729,
            "unit": "median tps",
            "extra": "avg tps: 559.3480822826602, max tps: 772.2842027490074, count: 53899"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - tps",
            "value": 93.16660544784042,
            "unit": "median tps",
            "extra": "avg tps: 93.22416891212035, max tps: 102.95434105040066, count: 53899"
          },
          {
            "name": "Top K - Subscriber - tps",
            "value": 263.0673774278131,
            "unit": "median tps",
            "extra": "avg tps: 257.43887011614123, max tps: 489.2179948196407, count: 107798"
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
        "date": 1778267837184,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - tps",
            "value": 600.2785823327359,
            "unit": "median tps",
            "extra": "avg tps: 600.2577682119804, max tps: 817.774507694041, count: 53855"
          },
          {
            "name": "Index Only Scan - Subscriber - tps",
            "value": 618.5323730158777,
            "unit": "median tps",
            "extra": "avg tps: 619.4136298819946, max tps: 794.6144370835115, count: 53855"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - tps",
            "value": 91.28777430621703,
            "unit": "median tps",
            "extra": "avg tps: 91.35732236814067, max tps: 101.62190782087376, count: 53855"
          },
          {
            "name": "Top K - Subscriber - tps",
            "value": 276.1883058818154,
            "unit": "median tps",
            "extra": "avg tps: 269.7935995396992, max tps: 637.222815471606, count: 107710"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "JLockerman",
            "username": "JLockerman",
            "email": "lockerman@paradedb.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "9a827ea3a0452e482e852785e743deb2f4630222",
          "message": "fix: Issues with text casts and memory layout for tokenizer types (#4900)\n\n# Ticket(s) Closed\n\n- fixes https://github.com/paradedb/paradedb/issues/5033\n\n## What\n\nChanges the tokenizer and alias types to function as regular SQL types\n(writable to disk, reallocatable in memory contexts etc).\n\n## Why\n\nWhen used incorrectly (eg. within a non-optimized function call) the\nprevious versions would access freed memory.\n\n## How\n\nThe tokenizer format is changed from `(header, magic_num, Oid, padding\nDatum)` to `(header, magic_num, metadata, padding, Oid, data_bytes)`\nwhere the `data_bytes` are the bytes from the original value (the\n`Datum` for by-value types, and the bytes pointed-at by the `Datum` for\nby-ref types). This lets us create a new `Datum` for that type (pointing\nat the inner bytes if needed).\n\nNOTE: Since the old version of the type was storing `Datum`s directly,\nany values stored to disk with the old code is broken unless they were\nin text format (the others store dangling pointers). In the updated\nversion such values will be output as meaningless text instead.\n\n## Tests\n\n- in\n`pg_search/tests/pg_regress/sql/tokenizer-types-inline-tokenization.sql`\n\n---------\n\nCo-authored-by: Mithun Chicklore Yogendra <mithun.cy@gmail.com>",
          "timestamp": "2026-05-08T18:32:32Z",
          "url": "https://github.com/paradedb/paradedb/commit/9a827ea3a0452e482e852785e743deb2f4630222"
        },
        "date": 1778268027683,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - tps",
            "value": 631.3016942651035,
            "unit": "median tps",
            "extra": "avg tps: 632.663391658997, max tps: 785.7107925911807, count: 53881"
          },
          {
            "name": "Index Only Scan - Subscriber - tps",
            "value": 648.6193957756346,
            "unit": "median tps",
            "extra": "avg tps: 649.6393374804204, max tps: 789.54181226441, count: 53881"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - tps",
            "value": 90.72130525306814,
            "unit": "median tps",
            "extra": "avg tps: 90.80213023771024, max tps: 103.12208853100772, count: 53881"
          },
          {
            "name": "Top K - Subscriber - tps",
            "value": 287.8866499380978,
            "unit": "median tps",
            "extra": "avg tps: 279.3337096017979, max tps: 548.6173647837381, count: 107762"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "lockerman@paradedb.com",
            "name": "JLockerman",
            "username": "JLockerman"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "9a827ea3a0452e482e852785e743deb2f4630222",
          "message": "fix: Issues with text casts and memory layout for tokenizer types (#4900)\n\n# Ticket(s) Closed\n\n- fixes https://github.com/paradedb/paradedb/issues/5033\n\n## What\n\nChanges the tokenizer and alias types to function as regular SQL types\n(writable to disk, reallocatable in memory contexts etc).\n\n## Why\n\nWhen used incorrectly (eg. within a non-optimized function call) the\nprevious versions would access freed memory.\n\n## How\n\nThe tokenizer format is changed from `(header, magic_num, Oid, padding\nDatum)` to `(header, magic_num, metadata, padding, Oid, data_bytes)`\nwhere the `data_bytes` are the bytes from the original value (the\n`Datum` for by-value types, and the bytes pointed-at by the `Datum` for\nby-ref types). This lets us create a new `Datum` for that type (pointing\nat the inner bytes if needed).\n\nNOTE: Since the old version of the type was storing `Datum`s directly,\nany values stored to disk with the old code is broken unless they were\nin text format (the others store dangling pointers). In the updated\nversion such values will be output as meaningless text instead.\n\n## Tests\n\n- in\n`pg_search/tests/pg_regress/sql/tokenizer-types-inline-tokenization.sql`\n\n---------\n\nCo-authored-by: Mithun Chicklore Yogendra <mithun.cy@gmail.com>",
          "timestamp": "2026-05-08T14:32:32-04:00",
          "tree_id": "9cf77ffd18186494bb164cc15f9f703749357d03",
          "url": "https://github.com/paradedb/paradedb/commit/9a827ea3a0452e482e852785e743deb2f4630222"
        },
        "date": 1778269735887,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - tps",
            "value": 616.9267857439152,
            "unit": "median tps",
            "extra": "avg tps: 613.524103159754, max tps: 740.9751974573736, count: 53918"
          },
          {
            "name": "Index Only Scan - Subscriber - tps",
            "value": 623.0800788811425,
            "unit": "median tps",
            "extra": "avg tps: 620.7595716425601, max tps: 740.810621765489, count: 53918"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - tps",
            "value": 93.46649471672498,
            "unit": "median tps",
            "extra": "avg tps: 93.40916586514142, max tps: 100.39415325798393, count: 53918"
          },
          {
            "name": "Top K - Subscriber - tps",
            "value": 274.5903706197721,
            "unit": "median tps",
            "extra": "avg tps: 270.9752045552992, max tps: 613.4888101323179, count: 107836"
          }
        ]
      }
    ],
    "pg_search logical-replication.toml Performance - Other Metrics": [
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
        "date": 1778267477444,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - cpu",
            "value": 4.5801525,
            "unit": "median cpu",
            "extra": "avg cpu: 5.1581524143585105, max cpu: 13.779904, count: 53850"
          },
          {
            "name": "Custom Scan - Subscriber - mem",
            "value": 47.37109375,
            "unit": "median mem",
            "extra": "avg mem: 47.4463006325441, max mem: 53.265625, count: 53850"
          },
          {
            "name": "Delete values - Publisher - cpu",
            "value": 4.5845275,
            "unit": "median cpu",
            "extra": "avg cpu: 3.2922873625121145, max cpu: 4.58891, count: 53850"
          },
          {
            "name": "Delete values - Publisher - mem",
            "value": 29.8203125,
            "unit": "median mem",
            "extra": "avg mem: 29.136591443245127, max mem: 30.1875, count: 53850"
          },
          {
            "name": "Find by ctid - Subscriber - cpu",
            "value": 9.142857,
            "unit": "median cpu",
            "extra": "avg cpu: 8.944302066364566, max cpu: 18.497108, count: 53850"
          },
          {
            "name": "Find by ctid - Subscriber - mem",
            "value": 50.25390625,
            "unit": "median mem",
            "extra": "avg mem: 49.96329982010214, max mem: 56.0625, count: 53850"
          },
          {
            "name": "Index Only Scan - Subscriber - cpu",
            "value": 4.5801525,
            "unit": "median cpu",
            "extra": "avg cpu: 5.074826027636358, max cpu: 9.284333, count: 53850"
          },
          {
            "name": "Index Only Scan - Subscriber - mem",
            "value": 47.25,
            "unit": "median mem",
            "extra": "avg mem: 47.28630476729341, max mem: 53.08203125, count: 53850"
          },
          {
            "name": "Index Size Info - Subscriber - cpu",
            "value": 4.5757866,
            "unit": "median cpu",
            "extra": "avg cpu: 4.623605626448631, max cpu: 9.213051, count: 53850"
          },
          {
            "name": "Index Size Info - Subscriber - mem",
            "value": 31.4375,
            "unit": "median mem",
            "extra": "avg mem: 31.47637665389972, max mem: 36.65625, count: 53850"
          },
          {
            "name": "Index Size Info - Subscriber - pages",
            "value": 1106,
            "unit": "median pages",
            "extra": "avg pages: 1111.1661281337047, max pages: 1833.0, count: 53850"
          },
          {
            "name": "Index Size Info - Subscriber - relation_size:MB",
            "value": 8.640625,
            "unit": "median relation_size:MB",
            "extra": "avg relation_size:MB: 8.680985376044568, max relation_size:MB: 14.3203125, count: 53850"
          },
          {
            "name": "Index Size Info - Subscriber - segment_count",
            "value": 10,
            "unit": "median segment_count",
            "extra": "avg segment_count: 10.275134633240484, max segment_count: 19.0, count: 53850"
          },
          {
            "name": "Insert value A - Publisher - cpu",
            "value": 4.5845275,
            "unit": "median cpu",
            "extra": "avg cpu: 3.7818848788430235, max cpu: 4.597701, count: 53850"
          },
          {
            "name": "Insert value A - Publisher - mem",
            "value": 27.375,
            "unit": "median mem",
            "extra": "avg mem: 26.674321610956362, max mem: 27.7421875, count: 53850"
          },
          {
            "name": "Insert value B - Publisher - cpu",
            "value": 4.5757866,
            "unit": "median cpu",
            "extra": "avg cpu: 4.5250484578084365, max cpu: 4.624277, count: 53850"
          },
          {
            "name": "Insert value B - Publisher - mem",
            "value": 27.59765625,
            "unit": "median mem",
            "extra": "avg mem: 26.886313834726092, max mem: 27.97265625, count: 53850"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - cpu",
            "value": 4.5933013,
            "unit": "median cpu",
            "extra": "avg cpu: 6.328662051073543, max cpu: 13.913043, count: 53850"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - mem",
            "value": 45.2109375,
            "unit": "median mem",
            "extra": "avg mem: 45.27519237465181, max mem: 51.1796875, count: 53850"
          },
          {
            "name": "SELECT\n  pid,\n  pg_wal_lsn_diff(sent_lsn, replay_lsn) AS replication_lag,\n  application_name::text,\n  state::text\nFROM pg_stat_replication; - Publisher - replication_lag:MB",
            "value": 0,
            "unit": "median replication_lag:MB",
            "extra": "avg replication_lag:MB: 0.00004910312323185063, max replication_lag:MB: 0.31922149658203125, count: 53850"
          },
          {
            "name": "Top N - Subscriber - cpu",
            "value": 4.5801525,
            "unit": "median cpu",
            "extra": "avg cpu: 5.266086662606814, max cpu: 13.819577, count: 107700"
          },
          {
            "name": "Top N - Subscriber - mem",
            "value": 45.90625,
            "unit": "median mem",
            "extra": "avg mem: 45.96103546454271, max mem: 52.1484375, count: 107700"
          },
          {
            "name": "Update 1..9 - Publisher - cpu",
            "value": 4.567079,
            "unit": "median cpu",
            "extra": "avg cpu: 3.9362619142786173, max cpu: 4.624277, count: 53850"
          },
          {
            "name": "Update 1..9 - Publisher - mem",
            "value": 30.421875,
            "unit": "median mem",
            "extra": "avg mem: 29.736393555594244, max mem: 30.79296875, count: 53850"
          },
          {
            "name": "Update 10,11 - Publisher - cpu",
            "value": 4.5584044,
            "unit": "median cpu",
            "extra": "avg cpu: 4.374868510195521, max cpu: 4.619827, count: 53850"
          },
          {
            "name": "Update 10,11 - Publisher - mem",
            "value": 30.4765625,
            "unit": "median mem",
            "extra": "avg mem: 29.786451587163416, max mem: 30.55859375, count: 53850"
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
        "date": 1778267768835,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - cpu",
            "value": 4.5757866,
            "unit": "median cpu",
            "extra": "avg cpu: 5.105165673470167, max cpu: 9.275363, count: 53899"
          },
          {
            "name": "Custom Scan - Subscriber - mem",
            "value": 50.62890625,
            "unit": "median mem",
            "extra": "avg mem: 50.68023271825544, max mem: 56.8125, count: 53899"
          },
          {
            "name": "Delete values - Publisher - cpu",
            "value": 4.562738,
            "unit": "median cpu",
            "extra": "avg cpu: 4.465885564409471, max cpu: 4.597701, count: 53899"
          },
          {
            "name": "Delete values - Publisher - mem",
            "value": 31.5703125,
            "unit": "median mem",
            "extra": "avg mem: 30.86122988715004, max mem: 31.91796875, count: 53899"
          },
          {
            "name": "Find by ctid - Subscriber - cpu",
            "value": 9.134158,
            "unit": "median cpu",
            "extra": "avg cpu: 8.694317173910441, max cpu: 18.66252, count: 53899"
          },
          {
            "name": "Find by ctid - Subscriber - mem",
            "value": 54.22265625,
            "unit": "median mem",
            "extra": "avg mem: 53.91169986340192, max mem: 60.3125, count: 53899"
          },
          {
            "name": "Index Only Scan - Subscriber - cpu",
            "value": 4.5757866,
            "unit": "median cpu",
            "extra": "avg cpu: 5.08180045701158, max cpu: 9.275363, count: 53899"
          },
          {
            "name": "Index Only Scan - Subscriber - mem",
            "value": 50.41796875,
            "unit": "median mem",
            "extra": "avg mem: 50.47739282906455, max mem: 56.5703125, count: 53899"
          },
          {
            "name": "Index Size Info - Subscriber - cpu",
            "value": 4.567079,
            "unit": "median cpu",
            "extra": "avg cpu: 4.619731330118287, max cpu: 9.221902, count: 53899"
          },
          {
            "name": "Index Size Info - Subscriber - mem",
            "value": 33.3125,
            "unit": "median mem",
            "extra": "avg mem: 33.344346094663166, max mem: 38.75390625, count: 53899"
          },
          {
            "name": "Index Size Info - Subscriber - pages",
            "value": 1129,
            "unit": "median pages",
            "extra": "avg pages: 1130.4871147887716, max pages: 1891.0, count: 53899"
          },
          {
            "name": "Index Size Info - Subscriber - relation_size:MB",
            "value": 8.8203125,
            "unit": "median relation_size:MB",
            "extra": "avg relation_size:MB: 8.831930584287278, max relation_size:MB: 14.7734375, count: 53899"
          },
          {
            "name": "Index Size Info - Subscriber - segment_count",
            "value": 12,
            "unit": "median segment_count",
            "extra": "avg segment_count: 11.550529694428468, max segment_count: 20.0, count: 53899"
          },
          {
            "name": "Insert value A - Publisher - cpu",
            "value": 4.5714283,
            "unit": "median cpu",
            "extra": "avg cpu: 3.9164548555241825, max cpu: 4.597701, count: 53899"
          },
          {
            "name": "Insert value A - Publisher - mem",
            "value": 29.20703125,
            "unit": "median mem",
            "extra": "avg mem: 28.509746093387633, max mem: 29.5703125, count: 53899"
          },
          {
            "name": "Insert value B - Publisher - cpu",
            "value": 4.549763,
            "unit": "median cpu",
            "extra": "avg cpu: 4.11665764030429, max cpu: 4.5933013, count: 53899"
          },
          {
            "name": "Insert value B - Publisher - mem",
            "value": 29.203125,
            "unit": "median mem",
            "extra": "avg mem: 28.493456873620104, max mem: 29.53125, count: 53899"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - cpu",
            "value": 4.58891,
            "unit": "median cpu",
            "extra": "avg cpu: 6.348883224228248, max cpu: 22.835394, count: 53899"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - mem",
            "value": 48.5078125,
            "unit": "median mem",
            "extra": "avg mem: 48.554785194298596, max mem: 54.671875, count: 53899"
          },
          {
            "name": "SELECT\n  pid,\n  pg_wal_lsn_diff(sent_lsn, replay_lsn) AS replication_lag,\n  application_name::text,\n  state::text\nFROM pg_stat_replication; - Publisher - replication_lag:MB",
            "value": 0,
            "unit": "median replication_lag:MB",
            "extra": "avg replication_lag:MB: 0.000023790139139365412, max replication_lag:MB: 0.14044189453125, count: 53899"
          },
          {
            "name": "Top K - Subscriber - cpu",
            "value": 4.5757866,
            "unit": "median cpu",
            "extra": "avg cpu: 5.344590307678667, max cpu: 13.88621, count: 107798"
          },
          {
            "name": "Top K - Subscriber - mem",
            "value": 49.21875,
            "unit": "median mem",
            "extra": "avg mem: 49.24728807729503, max mem: 55.68359375, count: 107798"
          },
          {
            "name": "Update 1..9 - Publisher - cpu",
            "value": 4.5714283,
            "unit": "median cpu",
            "extra": "avg cpu: 4.519786281466709, max cpu: 4.6021094, count: 53899"
          },
          {
            "name": "Update 1..9 - Publisher - mem",
            "value": 32.2109375,
            "unit": "median mem",
            "extra": "avg mem: 31.54874952747268, max mem: 32.55859375, count: 53899"
          },
          {
            "name": "Update 10,11 - Publisher - cpu",
            "value": 4.5714283,
            "unit": "median cpu",
            "extra": "avg cpu: 4.079191547495327, max cpu: 4.6376815, count: 53899"
          },
          {
            "name": "Update 10,11 - Publisher - mem",
            "value": 32.4296875,
            "unit": "median mem",
            "extra": "avg mem: 31.72982040193232, max mem: 32.5078125, count: 53899"
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
        "date": 1778267874057,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - cpu",
            "value": 4.58891,
            "unit": "median cpu",
            "extra": "avg cpu: 5.120243750635613, max cpu: 9.29332, count: 53855"
          },
          {
            "name": "Custom Scan - Subscriber - mem",
            "value": 52.70703125,
            "unit": "median mem",
            "extra": "avg mem: 52.75769013728995, max mem: 58.63671875, count: 53855"
          },
          {
            "name": "Delete values - Publisher - cpu",
            "value": 4.5757866,
            "unit": "median cpu",
            "extra": "avg cpu: 4.278637357261062, max cpu: 4.5801525, count: 53855"
          },
          {
            "name": "Delete values - Publisher - mem",
            "value": 31.81640625,
            "unit": "median mem",
            "extra": "avg mem: 31.136309447822857, max mem: 32.11328125, count: 53855"
          },
          {
            "name": "Find by ctid - Subscriber - cpu",
            "value": 9.151573,
            "unit": "median cpu",
            "extra": "avg cpu: 8.322868135298364, max cpu: 18.426102, count: 53855"
          },
          {
            "name": "Find by ctid - Subscriber - mem",
            "value": 55.21875,
            "unit": "median mem",
            "extra": "avg mem: 54.90825328138056, max mem: 61.08203125, count: 53855"
          },
          {
            "name": "Index Only Scan - Subscriber - cpu",
            "value": 4.58891,
            "unit": "median cpu",
            "extra": "avg cpu: 5.090280149812615, max cpu: 9.320388, count: 53855"
          },
          {
            "name": "Index Only Scan - Subscriber - mem",
            "value": 51.453125,
            "unit": "median mem",
            "extra": "avg mem: 51.54105156496611, max mem: 57.38671875, count: 53855"
          },
          {
            "name": "Index Size Info - Subscriber - cpu",
            "value": 4.5845275,
            "unit": "median cpu",
            "extra": "avg cpu: 4.646483805402407, max cpu: 9.230769, count: 53855"
          },
          {
            "name": "Index Size Info - Subscriber - mem",
            "value": 33.375,
            "unit": "median mem",
            "extra": "avg mem: 33.41393223122273, max mem: 38.60546875, count: 53855"
          },
          {
            "name": "Index Size Info - Subscriber - pages",
            "value": 1100,
            "unit": "median pages",
            "extra": "avg pages: 1102.8968897966763, max pages: 1830.0, count: 53855"
          },
          {
            "name": "Index Size Info - Subscriber - relation_size:MB",
            "value": 8.59375,
            "unit": "median relation_size:MB",
            "extra": "avg relation_size:MB: 8.616381951536534, max relation_size:MB: 14.296875, count: 53855"
          },
          {
            "name": "Index Size Info - Subscriber - segment_count",
            "value": 9,
            "unit": "median segment_count",
            "extra": "avg segment_count: 9.477875777550832, max segment_count: 16.0, count: 53855"
          },
          {
            "name": "Insert value A - Publisher - cpu",
            "value": 0,
            "unit": "median cpu",
            "extra": "avg cpu: 2.098972752456801, max cpu: 4.624277, count: 53855"
          },
          {
            "name": "Insert value A - Publisher - mem",
            "value": 29.16015625,
            "unit": "median mem",
            "extra": "avg mem: 28.460098949145856, max mem: 29.51171875, count: 53855"
          },
          {
            "name": "Insert value B - Publisher - cpu",
            "value": 4.5933013,
            "unit": "median cpu",
            "extra": "avg cpu: 4.325272562081453, max cpu: 4.6153846, count: 53855"
          },
          {
            "name": "Insert value B - Publisher - mem",
            "value": 29.23046875,
            "unit": "median mem",
            "extra": "avg mem: 28.51870539237304, max mem: 29.57421875, count: 53855"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - cpu",
            "value": 4.6021094,
            "unit": "median cpu",
            "extra": "avg cpu: 6.4328786118110965, max cpu: 23.188406, count: 53855"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - mem",
            "value": 49.97265625,
            "unit": "median mem",
            "extra": "avg mem: 49.98451114392814, max mem: 55.8125, count: 53855"
          },
          {
            "name": "SELECT\n  pid,\n  pg_wal_lsn_diff(sent_lsn, replay_lsn) AS replication_lag,\n  application_name::text,\n  state::text\nFROM pg_stat_replication; - Publisher - replication_lag:MB",
            "value": 0,
            "unit": "median replication_lag:MB",
            "extra": "avg replication_lag:MB: 0.000013726110213115658, max replication_lag:MB: 0.06903076171875, count: 53855"
          },
          {
            "name": "Top K - Subscriber - cpu",
            "value": 4.58891,
            "unit": "median cpu",
            "extra": "avg cpu: 5.29664353470196, max cpu: 13.872832, count: 107710"
          },
          {
            "name": "Top K - Subscriber - mem",
            "value": 50.8125,
            "unit": "median mem",
            "extra": "avg mem: 50.855152761175844, max mem: 56.9375, count: 107710"
          },
          {
            "name": "Update 1..9 - Publisher - cpu",
            "value": 4.567079,
            "unit": "median cpu",
            "extra": "avg cpu: 3.896087166651035, max cpu: 4.624277, count: 53855"
          },
          {
            "name": "Update 1..9 - Publisher - mem",
            "value": 32.51171875,
            "unit": "median mem",
            "extra": "avg mem: 31.83853961157274, max mem: 32.8671875, count: 53855"
          },
          {
            "name": "Update 10,11 - Publisher - cpu",
            "value": 4.5757866,
            "unit": "median cpu",
            "extra": "avg cpu: 4.229830489554468, max cpu: 4.610951, count: 53855"
          },
          {
            "name": "Update 10,11 - Publisher - mem",
            "value": 32.64453125,
            "unit": "median mem",
            "extra": "avg mem: 31.974247545492528, max mem: 32.7265625, count: 53855"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "JLockerman",
            "username": "JLockerman",
            "email": "lockerman@paradedb.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "9a827ea3a0452e482e852785e743deb2f4630222",
          "message": "fix: Issues with text casts and memory layout for tokenizer types (#4900)\n\n# Ticket(s) Closed\n\n- fixes https://github.com/paradedb/paradedb/issues/5033\n\n## What\n\nChanges the tokenizer and alias types to function as regular SQL types\n(writable to disk, reallocatable in memory contexts etc).\n\n## Why\n\nWhen used incorrectly (eg. within a non-optimized function call) the\nprevious versions would access freed memory.\n\n## How\n\nThe tokenizer format is changed from `(header, magic_num, Oid, padding\nDatum)` to `(header, magic_num, metadata, padding, Oid, data_bytes)`\nwhere the `data_bytes` are the bytes from the original value (the\n`Datum` for by-value types, and the bytes pointed-at by the `Datum` for\nby-ref types). This lets us create a new `Datum` for that type (pointing\nat the inner bytes if needed).\n\nNOTE: Since the old version of the type was storing `Datum`s directly,\nany values stored to disk with the old code is broken unless they were\nin text format (the others store dangling pointers). In the updated\nversion such values will be output as meaningless text instead.\n\n## Tests\n\n- in\n`pg_search/tests/pg_regress/sql/tokenizer-types-inline-tokenization.sql`\n\n---------\n\nCo-authored-by: Mithun Chicklore Yogendra <mithun.cy@gmail.com>",
          "timestamp": "2026-05-08T18:32:32Z",
          "url": "https://github.com/paradedb/paradedb/commit/9a827ea3a0452e482e852785e743deb2f4630222"
        },
        "date": 1778268070922,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - cpu",
            "value": 4.5845275,
            "unit": "median cpu",
            "extra": "avg cpu: 5.088301691101696, max cpu: 9.275363, count: 53881"
          },
          {
            "name": "Custom Scan - Subscriber - mem",
            "value": 52.875,
            "unit": "median mem",
            "extra": "avg mem: 52.8338051243713, max mem: 58.703125, count: 53881"
          },
          {
            "name": "Delete values - Publisher - cpu",
            "value": 4.549763,
            "unit": "median cpu",
            "extra": "avg cpu: 2.831661996094453, max cpu: 4.6021094, count: 53881"
          },
          {
            "name": "Delete values - Publisher - mem",
            "value": 32.35546875,
            "unit": "median mem",
            "extra": "avg mem: 31.645319590277648, max mem: 32.67578125, count: 53881"
          },
          {
            "name": "Find by ctid - Subscriber - cpu",
            "value": 9.134158,
            "unit": "median cpu",
            "extra": "avg cpu: 7.9383935214087105, max cpu: 18.33811, count: 53881"
          },
          {
            "name": "Find by ctid - Subscriber - mem",
            "value": 56.14453125,
            "unit": "median mem",
            "extra": "avg mem: 55.771439027672095, max mem: 61.91796875, count: 53881"
          },
          {
            "name": "Index Only Scan - Subscriber - cpu",
            "value": 4.5845275,
            "unit": "median cpu",
            "extra": "avg cpu: 5.079267896343178, max cpu: 9.29332, count: 53881"
          },
          {
            "name": "Index Only Scan - Subscriber - mem",
            "value": 52.2890625,
            "unit": "median mem",
            "extra": "avg mem: 52.22251400946066, max mem: 58.06640625, count: 53881"
          },
          {
            "name": "Index Size Info - Subscriber - cpu",
            "value": 4.5845275,
            "unit": "median cpu",
            "extra": "avg cpu: 4.6367991432073, max cpu: 9.275363, count: 53881"
          },
          {
            "name": "Index Size Info - Subscriber - mem",
            "value": 33.8984375,
            "unit": "median mem",
            "extra": "avg mem: 33.90286189125109, max mem: 39.0625, count: 53881"
          },
          {
            "name": "Index Size Info - Subscriber - pages",
            "value": 1115,
            "unit": "median pages",
            "extra": "avg pages: 1107.1512221376738, max pages: 1837.0, count: 53881"
          },
          {
            "name": "Index Size Info - Subscriber - relation_size:MB",
            "value": 8.7109375,
            "unit": "median relation_size:MB",
            "extra": "avg relation_size:MB: 8.649619067946029, max relation_size:MB: 14.3515625, count: 53881"
          },
          {
            "name": "Index Size Info - Subscriber - segment_count",
            "value": 7,
            "unit": "median segment_count",
            "extra": "avg segment_count: 7.189157588018039, max segment_count: 12.0, count: 53881"
          },
          {
            "name": "Insert value A - Publisher - cpu",
            "value": 4.5801525,
            "unit": "median cpu",
            "extra": "avg cpu: 4.424633798165177, max cpu: 4.619827, count: 53881"
          },
          {
            "name": "Insert value A - Publisher - mem",
            "value": 29.44921875,
            "unit": "median mem",
            "extra": "avg mem: 28.72998388520536, max mem: 29.796875, count: 53881"
          },
          {
            "name": "Insert value B - Publisher - cpu",
            "value": 4.5411544,
            "unit": "median cpu",
            "extra": "avg cpu: 3.3888940224293123, max cpu: 4.597701, count: 53881"
          },
          {
            "name": "Insert value B - Publisher - mem",
            "value": 29.375,
            "unit": "median mem",
            "extra": "avg mem: 28.656345117017132, max mem: 29.74609375, count: 53881"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - cpu",
            "value": 4.597701,
            "unit": "median cpu",
            "extra": "avg cpu: 6.427146608689761, max cpu: 23.032629, count: 53881"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - mem",
            "value": 51.47265625,
            "unit": "median mem",
            "extra": "avg mem: 51.39180548048013, max mem: 57.23046875, count: 53881"
          },
          {
            "name": "SELECT\n  pid,\n  pg_wal_lsn_diff(sent_lsn, replay_lsn) AS replication_lag,\n  application_name::text,\n  state::text\nFROM pg_stat_replication; - Publisher - replication_lag:MB",
            "value": 0,
            "unit": "median replication_lag:MB",
            "extra": "avg replication_lag:MB: 0.00003313910724197189, max replication_lag:MB: 0.31896209716796875, count: 53881"
          },
          {
            "name": "Top K - Subscriber - cpu",
            "value": 4.58891,
            "unit": "median cpu",
            "extra": "avg cpu: 5.224920224408537, max cpu: 13.819577, count: 107762"
          },
          {
            "name": "Top K - Subscriber - mem",
            "value": 51.8359375,
            "unit": "median mem",
            "extra": "avg mem: 51.76473182800987, max mem: 58.0859375, count: 107762"
          },
          {
            "name": "Update 1..9 - Publisher - cpu",
            "value": 4.515522,
            "unit": "median cpu",
            "extra": "avg cpu: 3.588281460308457, max cpu: 4.58891, count: 53881"
          },
          {
            "name": "Update 1..9 - Publisher - mem",
            "value": 32.96484375,
            "unit": "median mem",
            "extra": "avg mem: 32.285758706106975, max mem: 33.26953125, count: 53881"
          },
          {
            "name": "Update 10,11 - Publisher - cpu",
            "value": 4.567079,
            "unit": "median cpu",
            "extra": "avg cpu: 3.9700309048814293, max cpu: 4.597701, count: 53881"
          },
          {
            "name": "Update 10,11 - Publisher - mem",
            "value": 33.12890625,
            "unit": "median mem",
            "extra": "avg mem: 32.45201150741913, max mem: 33.23828125, count: 53881"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "lockerman@paradedb.com",
            "name": "JLockerman",
            "username": "JLockerman"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "9a827ea3a0452e482e852785e743deb2f4630222",
          "message": "fix: Issues with text casts and memory layout for tokenizer types (#4900)\n\n# Ticket(s) Closed\n\n- fixes https://github.com/paradedb/paradedb/issues/5033\n\n## What\n\nChanges the tokenizer and alias types to function as regular SQL types\n(writable to disk, reallocatable in memory contexts etc).\n\n## Why\n\nWhen used incorrectly (eg. within a non-optimized function call) the\nprevious versions would access freed memory.\n\n## How\n\nThe tokenizer format is changed from `(header, magic_num, Oid, padding\nDatum)` to `(header, magic_num, metadata, padding, Oid, data_bytes)`\nwhere the `data_bytes` are the bytes from the original value (the\n`Datum` for by-value types, and the bytes pointed-at by the `Datum` for\nby-ref types). This lets us create a new `Datum` for that type (pointing\nat the inner bytes if needed).\n\nNOTE: Since the old version of the type was storing `Datum`s directly,\nany values stored to disk with the old code is broken unless they were\nin text format (the others store dangling pointers). In the updated\nversion such values will be output as meaningless text instead.\n\n## Tests\n\n- in\n`pg_search/tests/pg_regress/sql/tokenizer-types-inline-tokenization.sql`\n\n---------\n\nCo-authored-by: Mithun Chicklore Yogendra <mithun.cy@gmail.com>",
          "timestamp": "2026-05-08T14:32:32-04:00",
          "tree_id": "9cf77ffd18186494bb164cc15f9f703749357d03",
          "url": "https://github.com/paradedb/paradedb/commit/9a827ea3a0452e482e852785e743deb2f4630222"
        },
        "date": 1778269773640,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - cpu",
            "value": 4.562738,
            "unit": "median cpu",
            "extra": "avg cpu: 5.068155420107755, max cpu: 9.239654, count: 53918"
          },
          {
            "name": "Custom Scan - Subscriber - mem",
            "value": 52.46875,
            "unit": "median mem",
            "extra": "avg mem: 52.4619273567037, max mem: 58.29296875, count: 53918"
          },
          {
            "name": "Delete values - Publisher - cpu",
            "value": 4.5028143,
            "unit": "median cpu",
            "extra": "avg cpu: 2.8438904330315613, max cpu: 4.567079, count: 53918"
          },
          {
            "name": "Delete values - Publisher - mem",
            "value": 31.94140625,
            "unit": "median mem",
            "extra": "avg mem: 31.269516035924923, max mem: 32.25390625, count: 53918"
          },
          {
            "name": "Find by ctid - Subscriber - cpu",
            "value": 9.108159,
            "unit": "median cpu",
            "extra": "avg cpu: 8.458758344875772, max cpu: 18.426102, count: 53918"
          },
          {
            "name": "Find by ctid - Subscriber - mem",
            "value": 55.39453125,
            "unit": "median mem",
            "extra": "avg mem: 55.028159659111985, max mem: 61.1484375, count: 53918"
          },
          {
            "name": "Index Only Scan - Subscriber - cpu",
            "value": 4.562738,
            "unit": "median cpu",
            "extra": "avg cpu: 5.079992062364404, max cpu: 9.239654, count: 53918"
          },
          {
            "name": "Index Only Scan - Subscriber - mem",
            "value": 51.62109375,
            "unit": "median mem",
            "extra": "avg mem: 51.64833353715364, max mem: 57.51171875, count: 53918"
          },
          {
            "name": "Index Size Info - Subscriber - cpu",
            "value": 4.5584044,
            "unit": "median cpu",
            "extra": "avg cpu: 4.642827910808374, max cpu: 9.195402, count: 53918"
          },
          {
            "name": "Index Size Info - Subscriber - mem",
            "value": 33.28125,
            "unit": "median mem",
            "extra": "avg mem: 33.3430228396593, max mem: 38.51171875, count: 53918"
          },
          {
            "name": "Index Size Info - Subscriber - pages",
            "value": 1111,
            "unit": "median pages",
            "extra": "avg pages: 1109.6617270670276, max pages: 1832.0, count: 53918"
          },
          {
            "name": "Index Size Info - Subscriber - relation_size:MB",
            "value": 8.6796875,
            "unit": "median relation_size:MB",
            "extra": "avg relation_size:MB: 8.669232387607106, max relation_size:MB: 14.3125, count: 53918"
          },
          {
            "name": "Index Size Info - Subscriber - segment_count",
            "value": 10,
            "unit": "median segment_count",
            "extra": "avg segment_count: 10.169405393375126, max segment_count: 20.0, count: 53918"
          },
          {
            "name": "Insert value A - Publisher - cpu",
            "value": 4.5540795,
            "unit": "median cpu",
            "extra": "avg cpu: 4.0492089843300345, max cpu: 4.597701, count: 53918"
          },
          {
            "name": "Insert value A - Publisher - mem",
            "value": 29.42578125,
            "unit": "median mem",
            "extra": "avg mem: 28.726936766246894, max mem: 29.73046875, count: 53918"
          },
          {
            "name": "Insert value B - Publisher - cpu",
            "value": 4.549763,
            "unit": "median cpu",
            "extra": "avg cpu: 3.9251285340129836, max cpu: 4.567079, count: 53918"
          },
          {
            "name": "Insert value B - Publisher - mem",
            "value": 29.4296875,
            "unit": "median mem",
            "extra": "avg mem: 28.7227446365314, max mem: 29.8203125, count: 53918"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - cpu",
            "value": 4.5757866,
            "unit": "median cpu",
            "extra": "avg cpu: 6.170392124386594, max cpu: 22.922636, count: 53918"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - mem",
            "value": 50.48046875,
            "unit": "median mem",
            "extra": "avg mem: 50.51053581943878, max mem: 56.34375, count: 53918"
          },
          {
            "name": "SELECT\n  pid,\n  pg_wal_lsn_diff(sent_lsn, replay_lsn) AS replication_lag,\n  application_name::text,\n  state::text\nFROM pg_stat_replication; - Publisher - replication_lag:MB",
            "value": 0,
            "unit": "median replication_lag:MB",
            "extra": "avg replication_lag:MB: 0.000018346742563287654, max replication_lag:MB: 0.1705169677734375, count: 53918"
          },
          {
            "name": "Top K - Subscriber - cpu",
            "value": 4.567079,
            "unit": "median cpu",
            "extra": "avg cpu: 5.254197285725174, max cpu: 13.766731, count: 107836"
          },
          {
            "name": "Top K - Subscriber - mem",
            "value": 51.73046875,
            "unit": "median mem",
            "extra": "avg mem: 51.74735224380077, max mem: 57.75, count: 107836"
          },
          {
            "name": "Update 1..9 - Publisher - cpu",
            "value": 4.5584044,
            "unit": "median cpu",
            "extra": "avg cpu: 4.386450046493038, max cpu: 4.5714283, count: 53918"
          },
          {
            "name": "Update 1..9 - Publisher - mem",
            "value": 32.57421875,
            "unit": "median mem",
            "extra": "avg mem: 31.827803983015876, max mem: 32.8984375, count: 53918"
          },
          {
            "name": "Update 10,11 - Publisher - cpu",
            "value": 4.524034,
            "unit": "median cpu",
            "extra": "avg cpu: 4.491905958739107, max cpu: 4.5801525, count: 53918"
          },
          {
            "name": "Update 10,11 - Publisher - mem",
            "value": 32.72265625,
            "unit": "median mem",
            "extra": "avg mem: 32.06476284009978, max mem: 32.82421875, count: 53918"
          }
        ]
      }
    ]
  }
}