window.BENCHMARK_DATA = {
  "lastUpdate": 1778524163731,
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
      }
    ]
  }
}