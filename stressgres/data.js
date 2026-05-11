window.BENCHMARK_DATA = {
  "lastUpdate": 1778525289870,
  "repoUrl": "https://github.com/paradedb/paradedb",
  "entries": {
    "pg_search single-server.toml Performance - TPS": [
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
        "date": 1778524113825,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - tps",
            "value": 130.22291348567046,
            "unit": "median tps",
            "extra": "avg tps: 130.2368552836531, max tps: 144.98839256387993, count: 55233"
          },
          {
            "name": "Columnar Scan - Primary - tps",
            "value": 465.0349144000749,
            "unit": "median tps",
            "extra": "avg tps: 463.9053108462951, max tps: 568.6816825527001, count: 55233"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2954.3446833027974,
            "unit": "median tps",
            "extra": "avg tps: 2934.5990246514584, max tps: 2964.529908853002, count: 55233"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 416.8586998596091,
            "unit": "median tps",
            "extra": "avg tps: 416.75215994639143, max tps: 546.9208900809333, count: 55233"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 2965.0922869349074,
            "unit": "median tps",
            "extra": "avg tps: 3011.240472166101, max tps: 3097.1860059374167, count: 110466"
          },
          {
            "name": "Normal Scan - Primary - tps",
            "value": 449.80429028285363,
            "unit": "median tps",
            "extra": "avg tps: 449.20906259538145, max tps: 581.1111798030348, count: 55233"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1956.0385968367616,
            "unit": "median tps",
            "extra": "avg tps: 1940.7293477535934, max tps: 1962.3618433556128, count: 55233"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 36.781161842814036,
            "unit": "median tps",
            "extra": "avg tps: 63.235529810054764, max tps: 866.4332495492381, count: 55233"
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
        "date": 1778524463474,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - tps",
            "value": 128.64386292411558,
            "unit": "median tps",
            "extra": "avg tps: 128.95555850005914, max tps: 142.44541271641796, count: 55043"
          },
          {
            "name": "Columnar Scan - Primary - tps",
            "value": 503.0313013123388,
            "unit": "median tps",
            "extra": "avg tps: 501.33224364016957, max tps: 526.9012510048858, count: 55043"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3262.543500905718,
            "unit": "median tps",
            "extra": "avg tps: 3257.9866946578036, max tps: 3346.673209321352, count: 55043"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 424.6765627744994,
            "unit": "median tps",
            "extra": "avg tps: 423.8834197585094, max tps: 477.8197766192544, count: 55043"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 3040.769535480371,
            "unit": "median tps",
            "extra": "avg tps: 3034.1818303189666, max tps: 3115.5200720347802, count: 110086"
          },
          {
            "name": "Normal Scan - Primary - tps",
            "value": 476.05112409428085,
            "unit": "median tps",
            "extra": "avg tps: 474.99212012668545, max tps: 596.8742125102216, count: 55043"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 2134.266477374074,
            "unit": "median tps",
            "extra": "avg tps: 2122.8547315101973, max tps: 2140.8896996861367, count: 55043"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 73.56459051416459,
            "unit": "median tps",
            "extra": "avg tps: 84.20056535639911, max tps: 867.8242864097849, count: 55043"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Philippe Noël",
            "username": "philippemnoel",
            "email": "21990816+philippemnoel@users.noreply.github.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "b09f4be12951cd04a36e9bbb0dc8c405645ae09b",
          "message": "ci: Migrate create-github-app-token to client-id (#5050)\n\n## Summary\n- The `actions/create-github-app-token` action deprecated `app-id` in\nfavor of `client-id` (warning: `Input 'app-id' has been deprecated with\nmessage: Use 'client-id' instead.`)\n- Replaces `app-id:` with `client-id:` across all workflows and the\n`benchmark-stressgres` composite action\n- Switches from `vars.PARADEDB_GITHUB_APP_ID` (numeric App ID) to\n`vars.PARADEDB_GITHUB_APP_CLIENT_ID` (the App's Client ID, e.g.\n`Iv23li...`)\n\n## Notes\n- Client ID is a public identifier, so `vars.*` is appropriate; the\nPrivate Key remains in `secrets.PARADEDB_GITHUB_APP_PRIVATE_KEY`\n- `vars.PARADEDB_GITHUB_APP_CLIENT_ID` has been added to repo variables\n- The old `vars.PARADEDB_GITHUB_APP_ID` is no longer referenced and can\nbe deleted after merge\n\n## Test plan\n- [ ] Verify cherry-pick, publish-github-release,\npublish-paradedb-docker, test-pg_search-nix, and\nbenchmark-pg_search-stressgres workflows successfully mint a token on\nnext run\n\n---------\n\nCo-authored-by: paradedb-github-app[bot] <282009505+paradedb-github-app[bot]@users.noreply.github.com>",
          "timestamp": "2026-05-09T18:11:33Z",
          "url": "https://github.com/paradedb/paradedb/commit/b09f4be12951cd04a36e9bbb0dc8c405645ae09b"
        },
        "date": 1778524610227,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - tps",
            "value": 130.39790779890825,
            "unit": "median tps",
            "extra": "avg tps: 131.03661911571334, max tps: 145.8705190395134, count: 55248"
          },
          {
            "name": "Columnar Scan - Primary - tps",
            "value": 473.2180747571198,
            "unit": "median tps",
            "extra": "avg tps: 474.78234792852726, max tps: 612.1502546641715, count: 55248"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3257.350083436879,
            "unit": "median tps",
            "extra": "avg tps: 3249.7422604788667, max tps: 3267.2589303659397, count: 55248"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 444.0734380142206,
            "unit": "median tps",
            "extra": "avg tps: 446.08093486203364, max tps: 480.5364214342859, count: 55248"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 2968.415674554292,
            "unit": "median tps",
            "extra": "avg tps: 2968.7702024265236, max tps: 3089.6579560714977, count: 110496"
          },
          {
            "name": "Normal Scan - Primary - tps",
            "value": 531.3018308127793,
            "unit": "median tps",
            "extra": "avg tps: 530.4682826843944, max tps: 605.1789623017568, count: 55248"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 2016.9813940072656,
            "unit": "median tps",
            "extra": "avg tps: 2012.2135535571604, max tps: 2025.738747970457, count: 55248"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 172.6774398613718,
            "unit": "median tps",
            "extra": "avg tps: 205.28319970200715, max tps: 327.18365201919784, count: 55248"
          }
        ]
      }
    ],
    "pg_search single-server.toml Performance - Other Metrics": [
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
        "date": 1778524165601,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - cpu",
            "value": 9.266409,
            "unit": "median cpu",
            "extra": "avg cpu: 8.555495544781406, max cpu: 23.30097, count: 55233"
          },
          {
            "name": "Aggregate Custom Scan - Primary - mem",
            "value": 63.59765625,
            "unit": "median mem",
            "extra": "avg mem: 63.498419691918784, max mem: 74.96484375, count: 55233"
          },
          {
            "name": "Columnar Scan - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 5.717157957829779, max cpu: 18.879055, count: 55233"
          },
          {
            "name": "Columnar Scan - Primary - mem",
            "value": 62.875,
            "unit": "median mem",
            "extra": "avg mem: 62.739240184762735, max mem: 74.14453125, count: 55233"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.702224957744425, max cpu: 9.239654, count: 55233"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 36.09375,
            "unit": "median mem",
            "extra": "avg mem: 35.75835466342585, max mem: 37.859375, count: 55233"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.633045253878696, max cpu: 9.221902, count: 55233"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 61.98046875,
            "unit": "median mem",
            "extra": "avg mem: 61.45061093458621, max mem: 73.3671875, count: 55233"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.672038941219931, max cpu: 9.329447, count: 110466"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 53.1796875,
            "unit": "median mem",
            "extra": "avg mem: 52.345820313207234, max mem: 67.9296875, count: 110466"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 1793,
            "unit": "median block_count",
            "extra": "avg block_count: 1797.0626980247316, max block_count: 3185.0, count: 55233"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 13,
            "unit": "median segment_count",
            "extra": "avg segment_count: 13.509097821954267, max segment_count: 30.0, count: 55233"
          },
          {
            "name": "Normal Scan - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 5.765923358939593, max cpu: 18.461538, count: 55233"
          },
          {
            "name": "Normal Scan - Primary - mem",
            "value": 62.703125,
            "unit": "median mem",
            "extra": "avg mem: 62.56244158270418, max mem: 73.9921875, count: 55233"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.548462779106955, max cpu: 4.7619047, count: 55233"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 52.45703125,
            "unit": "median mem",
            "extra": "avg mem: 52.29626621539659, max mem: 63.4140625, count: 55233"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.14361091012378, max cpu: 4.7151275, count: 55233"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 54.59375,
            "unit": "median mem",
            "extra": "avg mem: 52.503778806940595, max mem: 66.95703125, count: 55233"
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
        "date": 1778524506146,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - cpu",
            "value": 9.248554,
            "unit": "median cpu",
            "extra": "avg cpu: 8.503531390340118, max cpu: 23.904383, count: 55043"
          },
          {
            "name": "Aggregate Custom Scan - Primary - mem",
            "value": 64.7421875,
            "unit": "median mem",
            "extra": "avg mem: 64.59525202171484, max mem: 75.8984375, count: 55043"
          },
          {
            "name": "Columnar Scan - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 5.576367515411542, max cpu: 18.879055, count: 55043"
          },
          {
            "name": "Columnar Scan - Primary - mem",
            "value": 63.5625,
            "unit": "median mem",
            "extra": "avg mem: 63.453343721045364, max mem: 74.765625, count: 55043"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.6273998622160235, max cpu: 9.239654, count: 55043"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 36.23046875,
            "unit": "median mem",
            "extra": "avg mem: 36.038118564690336, max mem: 38.33984375, count: 55043"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.588217149122594, max cpu: 9.266409, count: 55043"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 62.19921875,
            "unit": "median mem",
            "extra": "avg mem: 61.75364558731356, max mem: 73.390625, count: 55043"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.654309706807927, max cpu: 9.284333, count: 110086"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 60.5859375,
            "unit": "median mem",
            "extra": "avg mem: 58.60893510953936, max mem: 71.96875, count: 110086"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 1726,
            "unit": "median block_count",
            "extra": "avg block_count: 1730.3158803117562, max block_count: 3102.0, count: 55043"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 14,
            "unit": "median segment_count",
            "extra": "avg segment_count: 15.201951201787693, max segment_count: 28.0, count: 55043"
          },
          {
            "name": "Normal Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 5.676876637314322, max cpu: 18.550726, count: 55043"
          },
          {
            "name": "Normal Scan - Primary - mem",
            "value": 63.4453125,
            "unit": "median mem",
            "extra": "avg mem: 63.35300121893338, max mem: 74.61328125, count: 55043"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.6075382437635515, max cpu: 4.833837, count: 55043"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 53.0625,
            "unit": "median mem",
            "extra": "avg mem: 52.87469434408099, max mem: 63.9921875, count: 55043"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.610951,
            "unit": "median cpu",
            "extra": "avg cpu: 4.059666024946636, max cpu: 4.619827, count: 55043"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 52.91015625,
            "unit": "median mem",
            "extra": "avg mem: 54.67124772053213, max mem: 67.32421875, count: 55043"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Philippe Noël",
            "username": "philippemnoel",
            "email": "21990816+philippemnoel@users.noreply.github.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "b09f4be12951cd04a36e9bbb0dc8c405645ae09b",
          "message": "ci: Migrate create-github-app-token to client-id (#5050)\n\n## Summary\n- The `actions/create-github-app-token` action deprecated `app-id` in\nfavor of `client-id` (warning: `Input 'app-id' has been deprecated with\nmessage: Use 'client-id' instead.`)\n- Replaces `app-id:` with `client-id:` across all workflows and the\n`benchmark-stressgres` composite action\n- Switches from `vars.PARADEDB_GITHUB_APP_ID` (numeric App ID) to\n`vars.PARADEDB_GITHUB_APP_CLIENT_ID` (the App's Client ID, e.g.\n`Iv23li...`)\n\n## Notes\n- Client ID is a public identifier, so `vars.*` is appropriate; the\nPrivate Key remains in `secrets.PARADEDB_GITHUB_APP_PRIVATE_KEY`\n- `vars.PARADEDB_GITHUB_APP_CLIENT_ID` has been added to repo variables\n- The old `vars.PARADEDB_GITHUB_APP_ID` is no longer referenced and can\nbe deleted after merge\n\n## Test plan\n- [ ] Verify cherry-pick, publish-github-release,\npublish-paradedb-docker, test-pg_search-nix, and\nbenchmark-pg_search-stressgres workflows successfully mint a token on\nnext run\n\n---------\n\nCo-authored-by: paradedb-github-app[bot] <282009505+paradedb-github-app[bot]@users.noreply.github.com>",
          "timestamp": "2026-05-09T18:11:33Z",
          "url": "https://github.com/paradedb/paradedb/commit/b09f4be12951cd04a36e9bbb0dc8c405645ae09b"
        },
        "date": 1778524668947,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - cpu",
            "value": 9.284333,
            "unit": "median cpu",
            "extra": "avg cpu: 8.619795371956059, max cpu: 19.104477, count: 55248"
          },
          {
            "name": "Aggregate Custom Scan - Primary - mem",
            "value": 66.40234375,
            "unit": "median mem",
            "extra": "avg mem: 66.15977982234651, max mem: 77.609375, count: 55248"
          },
          {
            "name": "Columnar Scan - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 5.736821397461783, max cpu: 18.713451, count: 55248"
          },
          {
            "name": "Columnar Scan - Primary - mem",
            "value": 65.16796875,
            "unit": "median mem",
            "extra": "avg mem: 64.90599178929554, max mem: 76.3359375, count: 55248"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.708439094903292, max cpu: 9.329447, count: 55248"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 35.12109375,
            "unit": "median mem",
            "extra": "avg mem: 34.79579132106592, max mem: 36.45703125, count: 55248"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.652706646035298, max cpu: 9.275363, count: 55248"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 63.5390625,
            "unit": "median mem",
            "extra": "avg mem: 62.81374438893716, max mem: 74.7734375, count: 55248"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.639315523248876, max cpu: 9.365853, count: 110496"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 62.4375,
            "unit": "median mem",
            "extra": "avg mem: 60.73561698500398, max mem: 73.4375, count: 110496"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 1783,
            "unit": "median block_count",
            "extra": "avg block_count: 1779.3004090645816, max block_count: 3155.0, count: 55248"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 11,
            "unit": "median segment_count",
            "extra": "avg segment_count: 10.464433101650739, max segment_count: 23.0, count: 55248"
          },
          {
            "name": "Normal Scan - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 5.45458948950771, max cpu: 14.243324, count: 55248"
          },
          {
            "name": "Normal Scan - Primary - mem",
            "value": 64.83203125,
            "unit": "median mem",
            "extra": "avg mem: 64.65307963636693, max mem: 76.10546875, count: 55248"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.417500983217531, max cpu: 4.7619047, count: 55248"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 62.32421875,
            "unit": "median mem",
            "extra": "avg mem: 58.96950639050282, max mem: 72.8984375, count: 55248"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 0,
            "unit": "median cpu",
            "extra": "avg cpu: 1.8083253167823952, max cpu: 4.6875, count: 55248"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 56.34765625,
            "unit": "median mem",
            "extra": "avg mem: 55.46840171250543, max mem: 67.71484375, count: 55248"
          }
        ]
      }
    ],
    "pg_search bulk-updates.toml Performance - TPS": [
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
        "date": 1778524853202,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 7.842830522568889,
            "unit": "median tps",
            "extra": "avg tps: 6.70858065661378, max tps: 10.218102947795867, count: 57574"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.346574107000282,
            "unit": "median tps",
            "extra": "avg tps: 4.798661284996681, max tps: 5.995420939811215, count: 57574"
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
        "date": 1778525186781,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 7.92700021144408,
            "unit": "median tps",
            "extra": "avg tps: 6.764277775948097, max tps: 10.304365546692813, count: 57789"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.4212954831500335,
            "unit": "median tps",
            "extra": "avg tps: 4.8722095172010595, max tps: 6.094497440239919, count: 57789"
          }
        ]
      }
    ],
    "pg_search bulk-updates.toml Performance - Other Metrics": [
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
        "date": 1778524904579,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.255816,
            "unit": "median cpu",
            "extra": "avg cpu: 20.919273614866285, max cpu: 43.286575, count: 57574"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 233.171875,
            "unit": "median mem",
            "extra": "avg mem: 233.0931331308403, max mem: 234.65234375, count: 57574"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.391813,
            "unit": "median cpu",
            "extra": "avg cpu: 22.542420005190632, max cpu: 33.3996, count: 57574"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 175.50390625,
            "unit": "median mem",
            "extra": "avg mem: 175.39045870586116, max mem: 176.60546875, count: 57574"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 34807,
            "unit": "median block_count",
            "extra": "avg block_count: 33774.47022961754, max block_count: 36798.0, count: 57574"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 79,
            "unit": "median segment_count",
            "extra": "avg segment_count: 81.78229756487303, max segment_count: 134.0, count: 57574"
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
        "date": 1778525238795,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.210833,
            "unit": "median cpu",
            "extra": "avg cpu: 20.608575057151693, max cpu: 42.814667, count: 57789"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 234.3671875,
            "unit": "median mem",
            "extra": "avg mem: 234.20679121999862, max mem: 235.84375, count: 57789"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.323614,
            "unit": "median cpu",
            "extra": "avg cpu: 22.445049996802574, max cpu: 33.136093, count: 57789"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 176.96875,
            "unit": "median mem",
            "extra": "avg mem: 176.85239155321514, max mem: 177.59765625, count: 57789"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 34706,
            "unit": "median block_count",
            "extra": "avg block_count: 33833.76256727059, max block_count: 36533.0, count: 57789"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 79,
            "unit": "median segment_count",
            "extra": "avg segment_count: 81.95890221322396, max segment_count: 129.0, count: 57789"
          }
        ]
      }
    ]
  }
}