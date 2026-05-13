window.BENCHMARK_DATA = {
  "lastUpdate": 1778653710487,
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
      },
      {
        "commit": {
          "author": {
            "email": "mdashti@gmail.com",
            "name": "Moe",
            "username": "mdashti"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "6bdea0d414fb563f086ddfe56712b2960d200626",
          "message": "feat(mpp): AggregateScan in-process MPP via custom shm_mq transport (#4988)\n\n# Ticket(s) Closed\n\n- Closes #4152\n\n## What\n\nAdds an MPP execution path for AggregateScan that runs\njoin-with-aggregate shapes inside Postgres parallel-worker processes via\na custom transport on top of `paradedb/datafusion-distributed`. Default\noff behind `paradedb.enable_mpp` (minimum `mpp_worker_count = 3`). Other\nshapes and the JoinScan path are unchanged.\n\n## Why\n\nSingle-process DataFusion bottlenecks on join-with-aggregate at scale.\nDistributing the producer fragment across PG parallel workers gives us\n1.55–1.92× speedup on the 25M `aggregate_join_groupby` bench at N=2/4\nwithout leaving the embedded model — every worker is still a real PG\nprocess with its own snapshot, no gRPC, no extra daemon.\n\n## How\n\n- Leader builds the logical plan and stashes it in a DSM segment\nalongside an N-way `shm_mq` mesh.\n- Workers attach, deserialize, and re-plan with the same `SessionState`.\nIdentical inputs ⇒ structurally identical physical plans on every\nworker, so we don't need to serialize physical subplans.\n- Each worker runs its producer fragment and pushes batches through its\noutbound queue. Leader runs `NetworkShuffleExec` + final aggregate and\nreturns rows to the client. Leader is consumer-only in this iteration.\n- Build side (non-partitioning sources like `HashJoinExec(CollectLeft)`)\nis split via DSM all-gather: each worker scans its 1/N slice, writes to\na per-worker DSM region, completion-flag barrier, then everyone reads\nevery slice. Build is fully parallel; no leader-side serial scan.\n- The fork emits the network operators (`NetworkShuffleExec`,\n`NetworkBroadcastExec`, `NetworkCoalesceExec`) and we register a custom\ntransport that short-circuits the gRPC dialer. The fork's in-process\ntwo-boundary planner distinguishes outer (worker → leader, N producers)\nfrom nested (single local producer) Network boundaries.\n\n## Reviewer's Guide\n\nSuggested reading order — most of the diff lives under\n`pg_search/src/postgres/customscan/mpp/`:\n\n1. `mpp/dsm.rs` — the DSM layout: header, queue mesh, build-cache\nregion. `compute_dsm_layout` is the math; `leader_init` /\n`worker_attach` are the unsafe FFI boundaries.\n2. `mpp/runtime.rs` — `MppMesh` (runtime handle), `ShmMqWorkerTransport`\n(the `WorkerTransport` impl the leader registers),\n`LocalExecWorkerTransport` (the worker-side stub for nested broadcasts),\n`MppWorkerResolver`.\n3. `mpp/transport.rs` — `DrainHandle` and the cooperative-pull\nprimitives. The drain runs inline on the backend thread because pgrx\n0.18 enforces single-threaded Postgres FFI.\n4. `mpp/glue.rs` — the public API the customscan calls:\n`estimate_dsm_size`, `leader_setup`, `worker_setup`. Thin wrappers\naround dsm/runtime.\n5. `mpp/exec.rs` — `run_producer_fragment` is the worker push loop.\n6. `aggregatescan/mod.rs` — the integration: `stash_mpp_plan_bytes`,\n`exec_mpp_worker`, `build_mpp_leader_session_context`, the\n`ParallelQueryCapable` impl, the `parallel_workers` clamp in\n`try_build_datafusion_aggregate_path`. Both leader and worker session\ncontexts call `with_distributed_in_process_mode(true)` explicitly.\n\nThe build-side all-gather lives in\n`aggregatescan/mod.rs::exec_mpp_worker` and `mpp/dsm.rs` (the cache\nregion). Read those together.\n\n## Tests\n\n- pgrx regression suite — new `mpp_aggregate.sql` covers correctness on\nthe join-with-aggregate shapes; existing `mpp_*` suites still pass.\n- 25M `aggregate_join_groupby` bench: byte-exact result vs serial\nDataFusion at N=2/4/8/10.\n- Build-side all-gather is exercised at all N in the regression suite\n(workers vs leader-only-writer paths).\n\n---------\n\nCo-authored-by: paradedb-bot <developers@paradedb.com>\nCo-authored-by: paradedb-github-app[bot] <282009505+paradedb-github-app[bot]@users.noreply.github.com>",
          "timestamp": "2026-05-12T16:47:00-07:00",
          "tree_id": "38dfa579bfcab58f4b6b66ddea91de57c32f5204",
          "url": "https://github.com/paradedb/paradedb/commit/6bdea0d414fb563f086ddfe56712b2960d200626"
        },
        "date": 1778631482950,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - tps",
            "value": 131.00641805298625,
            "unit": "median tps",
            "extra": "avg tps: 131.7183530710557, max tps: 144.5622552840286, count: 55221"
          },
          {
            "name": "Columnar Scan - Primary - tps",
            "value": 508.96458956338586,
            "unit": "median tps",
            "extra": "avg tps: 510.03930121755263, max tps: 686.4573856407759, count: 55221"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3292.706491658971,
            "unit": "median tps",
            "extra": "avg tps: 3284.9867479303393, max tps: 3315.5124231811415, count: 55221"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 436.6589478343826,
            "unit": "median tps",
            "extra": "avg tps: 437.6741175383611, max tps: 511.0736020944478, count: 55221"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 3030.7353230482795,
            "unit": "median tps",
            "extra": "avg tps: 3029.332350761033, max tps: 3081.169157362041, count: 110442"
          },
          {
            "name": "Normal Scan - Primary - tps",
            "value": 511.84773501251243,
            "unit": "median tps",
            "extra": "avg tps: 513.2184817622841, max tps: 596.0553928316975, count: 55221"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 2018.60203568295,
            "unit": "median tps",
            "extra": "avg tps: 2006.1456982104362, max tps: 2024.643088758003, count: 55221"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 67.34385086164706,
            "unit": "median tps",
            "extra": "avg tps: 76.03067470667787, max tps: 222.15077195171745, count: 55221"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "james.sewell@gmail.com",
            "name": "James Sewell",
            "username": "jamessewell"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "8182eaf110c30cbefe008197caa40efa8b44f8e0",
          "message": "refactor: use existing FFHelper ctid cache instead of dedicated cache (#4905)\n\nFix a performance regression introduced in e0804b347 (#4765) which\nremoved ctid from SearchIndexScore and switched to lazy per-row\nresolution.\n\nPrior to #4765, ctid was resolved during result construction and carried\nin `SearchIndexScore` — no per-row fast-field lookups needed. #4765\nmoved ctid resolution to the consumption side (top_k.rs, normal.rs,\nscan.rs) using a single-entry `Option<(SegmentOrdinal, FFType)>` cache.\nWhen TopK results interleave across segments (sorted by score), every\nsegment transition re-opens the ctid column via `FastFieldReaders::u64\n-> DynamicColumnHandle::open -> BlockwiseLinearCodec::load`, which is\nvery expensive. Profiling showed 45% of total cycles spent in this\nre-open path.\n\nThe columnar scan path (`ColumnarExecState`) was unaffected — it already\nused `FFHelper`'s per-segment `OnceLock` ctid cache. This PR brings the\nremaining paths in line:\n\n- `scan.rs` uses its existing `Bm25ScanState.fast_fields` FFHelper\n- `normal.rs` and `top_k.rs` use a new `ctid_cache` FFHelper on\n`BaseScanState`\n\nEach segment's ctid column is opened at most once via `OnceLock`,\neliminating the thrashing. `FFHelper` has had this per-segment ctid\ncaching built in since cb78f0ca2 (Oct 2024).",
          "timestamp": "2026-05-13T12:18:31+12:00",
          "tree_id": "814e1da895eec41e0dfe3cbb5348bdb237811bf7",
          "url": "https://github.com/paradedb/paradedb/commit/8182eaf110c30cbefe008197caa40efa8b44f8e0"
        },
        "date": 1778633380401,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - tps",
            "value": 134.44772096721013,
            "unit": "median tps",
            "extra": "avg tps: 134.8665599203604, max tps: 146.58834784657859, count: 55114"
          },
          {
            "name": "Columnar Scan - Primary - tps",
            "value": 501.3056430048433,
            "unit": "median tps",
            "extra": "avg tps: 504.3773654119763, max tps: 670.3929495559473, count: 55114"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3322.836603293853,
            "unit": "median tps",
            "extra": "avg tps: 3299.83297960128, max tps: 3338.9642482025924, count: 55114"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 444.32427190473334,
            "unit": "median tps",
            "extra": "avg tps: 447.585824690253, max tps: 554.2868106424017, count: 55114"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 3017.037923029604,
            "unit": "median tps",
            "extra": "avg tps: 3006.1900651768856, max tps: 3040.8386928133427, count: 110228"
          },
          {
            "name": "Normal Scan - Primary - tps",
            "value": 524.8389229349693,
            "unit": "median tps",
            "extra": "avg tps: 526.9844944825845, max tps: 674.7039227652011, count: 55114"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 2058.30716837331,
            "unit": "median tps",
            "extra": "avg tps: 2048.970564480642, max tps: 2068.0258867143543, count: 55114"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 32.98109823577119,
            "unit": "median tps",
            "extra": "avg tps: 46.88167502744731, max tps: 787.3866950545817, count: 55114"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mithun.cy@gmail.com",
            "name": "Mithun Chicklore Yogendra",
            "username": "mithuncy"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "fa4b7613b7a49e0a05075ef635fac3a6f677fd31",
          "message": "feat: agg-on-join end-to-end IN/NOT IN/EXISTS/NOT EXISTS with null-aware semantics (#5005)\n\n## Summary\n\nMakes the agg-on-join path handle `IN (SELECT ...)`, `NOT IN (SELECT\n...)`, `EXISTS (SELECT ...)`, and `NOT EXISTS (SELECT ...)` end-to-end,\nincluding the `NOT IN` against a NULL-bearing inner case, which is the\nhard one because of SQL's three-valued NULL logic.\n\nTarget query shape - aggregate over a join with IN/NOT IN sublinks plus\na BM25 search predicate:\n\n```sql\nSELECT contact_job_title, COUNT(*) AS doc_count\nFROM contacts\nWHERE contact_id IN     (SELECT ldf_id FROM contact_list WHERE list_id IN ('include_list'))\n  AND contact_id NOT IN (SELECT ldf_id FROM contact_list WHERE list_id IN ('exclude_list'))\n  AND contact_id @@@ paradedb.boolean(...)\nGROUP BY contact_job_title\nORDER BY doc_count DESC LIMIT 10;\n```\n\nAfter this PR, this shape pushes down to a single `Custom Scan (ParadeDB\nAggregate Scan)` node and returns correct results in both NULL-bearing\nand non-NULL inner cases.\n\n## Coverage\n\n| Query shape | Result |\n\n|----------------------------------------------|-------------------------------------|\n| `IN (SELECT ...)` PG-pulled-up | Pushed down (Semi) |\n| `EXISTS / NOT EXISTS` | Pushed down (Semi/Anti) |\n| Single-col `IN` un-pulled-up | Lifted to Semi, pushed down |\n| Single-col `NOT IN`, no NULL inner | Lifted to null-aware Anti |\n| Single-col `NOT IN`, NULL inner | Lifted, returns 0 rows |\n| Multi-col `NOT IN` / `IN` | Declines cleanly, PG fallback |\n| OR-nested SubPlan | Declines cleanly, PG fallback |\n\n## What changed\n\nPre-PR the agg-on-join walker bailed on Semi/Anti shapes with one of:\n`unexpected node type T_FromExpr in join tree`, `aggregate-on-join does\nnot support Semi/Anti JOIN`, or `Aggregate-on-join does not support Anti\nJOIN`. Separately, un-pulled-up `IN`/`NOT IN` SubPlans in\n`baserestrictinfo` were silently dropped by the per-RI `extract_quals`\nloop, producing wrong row counts when push-down succeeded.\n\n**Walker / accept-list.** `build_relnode_from_node` recognizes\n`T_FromExpr` (the post-pull-up parse-tree shape PG produces) and\nrecurses into `build_relnode_from_fromexpr`. `build_join_node` extends\nto `Semi`/`Anti`/`RightSemi`/`RightAnti`; all four are unconditionally\nsafe for aggregate pushdown because they never project the non-preserved\nside. The translator's dead `JoinTypeAllowList::EquiOnly` enum is\ndropped.\n\n**SubPlan lifting.** `build_scan_node` classifies `baserestrictinfo`\ninto search predicates / top-level SubPlans / OR-nested SubPlans. Search\npredicates batch into one strict `extract_quals` call (no silent drop).\nOR-nested SubPlans decline upfront. Top-level SubPlans lift via shared\n`wrap_with_semi_anti`, which now returns `Result<RelNode, String>`;\nevery former silent-skip path returns Err with a site-specific reason.\nBoth callers (new agg caller, existing JoinScan caller) propagate to a\nclean decline. Side-effect: closes a latent silent-drop window in\nJoinScan non-LIMIT queries that `is_limit_pushdown_safe` only caught for\nLIMIT.\n\n**Null-aware NOT IN.** `JoinType::Anti` becomes a struct variant `Anti {\nnull_aware: bool }`. The flag lives on the variant rather than as a\nseparate `JoinNode` field, so `(JoinType::Inner, null_aware: true)` is\nunrepresentable in the type system. `wrap_with_semi_anti` constructs\n`Anti { null_aware: is_anti }` for `NOT IN` lifts.\n`build_null_aware_anti_join` lowers to `LogicalPlan::Join` with\n`null_equality=NullEqualsNothing` and `null_aware=true`. DataFusion's\n`HashJoinExec` then emits zero rows when the probe (inner) side has any\nNULL, matching SQL three-valued logic.\n\n**plan_position-stored targetlist refs.** Every agg-on-join targetlist\nref (`JoinGroupColumn`, `JoinAggColRef`, `AggOrderByEntry`,\n`FilterExpr::ColumnRef`) carries a `plan_position` resolved once at\nextraction time against the just-built `RelNode` tree; execution-time\ncolumn binding is a `plan_position` lookup. `rti` is only unique within\na single `PlannerInfo`, so post-lift trees that mix sources from\nsub-PlannerInfos (e.g. SubPlans lifted by `wrap_with_semi_anti`) need a\n`PlannerRootId` to disambiguate. Three new shared `RelNode` primitives\nback this and unify with how JoinScan already addresses output columns:\n`source_with(root_id, rti, attno)`, `plan_position(root_id, rti,\nattno)`, `source_at_plan_position(plan_position)`. The FILTER build\ncontext bundles `plan` + `outer_root_id` into\n`Option<FilterPlanResolution>` so the two can't go out of sync.\n\n**Executor plumbing.** `ExprContext` + `PlanState` are threaded from the\nexecutor's runtime into each per-relation `PgSearchTableProvider`.\nHeapFilter queries (runtime expressions like `=` on a `pdb.literal`-cast\ncolumn) need a live evaluation context. Skip the `ExecAssignExprContext`\nallocation under `EXEC_FLAG_EXPLAIN_ONLY`.\n\n## DataFusion null-aware single-column limitation\n\nDataFusion 53.1.0's null-aware mode is restricted to a single-column\nequi-key. The validation in `HashJoinExec::build` rejects multi-column\nnull-aware:\n\n```rust\nif exec.null_aware && on.len() != 1 {\n    return plan_err!(\"null_aware anti join only supports single column join key, got {} columns\", on.len());\n}\n```\n\nThe runtime stream code only inspects `state.values[0]` and\n`left_data.values()[0]`. Multi-column `NOT IN` therefore can't ride the\nnull-aware fast path; this PR declines pushdown and lets PG's\n`nodeSubplan.c::ExecHashSubPlan` handle them.\n\n## Why this works without a `datafusion-proto` patch\n\nSister PR #5006 noted that `datafusion-proto 53.1.0` is missing\n`null_aware` from the `LogicalPlan::Join` proto schema (oversight in\n[apache/datafusion#19635](https://github.com/apache/datafusion/pull/19635);\nadded everywhere except the logical Join proto). This bites consumers\nthat round-trip `LogicalPlan` through the proto codec.\n\n**The agg-on-join path is unaffected.** The agg executor builds a\n`LogicalPlan` in `build_join_aggregate_plan`, hands it to\n`build_physical_plan` in the same Rust process, and runs the physical\nplan via `physical_plan.execute(...)`. No proto serialization. The\n`null_aware` flag travels purely through Rust struct fields from\nconstruction to execution. The proto bug only matters for the JoinScan\npath (which serializes its `LogicalPlan` for parallel leader/worker IPC)\nand is tracked separately in #5006.\n\n## Test plan\n\n`aggregate_join_semi_anti.sql` - six tests covering the full feature\nsurface:\n\n- **Test 1**: `IN (SELECT ...)` pulls up to Semi -> AggregateScan\n- **Test 2**: `EXISTS / NOT EXISTS` -> AggregateScan\n- **Test 3**: single-column `NOT IN` un-pulled-up -> null-aware Anti\nlift, AggregateScan\n- **Test 4**: parity with `enable_aggregate_custom_scan = off` for Test\n3\n- **Test 5**: multi-column `(a,b) NOT IN (SELECT x,y FROM t)` declines\ncleanly with a precise WARNING; PG plan runs; result matches PG\ncustom-scan-OFF\n- **Test 6**: single-column `NOT IN` with a NULL-bearing inner ->\nAggregateScan returns zero rows (SQL three-valued logic), parity with PG\ncustom-scan-OFF, plus a sanity check that removing the NULL inner row\nmakes the query return non-zero rows (guards against trivially passing\nwith zero rows for the wrong reason)\n\nAll other `aggregate_join_*` and `join_*` regress tests pass on PG 18\n(`cargo pgrx regress`); `cargo check` + `cargo clippy -- -D warnings`\nclean.\n\nRefs #4911. Sister PR #5006 covers the JoinScan-side end-to-end via the\nproto fork (separate dependency).",
          "timestamp": "2026-05-13T11:17:13+05:30",
          "tree_id": "d71839d2438c950c53328948b31766398e213d87",
          "url": "https://github.com/paradedb/paradedb/commit/fa4b7613b7a49e0a05075ef635fac3a6f677fd31"
        },
        "date": 1778652977666,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - tps",
            "value": 129.8678466279457,
            "unit": "median tps",
            "extra": "avg tps: 130.19292060412968, max tps: 148.5863380976672, count: 54628"
          },
          {
            "name": "Columnar Scan - Primary - tps",
            "value": 487.90884886202275,
            "unit": "median tps",
            "extra": "avg tps: 489.3267296455401, max tps: 688.1652748189458, count: 54628"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3213.530345015931,
            "unit": "median tps",
            "extra": "avg tps: 3208.764480310673, max tps: 3280.2997134757006, count: 54628"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 411.65820266486173,
            "unit": "median tps",
            "extra": "avg tps: 413.9135304056252, max tps: 506.76672944661834, count: 54628"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 2956.8481495389115,
            "unit": "median tps",
            "extra": "avg tps: 2962.372728897761, max tps: 3009.5235018983376, count: 109256"
          },
          {
            "name": "Normal Scan - Primary - tps",
            "value": 492.96785630113584,
            "unit": "median tps",
            "extra": "avg tps: 493.6472955374516, max tps: 669.161165660014, count: 54628"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 2132.229286570798,
            "unit": "median tps",
            "extra": "avg tps: 2124.776321425405, max tps: 2143.3316246477543, count: 54628"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 73.92893171895703,
            "unit": "median tps",
            "extra": "avg tps: 90.28874661273329, max tps: 373.329676361748, count: 54628"
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
      },
      {
        "commit": {
          "author": {
            "email": "mdashti@gmail.com",
            "name": "Moe",
            "username": "mdashti"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "6bdea0d414fb563f086ddfe56712b2960d200626",
          "message": "feat(mpp): AggregateScan in-process MPP via custom shm_mq transport (#4988)\n\n# Ticket(s) Closed\n\n- Closes #4152\n\n## What\n\nAdds an MPP execution path for AggregateScan that runs\njoin-with-aggregate shapes inside Postgres parallel-worker processes via\na custom transport on top of `paradedb/datafusion-distributed`. Default\noff behind `paradedb.enable_mpp` (minimum `mpp_worker_count = 3`). Other\nshapes and the JoinScan path are unchanged.\n\n## Why\n\nSingle-process DataFusion bottlenecks on join-with-aggregate at scale.\nDistributing the producer fragment across PG parallel workers gives us\n1.55–1.92× speedup on the 25M `aggregate_join_groupby` bench at N=2/4\nwithout leaving the embedded model — every worker is still a real PG\nprocess with its own snapshot, no gRPC, no extra daemon.\n\n## How\n\n- Leader builds the logical plan and stashes it in a DSM segment\nalongside an N-way `shm_mq` mesh.\n- Workers attach, deserialize, and re-plan with the same `SessionState`.\nIdentical inputs ⇒ structurally identical physical plans on every\nworker, so we don't need to serialize physical subplans.\n- Each worker runs its producer fragment and pushes batches through its\noutbound queue. Leader runs `NetworkShuffleExec` + final aggregate and\nreturns rows to the client. Leader is consumer-only in this iteration.\n- Build side (non-partitioning sources like `HashJoinExec(CollectLeft)`)\nis split via DSM all-gather: each worker scans its 1/N slice, writes to\na per-worker DSM region, completion-flag barrier, then everyone reads\nevery slice. Build is fully parallel; no leader-side serial scan.\n- The fork emits the network operators (`NetworkShuffleExec`,\n`NetworkBroadcastExec`, `NetworkCoalesceExec`) and we register a custom\ntransport that short-circuits the gRPC dialer. The fork's in-process\ntwo-boundary planner distinguishes outer (worker → leader, N producers)\nfrom nested (single local producer) Network boundaries.\n\n## Reviewer's Guide\n\nSuggested reading order — most of the diff lives under\n`pg_search/src/postgres/customscan/mpp/`:\n\n1. `mpp/dsm.rs` — the DSM layout: header, queue mesh, build-cache\nregion. `compute_dsm_layout` is the math; `leader_init` /\n`worker_attach` are the unsafe FFI boundaries.\n2. `mpp/runtime.rs` — `MppMesh` (runtime handle), `ShmMqWorkerTransport`\n(the `WorkerTransport` impl the leader registers),\n`LocalExecWorkerTransport` (the worker-side stub for nested broadcasts),\n`MppWorkerResolver`.\n3. `mpp/transport.rs` — `DrainHandle` and the cooperative-pull\nprimitives. The drain runs inline on the backend thread because pgrx\n0.18 enforces single-threaded Postgres FFI.\n4. `mpp/glue.rs` — the public API the customscan calls:\n`estimate_dsm_size`, `leader_setup`, `worker_setup`. Thin wrappers\naround dsm/runtime.\n5. `mpp/exec.rs` — `run_producer_fragment` is the worker push loop.\n6. `aggregatescan/mod.rs` — the integration: `stash_mpp_plan_bytes`,\n`exec_mpp_worker`, `build_mpp_leader_session_context`, the\n`ParallelQueryCapable` impl, the `parallel_workers` clamp in\n`try_build_datafusion_aggregate_path`. Both leader and worker session\ncontexts call `with_distributed_in_process_mode(true)` explicitly.\n\nThe build-side all-gather lives in\n`aggregatescan/mod.rs::exec_mpp_worker` and `mpp/dsm.rs` (the cache\nregion). Read those together.\n\n## Tests\n\n- pgrx regression suite — new `mpp_aggregate.sql` covers correctness on\nthe join-with-aggregate shapes; existing `mpp_*` suites still pass.\n- 25M `aggregate_join_groupby` bench: byte-exact result vs serial\nDataFusion at N=2/4/8/10.\n- Build-side all-gather is exercised at all N in the regression suite\n(workers vs leader-only-writer paths).\n\n---------\n\nCo-authored-by: paradedb-bot <developers@paradedb.com>\nCo-authored-by: paradedb-github-app[bot] <282009505+paradedb-github-app[bot]@users.noreply.github.com>",
          "timestamp": "2026-05-12T16:47:00-07:00",
          "tree_id": "38dfa579bfcab58f4b6b66ddea91de57c32f5204",
          "url": "https://github.com/paradedb/paradedb/commit/6bdea0d414fb563f086ddfe56712b2960d200626"
        },
        "date": 1778631515642,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - cpu",
            "value": 9.248554,
            "unit": "median cpu",
            "extra": "avg cpu: 8.412968739345768, max cpu: 18.991098, count: 55221"
          },
          {
            "name": "Aggregate Custom Scan - Primary - mem",
            "value": 66.49609375,
            "unit": "median mem",
            "extra": "avg mem: 66.34479551484037, max mem: 77.93359375, count: 55221"
          },
          {
            "name": "Columnar Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 5.501234947942254, max cpu: 18.390804, count: 55221"
          },
          {
            "name": "Columnar Scan - Primary - mem",
            "value": 65.2734375,
            "unit": "median mem",
            "extra": "avg mem: 65.14815433383586, max mem: 76.66796875, count: 55221"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.640041464974658, max cpu: 9.302325, count: 55221"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 35.515625,
            "unit": "median mem",
            "extra": "avg mem: 35.249165568805346, max mem: 37.21875, count: 55221"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.692840964369078, max cpu: 9.275363, count: 55221"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 63.69921875,
            "unit": "median mem",
            "extra": "avg mem: 63.321956745395774, max mem: 75.22265625, count: 55221"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.650087296897771, max cpu: 9.302325, count: 110442"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 55.22265625,
            "unit": "median mem",
            "extra": "avg mem: 54.324137966647655, max mem: 70.53515625, count: 110442"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 1787,
            "unit": "median block_count",
            "extra": "avg block_count: 1795.385867695261, max block_count: 3189.0, count: 55221"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 12,
            "unit": "median segment_count",
            "extra": "avg segment_count: 11.678817841038736, max segment_count: 26.0, count: 55221"
          },
          {
            "name": "Normal Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 5.556890793457494, max cpu: 18.677044, count: 55221"
          },
          {
            "name": "Normal Scan - Primary - mem",
            "value": 64.94921875,
            "unit": "median mem",
            "extra": "avg mem: 64.880270866156, max mem: 76.44921875, count: 55221"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.663080551737546, max cpu: 9.275363, count: 55221"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 54.046875,
            "unit": "median mem",
            "extra": "avg mem: 53.777641983461905, max mem: 64.8515625, count: 55221"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 3.866798049303071, max cpu: 4.678363, count: 55221"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 55.8359375,
            "unit": "median mem",
            "extra": "avg mem: 55.76807750335018, max mem: 69.515625, count: 55221"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "james.sewell@gmail.com",
            "name": "James Sewell",
            "username": "jamessewell"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "8182eaf110c30cbefe008197caa40efa8b44f8e0",
          "message": "refactor: use existing FFHelper ctid cache instead of dedicated cache (#4905)\n\nFix a performance regression introduced in e0804b347 (#4765) which\nremoved ctid from SearchIndexScore and switched to lazy per-row\nresolution.\n\nPrior to #4765, ctid was resolved during result construction and carried\nin `SearchIndexScore` — no per-row fast-field lookups needed. #4765\nmoved ctid resolution to the consumption side (top_k.rs, normal.rs,\nscan.rs) using a single-entry `Option<(SegmentOrdinal, FFType)>` cache.\nWhen TopK results interleave across segments (sorted by score), every\nsegment transition re-opens the ctid column via `FastFieldReaders::u64\n-> DynamicColumnHandle::open -> BlockwiseLinearCodec::load`, which is\nvery expensive. Profiling showed 45% of total cycles spent in this\nre-open path.\n\nThe columnar scan path (`ColumnarExecState`) was unaffected — it already\nused `FFHelper`'s per-segment `OnceLock` ctid cache. This PR brings the\nremaining paths in line:\n\n- `scan.rs` uses its existing `Bm25ScanState.fast_fields` FFHelper\n- `normal.rs` and `top_k.rs` use a new `ctid_cache` FFHelper on\n`BaseScanState`\n\nEach segment's ctid column is opened at most once via `OnceLock`,\neliminating the thrashing. `FFHelper` has had this per-segment ctid\ncaching built in since cb78f0ca2 (Oct 2024).",
          "timestamp": "2026-05-13T12:18:31+12:00",
          "tree_id": "814e1da895eec41e0dfe3cbb5348bdb237811bf7",
          "url": "https://github.com/paradedb/paradedb/commit/8182eaf110c30cbefe008197caa40efa8b44f8e0"
        },
        "date": 1778633412584,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - cpu",
            "value": 9.275363,
            "unit": "median cpu",
            "extra": "avg cpu: 8.184210757695471, max cpu: 19.077902, count: 55114"
          },
          {
            "name": "Aggregate Custom Scan - Primary - mem",
            "value": 66.40234375,
            "unit": "median mem",
            "extra": "avg mem: 66.19309600044907, max mem: 77.23046875, count: 55114"
          },
          {
            "name": "Columnar Scan - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 5.591495673146632, max cpu: 18.897638, count: 55114"
          },
          {
            "name": "Columnar Scan - Primary - mem",
            "value": 65.08203125,
            "unit": "median mem",
            "extra": "avg mem: 64.81071293704866, max mem: 75.75390625, count: 55114"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.7371640980421725, max cpu: 9.347614, count: 55114"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 36.59765625,
            "unit": "median mem",
            "extra": "avg mem: 36.26604912091302, max mem: 38.46484375, count: 55114"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.873994065981994, max cpu: 9.467456, count: 55114"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 63.515625,
            "unit": "median mem",
            "extra": "avg mem: 62.99029199704249, max mem: 74.33984375, count: 55114"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.65669578333524, max cpu: 9.275363, count: 110228"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 62.90625,
            "unit": "median mem",
            "extra": "avg mem: 61.621889614594295, max mem: 73.8984375, count: 110228"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 1748,
            "unit": "median block_count",
            "extra": "avg block_count: 1756.3705592045578, max block_count: 3106.0, count: 55114"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 10,
            "unit": "median segment_count",
            "extra": "avg segment_count: 9.464346626991327, max segment_count: 16.0, count: 55114"
          },
          {
            "name": "Normal Scan - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 5.479926491940563, max cpu: 18.879055, count: 55114"
          },
          {
            "name": "Normal Scan - Primary - mem",
            "value": 64.7109375,
            "unit": "median mem",
            "extra": "avg mem: 64.53767120763327, max mem: 75.52734375, count: 55114"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.578715438909623, max cpu: 9.302325, count: 55114"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 54.18359375,
            "unit": "median mem",
            "extra": "avg mem: 57.855413679487064, max mem: 73.82421875, count: 55114"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 3.537875646704179, max cpu: 4.7244096, count: 55114"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 57.1171875,
            "unit": "median mem",
            "extra": "avg mem: 56.37168698062198, max mem: 68.78125, count: 55114"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mithun.cy@gmail.com",
            "name": "Mithun Chicklore Yogendra",
            "username": "mithuncy"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "fa4b7613b7a49e0a05075ef635fac3a6f677fd31",
          "message": "feat: agg-on-join end-to-end IN/NOT IN/EXISTS/NOT EXISTS with null-aware semantics (#5005)\n\n## Summary\n\nMakes the agg-on-join path handle `IN (SELECT ...)`, `NOT IN (SELECT\n...)`, `EXISTS (SELECT ...)`, and `NOT EXISTS (SELECT ...)` end-to-end,\nincluding the `NOT IN` against a NULL-bearing inner case, which is the\nhard one because of SQL's three-valued NULL logic.\n\nTarget query shape - aggregate over a join with IN/NOT IN sublinks plus\na BM25 search predicate:\n\n```sql\nSELECT contact_job_title, COUNT(*) AS doc_count\nFROM contacts\nWHERE contact_id IN     (SELECT ldf_id FROM contact_list WHERE list_id IN ('include_list'))\n  AND contact_id NOT IN (SELECT ldf_id FROM contact_list WHERE list_id IN ('exclude_list'))\n  AND contact_id @@@ paradedb.boolean(...)\nGROUP BY contact_job_title\nORDER BY doc_count DESC LIMIT 10;\n```\n\nAfter this PR, this shape pushes down to a single `Custom Scan (ParadeDB\nAggregate Scan)` node and returns correct results in both NULL-bearing\nand non-NULL inner cases.\n\n## Coverage\n\n| Query shape | Result |\n\n|----------------------------------------------|-------------------------------------|\n| `IN (SELECT ...)` PG-pulled-up | Pushed down (Semi) |\n| `EXISTS / NOT EXISTS` | Pushed down (Semi/Anti) |\n| Single-col `IN` un-pulled-up | Lifted to Semi, pushed down |\n| Single-col `NOT IN`, no NULL inner | Lifted to null-aware Anti |\n| Single-col `NOT IN`, NULL inner | Lifted, returns 0 rows |\n| Multi-col `NOT IN` / `IN` | Declines cleanly, PG fallback |\n| OR-nested SubPlan | Declines cleanly, PG fallback |\n\n## What changed\n\nPre-PR the agg-on-join walker bailed on Semi/Anti shapes with one of:\n`unexpected node type T_FromExpr in join tree`, `aggregate-on-join does\nnot support Semi/Anti JOIN`, or `Aggregate-on-join does not support Anti\nJOIN`. Separately, un-pulled-up `IN`/`NOT IN` SubPlans in\n`baserestrictinfo` were silently dropped by the per-RI `extract_quals`\nloop, producing wrong row counts when push-down succeeded.\n\n**Walker / accept-list.** `build_relnode_from_node` recognizes\n`T_FromExpr` (the post-pull-up parse-tree shape PG produces) and\nrecurses into `build_relnode_from_fromexpr`. `build_join_node` extends\nto `Semi`/`Anti`/`RightSemi`/`RightAnti`; all four are unconditionally\nsafe for aggregate pushdown because they never project the non-preserved\nside. The translator's dead `JoinTypeAllowList::EquiOnly` enum is\ndropped.\n\n**SubPlan lifting.** `build_scan_node` classifies `baserestrictinfo`\ninto search predicates / top-level SubPlans / OR-nested SubPlans. Search\npredicates batch into one strict `extract_quals` call (no silent drop).\nOR-nested SubPlans decline upfront. Top-level SubPlans lift via shared\n`wrap_with_semi_anti`, which now returns `Result<RelNode, String>`;\nevery former silent-skip path returns Err with a site-specific reason.\nBoth callers (new agg caller, existing JoinScan caller) propagate to a\nclean decline. Side-effect: closes a latent silent-drop window in\nJoinScan non-LIMIT queries that `is_limit_pushdown_safe` only caught for\nLIMIT.\n\n**Null-aware NOT IN.** `JoinType::Anti` becomes a struct variant `Anti {\nnull_aware: bool }`. The flag lives on the variant rather than as a\nseparate `JoinNode` field, so `(JoinType::Inner, null_aware: true)` is\nunrepresentable in the type system. `wrap_with_semi_anti` constructs\n`Anti { null_aware: is_anti }` for `NOT IN` lifts.\n`build_null_aware_anti_join` lowers to `LogicalPlan::Join` with\n`null_equality=NullEqualsNothing` and `null_aware=true`. DataFusion's\n`HashJoinExec` then emits zero rows when the probe (inner) side has any\nNULL, matching SQL three-valued logic.\n\n**plan_position-stored targetlist refs.** Every agg-on-join targetlist\nref (`JoinGroupColumn`, `JoinAggColRef`, `AggOrderByEntry`,\n`FilterExpr::ColumnRef`) carries a `plan_position` resolved once at\nextraction time against the just-built `RelNode` tree; execution-time\ncolumn binding is a `plan_position` lookup. `rti` is only unique within\na single `PlannerInfo`, so post-lift trees that mix sources from\nsub-PlannerInfos (e.g. SubPlans lifted by `wrap_with_semi_anti`) need a\n`PlannerRootId` to disambiguate. Three new shared `RelNode` primitives\nback this and unify with how JoinScan already addresses output columns:\n`source_with(root_id, rti, attno)`, `plan_position(root_id, rti,\nattno)`, `source_at_plan_position(plan_position)`. The FILTER build\ncontext bundles `plan` + `outer_root_id` into\n`Option<FilterPlanResolution>` so the two can't go out of sync.\n\n**Executor plumbing.** `ExprContext` + `PlanState` are threaded from the\nexecutor's runtime into each per-relation `PgSearchTableProvider`.\nHeapFilter queries (runtime expressions like `=` on a `pdb.literal`-cast\ncolumn) need a live evaluation context. Skip the `ExecAssignExprContext`\nallocation under `EXEC_FLAG_EXPLAIN_ONLY`.\n\n## DataFusion null-aware single-column limitation\n\nDataFusion 53.1.0's null-aware mode is restricted to a single-column\nequi-key. The validation in `HashJoinExec::build` rejects multi-column\nnull-aware:\n\n```rust\nif exec.null_aware && on.len() != 1 {\n    return plan_err!(\"null_aware anti join only supports single column join key, got {} columns\", on.len());\n}\n```\n\nThe runtime stream code only inspects `state.values[0]` and\n`left_data.values()[0]`. Multi-column `NOT IN` therefore can't ride the\nnull-aware fast path; this PR declines pushdown and lets PG's\n`nodeSubplan.c::ExecHashSubPlan` handle them.\n\n## Why this works without a `datafusion-proto` patch\n\nSister PR #5006 noted that `datafusion-proto 53.1.0` is missing\n`null_aware` from the `LogicalPlan::Join` proto schema (oversight in\n[apache/datafusion#19635](https://github.com/apache/datafusion/pull/19635);\nadded everywhere except the logical Join proto). This bites consumers\nthat round-trip `LogicalPlan` through the proto codec.\n\n**The agg-on-join path is unaffected.** The agg executor builds a\n`LogicalPlan` in `build_join_aggregate_plan`, hands it to\n`build_physical_plan` in the same Rust process, and runs the physical\nplan via `physical_plan.execute(...)`. No proto serialization. The\n`null_aware` flag travels purely through Rust struct fields from\nconstruction to execution. The proto bug only matters for the JoinScan\npath (which serializes its `LogicalPlan` for parallel leader/worker IPC)\nand is tracked separately in #5006.\n\n## Test plan\n\n`aggregate_join_semi_anti.sql` - six tests covering the full feature\nsurface:\n\n- **Test 1**: `IN (SELECT ...)` pulls up to Semi -> AggregateScan\n- **Test 2**: `EXISTS / NOT EXISTS` -> AggregateScan\n- **Test 3**: single-column `NOT IN` un-pulled-up -> null-aware Anti\nlift, AggregateScan\n- **Test 4**: parity with `enable_aggregate_custom_scan = off` for Test\n3\n- **Test 5**: multi-column `(a,b) NOT IN (SELECT x,y FROM t)` declines\ncleanly with a precise WARNING; PG plan runs; result matches PG\ncustom-scan-OFF\n- **Test 6**: single-column `NOT IN` with a NULL-bearing inner ->\nAggregateScan returns zero rows (SQL three-valued logic), parity with PG\ncustom-scan-OFF, plus a sanity check that removing the NULL inner row\nmakes the query return non-zero rows (guards against trivially passing\nwith zero rows for the wrong reason)\n\nAll other `aggregate_join_*` and `join_*` regress tests pass on PG 18\n(`cargo pgrx regress`); `cargo check` + `cargo clippy -- -D warnings`\nclean.\n\nRefs #4911. Sister PR #5006 covers the JoinScan-side end-to-end via the\nproto fork (separate dependency).",
          "timestamp": "2026-05-13T11:17:13+05:30",
          "tree_id": "d71839d2438c950c53328948b31766398e213d87",
          "url": "https://github.com/paradedb/paradedb/commit/fa4b7613b7a49e0a05075ef635fac3a6f677fd31"
        },
        "date": 1778653011814,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - cpu",
            "value": 9.284333,
            "unit": "median cpu",
            "extra": "avg cpu: 8.62368521102075, max cpu: 23.738873, count: 54628"
          },
          {
            "name": "Aggregate Custom Scan - Primary - mem",
            "value": 66.89453125,
            "unit": "median mem",
            "extra": "avg mem: 66.73399773238998, max mem: 77.80078125, count: 54628"
          },
          {
            "name": "Columnar Scan - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 5.779802277372747, max cpu: 18.953604, count: 54628"
          },
          {
            "name": "Columnar Scan - Primary - mem",
            "value": 65.41015625,
            "unit": "median mem",
            "extra": "avg mem: 65.20983093891869, max mem: 76.328125, count: 54628"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.7182650139183115, max cpu: 9.384164, count: 54628"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 34.859375,
            "unit": "median mem",
            "extra": "avg mem: 34.99356149490646, max mem: 36.76953125, count: 54628"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.739886271630462, max cpu: 9.476802, count: 54628"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 64.01171875,
            "unit": "median mem",
            "extra": "avg mem: 63.528140545484916, max mem: 75.15625, count: 54628"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.632345036615748, max cpu: 9.320388, count: 109256"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 61.4140625,
            "unit": "median mem",
            "extra": "avg mem: 59.7296989481699, max mem: 73.09375, count: 109256"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 1758,
            "unit": "median block_count",
            "extra": "avg block_count: 1759.1594054331113, max block_count: 3102.0, count: 54628"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 13,
            "unit": "median segment_count",
            "extra": "avg segment_count: 13.48724097532401, max segment_count: 29.0, count: 54628"
          },
          {
            "name": "Normal Scan - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 5.709557826460843, max cpu: 18.972332, count: 54628"
          },
          {
            "name": "Normal Scan - Primary - mem",
            "value": 65.14453125,
            "unit": "median mem",
            "extra": "avg mem: 65.00077412796551, max mem: 76.125, count: 54628"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.64508321384006, max cpu: 9.338522, count: 54628"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 62.37890625,
            "unit": "median mem",
            "extra": "avg mem: 61.55401691325877, max mem: 72.9765625, count: 54628"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.000258767535239, max cpu: 4.6829267, count: 54628"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 57.15234375,
            "unit": "median mem",
            "extra": "avg mem: 56.32322717116222, max mem: 68.9765625, count: 54628"
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
        "date": 1778525357500,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 7.810530906296144,
            "unit": "median tps",
            "extra": "avg tps: 6.69194846246484, max tps: 10.177023253883151, count: 57763"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.4913965173316575,
            "unit": "median tps",
            "extra": "avg tps: 4.925843773324201, max tps: 6.172012195202868, count: 57763"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mdashti@gmail.com",
            "name": "Moe",
            "username": "mdashti"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "6bdea0d414fb563f086ddfe56712b2960d200626",
          "message": "feat(mpp): AggregateScan in-process MPP via custom shm_mq transport (#4988)\n\n# Ticket(s) Closed\n\n- Closes #4152\n\n## What\n\nAdds an MPP execution path for AggregateScan that runs\njoin-with-aggregate shapes inside Postgres parallel-worker processes via\na custom transport on top of `paradedb/datafusion-distributed`. Default\noff behind `paradedb.enable_mpp` (minimum `mpp_worker_count = 3`). Other\nshapes and the JoinScan path are unchanged.\n\n## Why\n\nSingle-process DataFusion bottlenecks on join-with-aggregate at scale.\nDistributing the producer fragment across PG parallel workers gives us\n1.55–1.92× speedup on the 25M `aggregate_join_groupby` bench at N=2/4\nwithout leaving the embedded model — every worker is still a real PG\nprocess with its own snapshot, no gRPC, no extra daemon.\n\n## How\n\n- Leader builds the logical plan and stashes it in a DSM segment\nalongside an N-way `shm_mq` mesh.\n- Workers attach, deserialize, and re-plan with the same `SessionState`.\nIdentical inputs ⇒ structurally identical physical plans on every\nworker, so we don't need to serialize physical subplans.\n- Each worker runs its producer fragment and pushes batches through its\noutbound queue. Leader runs `NetworkShuffleExec` + final aggregate and\nreturns rows to the client. Leader is consumer-only in this iteration.\n- Build side (non-partitioning sources like `HashJoinExec(CollectLeft)`)\nis split via DSM all-gather: each worker scans its 1/N slice, writes to\na per-worker DSM region, completion-flag barrier, then everyone reads\nevery slice. Build is fully parallel; no leader-side serial scan.\n- The fork emits the network operators (`NetworkShuffleExec`,\n`NetworkBroadcastExec`, `NetworkCoalesceExec`) and we register a custom\ntransport that short-circuits the gRPC dialer. The fork's in-process\ntwo-boundary planner distinguishes outer (worker → leader, N producers)\nfrom nested (single local producer) Network boundaries.\n\n## Reviewer's Guide\n\nSuggested reading order — most of the diff lives under\n`pg_search/src/postgres/customscan/mpp/`:\n\n1. `mpp/dsm.rs` — the DSM layout: header, queue mesh, build-cache\nregion. `compute_dsm_layout` is the math; `leader_init` /\n`worker_attach` are the unsafe FFI boundaries.\n2. `mpp/runtime.rs` — `MppMesh` (runtime handle), `ShmMqWorkerTransport`\n(the `WorkerTransport` impl the leader registers),\n`LocalExecWorkerTransport` (the worker-side stub for nested broadcasts),\n`MppWorkerResolver`.\n3. `mpp/transport.rs` — `DrainHandle` and the cooperative-pull\nprimitives. The drain runs inline on the backend thread because pgrx\n0.18 enforces single-threaded Postgres FFI.\n4. `mpp/glue.rs` — the public API the customscan calls:\n`estimate_dsm_size`, `leader_setup`, `worker_setup`. Thin wrappers\naround dsm/runtime.\n5. `mpp/exec.rs` — `run_producer_fragment` is the worker push loop.\n6. `aggregatescan/mod.rs` — the integration: `stash_mpp_plan_bytes`,\n`exec_mpp_worker`, `build_mpp_leader_session_context`, the\n`ParallelQueryCapable` impl, the `parallel_workers` clamp in\n`try_build_datafusion_aggregate_path`. Both leader and worker session\ncontexts call `with_distributed_in_process_mode(true)` explicitly.\n\nThe build-side all-gather lives in\n`aggregatescan/mod.rs::exec_mpp_worker` and `mpp/dsm.rs` (the cache\nregion). Read those together.\n\n## Tests\n\n- pgrx regression suite — new `mpp_aggregate.sql` covers correctness on\nthe join-with-aggregate shapes; existing `mpp_*` suites still pass.\n- 25M `aggregate_join_groupby` bench: byte-exact result vs serial\nDataFusion at N=2/4/8/10.\n- Build-side all-gather is exercised at all N in the regression suite\n(workers vs leader-only-writer paths).\n\n---------\n\nCo-authored-by: paradedb-bot <developers@paradedb.com>\nCo-authored-by: paradedb-github-app[bot] <282009505+paradedb-github-app[bot]@users.noreply.github.com>",
          "timestamp": "2026-05-12T16:47:00-07:00",
          "tree_id": "38dfa579bfcab58f4b6b66ddea91de57c32f5204",
          "url": "https://github.com/paradedb/paradedb/commit/6bdea0d414fb563f086ddfe56712b2960d200626"
        },
        "date": 1778632184064,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 7.957131236837499,
            "unit": "median tps",
            "extra": "avg tps: 6.782484418117509, max tps: 10.447332758528859, count: 57761"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.413301286493733,
            "unit": "median tps",
            "extra": "avg tps: 4.857942720922933, max tps: 6.032761259960225, count: 57761"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "james.sewell@gmail.com",
            "name": "James Sewell",
            "username": "jamessewell"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "8182eaf110c30cbefe008197caa40efa8b44f8e0",
          "message": "refactor: use existing FFHelper ctid cache instead of dedicated cache (#4905)\n\nFix a performance regression introduced in e0804b347 (#4765) which\nremoved ctid from SearchIndexScore and switched to lazy per-row\nresolution.\n\nPrior to #4765, ctid was resolved during result construction and carried\nin `SearchIndexScore` — no per-row fast-field lookups needed. #4765\nmoved ctid resolution to the consumption side (top_k.rs, normal.rs,\nscan.rs) using a single-entry `Option<(SegmentOrdinal, FFType)>` cache.\nWhen TopK results interleave across segments (sorted by score), every\nsegment transition re-opens the ctid column via `FastFieldReaders::u64\n-> DynamicColumnHandle::open -> BlockwiseLinearCodec::load`, which is\nvery expensive. Profiling showed 45% of total cycles spent in this\nre-open path.\n\nThe columnar scan path (`ColumnarExecState`) was unaffected — it already\nused `FFHelper`'s per-segment `OnceLock` ctid cache. This PR brings the\nremaining paths in line:\n\n- `scan.rs` uses its existing `Bm25ScanState.fast_fields` FFHelper\n- `normal.rs` and `top_k.rs` use a new `ctid_cache` FFHelper on\n`BaseScanState`\n\nEach segment's ctid column is opened at most once via `OnceLock`,\neliminating the thrashing. `FFHelper` has had this per-segment ctid\ncaching built in since cb78f0ca2 (Oct 2024).",
          "timestamp": "2026-05-13T12:18:31+12:00",
          "tree_id": "814e1da895eec41e0dfe3cbb5348bdb237811bf7",
          "url": "https://github.com/paradedb/paradedb/commit/8182eaf110c30cbefe008197caa40efa8b44f8e0"
        },
        "date": 1778634083422,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 7.856309421069924,
            "unit": "median tps",
            "extra": "avg tps: 6.669163171316379, max tps: 10.17512328120111, count: 57375"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 4.950990082926696,
            "unit": "median tps",
            "extra": "avg tps: 4.474101566994755, max tps: 5.503059961432824, count: 57375"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mithun.cy@gmail.com",
            "name": "Mithun Chicklore Yogendra",
            "username": "mithuncy"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "fa4b7613b7a49e0a05075ef635fac3a6f677fd31",
          "message": "feat: agg-on-join end-to-end IN/NOT IN/EXISTS/NOT EXISTS with null-aware semantics (#5005)\n\n## Summary\n\nMakes the agg-on-join path handle `IN (SELECT ...)`, `NOT IN (SELECT\n...)`, `EXISTS (SELECT ...)`, and `NOT EXISTS (SELECT ...)` end-to-end,\nincluding the `NOT IN` against a NULL-bearing inner case, which is the\nhard one because of SQL's three-valued NULL logic.\n\nTarget query shape - aggregate over a join with IN/NOT IN sublinks plus\na BM25 search predicate:\n\n```sql\nSELECT contact_job_title, COUNT(*) AS doc_count\nFROM contacts\nWHERE contact_id IN     (SELECT ldf_id FROM contact_list WHERE list_id IN ('include_list'))\n  AND contact_id NOT IN (SELECT ldf_id FROM contact_list WHERE list_id IN ('exclude_list'))\n  AND contact_id @@@ paradedb.boolean(...)\nGROUP BY contact_job_title\nORDER BY doc_count DESC LIMIT 10;\n```\n\nAfter this PR, this shape pushes down to a single `Custom Scan (ParadeDB\nAggregate Scan)` node and returns correct results in both NULL-bearing\nand non-NULL inner cases.\n\n## Coverage\n\n| Query shape | Result |\n\n|----------------------------------------------|-------------------------------------|\n| `IN (SELECT ...)` PG-pulled-up | Pushed down (Semi) |\n| `EXISTS / NOT EXISTS` | Pushed down (Semi/Anti) |\n| Single-col `IN` un-pulled-up | Lifted to Semi, pushed down |\n| Single-col `NOT IN`, no NULL inner | Lifted to null-aware Anti |\n| Single-col `NOT IN`, NULL inner | Lifted, returns 0 rows |\n| Multi-col `NOT IN` / `IN` | Declines cleanly, PG fallback |\n| OR-nested SubPlan | Declines cleanly, PG fallback |\n\n## What changed\n\nPre-PR the agg-on-join walker bailed on Semi/Anti shapes with one of:\n`unexpected node type T_FromExpr in join tree`, `aggregate-on-join does\nnot support Semi/Anti JOIN`, or `Aggregate-on-join does not support Anti\nJOIN`. Separately, un-pulled-up `IN`/`NOT IN` SubPlans in\n`baserestrictinfo` were silently dropped by the per-RI `extract_quals`\nloop, producing wrong row counts when push-down succeeded.\n\n**Walker / accept-list.** `build_relnode_from_node` recognizes\n`T_FromExpr` (the post-pull-up parse-tree shape PG produces) and\nrecurses into `build_relnode_from_fromexpr`. `build_join_node` extends\nto `Semi`/`Anti`/`RightSemi`/`RightAnti`; all four are unconditionally\nsafe for aggregate pushdown because they never project the non-preserved\nside. The translator's dead `JoinTypeAllowList::EquiOnly` enum is\ndropped.\n\n**SubPlan lifting.** `build_scan_node` classifies `baserestrictinfo`\ninto search predicates / top-level SubPlans / OR-nested SubPlans. Search\npredicates batch into one strict `extract_quals` call (no silent drop).\nOR-nested SubPlans decline upfront. Top-level SubPlans lift via shared\n`wrap_with_semi_anti`, which now returns `Result<RelNode, String>`;\nevery former silent-skip path returns Err with a site-specific reason.\nBoth callers (new agg caller, existing JoinScan caller) propagate to a\nclean decline. Side-effect: closes a latent silent-drop window in\nJoinScan non-LIMIT queries that `is_limit_pushdown_safe` only caught for\nLIMIT.\n\n**Null-aware NOT IN.** `JoinType::Anti` becomes a struct variant `Anti {\nnull_aware: bool }`. The flag lives on the variant rather than as a\nseparate `JoinNode` field, so `(JoinType::Inner, null_aware: true)` is\nunrepresentable in the type system. `wrap_with_semi_anti` constructs\n`Anti { null_aware: is_anti }` for `NOT IN` lifts.\n`build_null_aware_anti_join` lowers to `LogicalPlan::Join` with\n`null_equality=NullEqualsNothing` and `null_aware=true`. DataFusion's\n`HashJoinExec` then emits zero rows when the probe (inner) side has any\nNULL, matching SQL three-valued logic.\n\n**plan_position-stored targetlist refs.** Every agg-on-join targetlist\nref (`JoinGroupColumn`, `JoinAggColRef`, `AggOrderByEntry`,\n`FilterExpr::ColumnRef`) carries a `plan_position` resolved once at\nextraction time against the just-built `RelNode` tree; execution-time\ncolumn binding is a `plan_position` lookup. `rti` is only unique within\na single `PlannerInfo`, so post-lift trees that mix sources from\nsub-PlannerInfos (e.g. SubPlans lifted by `wrap_with_semi_anti`) need a\n`PlannerRootId` to disambiguate. Three new shared `RelNode` primitives\nback this and unify with how JoinScan already addresses output columns:\n`source_with(root_id, rti, attno)`, `plan_position(root_id, rti,\nattno)`, `source_at_plan_position(plan_position)`. The FILTER build\ncontext bundles `plan` + `outer_root_id` into\n`Option<FilterPlanResolution>` so the two can't go out of sync.\n\n**Executor plumbing.** `ExprContext` + `PlanState` are threaded from the\nexecutor's runtime into each per-relation `PgSearchTableProvider`.\nHeapFilter queries (runtime expressions like `=` on a `pdb.literal`-cast\ncolumn) need a live evaluation context. Skip the `ExecAssignExprContext`\nallocation under `EXEC_FLAG_EXPLAIN_ONLY`.\n\n## DataFusion null-aware single-column limitation\n\nDataFusion 53.1.0's null-aware mode is restricted to a single-column\nequi-key. The validation in `HashJoinExec::build` rejects multi-column\nnull-aware:\n\n```rust\nif exec.null_aware && on.len() != 1 {\n    return plan_err!(\"null_aware anti join only supports single column join key, got {} columns\", on.len());\n}\n```\n\nThe runtime stream code only inspects `state.values[0]` and\n`left_data.values()[0]`. Multi-column `NOT IN` therefore can't ride the\nnull-aware fast path; this PR declines pushdown and lets PG's\n`nodeSubplan.c::ExecHashSubPlan` handle them.\n\n## Why this works without a `datafusion-proto` patch\n\nSister PR #5006 noted that `datafusion-proto 53.1.0` is missing\n`null_aware` from the `LogicalPlan::Join` proto schema (oversight in\n[apache/datafusion#19635](https://github.com/apache/datafusion/pull/19635);\nadded everywhere except the logical Join proto). This bites consumers\nthat round-trip `LogicalPlan` through the proto codec.\n\n**The agg-on-join path is unaffected.** The agg executor builds a\n`LogicalPlan` in `build_join_aggregate_plan`, hands it to\n`build_physical_plan` in the same Rust process, and runs the physical\nplan via `physical_plan.execute(...)`. No proto serialization. The\n`null_aware` flag travels purely through Rust struct fields from\nconstruction to execution. The proto bug only matters for the JoinScan\npath (which serializes its `LogicalPlan` for parallel leader/worker IPC)\nand is tracked separately in #5006.\n\n## Test plan\n\n`aggregate_join_semi_anti.sql` - six tests covering the full feature\nsurface:\n\n- **Test 1**: `IN (SELECT ...)` pulls up to Semi -> AggregateScan\n- **Test 2**: `EXISTS / NOT EXISTS` -> AggregateScan\n- **Test 3**: single-column `NOT IN` un-pulled-up -> null-aware Anti\nlift, AggregateScan\n- **Test 4**: parity with `enable_aggregate_custom_scan = off` for Test\n3\n- **Test 5**: multi-column `(a,b) NOT IN (SELECT x,y FROM t)` declines\ncleanly with a precise WARNING; PG plan runs; result matches PG\ncustom-scan-OFF\n- **Test 6**: single-column `NOT IN` with a NULL-bearing inner ->\nAggregateScan returns zero rows (SQL three-valued logic), parity with PG\ncustom-scan-OFF, plus a sanity check that removing the NULL inner row\nmakes the query return non-zero rows (guards against trivially passing\nwith zero rows for the wrong reason)\n\nAll other `aggregate_join_*` and `join_*` regress tests pass on PG 18\n(`cargo pgrx regress`); `cargo check` + `cargo clippy -- -D warnings`\nclean.\n\nRefs #4911. Sister PR #5006 covers the JoinScan-side end-to-end via the\nproto fork (separate dependency).",
          "timestamp": "2026-05-13T11:17:13+05:30",
          "tree_id": "d71839d2438c950c53328948b31766398e213d87",
          "url": "https://github.com/paradedb/paradedb/commit/fa4b7613b7a49e0a05075ef635fac3a6f677fd31"
        },
        "date": 1778653680381,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 7.9106820795180255,
            "unit": "median tps",
            "extra": "avg tps: 6.749231794097677, max tps: 10.282443432639367, count: 57775"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.071670097862815,
            "unit": "median tps",
            "extra": "avg tps: 4.580165738903883, max tps: 5.628719409532594, count: 57775"
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
        "date": 1778525408584,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.27837,
            "unit": "median cpu",
            "extra": "avg cpu: 20.980733205963524, max cpu: 42.899704, count: 57763"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 235.1328125,
            "unit": "median mem",
            "extra": "avg mem: 234.98101780118762, max mem: 236.60546875, count: 57763"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.414635,
            "unit": "median cpu",
            "extra": "avg cpu: 22.45841173996253, max cpu: 33.432835, count: 57763"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 176.76953125,
            "unit": "median mem",
            "extra": "avg mem: 176.8379225104089, max mem: 177.68359375, count: 57763"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 34570,
            "unit": "median block_count",
            "extra": "avg block_count: 33794.764018489346, max block_count: 36763.0, count: 57763"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 79,
            "unit": "median segment_count",
            "extra": "avg segment_count: 81.6345584543739, max segment_count: 130.0, count: 57763"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mdashti@gmail.com",
            "name": "Moe",
            "username": "mdashti"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "6bdea0d414fb563f086ddfe56712b2960d200626",
          "message": "feat(mpp): AggregateScan in-process MPP via custom shm_mq transport (#4988)\n\n# Ticket(s) Closed\n\n- Closes #4152\n\n## What\n\nAdds an MPP execution path for AggregateScan that runs\njoin-with-aggregate shapes inside Postgres parallel-worker processes via\na custom transport on top of `paradedb/datafusion-distributed`. Default\noff behind `paradedb.enable_mpp` (minimum `mpp_worker_count = 3`). Other\nshapes and the JoinScan path are unchanged.\n\n## Why\n\nSingle-process DataFusion bottlenecks on join-with-aggregate at scale.\nDistributing the producer fragment across PG parallel workers gives us\n1.55–1.92× speedup on the 25M `aggregate_join_groupby` bench at N=2/4\nwithout leaving the embedded model — every worker is still a real PG\nprocess with its own snapshot, no gRPC, no extra daemon.\n\n## How\n\n- Leader builds the logical plan and stashes it in a DSM segment\nalongside an N-way `shm_mq` mesh.\n- Workers attach, deserialize, and re-plan with the same `SessionState`.\nIdentical inputs ⇒ structurally identical physical plans on every\nworker, so we don't need to serialize physical subplans.\n- Each worker runs its producer fragment and pushes batches through its\noutbound queue. Leader runs `NetworkShuffleExec` + final aggregate and\nreturns rows to the client. Leader is consumer-only in this iteration.\n- Build side (non-partitioning sources like `HashJoinExec(CollectLeft)`)\nis split via DSM all-gather: each worker scans its 1/N slice, writes to\na per-worker DSM region, completion-flag barrier, then everyone reads\nevery slice. Build is fully parallel; no leader-side serial scan.\n- The fork emits the network operators (`NetworkShuffleExec`,\n`NetworkBroadcastExec`, `NetworkCoalesceExec`) and we register a custom\ntransport that short-circuits the gRPC dialer. The fork's in-process\ntwo-boundary planner distinguishes outer (worker → leader, N producers)\nfrom nested (single local producer) Network boundaries.\n\n## Reviewer's Guide\n\nSuggested reading order — most of the diff lives under\n`pg_search/src/postgres/customscan/mpp/`:\n\n1. `mpp/dsm.rs` — the DSM layout: header, queue mesh, build-cache\nregion. `compute_dsm_layout` is the math; `leader_init` /\n`worker_attach` are the unsafe FFI boundaries.\n2. `mpp/runtime.rs` — `MppMesh` (runtime handle), `ShmMqWorkerTransport`\n(the `WorkerTransport` impl the leader registers),\n`LocalExecWorkerTransport` (the worker-side stub for nested broadcasts),\n`MppWorkerResolver`.\n3. `mpp/transport.rs` — `DrainHandle` and the cooperative-pull\nprimitives. The drain runs inline on the backend thread because pgrx\n0.18 enforces single-threaded Postgres FFI.\n4. `mpp/glue.rs` — the public API the customscan calls:\n`estimate_dsm_size`, `leader_setup`, `worker_setup`. Thin wrappers\naround dsm/runtime.\n5. `mpp/exec.rs` — `run_producer_fragment` is the worker push loop.\n6. `aggregatescan/mod.rs` — the integration: `stash_mpp_plan_bytes`,\n`exec_mpp_worker`, `build_mpp_leader_session_context`, the\n`ParallelQueryCapable` impl, the `parallel_workers` clamp in\n`try_build_datafusion_aggregate_path`. Both leader and worker session\ncontexts call `with_distributed_in_process_mode(true)` explicitly.\n\nThe build-side all-gather lives in\n`aggregatescan/mod.rs::exec_mpp_worker` and `mpp/dsm.rs` (the cache\nregion). Read those together.\n\n## Tests\n\n- pgrx regression suite — new `mpp_aggregate.sql` covers correctness on\nthe join-with-aggregate shapes; existing `mpp_*` suites still pass.\n- 25M `aggregate_join_groupby` bench: byte-exact result vs serial\nDataFusion at N=2/4/8/10.\n- Build-side all-gather is exercised at all N in the regression suite\n(workers vs leader-only-writer paths).\n\n---------\n\nCo-authored-by: paradedb-bot <developers@paradedb.com>\nCo-authored-by: paradedb-github-app[bot] <282009505+paradedb-github-app[bot]@users.noreply.github.com>",
          "timestamp": "2026-05-12T16:47:00-07:00",
          "tree_id": "38dfa579bfcab58f4b6b66ddea91de57c32f5204",
          "url": "https://github.com/paradedb/paradedb/commit/6bdea0d414fb563f086ddfe56712b2960d200626"
        },
        "date": 1778632215670,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.255816,
            "unit": "median cpu",
            "extra": "avg cpu: 20.48117588460942, max cpu: 42.814667, count: 57761"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 235.453125,
            "unit": "median mem",
            "extra": "avg mem: 235.28408698667786, max mem: 236.9375, count: 57761"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.391813,
            "unit": "median cpu",
            "extra": "avg cpu: 22.523719939580943, max cpu: 33.432835, count: 57761"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 177.63671875,
            "unit": "median mem",
            "extra": "avg mem: 177.42985413492667, max mem: 178.46875, count: 57761"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 34962,
            "unit": "median block_count",
            "extra": "avg block_count: 34104.360156506984, max block_count: 37142.0, count: 57761"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 79,
            "unit": "median segment_count",
            "extra": "avg segment_count: 82.30023718425927, max segment_count: 135.0, count: 57761"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "james.sewell@gmail.com",
            "name": "James Sewell",
            "username": "jamessewell"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "8182eaf110c30cbefe008197caa40efa8b44f8e0",
          "message": "refactor: use existing FFHelper ctid cache instead of dedicated cache (#4905)\n\nFix a performance regression introduced in e0804b347 (#4765) which\nremoved ctid from SearchIndexScore and switched to lazy per-row\nresolution.\n\nPrior to #4765, ctid was resolved during result construction and carried\nin `SearchIndexScore` — no per-row fast-field lookups needed. #4765\nmoved ctid resolution to the consumption side (top_k.rs, normal.rs,\nscan.rs) using a single-entry `Option<(SegmentOrdinal, FFType)>` cache.\nWhen TopK results interleave across segments (sorted by score), every\nsegment transition re-opens the ctid column via `FastFieldReaders::u64\n-> DynamicColumnHandle::open -> BlockwiseLinearCodec::load`, which is\nvery expensive. Profiling showed 45% of total cycles spent in this\nre-open path.\n\nThe columnar scan path (`ColumnarExecState`) was unaffected — it already\nused `FFHelper`'s per-segment `OnceLock` ctid cache. This PR brings the\nremaining paths in line:\n\n- `scan.rs` uses its existing `Bm25ScanState.fast_fields` FFHelper\n- `normal.rs` and `top_k.rs` use a new `ctid_cache` FFHelper on\n`BaseScanState`\n\nEach segment's ctid column is opened at most once via `OnceLock`,\neliminating the thrashing. `FFHelper` has had this per-segment ctid\ncaching built in since cb78f0ca2 (Oct 2024).",
          "timestamp": "2026-05-13T12:18:31+12:00",
          "tree_id": "814e1da895eec41e0dfe3cbb5348bdb237811bf7",
          "url": "https://github.com/paradedb/paradedb/commit/8182eaf110c30cbefe008197caa40efa8b44f8e0"
        },
        "date": 1778634115119,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.255816,
            "unit": "median cpu",
            "extra": "avg cpu: 20.68650728356433, max cpu: 43.286575, count: 57375"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 235.234375,
            "unit": "median mem",
            "extra": "avg mem: 235.10733789488017, max mem: 236.71875, count: 57375"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.369036,
            "unit": "median cpu",
            "extra": "avg cpu: 22.729099627924615, max cpu: 33.432835, count: 57375"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 177.46484375,
            "unit": "median mem",
            "extra": "avg mem: 177.3731869553377, max mem: 178.12890625, count: 57375"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 34629,
            "unit": "median block_count",
            "extra": "avg block_count: 33899.37336819172, max block_count: 36543.0, count: 57375"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 79,
            "unit": "median segment_count",
            "extra": "avg segment_count: 81.85570370370371, max segment_count: 133.0, count: 57375"
          }
        ]
      }
    ],
    "pg_search wide-table.toml Performance - TPS": [
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
        "date": 1778525611398,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 1127.3230876086723,
            "unit": "median tps",
            "extra": "avg tps: 1132.7223535368896, max tps: 1192.254743902077, count: 56126"
          },
          {
            "name": "Single Insert - Primary - tps",
            "value": 1217.5320187625425,
            "unit": "median tps",
            "extra": "avg tps: 1215.1907892892023, max tps: 1265.0979276434825, count: 56126"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 1093.5876380310779,
            "unit": "median tps",
            "extra": "avg tps: 1000.4283417902755, max tps: 1495.8151082112538, count: 56126"
          },
          {
            "name": "Top K - Primary - tps",
            "value": 5.1516617407043865,
            "unit": "median tps",
            "extra": "avg tps: 5.229186842547593, max tps: 7.667708650764491, count: 56126"
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
        "date": 1778525948159,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 1135.0932776983934,
            "unit": "median tps",
            "extra": "avg tps: 1139.222493917897, max tps: 1180.0321757859667, count: 56373"
          },
          {
            "name": "Single Insert - Primary - tps",
            "value": 1261.5996089626092,
            "unit": "median tps",
            "extra": "avg tps: 1236.2935843356695, max tps: 1276.0949449199438, count: 56373"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 1852.4445429649704,
            "unit": "median tps",
            "extra": "avg tps: 1820.0727604818615, max tps: 1980.5753468856265, count: 56373"
          },
          {
            "name": "Top K - Primary - tps",
            "value": 5.203101181938212,
            "unit": "median tps",
            "extra": "avg tps: 5.237856458471322, max tps: 6.95816246173776, count: 56373"
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
        "date": 1778526114418,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 1136.3153680785579,
            "unit": "median tps",
            "extra": "avg tps: 1138.333962275335, max tps: 1177.0543274246186, count: 56261"
          },
          {
            "name": "Single Insert - Primary - tps",
            "value": 1230.2181306218781,
            "unit": "median tps",
            "extra": "avg tps: 1218.8185956316618, max tps: 1253.2013846676587, count: 56261"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 1885.0183212891604,
            "unit": "median tps",
            "extra": "avg tps: 1854.9938598691147, max tps: 2017.1262520660493, count: 56261"
          },
          {
            "name": "Top K - Primary - tps",
            "value": 5.5052602075878605,
            "unit": "median tps",
            "extra": "avg tps: 5.520573370339791, max tps: 7.644140668332723, count: 56261"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mdashti@gmail.com",
            "name": "Moe",
            "username": "mdashti"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "6bdea0d414fb563f086ddfe56712b2960d200626",
          "message": "feat(mpp): AggregateScan in-process MPP via custom shm_mq transport (#4988)\n\n# Ticket(s) Closed\n\n- Closes #4152\n\n## What\n\nAdds an MPP execution path for AggregateScan that runs\njoin-with-aggregate shapes inside Postgres parallel-worker processes via\na custom transport on top of `paradedb/datafusion-distributed`. Default\noff behind `paradedb.enable_mpp` (minimum `mpp_worker_count = 3`). Other\nshapes and the JoinScan path are unchanged.\n\n## Why\n\nSingle-process DataFusion bottlenecks on join-with-aggregate at scale.\nDistributing the producer fragment across PG parallel workers gives us\n1.55–1.92× speedup on the 25M `aggregate_join_groupby` bench at N=2/4\nwithout leaving the embedded model — every worker is still a real PG\nprocess with its own snapshot, no gRPC, no extra daemon.\n\n## How\n\n- Leader builds the logical plan and stashes it in a DSM segment\nalongside an N-way `shm_mq` mesh.\n- Workers attach, deserialize, and re-plan with the same `SessionState`.\nIdentical inputs ⇒ structurally identical physical plans on every\nworker, so we don't need to serialize physical subplans.\n- Each worker runs its producer fragment and pushes batches through its\noutbound queue. Leader runs `NetworkShuffleExec` + final aggregate and\nreturns rows to the client. Leader is consumer-only in this iteration.\n- Build side (non-partitioning sources like `HashJoinExec(CollectLeft)`)\nis split via DSM all-gather: each worker scans its 1/N slice, writes to\na per-worker DSM region, completion-flag barrier, then everyone reads\nevery slice. Build is fully parallel; no leader-side serial scan.\n- The fork emits the network operators (`NetworkShuffleExec`,\n`NetworkBroadcastExec`, `NetworkCoalesceExec`) and we register a custom\ntransport that short-circuits the gRPC dialer. The fork's in-process\ntwo-boundary planner distinguishes outer (worker → leader, N producers)\nfrom nested (single local producer) Network boundaries.\n\n## Reviewer's Guide\n\nSuggested reading order — most of the diff lives under\n`pg_search/src/postgres/customscan/mpp/`:\n\n1. `mpp/dsm.rs` — the DSM layout: header, queue mesh, build-cache\nregion. `compute_dsm_layout` is the math; `leader_init` /\n`worker_attach` are the unsafe FFI boundaries.\n2. `mpp/runtime.rs` — `MppMesh` (runtime handle), `ShmMqWorkerTransport`\n(the `WorkerTransport` impl the leader registers),\n`LocalExecWorkerTransport` (the worker-side stub for nested broadcasts),\n`MppWorkerResolver`.\n3. `mpp/transport.rs` — `DrainHandle` and the cooperative-pull\nprimitives. The drain runs inline on the backend thread because pgrx\n0.18 enforces single-threaded Postgres FFI.\n4. `mpp/glue.rs` — the public API the customscan calls:\n`estimate_dsm_size`, `leader_setup`, `worker_setup`. Thin wrappers\naround dsm/runtime.\n5. `mpp/exec.rs` — `run_producer_fragment` is the worker push loop.\n6. `aggregatescan/mod.rs` — the integration: `stash_mpp_plan_bytes`,\n`exec_mpp_worker`, `build_mpp_leader_session_context`, the\n`ParallelQueryCapable` impl, the `parallel_workers` clamp in\n`try_build_datafusion_aggregate_path`. Both leader and worker session\ncontexts call `with_distributed_in_process_mode(true)` explicitly.\n\nThe build-side all-gather lives in\n`aggregatescan/mod.rs::exec_mpp_worker` and `mpp/dsm.rs` (the cache\nregion). Read those together.\n\n## Tests\n\n- pgrx regression suite — new `mpp_aggregate.sql` covers correctness on\nthe join-with-aggregate shapes; existing `mpp_*` suites still pass.\n- 25M `aggregate_join_groupby` bench: byte-exact result vs serial\nDataFusion at N=2/4/8/10.\n- Build-side all-gather is exercised at all N in the regression suite\n(workers vs leader-only-writer paths).\n\n---------\n\nCo-authored-by: paradedb-bot <developers@paradedb.com>\nCo-authored-by: paradedb-github-app[bot] <282009505+paradedb-github-app[bot]@users.noreply.github.com>",
          "timestamp": "2026-05-12T16:47:00-07:00",
          "tree_id": "38dfa579bfcab58f4b6b66ddea91de57c32f5204",
          "url": "https://github.com/paradedb/paradedb/commit/6bdea0d414fb563f086ddfe56712b2960d200626"
        },
        "date": 1778632902198,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 1098.9953449266034,
            "unit": "median tps",
            "extra": "avg tps: 1100.7395954350595, max tps: 1138.0530138275876, count: 56280"
          },
          {
            "name": "Single Insert - Primary - tps",
            "value": 1167.1140698529796,
            "unit": "median tps",
            "extra": "avg tps: 1176.7529285055837, max tps: 1273.2783866966777, count: 56280"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 1858.7442129407605,
            "unit": "median tps",
            "extra": "avg tps: 1834.2612158261784, max tps: 1987.9031200672518, count: 56280"
          },
          {
            "name": "Top K - Primary - tps",
            "value": 5.2481414287796735,
            "unit": "median tps",
            "extra": "avg tps: 5.30110600266312, max tps: 6.931926468356105, count: 56280"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "james.sewell@gmail.com",
            "name": "James Sewell",
            "username": "jamessewell"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "8182eaf110c30cbefe008197caa40efa8b44f8e0",
          "message": "refactor: use existing FFHelper ctid cache instead of dedicated cache (#4905)\n\nFix a performance regression introduced in e0804b347 (#4765) which\nremoved ctid from SearchIndexScore and switched to lazy per-row\nresolution.\n\nPrior to #4765, ctid was resolved during result construction and carried\nin `SearchIndexScore` — no per-row fast-field lookups needed. #4765\nmoved ctid resolution to the consumption side (top_k.rs, normal.rs,\nscan.rs) using a single-entry `Option<(SegmentOrdinal, FFType)>` cache.\nWhen TopK results interleave across segments (sorted by score), every\nsegment transition re-opens the ctid column via `FastFieldReaders::u64\n-> DynamicColumnHandle::open -> BlockwiseLinearCodec::load`, which is\nvery expensive. Profiling showed 45% of total cycles spent in this\nre-open path.\n\nThe columnar scan path (`ColumnarExecState`) was unaffected — it already\nused `FFHelper`'s per-segment `OnceLock` ctid cache. This PR brings the\nremaining paths in line:\n\n- `scan.rs` uses its existing `Bm25ScanState.fast_fields` FFHelper\n- `normal.rs` and `top_k.rs` use a new `ctid_cache` FFHelper on\n`BaseScanState`\n\nEach segment's ctid column is opened at most once via `OnceLock`,\neliminating the thrashing. `FFHelper` has had this per-segment ctid\ncaching built in since cb78f0ca2 (Oct 2024).",
          "timestamp": "2026-05-13T12:18:31+12:00",
          "tree_id": "814e1da895eec41e0dfe3cbb5348bdb237811bf7",
          "url": "https://github.com/paradedb/paradedb/commit/8182eaf110c30cbefe008197caa40efa8b44f8e0"
        },
        "date": 1778634802633,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 1123.9446343920936,
            "unit": "median tps",
            "extra": "avg tps: 1127.9208009212323, max tps: 1182.6705979574772, count: 56596"
          },
          {
            "name": "Single Insert - Primary - tps",
            "value": 1248.7198416926376,
            "unit": "median tps",
            "extra": "avg tps: 1239.5517000635755, max tps: 1256.6266616414973, count: 56596"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 1771.9779346561904,
            "unit": "median tps",
            "extra": "avg tps: 1772.6759135297948, max tps: 1976.9182710536963, count: 56596"
          },
          {
            "name": "Top K - Primary - tps",
            "value": 5.432104241692168,
            "unit": "median tps",
            "extra": "avg tps: 5.4477160674660015, max tps: 7.024264873452389, count: 56596"
          }
        ]
      }
    ],
    "pg_search wide-table.toml Performance - Other Metrics": [
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
        "date": 1778525662479,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Background Merger - Primary - background_merging",
            "value": 0,
            "unit": "median background_merging",
            "extra": "avg background_merging: 0.06018601004881873, max background_merging: 2.0, count: 56126"
          },
          {
            "name": "Background Merger - Primary - cpu",
            "value": 4.669261,
            "unit": "median cpu",
            "extra": "avg cpu: 4.714612051348833, max cpu: 9.687184, count: 56126"
          },
          {
            "name": "Background Merger - Primary - mem",
            "value": 28.67578125,
            "unit": "median mem",
            "extra": "avg mem: 28.659468275064143, max mem: 28.6796875, count: 56126"
          },
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 4.673807,
            "unit": "median cpu",
            "extra": "avg cpu: 4.973729893349462, max cpu: 27.961164, count: 56126"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 187.53515625,
            "unit": "median mem",
            "extra": "avg mem: 182.89124254189235, max mem: 187.80078125, count: 56126"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 59761,
            "unit": "median block_count",
            "extra": "avg block_count: 59508.08632362898, max block_count: 59761.0, count: 56126"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 45,
            "unit": "median segment_count",
            "extra": "avg segment_count: 41.77215550725154, max segment_count: 58.0, count: 56126"
          },
          {
            "name": "Single Insert - Primary - cpu",
            "value": 4.669261,
            "unit": "median cpu",
            "extra": "avg cpu: 4.707482438228849, max cpu: 33.667336, count: 56126"
          },
          {
            "name": "Single Insert - Primary - mem",
            "value": 144.15625,
            "unit": "median mem",
            "extra": "avg mem: 130.60882886552133, max mem: 160.03515625, count: 56126"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.669261,
            "unit": "median cpu",
            "extra": "avg cpu: 5.182359842513575, max cpu: 32.621357, count: 56126"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 201.453125,
            "unit": "median mem",
            "extra": "avg mem: 199.85120560212735, max mem: 215.74609375, count: 56126"
          },
          {
            "name": "Top K - Primary - cpu",
            "value": 23.391813,
            "unit": "median cpu",
            "extra": "avg cpu: 23.90642493439596, max cpu: 33.4995, count: 56126"
          },
          {
            "name": "Top K - Primary - mem",
            "value": 162.953125,
            "unit": "median mem",
            "extra": "avg mem: 181.3543009673547, max mem: 221.390625, count: 56126"
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
        "date": 1778525989222,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Background Merger - Primary - background_merging",
            "value": 0,
            "unit": "median background_merging",
            "extra": "avg background_merging: 0.07301367676015114, max background_merging: 2.0, count: 56373"
          },
          {
            "name": "Background Merger - Primary - cpu",
            "value": 4.669261,
            "unit": "median cpu",
            "extra": "avg cpu: 4.816534629554224, max cpu: 9.657948, count: 56373"
          },
          {
            "name": "Background Merger - Primary - mem",
            "value": 27.94921875,
            "unit": "median mem",
            "extra": "avg mem: 28.00163420431767, max mem: 28.0703125, count: 56373"
          },
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 4.669261,
            "unit": "median cpu",
            "extra": "avg cpu: 4.990115413297869, max cpu: 9.7165985, count: 56373"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 169.73046875,
            "unit": "median mem",
            "extra": "avg mem: 168.422358595316, max mem: 169.98046875, count: 56373"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 51477,
            "unit": "median block_count",
            "extra": "avg block_count: 51345.50703350895, max block_count: 51477.0, count: 56373"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 45,
            "unit": "median segment_count",
            "extra": "avg segment_count: 42.92602841785961, max segment_count: 57.0, count: 56373"
          },
          {
            "name": "Single Insert - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 4.633980690515308, max cpu: 23.210833, count: 56373"
          },
          {
            "name": "Single Insert - Primary - mem",
            "value": 147.41796875,
            "unit": "median mem",
            "extra": "avg mem: 139.0357681065182, max mem: 161.45703125, count: 56373"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.669261,
            "unit": "median cpu",
            "extra": "avg cpu: 4.7651319900082605, max cpu: 23.323614, count: 56373"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 183.84375,
            "unit": "median mem",
            "extra": "avg mem: 179.78810563401362, max mem: 184.0234375, count: 56373"
          },
          {
            "name": "Top K - Primary - cpu",
            "value": 23.391813,
            "unit": "median cpu",
            "extra": "avg cpu: 23.85771504734382, max cpu: 33.300297, count: 56373"
          },
          {
            "name": "Top K - Primary - mem",
            "value": 163.58984375,
            "unit": "median mem",
            "extra": "avg mem: 181.92006797913007, max mem: 221.9453125, count: 56373"
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
        "date": 1778526165504,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Background Merger - Primary - background_merging",
            "value": 0,
            "unit": "median background_merging",
            "extra": "avg background_merging: 0.07219921437585539, max background_merging: 2.0, count: 56261"
          },
          {
            "name": "Background Merger - Primary - cpu",
            "value": 4.673807,
            "unit": "median cpu",
            "extra": "avg cpu: 4.856432729358954, max cpu: 9.687184, count: 56261"
          },
          {
            "name": "Background Merger - Primary - mem",
            "value": 26.84375,
            "unit": "median mem",
            "extra": "avg mem: 26.895830787534884, max mem: 26.96484375, count: 56261"
          },
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 4.669261,
            "unit": "median cpu",
            "extra": "avg cpu: 4.9239912442887395, max cpu: 28.09756, count: 56261"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 186.75,
            "unit": "median mem",
            "extra": "avg mem: 178.77632485202895, max mem: 188.26953125, count: 56261"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 51447,
            "unit": "median block_count",
            "extra": "avg block_count: 51315.471605552695, max block_count: 51447.0, count: 56261"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 45,
            "unit": "median segment_count",
            "extra": "avg segment_count: 42.739873091484334, max segment_count: 56.0, count: 56261"
          },
          {
            "name": "Single Insert - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 4.661029561137625, max cpu: 28.042841, count: 56261"
          },
          {
            "name": "Single Insert - Primary - mem",
            "value": 145.4453125,
            "unit": "median mem",
            "extra": "avg mem: 134.43051331073033, max mem: 159.05078125, count: 56261"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.669261,
            "unit": "median cpu",
            "extra": "avg cpu: 4.797176169357374, max cpu: 27.988338, count: 56261"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 183.78515625,
            "unit": "median mem",
            "extra": "avg mem: 179.06202169075826, max mem: 184.01171875, count: 56261"
          },
          {
            "name": "Top K - Primary - cpu",
            "value": 23.414635,
            "unit": "median cpu",
            "extra": "avg cpu: 23.919663097825403, max cpu: 33.768845, count: 56261"
          },
          {
            "name": "Top K - Primary - mem",
            "value": 163.87890625,
            "unit": "median mem",
            "extra": "avg mem: 182.11343052636374, max mem: 222.2578125, count: 56261"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mdashti@gmail.com",
            "name": "Moe",
            "username": "mdashti"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "6bdea0d414fb563f086ddfe56712b2960d200626",
          "message": "feat(mpp): AggregateScan in-process MPP via custom shm_mq transport (#4988)\n\n# Ticket(s) Closed\n\n- Closes #4152\n\n## What\n\nAdds an MPP execution path for AggregateScan that runs\njoin-with-aggregate shapes inside Postgres parallel-worker processes via\na custom transport on top of `paradedb/datafusion-distributed`. Default\noff behind `paradedb.enable_mpp` (minimum `mpp_worker_count = 3`). Other\nshapes and the JoinScan path are unchanged.\n\n## Why\n\nSingle-process DataFusion bottlenecks on join-with-aggregate at scale.\nDistributing the producer fragment across PG parallel workers gives us\n1.55–1.92× speedup on the 25M `aggregate_join_groupby` bench at N=2/4\nwithout leaving the embedded model — every worker is still a real PG\nprocess with its own snapshot, no gRPC, no extra daemon.\n\n## How\n\n- Leader builds the logical plan and stashes it in a DSM segment\nalongside an N-way `shm_mq` mesh.\n- Workers attach, deserialize, and re-plan with the same `SessionState`.\nIdentical inputs ⇒ structurally identical physical plans on every\nworker, so we don't need to serialize physical subplans.\n- Each worker runs its producer fragment and pushes batches through its\noutbound queue. Leader runs `NetworkShuffleExec` + final aggregate and\nreturns rows to the client. Leader is consumer-only in this iteration.\n- Build side (non-partitioning sources like `HashJoinExec(CollectLeft)`)\nis split via DSM all-gather: each worker scans its 1/N slice, writes to\na per-worker DSM region, completion-flag barrier, then everyone reads\nevery slice. Build is fully parallel; no leader-side serial scan.\n- The fork emits the network operators (`NetworkShuffleExec`,\n`NetworkBroadcastExec`, `NetworkCoalesceExec`) and we register a custom\ntransport that short-circuits the gRPC dialer. The fork's in-process\ntwo-boundary planner distinguishes outer (worker → leader, N producers)\nfrom nested (single local producer) Network boundaries.\n\n## Reviewer's Guide\n\nSuggested reading order — most of the diff lives under\n`pg_search/src/postgres/customscan/mpp/`:\n\n1. `mpp/dsm.rs` — the DSM layout: header, queue mesh, build-cache\nregion. `compute_dsm_layout` is the math; `leader_init` /\n`worker_attach` are the unsafe FFI boundaries.\n2. `mpp/runtime.rs` — `MppMesh` (runtime handle), `ShmMqWorkerTransport`\n(the `WorkerTransport` impl the leader registers),\n`LocalExecWorkerTransport` (the worker-side stub for nested broadcasts),\n`MppWorkerResolver`.\n3. `mpp/transport.rs` — `DrainHandle` and the cooperative-pull\nprimitives. The drain runs inline on the backend thread because pgrx\n0.18 enforces single-threaded Postgres FFI.\n4. `mpp/glue.rs` — the public API the customscan calls:\n`estimate_dsm_size`, `leader_setup`, `worker_setup`. Thin wrappers\naround dsm/runtime.\n5. `mpp/exec.rs` — `run_producer_fragment` is the worker push loop.\n6. `aggregatescan/mod.rs` — the integration: `stash_mpp_plan_bytes`,\n`exec_mpp_worker`, `build_mpp_leader_session_context`, the\n`ParallelQueryCapable` impl, the `parallel_workers` clamp in\n`try_build_datafusion_aggregate_path`. Both leader and worker session\ncontexts call `with_distributed_in_process_mode(true)` explicitly.\n\nThe build-side all-gather lives in\n`aggregatescan/mod.rs::exec_mpp_worker` and `mpp/dsm.rs` (the cache\nregion). Read those together.\n\n## Tests\n\n- pgrx regression suite — new `mpp_aggregate.sql` covers correctness on\nthe join-with-aggregate shapes; existing `mpp_*` suites still pass.\n- 25M `aggregate_join_groupby` bench: byte-exact result vs serial\nDataFusion at N=2/4/8/10.\n- Build-side all-gather is exercised at all N in the regression suite\n(workers vs leader-only-writer paths).\n\n---------\n\nCo-authored-by: paradedb-bot <developers@paradedb.com>\nCo-authored-by: paradedb-github-app[bot] <282009505+paradedb-github-app[bot]@users.noreply.github.com>",
          "timestamp": "2026-05-12T16:47:00-07:00",
          "tree_id": "38dfa579bfcab58f4b6b66ddea91de57c32f5204",
          "url": "https://github.com/paradedb/paradedb/commit/6bdea0d414fb563f086ddfe56712b2960d200626"
        },
        "date": 1778632933913,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Background Merger - Primary - background_merging",
            "value": 0,
            "unit": "median background_merging",
            "extra": "avg background_merging: 0.06950959488272922, max background_merging: 2.0, count: 56280"
          },
          {
            "name": "Background Merger - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 4.830826643948626, max cpu: 13.967022, count: 56280"
          },
          {
            "name": "Background Merger - Primary - mem",
            "value": 27.93359375,
            "unit": "median mem",
            "extra": "avg mem: 27.98479949582445, max mem: 28.0546875, count: 56280"
          },
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 5.071075621653811, max cpu: 24.072216, count: 56280"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 189.72265625,
            "unit": "median mem",
            "extra": "avg mem: 181.86135887249023, max mem: 189.96875, count: 56280"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 51426,
            "unit": "median block_count",
            "extra": "avg block_count: 51292.36364605543, max block_count: 51426.0, count: 56280"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 46,
            "unit": "median segment_count",
            "extra": "avg segment_count: 42.84662402274343, max segment_count: 56.0, count: 56280"
          },
          {
            "name": "Single Insert - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.794482538772412, max cpu: 28.973843, count: 56280"
          },
          {
            "name": "Single Insert - Primary - mem",
            "value": 163.8125,
            "unit": "median mem",
            "extra": "avg mem: 144.76274764570007, max mem: 176.75390625, count: 56280"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 4.775576090588614, max cpu: 28.374382, count: 56280"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 184.625,
            "unit": "median mem",
            "extra": "avg mem: 176.33161494425195, max mem: 185.87890625, count: 56280"
          },
          {
            "name": "Top K - Primary - cpu",
            "value": 23.369036,
            "unit": "median cpu",
            "extra": "avg cpu: 23.788570906052662, max cpu: 33.939396, count: 56280"
          },
          {
            "name": "Top K - Primary - mem",
            "value": 164.15234375,
            "unit": "median mem",
            "extra": "avg mem: 182.21498756218907, max mem: 222.54296875, count: 56280"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "james.sewell@gmail.com",
            "name": "James Sewell",
            "username": "jamessewell"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "8182eaf110c30cbefe008197caa40efa8b44f8e0",
          "message": "refactor: use existing FFHelper ctid cache instead of dedicated cache (#4905)\n\nFix a performance regression introduced in e0804b347 (#4765) which\nremoved ctid from SearchIndexScore and switched to lazy per-row\nresolution.\n\nPrior to #4765, ctid was resolved during result construction and carried\nin `SearchIndexScore` — no per-row fast-field lookups needed. #4765\nmoved ctid resolution to the consumption side (top_k.rs, normal.rs,\nscan.rs) using a single-entry `Option<(SegmentOrdinal, FFType)>` cache.\nWhen TopK results interleave across segments (sorted by score), every\nsegment transition re-opens the ctid column via `FastFieldReaders::u64\n-> DynamicColumnHandle::open -> BlockwiseLinearCodec::load`, which is\nvery expensive. Profiling showed 45% of total cycles spent in this\nre-open path.\n\nThe columnar scan path (`ColumnarExecState`) was unaffected — it already\nused `FFHelper`'s per-segment `OnceLock` ctid cache. This PR brings the\nremaining paths in line:\n\n- `scan.rs` uses its existing `Bm25ScanState.fast_fields` FFHelper\n- `normal.rs` and `top_k.rs` use a new `ctid_cache` FFHelper on\n`BaseScanState`\n\nEach segment's ctid column is opened at most once via `OnceLock`,\neliminating the thrashing. `FFHelper` has had this per-segment ctid\ncaching built in since cb78f0ca2 (Oct 2024).",
          "timestamp": "2026-05-13T12:18:31+12:00",
          "tree_id": "814e1da895eec41e0dfe3cbb5348bdb237811bf7",
          "url": "https://github.com/paradedb/paradedb/commit/8182eaf110c30cbefe008197caa40efa8b44f8e0"
        },
        "date": 1778634834038,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Background Merger - Primary - background_merging",
            "value": 0,
            "unit": "median background_merging",
            "extra": "avg background_merging: 0.07046434376987773, max background_merging: 2.0, count: 56596"
          },
          {
            "name": "Background Merger - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.879072499869863, max cpu: 9.67742, count: 56596"
          },
          {
            "name": "Background Merger - Primary - mem",
            "value": 29.38671875,
            "unit": "median mem",
            "extra": "avg mem: 29.32580482719627, max mem: 29.39453125, count: 56596"
          },
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 5.0373522673360265, max cpu: 27.906979, count: 56596"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 184.484375,
            "unit": "median mem",
            "extra": "avg mem: 180.22597859168494, max mem: 186.0, count: 56596"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 51453,
            "unit": "median block_count",
            "extra": "avg block_count: 51324.20930807831, max block_count: 51453.0, count: 56596"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 45,
            "unit": "median segment_count",
            "extra": "avg segment_count: 42.93439465686621, max segment_count: 56.0, count: 56596"
          },
          {
            "name": "Single Insert - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 4.636874247588598, max cpu: 32.40116, count: 56596"
          },
          {
            "name": "Single Insert - Primary - mem",
            "value": 145.0625,
            "unit": "median mem",
            "extra": "avg mem: 132.94325059964484, max mem: 160.109375, count: 56596"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.708351953049828, max cpu: 28.543112, count: 56596"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 189.609375,
            "unit": "median mem",
            "extra": "avg mem: 182.76740329760494, max mem: 189.86328125, count: 56596"
          },
          {
            "name": "Top K - Primary - cpu",
            "value": 23.346306,
            "unit": "median cpu",
            "extra": "avg cpu: 23.566226249860446, max cpu: 33.768845, count: 56596"
          },
          {
            "name": "Top K - Primary - mem",
            "value": 163.828125,
            "unit": "median mem",
            "extra": "avg mem: 182.87408700482365, max mem: 222.23828125, count: 56596"
          }
        ]
      }
    ],
    "pg_search background-merge.toml Performance - TPS": [
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
        "date": 1778526327803,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 31.622480118771215,
            "unit": "median tps",
            "extra": "avg tps: 31.384969383361298, max tps: 33.48489219277973, count: 55534"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 244.86223532123097,
            "unit": "median tps",
            "extra": "avg tps: 272.7215655722178, max tps: 2782.275162652182, count: 55534"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 628.4771557117015,
            "unit": "median tps",
            "extra": "avg tps: 613.9547212578607, max tps: 1546.4350922698989, count: 55534"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 161.19052513415582,
            "unit": "median tps",
            "extra": "avg tps: 177.21993922265375, max tps: 869.5616537288105, count: 111068"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 15.37836316983882,
            "unit": "median tps",
            "extra": "avg tps: 15.342457182507506, max tps: 19.47480274069678, count: 55534"
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
        "date": 1778526643634,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 31.257217025597768,
            "unit": "median tps",
            "extra": "avg tps: 30.967563192982475, max tps: 33.05068868533235, count: 55666"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 254.8871902583693,
            "unit": "median tps",
            "extra": "avg tps: 284.7347091400273, max tps: 3152.012462472367, count: 55666"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 656.6371581403156,
            "unit": "median tps",
            "extra": "avg tps: 650.8594682957536, max tps: 1215.594448940382, count: 55666"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 164.828331975225,
            "unit": "median tps",
            "extra": "avg tps: 183.53874239525044, max tps: 1101.8026866228663, count: 111332"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 16.800845892308345,
            "unit": "median tps",
            "extra": "avg tps: 16.749898285445635, max tps: 22.25088581332695, count: 55666"
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
        "date": 1778526820645,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 32.45369432026579,
            "unit": "median tps",
            "extra": "avg tps: 32.13410047001808, max tps: 33.171163770016946, count: 55811"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 250.41492995067242,
            "unit": "median tps",
            "extra": "avg tps: 279.31233816713467, max tps: 3208.546514854151, count: 55811"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 673.0728618158321,
            "unit": "median tps",
            "extra": "avg tps: 654.7949513329069, max tps: 900.0381300569359, count: 55811"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 156.87489044131692,
            "unit": "median tps",
            "extra": "avg tps: 180.2176146244029, max tps: 1176.1358056909976, count: 111622"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 18.145606564722975,
            "unit": "median tps",
            "extra": "avg tps: 17.685117271918866, max tps: 20.38934015935819, count: 55811"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mdashti@gmail.com",
            "name": "Moe",
            "username": "mdashti"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "6bdea0d414fb563f086ddfe56712b2960d200626",
          "message": "feat(mpp): AggregateScan in-process MPP via custom shm_mq transport (#4988)\n\n# Ticket(s) Closed\n\n- Closes #4152\n\n## What\n\nAdds an MPP execution path for AggregateScan that runs\njoin-with-aggregate shapes inside Postgres parallel-worker processes via\na custom transport on top of `paradedb/datafusion-distributed`. Default\noff behind `paradedb.enable_mpp` (minimum `mpp_worker_count = 3`). Other\nshapes and the JoinScan path are unchanged.\n\n## Why\n\nSingle-process DataFusion bottlenecks on join-with-aggregate at scale.\nDistributing the producer fragment across PG parallel workers gives us\n1.55–1.92× speedup on the 25M `aggregate_join_groupby` bench at N=2/4\nwithout leaving the embedded model — every worker is still a real PG\nprocess with its own snapshot, no gRPC, no extra daemon.\n\n## How\n\n- Leader builds the logical plan and stashes it in a DSM segment\nalongside an N-way `shm_mq` mesh.\n- Workers attach, deserialize, and re-plan with the same `SessionState`.\nIdentical inputs ⇒ structurally identical physical plans on every\nworker, so we don't need to serialize physical subplans.\n- Each worker runs its producer fragment and pushes batches through its\noutbound queue. Leader runs `NetworkShuffleExec` + final aggregate and\nreturns rows to the client. Leader is consumer-only in this iteration.\n- Build side (non-partitioning sources like `HashJoinExec(CollectLeft)`)\nis split via DSM all-gather: each worker scans its 1/N slice, writes to\na per-worker DSM region, completion-flag barrier, then everyone reads\nevery slice. Build is fully parallel; no leader-side serial scan.\n- The fork emits the network operators (`NetworkShuffleExec`,\n`NetworkBroadcastExec`, `NetworkCoalesceExec`) and we register a custom\ntransport that short-circuits the gRPC dialer. The fork's in-process\ntwo-boundary planner distinguishes outer (worker → leader, N producers)\nfrom nested (single local producer) Network boundaries.\n\n## Reviewer's Guide\n\nSuggested reading order — most of the diff lives under\n`pg_search/src/postgres/customscan/mpp/`:\n\n1. `mpp/dsm.rs` — the DSM layout: header, queue mesh, build-cache\nregion. `compute_dsm_layout` is the math; `leader_init` /\n`worker_attach` are the unsafe FFI boundaries.\n2. `mpp/runtime.rs` — `MppMesh` (runtime handle), `ShmMqWorkerTransport`\n(the `WorkerTransport` impl the leader registers),\n`LocalExecWorkerTransport` (the worker-side stub for nested broadcasts),\n`MppWorkerResolver`.\n3. `mpp/transport.rs` — `DrainHandle` and the cooperative-pull\nprimitives. The drain runs inline on the backend thread because pgrx\n0.18 enforces single-threaded Postgres FFI.\n4. `mpp/glue.rs` — the public API the customscan calls:\n`estimate_dsm_size`, `leader_setup`, `worker_setup`. Thin wrappers\naround dsm/runtime.\n5. `mpp/exec.rs` — `run_producer_fragment` is the worker push loop.\n6. `aggregatescan/mod.rs` — the integration: `stash_mpp_plan_bytes`,\n`exec_mpp_worker`, `build_mpp_leader_session_context`, the\n`ParallelQueryCapable` impl, the `parallel_workers` clamp in\n`try_build_datafusion_aggregate_path`. Both leader and worker session\ncontexts call `with_distributed_in_process_mode(true)` explicitly.\n\nThe build-side all-gather lives in\n`aggregatescan/mod.rs::exec_mpp_worker` and `mpp/dsm.rs` (the cache\nregion). Read those together.\n\n## Tests\n\n- pgrx regression suite — new `mpp_aggregate.sql` covers correctness on\nthe join-with-aggregate shapes; existing `mpp_*` suites still pass.\n- 25M `aggregate_join_groupby` bench: byte-exact result vs serial\nDataFusion at N=2/4/8/10.\n- Build-side all-gather is exercised at all N in the regression suite\n(workers vs leader-only-writer paths).\n\n---------\n\nCo-authored-by: paradedb-bot <developers@paradedb.com>\nCo-authored-by: paradedb-github-app[bot] <282009505+paradedb-github-app[bot]@users.noreply.github.com>",
          "timestamp": "2026-05-12T16:47:00-07:00",
          "tree_id": "38dfa579bfcab58f4b6b66ddea91de57c32f5204",
          "url": "https://github.com/paradedb/paradedb/commit/6bdea0d414fb563f086ddfe56712b2960d200626"
        },
        "date": 1778633578364,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 31.19334540347221,
            "unit": "median tps",
            "extra": "avg tps: 30.89114801433187, max tps: 31.83024183382732, count: 55647"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 248.54864305713562,
            "unit": "median tps",
            "extra": "avg tps: 276.62271615320583, max tps: 3108.630447834662, count: 55647"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 625.9840363083302,
            "unit": "median tps",
            "extra": "avg tps: 618.7352835918243, max tps: 740.1268523084389, count: 55647"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 168.43755048785232,
            "unit": "median tps",
            "extra": "avg tps: 184.87282332450712, max tps: 1208.267806472082, count: 111294"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 16.9234911784729,
            "unit": "median tps",
            "extra": "avg tps: 16.70718102560011, max tps: 21.098162746670223, count: 55647"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "james.sewell@gmail.com",
            "name": "James Sewell",
            "username": "jamessewell"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "8182eaf110c30cbefe008197caa40efa8b44f8e0",
          "message": "refactor: use existing FFHelper ctid cache instead of dedicated cache (#4905)\n\nFix a performance regression introduced in e0804b347 (#4765) which\nremoved ctid from SearchIndexScore and switched to lazy per-row\nresolution.\n\nPrior to #4765, ctid was resolved during result construction and carried\nin `SearchIndexScore` — no per-row fast-field lookups needed. #4765\nmoved ctid resolution to the consumption side (top_k.rs, normal.rs,\nscan.rs) using a single-entry `Option<(SegmentOrdinal, FFType)>` cache.\nWhen TopK results interleave across segments (sorted by score), every\nsegment transition re-opens the ctid column via `FastFieldReaders::u64\n-> DynamicColumnHandle::open -> BlockwiseLinearCodec::load`, which is\nvery expensive. Profiling showed 45% of total cycles spent in this\nre-open path.\n\nThe columnar scan path (`ColumnarExecState`) was unaffected — it already\nused `FFHelper`'s per-segment `OnceLock` ctid cache. This PR brings the\nremaining paths in line:\n\n- `scan.rs` uses its existing `Bm25ScanState.fast_fields` FFHelper\n- `normal.rs` and `top_k.rs` use a new `ctid_cache` FFHelper on\n`BaseScanState`\n\nEach segment's ctid column is opened at most once via `OnceLock`,\neliminating the thrashing. `FFHelper` has had this per-segment ctid\ncaching built in since cb78f0ca2 (Oct 2024).",
          "timestamp": "2026-05-13T12:18:31+12:00",
          "tree_id": "814e1da895eec41e0dfe3cbb5348bdb237811bf7",
          "url": "https://github.com/paradedb/paradedb/commit/8182eaf110c30cbefe008197caa40efa8b44f8e0"
        },
        "date": 1778635478017,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 30.582010897436113,
            "unit": "median tps",
            "extra": "avg tps: 30.329933405155874, max tps: 31.361927539637982, count: 55456"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 247.72881376568037,
            "unit": "median tps",
            "extra": "avg tps: 275.7657039216442, max tps: 3044.635120388051, count: 55456"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 634.9306498886136,
            "unit": "median tps",
            "extra": "avg tps: 625.2856608861472, max tps: 1087.2867026796441, count: 55456"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 166.9335024012785,
            "unit": "median tps",
            "extra": "avg tps: 183.15535911594336, max tps: 1000.3524336895549, count: 110912"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 16.69204795595703,
            "unit": "median tps",
            "extra": "avg tps: 16.493177612135643, max tps: 19.652446870150328, count: 55456"
          }
        ]
      }
    ],
    "pg_search background-merge.toml Performance - Other Metrics": [
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
        "date": 1778526381444,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.622696,
            "unit": "median cpu",
            "extra": "avg cpu: 19.68408650696802, max cpu: 46.966736, count: 55534"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 173.89453125,
            "unit": "median mem",
            "extra": "avg mem: 157.27299407795223, max mem: 176.9296875, count: 55534"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 7.684576293282276, max cpu: 28.09756, count: 55534"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 119.80078125,
            "unit": "median mem",
            "extra": "avg mem: 118.59181614810296, max mem: 119.86328125, count: 55534"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 6.225817924740211, max cpu: 18.658894, count: 55534"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 167.1328125,
            "unit": "median mem",
            "extra": "avg mem: 141.30508034773743, max mem: 175.65625, count: 55534"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 16568,
            "unit": "median block_count",
            "extra": "avg block_count: 16793.973907876258, max block_count: 31441.0, count: 55534"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.280215944697428, max cpu: 4.669261, count: 55534"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 108.6953125,
            "unit": "median mem",
            "extra": "avg mem: 95.44924252484964, max mem: 136.9296875, count: 55534"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 25,
            "unit": "median segment_count",
            "extra": "avg segment_count: 24.70081391579933, max segment_count: 36.0, count: 55534"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.266409,
            "unit": "median cpu",
            "extra": "avg cpu: 9.319064600361308, max cpu: 32.495163, count: 111068"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 177.25390625,
            "unit": "median mem",
            "extra": "avg mem: 159.6744916754263, max mem: 180.9375, count: 111068"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.913043,
            "unit": "median cpu",
            "extra": "avg cpu: 13.679215238596106, max cpu: 28.263002, count: 55534"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 171.296875,
            "unit": "median mem",
            "extra": "avg mem: 168.32178611863904, max mem: 172.05859375, count: 55534"
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
        "date": 1778526696254,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.640776,
            "unit": "median cpu",
            "extra": "avg cpu: 19.787466919648367, max cpu: 42.02335, count: 55666"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 177.84765625,
            "unit": "median mem",
            "extra": "avg mem: 176.02303112817967, max mem: 178.203125, count: 55666"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 7.64699952370718, max cpu: 30.094042, count: 55666"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 120.9609375,
            "unit": "median mem",
            "extra": "avg mem: 119.75524543145727, max mem: 121.0390625, count: 55666"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 6.191778597951396, max cpu: 18.695229, count: 55666"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 162.5,
            "unit": "median mem",
            "extra": "avg mem: 143.54047978400192, max mem: 178.44921875, count: 55666"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 16657,
            "unit": "median block_count",
            "extra": "avg block_count: 16874.125139223223, max block_count: 31570.0, count: 55666"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6021094,
            "unit": "median cpu",
            "extra": "avg cpu: 2.5671059944125183, max cpu: 4.673807, count: 55666"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 106.24609375,
            "unit": "median mem",
            "extra": "avg mem: 95.83040695565965, max mem: 137.75390625, count: 55666"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 26,
            "unit": "median segment_count",
            "extra": "avg segment_count: 25.563288183092013, max segment_count: 36.0, count: 55666"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.275363,
            "unit": "median cpu",
            "extra": "avg cpu: 9.212644161076819, max cpu: 28.09756, count: 111332"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 180.39453125,
            "unit": "median mem",
            "extra": "avg mem: 161.78413165408418, max mem: 182.5546875, count: 111332"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.899614,
            "unit": "median cpu",
            "extra": "avg cpu: 12.002531268745514, max cpu: 28.125, count: 55666"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 172.66796875,
            "unit": "median mem",
            "extra": "avg mem: 170.09818752975335, max mem: 173.17578125, count: 55666"
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
        "date": 1778526865774,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.640776,
            "unit": "median cpu",
            "extra": "avg cpu: 19.429082889184905, max cpu: 42.22874, count: 55811"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 175.67578125,
            "unit": "median mem",
            "extra": "avg mem: 158.67743456991005, max mem: 179.0390625, count: 55811"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 7.7239261003105275, max cpu: 28.180038, count: 55811"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 120.09765625,
            "unit": "median mem",
            "extra": "avg mem: 118.91228746629248, max mem: 120.234375, count: 55811"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 6.184803580036954, max cpu: 18.640776, count: 55811"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 174.6875,
            "unit": "median mem",
            "extra": "avg mem: 145.91514070646915, max mem: 179.9140625, count: 55811"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 16765,
            "unit": "median block_count",
            "extra": "avg block_count: 17059.832058196414, max block_count: 31945.0, count: 55811"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.4529820943789105, max cpu: 4.678363, count: 55811"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 111.28125,
            "unit": "median mem",
            "extra": "avg mem: 95.90906453532905, max mem: 137.35546875, count: 55811"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 24,
            "unit": "median segment_count",
            "extra": "avg segment_count: 24.40909498127609, max segment_count: 36.0, count: 55811"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.29332,
            "unit": "median cpu",
            "extra": "avg cpu: 9.430435662523818, max cpu: 28.374382, count: 111622"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 181.359375,
            "unit": "median mem",
            "extra": "avg mem: 163.33898811390227, max mem: 183.58984375, count: 111622"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 9.375,
            "unit": "median cpu",
            "extra": "avg cpu: 11.101200270126961, max cpu: 23.414635, count: 55811"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 173.5703125,
            "unit": "median mem",
            "extra": "avg mem: 170.77610533485782, max mem: 174.109375, count: 55811"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mdashti@gmail.com",
            "name": "Moe",
            "username": "mdashti"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "6bdea0d414fb563f086ddfe56712b2960d200626",
          "message": "feat(mpp): AggregateScan in-process MPP via custom shm_mq transport (#4988)\n\n# Ticket(s) Closed\n\n- Closes #4152\n\n## What\n\nAdds an MPP execution path for AggregateScan that runs\njoin-with-aggregate shapes inside Postgres parallel-worker processes via\na custom transport on top of `paradedb/datafusion-distributed`. Default\noff behind `paradedb.enable_mpp` (minimum `mpp_worker_count = 3`). Other\nshapes and the JoinScan path are unchanged.\n\n## Why\n\nSingle-process DataFusion bottlenecks on join-with-aggregate at scale.\nDistributing the producer fragment across PG parallel workers gives us\n1.55–1.92× speedup on the 25M `aggregate_join_groupby` bench at N=2/4\nwithout leaving the embedded model — every worker is still a real PG\nprocess with its own snapshot, no gRPC, no extra daemon.\n\n## How\n\n- Leader builds the logical plan and stashes it in a DSM segment\nalongside an N-way `shm_mq` mesh.\n- Workers attach, deserialize, and re-plan with the same `SessionState`.\nIdentical inputs ⇒ structurally identical physical plans on every\nworker, so we don't need to serialize physical subplans.\n- Each worker runs its producer fragment and pushes batches through its\noutbound queue. Leader runs `NetworkShuffleExec` + final aggregate and\nreturns rows to the client. Leader is consumer-only in this iteration.\n- Build side (non-partitioning sources like `HashJoinExec(CollectLeft)`)\nis split via DSM all-gather: each worker scans its 1/N slice, writes to\na per-worker DSM region, completion-flag barrier, then everyone reads\nevery slice. Build is fully parallel; no leader-side serial scan.\n- The fork emits the network operators (`NetworkShuffleExec`,\n`NetworkBroadcastExec`, `NetworkCoalesceExec`) and we register a custom\ntransport that short-circuits the gRPC dialer. The fork's in-process\ntwo-boundary planner distinguishes outer (worker → leader, N producers)\nfrom nested (single local producer) Network boundaries.\n\n## Reviewer's Guide\n\nSuggested reading order — most of the diff lives under\n`pg_search/src/postgres/customscan/mpp/`:\n\n1. `mpp/dsm.rs` — the DSM layout: header, queue mesh, build-cache\nregion. `compute_dsm_layout` is the math; `leader_init` /\n`worker_attach` are the unsafe FFI boundaries.\n2. `mpp/runtime.rs` — `MppMesh` (runtime handle), `ShmMqWorkerTransport`\n(the `WorkerTransport` impl the leader registers),\n`LocalExecWorkerTransport` (the worker-side stub for nested broadcasts),\n`MppWorkerResolver`.\n3. `mpp/transport.rs` — `DrainHandle` and the cooperative-pull\nprimitives. The drain runs inline on the backend thread because pgrx\n0.18 enforces single-threaded Postgres FFI.\n4. `mpp/glue.rs` — the public API the customscan calls:\n`estimate_dsm_size`, `leader_setup`, `worker_setup`. Thin wrappers\naround dsm/runtime.\n5. `mpp/exec.rs` — `run_producer_fragment` is the worker push loop.\n6. `aggregatescan/mod.rs` — the integration: `stash_mpp_plan_bytes`,\n`exec_mpp_worker`, `build_mpp_leader_session_context`, the\n`ParallelQueryCapable` impl, the `parallel_workers` clamp in\n`try_build_datafusion_aggregate_path`. Both leader and worker session\ncontexts call `with_distributed_in_process_mode(true)` explicitly.\n\nThe build-side all-gather lives in\n`aggregatescan/mod.rs::exec_mpp_worker` and `mpp/dsm.rs` (the cache\nregion). Read those together.\n\n## Tests\n\n- pgrx regression suite — new `mpp_aggregate.sql` covers correctness on\nthe join-with-aggregate shapes; existing `mpp_*` suites still pass.\n- 25M `aggregate_join_groupby` bench: byte-exact result vs serial\nDataFusion at N=2/4/8/10.\n- Build-side all-gather is exercised at all N in the regression suite\n(workers vs leader-only-writer paths).\n\n---------\n\nCo-authored-by: paradedb-bot <developers@paradedb.com>\nCo-authored-by: paradedb-github-app[bot] <282009505+paradedb-github-app[bot]@users.noreply.github.com>",
          "timestamp": "2026-05-12T16:47:00-07:00",
          "tree_id": "38dfa579bfcab58f4b6b66ddea91de57c32f5204",
          "url": "https://github.com/paradedb/paradedb/commit/6bdea0d414fb563f086ddfe56712b2960d200626"
        },
        "date": 1778633609847,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.658894,
            "unit": "median cpu",
            "extra": "avg cpu: 19.91552624424435, max cpu: 42.72997, count: 55647"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 172.78515625,
            "unit": "median mem",
            "extra": "avg mem: 153.00862222188528, max mem: 179.23828125, count: 55647"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 7.685696606198373, max cpu: 37.137333, count: 55647"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 120.93359375,
            "unit": "median mem",
            "extra": "avg mem: 119.70391471575287, max mem: 121.01171875, count: 55647"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 6.281592376893891, max cpu: 18.658894, count: 55647"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 168.26171875,
            "unit": "median mem",
            "extra": "avg mem: 145.81559779165994, max mem: 179.953125, count: 55647"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 16563,
            "unit": "median block_count",
            "extra": "avg block_count: 16920.578557694036, max block_count: 31369.0, count: 55647"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.288702196949587, max cpu: 4.692082, count: 55647"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 105.9140625,
            "unit": "median mem",
            "extra": "avg mem: 95.52118108905242, max mem: 137.1640625, count: 55647"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 24,
            "unit": "median segment_count",
            "extra": "avg segment_count: 24.495875788452206, max segment_count: 37.0, count: 55647"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.275363,
            "unit": "median cpu",
            "extra": "avg cpu: 9.03974301383281, max cpu: 37.137333, count: 111294"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 180.3203125,
            "unit": "median mem",
            "extra": "avg mem: 162.61422283686676, max mem: 183.74609375, count: 111294"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.899614,
            "unit": "median cpu",
            "extra": "avg cpu: 12.451242477324495, max cpu: 27.799229, count: 55647"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 173.625,
            "unit": "median mem",
            "extra": "avg mem: 170.95862422951822, max mem: 174.453125, count: 55647"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "james.sewell@gmail.com",
            "name": "James Sewell",
            "username": "jamessewell"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "8182eaf110c30cbefe008197caa40efa8b44f8e0",
          "message": "refactor: use existing FFHelper ctid cache instead of dedicated cache (#4905)\n\nFix a performance regression introduced in e0804b347 (#4765) which\nremoved ctid from SearchIndexScore and switched to lazy per-row\nresolution.\n\nPrior to #4765, ctid was resolved during result construction and carried\nin `SearchIndexScore` — no per-row fast-field lookups needed. #4765\nmoved ctid resolution to the consumption side (top_k.rs, normal.rs,\nscan.rs) using a single-entry `Option<(SegmentOrdinal, FFType)>` cache.\nWhen TopK results interleave across segments (sorted by score), every\nsegment transition re-opens the ctid column via `FastFieldReaders::u64\n-> DynamicColumnHandle::open -> BlockwiseLinearCodec::load`, which is\nvery expensive. Profiling showed 45% of total cycles spent in this\nre-open path.\n\nThe columnar scan path (`ColumnarExecState`) was unaffected — it already\nused `FFHelper`'s per-segment `OnceLock` ctid cache. This PR brings the\nremaining paths in line:\n\n- `scan.rs` uses its existing `Bm25ScanState.fast_fields` FFHelper\n- `normal.rs` and `top_k.rs` use a new `ctid_cache` FFHelper on\n`BaseScanState`\n\nEach segment's ctid column is opened at most once via `OnceLock`,\neliminating the thrashing. `FFHelper` has had this per-segment ctid\ncaching built in since cb78f0ca2 (Oct 2024).",
          "timestamp": "2026-05-13T12:18:31+12:00",
          "tree_id": "814e1da895eec41e0dfe3cbb5348bdb237811bf7",
          "url": "https://github.com/paradedb/paradedb/commit/8182eaf110c30cbefe008197caa40efa8b44f8e0"
        },
        "date": 1778635509599,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.568666,
            "unit": "median cpu",
            "extra": "avg cpu: 19.94787458854853, max cpu: 46.10951, count: 55456"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 178.7578125,
            "unit": "median mem",
            "extra": "avg mem: 176.55455993546687, max mem: 179.015625, count: 55456"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 7.716075203023097, max cpu: 36.887608, count: 55456"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 120.08984375,
            "unit": "median mem",
            "extra": "avg mem: 118.83833065572256, max mem: 120.19921875, count: 55456"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 6.182231062785267, max cpu: 18.60465, count: 55456"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 173.76171875,
            "unit": "median mem",
            "extra": "avg mem: 147.53501685176084, max mem: 179.84375, count: 55456"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 16411,
            "unit": "median block_count",
            "extra": "avg block_count: 16799.06895556838, max block_count: 31120.0, count: 55456"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.619827,
            "unit": "median cpu",
            "extra": "avg cpu: 4.333558928004811, max cpu: 4.7058825, count: 55456"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 110.50390625,
            "unit": "median mem",
            "extra": "avg mem: 96.90532084276273, max mem: 137.015625, count: 55456"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 25,
            "unit": "median segment_count",
            "extra": "avg segment_count: 24.78285487593768, max segment_count: 36.0, count: 55456"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.230769,
            "unit": "median cpu",
            "extra": "avg cpu: 8.980272341157027, max cpu: 36.887608, count: 110912"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 179.05078125,
            "unit": "median mem",
            "extra": "avg mem: 163.2198084475192, max mem: 183.58203125, count: 110912"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.779904,
            "unit": "median cpu",
            "extra": "avg cpu: 11.496384244944181, max cpu: 23.460411, count: 55456"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 173.39453125,
            "unit": "median mem",
            "extra": "avg mem: 170.55819901137838, max mem: 174.55078125, count: 55456"
          }
        ]
      }
    ],
    "pg_search logical-replication.toml Performance - TPS": [
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
        "date": 1778527048524,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - tps",
            "value": 537.2275568087026,
            "unit": "median tps",
            "extra": "avg tps: 540.1706693811136, max tps: 671.2275746815696, count: 53877"
          },
          {
            "name": "Index Only Scan - Subscriber - tps",
            "value": 597.939357438685,
            "unit": "median tps",
            "extra": "avg tps: 600.4485752330294, max tps: 767.1860767020058, count: 53877"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - tps",
            "value": 91.49920949709565,
            "unit": "median tps",
            "extra": "avg tps: 91.67578211175103, max tps: 99.8849659182976, count: 53877"
          },
          {
            "name": "Top K - Subscriber - tps",
            "value": 260.02897344632083,
            "unit": "median tps",
            "extra": "avg tps: 255.4911100902889, max tps: 526.8993536174363, count: 107754"
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
        "date": 1778527365723,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - tps",
            "value": 610.6675008039614,
            "unit": "median tps",
            "extra": "avg tps: 612.3750299230651, max tps: 824.7725522155341, count: 53891"
          },
          {
            "name": "Index Only Scan - Subscriber - tps",
            "value": 624.6840421915658,
            "unit": "median tps",
            "extra": "avg tps: 628.0882989976594, max tps: 929.3315432005458, count: 53891"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - tps",
            "value": 91.42308117407508,
            "unit": "median tps",
            "extra": "avg tps: 91.64990218621483, max tps: 101.42854162999988, count: 53891"
          },
          {
            "name": "Top K - Subscriber - tps",
            "value": 285.82457444936955,
            "unit": "median tps",
            "extra": "avg tps: 269.5117849223434, max tps: 578.5017425994861, count: 107782"
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
        "date": 1778527533434,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - tps",
            "value": 596.2171982938285,
            "unit": "median tps",
            "extra": "avg tps: 601.0952684419645, max tps: 805.9445622549226, count: 53866"
          },
          {
            "name": "Index Only Scan - Subscriber - tps",
            "value": 592.424314053784,
            "unit": "median tps",
            "extra": "avg tps: 600.1631546774296, max tps: 829.0121742309749, count: 53866"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - tps",
            "value": 88.6304175959784,
            "unit": "median tps",
            "extra": "avg tps: 88.86868089803669, max tps: 102.31154758209422, count: 53866"
          },
          {
            "name": "Top K - Subscriber - tps",
            "value": 269.60369601797794,
            "unit": "median tps",
            "extra": "avg tps: 262.898741366857, max tps: 654.7080338998861, count: 107732"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mdashti@gmail.com",
            "name": "Moe",
            "username": "mdashti"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "6bdea0d414fb563f086ddfe56712b2960d200626",
          "message": "feat(mpp): AggregateScan in-process MPP via custom shm_mq transport (#4988)\n\n# Ticket(s) Closed\n\n- Closes #4152\n\n## What\n\nAdds an MPP execution path for AggregateScan that runs\njoin-with-aggregate shapes inside Postgres parallel-worker processes via\na custom transport on top of `paradedb/datafusion-distributed`. Default\noff behind `paradedb.enable_mpp` (minimum `mpp_worker_count = 3`). Other\nshapes and the JoinScan path are unchanged.\n\n## Why\n\nSingle-process DataFusion bottlenecks on join-with-aggregate at scale.\nDistributing the producer fragment across PG parallel workers gives us\n1.55–1.92× speedup on the 25M `aggregate_join_groupby` bench at N=2/4\nwithout leaving the embedded model — every worker is still a real PG\nprocess with its own snapshot, no gRPC, no extra daemon.\n\n## How\n\n- Leader builds the logical plan and stashes it in a DSM segment\nalongside an N-way `shm_mq` mesh.\n- Workers attach, deserialize, and re-plan with the same `SessionState`.\nIdentical inputs ⇒ structurally identical physical plans on every\nworker, so we don't need to serialize physical subplans.\n- Each worker runs its producer fragment and pushes batches through its\noutbound queue. Leader runs `NetworkShuffleExec` + final aggregate and\nreturns rows to the client. Leader is consumer-only in this iteration.\n- Build side (non-partitioning sources like `HashJoinExec(CollectLeft)`)\nis split via DSM all-gather: each worker scans its 1/N slice, writes to\na per-worker DSM region, completion-flag barrier, then everyone reads\nevery slice. Build is fully parallel; no leader-side serial scan.\n- The fork emits the network operators (`NetworkShuffleExec`,\n`NetworkBroadcastExec`, `NetworkCoalesceExec`) and we register a custom\ntransport that short-circuits the gRPC dialer. The fork's in-process\ntwo-boundary planner distinguishes outer (worker → leader, N producers)\nfrom nested (single local producer) Network boundaries.\n\n## Reviewer's Guide\n\nSuggested reading order — most of the diff lives under\n`pg_search/src/postgres/customscan/mpp/`:\n\n1. `mpp/dsm.rs` — the DSM layout: header, queue mesh, build-cache\nregion. `compute_dsm_layout` is the math; `leader_init` /\n`worker_attach` are the unsafe FFI boundaries.\n2. `mpp/runtime.rs` — `MppMesh` (runtime handle), `ShmMqWorkerTransport`\n(the `WorkerTransport` impl the leader registers),\n`LocalExecWorkerTransport` (the worker-side stub for nested broadcasts),\n`MppWorkerResolver`.\n3. `mpp/transport.rs` — `DrainHandle` and the cooperative-pull\nprimitives. The drain runs inline on the backend thread because pgrx\n0.18 enforces single-threaded Postgres FFI.\n4. `mpp/glue.rs` — the public API the customscan calls:\n`estimate_dsm_size`, `leader_setup`, `worker_setup`. Thin wrappers\naround dsm/runtime.\n5. `mpp/exec.rs` — `run_producer_fragment` is the worker push loop.\n6. `aggregatescan/mod.rs` — the integration: `stash_mpp_plan_bytes`,\n`exec_mpp_worker`, `build_mpp_leader_session_context`, the\n`ParallelQueryCapable` impl, the `parallel_workers` clamp in\n`try_build_datafusion_aggregate_path`. Both leader and worker session\ncontexts call `with_distributed_in_process_mode(true)` explicitly.\n\nThe build-side all-gather lives in\n`aggregatescan/mod.rs::exec_mpp_worker` and `mpp/dsm.rs` (the cache\nregion). Read those together.\n\n## Tests\n\n- pgrx regression suite — new `mpp_aggregate.sql` covers correctness on\nthe join-with-aggregate shapes; existing `mpp_*` suites still pass.\n- 25M `aggregate_join_groupby` bench: byte-exact result vs serial\nDataFusion at N=2/4/8/10.\n- Build-side all-gather is exercised at all N in the regression suite\n(workers vs leader-only-writer paths).\n\n---------\n\nCo-authored-by: paradedb-bot <developers@paradedb.com>\nCo-authored-by: paradedb-github-app[bot] <282009505+paradedb-github-app[bot]@users.noreply.github.com>",
          "timestamp": "2026-05-12T16:47:00-07:00",
          "tree_id": "38dfa579bfcab58f4b6b66ddea91de57c32f5204",
          "url": "https://github.com/paradedb/paradedb/commit/6bdea0d414fb563f086ddfe56712b2960d200626"
        },
        "date": 1778634257638,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - tps",
            "value": 595.8950218679295,
            "unit": "median tps",
            "extra": "avg tps: 596.9483225503457, max tps: 739.5217704116383, count: 53907"
          },
          {
            "name": "Index Only Scan - Subscriber - tps",
            "value": 631.0980525800898,
            "unit": "median tps",
            "extra": "avg tps: 632.95239056308, max tps: 796.1922340580361, count: 53907"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - tps",
            "value": 89.68304006015221,
            "unit": "median tps",
            "extra": "avg tps: 89.85029697087066, max tps: 102.2987066686527, count: 53907"
          },
          {
            "name": "Top K - Subscriber - tps",
            "value": 279.12537518017217,
            "unit": "median tps",
            "extra": "avg tps: 272.8891289335118, max tps: 591.9862923241946, count: 107814"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "james.sewell@gmail.com",
            "name": "James Sewell",
            "username": "jamessewell"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "8182eaf110c30cbefe008197caa40efa8b44f8e0",
          "message": "refactor: use existing FFHelper ctid cache instead of dedicated cache (#4905)\n\nFix a performance regression introduced in e0804b347 (#4765) which\nremoved ctid from SearchIndexScore and switched to lazy per-row\nresolution.\n\nPrior to #4765, ctid was resolved during result construction and carried\nin `SearchIndexScore` — no per-row fast-field lookups needed. #4765\nmoved ctid resolution to the consumption side (top_k.rs, normal.rs,\nscan.rs) using a single-entry `Option<(SegmentOrdinal, FFType)>` cache.\nWhen TopK results interleave across segments (sorted by score), every\nsegment transition re-opens the ctid column via `FastFieldReaders::u64\n-> DynamicColumnHandle::open -> BlockwiseLinearCodec::load`, which is\nvery expensive. Profiling showed 45% of total cycles spent in this\nre-open path.\n\nThe columnar scan path (`ColumnarExecState`) was unaffected — it already\nused `FFHelper`'s per-segment `OnceLock` ctid cache. This PR brings the\nremaining paths in line:\n\n- `scan.rs` uses its existing `Bm25ScanState.fast_fields` FFHelper\n- `normal.rs` and `top_k.rs` use a new `ctid_cache` FFHelper on\n`BaseScanState`\n\nEach segment's ctid column is opened at most once via `OnceLock`,\neliminating the thrashing. `FFHelper` has had this per-segment ctid\ncaching built in since cb78f0ca2 (Oct 2024).",
          "timestamp": "2026-05-13T12:18:31+12:00",
          "tree_id": "814e1da895eec41e0dfe3cbb5348bdb237811bf7",
          "url": "https://github.com/paradedb/paradedb/commit/8182eaf110c30cbefe008197caa40efa8b44f8e0"
        },
        "date": 1778636159675,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - tps",
            "value": 598.5362663422208,
            "unit": "median tps",
            "extra": "avg tps: 597.2453042265969, max tps: 813.2693734337558, count: 53855"
          },
          {
            "name": "Index Only Scan - Subscriber - tps",
            "value": 621.4645104590571,
            "unit": "median tps",
            "extra": "avg tps: 619.4992466752802, max tps: 795.0138657645589, count: 53855"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - tps",
            "value": 85.4806333378026,
            "unit": "median tps",
            "extra": "avg tps: 85.60652280002031, max tps: 101.01415038818658, count: 53855"
          },
          {
            "name": "Top K - Subscriber - tps",
            "value": 254.70333069674672,
            "unit": "median tps",
            "extra": "avg tps: 252.07085183968482, max tps: 575.3361871249135, count: 107710"
          }
        ]
      }
    ],
    "pg_search logical-replication.toml Performance - Other Metrics": [
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
        "date": 1778527099837,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - cpu",
            "value": 4.5801525,
            "unit": "median cpu",
            "extra": "avg cpu: 5.047102790522935, max cpu: 9.275363, count: 53877"
          },
          {
            "name": "Custom Scan - Subscriber - mem",
            "value": 50.5390625,
            "unit": "median mem",
            "extra": "avg mem: 50.66569439997587, max mem: 56.41015625, count: 53877"
          },
          {
            "name": "Delete values - Publisher - cpu",
            "value": 4.567079,
            "unit": "median cpu",
            "extra": "avg cpu: 4.118317203527869, max cpu: 4.5845275, count: 53877"
          },
          {
            "name": "Delete values - Publisher - mem",
            "value": 31.59375,
            "unit": "median mem",
            "extra": "avg mem: 30.92719825076099, max mem: 32.0, count: 53877"
          },
          {
            "name": "Find by ctid - Subscriber - cpu",
            "value": 9.142857,
            "unit": "median cpu",
            "extra": "avg cpu: 8.604954582151503, max cpu: 18.479307, count: 53877"
          },
          {
            "name": "Find by ctid - Subscriber - mem",
            "value": 54.19140625,
            "unit": "median mem",
            "extra": "avg mem: 53.93961114552592, max mem: 60.0078125, count: 53877"
          },
          {
            "name": "Index Only Scan - Subscriber - cpu",
            "value": 4.5801525,
            "unit": "median cpu",
            "extra": "avg cpu: 5.060979042186233, max cpu: 9.257474, count: 53877"
          },
          {
            "name": "Index Only Scan - Subscriber - mem",
            "value": 50.11328125,
            "unit": "median mem",
            "extra": "avg mem: 50.24837709040964, max mem: 56.046875, count: 53877"
          },
          {
            "name": "Index Size Info - Subscriber - cpu",
            "value": 4.5757866,
            "unit": "median cpu",
            "extra": "avg cpu: 4.595558084444877, max cpu: 9.195402, count: 53877"
          },
          {
            "name": "Index Size Info - Subscriber - mem",
            "value": 33.08984375,
            "unit": "median mem",
            "extra": "avg mem: 33.15489578693134, max mem: 38.09765625, count: 53877"
          },
          {
            "name": "Index Size Info - Subscriber - pages",
            "value": 1086,
            "unit": "median pages",
            "extra": "avg pages: 1098.0913562373555, max pages: 1811.0, count: 53877"
          },
          {
            "name": "Index Size Info - Subscriber - relation_size:MB",
            "value": 8.484375,
            "unit": "median relation_size:MB",
            "extra": "avg relation_size:MB: 8.578838865610557, max relation_size:MB: 14.1484375, count: 53877"
          },
          {
            "name": "Index Size Info - Subscriber - segment_count",
            "value": 9,
            "unit": "median segment_count",
            "extra": "avg segment_count: 9.065018467991907, max segment_count: 16.0, count: 53877"
          },
          {
            "name": "Insert value A - Publisher - cpu",
            "value": 4.5714283,
            "unit": "median cpu",
            "extra": "avg cpu: 4.53616781890249, max cpu: 4.6021094, count: 53877"
          },
          {
            "name": "Insert value A - Publisher - mem",
            "value": 29.56640625,
            "unit": "median mem",
            "extra": "avg mem: 28.846571313477924, max mem: 29.91796875, count: 53877"
          },
          {
            "name": "Insert value B - Publisher - cpu",
            "value": 4.5933013,
            "unit": "median cpu",
            "extra": "avg cpu: 4.324510228216321, max cpu: 4.6153846, count: 53877"
          },
          {
            "name": "Insert value B - Publisher - mem",
            "value": 29.5703125,
            "unit": "median mem",
            "extra": "avg mem: 28.860362202331235, max mem: 29.921875, count: 53877"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - cpu",
            "value": 4.5933013,
            "unit": "median cpu",
            "extra": "avg cpu: 6.470479780514458, max cpu: 31.728045, count: 53877"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - mem",
            "value": 48.3046875,
            "unit": "median mem",
            "extra": "avg mem: 48.42098741694044, max mem: 54.24609375, count: 53877"
          },
          {
            "name": "SELECT\n  pid,\n  pg_wal_lsn_diff(sent_lsn, replay_lsn) AS replication_lag,\n  application_name::text,\n  state::text\nFROM pg_stat_replication; - Publisher - replication_lag:MB",
            "value": 0,
            "unit": "median replication_lag:MB",
            "extra": "avg replication_lag:MB: 0.0000241215860914241, max replication_lag:MB: 0.29253387451171875, count: 53877"
          },
          {
            "name": "Top K - Subscriber - cpu",
            "value": 4.5801525,
            "unit": "median cpu",
            "extra": "avg cpu: 5.344216758348764, max cpu: 13.88621, count: 107754"
          },
          {
            "name": "Top K - Subscriber - mem",
            "value": 49.01171875,
            "unit": "median mem",
            "extra": "avg mem: 49.09160630057817, max mem: 55.16796875, count: 107754"
          },
          {
            "name": "Update 1..9 - Publisher - cpu",
            "value": 4.562738,
            "unit": "median cpu",
            "extra": "avg cpu: 4.514309341621512, max cpu: 4.5933013, count: 53877"
          },
          {
            "name": "Update 1..9 - Publisher - mem",
            "value": 32.23828125,
            "unit": "median mem",
            "extra": "avg mem: 31.580400437570763, max mem: 32.6484375, count: 53877"
          },
          {
            "name": "Update 10,11 - Publisher - cpu",
            "value": 4.562738,
            "unit": "median cpu",
            "extra": "avg cpu: 4.523756338008626, max cpu: 4.6021094, count: 53877"
          },
          {
            "name": "Update 10,11 - Publisher - mem",
            "value": 32.41796875,
            "unit": "median mem",
            "extra": "avg mem: 31.75117578291757, max mem: 32.5234375, count: 53877"
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
        "date": 1778527416928,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - cpu",
            "value": 4.5757866,
            "unit": "median cpu",
            "extra": "avg cpu: 5.104556199972011, max cpu: 9.275363, count: 53891"
          },
          {
            "name": "Custom Scan - Subscriber - mem",
            "value": 51.96875,
            "unit": "median mem",
            "extra": "avg mem: 51.983461480697144, max mem: 57.84375, count: 53891"
          },
          {
            "name": "Delete values - Publisher - cpu",
            "value": 4.5584044,
            "unit": "median cpu",
            "extra": "avg cpu: 2.9978297732800767, max cpu: 4.5714283, count: 53891"
          },
          {
            "name": "Delete values - Publisher - mem",
            "value": 32.08984375,
            "unit": "median mem",
            "extra": "avg mem: 31.36674425866564, max mem: 32.38671875, count: 53891"
          },
          {
            "name": "Find by ctid - Subscriber - cpu",
            "value": 9.108159,
            "unit": "median cpu",
            "extra": "avg cpu: 7.657590128985056, max cpu: 18.373205, count: 53891"
          },
          {
            "name": "Find by ctid - Subscriber - mem",
            "value": 55.39453125,
            "unit": "median mem",
            "extra": "avg mem: 55.03570633315396, max mem: 61.20703125, count: 53891"
          },
          {
            "name": "Index Only Scan - Subscriber - cpu",
            "value": 4.5757866,
            "unit": "median cpu",
            "extra": "avg cpu: 5.058691302248854, max cpu: 9.257474, count: 53891"
          },
          {
            "name": "Index Only Scan - Subscriber - mem",
            "value": 51.58984375,
            "unit": "median mem",
            "extra": "avg mem: 51.58910346811156, max mem: 57.4609375, count: 53891"
          },
          {
            "name": "Index Size Info - Subscriber - cpu",
            "value": 4.5757866,
            "unit": "median cpu",
            "extra": "avg cpu: 4.600185046572753, max cpu: 9.142857, count: 53891"
          },
          {
            "name": "Index Size Info - Subscriber - mem",
            "value": 33.40234375,
            "unit": "median mem",
            "extra": "avg mem: 33.420079709506226, max mem: 38.62890625, count: 53891"
          },
          {
            "name": "Index Size Info - Subscriber - pages",
            "value": 1112,
            "unit": "median pages",
            "extra": "avg pages: 1109.9379859345715, max pages: 1842.0, count: 53891"
          },
          {
            "name": "Index Size Info - Subscriber - relation_size:MB",
            "value": 8.6875,
            "unit": "median relation_size:MB",
            "extra": "avg relation_size:MB: 8.67139051511384, max relation_size:MB: 14.390625, count: 53891"
          },
          {
            "name": "Index Size Info - Subscriber - segment_count",
            "value": 7,
            "unit": "median segment_count",
            "extra": "avg segment_count: 6.711436046835279, max segment_count: 12.0, count: 53891"
          },
          {
            "name": "Insert value A - Publisher - cpu",
            "value": 4.5540795,
            "unit": "median cpu",
            "extra": "avg cpu: 4.039679124584621, max cpu: 4.58891, count: 53891"
          },
          {
            "name": "Insert value A - Publisher - mem",
            "value": 29.45703125,
            "unit": "median mem",
            "extra": "avg mem: 28.715580045253382, max mem: 29.80859375, count: 53891"
          },
          {
            "name": "Insert value B - Publisher - cpu",
            "value": 4.6065254,
            "unit": "median cpu",
            "extra": "avg cpu: 4.024887155221776, max cpu: 4.6065254, count: 53891"
          },
          {
            "name": "Insert value B - Publisher - mem",
            "value": 29.4140625,
            "unit": "median mem",
            "extra": "avg mem: 28.675979639457424, max mem: 29.75390625, count: 53891"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - cpu",
            "value": 4.58891,
            "unit": "median cpu",
            "extra": "avg cpu: 6.378180092560301, max cpu: 27.639154, count: 53891"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - mem",
            "value": 50.015625,
            "unit": "median mem",
            "extra": "avg mem: 50.02135886847989, max mem: 55.88671875, count: 53891"
          },
          {
            "name": "SELECT\n  pid,\n  pg_wal_lsn_diff(sent_lsn, replay_lsn) AS replication_lag,\n  application_name::text,\n  state::text\nFROM pg_stat_replication; - Publisher - replication_lag:MB",
            "value": 0,
            "unit": "median replication_lag:MB",
            "extra": "avg replication_lag:MB: 0.00002134860062948243, max replication_lag:MB: 0.213165283203125, count: 53891"
          },
          {
            "name": "Top K - Subscriber - cpu",
            "value": 4.5801525,
            "unit": "median cpu",
            "extra": "avg cpu: 5.290599671464832, max cpu: 13.740458, count: 107782"
          },
          {
            "name": "Top K - Subscriber - mem",
            "value": 51.30859375,
            "unit": "median mem",
            "extra": "avg mem: 51.26902487486315, max mem: 57.66796875, count: 107782"
          },
          {
            "name": "Update 1..9 - Publisher - cpu",
            "value": 4.5540795,
            "unit": "median cpu",
            "extra": "avg cpu: 4.230076041254057, max cpu: 4.6065254, count: 53891"
          },
          {
            "name": "Update 1..9 - Publisher - mem",
            "value": 32.92578125,
            "unit": "median mem",
            "extra": "avg mem: 32.197794071598224, max mem: 33.23828125, count: 53891"
          },
          {
            "name": "Update 10,11 - Publisher - cpu",
            "value": 4.567079,
            "unit": "median cpu",
            "extra": "avg cpu: 4.296560902979441, max cpu: 4.58891, count: 53891"
          },
          {
            "name": "Update 10,11 - Publisher - mem",
            "value": 32.99609375,
            "unit": "median mem",
            "extra": "avg mem: 32.24848558606725, max mem: 33.1015625, count: 53891"
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
        "date": 1778527584400,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - cpu",
            "value": 4.567079,
            "unit": "median cpu",
            "extra": "avg cpu: 5.078461961290287, max cpu: 9.239654, count: 53866"
          },
          {
            "name": "Custom Scan - Subscriber - mem",
            "value": 52.57421875,
            "unit": "median mem",
            "extra": "avg mem: 52.65618335603627, max mem: 58.6171875, count: 53866"
          },
          {
            "name": "Delete values - Publisher - cpu",
            "value": 4.5411544,
            "unit": "median cpu",
            "extra": "avg cpu: 3.79997508084782, max cpu: 4.5757866, count: 53866"
          },
          {
            "name": "Delete values - Publisher - mem",
            "value": 32.2421875,
            "unit": "median mem",
            "extra": "avg mem: 31.530350632820333, max mem: 32.53515625, count: 53866"
          },
          {
            "name": "Find by ctid - Subscriber - cpu",
            "value": 9.108159,
            "unit": "median cpu",
            "extra": "avg cpu: 8.5002061256766, max cpu: 18.35564, count: 53866"
          },
          {
            "name": "Find by ctid - Subscriber - mem",
            "value": 55.3359375,
            "unit": "median mem",
            "extra": "avg mem: 55.055527765079084, max mem: 61.2890625, count: 53866"
          },
          {
            "name": "Index Only Scan - Subscriber - cpu",
            "value": 4.567079,
            "unit": "median cpu",
            "extra": "avg cpu: 5.061523749388442, max cpu: 9.338522, count: 53866"
          },
          {
            "name": "Index Only Scan - Subscriber - mem",
            "value": 51.7265625,
            "unit": "median mem",
            "extra": "avg mem: 51.80008725355512, max mem: 57.73046875, count: 53866"
          },
          {
            "name": "Index Size Info - Subscriber - cpu",
            "value": 4.562738,
            "unit": "median cpu",
            "extra": "avg cpu: 4.591616459002924, max cpu: 9.186603, count: 53866"
          },
          {
            "name": "Index Size Info - Subscriber - mem",
            "value": 33.55859375,
            "unit": "median mem",
            "extra": "avg mem: 33.568756338065754, max mem: 38.859375, count: 53866"
          },
          {
            "name": "Index Size Info - Subscriber - pages",
            "value": 1109,
            "unit": "median pages",
            "extra": "avg pages: 1114.3518917313334, max pages: 1856.0, count: 53866"
          },
          {
            "name": "Index Size Info - Subscriber - relation_size:MB",
            "value": 8.6640625,
            "unit": "median relation_size:MB",
            "extra": "avg relation_size:MB: 8.70587429918687, max relation_size:MB: 14.5, count: 53866"
          },
          {
            "name": "Index Size Info - Subscriber - segment_count",
            "value": 10,
            "unit": "median segment_count",
            "extra": "avg segment_count: 10.419689600118813, max segment_count: 22.0, count: 53866"
          },
          {
            "name": "Insert value A - Publisher - cpu",
            "value": 4.5368624,
            "unit": "median cpu",
            "extra": "avg cpu: 4.438022069856933, max cpu: 4.6021094, count: 53866"
          },
          {
            "name": "Insert value A - Publisher - mem",
            "value": 29.390625,
            "unit": "median mem",
            "extra": "avg mem: 28.65369591903984, max mem: 29.71875, count: 53866"
          },
          {
            "name": "Insert value B - Publisher - cpu",
            "value": 4.5411544,
            "unit": "median cpu",
            "extra": "avg cpu: 3.682498184274365, max cpu: 4.5757866, count: 53866"
          },
          {
            "name": "Insert value B - Publisher - mem",
            "value": 29.41015625,
            "unit": "median mem",
            "extra": "avg mem: 28.7012354732113, max mem: 29.7578125, count: 53866"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - cpu",
            "value": 4.5845275,
            "unit": "median cpu",
            "extra": "avg cpu: 6.632233330016474, max cpu: 27.533463, count: 53866"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - mem",
            "value": 50.6640625,
            "unit": "median mem",
            "extra": "avg mem: 50.69524201258679, max mem: 56.62109375, count: 53866"
          },
          {
            "name": "SELECT\n  pid,\n  pg_wal_lsn_diff(sent_lsn, replay_lsn) AS replication_lag,\n  application_name::text,\n  state::text\nFROM pg_stat_replication; - Publisher - replication_lag:MB",
            "value": 0,
            "unit": "median replication_lag:MB",
            "extra": "avg replication_lag:MB: 0.000023121317358990366, max replication_lag:MB: 0.18235015869140625, count: 53866"
          },
          {
            "name": "Top K - Subscriber - cpu",
            "value": 4.567079,
            "unit": "median cpu",
            "extra": "avg cpu: 5.233239849240295, max cpu: 13.779904, count: 107732"
          },
          {
            "name": "Top K - Subscriber - mem",
            "value": 51.75390625,
            "unit": "median mem",
            "extra": "avg mem: 51.79689251307643, max mem: 57.9296875, count: 107732"
          },
          {
            "name": "Update 1..9 - Publisher - cpu",
            "value": 4.5540795,
            "unit": "median cpu",
            "extra": "avg cpu: 3.4415202026343037, max cpu: 4.6021094, count: 53866"
          },
          {
            "name": "Update 1..9 - Publisher - mem",
            "value": 32.88671875,
            "unit": "median mem",
            "extra": "avg mem: 32.16125090792429, max mem: 33.23828125, count: 53866"
          },
          {
            "name": "Update 10,11 - Publisher - cpu",
            "value": 4.5540795,
            "unit": "median cpu",
            "extra": "avg cpu: 4.059273250575732, max cpu: 4.5933013, count: 53866"
          },
          {
            "name": "Update 10,11 - Publisher - mem",
            "value": 32.81640625,
            "unit": "median mem",
            "extra": "avg mem: 32.13750571441169, max mem: 32.90234375, count: 53866"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mdashti@gmail.com",
            "name": "Moe",
            "username": "mdashti"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "6bdea0d414fb563f086ddfe56712b2960d200626",
          "message": "feat(mpp): AggregateScan in-process MPP via custom shm_mq transport (#4988)\n\n# Ticket(s) Closed\n\n- Closes #4152\n\n## What\n\nAdds an MPP execution path for AggregateScan that runs\njoin-with-aggregate shapes inside Postgres parallel-worker processes via\na custom transport on top of `paradedb/datafusion-distributed`. Default\noff behind `paradedb.enable_mpp` (minimum `mpp_worker_count = 3`). Other\nshapes and the JoinScan path are unchanged.\n\n## Why\n\nSingle-process DataFusion bottlenecks on join-with-aggregate at scale.\nDistributing the producer fragment across PG parallel workers gives us\n1.55–1.92× speedup on the 25M `aggregate_join_groupby` bench at N=2/4\nwithout leaving the embedded model — every worker is still a real PG\nprocess with its own snapshot, no gRPC, no extra daemon.\n\n## How\n\n- Leader builds the logical plan and stashes it in a DSM segment\nalongside an N-way `shm_mq` mesh.\n- Workers attach, deserialize, and re-plan with the same `SessionState`.\nIdentical inputs ⇒ structurally identical physical plans on every\nworker, so we don't need to serialize physical subplans.\n- Each worker runs its producer fragment and pushes batches through its\noutbound queue. Leader runs `NetworkShuffleExec` + final aggregate and\nreturns rows to the client. Leader is consumer-only in this iteration.\n- Build side (non-partitioning sources like `HashJoinExec(CollectLeft)`)\nis split via DSM all-gather: each worker scans its 1/N slice, writes to\na per-worker DSM region, completion-flag barrier, then everyone reads\nevery slice. Build is fully parallel; no leader-side serial scan.\n- The fork emits the network operators (`NetworkShuffleExec`,\n`NetworkBroadcastExec`, `NetworkCoalesceExec`) and we register a custom\ntransport that short-circuits the gRPC dialer. The fork's in-process\ntwo-boundary planner distinguishes outer (worker → leader, N producers)\nfrom nested (single local producer) Network boundaries.\n\n## Reviewer's Guide\n\nSuggested reading order — most of the diff lives under\n`pg_search/src/postgres/customscan/mpp/`:\n\n1. `mpp/dsm.rs` — the DSM layout: header, queue mesh, build-cache\nregion. `compute_dsm_layout` is the math; `leader_init` /\n`worker_attach` are the unsafe FFI boundaries.\n2. `mpp/runtime.rs` — `MppMesh` (runtime handle), `ShmMqWorkerTransport`\n(the `WorkerTransport` impl the leader registers),\n`LocalExecWorkerTransport` (the worker-side stub for nested broadcasts),\n`MppWorkerResolver`.\n3. `mpp/transport.rs` — `DrainHandle` and the cooperative-pull\nprimitives. The drain runs inline on the backend thread because pgrx\n0.18 enforces single-threaded Postgres FFI.\n4. `mpp/glue.rs` — the public API the customscan calls:\n`estimate_dsm_size`, `leader_setup`, `worker_setup`. Thin wrappers\naround dsm/runtime.\n5. `mpp/exec.rs` — `run_producer_fragment` is the worker push loop.\n6. `aggregatescan/mod.rs` — the integration: `stash_mpp_plan_bytes`,\n`exec_mpp_worker`, `build_mpp_leader_session_context`, the\n`ParallelQueryCapable` impl, the `parallel_workers` clamp in\n`try_build_datafusion_aggregate_path`. Both leader and worker session\ncontexts call `with_distributed_in_process_mode(true)` explicitly.\n\nThe build-side all-gather lives in\n`aggregatescan/mod.rs::exec_mpp_worker` and `mpp/dsm.rs` (the cache\nregion). Read those together.\n\n## Tests\n\n- pgrx regression suite — new `mpp_aggregate.sql` covers correctness on\nthe join-with-aggregate shapes; existing `mpp_*` suites still pass.\n- 25M `aggregate_join_groupby` bench: byte-exact result vs serial\nDataFusion at N=2/4/8/10.\n- Build-side all-gather is exercised at all N in the regression suite\n(workers vs leader-only-writer paths).\n\n---------\n\nCo-authored-by: paradedb-bot <developers@paradedb.com>\nCo-authored-by: paradedb-github-app[bot] <282009505+paradedb-github-app[bot]@users.noreply.github.com>",
          "timestamp": "2026-05-12T16:47:00-07:00",
          "tree_id": "38dfa579bfcab58f4b6b66ddea91de57c32f5204",
          "url": "https://github.com/paradedb/paradedb/commit/6bdea0d414fb563f086ddfe56712b2960d200626"
        },
        "date": 1778634289464,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - cpu",
            "value": 4.5757866,
            "unit": "median cpu",
            "extra": "avg cpu: 5.082804762089665, max cpu: 9.266409, count: 53907"
          },
          {
            "name": "Custom Scan - Subscriber - mem",
            "value": 52.61328125,
            "unit": "median mem",
            "extra": "avg mem: 52.645079938018256, max mem: 58.625, count: 53907"
          },
          {
            "name": "Delete values - Publisher - cpu",
            "value": 4.5584044,
            "unit": "median cpu",
            "extra": "avg cpu: 3.8228658688937975, max cpu: 4.6021094, count: 53907"
          },
          {
            "name": "Delete values - Publisher - mem",
            "value": 32.33984375,
            "unit": "median mem",
            "extra": "avg mem: 31.70566866026212, max mem: 32.84765625, count: 53907"
          },
          {
            "name": "Find by ctid - Subscriber - cpu",
            "value": 9.090909,
            "unit": "median cpu",
            "extra": "avg cpu: 7.611902613832086, max cpu: 18.373205, count: 53907"
          },
          {
            "name": "Find by ctid - Subscriber - mem",
            "value": 55.703125,
            "unit": "median mem",
            "extra": "avg mem: 55.40385104786948, max mem: 61.73046875, count: 53907"
          },
          {
            "name": "Index Only Scan - Subscriber - cpu",
            "value": 4.5757866,
            "unit": "median cpu",
            "extra": "avg cpu: 5.029599207525417, max cpu: 9.257474, count: 53907"
          },
          {
            "name": "Index Only Scan - Subscriber - mem",
            "value": 52.26171875,
            "unit": "median mem",
            "extra": "avg mem: 52.324574542151296, max mem: 58.25, count: 53907"
          },
          {
            "name": "Index Size Info - Subscriber - cpu",
            "value": 4.5714283,
            "unit": "median cpu",
            "extra": "avg cpu: 4.6131271390796105, max cpu: 9.213051, count: 53907"
          },
          {
            "name": "Index Size Info - Subscriber - mem",
            "value": 33.9453125,
            "unit": "median mem",
            "extra": "avg mem: 34.04992263875749, max mem: 39.30078125, count: 53907"
          },
          {
            "name": "Index Size Info - Subscriber - pages",
            "value": 1115,
            "unit": "median pages",
            "extra": "avg pages: 1118.253288070195, max pages: 1857.0, count: 53907"
          },
          {
            "name": "Index Size Info - Subscriber - relation_size:MB",
            "value": 8.7109375,
            "unit": "median relation_size:MB",
            "extra": "avg relation_size:MB: 8.736353957973918, max relation_size:MB: 14.5078125, count: 53907"
          },
          {
            "name": "Index Size Info - Subscriber - segment_count",
            "value": 7,
            "unit": "median segment_count",
            "extra": "avg segment_count: 6.708572170590091, max segment_count: 12.0, count: 53907"
          },
          {
            "name": "Insert value A - Publisher - cpu",
            "value": 4.5411544,
            "unit": "median cpu",
            "extra": "avg cpu: 3.7501826367435984, max cpu: 4.6021094, count: 53907"
          },
          {
            "name": "Insert value A - Publisher - mem",
            "value": 29.390625,
            "unit": "median mem",
            "extra": "avg mem: 28.735466216701912, max mem: 29.80078125, count: 53907"
          },
          {
            "name": "Insert value B - Publisher - cpu",
            "value": 4.562738,
            "unit": "median cpu",
            "extra": "avg cpu: 4.041080792406923, max cpu: 4.6021094, count: 53907"
          },
          {
            "name": "Insert value B - Publisher - mem",
            "value": 29.21875,
            "unit": "median mem",
            "extra": "avg mem: 28.561703489342758, max mem: 29.6484375, count: 53907"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - cpu",
            "value": 4.58891,
            "unit": "median cpu",
            "extra": "avg cpu: 6.543067941190928, max cpu: 27.480915, count: 53907"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - mem",
            "value": 50.7578125,
            "unit": "median mem",
            "extra": "avg mem: 50.78170789218005, max mem: 56.734375, count: 53907"
          },
          {
            "name": "SELECT\n  pid,\n  pg_wal_lsn_diff(sent_lsn, replay_lsn) AS replication_lag,\n  application_name::text,\n  state::text\nFROM pg_stat_replication; - Publisher - replication_lag:MB",
            "value": 0,
            "unit": "median replication_lag:MB",
            "extra": "avg replication_lag:MB: 0.00002548184089576435, max replication_lag:MB: 0.2012176513671875, count: 53907"
          },
          {
            "name": "Top K - Subscriber - cpu",
            "value": 4.5757866,
            "unit": "median cpu",
            "extra": "avg cpu: 5.284605802994717, max cpu: 13.779904, count: 107814"
          },
          {
            "name": "Top K - Subscriber - mem",
            "value": 51.4296875,
            "unit": "median mem",
            "extra": "avg mem: 51.52270895941158, max mem: 57.80859375, count: 107814"
          },
          {
            "name": "Update 1..9 - Publisher - cpu",
            "value": 4.5714283,
            "unit": "median cpu",
            "extra": "avg cpu: 3.714036552679468, max cpu: 4.5933013, count: 53907"
          },
          {
            "name": "Update 1..9 - Publisher - mem",
            "value": 32.99609375,
            "unit": "median mem",
            "extra": "avg mem: 32.368512988804795, max mem: 33.56640625, count: 53907"
          },
          {
            "name": "Update 10,11 - Publisher - cpu",
            "value": 4.5540795,
            "unit": "median cpu",
            "extra": "avg cpu: 4.057855079718936, max cpu: 4.5801525, count: 53907"
          },
          {
            "name": "Update 10,11 - Publisher - mem",
            "value": 33.01953125,
            "unit": "median mem",
            "extra": "avg mem: 32.38525282836645, max mem: 33.2734375, count: 53907"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "james.sewell@gmail.com",
            "name": "James Sewell",
            "username": "jamessewell"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "8182eaf110c30cbefe008197caa40efa8b44f8e0",
          "message": "refactor: use existing FFHelper ctid cache instead of dedicated cache (#4905)\n\nFix a performance regression introduced in e0804b347 (#4765) which\nremoved ctid from SearchIndexScore and switched to lazy per-row\nresolution.\n\nPrior to #4765, ctid was resolved during result construction and carried\nin `SearchIndexScore` — no per-row fast-field lookups needed. #4765\nmoved ctid resolution to the consumption side (top_k.rs, normal.rs,\nscan.rs) using a single-entry `Option<(SegmentOrdinal, FFType)>` cache.\nWhen TopK results interleave across segments (sorted by score), every\nsegment transition re-opens the ctid column via `FastFieldReaders::u64\n-> DynamicColumnHandle::open -> BlockwiseLinearCodec::load`, which is\nvery expensive. Profiling showed 45% of total cycles spent in this\nre-open path.\n\nThe columnar scan path (`ColumnarExecState`) was unaffected — it already\nused `FFHelper`'s per-segment `OnceLock` ctid cache. This PR brings the\nremaining paths in line:\n\n- `scan.rs` uses its existing `Bm25ScanState.fast_fields` FFHelper\n- `normal.rs` and `top_k.rs` use a new `ctid_cache` FFHelper on\n`BaseScanState`\n\nEach segment's ctid column is opened at most once via `OnceLock`,\neliminating the thrashing. `FFHelper` has had this per-segment ctid\ncaching built in since cb78f0ca2 (Oct 2024).",
          "timestamp": "2026-05-13T12:18:31+12:00",
          "tree_id": "814e1da895eec41e0dfe3cbb5348bdb237811bf7",
          "url": "https://github.com/paradedb/paradedb/commit/8182eaf110c30cbefe008197caa40efa8b44f8e0"
        },
        "date": 1778636191296,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - cpu",
            "value": 4.5801525,
            "unit": "median cpu",
            "extra": "avg cpu: 5.102218232328803, max cpu: 9.266409, count: 53855"
          },
          {
            "name": "Custom Scan - Subscriber - mem",
            "value": 52.546875,
            "unit": "median mem",
            "extra": "avg mem: 52.62616937262093, max mem: 58.61328125, count: 53855"
          },
          {
            "name": "Delete values - Publisher - cpu",
            "value": 4.58891,
            "unit": "median cpu",
            "extra": "avg cpu: 4.348040642224783, max cpu: 4.619827, count: 53855"
          },
          {
            "name": "Delete values - Publisher - mem",
            "value": 32.0546875,
            "unit": "median mem",
            "extra": "avg mem: 31.351822167161824, max mem: 32.359375, count: 53855"
          },
          {
            "name": "Find by ctid - Subscriber - cpu",
            "value": 9.125476,
            "unit": "median cpu",
            "extra": "avg cpu: 8.17453954423334, max cpu: 18.35564, count: 53855"
          },
          {
            "name": "Find by ctid - Subscriber - mem",
            "value": 55.66796875,
            "unit": "median mem",
            "extra": "avg mem: 55.400529779036304, max mem: 61.65625, count: 53855"
          },
          {
            "name": "Index Only Scan - Subscriber - cpu",
            "value": 4.5801525,
            "unit": "median cpu",
            "extra": "avg cpu: 5.086557712325163, max cpu: 9.266409, count: 53855"
          },
          {
            "name": "Index Only Scan - Subscriber - mem",
            "value": 52.046875,
            "unit": "median mem",
            "extra": "avg mem: 52.13925405893139, max mem: 58.15625, count: 53855"
          },
          {
            "name": "Index Size Info - Subscriber - cpu",
            "value": 4.5757866,
            "unit": "median cpu",
            "extra": "avg cpu: 4.618570424449943, max cpu: 9.195402, count: 53855"
          },
          {
            "name": "Index Size Info - Subscriber - mem",
            "value": 33.28125,
            "unit": "median mem",
            "extra": "avg mem: 33.29630126613128, max mem: 38.5625, count: 53855"
          },
          {
            "name": "Index Size Info - Subscriber - pages",
            "value": 1106,
            "unit": "median pages",
            "extra": "avg pages: 1114.546207408783, max pages: 1859.0, count: 53855"
          },
          {
            "name": "Index Size Info - Subscriber - relation_size:MB",
            "value": 8.640625,
            "unit": "median relation_size:MB",
            "extra": "avg relation_size:MB: 8.707392680577476, max relation_size:MB: 14.5234375, count: 53855"
          },
          {
            "name": "Index Size Info - Subscriber - segment_count",
            "value": 7,
            "unit": "median segment_count",
            "extra": "avg segment_count: 6.861814130535698, max segment_count: 13.0, count: 53855"
          },
          {
            "name": "Insert value A - Publisher - cpu",
            "value": 4.5757866,
            "unit": "median cpu",
            "extra": "avg cpu: 4.418006128136538, max cpu: 4.58891, count: 53855"
          },
          {
            "name": "Insert value A - Publisher - mem",
            "value": 29.05078125,
            "unit": "median mem",
            "extra": "avg mem: 28.355154393162195, max mem: 29.39453125, count: 53855"
          },
          {
            "name": "Insert value B - Publisher - cpu",
            "value": 4.5540795,
            "unit": "median cpu",
            "extra": "avg cpu: 3.4199408060506267, max cpu: 4.5845275, count: 53855"
          },
          {
            "name": "Insert value B - Publisher - mem",
            "value": 29.0625,
            "unit": "median mem",
            "extra": "avg mem: 28.37752116504967, max mem: 29.4140625, count: 53855"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - cpu",
            "value": 9.169055,
            "unit": "median cpu",
            "extra": "avg cpu: 10.078248622556945, max cpu: 27.480915, count: 53855"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - mem",
            "value": 51.33984375,
            "unit": "median mem",
            "extra": "avg mem: 51.43587824076223, max mem: 57.421875, count: 53855"
          },
          {
            "name": "SELECT\n  pid,\n  pg_wal_lsn_diff(sent_lsn, replay_lsn) AS replication_lag,\n  application_name::text,\n  state::text\nFROM pg_stat_replication; - Publisher - replication_lag:MB",
            "value": 0,
            "unit": "median replication_lag:MB",
            "extra": "avg replication_lag:MB: 0.00001195755833625882, max replication_lag:MB: 0.04164886474609375, count: 53855"
          },
          {
            "name": "Top K - Subscriber - cpu",
            "value": 4.5801525,
            "unit": "median cpu",
            "extra": "avg cpu: 5.255052596399401, max cpu: 13.832853, count: 107710"
          },
          {
            "name": "Top K - Subscriber - mem",
            "value": 51.46875,
            "unit": "median mem",
            "extra": "avg mem: 51.52026767477486, max mem: 57.89453125, count: 107710"
          },
          {
            "name": "Update 1..9 - Publisher - cpu",
            "value": 4.5845275,
            "unit": "median cpu",
            "extra": "avg cpu: 4.370628860339577, max cpu: 4.6153846, count: 53855"
          },
          {
            "name": "Update 1..9 - Publisher - mem",
            "value": 32.75390625,
            "unit": "median mem",
            "extra": "avg mem: 32.00668120706062, max mem: 33.03515625, count: 53855"
          },
          {
            "name": "Update 10,11 - Publisher - cpu",
            "value": 4.562738,
            "unit": "median cpu",
            "extra": "avg cpu: 4.2549341032772, max cpu: 4.6021094, count: 53855"
          },
          {
            "name": "Update 10,11 - Publisher - mem",
            "value": 32.83984375,
            "unit": "median mem",
            "extra": "avg mem: 32.168684575480455, max mem: 32.9296875, count: 53855"
          }
        ]
      }
    ]
  }
}