window.BENCHMARK_DATA = {
  "lastUpdate": 1778860487605,
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
      },
      {
        "commit": {
          "author": {
            "email": "59696464+saadtajwar@users.noreply.github.com",
            "name": "Saad Tajwar",
            "username": "saadtajwar"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "bc20c3dd4b36a13dbcc74d03499f966a4dc93fe6",
          "message": "refactor: Deduplicate deferred materialization request partitioning between visibility and lookup (#4903)\n\n# Ticket(s) Closed\n- Closes https://github.com/paradedb/paradedb/issues/4568\n\n## What\nDeduplicated the segment-grouping/materialization loop shared between\n`materialize_deferred_ctid()` in `visibility_filter.rs` and\n`materialize_deferred_column()` in `tantivy_lookup_exec.rs`.\n\n## Why\nBoth functions implemented the same pattern of partitioning packed doc\naddresses by segment ordinal, batch-reading via `FFHelper`, and writing\nresults back in row order. This duplication made the two paths harder to\nkeep consistent and made future optimization work more tedious.\n\n## How\n\n### Shared helper in `fast_fields_helper.rs`\n- Added `for_each_segment`: partitions an iterator of `(row_index,\npacked_doc_address)` pairs into per-segment buckets and invokes a\ncaller-supplied closure once per non-empty segment, in segment-ordinal\norder. Backed by a `Vec<Vec<(usize, DocId)>>` indexed by segment ordinal\n(dense in practice; cheaper than a hash map for typical segment counts).\n- Added `FFHelper::num_segments()` so callers can size the bucket vector\nwithout reaching into private fields.\n\n### `materialize_deferred_ctid` (visibility_filter.rs)\n- Replaced the manual sort + partition + per-segment slice loop with a\nsingle `for_each_segment` call.\n- Kept `DeferredCtidMaterializationState` for buffer reuse across calls,\nbut removed its now-unused `requests` field; the per-segment\npartitioning lives inside `for_each_segment`.\n- Removed the TODO comment that flagged this duplication.\n\n### `materialize_deferred_column` (tantivy_lookup_exec.rs)\nFunction body shrank from ~160 lines to ~50 by extracting three\nsingle-responsibility helpers:\n- `resolve_doc_addresses_to_term_ords` — resolves State 0 (packed doc\naddresses) into per-segment `(row_index, Option<TermOrdinal>)` pairs via\n`for_each_segment`.\n- `extract_term_ords` — parses State 1 (pre-resolved `(segment_ord,\nterm_ord)` pairs from the dense union's `StructArray` child) into the\nsame per-segment shape.\n- `decode_term_ordinals` — takes the merged per-segment ordinals and\nperforms the bulk dictionary lookup once per segment, recording\npositions for the final `interleave`.\n\nState 0 and State 1 are now merged into a single `Vec<Vec<(row_index,\nOption<TermOrdinal>)>>` indexed by segment ordinal, then decoded in one\npass — previously each state was iterated and decoded separately,\nproducing two `segment_arrays` entries per segment touched by both. The\nfinal interleaved output is identical.\n\nReplaced the `(ff_index: usize, is_bytes: bool)` parameter pair with a\n`DeferredColumnKind { Text { ff_index }, Bytes { ff_index } }` enum to\nmake the `is_bytes && wrong-ff-type` mismatch unrepresentable.\n\n## Tests\nNo new tests; behavior is unchanged. Existing coverage exercises both\npaths end-to-end:\n- `pg_search/tests/pg_regress/sql/join_deferred_visibility.sql` —\n`materialize_deferred_ctid`.\n- `pg_search/tests/pg_regress/sql/segmented_topk.sql` plus joinscan\ntests — `materialize_deferred_column` (the segmented top-K rule is what\nproduces State 1 rows below `TantivyLookupExec`).\n\n---------\n\nCo-authored-by: Mithun Chicklore Yogendra <mithun.cy@gmail.com>",
          "timestamp": "2026-05-13T10:07:59-07:00",
          "tree_id": "cc9e05e63ed6052c202d00901357d2c5026923d0",
          "url": "https://github.com/paradedb/paradedb/commit/bc20c3dd4b36a13dbcc74d03499f966a4dc93fe6"
        },
        "date": 1778693838622,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - tps",
            "value": 131.74144755010374,
            "unit": "median tps",
            "extra": "avg tps: 131.6088467540558, max tps: 146.22080214675532, count: 54970"
          },
          {
            "name": "Columnar Scan - Primary - tps",
            "value": 518.2588996266887,
            "unit": "median tps",
            "extra": "avg tps: 516.3779970335505, max tps: 659.6265612641431, count: 54970"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3302.7109484838993,
            "unit": "median tps",
            "extra": "avg tps: 3286.8258706668285, max tps: 3314.3917503534685, count: 54970"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 420.44643767578657,
            "unit": "median tps",
            "extra": "avg tps: 421.42989183849613, max tps: 492.07613894097835, count: 54970"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 3012.2925597460094,
            "unit": "median tps",
            "extra": "avg tps: 3006.8863886721756, max tps: 3035.3311424557605, count: 109940"
          },
          {
            "name": "Normal Scan - Primary - tps",
            "value": 540.7824798578989,
            "unit": "median tps",
            "extra": "avg tps: 538.4748340785521, max tps: 633.121372829133, count: 54970"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 2022.3187497713584,
            "unit": "median tps",
            "extra": "avg tps: 2016.0734971407708, max tps: 2036.5169566309437, count: 54970"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 59.683428784769696,
            "unit": "median tps",
            "extra": "avg tps: 64.90349238862323, max tps: 355.4357500120848, count: 54970"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "stuhood@paradedb.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "713eaca82145388378043ddb2c750c41e9c632d9",
          "message": "chore: Revert #4905 (#5066)\n\nRevert #4905 due to performance regressions.",
          "timestamp": "2026-05-13T13:34:27-07:00",
          "tree_id": "2894402ba71bbb0b86d5990862fb2adba4fcbf49",
          "url": "https://github.com/paradedb/paradedb/commit/713eaca82145388378043ddb2c750c41e9c632d9"
        },
        "date": 1778706216983,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - tps",
            "value": 131.6478778053281,
            "unit": "median tps",
            "extra": "avg tps: 131.67326135449048, max tps: 144.19565294043647, count: 55044"
          },
          {
            "name": "Columnar Scan - Primary - tps",
            "value": 509.4781458561305,
            "unit": "median tps",
            "extra": "avg tps: 508.6859062321663, max tps: 615.351189799276, count: 55044"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3272.47714743983,
            "unit": "median tps",
            "extra": "avg tps: 3260.0933227683468, max tps: 3289.1169599928826, count: 55044"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 407.89506648167117,
            "unit": "median tps",
            "extra": "avg tps: 410.3637768538499, max tps: 488.2229397797509, count: 55044"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 3005.609240889761,
            "unit": "median tps",
            "extra": "avg tps: 3013.882126798748, max tps: 3067.9714561117066, count: 110088"
          },
          {
            "name": "Normal Scan - Primary - tps",
            "value": 505.5188149524863,
            "unit": "median tps",
            "extra": "avg tps: 504.33279296438104, max tps: 600.8556033577914, count: 55044"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1947.8153240668814,
            "unit": "median tps",
            "extra": "avg tps: 1941.6464605390158, max tps: 1963.1626097245417, count: 55044"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 72.27886425126417,
            "unit": "median tps",
            "extra": "avg tps: 76.4040700458295, max tps: 710.5361137085154, count: 55044"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "035982deb7c1e630e0d8a13e4ca1910b979b08f6",
          "message": "ci: add logical-replication-merge stressgres suite to cover FSM race (#4935) (#5072)\n\nRe-opened from #5068 (originally from a fork, where CI could not access\nworkflow secrets — see\nhttps://github.com/paradedb/paradedb/actions/runs/25830120431/job/75892835684?pr=5068).\nSame intent, from a branch in this repo, with the FSM-race suite landing\nas its own file rather than folded into `logical-replication.toml`.\n\n## Changes\n\n- `stressgres/suites/logical-replication-merge.toml` (new): standalone\nsuite that reliably reproduces the FSM race from #4935 (fixed in #5067).\nLogical-replication subscriber with aggressive autovacuum (`naptime=1s`,\n`threshold=50`), small `layer_sizes = '10kb, 100kb, 1mb, 100mb'`,\nmultiple concurrent BM25 readers, and sustained UPDATE/INSERT/DELETE\ntraffic on the publisher. The key difference from\n`logical-replication.toml` is the writer: `message = message || ' ' ||\ntxid_current()` grows each row's terms unbounded, generating ~10× more\nmerge/GC pressure and reliably opening the race window — folding the\nsame churn into `logical-replication.toml` (which strips-then-appends,\nkeeping row size constant) did not reproduce the bug.\n- `.github/workflows/benchmark-pg_search-stressgres.yml`:\n- Comment out single-server, bulk-updates, wide-table, and\nbackground-merge so CI focuses on the two replication suites while we\niterate. To be re-enabled before final merge.\n- Run `logical-replication-merge.toml` **before**\n`logical-replication.toml`.\n\n`stressgres/suites/logical-replication.toml` is unchanged from `main`.\n\n## Expected behavior\n\n- Without #5067: SIGSEGV or `SegmentMetaEntryHeader: UnexpectedEnd`\nwithin minutes.\n- With #5067: runs the full duration without errors.\n\n## Follow-ups\n\n- Re-enable the four commented-out suites before final merge.\n- Antithesis wiring for this suite belongs in `paradedb-enterprise` next\nto the existing `physical-logical-replication` driver, since the OSS\nAntithesis manifest only stands up a single paradedb cluster.\n\nRef: #4935\nRelated: #5067\nSupersedes: #5068",
          "timestamp": "2026-05-13T19:51:05-04:00",
          "tree_id": "c71af69d7df60d54d1631876f7e3c7af0782c3e3",
          "url": "https://github.com/paradedb/paradedb/commit/035982deb7c1e630e0d8a13e4ca1910b979b08f6"
        },
        "date": 1778718008332,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - tps",
            "value": 132.01523430579797,
            "unit": "median tps",
            "extra": "avg tps: 132.2830965725004, max tps: 175.11688143416907, count: 55110"
          },
          {
            "name": "Columnar Scan - Primary - tps",
            "value": 500.28817095013346,
            "unit": "median tps",
            "extra": "avg tps: 501.2388932252594, max tps: 611.470848701322, count: 55110"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3216.0846230267034,
            "unit": "median tps",
            "extra": "avg tps: 3197.138985936344, max tps: 3256.929493369996, count: 55110"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 420.70261681974955,
            "unit": "median tps",
            "extra": "avg tps: 423.0406636420492, max tps: 499.1124781912803, count: 55110"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 2983.0198105421478,
            "unit": "median tps",
            "extra": "avg tps: 3026.0400242929077, max tps: 3177.9928241581824, count: 110220"
          },
          {
            "name": "Normal Scan - Primary - tps",
            "value": 504.9957952928463,
            "unit": "median tps",
            "extra": "avg tps: 505.2691617823303, max tps: 684.1135183642314, count: 55110"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 2085.41163491251,
            "unit": "median tps",
            "extra": "avg tps: 2069.7263139109373, max tps: 2093.1742687435617, count: 55110"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 47.257464722671784,
            "unit": "median tps",
            "extra": "avg tps: 74.0380416326548, max tps: 779.7258795697784, count: 55110"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "36623265+daniel3303@users.noreply.github.com",
            "name": "Daniel Oliveira",
            "username": "daniel3303"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "b9c06c5f75ca94cf815eb57c71c46180e84b8038",
          "message": "fix(mlt): quote key_field identifier in internal SPI lookup (#5078)\n\n## Summary\n\n- `pdb.more_like_this(key_value)` raises `ERROR: column \"id\" does not\nexist` whenever the index's `key_field` column is a mixed-case\nPostgreSQL identifier (e.g. `\"Id\"`, `\"DocumentId\"`). Direct `@@@`-on-LHS\nsearches (`\"Content\" @@@ 'foo'`) are unaffected because they don't go\nthrough the internal SPI lookup. Repro in #5065.\n- Root cause: `pg_search/src/query/more_like_this.rs:152-157` builds the\nSPI `SELECT * FROM <ns>.<rel> WHERE <key_field> = $1` with `<ns>` and\n`<rel>` already routed through `pgrx::spi::quote_identifier`, but\ninterpolates `<key_field>` verbatim via `Display`. PostgreSQL folds the\nunquoted reference to lowercase, so a column named `\"Id\"` is looked up\nas `id` and the SPI call fails before MLT ever runs.\n- Fix: send the key field through\n`pgrx::spi::quote_identifier(key_field_name.root())`, matching how the\nnamespace and relation names are already quoted on the lines immediately\nabove. `.root()` strips the JSON sub-path (`key_field` is always a\ntop-level column).\n\n## Scope\n\nThe linked issue also lists JSON `term` filters (`@@@\n'{\"term\":{\"field\":\"Category\",…}}'::jsonb`) as failing on mixed-case\ncolumns. That path does **not** go through SPI — `term()` in\n`pg_search/src/query/pdb_query.rs:792` resolves the field via\n`schema.search_field(field.root())`, a pure Tantivy schema lookup — so\nit isn't fixed here and I couldn't find a corresponding\nunquoted-identifier hazard. If it reproduces on `0.23.x` it's a separate\nbug; tracking it on its own issue is cleaner than bundling a speculative\nfix.\n\n## Test plan\n\n- [x] `cargo test -p tests --test mlt --\nmlt_mixed_case_key_field_issue5065` — new regression test: `\"Id\"` /\n`\"Content\"` table, `key_field='Id'`, asserts `pdb.more_like_this(1)`\nreturns rows. Fails on `main` with `column \"id\" does not exist`, passes\nwith this change.\n- [x] `cargo test -p tests --test mlt` — existing\n`mlt_enables_scoring_issue1747`, `mlt_datetime_key`,\n`mlt_scoring_nested` still pass.\n- [x] `cargo pgrx regress -p pg_search --auto -- pg18 more_like_this` —\ngolden output unchanged (`quote_identifier(\"id\")` is a no-op for\nalready-lowercase identifiers).\n- [x] Manual repro from #5065 (`CREATE TABLE items (\"Id\" int primary\nkey, \"Content\" text); … pdb.more_like_this(1)`) returns rows instead of\nerroring.\n\nCloses #5065.",
          "timestamp": "2026-05-14T14:45:18-04:00",
          "tree_id": "812b4a66ffce7bc074cb919986f1de3b6474813f",
          "url": "https://github.com/paradedb/paradedb/commit/b9c06c5f75ca94cf815eb57c71c46180e84b8038"
        },
        "date": 1778786076470,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - tps",
            "value": 145.05905394651725,
            "unit": "median tps",
            "extra": "avg tps: 144.9204173863158, max tps: 148.59875750558018, count: 55163"
          },
          {
            "name": "Columnar Scan - Primary - tps",
            "value": 497.6520149462677,
            "unit": "median tps",
            "extra": "avg tps: 499.8565048171039, max tps: 678.2158335506996, count: 55163"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3181.9259648909224,
            "unit": "median tps",
            "extra": "avg tps: 3172.9623056437003, max tps: 3273.011622175475, count: 55163"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 428.03567316744557,
            "unit": "median tps",
            "extra": "avg tps: 429.0388865674299, max tps: 513.5887450108063, count: 55163"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 3008.178184408523,
            "unit": "median tps",
            "extra": "avg tps: 3008.8204962248124, max tps: 3099.048962132807, count: 110326"
          },
          {
            "name": "Normal Scan - Primary - tps",
            "value": 496.53035473753545,
            "unit": "median tps",
            "extra": "avg tps: 497.97990743236755, max tps: 624.7936343378349, count: 55163"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 2083.1029724115783,
            "unit": "median tps",
            "extra": "avg tps: 2071.199786777068, max tps: 2087.6508554331385, count: 55163"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 100.37184113968937,
            "unit": "median tps",
            "extra": "avg tps: 124.44286416859633, max tps: 830.6986341653057, count: 55163"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "balavignesh449@gmail.com",
            "name": "S Bala Vignesh",
            "username": "SBALAVIGNESH123"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "d0039460372b22539b33262920b0194c694b7205",
          "message": "fix: use fixed-length updates in stressgres to avoid TOAST and expose FSM race (#5080)\n\nThe old UPDATE pattern in the logical-replication stressgres suite\nappended txid_current() to the message column every iteration, growing\nit past the TOAST threshold (~2KB). This caused the suite to hit the\nunrelated TOAST visibility race (#5076) before the FSM segment metadata\nrace (#4935) could surface.\n\nChanged to fixed-length updates that keep the first search term and\nappend a small txid-derived number, staying well under the TOAST\nthreshold. This way the suite can run long enough to exercise the FSM\npath.\n\nRelated: #5067 (FSM race fix), #5076 (TOAST bug)",
          "timestamp": "2026-05-14T14:57:31-04:00",
          "tree_id": "96511f40645bb3416046b0914f9758c60c159a20",
          "url": "https://github.com/paradedb/paradedb/commit/d0039460372b22539b33262920b0194c694b7205"
        },
        "date": 1778786814543,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - tps",
            "value": 129.01337921871118,
            "unit": "median tps",
            "extra": "avg tps: 129.61991709130268, max tps: 168.35689128865548, count: 55303"
          },
          {
            "name": "Columnar Scan - Primary - tps",
            "value": 477.34370865531577,
            "unit": "median tps",
            "extra": "avg tps: 480.33551126884964, max tps: 703.8999025056401, count: 55303"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3214.150426611533,
            "unit": "median tps",
            "extra": "avg tps: 3209.2826226814896, max tps: 3343.8759385548146, count: 55303"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 398.6828122040063,
            "unit": "median tps",
            "extra": "avg tps: 403.30708338187804, max tps: 543.8435818843527, count: 55303"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 2936.2325598663915,
            "unit": "median tps",
            "extra": "avg tps: 2954.0328962152644, max tps: 3010.0783602752267, count: 110606"
          },
          {
            "name": "Normal Scan - Primary - tps",
            "value": 480.32472664134843,
            "unit": "median tps",
            "extra": "avg tps: 482.9586806003716, max tps: 668.3171158833676, count: 55303"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 2077.1875581959375,
            "unit": "median tps",
            "extra": "avg tps: 2067.4246929176866, max tps: 2083.312264477782, count: 55303"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 98.67930754576892,
            "unit": "median tps",
            "extra": "avg tps: 96.687512913416, max tps: 274.00908723736916, count: 55303"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "balavignesh449@gmail.com",
            "name": "S Bala Vignesh",
            "username": "SBALAVIGNESH123"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "b597b183d25a561cf8c81d81decc7f07b7866e55",
          "message": "fix: hold header shared lock during LinkedItemList iteration to prevent FSM race (#4935) (#5067)\n\n## Problem\n\nLinkedItemList read-only iteration methods (list(), is_empty(),\nor_each(), lookup()/lookup_ex()) were releasing the header shared lock\nbefore iterating through the linked list blocks. This allowed\nAtomicGuard::commit() to:\n\n1. Swap the header pointer to a new list\n2. Immediately recycle old blocks to the FSM with\nReadNextFullTransactionId()\n3. The next transaction pops a recycled block via RBM_ZERO_AND_LOCK,\nzeroing it\n4. A concurrent reader still traversing the old list hits the\nzeroed/repurposed block\n5. **SIGSEGV** or SegmentMetaEntryHeader: UnexpectedEnd deserialization\nerror\n\nThis manifests as periodic crashes and durable index corruption under\nsustained write traffic, particularly on logical replication subscribers\nat high apply rates (~395 commits/sec). Correlates strongly with\nautovacuum events that trigger garbage_collect_index().\n\n## Root Cause\n\nThe race window exists because or_each(), list(), is_empty(), and\nlookup_ex() call get_start_blockno() which acquires a shared lock on the\nheader, reads start_blockno, then immediately releases the header lock\nwhen exchanging to the first data block. After that point, \u0007tomically()\ncan take an exclusive header lock and proceed with the swap+recycle\nwhile the reader is deep in the old list.\n\n**This was already a known pattern in the codebase** — emove_item() and\nupdate_item() in the same file both hold the header shared lock for\ntheir entire operation with this comment:\n\n\\\\\\\rust\n// Acquire and hold a shared lock on the header for the entire\noperation, preventing the\n// list from being swapped out from under us by atomically between our\nread locks and\n// our write locks.\nlet header_lock = self.bman.get_buffer(self.header_blockno);\n\\\\\\\n\nThe read-only methods simply weren't given the same treatment.\n\n## Fix\n\nHold a shared lock on the header for the entire duration of iteration in\nall 4 methods, matching the existing emove_item()/update_item() pattern:\n\n- **list()** — hold header_lock from start to end of iteration\n- **is_empty()** — same\n- **\for_each()** — same\n- **lookup_ex()** — conditionally: only when \blockno is None (top-level\ncall). When \blockno is Some, the caller ( emove_item/update_item)\nalready holds the header lock\n\nRead start_blockno directly from the already-held header_lock instead of\ncalling get_start_blockno(), avoiding a double shared-lock acquisition\non the same block (which would trigger a panic under the \block_tracker\ndebug feature).\n\n## Why This Is Safe\n\n- **No deadlock**: Header block is always locked first, content blocks\nin ascending order — consistent lock ordering\n- **No reader-reader blocking**: Multiple readers hold shared locks\nconcurrently (shared locks are compatible)\n- **Writer waits for readers**: \u0007tomically() takes an exclusive header\nlock, which blocks until all shared locks are released — correct\nserialization\n- **Minimal performance impact**: The header lock was already acquired;\nwe just hold it slightly longer\n\n## Verification\n\n- \rustfmt --check passes\n- Pattern matches the proven emove_item()/update_item() implementation\nin the same file\n- Full cargo check requires pgrx setup (PostgreSQL extension); the\nchange is limited to lock lifetime management with no new APIs\n\nCloses #4935",
          "timestamp": "2026-05-14T15:00:05-04:00",
          "tree_id": "46e474245958de09c21d0198195343d8a87fb72d",
          "url": "https://github.com/paradedb/paradedb/commit/b597b183d25a561cf8c81d81decc7f07b7866e55"
        },
        "date": 1778786965019,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - tps",
            "value": 130.90220919971944,
            "unit": "median tps",
            "extra": "avg tps: 131.03476279970536, max tps: 146.87462731874697, count: 55260"
          },
          {
            "name": "Columnar Scan - Primary - tps",
            "value": 502.5063511803464,
            "unit": "median tps",
            "extra": "avg tps: 501.90526799113496, max tps: 657.846452801024, count: 55260"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3326.9734236160893,
            "unit": "median tps",
            "extra": "avg tps: 3322.610714553101, max tps: 3382.675874034259, count: 55260"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 445.97876167564465,
            "unit": "median tps",
            "extra": "avg tps: 443.15615333091046, max tps: 549.7277793847135, count: 55260"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 3113.717351978235,
            "unit": "median tps",
            "extra": "avg tps: 3108.189808279307, max tps: 3152.2953979617473, count: 110520"
          },
          {
            "name": "Normal Scan - Primary - tps",
            "value": 534.3296234924524,
            "unit": "median tps",
            "extra": "avg tps: 532.3485482073128, max tps: 635.7902584452079, count: 55260"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 2062.960510559414,
            "unit": "median tps",
            "extra": "avg tps: 2049.497494344235, max tps: 2068.1872720864408, count: 55260"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 78.5884302249875,
            "unit": "median tps",
            "extra": "avg tps: 95.16167508658148, max tps: 819.5619605233395, count: 55260"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "652b0952a7f3be58ea4b9fa80a863426cf03185c",
          "message": "chore(stressgres): remove dormant suites not wired into CI (#5085)\n\n## Summary\n\n- Removes 8 Stressgres suites that have not been touched since\nStressgres was added (#3821), are not referenced by any CI workflow or\ndriver script, and use the legacy `[setup_primary]` schema rather than\nthe current `[[server]]` schema.\n- No CI workflow changes; the 6 suites currently exercised by\n`benchmark-pg_search-stressgres.yml` (`single-server`, `bulk-updates`,\n`wide-table`, `background-merge`, `logical-replication`,\n`logical-replication-merge`) and `vanilla-postgres.toml` (used by the\nantithesis singleton driver) are kept.\n\n### Removed\n- `large-inserts.toml`\n- `lr.toml` — predecessor of `logical-replication.toml`\n- `lr-graphable.toml`\n- `lr-large-inserts.toml`\n- `lr-no-pg_search.toml`\n- `many-updates.toml` — conceptually useful (MVCC correctness with\n`assert(count, expected)` under concurrency), but on legacy schema; if\nwe want this coverage in CI it should be a deliberate port, not a\nrevival of dead config\n- `read-write.toml` — contained placeholder `<password>` literal, not\nCI-runnable\n- `topk-crash.toml`\n\n### Why now\nCompanion to #5080 (which adjusts `logical-replication.toml` to expose\nthe FSM race instead of being masked by the TOAST bug already covered by\n`logical-replication-merge.toml`). Each CI suite should own a distinct\nrepro; dormant files muddy that mapping.\n\n## Test plan\n- [x] `benchmark-pg_search-stressgres` workflow still kicks off for all\n6 in-CI suites\n- [x] Antithesis `singleton_driver_vanilla-postgres.sh` still resolves\nits suite path",
          "timestamp": "2026-05-14T15:45:37-04:00",
          "tree_id": "090e72f7bbb9817642fb0c7495c2b12e0eed2fe7",
          "url": "https://github.com/paradedb/paradedb/commit/652b0952a7f3be58ea4b9fa80a863426cf03185c"
        },
        "date": 1778789688769,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - tps",
            "value": 137.39281684252595,
            "unit": "median tps",
            "extra": "avg tps: 136.56424819890483, max tps: 150.073196376318, count: 55172"
          },
          {
            "name": "Columnar Scan - Primary - tps",
            "value": 509.21697996736725,
            "unit": "median tps",
            "extra": "avg tps: 505.5163331458471, max tps: 634.2768942124453, count: 55172"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3219.48434629824,
            "unit": "median tps",
            "extra": "avg tps: 3210.943109568813, max tps: 3243.6775312895675, count: 55172"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 427.8223792293773,
            "unit": "median tps",
            "extra": "avg tps: 424.97874702169304, max tps: 481.4808965713553, count: 55172"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 2980.789965838484,
            "unit": "median tps",
            "extra": "avg tps: 2987.024858281625, max tps: 3043.3900699600126, count: 110344"
          },
          {
            "name": "Normal Scan - Primary - tps",
            "value": 522.9684181884832,
            "unit": "median tps",
            "extra": "avg tps: 518.923244424194, max tps: 635.7488722298729, count: 55172"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 2061.2555904638602,
            "unit": "median tps",
            "extra": "avg tps: 2044.6522453111265, max tps: 2072.105999251957, count: 55172"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 113.55780564806703,
            "unit": "median tps",
            "extra": "avg tps: 138.4911083612354, max tps: 284.8266843866841, count: 55172"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ming.ying.nyc@gmail.com",
            "name": "Ming",
            "username": "rebasedming"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "21bdc9a753582039cc25a38cb07111026c0fd378",
          "message": "feat: Crash recovery via WAL (#4901)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nPorts WAL integration over to community, which gives `pg_search` crash\nrecovery.\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2026-05-14T14:59:53-07:00",
          "tree_id": "7580b5f2bfcc1a95ed1b62c4628f2ec6129b7c00",
          "url": "https://github.com/paradedb/paradedb/commit/21bdc9a753582039cc25a38cb07111026c0fd378"
        },
        "date": 1778797859963,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - tps",
            "value": 135.40341722419655,
            "unit": "median tps",
            "extra": "avg tps: 135.07989990810947, max tps: 151.26012895007156, count: 55218"
          },
          {
            "name": "Columnar Scan - Primary - tps",
            "value": 552.368757260527,
            "unit": "median tps",
            "extra": "avg tps: 546.7403981773986, max tps: 635.681176252777, count: 55218"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3293.834177481912,
            "unit": "median tps",
            "extra": "avg tps: 3268.03547729072, max tps: 3308.0813425126703, count: 55218"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 451.4200292157023,
            "unit": "median tps",
            "extra": "avg tps: 447.43819404142954, max tps: 518.785302259016, count: 55218"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 2843.249323072177,
            "unit": "median tps",
            "extra": "avg tps: 2829.738084999124, max tps: 2871.1886455880226, count: 110436"
          },
          {
            "name": "Normal Scan - Primary - tps",
            "value": 533.4653436747701,
            "unit": "median tps",
            "extra": "avg tps: 528.7222762105888, max tps: 625.1579938623928, count: 55218"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1975.6191136870277,
            "unit": "median tps",
            "extra": "avg tps: 1960.6426163168542, max tps: 1983.6881491583808, count: 55218"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 83.87842811086543,
            "unit": "median tps",
            "extra": "avg tps: 102.91435931469233, max tps: 750.4566528732734, count: 55218"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "d99908a1f58307567dd34698345b3fc836be6135",
          "message": "feat(antithesis): add singleton drivers for logical-replication suites (#5087)\n\n## Summary\n\nAdds OSS Antithesis singleton drivers for the two CI\n`logical-replication` suites that previously had no singleton\n(`single-server`, `bulk-updates`, `wide-table`, `background-merge`, and\n`vanilla-postgres` already had one).\n\nMirrors the enterprise pattern used for `physical-logical-replication`:\n- A **vanilla Postgres 18** publisher pod (with `wal_level=logical`)\nthat lives outside the CNPG cluster, reflecting real-world\nlogical-replication topologies where the upstream primary is not under\nour control.\n- Subscriber points at `paradedb-rw` (the CNPG primary, which has\n`pg_search`).\n\n## Changes\n- `docker/manifests/antithesis-paradedb.yaml` — add\n`logical-replication-publisher` Service + Deployment (vanilla Postgres\n18 with `wal_level=logical`), reusing the existing `paradedb-superuser`\nsecret.\n- `stressgres/suites/logical-replication.toml`,\n`stressgres/suites/logical-replication-merge.toml` — drop `CREATE\nEXTENSION pg_search` from the **Publisher** setup. Only the Subscriber\nuses `pg_search`; the line was cosmetic and incompatible with a vanilla\nPostgres publisher (the line in the Subscriber setup is unchanged).\n-\n`stressgres/suites/antithesis/singleton_driver_logical-replication.sh`,\n`singleton_driver_logical-replication-merge.sh` — new drivers that\nperform per-block `sed -z` rewrites of the `[server.style.Automatic]`\nblocks into `[server.style.With]` connection strings (Publisher →\n`logical-replication-publisher:5432`, Subscriber → `paradedb-rw:5432`).\n- `.github/workflows/antithesis-trigger-test-run.yml` — add\n`logical-replication-publisher` to\n`container_faults_stop_exclusion_patterns` and\n`container_faults_kill_exclusion_patterns`, matching enterprise. Network\nfaults to/from the publisher are intentionally still injected.\n\n## Why\nWithout these, the FSM race repro in `logical-replication-merge.toml`\n(issue #4935, fixed by #5067) and the broader logical-replication\ncoverage in `logical-replication.toml` were running in\n`benchmark-pg_search-stressgres` but had no Antithesis fault-injection\nequivalent — that's the half of the matrix where the bugs originally\nsurfaced.\n\n## Test plan\n- [x] Antithesis trigger workflow picks up both new singleton drivers\nfrom `/opt/antithesis/test/v1/quickstart/`\n- [x] Publisher pod (`logical-replication-publisher`) starts with\n`wal_level=logical` and is reachable from the stressgres-runner pod\n- [x] Subscriber's `CREATE SUBSCRIPTION ... CONNECTION\n'@Publisher_CONNSTR@'` resolves to the publisher pod after the `sed`\nrewrite\n- [x] `logical-replication-merge.toml` still reproduces the FSM race\nwhen run against a build without #5067\n- [x] `benchmark-pg_search-stressgres` (local Stressgres, not\nAntithesis) still runs both suites unchanged",
          "timestamp": "2026-05-14T18:54:16-04:00",
          "tree_id": "8c7a6dab334db43248e88e7a4ebc75fd5840e446",
          "url": "https://github.com/paradedb/paradedb/commit/d99908a1f58307567dd34698345b3fc836be6135"
        },
        "date": 1778801028442,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - tps",
            "value": 134.275941628273,
            "unit": "median tps",
            "extra": "avg tps: 134.49324859770695, max tps: 146.12929831530852, count: 55026"
          },
          {
            "name": "Columnar Scan - Primary - tps",
            "value": 498.6265785691718,
            "unit": "median tps",
            "extra": "avg tps: 499.32742238609745, max tps: 676.5836881878683, count: 55026"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3250.3551178052276,
            "unit": "median tps",
            "extra": "avg tps: 3229.026778893718, max tps: 3277.3284933065083, count: 55026"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 426.60025722048374,
            "unit": "median tps",
            "extra": "avg tps: 428.73619526465063, max tps: 510.9449518127816, count: 55026"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 2763.469881708891,
            "unit": "median tps",
            "extra": "avg tps: 2768.9843734044734, max tps: 2848.0724126772448, count: 110052"
          },
          {
            "name": "Normal Scan - Primary - tps",
            "value": 504.72104756832783,
            "unit": "median tps",
            "extra": "avg tps: 505.23426942530284, max tps: 648.2059765239242, count: 55026"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1930.74408032169,
            "unit": "median tps",
            "extra": "avg tps: 1912.6631478426136, max tps: 1937.8694489533593, count: 55026"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 54.34831633005608,
            "unit": "median tps",
            "extra": "avg tps: 53.71168429083561, max tps: 656.3820353487981, count: 55026"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "balavignesh449@gmail.com",
            "name": "S Bala Vignesh",
            "username": "SBALAVIGNESH123"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "d453f45b157fe9286838cba35678d294100dbce7",
          "message": "fix: eager detoast in index_memory_segment to prevent TOAST race with VACUUM (#5076) (#5086)\n\n## Problem\n\n`index_memory_segment` calls `ExecFetchSlotHeapTuple(slot, true, ...)`\nwhich **materializes** the tuple into palloc memory, **releasing the\nbuffer pin** held by the `BufferHeapTupleTableSlot`. Without the pin,\nVACUUM's `LockBufferForCleanup` proceeds immediately, removes the heap\ntuple, and deletes its TOAST chunks — while `row_to_search_document`\nhasn't detoasted yet.\n\nThis causes the crash:\n`\nmissing chunk number 0 for toast value XXXXX in pg_toast_17265\nLOCATION: heaptoast.c:782\n`\n\nReproduced consistently by the `logical-replication-merge.toml`\nstressgres suite.\n\n## Root Cause\n\nThe race window:\n\n1. `table_index_fetch_tuple` → slot holds buffer **pin**\n2. `HeapTupleSatisfiesVacuum` → tuple is alive, share lock dropped but\n**pin still held**\n3. `ExecFetchSlotHeapTuple(slot, true)` → materializes tuple →\n**releases pin** ⚡\n4. VACUUM calls `LockBufferForCleanup` → pin count is 0 → proceeds\n5. VACUUM removes heap tuple → deletes TOAST chunks\n6. `row_to_search_document` → `String::from_datum` → `pg_detoast_datum`\n→ reads deleted TOAST → **CRASH**\n\n## Fix\n\nTwo changes:\n\n1. **Don't materialize** — pass `false` to `ExecFetchSlotHeapTuple` to\nkeep the buffer pin held by the slot. While the pin is held,\n`LockBufferForCleanup` blocks, so VACUUM can't remove the heap tuple or\nits TOAST chunks.\n\n2. **Eager detoast** — immediately after `heap_deform_tuple`, loop\nthrough all varlena (`attlen == -1`) datums and call `pg_detoast_datum`\nwhile the pin protects the TOAST data. `pg_detoast_datum` is a no-op for\nnon-TOASTed / already-inline data.\n\n## Why This Is Safe\n\n- **No deadlock**: buffer pin blocks only `LockBufferForCleanup` (VACUUM\ncleanup), not normal reads/writes\n- **Heap tuple immutability**: tuple data in the buffer page is\nimmutable once written — updates create new tuples\n- **Expression eval safe**: `expression_state.evaluate(slot)` still\nworks because the slot has a valid buffer-backed tuple with pin held\n- **Memory**: only allocates palloc copies for actually-TOASTed datums;\nfreed at memory context reset\n- **HOT chains**: handled by `table_index_fetch_tuple` before we see the\ntuple\n\n## Verification\n\n- `rustfmt --check` passes\n- 1 file changed, 28 insertions, 6 deletions\n- Should be validated against `logical-replication-merge.toml`\nstressgres suite\n\nCloses #5076\n\n---------\n\nCo-authored-by: Philippe Noël <philippemnoel@gmail.com>",
          "timestamp": "2026-05-15T11:24:27-04:00",
          "tree_id": "83e864df0221b99ca4fe39e5255c3e8061c8c9c6",
          "url": "https://github.com/paradedb/paradedb/commit/d453f45b157fe9286838cba35678d294100dbce7"
        },
        "date": 1778860424081,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - tps",
            "value": 133.79543062909136,
            "unit": "median tps",
            "extra": "avg tps: 133.40525902427854, max tps: 149.4504566598813, count: 55134"
          },
          {
            "name": "Columnar Scan - Primary - tps",
            "value": 485.20894356778786,
            "unit": "median tps",
            "extra": "avg tps: 487.615619053897, max tps: 627.9929962479824, count: 55134"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 3286.0451974512307,
            "unit": "median tps",
            "extra": "avg tps: 3272.6167229688376, max tps: 3321.4147932360834, count: 55134"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 417.30744560332226,
            "unit": "median tps",
            "extra": "avg tps: 419.4334094916138, max tps: 534.4579468712457, count: 55134"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 2789.4764147442593,
            "unit": "median tps",
            "extra": "avg tps: 2780.9467134051615, max tps: 2861.1760435586457, count: 110268"
          },
          {
            "name": "Normal Scan - Primary - tps",
            "value": 488.3493153574286,
            "unit": "median tps",
            "extra": "avg tps: 490.1089570947139, max tps: 629.738156448894, count: 55134"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 1961.0016833818772,
            "unit": "median tps",
            "extra": "avg tps: 1949.2050362623775, max tps: 1969.2450671034817, count: 55134"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 92.4141578859538,
            "unit": "median tps",
            "extra": "avg tps: 118.24703096194769, max tps: 854.1985567461185, count: 55134"
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
      },
      {
        "commit": {
          "author": {
            "email": "59696464+saadtajwar@users.noreply.github.com",
            "name": "Saad Tajwar",
            "username": "saadtajwar"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "bc20c3dd4b36a13dbcc74d03499f966a4dc93fe6",
          "message": "refactor: Deduplicate deferred materialization request partitioning between visibility and lookup (#4903)\n\n# Ticket(s) Closed\n- Closes https://github.com/paradedb/paradedb/issues/4568\n\n## What\nDeduplicated the segment-grouping/materialization loop shared between\n`materialize_deferred_ctid()` in `visibility_filter.rs` and\n`materialize_deferred_column()` in `tantivy_lookup_exec.rs`.\n\n## Why\nBoth functions implemented the same pattern of partitioning packed doc\naddresses by segment ordinal, batch-reading via `FFHelper`, and writing\nresults back in row order. This duplication made the two paths harder to\nkeep consistent and made future optimization work more tedious.\n\n## How\n\n### Shared helper in `fast_fields_helper.rs`\n- Added `for_each_segment`: partitions an iterator of `(row_index,\npacked_doc_address)` pairs into per-segment buckets and invokes a\ncaller-supplied closure once per non-empty segment, in segment-ordinal\norder. Backed by a `Vec<Vec<(usize, DocId)>>` indexed by segment ordinal\n(dense in practice; cheaper than a hash map for typical segment counts).\n- Added `FFHelper::num_segments()` so callers can size the bucket vector\nwithout reaching into private fields.\n\n### `materialize_deferred_ctid` (visibility_filter.rs)\n- Replaced the manual sort + partition + per-segment slice loop with a\nsingle `for_each_segment` call.\n- Kept `DeferredCtidMaterializationState` for buffer reuse across calls,\nbut removed its now-unused `requests` field; the per-segment\npartitioning lives inside `for_each_segment`.\n- Removed the TODO comment that flagged this duplication.\n\n### `materialize_deferred_column` (tantivy_lookup_exec.rs)\nFunction body shrank from ~160 lines to ~50 by extracting three\nsingle-responsibility helpers:\n- `resolve_doc_addresses_to_term_ords` — resolves State 0 (packed doc\naddresses) into per-segment `(row_index, Option<TermOrdinal>)` pairs via\n`for_each_segment`.\n- `extract_term_ords` — parses State 1 (pre-resolved `(segment_ord,\nterm_ord)` pairs from the dense union's `StructArray` child) into the\nsame per-segment shape.\n- `decode_term_ordinals` — takes the merged per-segment ordinals and\nperforms the bulk dictionary lookup once per segment, recording\npositions for the final `interleave`.\n\nState 0 and State 1 are now merged into a single `Vec<Vec<(row_index,\nOption<TermOrdinal>)>>` indexed by segment ordinal, then decoded in one\npass — previously each state was iterated and decoded separately,\nproducing two `segment_arrays` entries per segment touched by both. The\nfinal interleaved output is identical.\n\nReplaced the `(ff_index: usize, is_bytes: bool)` parameter pair with a\n`DeferredColumnKind { Text { ff_index }, Bytes { ff_index } }` enum to\nmake the `is_bytes && wrong-ff-type` mismatch unrepresentable.\n\n## Tests\nNo new tests; behavior is unchanged. Existing coverage exercises both\npaths end-to-end:\n- `pg_search/tests/pg_regress/sql/join_deferred_visibility.sql` —\n`materialize_deferred_ctid`.\n- `pg_search/tests/pg_regress/sql/segmented_topk.sql` plus joinscan\ntests — `materialize_deferred_column` (the segmented top-K rule is what\nproduces State 1 rows below `TantivyLookupExec`).\n\n---------\n\nCo-authored-by: Mithun Chicklore Yogendra <mithun.cy@gmail.com>",
          "timestamp": "2026-05-13T10:07:59-07:00",
          "tree_id": "cc9e05e63ed6052c202d00901357d2c5026923d0",
          "url": "https://github.com/paradedb/paradedb/commit/bc20c3dd4b36a13dbcc74d03499f966a4dc93fe6"
        },
        "date": 1778693871950,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - cpu",
            "value": 9.248554,
            "unit": "median cpu",
            "extra": "avg cpu: 8.363714104900016, max cpu: 23.692005, count: 54970"
          },
          {
            "name": "Aggregate Custom Scan - Primary - mem",
            "value": 66.67578125,
            "unit": "median mem",
            "extra": "avg mem: 66.58209399729398, max mem: 78.15234375, count: 54970"
          },
          {
            "name": "Columnar Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 5.4843678059851335, max cpu: 18.58664, count: 54970"
          },
          {
            "name": "Columnar Scan - Primary - mem",
            "value": 65.62890625,
            "unit": "median mem",
            "extra": "avg mem: 65.49964675334273, max mem: 76.96484375, count: 54970"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.659255682205345, max cpu: 9.239654, count: 54970"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 36.0546875,
            "unit": "median mem",
            "extra": "avg mem: 35.68127074995452, max mem: 37.890625, count: 54970"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.6231285207437995, max cpu: 9.248554, count: 54970"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 63.703125,
            "unit": "median mem",
            "extra": "avg mem: 63.22185645295161, max mem: 75.16796875, count: 54970"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.6057064126807195, max cpu: 9.402546, count: 109940"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 55.65625,
            "unit": "median mem",
            "extra": "avg mem: 54.77262668842096, max mem: 73.625, count: 109940"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 1783,
            "unit": "median block_count",
            "extra": "avg block_count: 1788.6193560123704, max block_count: 3186.0, count: 54970"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 11,
            "unit": "median segment_count",
            "extra": "avg segment_count: 11.619283245406585, max segment_count: 29.0, count: 54970"
          },
          {
            "name": "Normal Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 5.4641777286646604, max cpu: 18.934912, count: 54970"
          },
          {
            "name": "Normal Scan - Primary - mem",
            "value": 65.17578125,
            "unit": "median mem",
            "extra": "avg mem: 65.11406200256958, max mem: 76.6171875, count: 54970"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.581335599448414, max cpu: 4.7666335, count: 54970"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 54.18359375,
            "unit": "median mem",
            "extra": "avg mem: 53.82623426698654, max mem: 65.3203125, count: 54970"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6153846,
            "unit": "median cpu",
            "extra": "avg cpu: 3.721402135017601, max cpu: 4.6332045, count: 54970"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 56.91015625,
            "unit": "median mem",
            "extra": "avg mem: 56.740882882140255, max mem: 70.08984375, count: 54970"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "stuhood@paradedb.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "713eaca82145388378043ddb2c750c41e9c632d9",
          "message": "chore: Revert #4905 (#5066)\n\nRevert #4905 due to performance regressions.",
          "timestamp": "2026-05-13T13:34:27-07:00",
          "tree_id": "2894402ba71bbb0b86d5990862fb2adba4fcbf49",
          "url": "https://github.com/paradedb/paradedb/commit/713eaca82145388378043ddb2c750c41e9c632d9"
        },
        "date": 1778706252026,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - cpu",
            "value": 9.248554,
            "unit": "median cpu",
            "extra": "avg cpu: 8.379884950011311, max cpu: 23.054754, count: 55044"
          },
          {
            "name": "Aggregate Custom Scan - Primary - mem",
            "value": 66.64453125,
            "unit": "median mem",
            "extra": "avg mem: 66.53024419952675, max mem: 77.88671875, count: 55044"
          },
          {
            "name": "Columnar Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 5.54726766302065, max cpu: 18.408438, count: 55044"
          },
          {
            "name": "Columnar Scan - Primary - mem",
            "value": 65.50390625,
            "unit": "median mem",
            "extra": "avg mem: 65.38381943592853, max mem: 76.5546875, count: 55044"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.612425369539789, max cpu: 9.213051, count: 55044"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 35.609375,
            "unit": "median mem",
            "extra": "avg mem: 35.55862085899462, max mem: 37.8359375, count: 55044"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.671061435094161, max cpu: 9.275363, count: 55044"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 63.23828125,
            "unit": "median mem",
            "extra": "avg mem: 62.79599679631295, max mem: 74.41796875, count: 55044"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.6598133850151875, max cpu: 9.476802, count: 110088"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 51.03125,
            "unit": "median mem",
            "extra": "avg mem: 50.85595209911616, max mem: 61.6953125, count: 110088"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 1755,
            "unit": "median block_count",
            "extra": "avg block_count: 1761.1773853644356, max block_count: 3110.0, count: 55044"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 14,
            "unit": "median segment_count",
            "extra": "avg segment_count: 14.260791366906474, max segment_count: 29.0, count: 55044"
          },
          {
            "name": "Normal Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 5.508816411669013, max cpu: 18.897638, count: 55044"
          },
          {
            "name": "Normal Scan - Primary - mem",
            "value": 64.65625,
            "unit": "median mem",
            "extra": "avg mem: 64.57788811564384, max mem: 75.78515625, count: 55044"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.534078634718272, max cpu: 4.7524753, count: 55044"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 62.4296875,
            "unit": "median mem",
            "extra": "avg mem: 61.70001044618851, max mem: 72.94921875, count: 55044"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 3.7374599165406783, max cpu: 9.257474, count: 55044"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 57.4296875,
            "unit": "median mem",
            "extra": "avg mem: 56.90282724104807, max mem: 69.68359375, count: 55044"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "035982deb7c1e630e0d8a13e4ca1910b979b08f6",
          "message": "ci: add logical-replication-merge stressgres suite to cover FSM race (#4935) (#5072)\n\nRe-opened from #5068 (originally from a fork, where CI could not access\nworkflow secrets — see\nhttps://github.com/paradedb/paradedb/actions/runs/25830120431/job/75892835684?pr=5068).\nSame intent, from a branch in this repo, with the FSM-race suite landing\nas its own file rather than folded into `logical-replication.toml`.\n\n## Changes\n\n- `stressgres/suites/logical-replication-merge.toml` (new): standalone\nsuite that reliably reproduces the FSM race from #4935 (fixed in #5067).\nLogical-replication subscriber with aggressive autovacuum (`naptime=1s`,\n`threshold=50`), small `layer_sizes = '10kb, 100kb, 1mb, 100mb'`,\nmultiple concurrent BM25 readers, and sustained UPDATE/INSERT/DELETE\ntraffic on the publisher. The key difference from\n`logical-replication.toml` is the writer: `message = message || ' ' ||\ntxid_current()` grows each row's terms unbounded, generating ~10× more\nmerge/GC pressure and reliably opening the race window — folding the\nsame churn into `logical-replication.toml` (which strips-then-appends,\nkeeping row size constant) did not reproduce the bug.\n- `.github/workflows/benchmark-pg_search-stressgres.yml`:\n- Comment out single-server, bulk-updates, wide-table, and\nbackground-merge so CI focuses on the two replication suites while we\niterate. To be re-enabled before final merge.\n- Run `logical-replication-merge.toml` **before**\n`logical-replication.toml`.\n\n`stressgres/suites/logical-replication.toml` is unchanged from `main`.\n\n## Expected behavior\n\n- Without #5067: SIGSEGV or `SegmentMetaEntryHeader: UnexpectedEnd`\nwithin minutes.\n- With #5067: runs the full duration without errors.\n\n## Follow-ups\n\n- Re-enable the four commented-out suites before final merge.\n- Antithesis wiring for this suite belongs in `paradedb-enterprise` next\nto the existing `physical-logical-replication` driver, since the OSS\nAntithesis manifest only stands up a single paradedb cluster.\n\nRef: #4935\nRelated: #5067\nSupersedes: #5068",
          "timestamp": "2026-05-13T19:51:05-04:00",
          "tree_id": "c71af69d7df60d54d1631876f7e3c7af0782c3e3",
          "url": "https://github.com/paradedb/paradedb/commit/035982deb7c1e630e0d8a13e4ca1910b979b08f6"
        },
        "date": 1778718040650,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - cpu",
            "value": 9.266409,
            "unit": "median cpu",
            "extra": "avg cpu: 8.457085634268127, max cpu: 28.8, count: 55110"
          },
          {
            "name": "Aggregate Custom Scan - Primary - mem",
            "value": 66.81640625,
            "unit": "median mem",
            "extra": "avg mem: 66.64521376281527, max mem: 78.0703125, count: 55110"
          },
          {
            "name": "Columnar Scan - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 5.591686578407867, max cpu: 14.4, count: 55110"
          },
          {
            "name": "Columnar Scan - Primary - mem",
            "value": 65.6328125,
            "unit": "median mem",
            "extra": "avg mem: 65.44538444417982, max mem: 76.78515625, count: 55110"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.61448699061823, max cpu: 9.239654, count: 55110"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 35.8203125,
            "unit": "median mem",
            "extra": "avg mem: 35.49051314983669, max mem: 37.125, count: 55110"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.5701648597911655, max cpu: 4.7713714, count: 55110"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 64.17578125,
            "unit": "median mem",
            "extra": "avg mem: 63.58322134140809, max mem: 75.4609375, count: 55110"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.676991092231447, max cpu: 9.533267, count: 110220"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 56.8671875,
            "unit": "median mem",
            "extra": "avg mem: 56.18050011482716, max mem: 71.12109375, count: 110220"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 1802,
            "unit": "median block_count",
            "extra": "avg block_count: 1793.5841952458718, max block_count: 3170.0, count: 55110"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 14,
            "unit": "median segment_count",
            "extra": "avg segment_count: 15.272836145890038, max segment_count: 30.0, count: 55110"
          },
          {
            "name": "Normal Scan - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 5.7137884314736125, max cpu: 15.141956, count: 55110"
          },
          {
            "name": "Normal Scan - Primary - mem",
            "value": 65.45703125,
            "unit": "median mem",
            "extra": "avg mem: 65.25806412742696, max mem: 76.56640625, count: 55110"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.598696888274442, max cpu: 9.275363, count: 55110"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 54.33984375,
            "unit": "median mem",
            "extra": "avg mem: 54.0728347991517, max mem: 65.32421875, count: 55110"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 3.7161361749920374, max cpu: 4.6829267, count: 55110"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 57.30859375,
            "unit": "median mem",
            "extra": "avg mem: 56.05386457199238, max mem: 70.43359375, count: 55110"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "36623265+daniel3303@users.noreply.github.com",
            "name": "Daniel Oliveira",
            "username": "daniel3303"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "b9c06c5f75ca94cf815eb57c71c46180e84b8038",
          "message": "fix(mlt): quote key_field identifier in internal SPI lookup (#5078)\n\n## Summary\n\n- `pdb.more_like_this(key_value)` raises `ERROR: column \"id\" does not\nexist` whenever the index's `key_field` column is a mixed-case\nPostgreSQL identifier (e.g. `\"Id\"`, `\"DocumentId\"`). Direct `@@@`-on-LHS\nsearches (`\"Content\" @@@ 'foo'`) are unaffected because they don't go\nthrough the internal SPI lookup. Repro in #5065.\n- Root cause: `pg_search/src/query/more_like_this.rs:152-157` builds the\nSPI `SELECT * FROM <ns>.<rel> WHERE <key_field> = $1` with `<ns>` and\n`<rel>` already routed through `pgrx::spi::quote_identifier`, but\ninterpolates `<key_field>` verbatim via `Display`. PostgreSQL folds the\nunquoted reference to lowercase, so a column named `\"Id\"` is looked up\nas `id` and the SPI call fails before MLT ever runs.\n- Fix: send the key field through\n`pgrx::spi::quote_identifier(key_field_name.root())`, matching how the\nnamespace and relation names are already quoted on the lines immediately\nabove. `.root()` strips the JSON sub-path (`key_field` is always a\ntop-level column).\n\n## Scope\n\nThe linked issue also lists JSON `term` filters (`@@@\n'{\"term\":{\"field\":\"Category\",…}}'::jsonb`) as failing on mixed-case\ncolumns. That path does **not** go through SPI — `term()` in\n`pg_search/src/query/pdb_query.rs:792` resolves the field via\n`schema.search_field(field.root())`, a pure Tantivy schema lookup — so\nit isn't fixed here and I couldn't find a corresponding\nunquoted-identifier hazard. If it reproduces on `0.23.x` it's a separate\nbug; tracking it on its own issue is cleaner than bundling a speculative\nfix.\n\n## Test plan\n\n- [x] `cargo test -p tests --test mlt --\nmlt_mixed_case_key_field_issue5065` — new regression test: `\"Id\"` /\n`\"Content\"` table, `key_field='Id'`, asserts `pdb.more_like_this(1)`\nreturns rows. Fails on `main` with `column \"id\" does not exist`, passes\nwith this change.\n- [x] `cargo test -p tests --test mlt` — existing\n`mlt_enables_scoring_issue1747`, `mlt_datetime_key`,\n`mlt_scoring_nested` still pass.\n- [x] `cargo pgrx regress -p pg_search --auto -- pg18 more_like_this` —\ngolden output unchanged (`quote_identifier(\"id\")` is a no-op for\nalready-lowercase identifiers).\n- [x] Manual repro from #5065 (`CREATE TABLE items (\"Id\" int primary\nkey, \"Content\" text); … pdb.more_like_this(1)`) returns rows instead of\nerroring.\n\nCloses #5065.",
          "timestamp": "2026-05-14T14:45:18-04:00",
          "tree_id": "812b4a66ffce7bc074cb919986f1de3b6474813f",
          "url": "https://github.com/paradedb/paradedb/commit/b9c06c5f75ca94cf815eb57c71c46180e84b8038"
        },
        "date": 1778786108329,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - cpu",
            "value": 9.248554,
            "unit": "median cpu",
            "extra": "avg cpu: 8.110881428023612, max cpu: 23.391813, count: 55163"
          },
          {
            "name": "Aggregate Custom Scan - Primary - mem",
            "value": 66.875,
            "unit": "median mem",
            "extra": "avg mem: 66.6411578667993, max mem: 77.80078125, count: 55163"
          },
          {
            "name": "Columnar Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 5.640318049428433, max cpu: 18.640776, count: 55163"
          },
          {
            "name": "Columnar Scan - Primary - mem",
            "value": 65.3203125,
            "unit": "median mem",
            "extra": "avg mem: 65.09164282161503, max mem: 76.19921875, count: 55163"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.610992614267575, max cpu: 9.266409, count: 55163"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 36.6875,
            "unit": "median mem",
            "extra": "avg mem: 36.39972374485615, max mem: 38.6796875, count: 55163"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.5724237422757374, max cpu: 9.320388, count: 55163"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 63.98046875,
            "unit": "median mem",
            "extra": "avg mem: 63.47389972388648, max mem: 75.01953125, count: 55163"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.668501452100787, max cpu: 9.467456, count: 110326"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 62.4140625,
            "unit": "median mem",
            "extra": "avg mem: 60.32372373267634, max mem: 73.90234375, count: 110326"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 1786,
            "unit": "median block_count",
            "extra": "avg block_count: 1786.391548682994, max block_count: 3156.0, count: 55163"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 10,
            "unit": "median segment_count",
            "extra": "avg segment_count: 10.030727117814477, max segment_count: 20.0, count: 55163"
          },
          {
            "name": "Normal Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 5.60615793383659, max cpu: 18.972332, count: 55163"
          },
          {
            "name": "Normal Scan - Primary - mem",
            "value": 65.28125,
            "unit": "median mem",
            "extra": "avg mem: 65.01225565313254, max mem: 76.1796875, count: 55163"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.52645391685256, max cpu: 7.4766355, count: 55163"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 54.43359375,
            "unit": "median mem",
            "extra": "avg mem: 53.97597248721969, max mem: 64.88671875, count: 55163"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 3.6594680917407754, max cpu: 4.6647234, count: 55163"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 56.85546875,
            "unit": "median mem",
            "extra": "avg mem: 55.85959883946214, max mem: 69.671875, count: 55163"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "balavignesh449@gmail.com",
            "name": "S Bala Vignesh",
            "username": "SBALAVIGNESH123"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "d0039460372b22539b33262920b0194c694b7205",
          "message": "fix: use fixed-length updates in stressgres to avoid TOAST and expose FSM race (#5080)\n\nThe old UPDATE pattern in the logical-replication stressgres suite\nappended txid_current() to the message column every iteration, growing\nit past the TOAST threshold (~2KB). This caused the suite to hit the\nunrelated TOAST visibility race (#5076) before the FSM segment metadata\nrace (#4935) could surface.\n\nChanged to fixed-length updates that keep the first search term and\nappend a small txid-derived number, staying well under the TOAST\nthreshold. This way the suite can run long enough to exercise the FSM\npath.\n\nRelated: #5067 (FSM race fix), #5076 (TOAST bug)",
          "timestamp": "2026-05-14T14:57:31-04:00",
          "tree_id": "96511f40645bb3416046b0914f9758c60c159a20",
          "url": "https://github.com/paradedb/paradedb/commit/d0039460372b22539b33262920b0194c694b7205"
        },
        "date": 1778786878737,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - cpu",
            "value": 9.284333,
            "unit": "median cpu",
            "extra": "avg cpu: 8.790715129734707, max cpu: 23.575638, count: 55303"
          },
          {
            "name": "Aggregate Custom Scan - Primary - mem",
            "value": 67.44921875,
            "unit": "median mem",
            "extra": "avg mem: 67.05933180049003, max mem: 78.4765625, count: 55303"
          },
          {
            "name": "Columnar Scan - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 5.756091047122363, max cpu: 18.640776, count: 55303"
          },
          {
            "name": "Columnar Scan - Primary - mem",
            "value": 65.91015625,
            "unit": "median mem",
            "extra": "avg mem: 65.60312124229246, max mem: 76.95703125, count: 55303"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.678978912642785, max cpu: 9.356726, count: 55303"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 35.7734375,
            "unit": "median mem",
            "extra": "avg mem: 35.600475166356254, max mem: 37.55859375, count: 55303"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.754225481794006, max cpu: 9.365853, count: 55303"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 64.28125,
            "unit": "median mem",
            "extra": "avg mem: 63.85048802167152, max mem: 75.72265625, count: 55303"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.654093572285794, max cpu: 9.329447, count: 110606"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 59.96875,
            "unit": "median mem",
            "extra": "avg mem: 56.29830095112381, max mem: 71.30078125, count: 110606"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 1813,
            "unit": "median block_count",
            "extra": "avg block_count: 1808.1032855360468, max block_count: 3199.0, count: 55303"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 19,
            "unit": "median segment_count",
            "extra": "avg segment_count: 17.831274976041083, max segment_count: 30.0, count: 55303"
          },
          {
            "name": "Normal Scan - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 5.757748302399663, max cpu: 18.677044, count: 55303"
          },
          {
            "name": "Normal Scan - Primary - mem",
            "value": 65.80078125,
            "unit": "median mem",
            "extra": "avg mem: 65.44874649374808, max mem: 76.875, count: 55303"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.582073816488372, max cpu: 4.743083, count: 55303"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 54.42578125,
            "unit": "median mem",
            "extra": "avg mem: 53.997862344719096, max mem: 65.30859375, count: 55303"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 0,
            "unit": "median cpu",
            "extra": "avg cpu: 2.205093707707399, max cpu: 4.7244096, count: 55303"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 57.4921875,
            "unit": "median mem",
            "extra": "avg mem: 57.067889273072886, max mem: 70.43359375, count: 55303"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "balavignesh449@gmail.com",
            "name": "S Bala Vignesh",
            "username": "SBALAVIGNESH123"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "b597b183d25a561cf8c81d81decc7f07b7866e55",
          "message": "fix: hold header shared lock during LinkedItemList iteration to prevent FSM race (#4935) (#5067)\n\n## Problem\n\nLinkedItemList read-only iteration methods (list(), is_empty(),\nor_each(), lookup()/lookup_ex()) were releasing the header shared lock\nbefore iterating through the linked list blocks. This allowed\nAtomicGuard::commit() to:\n\n1. Swap the header pointer to a new list\n2. Immediately recycle old blocks to the FSM with\nReadNextFullTransactionId()\n3. The next transaction pops a recycled block via RBM_ZERO_AND_LOCK,\nzeroing it\n4. A concurrent reader still traversing the old list hits the\nzeroed/repurposed block\n5. **SIGSEGV** or SegmentMetaEntryHeader: UnexpectedEnd deserialization\nerror\n\nThis manifests as periodic crashes and durable index corruption under\nsustained write traffic, particularly on logical replication subscribers\nat high apply rates (~395 commits/sec). Correlates strongly with\nautovacuum events that trigger garbage_collect_index().\n\n## Root Cause\n\nThe race window exists because or_each(), list(), is_empty(), and\nlookup_ex() call get_start_blockno() which acquires a shared lock on the\nheader, reads start_blockno, then immediately releases the header lock\nwhen exchanging to the first data block. After that point, \u0007tomically()\ncan take an exclusive header lock and proceed with the swap+recycle\nwhile the reader is deep in the old list.\n\n**This was already a known pattern in the codebase** — emove_item() and\nupdate_item() in the same file both hold the header shared lock for\ntheir entire operation with this comment:\n\n\\\\\\\rust\n// Acquire and hold a shared lock on the header for the entire\noperation, preventing the\n// list from being swapped out from under us by atomically between our\nread locks and\n// our write locks.\nlet header_lock = self.bman.get_buffer(self.header_blockno);\n\\\\\\\n\nThe read-only methods simply weren't given the same treatment.\n\n## Fix\n\nHold a shared lock on the header for the entire duration of iteration in\nall 4 methods, matching the existing emove_item()/update_item() pattern:\n\n- **list()** — hold header_lock from start to end of iteration\n- **is_empty()** — same\n- **\for_each()** — same\n- **lookup_ex()** — conditionally: only when \blockno is None (top-level\ncall). When \blockno is Some, the caller ( emove_item/update_item)\nalready holds the header lock\n\nRead start_blockno directly from the already-held header_lock instead of\ncalling get_start_blockno(), avoiding a double shared-lock acquisition\non the same block (which would trigger a panic under the \block_tracker\ndebug feature).\n\n## Why This Is Safe\n\n- **No deadlock**: Header block is always locked first, content blocks\nin ascending order — consistent lock ordering\n- **No reader-reader blocking**: Multiple readers hold shared locks\nconcurrently (shared locks are compatible)\n- **Writer waits for readers**: \u0007tomically() takes an exclusive header\nlock, which blocks until all shared locks are released — correct\nserialization\n- **Minimal performance impact**: The header lock was already acquired;\nwe just hold it slightly longer\n\n## Verification\n\n- \rustfmt --check passes\n- Pattern matches the proven emove_item()/update_item() implementation\nin the same file\n- Full cargo check requires pgrx setup (PostgreSQL extension); the\nchange is limited to lock lifetime management with no new APIs\n\nCloses #4935",
          "timestamp": "2026-05-14T15:00:05-04:00",
          "tree_id": "46e474245958de09c21d0198195343d8a87fb72d",
          "url": "https://github.com/paradedb/paradedb/commit/b597b183d25a561cf8c81d81decc7f07b7866e55"
        },
        "date": 1778786997768,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - cpu",
            "value": 9.266409,
            "unit": "median cpu",
            "extra": "avg cpu: 8.412112170784164, max cpu: 19.277107, count: 55260"
          },
          {
            "name": "Aggregate Custom Scan - Primary - mem",
            "value": 67.11328125,
            "unit": "median mem",
            "extra": "avg mem: 66.91932724280673, max mem: 78.2421875, count: 55260"
          },
          {
            "name": "Columnar Scan - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 5.565489658207007, max cpu: 18.550726, count: 55260"
          },
          {
            "name": "Columnar Scan - Primary - mem",
            "value": 65.796875,
            "unit": "median mem",
            "extra": "avg mem: 65.57855033308451, max mem: 76.921875, count: 55260"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.710168283509179, max cpu: 9.320388, count: 55260"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 35.49609375,
            "unit": "median mem",
            "extra": "avg mem: 35.392050081433226, max mem: 37.76171875, count: 55260"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.685141779379805, max cpu: 9.356726, count: 55260"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 63.87109375,
            "unit": "median mem",
            "extra": "avg mem: 63.59846224099711, max mem: 75.26171875, count: 55260"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.663654606270549, max cpu: 9.320388, count: 110520"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 51.05078125,
            "unit": "median mem",
            "extra": "avg mem: 52.60004499327045, max mem: 70.82421875, count: 110520"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 1785,
            "unit": "median block_count",
            "extra": "avg block_count: 1783.3216974303293, max block_count: 3161.0, count: 55260"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 13,
            "unit": "median segment_count",
            "extra": "avg segment_count: 14.296941730003619, max segment_count: 30.0, count: 55260"
          },
          {
            "name": "Normal Scan - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 5.477819127258756, max cpu: 14.257426, count: 55260"
          },
          {
            "name": "Normal Scan - Primary - mem",
            "value": 65.3828125,
            "unit": "median mem",
            "extra": "avg mem: 65.26416026228284, max mem: 76.609375, count: 55260"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.562913899041248, max cpu: 4.7666335, count: 55260"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 54.0859375,
            "unit": "median mem",
            "extra": "avg mem: 53.4024417950371, max mem: 64.734375, count: 55260"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 3.0559913254571995, max cpu: 4.7151275, count: 55260"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 56.484375,
            "unit": "median mem",
            "extra": "avg mem: 56.23099127985885, max mem: 69.64453125, count: 55260"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "652b0952a7f3be58ea4b9fa80a863426cf03185c",
          "message": "chore(stressgres): remove dormant suites not wired into CI (#5085)\n\n## Summary\n\n- Removes 8 Stressgres suites that have not been touched since\nStressgres was added (#3821), are not referenced by any CI workflow or\ndriver script, and use the legacy `[setup_primary]` schema rather than\nthe current `[[server]]` schema.\n- No CI workflow changes; the 6 suites currently exercised by\n`benchmark-pg_search-stressgres.yml` (`single-server`, `bulk-updates`,\n`wide-table`, `background-merge`, `logical-replication`,\n`logical-replication-merge`) and `vanilla-postgres.toml` (used by the\nantithesis singleton driver) are kept.\n\n### Removed\n- `large-inserts.toml`\n- `lr.toml` — predecessor of `logical-replication.toml`\n- `lr-graphable.toml`\n- `lr-large-inserts.toml`\n- `lr-no-pg_search.toml`\n- `many-updates.toml` — conceptually useful (MVCC correctness with\n`assert(count, expected)` under concurrency), but on legacy schema; if\nwe want this coverage in CI it should be a deliberate port, not a\nrevival of dead config\n- `read-write.toml` — contained placeholder `<password>` literal, not\nCI-runnable\n- `topk-crash.toml`\n\n### Why now\nCompanion to #5080 (which adjusts `logical-replication.toml` to expose\nthe FSM race instead of being masked by the TOAST bug already covered by\n`logical-replication-merge.toml`). Each CI suite should own a distinct\nrepro; dormant files muddy that mapping.\n\n## Test plan\n- [x] `benchmark-pg_search-stressgres` workflow still kicks off for all\n6 in-CI suites\n- [x] Antithesis `singleton_driver_vanilla-postgres.sh` still resolves\nits suite path",
          "timestamp": "2026-05-14T15:45:37-04:00",
          "tree_id": "090e72f7bbb9817642fb0c7495c2b12e0eed2fe7",
          "url": "https://github.com/paradedb/paradedb/commit/652b0952a7f3be58ea4b9fa80a863426cf03185c"
        },
        "date": 1778789848844,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - cpu",
            "value": 9.266409,
            "unit": "median cpu",
            "extra": "avg cpu: 8.456777490285177, max cpu: 24.120604, count: 55172"
          },
          {
            "name": "Aggregate Custom Scan - Primary - mem",
            "value": 67.23828125,
            "unit": "median mem",
            "extra": "avg mem: 67.04977912822628, max mem: 78.33984375, count: 55172"
          },
          {
            "name": "Columnar Scan - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 5.652838189246563, max cpu: 18.953604, count: 55172"
          },
          {
            "name": "Columnar Scan - Primary - mem",
            "value": 65.75390625,
            "unit": "median mem",
            "extra": "avg mem: 65.60061723168002, max mem: 76.87890625, count: 55172"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.66665997923363, max cpu: 9.320388, count: 55172"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 35.3515625,
            "unit": "median mem",
            "extra": "avg mem: 35.42989069976618, max mem: 37.546875, count: 55172"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.768442227543103, max cpu: 9.347614, count: 55172"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 64.36328125,
            "unit": "median mem",
            "extra": "avg mem: 63.96983672934188, max mem: 75.5703125, count: 55172"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.694769376202754, max cpu: 9.338522, count: 110344"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 51.24609375,
            "unit": "median mem",
            "extra": "avg mem: 51.60111790315966, max mem: 71.05078125, count: 110344"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 1815,
            "unit": "median block_count",
            "extra": "avg block_count: 1803.970220401653, max block_count: 3179.0, count: 55172"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 16,
            "unit": "median segment_count",
            "extra": "avg segment_count: 17.3053360400203, max segment_count: 29.0, count: 55172"
          },
          {
            "name": "Normal Scan - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 5.640297461154559, max cpu: 14.4723625, count: 55172"
          },
          {
            "name": "Normal Scan - Primary - mem",
            "value": 65.6953125,
            "unit": "median mem",
            "extra": "avg mem: 65.4844465801267, max mem: 76.76171875, count: 55172"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.563785940246625, max cpu: 9.402546, count: 55172"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 54.23828125,
            "unit": "median mem",
            "extra": "avg mem: 56.16577015685039, max mem: 74.1484375, count: 55172"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 0,
            "unit": "median cpu",
            "extra": "avg cpu: 0.0, max cpu: 0.0, count: 55172"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 57.38671875,
            "unit": "median mem",
            "extra": "avg mem: 56.33343198316175, max mem: 69.75390625, count: 55172"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ming.ying.nyc@gmail.com",
            "name": "Ming",
            "username": "rebasedming"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "21bdc9a753582039cc25a38cb07111026c0fd378",
          "message": "feat: Crash recovery via WAL (#4901)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nPorts WAL integration over to community, which gives `pg_search` crash\nrecovery.\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2026-05-14T14:59:53-07:00",
          "tree_id": "7580b5f2bfcc1a95ed1b62c4628f2ec6129b7c00",
          "url": "https://github.com/paradedb/paradedb/commit/21bdc9a753582039cc25a38cb07111026c0fd378"
        },
        "date": 1778797892186,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - cpu",
            "value": 9.248554,
            "unit": "median cpu",
            "extra": "avg cpu: 8.222004859084477, max cpu: 23.054754, count: 55218"
          },
          {
            "name": "Aggregate Custom Scan - Primary - mem",
            "value": 66.828125,
            "unit": "median mem",
            "extra": "avg mem: 66.69942100232261, max mem: 77.90625, count: 55218"
          },
          {
            "name": "Columnar Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 5.408307979543307, max cpu: 18.443804, count: 55218"
          },
          {
            "name": "Columnar Scan - Primary - mem",
            "value": 65.63671875,
            "unit": "median mem",
            "extra": "avg mem: 65.49826079980713, max mem: 76.69921875, count: 55218"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.68018554736896, max cpu: 9.302325, count: 55218"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 35.60546875,
            "unit": "median mem",
            "extra": "avg mem: 35.407003264334094, max mem: 36.90234375, count: 55218"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.757181566989444, max cpu: 9.302325, count: 55218"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 63.90625,
            "unit": "median mem",
            "extra": "avg mem: 63.39495980703756, max mem: 75.0, count: 55218"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.636506996208076, max cpu: 9.375, count: 110436"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 62.54296875,
            "unit": "median mem",
            "extra": "avg mem: 61.93064329962603, max mem: 73.84375, count: 110436"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 1748,
            "unit": "median block_count",
            "extra": "avg block_count: 1753.480314390235, max block_count: 3107.0, count: 55218"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 7,
            "unit": "median segment_count",
            "extra": "avg segment_count: 8.17584845521388, max segment_count: 21.0, count: 55218"
          },
          {
            "name": "Normal Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 5.457131961330067, max cpu: 14.215202, count: 55218"
          },
          {
            "name": "Normal Scan - Primary - mem",
            "value": 65.49609375,
            "unit": "median mem",
            "extra": "avg mem: 65.35938044715944, max mem: 76.53125, count: 55218"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.549245398060131, max cpu: 4.743083, count: 55218"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 54.689453125,
            "unit": "median mem",
            "extra": "avg mem: 54.39268742190047, max mem: 65.03515625, count: 55218"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.309393207398186, max cpu: 4.6647234, count: 55218"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 57.44140625,
            "unit": "median mem",
            "extra": "avg mem: 56.31210914862454, max mem: 69.328125, count: 55218"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "d99908a1f58307567dd34698345b3fc836be6135",
          "message": "feat(antithesis): add singleton drivers for logical-replication suites (#5087)\n\n## Summary\n\nAdds OSS Antithesis singleton drivers for the two CI\n`logical-replication` suites that previously had no singleton\n(`single-server`, `bulk-updates`, `wide-table`, `background-merge`, and\n`vanilla-postgres` already had one).\n\nMirrors the enterprise pattern used for `physical-logical-replication`:\n- A **vanilla Postgres 18** publisher pod (with `wal_level=logical`)\nthat lives outside the CNPG cluster, reflecting real-world\nlogical-replication topologies where the upstream primary is not under\nour control.\n- Subscriber points at `paradedb-rw` (the CNPG primary, which has\n`pg_search`).\n\n## Changes\n- `docker/manifests/antithesis-paradedb.yaml` — add\n`logical-replication-publisher` Service + Deployment (vanilla Postgres\n18 with `wal_level=logical`), reusing the existing `paradedb-superuser`\nsecret.\n- `stressgres/suites/logical-replication.toml`,\n`stressgres/suites/logical-replication-merge.toml` — drop `CREATE\nEXTENSION pg_search` from the **Publisher** setup. Only the Subscriber\nuses `pg_search`; the line was cosmetic and incompatible with a vanilla\nPostgres publisher (the line in the Subscriber setup is unchanged).\n-\n`stressgres/suites/antithesis/singleton_driver_logical-replication.sh`,\n`singleton_driver_logical-replication-merge.sh` — new drivers that\nperform per-block `sed -z` rewrites of the `[server.style.Automatic]`\nblocks into `[server.style.With]` connection strings (Publisher →\n`logical-replication-publisher:5432`, Subscriber → `paradedb-rw:5432`).\n- `.github/workflows/antithesis-trigger-test-run.yml` — add\n`logical-replication-publisher` to\n`container_faults_stop_exclusion_patterns` and\n`container_faults_kill_exclusion_patterns`, matching enterprise. Network\nfaults to/from the publisher are intentionally still injected.\n\n## Why\nWithout these, the FSM race repro in `logical-replication-merge.toml`\n(issue #4935, fixed by #5067) and the broader logical-replication\ncoverage in `logical-replication.toml` were running in\n`benchmark-pg_search-stressgres` but had no Antithesis fault-injection\nequivalent — that's the half of the matrix where the bugs originally\nsurfaced.\n\n## Test plan\n- [x] Antithesis trigger workflow picks up both new singleton drivers\nfrom `/opt/antithesis/test/v1/quickstart/`\n- [x] Publisher pod (`logical-replication-publisher`) starts with\n`wal_level=logical` and is reachable from the stressgres-runner pod\n- [x] Subscriber's `CREATE SUBSCRIPTION ... CONNECTION\n'@Publisher_CONNSTR@'` resolves to the publisher pod after the `sed`\nrewrite\n- [x] `logical-replication-merge.toml` still reproduces the FSM race\nwhen run against a build without #5067\n- [x] `benchmark-pg_search-stressgres` (local Stressgres, not\nAntithesis) still runs both suites unchanged",
          "timestamp": "2026-05-14T18:54:16-04:00",
          "tree_id": "8c7a6dab334db43248e88e7a4ebc75fd5840e446",
          "url": "https://github.com/paradedb/paradedb/commit/d99908a1f58307567dd34698345b3fc836be6135"
        },
        "date": 1778801060859,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - cpu",
            "value": 9.257474,
            "unit": "median cpu",
            "extra": "avg cpu: 8.121336882421177, max cpu: 24.072216, count: 55026"
          },
          {
            "name": "Aggregate Custom Scan - Primary - mem",
            "value": 67.0703125,
            "unit": "median mem",
            "extra": "avg mem: 66.99224343946499, max mem: 78.25, count: 55026"
          },
          {
            "name": "Columnar Scan - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 5.553868875962391, max cpu: 18.916256, count: 55026"
          },
          {
            "name": "Columnar Scan - Primary - mem",
            "value": 65.87890625,
            "unit": "median mem",
            "extra": "avg mem: 65.78806112678552, max mem: 77.1328125, count: 55026"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.659852817381757, max cpu: 9.320388, count: 55026"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 35.484375,
            "unit": "median mem",
            "extra": "avg mem: 35.29357215793443, max mem: 37.171875, count: 55026"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.6029185298735165, max cpu: 9.29332, count: 55026"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 64.25390625,
            "unit": "median mem",
            "extra": "avg mem: 63.71599221277214, max mem: 75.6484375, count: 55026"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.708184204078254, max cpu: 9.467456, count: 110052"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 52.84765625,
            "unit": "median mem",
            "extra": "avg mem: 54.559237408793116, max mem: 74.08203125, count: 110052"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 1788,
            "unit": "median block_count",
            "extra": "avg block_count: 1789.28860902119, max block_count: 3170.0, count: 55026"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 12,
            "unit": "median segment_count",
            "extra": "avg segment_count: 12.099280340202814, max segment_count: 33.0, count: 55026"
          },
          {
            "name": "Normal Scan - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 5.595714048856729, max cpu: 18.514948, count: 55026"
          },
          {
            "name": "Normal Scan - Primary - mem",
            "value": 65.8125,
            "unit": "median mem",
            "extra": "avg mem: 65.73447488176043, max mem: 77.02734375, count: 55026"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.751844216045366, max cpu: 9.365853, count: 55026"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 54.84375,
            "unit": "median mem",
            "extra": "avg mem: 54.60135975493403, max mem: 65.62890625, count: 55026"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.619827,
            "unit": "median cpu",
            "extra": "avg cpu: 4.34503516882518, max cpu: 4.738401, count: 55026"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 57.88671875,
            "unit": "median mem",
            "extra": "avg mem: 57.91009023007306, max mem: 70.79296875, count: 55026"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "balavignesh449@gmail.com",
            "name": "S Bala Vignesh",
            "username": "SBALAVIGNESH123"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "d453f45b157fe9286838cba35678d294100dbce7",
          "message": "fix: eager detoast in index_memory_segment to prevent TOAST race with VACUUM (#5076) (#5086)\n\n## Problem\n\n`index_memory_segment` calls `ExecFetchSlotHeapTuple(slot, true, ...)`\nwhich **materializes** the tuple into palloc memory, **releasing the\nbuffer pin** held by the `BufferHeapTupleTableSlot`. Without the pin,\nVACUUM's `LockBufferForCleanup` proceeds immediately, removes the heap\ntuple, and deletes its TOAST chunks — while `row_to_search_document`\nhasn't detoasted yet.\n\nThis causes the crash:\n`\nmissing chunk number 0 for toast value XXXXX in pg_toast_17265\nLOCATION: heaptoast.c:782\n`\n\nReproduced consistently by the `logical-replication-merge.toml`\nstressgres suite.\n\n## Root Cause\n\nThe race window:\n\n1. `table_index_fetch_tuple` → slot holds buffer **pin**\n2. `HeapTupleSatisfiesVacuum` → tuple is alive, share lock dropped but\n**pin still held**\n3. `ExecFetchSlotHeapTuple(slot, true)` → materializes tuple →\n**releases pin** ⚡\n4. VACUUM calls `LockBufferForCleanup` → pin count is 0 → proceeds\n5. VACUUM removes heap tuple → deletes TOAST chunks\n6. `row_to_search_document` → `String::from_datum` → `pg_detoast_datum`\n→ reads deleted TOAST → **CRASH**\n\n## Fix\n\nTwo changes:\n\n1. **Don't materialize** — pass `false` to `ExecFetchSlotHeapTuple` to\nkeep the buffer pin held by the slot. While the pin is held,\n`LockBufferForCleanup` blocks, so VACUUM can't remove the heap tuple or\nits TOAST chunks.\n\n2. **Eager detoast** — immediately after `heap_deform_tuple`, loop\nthrough all varlena (`attlen == -1`) datums and call `pg_detoast_datum`\nwhile the pin protects the TOAST data. `pg_detoast_datum` is a no-op for\nnon-TOASTed / already-inline data.\n\n## Why This Is Safe\n\n- **No deadlock**: buffer pin blocks only `LockBufferForCleanup` (VACUUM\ncleanup), not normal reads/writes\n- **Heap tuple immutability**: tuple data in the buffer page is\nimmutable once written — updates create new tuples\n- **Expression eval safe**: `expression_state.evaluate(slot)` still\nworks because the slot has a valid buffer-backed tuple with pin held\n- **Memory**: only allocates palloc copies for actually-TOASTed datums;\nfreed at memory context reset\n- **HOT chains**: handled by `table_index_fetch_tuple` before we see the\ntuple\n\n## Verification\n\n- `rustfmt --check` passes\n- 1 file changed, 28 insertions, 6 deletions\n- Should be validated against `logical-replication-merge.toml`\nstressgres suite\n\nCloses #5076\n\n---------\n\nCo-authored-by: Philippe Noël <philippemnoel@gmail.com>",
          "timestamp": "2026-05-15T11:24:27-04:00",
          "tree_id": "83e864df0221b99ca4fe39e5255c3e8061c8c9c6",
          "url": "https://github.com/paradedb/paradedb/commit/d453f45b157fe9286838cba35678d294100dbce7"
        },
        "date": 1778860456722,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Aggregate Custom Scan - Primary - cpu",
            "value": 9.266409,
            "unit": "median cpu",
            "extra": "avg cpu: 8.393935109711286, max cpu: 24.096386, count: 55134"
          },
          {
            "name": "Aggregate Custom Scan - Primary - mem",
            "value": 67.21875,
            "unit": "median mem",
            "extra": "avg mem: 66.80408584086045, max mem: 78.01953125, count: 55134"
          },
          {
            "name": "Columnar Scan - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 5.800912431190017, max cpu: 18.916256, count: 55134"
          },
          {
            "name": "Columnar Scan - Primary - mem",
            "value": 66.15234375,
            "unit": "median mem",
            "extra": "avg mem: 65.691273689443, max mem: 76.90234375, count: 55134"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.676880876619584, max cpu: 9.329447, count: 55134"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 34.84375,
            "unit": "median mem",
            "extra": "avg mem: 35.06650848670965, max mem: 36.4609375, count: 55134"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.7515141440214705, max cpu: 9.384164, count: 55134"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 64.5234375,
            "unit": "median mem",
            "extra": "avg mem: 63.79963114015399, max mem: 75.3828125, count: 55134"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.678366555111544, max cpu: 9.638554, count: 110268"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 51.96484375,
            "unit": "median mem",
            "extra": "avg mem: 53.4404955781029, max mem: 72.328125, count: 110268"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 1822,
            "unit": "median block_count",
            "extra": "avg block_count: 1800.3559328182248, max block_count: 3171.0, count: 55134"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 20,
            "unit": "median segment_count",
            "extra": "avg segment_count: 18.19563245909965, max segment_count: 30.0, count: 55134"
          },
          {
            "name": "Normal Scan - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 5.717465815100169, max cpu: 18.695229, count: 55134"
          },
          {
            "name": "Normal Scan - Primary - mem",
            "value": 66.00390625,
            "unit": "median mem",
            "extra": "avg mem: 65.53467035994123, max mem: 76.73828125, count: 55134"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.526171420534327, max cpu: 4.743083, count: 55134"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 54.87109375,
            "unit": "median mem",
            "extra": "avg mem: 54.590692038374684, max mem: 65.484375, count: 55134"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 2.5898608769206697, max cpu: 4.655674, count: 55134"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 57.55859375,
            "unit": "median mem",
            "extra": "avg mem: 57.11222303274749, max mem: 69.6328125, count: 55134"
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
      },
      {
        "commit": {
          "author": {
            "email": "59696464+saadtajwar@users.noreply.github.com",
            "name": "Saad Tajwar",
            "username": "saadtajwar"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "bc20c3dd4b36a13dbcc74d03499f966a4dc93fe6",
          "message": "refactor: Deduplicate deferred materialization request partitioning between visibility and lookup (#4903)\n\n# Ticket(s) Closed\n- Closes https://github.com/paradedb/paradedb/issues/4568\n\n## What\nDeduplicated the segment-grouping/materialization loop shared between\n`materialize_deferred_ctid()` in `visibility_filter.rs` and\n`materialize_deferred_column()` in `tantivy_lookup_exec.rs`.\n\n## Why\nBoth functions implemented the same pattern of partitioning packed doc\naddresses by segment ordinal, batch-reading via `FFHelper`, and writing\nresults back in row order. This duplication made the two paths harder to\nkeep consistent and made future optimization work more tedious.\n\n## How\n\n### Shared helper in `fast_fields_helper.rs`\n- Added `for_each_segment`: partitions an iterator of `(row_index,\npacked_doc_address)` pairs into per-segment buckets and invokes a\ncaller-supplied closure once per non-empty segment, in segment-ordinal\norder. Backed by a `Vec<Vec<(usize, DocId)>>` indexed by segment ordinal\n(dense in practice; cheaper than a hash map for typical segment counts).\n- Added `FFHelper::num_segments()` so callers can size the bucket vector\nwithout reaching into private fields.\n\n### `materialize_deferred_ctid` (visibility_filter.rs)\n- Replaced the manual sort + partition + per-segment slice loop with a\nsingle `for_each_segment` call.\n- Kept `DeferredCtidMaterializationState` for buffer reuse across calls,\nbut removed its now-unused `requests` field; the per-segment\npartitioning lives inside `for_each_segment`.\n- Removed the TODO comment that flagged this duplication.\n\n### `materialize_deferred_column` (tantivy_lookup_exec.rs)\nFunction body shrank from ~160 lines to ~50 by extracting three\nsingle-responsibility helpers:\n- `resolve_doc_addresses_to_term_ords` — resolves State 0 (packed doc\naddresses) into per-segment `(row_index, Option<TermOrdinal>)` pairs via\n`for_each_segment`.\n- `extract_term_ords` — parses State 1 (pre-resolved `(segment_ord,\nterm_ord)` pairs from the dense union's `StructArray` child) into the\nsame per-segment shape.\n- `decode_term_ordinals` — takes the merged per-segment ordinals and\nperforms the bulk dictionary lookup once per segment, recording\npositions for the final `interleave`.\n\nState 0 and State 1 are now merged into a single `Vec<Vec<(row_index,\nOption<TermOrdinal>)>>` indexed by segment ordinal, then decoded in one\npass — previously each state was iterated and decoded separately,\nproducing two `segment_arrays` entries per segment touched by both. The\nfinal interleaved output is identical.\n\nReplaced the `(ff_index: usize, is_bytes: bool)` parameter pair with a\n`DeferredColumnKind { Text { ff_index }, Bytes { ff_index } }` enum to\nmake the `is_bytes && wrong-ff-type` mismatch unrepresentable.\n\n## Tests\nNo new tests; behavior is unchanged. Existing coverage exercises both\npaths end-to-end:\n- `pg_search/tests/pg_regress/sql/join_deferred_visibility.sql` —\n`materialize_deferred_ctid`.\n- `pg_search/tests/pg_regress/sql/segmented_topk.sql` plus joinscan\ntests — `materialize_deferred_column` (the segmented top-K rule is what\nproduces State 1 rows below `TantivyLookupExec`).\n\n---------\n\nCo-authored-by: Mithun Chicklore Yogendra <mithun.cy@gmail.com>",
          "timestamp": "2026-05-13T10:07:59-07:00",
          "tree_id": "cc9e05e63ed6052c202d00901357d2c5026923d0",
          "url": "https://github.com/paradedb/paradedb/commit/bc20c3dd4b36a13dbcc74d03499f966a4dc93fe6"
        },
        "date": 1778694539912,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 7.873468966241056,
            "unit": "median tps",
            "extra": "avg tps: 6.71580526044521, max tps: 10.263221582026793, count: 57560"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 4.979055180956926,
            "unit": "median tps",
            "extra": "avg tps: 4.50420475253943, max tps: 5.5283729957916785, count: 57560"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "stuhood@paradedb.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "713eaca82145388378043ddb2c750c41e9c632d9",
          "message": "chore: Revert #4905 (#5066)\n\nRevert #4905 due to performance regressions.",
          "timestamp": "2026-05-13T13:34:27-07:00",
          "tree_id": "2894402ba71bbb0b86d5990862fb2adba4fcbf49",
          "url": "https://github.com/paradedb/paradedb/commit/713eaca82145388378043ddb2c750c41e9c632d9"
        },
        "date": 1778706920848,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 7.908492720193085,
            "unit": "median tps",
            "extra": "avg tps: 6.762105383131605, max tps: 10.310500958262214, count: 57800"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.542560894801501,
            "unit": "median tps",
            "extra": "avg tps: 4.9674367368052454, max tps: 6.210930003725544, count: 57800"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "035982deb7c1e630e0d8a13e4ca1910b979b08f6",
          "message": "ci: add logical-replication-merge stressgres suite to cover FSM race (#4935) (#5072)\n\nRe-opened from #5068 (originally from a fork, where CI could not access\nworkflow secrets — see\nhttps://github.com/paradedb/paradedb/actions/runs/25830120431/job/75892835684?pr=5068).\nSame intent, from a branch in this repo, with the FSM-race suite landing\nas its own file rather than folded into `logical-replication.toml`.\n\n## Changes\n\n- `stressgres/suites/logical-replication-merge.toml` (new): standalone\nsuite that reliably reproduces the FSM race from #4935 (fixed in #5067).\nLogical-replication subscriber with aggressive autovacuum (`naptime=1s`,\n`threshold=50`), small `layer_sizes = '10kb, 100kb, 1mb, 100mb'`,\nmultiple concurrent BM25 readers, and sustained UPDATE/INSERT/DELETE\ntraffic on the publisher. The key difference from\n`logical-replication.toml` is the writer: `message = message || ' ' ||\ntxid_current()` grows each row's terms unbounded, generating ~10× more\nmerge/GC pressure and reliably opening the race window — folding the\nsame churn into `logical-replication.toml` (which strips-then-appends,\nkeeping row size constant) did not reproduce the bug.\n- `.github/workflows/benchmark-pg_search-stressgres.yml`:\n- Comment out single-server, bulk-updates, wide-table, and\nbackground-merge so CI focuses on the two replication suites while we\niterate. To be re-enabled before final merge.\n- Run `logical-replication-merge.toml` **before**\n`logical-replication.toml`.\n\n`stressgres/suites/logical-replication.toml` is unchanged from `main`.\n\n## Expected behavior\n\n- Without #5067: SIGSEGV or `SegmentMetaEntryHeader: UnexpectedEnd`\nwithin minutes.\n- With #5067: runs the full duration without errors.\n\n## Follow-ups\n\n- Re-enable the four commented-out suites before final merge.\n- Antithesis wiring for this suite belongs in `paradedb-enterprise` next\nto the existing `physical-logical-replication` driver, since the OSS\nAntithesis manifest only stands up a single paradedb cluster.\n\nRef: #4935\nRelated: #5067\nSupersedes: #5068",
          "timestamp": "2026-05-13T19:51:05-04:00",
          "tree_id": "c71af69d7df60d54d1631876f7e3c7af0782c3e3",
          "url": "https://github.com/paradedb/paradedb/commit/035982deb7c1e630e0d8a13e4ca1910b979b08f6"
        },
        "date": 1778718709065,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.066524058548447,
            "unit": "median tps",
            "extra": "avg tps: 6.889636027430244, max tps: 10.456891354934571, count: 57831"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.5030537070324,
            "unit": "median tps",
            "extra": "avg tps: 4.941520072861939, max tps: 6.184354379873542, count: 57831"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "36623265+daniel3303@users.noreply.github.com",
            "name": "Daniel Oliveira",
            "username": "daniel3303"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "b9c06c5f75ca94cf815eb57c71c46180e84b8038",
          "message": "fix(mlt): quote key_field identifier in internal SPI lookup (#5078)\n\n## Summary\n\n- `pdb.more_like_this(key_value)` raises `ERROR: column \"id\" does not\nexist` whenever the index's `key_field` column is a mixed-case\nPostgreSQL identifier (e.g. `\"Id\"`, `\"DocumentId\"`). Direct `@@@`-on-LHS\nsearches (`\"Content\" @@@ 'foo'`) are unaffected because they don't go\nthrough the internal SPI lookup. Repro in #5065.\n- Root cause: `pg_search/src/query/more_like_this.rs:152-157` builds the\nSPI `SELECT * FROM <ns>.<rel> WHERE <key_field> = $1` with `<ns>` and\n`<rel>` already routed through `pgrx::spi::quote_identifier`, but\ninterpolates `<key_field>` verbatim via `Display`. PostgreSQL folds the\nunquoted reference to lowercase, so a column named `\"Id\"` is looked up\nas `id` and the SPI call fails before MLT ever runs.\n- Fix: send the key field through\n`pgrx::spi::quote_identifier(key_field_name.root())`, matching how the\nnamespace and relation names are already quoted on the lines immediately\nabove. `.root()` strips the JSON sub-path (`key_field` is always a\ntop-level column).\n\n## Scope\n\nThe linked issue also lists JSON `term` filters (`@@@\n'{\"term\":{\"field\":\"Category\",…}}'::jsonb`) as failing on mixed-case\ncolumns. That path does **not** go through SPI — `term()` in\n`pg_search/src/query/pdb_query.rs:792` resolves the field via\n`schema.search_field(field.root())`, a pure Tantivy schema lookup — so\nit isn't fixed here and I couldn't find a corresponding\nunquoted-identifier hazard. If it reproduces on `0.23.x` it's a separate\nbug; tracking it on its own issue is cleaner than bundling a speculative\nfix.\n\n## Test plan\n\n- [x] `cargo test -p tests --test mlt --\nmlt_mixed_case_key_field_issue5065` — new regression test: `\"Id\"` /\n`\"Content\"` table, `key_field='Id'`, asserts `pdb.more_like_this(1)`\nreturns rows. Fails on `main` with `column \"id\" does not exist`, passes\nwith this change.\n- [x] `cargo test -p tests --test mlt` — existing\n`mlt_enables_scoring_issue1747`, `mlt_datetime_key`,\n`mlt_scoring_nested` still pass.\n- [x] `cargo pgrx regress -p pg_search --auto -- pg18 more_like_this` —\ngolden output unchanged (`quote_identifier(\"id\")` is a no-op for\nalready-lowercase identifiers).\n- [x] Manual repro from #5065 (`CREATE TABLE items (\"Id\" int primary\nkey, \"Content\" text); … pdb.more_like_this(1)`) returns rows instead of\nerroring.\n\nCloses #5065.",
          "timestamp": "2026-05-14T14:45:18-04:00",
          "tree_id": "812b4a66ffce7bc074cb919986f1de3b6474813f",
          "url": "https://github.com/paradedb/paradedb/commit/b9c06c5f75ca94cf815eb57c71c46180e84b8038"
        },
        "date": 1778786779017,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.0084375242706,
            "unit": "median tps",
            "extra": "avg tps: 6.843249044361455, max tps: 10.512587573874692, count: 57549"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.442195151952854,
            "unit": "median tps",
            "extra": "avg tps: 4.888723181460216, max tps: 6.101064832013489, count: 57549"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "balavignesh449@gmail.com",
            "name": "S Bala Vignesh",
            "username": "SBALAVIGNESH123"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "d0039460372b22539b33262920b0194c694b7205",
          "message": "fix: use fixed-length updates in stressgres to avoid TOAST and expose FSM race (#5080)\n\nThe old UPDATE pattern in the logical-replication stressgres suite\nappended txid_current() to the message column every iteration, growing\nit past the TOAST threshold (~2KB). This caused the suite to hit the\nunrelated TOAST visibility race (#5076) before the FSM segment metadata\nrace (#4935) could surface.\n\nChanged to fixed-length updates that keep the first search term and\nappend a small txid-derived number, staying well under the TOAST\nthreshold. This way the suite can run long enough to exercise the FSM\npath.\n\nRelated: #5067 (FSM race fix), #5076 (TOAST bug)",
          "timestamp": "2026-05-14T14:57:31-04:00",
          "tree_id": "96511f40645bb3416046b0914f9758c60c159a20",
          "url": "https://github.com/paradedb/paradedb/commit/d0039460372b22539b33262920b0194c694b7205"
        },
        "date": 1778787548875,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.234612414074082,
            "unit": "median tps",
            "extra": "avg tps: 7.025852448959892, max tps: 10.643308953453184, count: 57798"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.451507079026225,
            "unit": "median tps",
            "extra": "avg tps: 4.909010731389204, max tps: 6.1049126073980355, count: 57798"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "balavignesh449@gmail.com",
            "name": "S Bala Vignesh",
            "username": "SBALAVIGNESH123"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "b597b183d25a561cf8c81d81decc7f07b7866e55",
          "message": "fix: hold header shared lock during LinkedItemList iteration to prevent FSM race (#4935) (#5067)\n\n## Problem\n\nLinkedItemList read-only iteration methods (list(), is_empty(),\nor_each(), lookup()/lookup_ex()) were releasing the header shared lock\nbefore iterating through the linked list blocks. This allowed\nAtomicGuard::commit() to:\n\n1. Swap the header pointer to a new list\n2. Immediately recycle old blocks to the FSM with\nReadNextFullTransactionId()\n3. The next transaction pops a recycled block via RBM_ZERO_AND_LOCK,\nzeroing it\n4. A concurrent reader still traversing the old list hits the\nzeroed/repurposed block\n5. **SIGSEGV** or SegmentMetaEntryHeader: UnexpectedEnd deserialization\nerror\n\nThis manifests as periodic crashes and durable index corruption under\nsustained write traffic, particularly on logical replication subscribers\nat high apply rates (~395 commits/sec). Correlates strongly with\nautovacuum events that trigger garbage_collect_index().\n\n## Root Cause\n\nThe race window exists because or_each(), list(), is_empty(), and\nlookup_ex() call get_start_blockno() which acquires a shared lock on the\nheader, reads start_blockno, then immediately releases the header lock\nwhen exchanging to the first data block. After that point, \u0007tomically()\ncan take an exclusive header lock and proceed with the swap+recycle\nwhile the reader is deep in the old list.\n\n**This was already a known pattern in the codebase** — emove_item() and\nupdate_item() in the same file both hold the header shared lock for\ntheir entire operation with this comment:\n\n\\\\\\\rust\n// Acquire and hold a shared lock on the header for the entire\noperation, preventing the\n// list from being swapped out from under us by atomically between our\nread locks and\n// our write locks.\nlet header_lock = self.bman.get_buffer(self.header_blockno);\n\\\\\\\n\nThe read-only methods simply weren't given the same treatment.\n\n## Fix\n\nHold a shared lock on the header for the entire duration of iteration in\nall 4 methods, matching the existing emove_item()/update_item() pattern:\n\n- **list()** — hold header_lock from start to end of iteration\n- **is_empty()** — same\n- **\for_each()** — same\n- **lookup_ex()** — conditionally: only when \blockno is None (top-level\ncall). When \blockno is Some, the caller ( emove_item/update_item)\nalready holds the header lock\n\nRead start_blockno directly from the already-held header_lock instead of\ncalling get_start_blockno(), avoiding a double shared-lock acquisition\non the same block (which would trigger a panic under the \block_tracker\ndebug feature).\n\n## Why This Is Safe\n\n- **No deadlock**: Header block is always locked first, content blocks\nin ascending order — consistent lock ordering\n- **No reader-reader blocking**: Multiple readers hold shared locks\nconcurrently (shared locks are compatible)\n- **Writer waits for readers**: \u0007tomically() takes an exclusive header\nlock, which blocks until all shared locks are released — correct\nserialization\n- **Minimal performance impact**: The header lock was already acquired;\nwe just hold it slightly longer\n\n## Verification\n\n- \rustfmt --check passes\n- Pattern matches the proven emove_item()/update_item() implementation\nin the same file\n- Full cargo check requires pgrx setup (PostgreSQL extension); the\nchange is limited to lock lifetime management with no new APIs\n\nCloses #4935",
          "timestamp": "2026-05-14T15:00:05-04:00",
          "tree_id": "46e474245958de09c21d0198195343d8a87fb72d",
          "url": "https://github.com/paradedb/paradedb/commit/b597b183d25a561cf8c81d81decc7f07b7866e55"
        },
        "date": 1778787665773,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.049615394622446,
            "unit": "median tps",
            "extra": "avg tps: 6.860910403691916, max tps: 10.519633329855818, count: 57788"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.4465829378269675,
            "unit": "median tps",
            "extra": "avg tps: 4.893592866028497, max tps: 6.105128860429523, count: 57788"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "652b0952a7f3be58ea4b9fa80a863426cf03185c",
          "message": "chore(stressgres): remove dormant suites not wired into CI (#5085)\n\n## Summary\n\n- Removes 8 Stressgres suites that have not been touched since\nStressgres was added (#3821), are not referenced by any CI workflow or\ndriver script, and use the legacy `[setup_primary]` schema rather than\nthe current `[[server]]` schema.\n- No CI workflow changes; the 6 suites currently exercised by\n`benchmark-pg_search-stressgres.yml` (`single-server`, `bulk-updates`,\n`wide-table`, `background-merge`, `logical-replication`,\n`logical-replication-merge`) and `vanilla-postgres.toml` (used by the\nantithesis singleton driver) are kept.\n\n### Removed\n- `large-inserts.toml`\n- `lr.toml` — predecessor of `logical-replication.toml`\n- `lr-graphable.toml`\n- `lr-large-inserts.toml`\n- `lr-no-pg_search.toml`\n- `many-updates.toml` — conceptually useful (MVCC correctness with\n`assert(count, expected)` under concurrency), but on legacy schema; if\nwe want this coverage in CI it should be a deliberate port, not a\nrevival of dead config\n- `read-write.toml` — contained placeholder `<password>` literal, not\nCI-runnable\n- `topk-crash.toml`\n\n### Why now\nCompanion to #5080 (which adjusts `logical-replication.toml` to expose\nthe FSM race instead of being masked by the TOAST bug already covered by\n`logical-replication-merge.toml`). Each CI suite should own a distinct\nrepro; dormant files muddy that mapping.\n\n## Test plan\n- [x] `benchmark-pg_search-stressgres` workflow still kicks off for all\n6 in-CI suites\n- [x] Antithesis `singleton_driver_vanilla-postgres.sh` still resolves\nits suite path",
          "timestamp": "2026-05-14T15:45:37-04:00",
          "tree_id": "090e72f7bbb9817642fb0c7495c2b12e0eed2fe7",
          "url": "https://github.com/paradedb/paradedb/commit/652b0952a7f3be58ea4b9fa80a863426cf03185c"
        },
        "date": 1778790517346,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 7.928725439789105,
            "unit": "median tps",
            "extra": "avg tps: 6.782043051896884, max tps: 10.332886875485269, count: 57762"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.555363464279229,
            "unit": "median tps",
            "extra": "avg tps: 4.980421325578402, max tps: 6.216536358685152, count: 57762"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ming.ying.nyc@gmail.com",
            "name": "Ming",
            "username": "rebasedming"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "21bdc9a753582039cc25a38cb07111026c0fd378",
          "message": "feat: Crash recovery via WAL (#4901)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nPorts WAL integration over to community, which gives `pg_search` crash\nrecovery.\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2026-05-14T14:59:53-07:00",
          "tree_id": "7580b5f2bfcc1a95ed1b62c4628f2ec6129b7c00",
          "url": "https://github.com/paradedb/paradedb/commit/21bdc9a753582039cc25a38cb07111026c0fd378"
        },
        "date": 1778798560687,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 7.867127882789328,
            "unit": "median tps",
            "extra": "avg tps: 6.7356997920890755, max tps: 10.292129127108055, count: 57916"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.596630837075229,
            "unit": "median tps",
            "extra": "avg tps: 5.015657679035947, max tps: 6.258931941371654, count: 57916"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "d99908a1f58307567dd34698345b3fc836be6135",
          "message": "feat(antithesis): add singleton drivers for logical-replication suites (#5087)\n\n## Summary\n\nAdds OSS Antithesis singleton drivers for the two CI\n`logical-replication` suites that previously had no singleton\n(`single-server`, `bulk-updates`, `wide-table`, `background-merge`, and\n`vanilla-postgres` already had one).\n\nMirrors the enterprise pattern used for `physical-logical-replication`:\n- A **vanilla Postgres 18** publisher pod (with `wal_level=logical`)\nthat lives outside the CNPG cluster, reflecting real-world\nlogical-replication topologies where the upstream primary is not under\nour control.\n- Subscriber points at `paradedb-rw` (the CNPG primary, which has\n`pg_search`).\n\n## Changes\n- `docker/manifests/antithesis-paradedb.yaml` — add\n`logical-replication-publisher` Service + Deployment (vanilla Postgres\n18 with `wal_level=logical`), reusing the existing `paradedb-superuser`\nsecret.\n- `stressgres/suites/logical-replication.toml`,\n`stressgres/suites/logical-replication-merge.toml` — drop `CREATE\nEXTENSION pg_search` from the **Publisher** setup. Only the Subscriber\nuses `pg_search`; the line was cosmetic and incompatible with a vanilla\nPostgres publisher (the line in the Subscriber setup is unchanged).\n-\n`stressgres/suites/antithesis/singleton_driver_logical-replication.sh`,\n`singleton_driver_logical-replication-merge.sh` — new drivers that\nperform per-block `sed -z` rewrites of the `[server.style.Automatic]`\nblocks into `[server.style.With]` connection strings (Publisher →\n`logical-replication-publisher:5432`, Subscriber → `paradedb-rw:5432`).\n- `.github/workflows/antithesis-trigger-test-run.yml` — add\n`logical-replication-publisher` to\n`container_faults_stop_exclusion_patterns` and\n`container_faults_kill_exclusion_patterns`, matching enterprise. Network\nfaults to/from the publisher are intentionally still injected.\n\n## Why\nWithout these, the FSM race repro in `logical-replication-merge.toml`\n(issue #4935, fixed by #5067) and the broader logical-replication\ncoverage in `logical-replication.toml` were running in\n`benchmark-pg_search-stressgres` but had no Antithesis fault-injection\nequivalent — that's the half of the matrix where the bugs originally\nsurfaced.\n\n## Test plan\n- [x] Antithesis trigger workflow picks up both new singleton drivers\nfrom `/opt/antithesis/test/v1/quickstart/`\n- [x] Publisher pod (`logical-replication-publisher`) starts with\n`wal_level=logical` and is reachable from the stressgres-runner pod\n- [x] Subscriber's `CREATE SUBSCRIPTION ... CONNECTION\n'@Publisher_CONNSTR@'` resolves to the publisher pod after the `sed`\nrewrite\n- [x] `logical-replication-merge.toml` still reproduces the FSM race\nwhen run against a build without #5067\n- [x] `benchmark-pg_search-stressgres` (local Stressgres, not\nAntithesis) still runs both suites unchanged",
          "timestamp": "2026-05-14T18:54:16-04:00",
          "tree_id": "8c7a6dab334db43248e88e7a4ebc75fd5840e446",
          "url": "https://github.com/paradedb/paradedb/commit/d99908a1f58307567dd34698345b3fc836be6135"
        },
        "date": 1778801731300,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.159284546092588,
            "unit": "median tps",
            "extra": "avg tps: 6.98008174969918, max tps: 10.574163137019223, count: 57839"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.429637983193888,
            "unit": "median tps",
            "extra": "avg tps: 4.885926358020856, max tps: 6.066870028530966, count: 57839"
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
        "date": 1778653711969,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.255816,
            "unit": "median cpu",
            "extra": "avg cpu: 20.731689672056202, max cpu: 42.942345, count: 57775"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 235.796875,
            "unit": "median mem",
            "extra": "avg mem: 235.58068125270447, max mem: 237.26953125, count: 57775"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.391813,
            "unit": "median cpu",
            "extra": "avg cpu: 22.794181973208214, max cpu: 33.366436, count: 57775"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 178.22265625,
            "unit": "median mem",
            "extra": "avg mem: 177.79632856447427, max mem: 178.34375, count: 57775"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 34859,
            "unit": "median block_count",
            "extra": "avg block_count: 33857.10874945911, max block_count: 36421.0, count: 57775"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 79,
            "unit": "median segment_count",
            "extra": "avg segment_count: 81.91861531804413, max segment_count: 131.0, count: 57775"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "59696464+saadtajwar@users.noreply.github.com",
            "name": "Saad Tajwar",
            "username": "saadtajwar"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "bc20c3dd4b36a13dbcc74d03499f966a4dc93fe6",
          "message": "refactor: Deduplicate deferred materialization request partitioning between visibility and lookup (#4903)\n\n# Ticket(s) Closed\n- Closes https://github.com/paradedb/paradedb/issues/4568\n\n## What\nDeduplicated the segment-grouping/materialization loop shared between\n`materialize_deferred_ctid()` in `visibility_filter.rs` and\n`materialize_deferred_column()` in `tantivy_lookup_exec.rs`.\n\n## Why\nBoth functions implemented the same pattern of partitioning packed doc\naddresses by segment ordinal, batch-reading via `FFHelper`, and writing\nresults back in row order. This duplication made the two paths harder to\nkeep consistent and made future optimization work more tedious.\n\n## How\n\n### Shared helper in `fast_fields_helper.rs`\n- Added `for_each_segment`: partitions an iterator of `(row_index,\npacked_doc_address)` pairs into per-segment buckets and invokes a\ncaller-supplied closure once per non-empty segment, in segment-ordinal\norder. Backed by a `Vec<Vec<(usize, DocId)>>` indexed by segment ordinal\n(dense in practice; cheaper than a hash map for typical segment counts).\n- Added `FFHelper::num_segments()` so callers can size the bucket vector\nwithout reaching into private fields.\n\n### `materialize_deferred_ctid` (visibility_filter.rs)\n- Replaced the manual sort + partition + per-segment slice loop with a\nsingle `for_each_segment` call.\n- Kept `DeferredCtidMaterializationState` for buffer reuse across calls,\nbut removed its now-unused `requests` field; the per-segment\npartitioning lives inside `for_each_segment`.\n- Removed the TODO comment that flagged this duplication.\n\n### `materialize_deferred_column` (tantivy_lookup_exec.rs)\nFunction body shrank from ~160 lines to ~50 by extracting three\nsingle-responsibility helpers:\n- `resolve_doc_addresses_to_term_ords` — resolves State 0 (packed doc\naddresses) into per-segment `(row_index, Option<TermOrdinal>)` pairs via\n`for_each_segment`.\n- `extract_term_ords` — parses State 1 (pre-resolved `(segment_ord,\nterm_ord)` pairs from the dense union's `StructArray` child) into the\nsame per-segment shape.\n- `decode_term_ordinals` — takes the merged per-segment ordinals and\nperforms the bulk dictionary lookup once per segment, recording\npositions for the final `interleave`.\n\nState 0 and State 1 are now merged into a single `Vec<Vec<(row_index,\nOption<TermOrdinal>)>>` indexed by segment ordinal, then decoded in one\npass — previously each state was iterated and decoded separately,\nproducing two `segment_arrays` entries per segment touched by both. The\nfinal interleaved output is identical.\n\nReplaced the `(ff_index: usize, is_bytes: bool)` parameter pair with a\n`DeferredColumnKind { Text { ff_index }, Bytes { ff_index } }` enum to\nmake the `is_bytes && wrong-ff-type` mismatch unrepresentable.\n\n## Tests\nNo new tests; behavior is unchanged. Existing coverage exercises both\npaths end-to-end:\n- `pg_search/tests/pg_regress/sql/join_deferred_visibility.sql` —\n`materialize_deferred_ctid`.\n- `pg_search/tests/pg_regress/sql/segmented_topk.sql` plus joinscan\ntests — `materialize_deferred_column` (the segmented top-K rule is what\nproduces State 1 rows below `TantivyLookupExec`).\n\n---------\n\nCo-authored-by: Mithun Chicklore Yogendra <mithun.cy@gmail.com>",
          "timestamp": "2026-05-13T10:07:59-07:00",
          "tree_id": "cc9e05e63ed6052c202d00901357d2c5026923d0",
          "url": "https://github.com/paradedb/paradedb/commit/bc20c3dd4b36a13dbcc74d03499f966a4dc93fe6"
        },
        "date": 1778694573529,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.255816,
            "unit": "median cpu",
            "extra": "avg cpu: 20.80269916122381, max cpu: 42.857143, count: 57560"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 235.859375,
            "unit": "median mem",
            "extra": "avg mem: 235.65804513768242, max mem: 237.36328125, count: 57560"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.391813,
            "unit": "median cpu",
            "extra": "avg cpu: 22.84282786654086, max cpu: 33.333336, count: 57560"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 178.08203125,
            "unit": "median mem",
            "extra": "avg mem: 177.51069183243573, max mem: 178.31640625, count: 57560"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 34797,
            "unit": "median block_count",
            "extra": "avg block_count: 33811.448436414175, max block_count: 36722.0, count: 57560"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 79,
            "unit": "median segment_count",
            "extra": "avg segment_count: 81.9292043085476, max segment_count: 131.0, count: 57560"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "stuhood@paradedb.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "713eaca82145388378043ddb2c750c41e9c632d9",
          "message": "chore: Revert #4905 (#5066)\n\nRevert #4905 due to performance regressions.",
          "timestamp": "2026-05-13T13:34:27-07:00",
          "tree_id": "2894402ba71bbb0b86d5990862fb2adba4fcbf49",
          "url": "https://github.com/paradedb/paradedb/commit/713eaca82145388378043ddb2c750c41e9c632d9"
        },
        "date": 1778706952702,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.255816,
            "unit": "median cpu",
            "extra": "avg cpu: 20.76999444963387, max cpu: 42.942345, count: 57800"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 235.484375,
            "unit": "median mem",
            "extra": "avg mem: 235.36457957125864, max mem: 236.953125, count: 57800"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.391813,
            "unit": "median cpu",
            "extra": "avg cpu: 22.421060363381077, max cpu: 33.267326, count: 57800"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 177.3671875,
            "unit": "median mem",
            "extra": "avg mem: 177.26452510002161, max mem: 178.10546875, count: 57800"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 34657,
            "unit": "median block_count",
            "extra": "avg block_count: 33870.134930795844, max block_count: 36524.0, count: 57800"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 79,
            "unit": "median segment_count",
            "extra": "avg segment_count: 81.8833044982699, max segment_count: 130.0, count: 57800"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "035982deb7c1e630e0d8a13e4ca1910b979b08f6",
          "message": "ci: add logical-replication-merge stressgres suite to cover FSM race (#4935) (#5072)\n\nRe-opened from #5068 (originally from a fork, where CI could not access\nworkflow secrets — see\nhttps://github.com/paradedb/paradedb/actions/runs/25830120431/job/75892835684?pr=5068).\nSame intent, from a branch in this repo, with the FSM-race suite landing\nas its own file rather than folded into `logical-replication.toml`.\n\n## Changes\n\n- `stressgres/suites/logical-replication-merge.toml` (new): standalone\nsuite that reliably reproduces the FSM race from #4935 (fixed in #5067).\nLogical-replication subscriber with aggressive autovacuum (`naptime=1s`,\n`threshold=50`), small `layer_sizes = '10kb, 100kb, 1mb, 100mb'`,\nmultiple concurrent BM25 readers, and sustained UPDATE/INSERT/DELETE\ntraffic on the publisher. The key difference from\n`logical-replication.toml` is the writer: `message = message || ' ' ||\ntxid_current()` grows each row's terms unbounded, generating ~10× more\nmerge/GC pressure and reliably opening the race window — folding the\nsame churn into `logical-replication.toml` (which strips-then-appends,\nkeeping row size constant) did not reproduce the bug.\n- `.github/workflows/benchmark-pg_search-stressgres.yml`:\n- Comment out single-server, bulk-updates, wide-table, and\nbackground-merge so CI focuses on the two replication suites while we\niterate. To be re-enabled before final merge.\n- Run `logical-replication-merge.toml` **before**\n`logical-replication.toml`.\n\n`stressgres/suites/logical-replication.toml` is unchanged from `main`.\n\n## Expected behavior\n\n- Without #5067: SIGSEGV or `SegmentMetaEntryHeader: UnexpectedEnd`\nwithin minutes.\n- With #5067: runs the full duration without errors.\n\n## Follow-ups\n\n- Re-enable the four commented-out suites before final merge.\n- Antithesis wiring for this suite belongs in `paradedb-enterprise` next\nto the existing `physical-logical-replication` driver, since the OSS\nAntithesis manifest only stands up a single paradedb cluster.\n\nRef: #4935\nRelated: #5067\nSupersedes: #5068",
          "timestamp": "2026-05-13T19:51:05-04:00",
          "tree_id": "c71af69d7df60d54d1631876f7e3c7af0782c3e3",
          "url": "https://github.com/paradedb/paradedb/commit/035982deb7c1e630e0d8a13e4ca1910b979b08f6"
        },
        "date": 1778718740602,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.255816,
            "unit": "median cpu",
            "extra": "avg cpu: 20.456157577567676, max cpu: 43.02789, count: 57831"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 235.68359375,
            "unit": "median mem",
            "extra": "avg mem: 235.5511626819958, max mem: 237.171875, count: 57831"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.391813,
            "unit": "median cpu",
            "extra": "avg cpu: 22.45937519438953, max cpu: 33.23442, count: 57831"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 177.55859375,
            "unit": "median mem",
            "extra": "avg mem: 177.47358392611662, max mem: 178.328125, count: 57831"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 34912,
            "unit": "median block_count",
            "extra": "avg block_count: 33940.98044301499, max block_count: 36764.0, count: 57831"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 79,
            "unit": "median segment_count",
            "extra": "avg segment_count: 82.15093980736975, max segment_count: 129.0, count: 57831"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "36623265+daniel3303@users.noreply.github.com",
            "name": "Daniel Oliveira",
            "username": "daniel3303"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "b9c06c5f75ca94cf815eb57c71c46180e84b8038",
          "message": "fix(mlt): quote key_field identifier in internal SPI lookup (#5078)\n\n## Summary\n\n- `pdb.more_like_this(key_value)` raises `ERROR: column \"id\" does not\nexist` whenever the index's `key_field` column is a mixed-case\nPostgreSQL identifier (e.g. `\"Id\"`, `\"DocumentId\"`). Direct `@@@`-on-LHS\nsearches (`\"Content\" @@@ 'foo'`) are unaffected because they don't go\nthrough the internal SPI lookup. Repro in #5065.\n- Root cause: `pg_search/src/query/more_like_this.rs:152-157` builds the\nSPI `SELECT * FROM <ns>.<rel> WHERE <key_field> = $1` with `<ns>` and\n`<rel>` already routed through `pgrx::spi::quote_identifier`, but\ninterpolates `<key_field>` verbatim via `Display`. PostgreSQL folds the\nunquoted reference to lowercase, so a column named `\"Id\"` is looked up\nas `id` and the SPI call fails before MLT ever runs.\n- Fix: send the key field through\n`pgrx::spi::quote_identifier(key_field_name.root())`, matching how the\nnamespace and relation names are already quoted on the lines immediately\nabove. `.root()` strips the JSON sub-path (`key_field` is always a\ntop-level column).\n\n## Scope\n\nThe linked issue also lists JSON `term` filters (`@@@\n'{\"term\":{\"field\":\"Category\",…}}'::jsonb`) as failing on mixed-case\ncolumns. That path does **not** go through SPI — `term()` in\n`pg_search/src/query/pdb_query.rs:792` resolves the field via\n`schema.search_field(field.root())`, a pure Tantivy schema lookup — so\nit isn't fixed here and I couldn't find a corresponding\nunquoted-identifier hazard. If it reproduces on `0.23.x` it's a separate\nbug; tracking it on its own issue is cleaner than bundling a speculative\nfix.\n\n## Test plan\n\n- [x] `cargo test -p tests --test mlt --\nmlt_mixed_case_key_field_issue5065` — new regression test: `\"Id\"` /\n`\"Content\"` table, `key_field='Id'`, asserts `pdb.more_like_this(1)`\nreturns rows. Fails on `main` with `column \"id\" does not exist`, passes\nwith this change.\n- [x] `cargo test -p tests --test mlt` — existing\n`mlt_enables_scoring_issue1747`, `mlt_datetime_key`,\n`mlt_scoring_nested` still pass.\n- [x] `cargo pgrx regress -p pg_search --auto -- pg18 more_like_this` —\ngolden output unchanged (`quote_identifier(\"id\")` is a no-op for\nalready-lowercase identifiers).\n- [x] Manual repro from #5065 (`CREATE TABLE items (\"Id\" int primary\nkey, \"Content\" text); … pdb.more_like_this(1)`) returns rows instead of\nerroring.\n\nCloses #5065.",
          "timestamp": "2026-05-14T14:45:18-04:00",
          "tree_id": "812b4a66ffce7bc074cb919986f1de3b6474813f",
          "url": "https://github.com/paradedb/paradedb/commit/b9c06c5f75ca94cf815eb57c71c46180e84b8038"
        },
        "date": 1778786811528,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.166023,
            "unit": "median cpu",
            "extra": "avg cpu: 20.400243833815164, max cpu: 42.899704, count: 57549"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 235.8671875,
            "unit": "median mem",
            "extra": "avg mem: 235.68721315205303, max mem: 237.33984375, count: 57549"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.30097,
            "unit": "median cpu",
            "extra": "avg cpu: 22.371934542168017, max cpu: 33.300297, count: 57549"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 177.64453125,
            "unit": "median mem",
            "extra": "avg mem: 177.45154543573736, max mem: 178.4921875, count: 57549"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 34942,
            "unit": "median block_count",
            "extra": "avg block_count: 33944.608299014755, max block_count: 36771.0, count: 57549"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 79,
            "unit": "median segment_count",
            "extra": "avg segment_count: 82.11225216771794, max segment_count: 132.0, count: 57549"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "balavignesh449@gmail.com",
            "name": "S Bala Vignesh",
            "username": "SBALAVIGNESH123"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "d0039460372b22539b33262920b0194c694b7205",
          "message": "fix: use fixed-length updates in stressgres to avoid TOAST and expose FSM race (#5080)\n\nThe old UPDATE pattern in the logical-replication stressgres suite\nappended txid_current() to the message column every iteration, growing\nit past the TOAST threshold (~2KB). This caused the suite to hit the\nunrelated TOAST visibility race (#5076) before the FSM segment metadata\nrace (#4935) could surface.\n\nChanged to fixed-length updates that keep the first search term and\nappend a small txid-derived number, staying well under the TOAST\nthreshold. This way the suite can run long enough to exercise the FSM\npath.\n\nRelated: #5067 (FSM race fix), #5076 (TOAST bug)",
          "timestamp": "2026-05-14T14:57:31-04:00",
          "tree_id": "96511f40645bb3416046b0914f9758c60c159a20",
          "url": "https://github.com/paradedb/paradedb/commit/d0039460372b22539b33262920b0194c694b7205"
        },
        "date": 1778787612478,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.188406,
            "unit": "median cpu",
            "extra": "avg cpu: 20.293507611301358, max cpu: 42.942345, count: 57798"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 235.74609375,
            "unit": "median mem",
            "extra": "avg mem: 235.59237891287327, max mem: 237.22265625, count: 57798"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.369036,
            "unit": "median cpu",
            "extra": "avg cpu: 22.379902707107796, max cpu: 33.333336, count: 57798"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 178.203125,
            "unit": "median mem",
            "extra": "avg mem: 177.7383608645628, max mem: 178.359375, count: 57798"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 35026,
            "unit": "median block_count",
            "extra": "avg block_count: 34046.98406519257, max block_count: 36872.0, count: 57798"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 80,
            "unit": "median segment_count",
            "extra": "avg segment_count: 82.43164123326066, max segment_count: 130.0, count: 57798"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "balavignesh449@gmail.com",
            "name": "S Bala Vignesh",
            "username": "SBALAVIGNESH123"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "b597b183d25a561cf8c81d81decc7f07b7866e55",
          "message": "fix: hold header shared lock during LinkedItemList iteration to prevent FSM race (#4935) (#5067)\n\n## Problem\n\nLinkedItemList read-only iteration methods (list(), is_empty(),\nor_each(), lookup()/lookup_ex()) were releasing the header shared lock\nbefore iterating through the linked list blocks. This allowed\nAtomicGuard::commit() to:\n\n1. Swap the header pointer to a new list\n2. Immediately recycle old blocks to the FSM with\nReadNextFullTransactionId()\n3. The next transaction pops a recycled block via RBM_ZERO_AND_LOCK,\nzeroing it\n4. A concurrent reader still traversing the old list hits the\nzeroed/repurposed block\n5. **SIGSEGV** or SegmentMetaEntryHeader: UnexpectedEnd deserialization\nerror\n\nThis manifests as periodic crashes and durable index corruption under\nsustained write traffic, particularly on logical replication subscribers\nat high apply rates (~395 commits/sec). Correlates strongly with\nautovacuum events that trigger garbage_collect_index().\n\n## Root Cause\n\nThe race window exists because or_each(), list(), is_empty(), and\nlookup_ex() call get_start_blockno() which acquires a shared lock on the\nheader, reads start_blockno, then immediately releases the header lock\nwhen exchanging to the first data block. After that point, \u0007tomically()\ncan take an exclusive header lock and proceed with the swap+recycle\nwhile the reader is deep in the old list.\n\n**This was already a known pattern in the codebase** — emove_item() and\nupdate_item() in the same file both hold the header shared lock for\ntheir entire operation with this comment:\n\n\\\\\\\rust\n// Acquire and hold a shared lock on the header for the entire\noperation, preventing the\n// list from being swapped out from under us by atomically between our\nread locks and\n// our write locks.\nlet header_lock = self.bman.get_buffer(self.header_blockno);\n\\\\\\\n\nThe read-only methods simply weren't given the same treatment.\n\n## Fix\n\nHold a shared lock on the header for the entire duration of iteration in\nall 4 methods, matching the existing emove_item()/update_item() pattern:\n\n- **list()** — hold header_lock from start to end of iteration\n- **is_empty()** — same\n- **\for_each()** — same\n- **lookup_ex()** — conditionally: only when \blockno is None (top-level\ncall). When \blockno is Some, the caller ( emove_item/update_item)\nalready holds the header lock\n\nRead start_blockno directly from the already-held header_lock instead of\ncalling get_start_blockno(), avoiding a double shared-lock acquisition\non the same block (which would trigger a panic under the \block_tracker\ndebug feature).\n\n## Why This Is Safe\n\n- **No deadlock**: Header block is always locked first, content blocks\nin ascending order — consistent lock ordering\n- **No reader-reader blocking**: Multiple readers hold shared locks\nconcurrently (shared locks are compatible)\n- **Writer waits for readers**: \u0007tomically() takes an exclusive header\nlock, which blocks until all shared locks are released — correct\nserialization\n- **Minimal performance impact**: The header lock was already acquired;\nwe just hold it slightly longer\n\n## Verification\n\n- \rustfmt --check passes\n- Pattern matches the proven emove_item()/update_item() implementation\nin the same file\n- Full cargo check requires pgrx setup (PostgreSQL extension); the\nchange is limited to lock lifetime management with no new APIs\n\nCloses #4935",
          "timestamp": "2026-05-14T15:00:05-04:00",
          "tree_id": "46e474245958de09c21d0198195343d8a87fb72d",
          "url": "https://github.com/paradedb/paradedb/commit/b597b183d25a561cf8c81d81decc7f07b7866e55"
        },
        "date": 1778787697620,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.233301,
            "unit": "median cpu",
            "extra": "avg cpu: 20.500520402303295, max cpu: 43.11377, count: 57788"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 235.73046875,
            "unit": "median mem",
            "extra": "avg mem: 235.57824072471794, max mem: 237.203125, count: 57788"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.391813,
            "unit": "median cpu",
            "extra": "avg cpu: 22.440725880732373, max cpu: 33.333336, count: 57788"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 177.62109375,
            "unit": "median mem",
            "extra": "avg mem: 177.42669406927044, max mem: 178.4140625, count: 57788"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 34842,
            "unit": "median block_count",
            "extra": "avg block_count: 34006.48115525715, max block_count: 36776.0, count: 57788"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 79,
            "unit": "median segment_count",
            "extra": "avg segment_count: 82.32806118917422, max segment_count: 132.0, count: 57788"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "652b0952a7f3be58ea4b9fa80a863426cf03185c",
          "message": "chore(stressgres): remove dormant suites not wired into CI (#5085)\n\n## Summary\n\n- Removes 8 Stressgres suites that have not been touched since\nStressgres was added (#3821), are not referenced by any CI workflow or\ndriver script, and use the legacy `[setup_primary]` schema rather than\nthe current `[[server]]` schema.\n- No CI workflow changes; the 6 suites currently exercised by\n`benchmark-pg_search-stressgres.yml` (`single-server`, `bulk-updates`,\n`wide-table`, `background-merge`, `logical-replication`,\n`logical-replication-merge`) and `vanilla-postgres.toml` (used by the\nantithesis singleton driver) are kept.\n\n### Removed\n- `large-inserts.toml`\n- `lr.toml` — predecessor of `logical-replication.toml`\n- `lr-graphable.toml`\n- `lr-large-inserts.toml`\n- `lr-no-pg_search.toml`\n- `many-updates.toml` — conceptually useful (MVCC correctness with\n`assert(count, expected)` under concurrency), but on legacy schema; if\nwe want this coverage in CI it should be a deliberate port, not a\nrevival of dead config\n- `read-write.toml` — contained placeholder `<password>` literal, not\nCI-runnable\n- `topk-crash.toml`\n\n### Why now\nCompanion to #5080 (which adjusts `logical-replication.toml` to expose\nthe FSM race instead of being masked by the TOAST bug already covered by\n`logical-replication-merge.toml`). Each CI suite should own a distinct\nrepro; dormant files muddy that mapping.\n\n## Test plan\n- [x] `benchmark-pg_search-stressgres` workflow still kicks off for all\n6 in-CI suites\n- [x] Antithesis `singleton_driver_vanilla-postgres.sh` still resolves\nits suite path",
          "timestamp": "2026-05-14T15:45:37-04:00",
          "tree_id": "090e72f7bbb9817642fb0c7495c2b12e0eed2fe7",
          "url": "https://github.com/paradedb/paradedb/commit/652b0952a7f3be58ea4b9fa80a863426cf03185c"
        },
        "date": 1778790549147,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.233301,
            "unit": "median cpu",
            "extra": "avg cpu: 20.62503953965039, max cpu: 42.899704, count: 57762"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 235.67578125,
            "unit": "median mem",
            "extra": "avg mem: 235.51434252839238, max mem: 237.1484375, count: 57762"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.369036,
            "unit": "median cpu",
            "extra": "avg cpu: 22.451520511391653, max cpu: 33.267326, count: 57762"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 177.75,
            "unit": "median mem",
            "extra": "avg mem: 177.6740535245923, max mem: 178.40625, count: 57762"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 34595,
            "unit": "median block_count",
            "extra": "avg block_count: 33820.53377653129, max block_count: 36588.0, count: 57762"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 79,
            "unit": "median segment_count",
            "extra": "avg segment_count: 81.96823170942834, max segment_count: 134.0, count: 57762"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ming.ying.nyc@gmail.com",
            "name": "Ming",
            "username": "rebasedming"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "21bdc9a753582039cc25a38cb07111026c0fd378",
          "message": "feat: Crash recovery via WAL (#4901)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nPorts WAL integration over to community, which gives `pg_search` crash\nrecovery.\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2026-05-14T14:59:53-07:00",
          "tree_id": "7580b5f2bfcc1a95ed1b62c4628f2ec6129b7c00",
          "url": "https://github.com/paradedb/paradedb/commit/21bdc9a753582039cc25a38cb07111026c0fd378"
        },
        "date": 1778798592482,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.233301,
            "unit": "median cpu",
            "extra": "avg cpu: 20.729577064515652, max cpu: 42.899704, count: 57916"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 235.98046875,
            "unit": "median mem",
            "extra": "avg mem: 235.77589296891188, max mem: 237.52734375, count: 57916"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.369036,
            "unit": "median cpu",
            "extra": "avg cpu: 22.386974823808412, max cpu: 33.366436, count: 57916"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 177.89453125,
            "unit": "median mem",
            "extra": "avg mem: 177.71920634518096, max mem: 178.69921875, count: 57916"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 34579,
            "unit": "median block_count",
            "extra": "avg block_count: 33874.113371089166, max block_count: 36626.0, count: 57916"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 79,
            "unit": "median segment_count",
            "extra": "avg segment_count: 82.15822915947234, max segment_count: 133.0, count: 57916"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "d99908a1f58307567dd34698345b3fc836be6135",
          "message": "feat(antithesis): add singleton drivers for logical-replication suites (#5087)\n\n## Summary\n\nAdds OSS Antithesis singleton drivers for the two CI\n`logical-replication` suites that previously had no singleton\n(`single-server`, `bulk-updates`, `wide-table`, `background-merge`, and\n`vanilla-postgres` already had one).\n\nMirrors the enterprise pattern used for `physical-logical-replication`:\n- A **vanilla Postgres 18** publisher pod (with `wal_level=logical`)\nthat lives outside the CNPG cluster, reflecting real-world\nlogical-replication topologies where the upstream primary is not under\nour control.\n- Subscriber points at `paradedb-rw` (the CNPG primary, which has\n`pg_search`).\n\n## Changes\n- `docker/manifests/antithesis-paradedb.yaml` — add\n`logical-replication-publisher` Service + Deployment (vanilla Postgres\n18 with `wal_level=logical`), reusing the existing `paradedb-superuser`\nsecret.\n- `stressgres/suites/logical-replication.toml`,\n`stressgres/suites/logical-replication-merge.toml` — drop `CREATE\nEXTENSION pg_search` from the **Publisher** setup. Only the Subscriber\nuses `pg_search`; the line was cosmetic and incompatible with a vanilla\nPostgres publisher (the line in the Subscriber setup is unchanged).\n-\n`stressgres/suites/antithesis/singleton_driver_logical-replication.sh`,\n`singleton_driver_logical-replication-merge.sh` — new drivers that\nperform per-block `sed -z` rewrites of the `[server.style.Automatic]`\nblocks into `[server.style.With]` connection strings (Publisher →\n`logical-replication-publisher:5432`, Subscriber → `paradedb-rw:5432`).\n- `.github/workflows/antithesis-trigger-test-run.yml` — add\n`logical-replication-publisher` to\n`container_faults_stop_exclusion_patterns` and\n`container_faults_kill_exclusion_patterns`, matching enterprise. Network\nfaults to/from the publisher are intentionally still injected.\n\n## Why\nWithout these, the FSM race repro in `logical-replication-merge.toml`\n(issue #4935, fixed by #5067) and the broader logical-replication\ncoverage in `logical-replication.toml` were running in\n`benchmark-pg_search-stressgres` but had no Antithesis fault-injection\nequivalent — that's the half of the matrix where the bugs originally\nsurfaced.\n\n## Test plan\n- [x] Antithesis trigger workflow picks up both new singleton drivers\nfrom `/opt/antithesis/test/v1/quickstart/`\n- [x] Publisher pod (`logical-replication-publisher`) starts with\n`wal_level=logical` and is reachable from the stressgres-runner pod\n- [x] Subscriber's `CREATE SUBSCRIPTION ... CONNECTION\n'@Publisher_CONNSTR@'` resolves to the publisher pod after the `sed`\nrewrite\n- [x] `logical-replication-merge.toml` still reproduces the FSM race\nwhen run against a build without #5067\n- [x] `benchmark-pg_search-stressgres` (local Stressgres, not\nAntithesis) still runs both suites unchanged",
          "timestamp": "2026-05-14T18:54:16-04:00",
          "tree_id": "8c7a6dab334db43248e88e7a4ebc75fd5840e446",
          "url": "https://github.com/paradedb/paradedb/commit/d99908a1f58307567dd34698345b3fc836be6135"
        },
        "date": 1778801763185,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.233301,
            "unit": "median cpu",
            "extra": "avg cpu: 20.446608089099144, max cpu: 43.548386, count: 57839"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 236.0078125,
            "unit": "median mem",
            "extra": "avg mem: 235.82522821971335, max mem: 237.6484375, count: 57839"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.391813,
            "unit": "median cpu",
            "extra": "avg cpu: 22.376473187044944, max cpu: 33.3996, count: 57839"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 178.04296875,
            "unit": "median mem",
            "extra": "avg mem: 177.82702786776656, max mem: 178.56640625, count: 57839"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 35102,
            "unit": "median block_count",
            "extra": "avg block_count: 34076.36961219938, max block_count: 36820.0, count: 57839"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 80,
            "unit": "median segment_count",
            "extra": "avg segment_count: 82.50156468818618, max segment_count: 134.0, count: 57839"
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
        "date": 1778654399181,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 1147.8470770345978,
            "unit": "median tps",
            "extra": "avg tps: 1150.2922515509767, max tps: 1200.2858076353207, count: 56471"
          },
          {
            "name": "Single Insert - Primary - tps",
            "value": 1244.4968631962186,
            "unit": "median tps",
            "extra": "avg tps: 1246.1833644196086, max tps: 1292.018511167934, count: 56471"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 1096.0162274728639,
            "unit": "median tps",
            "extra": "avg tps: 1001.3206117357543, max tps: 1572.364540139715, count: 56471"
          },
          {
            "name": "Top K - Primary - tps",
            "value": 5.443307528420728,
            "unit": "median tps",
            "extra": "avg tps: 5.4607327000455665, max tps: 7.366024535933088, count: 56471"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "59696464+saadtajwar@users.noreply.github.com",
            "name": "Saad Tajwar",
            "username": "saadtajwar"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "bc20c3dd4b36a13dbcc74d03499f966a4dc93fe6",
          "message": "refactor: Deduplicate deferred materialization request partitioning between visibility and lookup (#4903)\n\n# Ticket(s) Closed\n- Closes https://github.com/paradedb/paradedb/issues/4568\n\n## What\nDeduplicated the segment-grouping/materialization loop shared between\n`materialize_deferred_ctid()` in `visibility_filter.rs` and\n`materialize_deferred_column()` in `tantivy_lookup_exec.rs`.\n\n## Why\nBoth functions implemented the same pattern of partitioning packed doc\naddresses by segment ordinal, batch-reading via `FFHelper`, and writing\nresults back in row order. This duplication made the two paths harder to\nkeep consistent and made future optimization work more tedious.\n\n## How\n\n### Shared helper in `fast_fields_helper.rs`\n- Added `for_each_segment`: partitions an iterator of `(row_index,\npacked_doc_address)` pairs into per-segment buckets and invokes a\ncaller-supplied closure once per non-empty segment, in segment-ordinal\norder. Backed by a `Vec<Vec<(usize, DocId)>>` indexed by segment ordinal\n(dense in practice; cheaper than a hash map for typical segment counts).\n- Added `FFHelper::num_segments()` so callers can size the bucket vector\nwithout reaching into private fields.\n\n### `materialize_deferred_ctid` (visibility_filter.rs)\n- Replaced the manual sort + partition + per-segment slice loop with a\nsingle `for_each_segment` call.\n- Kept `DeferredCtidMaterializationState` for buffer reuse across calls,\nbut removed its now-unused `requests` field; the per-segment\npartitioning lives inside `for_each_segment`.\n- Removed the TODO comment that flagged this duplication.\n\n### `materialize_deferred_column` (tantivy_lookup_exec.rs)\nFunction body shrank from ~160 lines to ~50 by extracting three\nsingle-responsibility helpers:\n- `resolve_doc_addresses_to_term_ords` — resolves State 0 (packed doc\naddresses) into per-segment `(row_index, Option<TermOrdinal>)` pairs via\n`for_each_segment`.\n- `extract_term_ords` — parses State 1 (pre-resolved `(segment_ord,\nterm_ord)` pairs from the dense union's `StructArray` child) into the\nsame per-segment shape.\n- `decode_term_ordinals` — takes the merged per-segment ordinals and\nperforms the bulk dictionary lookup once per segment, recording\npositions for the final `interleave`.\n\nState 0 and State 1 are now merged into a single `Vec<Vec<(row_index,\nOption<TermOrdinal>)>>` indexed by segment ordinal, then decoded in one\npass — previously each state was iterated and decoded separately,\nproducing two `segment_arrays` entries per segment touched by both. The\nfinal interleaved output is identical.\n\nReplaced the `(ff_index: usize, is_bytes: bool)` parameter pair with a\n`DeferredColumnKind { Text { ff_index }, Bytes { ff_index } }` enum to\nmake the `is_bytes && wrong-ff-type` mismatch unrepresentable.\n\n## Tests\nNo new tests; behavior is unchanged. Existing coverage exercises both\npaths end-to-end:\n- `pg_search/tests/pg_regress/sql/join_deferred_visibility.sql` —\n`materialize_deferred_ctid`.\n- `pg_search/tests/pg_regress/sql/segmented_topk.sql` plus joinscan\ntests — `materialize_deferred_column` (the segmented top-K rule is what\nproduces State 1 rows below `TantivyLookupExec`).\n\n---------\n\nCo-authored-by: Mithun Chicklore Yogendra <mithun.cy@gmail.com>",
          "timestamp": "2026-05-13T10:07:59-07:00",
          "tree_id": "cc9e05e63ed6052c202d00901357d2c5026923d0",
          "url": "https://github.com/paradedb/paradedb/commit/bc20c3dd4b36a13dbcc74d03499f966a4dc93fe6"
        },
        "date": 1778695261727,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 1111.7071739256721,
            "unit": "median tps",
            "extra": "avg tps: 1112.0965858438828, max tps: 1153.4976576171416, count: 56232"
          },
          {
            "name": "Single Insert - Primary - tps",
            "value": 1266.5454611806958,
            "unit": "median tps",
            "extra": "avg tps: 1261.6975426719232, max tps: 1290.6292771076476, count: 56232"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 1825.45382137221,
            "unit": "median tps",
            "extra": "avg tps: 1802.688417733488, max tps: 1976.9885471604578, count: 56232"
          },
          {
            "name": "Top K - Primary - tps",
            "value": 5.4408748036708126,
            "unit": "median tps",
            "extra": "avg tps: 5.492919885131672, max tps: 7.647066470595882, count: 56232"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "stuhood@paradedb.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "713eaca82145388378043ddb2c750c41e9c632d9",
          "message": "chore: Revert #4905 (#5066)\n\nRevert #4905 due to performance regressions.",
          "timestamp": "2026-05-13T13:34:27-07:00",
          "tree_id": "2894402ba71bbb0b86d5990862fb2adba4fcbf49",
          "url": "https://github.com/paradedb/paradedb/commit/713eaca82145388378043ddb2c750c41e9c632d9"
        },
        "date": 1778707640454,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 1121.9940943074162,
            "unit": "median tps",
            "extra": "avg tps: 1123.2002472792774, max tps: 1176.5529475283774, count: 56376"
          },
          {
            "name": "Single Insert - Primary - tps",
            "value": 1280.2462425621757,
            "unit": "median tps",
            "extra": "avg tps: 1269.607610746773, max tps: 1301.141498631938, count: 56376"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 1126.0909025672488,
            "unit": "median tps",
            "extra": "avg tps: 1031.9013278790455, max tps: 1551.5950434399776, count: 56376"
          },
          {
            "name": "Top K - Primary - tps",
            "value": 5.605698507020971,
            "unit": "median tps",
            "extra": "avg tps: 5.587936701367002, max tps: 6.802997604834618, count: 56376"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "035982deb7c1e630e0d8a13e4ca1910b979b08f6",
          "message": "ci: add logical-replication-merge stressgres suite to cover FSM race (#4935) (#5072)\n\nRe-opened from #5068 (originally from a fork, where CI could not access\nworkflow secrets — see\nhttps://github.com/paradedb/paradedb/actions/runs/25830120431/job/75892835684?pr=5068).\nSame intent, from a branch in this repo, with the FSM-race suite landing\nas its own file rather than folded into `logical-replication.toml`.\n\n## Changes\n\n- `stressgres/suites/logical-replication-merge.toml` (new): standalone\nsuite that reliably reproduces the FSM race from #4935 (fixed in #5067).\nLogical-replication subscriber with aggressive autovacuum (`naptime=1s`,\n`threshold=50`), small `layer_sizes = '10kb, 100kb, 1mb, 100mb'`,\nmultiple concurrent BM25 readers, and sustained UPDATE/INSERT/DELETE\ntraffic on the publisher. The key difference from\n`logical-replication.toml` is the writer: `message = message || ' ' ||\ntxid_current()` grows each row's terms unbounded, generating ~10× more\nmerge/GC pressure and reliably opening the race window — folding the\nsame churn into `logical-replication.toml` (which strips-then-appends,\nkeeping row size constant) did not reproduce the bug.\n- `.github/workflows/benchmark-pg_search-stressgres.yml`:\n- Comment out single-server, bulk-updates, wide-table, and\nbackground-merge so CI focuses on the two replication suites while we\niterate. To be re-enabled before final merge.\n- Run `logical-replication-merge.toml` **before**\n`logical-replication.toml`.\n\n`stressgres/suites/logical-replication.toml` is unchanged from `main`.\n\n## Expected behavior\n\n- Without #5067: SIGSEGV or `SegmentMetaEntryHeader: UnexpectedEnd`\nwithin minutes.\n- With #5067: runs the full duration without errors.\n\n## Follow-ups\n\n- Re-enable the four commented-out suites before final merge.\n- Antithesis wiring for this suite belongs in `paradedb-enterprise` next\nto the existing `physical-logical-replication` driver, since the OSS\nAntithesis manifest only stands up a single paradedb cluster.\n\nRef: #4935\nRelated: #5067\nSupersedes: #5068",
          "timestamp": "2026-05-13T19:51:05-04:00",
          "tree_id": "c71af69d7df60d54d1631876f7e3c7af0782c3e3",
          "url": "https://github.com/paradedb/paradedb/commit/035982deb7c1e630e0d8a13e4ca1910b979b08f6"
        },
        "date": 1778719428028,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 1112.5879780280816,
            "unit": "median tps",
            "extra": "avg tps: 1113.9760431707687, max tps: 1157.0643630261054, count: 56406"
          },
          {
            "name": "Single Insert - Primary - tps",
            "value": 1283.385692994159,
            "unit": "median tps",
            "extra": "avg tps: 1278.075033198787, max tps: 1295.9067665655643, count: 56406"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 1858.2313367018533,
            "unit": "median tps",
            "extra": "avg tps: 1824.071143564629, max tps: 2018.1128773458463, count: 56406"
          },
          {
            "name": "Top K - Primary - tps",
            "value": 5.135590996941359,
            "unit": "median tps",
            "extra": "avg tps: 5.177790543813255, max tps: 6.840686830541955, count: 56406"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "36623265+daniel3303@users.noreply.github.com",
            "name": "Daniel Oliveira",
            "username": "daniel3303"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "b9c06c5f75ca94cf815eb57c71c46180e84b8038",
          "message": "fix(mlt): quote key_field identifier in internal SPI lookup (#5078)\n\n## Summary\n\n- `pdb.more_like_this(key_value)` raises `ERROR: column \"id\" does not\nexist` whenever the index's `key_field` column is a mixed-case\nPostgreSQL identifier (e.g. `\"Id\"`, `\"DocumentId\"`). Direct `@@@`-on-LHS\nsearches (`\"Content\" @@@ 'foo'`) are unaffected because they don't go\nthrough the internal SPI lookup. Repro in #5065.\n- Root cause: `pg_search/src/query/more_like_this.rs:152-157` builds the\nSPI `SELECT * FROM <ns>.<rel> WHERE <key_field> = $1` with `<ns>` and\n`<rel>` already routed through `pgrx::spi::quote_identifier`, but\ninterpolates `<key_field>` verbatim via `Display`. PostgreSQL folds the\nunquoted reference to lowercase, so a column named `\"Id\"` is looked up\nas `id` and the SPI call fails before MLT ever runs.\n- Fix: send the key field through\n`pgrx::spi::quote_identifier(key_field_name.root())`, matching how the\nnamespace and relation names are already quoted on the lines immediately\nabove. `.root()` strips the JSON sub-path (`key_field` is always a\ntop-level column).\n\n## Scope\n\nThe linked issue also lists JSON `term` filters (`@@@\n'{\"term\":{\"field\":\"Category\",…}}'::jsonb`) as failing on mixed-case\ncolumns. That path does **not** go through SPI — `term()` in\n`pg_search/src/query/pdb_query.rs:792` resolves the field via\n`schema.search_field(field.root())`, a pure Tantivy schema lookup — so\nit isn't fixed here and I couldn't find a corresponding\nunquoted-identifier hazard. If it reproduces on `0.23.x` it's a separate\nbug; tracking it on its own issue is cleaner than bundling a speculative\nfix.\n\n## Test plan\n\n- [x] `cargo test -p tests --test mlt --\nmlt_mixed_case_key_field_issue5065` — new regression test: `\"Id\"` /\n`\"Content\"` table, `key_field='Id'`, asserts `pdb.more_like_this(1)`\nreturns rows. Fails on `main` with `column \"id\" does not exist`, passes\nwith this change.\n- [x] `cargo test -p tests --test mlt` — existing\n`mlt_enables_scoring_issue1747`, `mlt_datetime_key`,\n`mlt_scoring_nested` still pass.\n- [x] `cargo pgrx regress -p pg_search --auto -- pg18 more_like_this` —\ngolden output unchanged (`quote_identifier(\"id\")` is a no-op for\nalready-lowercase identifiers).\n- [x] Manual repro from #5065 (`CREATE TABLE items (\"Id\" int primary\nkey, \"Content\" text); … pdb.more_like_this(1)`) returns rows instead of\nerroring.\n\nCloses #5065.",
          "timestamp": "2026-05-14T14:45:18-04:00",
          "tree_id": "812b4a66ffce7bc074cb919986f1de3b6474813f",
          "url": "https://github.com/paradedb/paradedb/commit/b9c06c5f75ca94cf815eb57c71c46180e84b8038"
        },
        "date": 1778787498594,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 1122.354107364427,
            "unit": "median tps",
            "extra": "avg tps: 1126.573094269115, max tps: 1175.95381828107, count: 56071"
          },
          {
            "name": "Single Insert - Primary - tps",
            "value": 1288.23724239867,
            "unit": "median tps",
            "extra": "avg tps: 1284.160649662028, max tps: 1310.0625366554532, count: 56071"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 1099.6940752503117,
            "unit": "median tps",
            "extra": "avg tps: 1019.71951357955, max tps: 1635.263047155871, count: 56071"
          },
          {
            "name": "Top K - Primary - tps",
            "value": 5.036873803130912,
            "unit": "median tps",
            "extra": "avg tps: 5.132978545164297, max tps: 7.718174441501512, count: 56071"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "balavignesh449@gmail.com",
            "name": "S Bala Vignesh",
            "username": "SBALAVIGNESH123"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "d0039460372b22539b33262920b0194c694b7205",
          "message": "fix: use fixed-length updates in stressgres to avoid TOAST and expose FSM race (#5080)\n\nThe old UPDATE pattern in the logical-replication stressgres suite\nappended txid_current() to the message column every iteration, growing\nit past the TOAST threshold (~2KB). This caused the suite to hit the\nunrelated TOAST visibility race (#5076) before the FSM segment metadata\nrace (#4935) could surface.\n\nChanged to fixed-length updates that keep the first search term and\nappend a small txid-derived number, staying well under the TOAST\nthreshold. This way the suite can run long enough to exercise the FSM\npath.\n\nRelated: #5067 (FSM race fix), #5076 (TOAST bug)",
          "timestamp": "2026-05-14T14:57:31-04:00",
          "tree_id": "96511f40645bb3416046b0914f9758c60c159a20",
          "url": "https://github.com/paradedb/paradedb/commit/d0039460372b22539b33262920b0194c694b7205"
        },
        "date": 1778788300541,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 1156.505375251615,
            "unit": "median tps",
            "extra": "avg tps: 1158.1572377976042, max tps: 1209.2406219871893, count: 56058"
          },
          {
            "name": "Single Insert - Primary - tps",
            "value": 681.2833713747418,
            "unit": "median tps",
            "extra": "avg tps: 620.7129763963011, max tps: 1173.162898422843, count: 56058"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 1871.7120697374198,
            "unit": "median tps",
            "extra": "avg tps: 1848.9625527026215, max tps: 1995.9115416384254, count: 56058"
          },
          {
            "name": "Top K - Primary - tps",
            "value": 5.281499447805792,
            "unit": "median tps",
            "extra": "avg tps: 5.3010982914697, max tps: 6.44331512717943, count: 56058"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "balavignesh449@gmail.com",
            "name": "S Bala Vignesh",
            "username": "SBALAVIGNESH123"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "b597b183d25a561cf8c81d81decc7f07b7866e55",
          "message": "fix: hold header shared lock during LinkedItemList iteration to prevent FSM race (#4935) (#5067)\n\n## Problem\n\nLinkedItemList read-only iteration methods (list(), is_empty(),\nor_each(), lookup()/lookup_ex()) were releasing the header shared lock\nbefore iterating through the linked list blocks. This allowed\nAtomicGuard::commit() to:\n\n1. Swap the header pointer to a new list\n2. Immediately recycle old blocks to the FSM with\nReadNextFullTransactionId()\n3. The next transaction pops a recycled block via RBM_ZERO_AND_LOCK,\nzeroing it\n4. A concurrent reader still traversing the old list hits the\nzeroed/repurposed block\n5. **SIGSEGV** or SegmentMetaEntryHeader: UnexpectedEnd deserialization\nerror\n\nThis manifests as periodic crashes and durable index corruption under\nsustained write traffic, particularly on logical replication subscribers\nat high apply rates (~395 commits/sec). Correlates strongly with\nautovacuum events that trigger garbage_collect_index().\n\n## Root Cause\n\nThe race window exists because or_each(), list(), is_empty(), and\nlookup_ex() call get_start_blockno() which acquires a shared lock on the\nheader, reads start_blockno, then immediately releases the header lock\nwhen exchanging to the first data block. After that point, \u0007tomically()\ncan take an exclusive header lock and proceed with the swap+recycle\nwhile the reader is deep in the old list.\n\n**This was already a known pattern in the codebase** — emove_item() and\nupdate_item() in the same file both hold the header shared lock for\ntheir entire operation with this comment:\n\n\\\\\\\rust\n// Acquire and hold a shared lock on the header for the entire\noperation, preventing the\n// list from being swapped out from under us by atomically between our\nread locks and\n// our write locks.\nlet header_lock = self.bman.get_buffer(self.header_blockno);\n\\\\\\\n\nThe read-only methods simply weren't given the same treatment.\n\n## Fix\n\nHold a shared lock on the header for the entire duration of iteration in\nall 4 methods, matching the existing emove_item()/update_item() pattern:\n\n- **list()** — hold header_lock from start to end of iteration\n- **is_empty()** — same\n- **\for_each()** — same\n- **lookup_ex()** — conditionally: only when \blockno is None (top-level\ncall). When \blockno is Some, the caller ( emove_item/update_item)\nalready holds the header lock\n\nRead start_blockno directly from the already-held header_lock instead of\ncalling get_start_blockno(), avoiding a double shared-lock acquisition\non the same block (which would trigger a panic under the \block_tracker\ndebug feature).\n\n## Why This Is Safe\n\n- **No deadlock**: Header block is always locked first, content blocks\nin ascending order — consistent lock ordering\n- **No reader-reader blocking**: Multiple readers hold shared locks\nconcurrently (shared locks are compatible)\n- **Writer waits for readers**: \u0007tomically() takes an exclusive header\nlock, which blocks until all shared locks are released — correct\nserialization\n- **Minimal performance impact**: The header lock was already acquired;\nwe just hold it slightly longer\n\n## Verification\n\n- \rustfmt --check passes\n- Pattern matches the proven emove_item()/update_item() implementation\nin the same file\n- Full cargo check requires pgrx setup (PostgreSQL extension); the\nchange is limited to lock lifetime management with no new APIs\n\nCloses #4935",
          "timestamp": "2026-05-14T15:00:05-04:00",
          "tree_id": "46e474245958de09c21d0198195343d8a87fb72d",
          "url": "https://github.com/paradedb/paradedb/commit/b597b183d25a561cf8c81d81decc7f07b7866e55"
        },
        "date": 1778788384868,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 1179.3495739068862,
            "unit": "median tps",
            "extra": "avg tps: 1174.9958255493416, max tps: 1239.723773773084, count: 56518"
          },
          {
            "name": "Single Insert - Primary - tps",
            "value": 1293.2830523015432,
            "unit": "median tps",
            "extra": "avg tps: 1279.4627093568704, max tps: 1303.1755308893894, count: 56518"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 2004.3994765167413,
            "unit": "median tps",
            "extra": "avg tps: 1962.1577530120765, max tps: 2161.806004743998, count: 56518"
          },
          {
            "name": "Top K - Primary - tps",
            "value": 5.164673460156943,
            "unit": "median tps",
            "extra": "avg tps: 5.202080157699975, max tps: 7.10732149992259, count: 56518"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "652b0952a7f3be58ea4b9fa80a863426cf03185c",
          "message": "chore(stressgres): remove dormant suites not wired into CI (#5085)\n\n## Summary\n\n- Removes 8 Stressgres suites that have not been touched since\nStressgres was added (#3821), are not referenced by any CI workflow or\ndriver script, and use the legacy `[setup_primary]` schema rather than\nthe current `[[server]]` schema.\n- No CI workflow changes; the 6 suites currently exercised by\n`benchmark-pg_search-stressgres.yml` (`single-server`, `bulk-updates`,\n`wide-table`, `background-merge`, `logical-replication`,\n`logical-replication-merge`) and `vanilla-postgres.toml` (used by the\nantithesis singleton driver) are kept.\n\n### Removed\n- `large-inserts.toml`\n- `lr.toml` — predecessor of `logical-replication.toml`\n- `lr-graphable.toml`\n- `lr-large-inserts.toml`\n- `lr-no-pg_search.toml`\n- `many-updates.toml` — conceptually useful (MVCC correctness with\n`assert(count, expected)` under concurrency), but on legacy schema; if\nwe want this coverage in CI it should be a deliberate port, not a\nrevival of dead config\n- `read-write.toml` — contained placeholder `<password>` literal, not\nCI-runnable\n- `topk-crash.toml`\n\n### Why now\nCompanion to #5080 (which adjusts `logical-replication.toml` to expose\nthe FSM race instead of being masked by the TOAST bug already covered by\n`logical-replication-merge.toml`). Each CI suite should own a distinct\nrepro; dormant files muddy that mapping.\n\n## Test plan\n- [x] `benchmark-pg_search-stressgres` workflow still kicks off for all\n6 in-CI suites\n- [x] Antithesis `singleton_driver_vanilla-postgres.sh` still resolves\nits suite path",
          "timestamp": "2026-05-14T15:45:37-04:00",
          "tree_id": "090e72f7bbb9817642fb0c7495c2b12e0eed2fe7",
          "url": "https://github.com/paradedb/paradedb/commit/652b0952a7f3be58ea4b9fa80a863426cf03185c"
        },
        "date": 1778791238820,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 1068.834895524562,
            "unit": "median tps",
            "extra": "avg tps: 1069.7336471721344, max tps: 1096.7530080505119, count: 56388"
          },
          {
            "name": "Single Insert - Primary - tps",
            "value": 1257.8836226172853,
            "unit": "median tps",
            "extra": "avg tps: 1247.8776973063307, max tps: 1269.7383455524698, count: 56388"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 1179.3071023273897,
            "unit": "median tps",
            "extra": "avg tps: 1081.0902400493699, max tps: 1555.887738543333, count: 56388"
          },
          {
            "name": "Top K - Primary - tps",
            "value": 5.402638853836446,
            "unit": "median tps",
            "extra": "avg tps: 5.402829388058415, max tps: 6.846793619503152, count: 56388"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ming.ying.nyc@gmail.com",
            "name": "Ming",
            "username": "rebasedming"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "21bdc9a753582039cc25a38cb07111026c0fd378",
          "message": "feat: Crash recovery via WAL (#4901)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nPorts WAL integration over to community, which gives `pg_search` crash\nrecovery.\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2026-05-14T14:59:53-07:00",
          "tree_id": "7580b5f2bfcc1a95ed1b62c4628f2ec6129b7c00",
          "url": "https://github.com/paradedb/paradedb/commit/21bdc9a753582039cc25a38cb07111026c0fd378"
        },
        "date": 1778799283029,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 1051.7720831217446,
            "unit": "median tps",
            "extra": "avg tps: 1047.4599734408482, max tps: 1112.2508285713216, count: 56394"
          },
          {
            "name": "Single Insert - Primary - tps",
            "value": 1277.699305970814,
            "unit": "median tps",
            "extra": "avg tps: 1252.539524535723, max tps: 1288.5424100502823, count: 56394"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 1667.8994797170885,
            "unit": "median tps",
            "extra": "avg tps: 1637.061481683245, max tps: 1860.8562079043645, count: 56394"
          },
          {
            "name": "Top K - Primary - tps",
            "value": 5.270532282763347,
            "unit": "median tps",
            "extra": "avg tps: 5.3221353426732945, max tps: 7.381559464422096, count: 56394"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "d99908a1f58307567dd34698345b3fc836be6135",
          "message": "feat(antithesis): add singleton drivers for logical-replication suites (#5087)\n\n## Summary\n\nAdds OSS Antithesis singleton drivers for the two CI\n`logical-replication` suites that previously had no singleton\n(`single-server`, `bulk-updates`, `wide-table`, `background-merge`, and\n`vanilla-postgres` already had one).\n\nMirrors the enterprise pattern used for `physical-logical-replication`:\n- A **vanilla Postgres 18** publisher pod (with `wal_level=logical`)\nthat lives outside the CNPG cluster, reflecting real-world\nlogical-replication topologies where the upstream primary is not under\nour control.\n- Subscriber points at `paradedb-rw` (the CNPG primary, which has\n`pg_search`).\n\n## Changes\n- `docker/manifests/antithesis-paradedb.yaml` — add\n`logical-replication-publisher` Service + Deployment (vanilla Postgres\n18 with `wal_level=logical`), reusing the existing `paradedb-superuser`\nsecret.\n- `stressgres/suites/logical-replication.toml`,\n`stressgres/suites/logical-replication-merge.toml` — drop `CREATE\nEXTENSION pg_search` from the **Publisher** setup. Only the Subscriber\nuses `pg_search`; the line was cosmetic and incompatible with a vanilla\nPostgres publisher (the line in the Subscriber setup is unchanged).\n-\n`stressgres/suites/antithesis/singleton_driver_logical-replication.sh`,\n`singleton_driver_logical-replication-merge.sh` — new drivers that\nperform per-block `sed -z` rewrites of the `[server.style.Automatic]`\nblocks into `[server.style.With]` connection strings (Publisher →\n`logical-replication-publisher:5432`, Subscriber → `paradedb-rw:5432`).\n- `.github/workflows/antithesis-trigger-test-run.yml` — add\n`logical-replication-publisher` to\n`container_faults_stop_exclusion_patterns` and\n`container_faults_kill_exclusion_patterns`, matching enterprise. Network\nfaults to/from the publisher are intentionally still injected.\n\n## Why\nWithout these, the FSM race repro in `logical-replication-merge.toml`\n(issue #4935, fixed by #5067) and the broader logical-replication\ncoverage in `logical-replication.toml` were running in\n`benchmark-pg_search-stressgres` but had no Antithesis fault-injection\nequivalent — that's the half of the matrix where the bugs originally\nsurfaced.\n\n## Test plan\n- [x] Antithesis trigger workflow picks up both new singleton drivers\nfrom `/opt/antithesis/test/v1/quickstart/`\n- [x] Publisher pod (`logical-replication-publisher`) starts with\n`wal_level=logical` and is reachable from the stressgres-runner pod\n- [x] Subscriber's `CREATE SUBSCRIPTION ... CONNECTION\n'@Publisher_CONNSTR@'` resolves to the publisher pod after the `sed`\nrewrite\n- [x] `logical-replication-merge.toml` still reproduces the FSM race\nwhen run against a build without #5067\n- [x] `benchmark-pg_search-stressgres` (local Stressgres, not\nAntithesis) still runs both suites unchanged",
          "timestamp": "2026-05-14T18:54:16-04:00",
          "tree_id": "8c7a6dab334db43248e88e7a4ebc75fd5840e446",
          "url": "https://github.com/paradedb/paradedb/commit/d99908a1f58307567dd34698345b3fc836be6135"
        },
        "date": 1778802452248,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 1060.5713278807746,
            "unit": "median tps",
            "extra": "avg tps: 1047.4184681675918, max tps: 1104.9105241917487, count: 55989"
          },
          {
            "name": "Single Insert - Primary - tps",
            "value": 1181.8448442712436,
            "unit": "median tps",
            "extra": "avg tps: 1153.4982921309727, max tps: 1212.3015745608382, count: 55989"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 1729.3680364807724,
            "unit": "median tps",
            "extra": "avg tps: 1660.274704893197, max tps: 1921.352119950438, count: 55989"
          },
          {
            "name": "Top K - Primary - tps",
            "value": 5.520103702040576,
            "unit": "median tps",
            "extra": "avg tps: 5.551652021365292, max tps: 7.053362169170688, count: 55989"
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
        "date": 1778654431479,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Background Merger - Primary - background_merging",
            "value": 0,
            "unit": "median background_merging",
            "extra": "avg background_merging: 0.052788156753023674, max background_merging: 2.0, count: 56471"
          },
          {
            "name": "Background Merger - Primary - cpu",
            "value": 4.669261,
            "unit": "median cpu",
            "extra": "avg cpu: 4.8005036085960695, max cpu: 9.67742, count: 56471"
          },
          {
            "name": "Background Merger - Primary - mem",
            "value": 26.046875,
            "unit": "median mem",
            "extra": "avg mem: 26.092163593924315, max mem: 26.16796875, count: 56471"
          },
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 4.669261,
            "unit": "median cpu",
            "extra": "avg cpu: 5.020428112559018, max cpu: 18.879055, count: 56471"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 184.32421875,
            "unit": "median mem",
            "extra": "avg mem: 178.21020689823095, max mem: 185.09765625, count: 56471"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 60176,
            "unit": "median block_count",
            "extra": "avg block_count: 59951.672699261566, max block_count: 60176.0, count: 56471"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 45,
            "unit": "median segment_count",
            "extra": "avg segment_count: 42.17377060792265, max segment_count: 57.0, count: 56471"
          },
          {
            "name": "Single Insert - Primary - cpu",
            "value": 4.669261,
            "unit": "median cpu",
            "extra": "avg cpu: 4.660414724090775, max cpu: 28.318584, count: 56471"
          },
          {
            "name": "Single Insert - Primary - mem",
            "value": 149.9453125,
            "unit": "median mem",
            "extra": "avg mem: 134.34749597691734, max mem: 165.01953125, count: 56471"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.669261,
            "unit": "median cpu",
            "extra": "avg cpu: 5.2011465786333275, max cpu: 32.684826, count: 56471"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 200.39453125,
            "unit": "median mem",
            "extra": "avg mem: 198.97633222417258, max mem: 217.12109375, count: 56471"
          },
          {
            "name": "Top K - Primary - cpu",
            "value": 23.391813,
            "unit": "median cpu",
            "extra": "avg cpu: 23.947994899873812, max cpu: 33.802814, count: 56471"
          },
          {
            "name": "Top K - Primary - mem",
            "value": 164.07421875,
            "unit": "median mem",
            "extra": "avg mem: 182.8561574331294, max mem: 222.49609375, count: 56471"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "59696464+saadtajwar@users.noreply.github.com",
            "name": "Saad Tajwar",
            "username": "saadtajwar"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "bc20c3dd4b36a13dbcc74d03499f966a4dc93fe6",
          "message": "refactor: Deduplicate deferred materialization request partitioning between visibility and lookup (#4903)\n\n# Ticket(s) Closed\n- Closes https://github.com/paradedb/paradedb/issues/4568\n\n## What\nDeduplicated the segment-grouping/materialization loop shared between\n`materialize_deferred_ctid()` in `visibility_filter.rs` and\n`materialize_deferred_column()` in `tantivy_lookup_exec.rs`.\n\n## Why\nBoth functions implemented the same pattern of partitioning packed doc\naddresses by segment ordinal, batch-reading via `FFHelper`, and writing\nresults back in row order. This duplication made the two paths harder to\nkeep consistent and made future optimization work more tedious.\n\n## How\n\n### Shared helper in `fast_fields_helper.rs`\n- Added `for_each_segment`: partitions an iterator of `(row_index,\npacked_doc_address)` pairs into per-segment buckets and invokes a\ncaller-supplied closure once per non-empty segment, in segment-ordinal\norder. Backed by a `Vec<Vec<(usize, DocId)>>` indexed by segment ordinal\n(dense in practice; cheaper than a hash map for typical segment counts).\n- Added `FFHelper::num_segments()` so callers can size the bucket vector\nwithout reaching into private fields.\n\n### `materialize_deferred_ctid` (visibility_filter.rs)\n- Replaced the manual sort + partition + per-segment slice loop with a\nsingle `for_each_segment` call.\n- Kept `DeferredCtidMaterializationState` for buffer reuse across calls,\nbut removed its now-unused `requests` field; the per-segment\npartitioning lives inside `for_each_segment`.\n- Removed the TODO comment that flagged this duplication.\n\n### `materialize_deferred_column` (tantivy_lookup_exec.rs)\nFunction body shrank from ~160 lines to ~50 by extracting three\nsingle-responsibility helpers:\n- `resolve_doc_addresses_to_term_ords` — resolves State 0 (packed doc\naddresses) into per-segment `(row_index, Option<TermOrdinal>)` pairs via\n`for_each_segment`.\n- `extract_term_ords` — parses State 1 (pre-resolved `(segment_ord,\nterm_ord)` pairs from the dense union's `StructArray` child) into the\nsame per-segment shape.\n- `decode_term_ordinals` — takes the merged per-segment ordinals and\nperforms the bulk dictionary lookup once per segment, recording\npositions for the final `interleave`.\n\nState 0 and State 1 are now merged into a single `Vec<Vec<(row_index,\nOption<TermOrdinal>)>>` indexed by segment ordinal, then decoded in one\npass — previously each state was iterated and decoded separately,\nproducing two `segment_arrays` entries per segment touched by both. The\nfinal interleaved output is identical.\n\nReplaced the `(ff_index: usize, is_bytes: bool)` parameter pair with a\n`DeferredColumnKind { Text { ff_index }, Bytes { ff_index } }` enum to\nmake the `is_bytes && wrong-ff-type` mismatch unrepresentable.\n\n## Tests\nNo new tests; behavior is unchanged. Existing coverage exercises both\npaths end-to-end:\n- `pg_search/tests/pg_regress/sql/join_deferred_visibility.sql` —\n`materialize_deferred_ctid`.\n- `pg_search/tests/pg_regress/sql/segmented_topk.sql` plus joinscan\ntests — `materialize_deferred_column` (the segmented top-K rule is what\nproduces State 1 rows below `TantivyLookupExec`).\n\n---------\n\nCo-authored-by: Mithun Chicklore Yogendra <mithun.cy@gmail.com>",
          "timestamp": "2026-05-13T10:07:59-07:00",
          "tree_id": "cc9e05e63ed6052c202d00901357d2c5026923d0",
          "url": "https://github.com/paradedb/paradedb/commit/bc20c3dd4b36a13dbcc74d03499f966a4dc93fe6"
        },
        "date": 1778695298839,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Background Merger - Primary - background_merging",
            "value": 0,
            "unit": "median background_merging",
            "extra": "avg background_merging: 0.06905320813771518, max background_merging: 2.0, count: 56232"
          },
          {
            "name": "Background Merger - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 4.786899678871244, max cpu: 9.726444, count: 56232"
          },
          {
            "name": "Background Merger - Primary - mem",
            "value": 26.1484375,
            "unit": "median mem",
            "extra": "avg mem: 26.14339164921664, max mem: 26.15234375, count: 56232"
          },
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 4.988528956266303, max cpu: 27.988338, count: 56232"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 192.3359375,
            "unit": "median mem",
            "extra": "avg mem: 188.35520547130636, max mem: 192.49609375, count: 56232"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 51421,
            "unit": "median block_count",
            "extra": "avg block_count: 51287.46674491393, max block_count: 51421.0, count: 56232"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 45,
            "unit": "median segment_count",
            "extra": "avg segment_count: 42.35165030587566, max segment_count: 56.0, count: 56232"
          },
          {
            "name": "Single Insert - Primary - cpu",
            "value": 4.669261,
            "unit": "median cpu",
            "extra": "avg cpu: 4.934124043265518, max cpu: 28.125, count: 56232"
          },
          {
            "name": "Single Insert - Primary - mem",
            "value": 148.96484375,
            "unit": "median mem",
            "extra": "avg mem: 130.67544440609885, max mem: 164.06640625, count: 56232"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 4.752104071960965, max cpu: 27.77242, count: 56232"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 189.32421875,
            "unit": "median mem",
            "extra": "avg mem: 183.2781981900875, max mem: 190.625, count: 56232"
          },
          {
            "name": "Top K - Primary - cpu",
            "value": 23.506365,
            "unit": "median cpu",
            "extra": "avg cpu: 23.938728191326955, max cpu: 33.83686, count: 56232"
          },
          {
            "name": "Top K - Primary - mem",
            "value": 164.140625,
            "unit": "median mem",
            "extra": "avg mem: 181.7253421789417, max mem: 222.69140625, count: 56232"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "stuhood@paradedb.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "713eaca82145388378043ddb2c750c41e9c632d9",
          "message": "chore: Revert #4905 (#5066)\n\nRevert #4905 due to performance regressions.",
          "timestamp": "2026-05-13T13:34:27-07:00",
          "tree_id": "2894402ba71bbb0b86d5990862fb2adba4fcbf49",
          "url": "https://github.com/paradedb/paradedb/commit/713eaca82145388378043ddb2c750c41e9c632d9"
        },
        "date": 1778707674823,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Background Merger - Primary - background_merging",
            "value": 0,
            "unit": "median background_merging",
            "extra": "avg background_merging: 0.0528771108273024, max background_merging: 2.0, count: 56376"
          },
          {
            "name": "Background Merger - Primary - cpu",
            "value": 4.669261,
            "unit": "median cpu",
            "extra": "avg cpu: 4.779819148212733, max cpu: 9.533267, count: 56376"
          },
          {
            "name": "Background Merger - Primary - mem",
            "value": 25.7890625,
            "unit": "median mem",
            "extra": "avg mem: 25.785211057786114, max mem: 25.79296875, count: 56376"
          },
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 4.669261,
            "unit": "median cpu",
            "extra": "avg cpu: 5.0301858359607134, max cpu: 23.529411, count: 56376"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 191.58203125,
            "unit": "median mem",
            "extra": "avg mem: 188.05849154837608, max mem: 191.80859375, count: 56376"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 59226,
            "unit": "median block_count",
            "extra": "avg block_count: 59008.140769121615, max block_count: 59226.0, count: 56376"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 45,
            "unit": "median segment_count",
            "extra": "avg segment_count: 42.009472115793955, max segment_count: 58.0, count: 56376"
          },
          {
            "name": "Single Insert - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 4.717805861698582, max cpu: 27.934044, count: 56376"
          },
          {
            "name": "Single Insert - Primary - mem",
            "value": 159.3515625,
            "unit": "median mem",
            "extra": "avg mem: 142.47594312351444, max mem: 171.12890625, count: 56376"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 5.115413300105314, max cpu: 32.55814, count: 56376"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 216.625,
            "unit": "median mem",
            "extra": "avg mem: 214.81029223705565, max mem: 222.24609375, count: 56376"
          },
          {
            "name": "Top K - Primary - cpu",
            "value": 23.391813,
            "unit": "median cpu",
            "extra": "avg cpu: 23.97221247657884, max cpu: 33.466137, count: 56376"
          },
          {
            "name": "Top K - Primary - mem",
            "value": 164.15625,
            "unit": "median mem",
            "extra": "avg mem: 182.5581407369714, max mem: 222.51171875, count: 56376"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "035982deb7c1e630e0d8a13e4ca1910b979b08f6",
          "message": "ci: add logical-replication-merge stressgres suite to cover FSM race (#4935) (#5072)\n\nRe-opened from #5068 (originally from a fork, where CI could not access\nworkflow secrets — see\nhttps://github.com/paradedb/paradedb/actions/runs/25830120431/job/75892835684?pr=5068).\nSame intent, from a branch in this repo, with the FSM-race suite landing\nas its own file rather than folded into `logical-replication.toml`.\n\n## Changes\n\n- `stressgres/suites/logical-replication-merge.toml` (new): standalone\nsuite that reliably reproduces the FSM race from #4935 (fixed in #5067).\nLogical-replication subscriber with aggressive autovacuum (`naptime=1s`,\n`threshold=50`), small `layer_sizes = '10kb, 100kb, 1mb, 100mb'`,\nmultiple concurrent BM25 readers, and sustained UPDATE/INSERT/DELETE\ntraffic on the publisher. The key difference from\n`logical-replication.toml` is the writer: `message = message || ' ' ||\ntxid_current()` grows each row's terms unbounded, generating ~10× more\nmerge/GC pressure and reliably opening the race window — folding the\nsame churn into `logical-replication.toml` (which strips-then-appends,\nkeeping row size constant) did not reproduce the bug.\n- `.github/workflows/benchmark-pg_search-stressgres.yml`:\n- Comment out single-server, bulk-updates, wide-table, and\nbackground-merge so CI focuses on the two replication suites while we\niterate. To be re-enabled before final merge.\n- Run `logical-replication-merge.toml` **before**\n`logical-replication.toml`.\n\n`stressgres/suites/logical-replication.toml` is unchanged from `main`.\n\n## Expected behavior\n\n- Without #5067: SIGSEGV or `SegmentMetaEntryHeader: UnexpectedEnd`\nwithin minutes.\n- With #5067: runs the full duration without errors.\n\n## Follow-ups\n\n- Re-enable the four commented-out suites before final merge.\n- Antithesis wiring for this suite belongs in `paradedb-enterprise` next\nto the existing `physical-logical-replication` driver, since the OSS\nAntithesis manifest only stands up a single paradedb cluster.\n\nRef: #4935\nRelated: #5067\nSupersedes: #5068",
          "timestamp": "2026-05-13T19:51:05-04:00",
          "tree_id": "c71af69d7df60d54d1631876f7e3c7af0782c3e3",
          "url": "https://github.com/paradedb/paradedb/commit/035982deb7c1e630e0d8a13e4ca1910b979b08f6"
        },
        "date": 1778719462435,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Background Merger - Primary - background_merging",
            "value": 0,
            "unit": "median background_merging",
            "extra": "avg background_merging: 0.07210225862496898, max background_merging: 2.0, count: 56406"
          },
          {
            "name": "Background Merger - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.7630970407991216, max cpu: 9.687184, count: 56406"
          },
          {
            "name": "Background Merger - Primary - mem",
            "value": 29.25,
            "unit": "median mem",
            "extra": "avg mem: 29.301639148539163, max mem: 29.37109375, count: 56406"
          },
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 4.669261,
            "unit": "median cpu",
            "extra": "avg cpu: 4.955518908689095, max cpu: 23.692005, count: 56406"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 186.26953125,
            "unit": "median mem",
            "extra": "avg mem: 179.73536981050333, max mem: 187.8984375, count: 56406"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 51426,
            "unit": "median block_count",
            "extra": "avg block_count: 51292.21586356062, max block_count: 51426.0, count: 56406"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 45,
            "unit": "median segment_count",
            "extra": "avg segment_count: 42.741055916037304, max segment_count: 56.0, count: 56406"
          },
          {
            "name": "Single Insert - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.602344239256405, max cpu: 28.543112, count: 56406"
          },
          {
            "name": "Single Insert - Primary - mem",
            "value": 161.86328125,
            "unit": "median mem",
            "extra": "avg mem: 144.09573629689217, max mem: 169.91796875, count: 56406"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.79178050083662, max cpu: 28.290766, count: 56406"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 190.83203125,
            "unit": "median mem",
            "extra": "avg mem: 186.11946638821667, max mem: 190.9765625, count: 56406"
          },
          {
            "name": "Top K - Primary - cpu",
            "value": 23.414635,
            "unit": "median cpu",
            "extra": "avg cpu: 23.812196613590505, max cpu: 33.300297, count: 56406"
          },
          {
            "name": "Top K - Primary - mem",
            "value": 163.77734375,
            "unit": "median mem",
            "extra": "avg mem: 182.3445331752163, max mem: 222.1328125, count: 56406"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "36623265+daniel3303@users.noreply.github.com",
            "name": "Daniel Oliveira",
            "username": "daniel3303"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "b9c06c5f75ca94cf815eb57c71c46180e84b8038",
          "message": "fix(mlt): quote key_field identifier in internal SPI lookup (#5078)\n\n## Summary\n\n- `pdb.more_like_this(key_value)` raises `ERROR: column \"id\" does not\nexist` whenever the index's `key_field` column is a mixed-case\nPostgreSQL identifier (e.g. `\"Id\"`, `\"DocumentId\"`). Direct `@@@`-on-LHS\nsearches (`\"Content\" @@@ 'foo'`) are unaffected because they don't go\nthrough the internal SPI lookup. Repro in #5065.\n- Root cause: `pg_search/src/query/more_like_this.rs:152-157` builds the\nSPI `SELECT * FROM <ns>.<rel> WHERE <key_field> = $1` with `<ns>` and\n`<rel>` already routed through `pgrx::spi::quote_identifier`, but\ninterpolates `<key_field>` verbatim via `Display`. PostgreSQL folds the\nunquoted reference to lowercase, so a column named `\"Id\"` is looked up\nas `id` and the SPI call fails before MLT ever runs.\n- Fix: send the key field through\n`pgrx::spi::quote_identifier(key_field_name.root())`, matching how the\nnamespace and relation names are already quoted on the lines immediately\nabove. `.root()` strips the JSON sub-path (`key_field` is always a\ntop-level column).\n\n## Scope\n\nThe linked issue also lists JSON `term` filters (`@@@\n'{\"term\":{\"field\":\"Category\",…}}'::jsonb`) as failing on mixed-case\ncolumns. That path does **not** go through SPI — `term()` in\n`pg_search/src/query/pdb_query.rs:792` resolves the field via\n`schema.search_field(field.root())`, a pure Tantivy schema lookup — so\nit isn't fixed here and I couldn't find a corresponding\nunquoted-identifier hazard. If it reproduces on `0.23.x` it's a separate\nbug; tracking it on its own issue is cleaner than bundling a speculative\nfix.\n\n## Test plan\n\n- [x] `cargo test -p tests --test mlt --\nmlt_mixed_case_key_field_issue5065` — new regression test: `\"Id\"` /\n`\"Content\"` table, `key_field='Id'`, asserts `pdb.more_like_this(1)`\nreturns rows. Fails on `main` with `column \"id\" does not exist`, passes\nwith this change.\n- [x] `cargo test -p tests --test mlt` — existing\n`mlt_enables_scoring_issue1747`, `mlt_datetime_key`,\n`mlt_scoring_nested` still pass.\n- [x] `cargo pgrx regress -p pg_search --auto -- pg18 more_like_this` —\ngolden output unchanged (`quote_identifier(\"id\")` is a no-op for\nalready-lowercase identifiers).\n- [x] Manual repro from #5065 (`CREATE TABLE items (\"Id\" int primary\nkey, \"Content\" text); … pdb.more_like_this(1)`) returns rows instead of\nerroring.\n\nCloses #5065.",
          "timestamp": "2026-05-14T14:45:18-04:00",
          "tree_id": "812b4a66ffce7bc074cb919986f1de3b6474813f",
          "url": "https://github.com/paradedb/paradedb/commit/b9c06c5f75ca94cf815eb57c71c46180e84b8038"
        },
        "date": 1778787531140,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Background Merger - Primary - background_merging",
            "value": 0,
            "unit": "median background_merging",
            "extra": "avg background_merging: 0.053878118813646986, max background_merging: 2.0, count: 56071"
          },
          {
            "name": "Background Merger - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.683252894036034, max cpu: 9.504951, count: 56071"
          },
          {
            "name": "Background Merger - Primary - mem",
            "value": 25.50390625,
            "unit": "median mem",
            "extra": "avg mem: 25.49411725201084, max mem: 25.5078125, count: 56071"
          },
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 4.940112820884421, max cpu: 9.667674, count: 56071"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 170.734375,
            "unit": "median mem",
            "extra": "avg mem: 169.3182356131066, max mem: 170.9375, count: 56071"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 55004,
            "unit": "median block_count",
            "extra": "avg block_count: 54849.315439353675, max block_count: 55004.0, count: 56071"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 45,
            "unit": "median segment_count",
            "extra": "avg segment_count: 41.87968825239429, max segment_count: 56.0, count: 56071"
          },
          {
            "name": "Single Insert - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.654189306152838, max cpu: 23.233301, count: 56071"
          },
          {
            "name": "Single Insert - Primary - mem",
            "value": 148.58984375,
            "unit": "median mem",
            "extra": "avg mem: 131.0393743256318, max mem: 161.8359375, count: 56071"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 5.208760637922064, max cpu: 32.589718, count: 56071"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 200.3046875,
            "unit": "median mem",
            "extra": "avg mem: 198.44344266086748, max mem: 229.61328125, count: 56071"
          },
          {
            "name": "Top K - Primary - cpu",
            "value": 23.346306,
            "unit": "median cpu",
            "extra": "avg cpu: 23.85182955772567, max cpu: 33.667336, count: 56071"
          },
          {
            "name": "Top K - Primary - mem",
            "value": 164.10546875,
            "unit": "median mem",
            "extra": "avg mem: 182.4257484372492, max mem: 222.53125, count: 56071"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "balavignesh449@gmail.com",
            "name": "S Bala Vignesh",
            "username": "SBALAVIGNESH123"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "d0039460372b22539b33262920b0194c694b7205",
          "message": "fix: use fixed-length updates in stressgres to avoid TOAST and expose FSM race (#5080)\n\nThe old UPDATE pattern in the logical-replication stressgres suite\nappended txid_current() to the message column every iteration, growing\nit past the TOAST threshold (~2KB). This caused the suite to hit the\nunrelated TOAST visibility race (#5076) before the FSM segment metadata\nrace (#4935) could surface.\n\nChanged to fixed-length updates that keep the first search term and\nappend a small txid-derived number, staying well under the TOAST\nthreshold. This way the suite can run long enough to exercise the FSM\npath.\n\nRelated: #5067 (FSM race fix), #5076 (TOAST bug)",
          "timestamp": "2026-05-14T14:57:31-04:00",
          "tree_id": "96511f40645bb3416046b0914f9758c60c159a20",
          "url": "https://github.com/paradedb/paradedb/commit/d0039460372b22539b33262920b0194c694b7205"
        },
        "date": 1778788335465,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Background Merger - Primary - background_merging",
            "value": 0,
            "unit": "median background_merging",
            "extra": "avg background_merging: 0.0546398373113561, max background_merging: 2.0, count: 56058"
          },
          {
            "name": "Background Merger - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.737488647469511, max cpu: 9.648242, count: 56058"
          },
          {
            "name": "Background Merger - Primary - mem",
            "value": 25.875,
            "unit": "median mem",
            "extra": "avg mem: 25.870038969125726, max mem: 25.8828125, count: 56058"
          },
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 4.99093139134679, max cpu: 15.177865, count: 56058"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 185.23828125,
            "unit": "median mem",
            "extra": "avg mem: 180.16594294636803, max mem: 191.078125, count: 56058"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 61379,
            "unit": "median block_count",
            "extra": "avg block_count: 61140.793035784365, max block_count: 61379.0, count: 56058"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 46,
            "unit": "median segment_count",
            "extra": "avg segment_count: 42.70444896357344, max segment_count: 57.0, count: 56058"
          },
          {
            "name": "Single Insert - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 5.05215652641243, max cpu: 32.526623, count: 56058"
          },
          {
            "name": "Single Insert - Primary - mem",
            "value": 202.6953125,
            "unit": "median mem",
            "extra": "avg mem: 201.35288116771915, max mem: 214.01953125, count: 56058"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 4.697041251363043, max cpu: 27.906979, count: 56058"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 187.19140625,
            "unit": "median mem",
            "extra": "avg mem: 175.6515895088569, max mem: 192.109375, count: 56058"
          },
          {
            "name": "Top K - Primary - cpu",
            "value": 23.369036,
            "unit": "median cpu",
            "extra": "avg cpu: 23.806663073397168, max cpu: 33.267326, count: 56058"
          },
          {
            "name": "Top K - Primary - mem",
            "value": 164.1953125,
            "unit": "median mem",
            "extra": "avg mem: 182.3082445023458, max mem: 222.58203125, count: 56058"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "balavignesh449@gmail.com",
            "name": "S Bala Vignesh",
            "username": "SBALAVIGNESH123"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "b597b183d25a561cf8c81d81decc7f07b7866e55",
          "message": "fix: hold header shared lock during LinkedItemList iteration to prevent FSM race (#4935) (#5067)\n\n## Problem\n\nLinkedItemList read-only iteration methods (list(), is_empty(),\nor_each(), lookup()/lookup_ex()) were releasing the header shared lock\nbefore iterating through the linked list blocks. This allowed\nAtomicGuard::commit() to:\n\n1. Swap the header pointer to a new list\n2. Immediately recycle old blocks to the FSM with\nReadNextFullTransactionId()\n3. The next transaction pops a recycled block via RBM_ZERO_AND_LOCK,\nzeroing it\n4. A concurrent reader still traversing the old list hits the\nzeroed/repurposed block\n5. **SIGSEGV** or SegmentMetaEntryHeader: UnexpectedEnd deserialization\nerror\n\nThis manifests as periodic crashes and durable index corruption under\nsustained write traffic, particularly on logical replication subscribers\nat high apply rates (~395 commits/sec). Correlates strongly with\nautovacuum events that trigger garbage_collect_index().\n\n## Root Cause\n\nThe race window exists because or_each(), list(), is_empty(), and\nlookup_ex() call get_start_blockno() which acquires a shared lock on the\nheader, reads start_blockno, then immediately releases the header lock\nwhen exchanging to the first data block. After that point, \u0007tomically()\ncan take an exclusive header lock and proceed with the swap+recycle\nwhile the reader is deep in the old list.\n\n**This was already a known pattern in the codebase** — emove_item() and\nupdate_item() in the same file both hold the header shared lock for\ntheir entire operation with this comment:\n\n\\\\\\\rust\n// Acquire and hold a shared lock on the header for the entire\noperation, preventing the\n// list from being swapped out from under us by atomically between our\nread locks and\n// our write locks.\nlet header_lock = self.bman.get_buffer(self.header_blockno);\n\\\\\\\n\nThe read-only methods simply weren't given the same treatment.\n\n## Fix\n\nHold a shared lock on the header for the entire duration of iteration in\nall 4 methods, matching the existing emove_item()/update_item() pattern:\n\n- **list()** — hold header_lock from start to end of iteration\n- **is_empty()** — same\n- **\for_each()** — same\n- **lookup_ex()** — conditionally: only when \blockno is None (top-level\ncall). When \blockno is Some, the caller ( emove_item/update_item)\nalready holds the header lock\n\nRead start_blockno directly from the already-held header_lock instead of\ncalling get_start_blockno(), avoiding a double shared-lock acquisition\non the same block (which would trigger a panic under the \block_tracker\ndebug feature).\n\n## Why This Is Safe\n\n- **No deadlock**: Header block is always locked first, content blocks\nin ascending order — consistent lock ordering\n- **No reader-reader blocking**: Multiple readers hold shared locks\nconcurrently (shared locks are compatible)\n- **Writer waits for readers**: \u0007tomically() takes an exclusive header\nlock, which blocks until all shared locks are released — correct\nserialization\n- **Minimal performance impact**: The header lock was already acquired;\nwe just hold it slightly longer\n\n## Verification\n\n- \rustfmt --check passes\n- Pattern matches the proven emove_item()/update_item() implementation\nin the same file\n- Full cargo check requires pgrx setup (PostgreSQL extension); the\nchange is limited to lock lifetime management with no new APIs\n\nCloses #4935",
          "timestamp": "2026-05-14T15:00:05-04:00",
          "tree_id": "46e474245958de09c21d0198195343d8a87fb72d",
          "url": "https://github.com/paradedb/paradedb/commit/b597b183d25a561cf8c81d81decc7f07b7866e55"
        },
        "date": 1778788419074,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Background Merger - Primary - background_merging",
            "value": 0,
            "unit": "median background_merging",
            "extra": "avg background_merging: 0.07211861707774514, max background_merging: 2.0, count: 56518"
          },
          {
            "name": "Background Merger - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.798056894191594, max cpu: 9.667674, count: 56518"
          },
          {
            "name": "Background Merger - Primary - mem",
            "value": 26.02734375,
            "unit": "median mem",
            "extra": "avg mem: 26.06634245671733, max mem: 26.1484375, count: 56518"
          },
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 5.032156503668478, max cpu: 28.042841, count: 56518"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 186.1796875,
            "unit": "median mem",
            "extra": "avg mem: 184.22165214511307, max mem: 191.6171875, count: 56518"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 51427,
            "unit": "median block_count",
            "extra": "avg block_count: 51293.696539155666, max block_count: 51427.0, count: 56518"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 46,
            "unit": "median segment_count",
            "extra": "avg segment_count: 43.10957571039315, max segment_count: 56.0, count: 56518"
          },
          {
            "name": "Single Insert - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.535827234334311, max cpu: 32.65306, count: 56518"
          },
          {
            "name": "Single Insert - Primary - mem",
            "value": 142.91796875,
            "unit": "median mem",
            "extra": "avg mem: 130.84746722006705, max mem: 157.5390625, count: 56518"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 4.767117423308078, max cpu: 9.696969, count: 56518"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 170.52734375,
            "unit": "median mem",
            "extra": "avg mem: 166.22647354880303, max mem: 170.67578125, count: 56518"
          },
          {
            "name": "Top K - Primary - cpu",
            "value": 23.369036,
            "unit": "median cpu",
            "extra": "avg cpu: 23.880596111561022, max cpu: 33.267326, count: 56518"
          },
          {
            "name": "Top K - Primary - mem",
            "value": 164.140625,
            "unit": "median mem",
            "extra": "avg mem: 183.18800882572808, max mem: 222.67578125, count: 56518"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "652b0952a7f3be58ea4b9fa80a863426cf03185c",
          "message": "chore(stressgres): remove dormant suites not wired into CI (#5085)\n\n## Summary\n\n- Removes 8 Stressgres suites that have not been touched since\nStressgres was added (#3821), are not referenced by any CI workflow or\ndriver script, and use the legacy `[setup_primary]` schema rather than\nthe current `[[server]]` schema.\n- No CI workflow changes; the 6 suites currently exercised by\n`benchmark-pg_search-stressgres.yml` (`single-server`, `bulk-updates`,\n`wide-table`, `background-merge`, `logical-replication`,\n`logical-replication-merge`) and `vanilla-postgres.toml` (used by the\nantithesis singleton driver) are kept.\n\n### Removed\n- `large-inserts.toml`\n- `lr.toml` — predecessor of `logical-replication.toml`\n- `lr-graphable.toml`\n- `lr-large-inserts.toml`\n- `lr-no-pg_search.toml`\n- `many-updates.toml` — conceptually useful (MVCC correctness with\n`assert(count, expected)` under concurrency), but on legacy schema; if\nwe want this coverage in CI it should be a deliberate port, not a\nrevival of dead config\n- `read-write.toml` — contained placeholder `<password>` literal, not\nCI-runnable\n- `topk-crash.toml`\n\n### Why now\nCompanion to #5080 (which adjusts `logical-replication.toml` to expose\nthe FSM race instead of being masked by the TOAST bug already covered by\n`logical-replication-merge.toml`). Each CI suite should own a distinct\nrepro; dormant files muddy that mapping.\n\n## Test plan\n- [x] `benchmark-pg_search-stressgres` workflow still kicks off for all\n6 in-CI suites\n- [x] Antithesis `singleton_driver_vanilla-postgres.sh` still resolves\nits suite path",
          "timestamp": "2026-05-14T15:45:37-04:00",
          "tree_id": "090e72f7bbb9817642fb0c7495c2b12e0eed2fe7",
          "url": "https://github.com/paradedb/paradedb/commit/652b0952a7f3be58ea4b9fa80a863426cf03185c"
        },
        "date": 1778791271688,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Background Merger - Primary - background_merging",
            "value": 0,
            "unit": "median background_merging",
            "extra": "avg background_merging: 0.05417819394197347, max background_merging: 2.0, count: 56388"
          },
          {
            "name": "Background Merger - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 4.816208929241608, max cpu: 9.628887, count: 56388"
          },
          {
            "name": "Background Merger - Primary - mem",
            "value": 26.5625,
            "unit": "median mem",
            "extra": "avg mem: 26.610138543440094, max mem: 26.68359375, count: 56388"
          },
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 5.000667320204473, max cpu: 33.168808, count: 56388"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 184.32421875,
            "unit": "median mem",
            "extra": "avg mem: 184.96920908205203, max mem: 191.08984375, count: 56388"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 54114,
            "unit": "median block_count",
            "extra": "avg block_count: 53971.024012201175, max block_count: 54114.0, count: 56388"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 45,
            "unit": "median segment_count",
            "extra": "avg segment_count: 42.04884017876144, max segment_count: 56.0, count: 56388"
          },
          {
            "name": "Single Insert - Primary - cpu",
            "value": 4.669261,
            "unit": "median cpu",
            "extra": "avg cpu: 4.665690537254712, max cpu: 27.906979, count: 56388"
          },
          {
            "name": "Single Insert - Primary - mem",
            "value": 143.78125,
            "unit": "median mem",
            "extra": "avg mem: 134.02725743398418, max mem: 161.96875, count: 56388"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 5.157209236568295, max cpu: 32.65306, count: 56388"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 199.859375,
            "unit": "median mem",
            "extra": "avg mem: 197.74826959248864, max mem: 228.125, count: 56388"
          },
          {
            "name": "Top K - Primary - cpu",
            "value": 23.391813,
            "unit": "median cpu",
            "extra": "avg cpu: 23.983863890932458, max cpu: 33.83686, count: 56388"
          },
          {
            "name": "Top K - Primary - mem",
            "value": 164.1015625,
            "unit": "median mem",
            "extra": "avg mem: 181.66840199266687, max mem: 222.375, count: 56388"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ming.ying.nyc@gmail.com",
            "name": "Ming",
            "username": "rebasedming"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "21bdc9a753582039cc25a38cb07111026c0fd378",
          "message": "feat: Crash recovery via WAL (#4901)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nPorts WAL integration over to community, which gives `pg_search` crash\nrecovery.\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2026-05-14T14:59:53-07:00",
          "tree_id": "7580b5f2bfcc1a95ed1b62c4628f2ec6129b7c00",
          "url": "https://github.com/paradedb/paradedb/commit/21bdc9a753582039cc25a38cb07111026c0fd378"
        },
        "date": 1778799317125,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Background Merger - Primary - background_merging",
            "value": 0,
            "unit": "median background_merging",
            "extra": "avg background_merging: 0.07172748873993687, max background_merging: 2.0, count: 56394"
          },
          {
            "name": "Background Merger - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.789528041084795, max cpu: 9.599999, count: 56394"
          },
          {
            "name": "Background Merger - Primary - mem",
            "value": 29.796875,
            "unit": "median mem",
            "extra": "avg mem: 29.785916595182997, max mem: 29.80078125, count: 56394"
          },
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.972211053129, max cpu: 27.87996, count: 56394"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 192.0390625,
            "unit": "median mem",
            "extra": "avg mem: 188.63515407501242, max mem: 192.25390625, count: 56394"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 51439,
            "unit": "median block_count",
            "extra": "avg block_count: 51303.87193673086, max block_count: 51439.0, count: 56394"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 45,
            "unit": "median segment_count",
            "extra": "avg segment_count: 43.06238252296344, max segment_count: 56.0, count: 56394"
          },
          {
            "name": "Single Insert - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.695524911336681, max cpu: 23.762377, count: 56394"
          },
          {
            "name": "Single Insert - Primary - mem",
            "value": 144.84375,
            "unit": "median mem",
            "extra": "avg mem: 132.22289231234706, max mem: 156.953125, count: 56394"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.817686504890414, max cpu: 28.042841, count: 56394"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 191.45703125,
            "unit": "median mem",
            "extra": "avg mem: 186.4568763687183, max mem: 191.578125, count: 56394"
          },
          {
            "name": "Top K - Primary - cpu",
            "value": 23.346306,
            "unit": "median cpu",
            "extra": "avg cpu: 23.955026031676418, max cpu: 33.939396, count: 56394"
          },
          {
            "name": "Top K - Primary - mem",
            "value": 164.1640625,
            "unit": "median mem",
            "extra": "avg mem: 182.27958592669432, max mem: 222.6796875, count: 56394"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "d99908a1f58307567dd34698345b3fc836be6135",
          "message": "feat(antithesis): add singleton drivers for logical-replication suites (#5087)\n\n## Summary\n\nAdds OSS Antithesis singleton drivers for the two CI\n`logical-replication` suites that previously had no singleton\n(`single-server`, `bulk-updates`, `wide-table`, `background-merge`, and\n`vanilla-postgres` already had one).\n\nMirrors the enterprise pattern used for `physical-logical-replication`:\n- A **vanilla Postgres 18** publisher pod (with `wal_level=logical`)\nthat lives outside the CNPG cluster, reflecting real-world\nlogical-replication topologies where the upstream primary is not under\nour control.\n- Subscriber points at `paradedb-rw` (the CNPG primary, which has\n`pg_search`).\n\n## Changes\n- `docker/manifests/antithesis-paradedb.yaml` — add\n`logical-replication-publisher` Service + Deployment (vanilla Postgres\n18 with `wal_level=logical`), reusing the existing `paradedb-superuser`\nsecret.\n- `stressgres/suites/logical-replication.toml`,\n`stressgres/suites/logical-replication-merge.toml` — drop `CREATE\nEXTENSION pg_search` from the **Publisher** setup. Only the Subscriber\nuses `pg_search`; the line was cosmetic and incompatible with a vanilla\nPostgres publisher (the line in the Subscriber setup is unchanged).\n-\n`stressgres/suites/antithesis/singleton_driver_logical-replication.sh`,\n`singleton_driver_logical-replication-merge.sh` — new drivers that\nperform per-block `sed -z` rewrites of the `[server.style.Automatic]`\nblocks into `[server.style.With]` connection strings (Publisher →\n`logical-replication-publisher:5432`, Subscriber → `paradedb-rw:5432`).\n- `.github/workflows/antithesis-trigger-test-run.yml` — add\n`logical-replication-publisher` to\n`container_faults_stop_exclusion_patterns` and\n`container_faults_kill_exclusion_patterns`, matching enterprise. Network\nfaults to/from the publisher are intentionally still injected.\n\n## Why\nWithout these, the FSM race repro in `logical-replication-merge.toml`\n(issue #4935, fixed by #5067) and the broader logical-replication\ncoverage in `logical-replication.toml` were running in\n`benchmark-pg_search-stressgres` but had no Antithesis fault-injection\nequivalent — that's the half of the matrix where the bugs originally\nsurfaced.\n\n## Test plan\n- [x] Antithesis trigger workflow picks up both new singleton drivers\nfrom `/opt/antithesis/test/v1/quickstart/`\n- [x] Publisher pod (`logical-replication-publisher`) starts with\n`wal_level=logical` and is reachable from the stressgres-runner pod\n- [x] Subscriber's `CREATE SUBSCRIPTION ... CONNECTION\n'@Publisher_CONNSTR@'` resolves to the publisher pod after the `sed`\nrewrite\n- [x] `logical-replication-merge.toml` still reproduces the FSM race\nwhen run against a build without #5067\n- [x] `benchmark-pg_search-stressgres` (local Stressgres, not\nAntithesis) still runs both suites unchanged",
          "timestamp": "2026-05-14T18:54:16-04:00",
          "tree_id": "8c7a6dab334db43248e88e7a4ebc75fd5840e446",
          "url": "https://github.com/paradedb/paradedb/commit/d99908a1f58307567dd34698345b3fc836be6135"
        },
        "date": 1778802486768,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Background Merger - Primary - background_merging",
            "value": 0,
            "unit": "median background_merging",
            "extra": "avg background_merging: 0.0698887281430281, max background_merging: 2.0, count: 55989"
          },
          {
            "name": "Background Merger - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.700788737798098, max cpu: 9.60961, count: 55989"
          },
          {
            "name": "Background Merger - Primary - mem",
            "value": 26.79296875,
            "unit": "median mem",
            "extra": "avg mem: 26.781932193154013, max mem: 26.796875, count: 55989"
          },
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 5.029046599336164, max cpu: 28.402367, count: 55989"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 192.015625,
            "unit": "median mem",
            "extra": "avg mem: 190.12153314992676, max mem: 192.25390625, count: 55989"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 51232,
            "unit": "median block_count",
            "extra": "avg block_count: 51100.603172051655, max block_count: 51232.0, count: 55989"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 45,
            "unit": "median segment_count",
            "extra": "avg segment_count: 42.22654449981246, max segment_count: 57.0, count: 55989"
          },
          {
            "name": "Single Insert - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.688072608059164, max cpu: 28.263002, count: 55989"
          },
          {
            "name": "Single Insert - Primary - mem",
            "value": 151.04296875,
            "unit": "median mem",
            "extra": "avg mem: 141.90886337606494, max mem: 166.1875, count: 55989"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.851010664114843, max cpu: 28.346458, count: 55989"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 192.19140625,
            "unit": "median mem",
            "extra": "avg mem: 188.65678498231796, max mem: 192.3359375, count: 55989"
          },
          {
            "name": "Top K - Primary - cpu",
            "value": 23.391813,
            "unit": "median cpu",
            "extra": "avg cpu: 23.714703175975732, max cpu: 33.768845, count: 55989"
          },
          {
            "name": "Top K - Primary - mem",
            "value": 164.07421875,
            "unit": "median mem",
            "extra": "avg mem: 181.6737318798112, max mem: 222.515625, count: 55989"
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
        "date": 1778655075601,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 30.775423880318776,
            "unit": "median tps",
            "extra": "avg tps: 30.415012186253257, max tps: 32.8455204795995, count: 55561"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 245.23800853242292,
            "unit": "median tps",
            "extra": "avg tps: 273.47086833731726, max tps: 2989.035358445735, count: 55561"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 626.4908585412311,
            "unit": "median tps",
            "extra": "avg tps: 615.4794115406081, max tps: 703.0442296439094, count: 55561"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 168.9955774401426,
            "unit": "median tps",
            "extra": "avg tps: 184.42279684760555, max tps: 1272.442796985447, count: 111122"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 16.716986996539305,
            "unit": "median tps",
            "extra": "avg tps: 16.554574188203365, max tps: 21.535667070302495, count: 55561"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "59696464+saadtajwar@users.noreply.github.com",
            "name": "Saad Tajwar",
            "username": "saadtajwar"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "bc20c3dd4b36a13dbcc74d03499f966a4dc93fe6",
          "message": "refactor: Deduplicate deferred materialization request partitioning between visibility and lookup (#4903)\n\n# Ticket(s) Closed\n- Closes https://github.com/paradedb/paradedb/issues/4568\n\n## What\nDeduplicated the segment-grouping/materialization loop shared between\n`materialize_deferred_ctid()` in `visibility_filter.rs` and\n`materialize_deferred_column()` in `tantivy_lookup_exec.rs`.\n\n## Why\nBoth functions implemented the same pattern of partitioning packed doc\naddresses by segment ordinal, batch-reading via `FFHelper`, and writing\nresults back in row order. This duplication made the two paths harder to\nkeep consistent and made future optimization work more tedious.\n\n## How\n\n### Shared helper in `fast_fields_helper.rs`\n- Added `for_each_segment`: partitions an iterator of `(row_index,\npacked_doc_address)` pairs into per-segment buckets and invokes a\ncaller-supplied closure once per non-empty segment, in segment-ordinal\norder. Backed by a `Vec<Vec<(usize, DocId)>>` indexed by segment ordinal\n(dense in practice; cheaper than a hash map for typical segment counts).\n- Added `FFHelper::num_segments()` so callers can size the bucket vector\nwithout reaching into private fields.\n\n### `materialize_deferred_ctid` (visibility_filter.rs)\n- Replaced the manual sort + partition + per-segment slice loop with a\nsingle `for_each_segment` call.\n- Kept `DeferredCtidMaterializationState` for buffer reuse across calls,\nbut removed its now-unused `requests` field; the per-segment\npartitioning lives inside `for_each_segment`.\n- Removed the TODO comment that flagged this duplication.\n\n### `materialize_deferred_column` (tantivy_lookup_exec.rs)\nFunction body shrank from ~160 lines to ~50 by extracting three\nsingle-responsibility helpers:\n- `resolve_doc_addresses_to_term_ords` — resolves State 0 (packed doc\naddresses) into per-segment `(row_index, Option<TermOrdinal>)` pairs via\n`for_each_segment`.\n- `extract_term_ords` — parses State 1 (pre-resolved `(segment_ord,\nterm_ord)` pairs from the dense union's `StructArray` child) into the\nsame per-segment shape.\n- `decode_term_ordinals` — takes the merged per-segment ordinals and\nperforms the bulk dictionary lookup once per segment, recording\npositions for the final `interleave`.\n\nState 0 and State 1 are now merged into a single `Vec<Vec<(row_index,\nOption<TermOrdinal>)>>` indexed by segment ordinal, then decoded in one\npass — previously each state was iterated and decoded separately,\nproducing two `segment_arrays` entries per segment touched by both. The\nfinal interleaved output is identical.\n\nReplaced the `(ff_index: usize, is_bytes: bool)` parameter pair with a\n`DeferredColumnKind { Text { ff_index }, Bytes { ff_index } }` enum to\nmake the `is_bytes && wrong-ff-type` mismatch unrepresentable.\n\n## Tests\nNo new tests; behavior is unchanged. Existing coverage exercises both\npaths end-to-end:\n- `pg_search/tests/pg_regress/sql/join_deferred_visibility.sql` —\n`materialize_deferred_ctid`.\n- `pg_search/tests/pg_regress/sql/segmented_topk.sql` plus joinscan\ntests — `materialize_deferred_column` (the segmented top-K rule is what\nproduces State 1 rows below `TantivyLookupExec`).\n\n---------\n\nCo-authored-by: Mithun Chicklore Yogendra <mithun.cy@gmail.com>",
          "timestamp": "2026-05-13T10:07:59-07:00",
          "tree_id": "cc9e05e63ed6052c202d00901357d2c5026923d0",
          "url": "https://github.com/paradedb/paradedb/commit/bc20c3dd4b36a13dbcc74d03499f966a4dc93fe6"
        },
        "date": 1778695942790,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 28.655200752172252,
            "unit": "median tps",
            "extra": "avg tps: 28.54374873661204, max tps: 30.948370745156932, count: 55459"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 241.61903400581235,
            "unit": "median tps",
            "extra": "avg tps: 269.3655339207487, max tps: 2931.326936838492, count: 55459"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 651.7142295397291,
            "unit": "median tps",
            "extra": "avg tps: 634.3523655728744, max tps: 976.0728105837428, count: 55459"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 158.09325838615982,
            "unit": "median tps",
            "extra": "avg tps: 177.26914474887613, max tps: 956.6120976323839, count: 110918"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 15.994468299723824,
            "unit": "median tps",
            "extra": "avg tps: 15.966047562122503, max tps: 24.39388116894828, count: 55459"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "stuhood@paradedb.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "713eaca82145388378043ddb2c750c41e9c632d9",
          "message": "chore: Revert #4905 (#5066)\n\nRevert #4905 due to performance regressions.",
          "timestamp": "2026-05-13T13:34:27-07:00",
          "tree_id": "2894402ba71bbb0b86d5990862fb2adba4fcbf49",
          "url": "https://github.com/paradedb/paradedb/commit/713eaca82145388378043ddb2c750c41e9c632d9"
        },
        "date": 1778708319416,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 29.596747882846497,
            "unit": "median tps",
            "extra": "avg tps: 29.44013292222217, max tps: 34.895485307411, count: 55735"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 241.5712798678313,
            "unit": "median tps",
            "extra": "avg tps: 272.07474497263377, max tps: 3121.97963196926, count: 55735"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 656.6361840575292,
            "unit": "median tps",
            "extra": "avg tps: 635.0704485864899, max tps: 1040.6380960191568, count: 55735"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 158.52752897222814,
            "unit": "median tps",
            "extra": "avg tps: 178.30314039640857, max tps: 1016.0657945776233, count: 111470"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 16.419604456045867,
            "unit": "median tps",
            "extra": "avg tps: 16.22484919101891, max tps: 20.54662740671893, count: 55735"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "035982deb7c1e630e0d8a13e4ca1910b979b08f6",
          "message": "ci: add logical-replication-merge stressgres suite to cover FSM race (#4935) (#5072)\n\nRe-opened from #5068 (originally from a fork, where CI could not access\nworkflow secrets — see\nhttps://github.com/paradedb/paradedb/actions/runs/25830120431/job/75892835684?pr=5068).\nSame intent, from a branch in this repo, with the FSM-race suite landing\nas its own file rather than folded into `logical-replication.toml`.\n\n## Changes\n\n- `stressgres/suites/logical-replication-merge.toml` (new): standalone\nsuite that reliably reproduces the FSM race from #4935 (fixed in #5067).\nLogical-replication subscriber with aggressive autovacuum (`naptime=1s`,\n`threshold=50`), small `layer_sizes = '10kb, 100kb, 1mb, 100mb'`,\nmultiple concurrent BM25 readers, and sustained UPDATE/INSERT/DELETE\ntraffic on the publisher. The key difference from\n`logical-replication.toml` is the writer: `message = message || ' ' ||\ntxid_current()` grows each row's terms unbounded, generating ~10× more\nmerge/GC pressure and reliably opening the race window — folding the\nsame churn into `logical-replication.toml` (which strips-then-appends,\nkeeping row size constant) did not reproduce the bug.\n- `.github/workflows/benchmark-pg_search-stressgres.yml`:\n- Comment out single-server, bulk-updates, wide-table, and\nbackground-merge so CI focuses on the two replication suites while we\niterate. To be re-enabled before final merge.\n- Run `logical-replication-merge.toml` **before**\n`logical-replication.toml`.\n\n`stressgres/suites/logical-replication.toml` is unchanged from `main`.\n\n## Expected behavior\n\n- Without #5067: SIGSEGV or `SegmentMetaEntryHeader: UnexpectedEnd`\nwithin minutes.\n- With #5067: runs the full duration without errors.\n\n## Follow-ups\n\n- Re-enable the four commented-out suites before final merge.\n- Antithesis wiring for this suite belongs in `paradedb-enterprise` next\nto the existing `physical-logical-replication` driver, since the OSS\nAntithesis manifest only stands up a single paradedb cluster.\n\nRef: #4935\nRelated: #5067\nSupersedes: #5068",
          "timestamp": "2026-05-13T19:51:05-04:00",
          "tree_id": "c71af69d7df60d54d1631876f7e3c7af0782c3e3",
          "url": "https://github.com/paradedb/paradedb/commit/035982deb7c1e630e0d8a13e4ca1910b979b08f6"
        },
        "date": 1778720106901,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 30.43732498324333,
            "unit": "median tps",
            "extra": "avg tps: 30.160917795348585, max tps: 31.3157373876248, count: 55494"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 247.52801581891356,
            "unit": "median tps",
            "extra": "avg tps: 279.99721516078483, max tps: 3070.3172836639824, count: 55494"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 648.1929067689173,
            "unit": "median tps",
            "extra": "avg tps: 636.1226101311144, max tps: 992.0698151873142, count: 55494"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 166.06798546255982,
            "unit": "median tps",
            "extra": "avg tps: 183.60105150712613, max tps: 1093.8135805437287, count: 110988"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 16.427897933565,
            "unit": "median tps",
            "extra": "avg tps: 16.40893877839354, max tps: 20.983589825544115, count: 55494"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "36623265+daniel3303@users.noreply.github.com",
            "name": "Daniel Oliveira",
            "username": "daniel3303"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "b9c06c5f75ca94cf815eb57c71c46180e84b8038",
          "message": "fix(mlt): quote key_field identifier in internal SPI lookup (#5078)\n\n## Summary\n\n- `pdb.more_like_this(key_value)` raises `ERROR: column \"id\" does not\nexist` whenever the index's `key_field` column is a mixed-case\nPostgreSQL identifier (e.g. `\"Id\"`, `\"DocumentId\"`). Direct `@@@`-on-LHS\nsearches (`\"Content\" @@@ 'foo'`) are unaffected because they don't go\nthrough the internal SPI lookup. Repro in #5065.\n- Root cause: `pg_search/src/query/more_like_this.rs:152-157` builds the\nSPI `SELECT * FROM <ns>.<rel> WHERE <key_field> = $1` with `<ns>` and\n`<rel>` already routed through `pgrx::spi::quote_identifier`, but\ninterpolates `<key_field>` verbatim via `Display`. PostgreSQL folds the\nunquoted reference to lowercase, so a column named `\"Id\"` is looked up\nas `id` and the SPI call fails before MLT ever runs.\n- Fix: send the key field through\n`pgrx::spi::quote_identifier(key_field_name.root())`, matching how the\nnamespace and relation names are already quoted on the lines immediately\nabove. `.root()` strips the JSON sub-path (`key_field` is always a\ntop-level column).\n\n## Scope\n\nThe linked issue also lists JSON `term` filters (`@@@\n'{\"term\":{\"field\":\"Category\",…}}'::jsonb`) as failing on mixed-case\ncolumns. That path does **not** go through SPI — `term()` in\n`pg_search/src/query/pdb_query.rs:792` resolves the field via\n`schema.search_field(field.root())`, a pure Tantivy schema lookup — so\nit isn't fixed here and I couldn't find a corresponding\nunquoted-identifier hazard. If it reproduces on `0.23.x` it's a separate\nbug; tracking it on its own issue is cleaner than bundling a speculative\nfix.\n\n## Test plan\n\n- [x] `cargo test -p tests --test mlt --\nmlt_mixed_case_key_field_issue5065` — new regression test: `\"Id\"` /\n`\"Content\"` table, `key_field='Id'`, asserts `pdb.more_like_this(1)`\nreturns rows. Fails on `main` with `column \"id\" does not exist`, passes\nwith this change.\n- [x] `cargo test -p tests --test mlt` — existing\n`mlt_enables_scoring_issue1747`, `mlt_datetime_key`,\n`mlt_scoring_nested` still pass.\n- [x] `cargo pgrx regress -p pg_search --auto -- pg18 more_like_this` —\ngolden output unchanged (`quote_identifier(\"id\")` is a no-op for\nalready-lowercase identifiers).\n- [x] Manual repro from #5065 (`CREATE TABLE items (\"Id\" int primary\nkey, \"Content\" text); … pdb.more_like_this(1)`) returns rows instead of\nerroring.\n\nCloses #5065.",
          "timestamp": "2026-05-14T14:45:18-04:00",
          "tree_id": "812b4a66ffce7bc074cb919986f1de3b6474813f",
          "url": "https://github.com/paradedb/paradedb/commit/b9c06c5f75ca94cf815eb57c71c46180e84b8038"
        },
        "date": 1778788174714,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 31.414254914370346,
            "unit": "median tps",
            "extra": "avg tps: 31.21516720606377, max tps: 33.41453313351819, count: 55509"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 247.76885015521665,
            "unit": "median tps",
            "extra": "avg tps: 280.4875029281766, max tps: 3061.2629085488097, count: 55509"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 637.3785351213331,
            "unit": "median tps",
            "extra": "avg tps: 617.8205049587241, max tps: 1146.3693974992116, count: 55509"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 166.29072591141357,
            "unit": "median tps",
            "extra": "avg tps: 185.01621400521046, max tps: 1295.9431555818542, count: 111018"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 16.43616672803072,
            "unit": "median tps",
            "extra": "avg tps: 16.396525047279628, max tps: 22.170800253483193, count: 55509"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "balavignesh449@gmail.com",
            "name": "S Bala Vignesh",
            "username": "SBALAVIGNESH123"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "d0039460372b22539b33262920b0194c694b7205",
          "message": "fix: use fixed-length updates in stressgres to avoid TOAST and expose FSM race (#5080)\n\nThe old UPDATE pattern in the logical-replication stressgres suite\nappended txid_current() to the message column every iteration, growing\nit past the TOAST threshold (~2KB). This caused the suite to hit the\nunrelated TOAST visibility race (#5076) before the FSM segment metadata\nrace (#4935) could surface.\n\nChanged to fixed-length updates that keep the first search term and\nappend a small txid-derived number, staying well under the TOAST\nthreshold. This way the suite can run long enough to exercise the FSM\npath.\n\nRelated: #5067 (FSM race fix), #5076 (TOAST bug)",
          "timestamp": "2026-05-14T14:57:31-04:00",
          "tree_id": "96511f40645bb3416046b0914f9758c60c159a20",
          "url": "https://github.com/paradedb/paradedb/commit/d0039460372b22539b33262920b0194c694b7205"
        },
        "date": 1778788980018,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 30.789842646835172,
            "unit": "median tps",
            "extra": "avg tps: 30.709396526842426, max tps: 33.09558460093368, count: 55653"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 250.67809954683236,
            "unit": "median tps",
            "extra": "avg tps: 282.3049658842204, max tps: 3099.6743112993854, count: 55653"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 671.2085827292306,
            "unit": "median tps",
            "extra": "avg tps: 654.4615233415154, max tps: 869.4356216391601, count: 55653"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 169.26382634058297,
            "unit": "median tps",
            "extra": "avg tps: 185.57018292696944, max tps: 1141.1878549550236, count: 111306"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 16.4943241547232,
            "unit": "median tps",
            "extra": "avg tps: 16.44598864912953, max tps: 20.668313641907535, count: 55653"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "balavignesh449@gmail.com",
            "name": "S Bala Vignesh",
            "username": "SBALAVIGNESH123"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "b597b183d25a561cf8c81d81decc7f07b7866e55",
          "message": "fix: hold header shared lock during LinkedItemList iteration to prevent FSM race (#4935) (#5067)\n\n## Problem\n\nLinkedItemList read-only iteration methods (list(), is_empty(),\nor_each(), lookup()/lookup_ex()) were releasing the header shared lock\nbefore iterating through the linked list blocks. This allowed\nAtomicGuard::commit() to:\n\n1. Swap the header pointer to a new list\n2. Immediately recycle old blocks to the FSM with\nReadNextFullTransactionId()\n3. The next transaction pops a recycled block via RBM_ZERO_AND_LOCK,\nzeroing it\n4. A concurrent reader still traversing the old list hits the\nzeroed/repurposed block\n5. **SIGSEGV** or SegmentMetaEntryHeader: UnexpectedEnd deserialization\nerror\n\nThis manifests as periodic crashes and durable index corruption under\nsustained write traffic, particularly on logical replication subscribers\nat high apply rates (~395 commits/sec). Correlates strongly with\nautovacuum events that trigger garbage_collect_index().\n\n## Root Cause\n\nThe race window exists because or_each(), list(), is_empty(), and\nlookup_ex() call get_start_blockno() which acquires a shared lock on the\nheader, reads start_blockno, then immediately releases the header lock\nwhen exchanging to the first data block. After that point, \u0007tomically()\ncan take an exclusive header lock and proceed with the swap+recycle\nwhile the reader is deep in the old list.\n\n**This was already a known pattern in the codebase** — emove_item() and\nupdate_item() in the same file both hold the header shared lock for\ntheir entire operation with this comment:\n\n\\\\\\\rust\n// Acquire and hold a shared lock on the header for the entire\noperation, preventing the\n// list from being swapped out from under us by atomically between our\nread locks and\n// our write locks.\nlet header_lock = self.bman.get_buffer(self.header_blockno);\n\\\\\\\n\nThe read-only methods simply weren't given the same treatment.\n\n## Fix\n\nHold a shared lock on the header for the entire duration of iteration in\nall 4 methods, matching the existing emove_item()/update_item() pattern:\n\n- **list()** — hold header_lock from start to end of iteration\n- **is_empty()** — same\n- **\for_each()** — same\n- **lookup_ex()** — conditionally: only when \blockno is None (top-level\ncall). When \blockno is Some, the caller ( emove_item/update_item)\nalready holds the header lock\n\nRead start_blockno directly from the already-held header_lock instead of\ncalling get_start_blockno(), avoiding a double shared-lock acquisition\non the same block (which would trigger a panic under the \block_tracker\ndebug feature).\n\n## Why This Is Safe\n\n- **No deadlock**: Header block is always locked first, content blocks\nin ascending order — consistent lock ordering\n- **No reader-reader blocking**: Multiple readers hold shared locks\nconcurrently (shared locks are compatible)\n- **Writer waits for readers**: \u0007tomically() takes an exclusive header\nlock, which blocks until all shared locks are released — correct\nserialization\n- **Minimal performance impact**: The header lock was already acquired;\nwe just hold it slightly longer\n\n## Verification\n\n- \rustfmt --check passes\n- Pattern matches the proven emove_item()/update_item() implementation\nin the same file\n- Full cargo check requires pgrx setup (PostgreSQL extension); the\nchange is limited to lock lifetime management with no new APIs\n\nCloses #4935",
          "timestamp": "2026-05-14T15:00:05-04:00",
          "tree_id": "46e474245958de09c21d0198195343d8a87fb72d",
          "url": "https://github.com/paradedb/paradedb/commit/b597b183d25a561cf8c81d81decc7f07b7866e55"
        },
        "date": 1778789063951,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 30.920374254358205,
            "unit": "median tps",
            "extra": "avg tps: 30.779514758283096, max tps: 34.51590096419388, count: 55579"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 248.4172711065372,
            "unit": "median tps",
            "extra": "avg tps: 283.28460771445276, max tps: 3292.3879502828286, count: 55579"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 645.4885568648756,
            "unit": "median tps",
            "extra": "avg tps: 632.8176525951947, max tps: 1363.2642544614528, count: 55579"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 169.94294493654266,
            "unit": "median tps",
            "extra": "avg tps: 183.90600437448893, max tps: 935.6421124689451, count: 111158"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 16.309364923923184,
            "unit": "median tps",
            "extra": "avg tps: 16.19479225998637, max tps: 21.074136071060302, count: 55579"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "652b0952a7f3be58ea4b9fa80a863426cf03185c",
          "message": "chore(stressgres): remove dormant suites not wired into CI (#5085)\n\n## Summary\n\n- Removes 8 Stressgres suites that have not been touched since\nStressgres was added (#3821), are not referenced by any CI workflow or\ndriver script, and use the legacy `[setup_primary]` schema rather than\nthe current `[[server]]` schema.\n- No CI workflow changes; the 6 suites currently exercised by\n`benchmark-pg_search-stressgres.yml` (`single-server`, `bulk-updates`,\n`wide-table`, `background-merge`, `logical-replication`,\n`logical-replication-merge`) and `vanilla-postgres.toml` (used by the\nantithesis singleton driver) are kept.\n\n### Removed\n- `large-inserts.toml`\n- `lr.toml` — predecessor of `logical-replication.toml`\n- `lr-graphable.toml`\n- `lr-large-inserts.toml`\n- `lr-no-pg_search.toml`\n- `many-updates.toml` — conceptually useful (MVCC correctness with\n`assert(count, expected)` under concurrency), but on legacy schema; if\nwe want this coverage in CI it should be a deliberate port, not a\nrevival of dead config\n- `read-write.toml` — contained placeholder `<password>` literal, not\nCI-runnable\n- `topk-crash.toml`\n\n### Why now\nCompanion to #5080 (which adjusts `logical-replication.toml` to expose\nthe FSM race instead of being masked by the TOAST bug already covered by\n`logical-replication-merge.toml`). Each CI suite should own a distinct\nrepro; dormant files muddy that mapping.\n\n## Test plan\n- [x] `benchmark-pg_search-stressgres` workflow still kicks off for all\n6 in-CI suites\n- [x] Antithesis `singleton_driver_vanilla-postgres.sh` still resolves\nits suite path",
          "timestamp": "2026-05-14T15:45:37-04:00",
          "tree_id": "090e72f7bbb9817642fb0c7495c2b12e0eed2fe7",
          "url": "https://github.com/paradedb/paradedb/commit/652b0952a7f3be58ea4b9fa80a863426cf03185c"
        },
        "date": 1778791916846,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 31.512351168412067,
            "unit": "median tps",
            "extra": "avg tps: 31.198718399237777, max tps: 34.05686564839136, count: 55633"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 247.23298847448768,
            "unit": "median tps",
            "extra": "avg tps: 279.0271825549024, max tps: 3068.1795372497068, count: 55633"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 650.1245458734927,
            "unit": "median tps",
            "extra": "avg tps: 636.6645156258545, max tps: 967.6865406320356, count: 55633"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 167.33679451390952,
            "unit": "median tps",
            "extra": "avg tps: 180.8615050967497, max tps: 865.0106542281018, count: 111266"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 15.855563956591451,
            "unit": "median tps",
            "extra": "avg tps: 15.697831732992082, max tps: 18.883181782896017, count: 55633"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ming.ying.nyc@gmail.com",
            "name": "Ming",
            "username": "rebasedming"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "21bdc9a753582039cc25a38cb07111026c0fd378",
          "message": "feat: Crash recovery via WAL (#4901)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nPorts WAL integration over to community, which gives `pg_search` crash\nrecovery.\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2026-05-14T14:59:53-07:00",
          "tree_id": "7580b5f2bfcc1a95ed1b62c4628f2ec6129b7c00",
          "url": "https://github.com/paradedb/paradedb/commit/21bdc9a753582039cc25a38cb07111026c0fd378"
        },
        "date": 1778799961941,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 30.006405977141064,
            "unit": "median tps",
            "extra": "avg tps: 30.02862726436292, max tps: 32.811108689367735, count: 55542"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 245.20126879462342,
            "unit": "median tps",
            "extra": "avg tps: 270.40644978476007, max tps: 2713.2780058049257, count: 55542"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 610.6074601626152,
            "unit": "median tps",
            "extra": "avg tps: 598.9979047274052, max tps: 699.404352179078, count: 55542"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 158.4055819789134,
            "unit": "median tps",
            "extra": "avg tps: 173.82327741809985, max tps: 914.9812598465181, count: 111084"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 16.657817970759876,
            "unit": "median tps",
            "extra": "avg tps: 16.73506484940348, max tps: 21.89891299708589, count: 55542"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "d99908a1f58307567dd34698345b3fc836be6135",
          "message": "feat(antithesis): add singleton drivers for logical-replication suites (#5087)\n\n## Summary\n\nAdds OSS Antithesis singleton drivers for the two CI\n`logical-replication` suites that previously had no singleton\n(`single-server`, `bulk-updates`, `wide-table`, `background-merge`, and\n`vanilla-postgres` already had one).\n\nMirrors the enterprise pattern used for `physical-logical-replication`:\n- A **vanilla Postgres 18** publisher pod (with `wal_level=logical`)\nthat lives outside the CNPG cluster, reflecting real-world\nlogical-replication topologies where the upstream primary is not under\nour control.\n- Subscriber points at `paradedb-rw` (the CNPG primary, which has\n`pg_search`).\n\n## Changes\n- `docker/manifests/antithesis-paradedb.yaml` — add\n`logical-replication-publisher` Service + Deployment (vanilla Postgres\n18 with `wal_level=logical`), reusing the existing `paradedb-superuser`\nsecret.\n- `stressgres/suites/logical-replication.toml`,\n`stressgres/suites/logical-replication-merge.toml` — drop `CREATE\nEXTENSION pg_search` from the **Publisher** setup. Only the Subscriber\nuses `pg_search`; the line was cosmetic and incompatible with a vanilla\nPostgres publisher (the line in the Subscriber setup is unchanged).\n-\n`stressgres/suites/antithesis/singleton_driver_logical-replication.sh`,\n`singleton_driver_logical-replication-merge.sh` — new drivers that\nperform per-block `sed -z` rewrites of the `[server.style.Automatic]`\nblocks into `[server.style.With]` connection strings (Publisher →\n`logical-replication-publisher:5432`, Subscriber → `paradedb-rw:5432`).\n- `.github/workflows/antithesis-trigger-test-run.yml` — add\n`logical-replication-publisher` to\n`container_faults_stop_exclusion_patterns` and\n`container_faults_kill_exclusion_patterns`, matching enterprise. Network\nfaults to/from the publisher are intentionally still injected.\n\n## Why\nWithout these, the FSM race repro in `logical-replication-merge.toml`\n(issue #4935, fixed by #5067) and the broader logical-replication\ncoverage in `logical-replication.toml` were running in\n`benchmark-pg_search-stressgres` but had no Antithesis fault-injection\nequivalent — that's the half of the matrix where the bugs originally\nsurfaced.\n\n## Test plan\n- [x] Antithesis trigger workflow picks up both new singleton drivers\nfrom `/opt/antithesis/test/v1/quickstart/`\n- [x] Publisher pod (`logical-replication-publisher`) starts with\n`wal_level=logical` and is reachable from the stressgres-runner pod\n- [x] Subscriber's `CREATE SUBSCRIPTION ... CONNECTION\n'@Publisher_CONNSTR@'` resolves to the publisher pod after the `sed`\nrewrite\n- [x] `logical-replication-merge.toml` still reproduces the FSM race\nwhen run against a build without #5067\n- [x] `benchmark-pg_search-stressgres` (local Stressgres, not\nAntithesis) still runs both suites unchanged",
          "timestamp": "2026-05-14T18:54:16-04:00",
          "tree_id": "8c7a6dab334db43248e88e7a4ebc75fd5840e446",
          "url": "https://github.com/paradedb/paradedb/commit/d99908a1f58307567dd34698345b3fc836be6135"
        },
        "date": 1778803132581,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 30.885425397435245,
            "unit": "median tps",
            "extra": "avg tps: 30.688238263529424, max tps: 32.69088493538128, count: 55578"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 244.4775764972922,
            "unit": "median tps",
            "extra": "avg tps: 273.55390014524755, max tps: 2958.325513801051, count: 55578"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 588.9164140208602,
            "unit": "median tps",
            "extra": "avg tps: 574.147805637711, max tps: 699.8316803453837, count: 55578"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 162.44533363391216,
            "unit": "median tps",
            "extra": "avg tps: 177.6912950051219, max tps: 993.7701195316821, count: 111156"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 16.62790356171714,
            "unit": "median tps",
            "extra": "avg tps: 16.529030752770787, max tps: 19.434950579058523, count: 55578"
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
        "date": 1778655109430,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.640776,
            "unit": "median cpu",
            "extra": "avg cpu: 20.004673951089067, max cpu: 42.814667, count: 55561"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 176.23828125,
            "unit": "median mem",
            "extra": "avg mem: 164.98792965164415, max mem: 179.296875, count: 55561"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 7.715356182542229, max cpu: 28.09756, count: 55561"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 121.1796875,
            "unit": "median mem",
            "extra": "avg mem: 119.86778530860225, max mem: 121.2421875, count: 55561"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 6.353737498402488, max cpu: 18.497108, count: 55561"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 165.78515625,
            "unit": "median mem",
            "extra": "avg mem: 145.4835018746288, max mem: 180.2734375, count: 55561"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 16654,
            "unit": "median block_count",
            "extra": "avg block_count: 17001.208221594283, max block_count: 31713.0, count: 55561"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.407397789487303, max cpu: 4.7058825, count: 55561"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 106.10546875,
            "unit": "median mem",
            "extra": "avg mem: 95.86252483194147, max mem: 138.49609375, count: 55561"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 25,
            "unit": "median segment_count",
            "extra": "avg segment_count: 25.30920969744965, max segment_count: 36.0, count: 55561"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.257474,
            "unit": "median cpu",
            "extra": "avg cpu: 8.979440905101422, max cpu: 33.908947, count: 111122"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 182.671875,
            "unit": "median mem",
            "extra": "avg mem: 163.57502336255422, max mem: 183.5546875, count: 111122"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.88621,
            "unit": "median cpu",
            "extra": "avg cpu: 12.390577723258808, max cpu: 23.369036, count: 55561"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 174.14453125,
            "unit": "median mem",
            "extra": "avg mem: 171.55960847086985, max mem: 174.84375, count: 55561"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "59696464+saadtajwar@users.noreply.github.com",
            "name": "Saad Tajwar",
            "username": "saadtajwar"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "bc20c3dd4b36a13dbcc74d03499f966a4dc93fe6",
          "message": "refactor: Deduplicate deferred materialization request partitioning between visibility and lookup (#4903)\n\n# Ticket(s) Closed\n- Closes https://github.com/paradedb/paradedb/issues/4568\n\n## What\nDeduplicated the segment-grouping/materialization loop shared between\n`materialize_deferred_ctid()` in `visibility_filter.rs` and\n`materialize_deferred_column()` in `tantivy_lookup_exec.rs`.\n\n## Why\nBoth functions implemented the same pattern of partitioning packed doc\naddresses by segment ordinal, batch-reading via `FFHelper`, and writing\nresults back in row order. This duplication made the two paths harder to\nkeep consistent and made future optimization work more tedious.\n\n## How\n\n### Shared helper in `fast_fields_helper.rs`\n- Added `for_each_segment`: partitions an iterator of `(row_index,\npacked_doc_address)` pairs into per-segment buckets and invokes a\ncaller-supplied closure once per non-empty segment, in segment-ordinal\norder. Backed by a `Vec<Vec<(usize, DocId)>>` indexed by segment ordinal\n(dense in practice; cheaper than a hash map for typical segment counts).\n- Added `FFHelper::num_segments()` so callers can size the bucket vector\nwithout reaching into private fields.\n\n### `materialize_deferred_ctid` (visibility_filter.rs)\n- Replaced the manual sort + partition + per-segment slice loop with a\nsingle `for_each_segment` call.\n- Kept `DeferredCtidMaterializationState` for buffer reuse across calls,\nbut removed its now-unused `requests` field; the per-segment\npartitioning lives inside `for_each_segment`.\n- Removed the TODO comment that flagged this duplication.\n\n### `materialize_deferred_column` (tantivy_lookup_exec.rs)\nFunction body shrank from ~160 lines to ~50 by extracting three\nsingle-responsibility helpers:\n- `resolve_doc_addresses_to_term_ords` — resolves State 0 (packed doc\naddresses) into per-segment `(row_index, Option<TermOrdinal>)` pairs via\n`for_each_segment`.\n- `extract_term_ords` — parses State 1 (pre-resolved `(segment_ord,\nterm_ord)` pairs from the dense union's `StructArray` child) into the\nsame per-segment shape.\n- `decode_term_ordinals` — takes the merged per-segment ordinals and\nperforms the bulk dictionary lookup once per segment, recording\npositions for the final `interleave`.\n\nState 0 and State 1 are now merged into a single `Vec<Vec<(row_index,\nOption<TermOrdinal>)>>` indexed by segment ordinal, then decoded in one\npass — previously each state was iterated and decoded separately,\nproducing two `segment_arrays` entries per segment touched by both. The\nfinal interleaved output is identical.\n\nReplaced the `(ff_index: usize, is_bytes: bool)` parameter pair with a\n`DeferredColumnKind { Text { ff_index }, Bytes { ff_index } }` enum to\nmake the `is_bytes && wrong-ff-type` mismatch unrepresentable.\n\n## Tests\nNo new tests; behavior is unchanged. Existing coverage exercises both\npaths end-to-end:\n- `pg_search/tests/pg_regress/sql/join_deferred_visibility.sql` —\n`materialize_deferred_ctid`.\n- `pg_search/tests/pg_regress/sql/segmented_topk.sql` plus joinscan\ntests — `materialize_deferred_column` (the segmented top-K rule is what\nproduces State 1 rows below `TantivyLookupExec`).\n\n---------\n\nCo-authored-by: Mithun Chicklore Yogendra <mithun.cy@gmail.com>",
          "timestamp": "2026-05-13T10:07:59-07:00",
          "tree_id": "cc9e05e63ed6052c202d00901357d2c5026923d0",
          "url": "https://github.com/paradedb/paradedb/commit/bc20c3dd4b36a13dbcc74d03499f966a4dc93fe6"
        },
        "date": 1778695974776,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.622696,
            "unit": "median cpu",
            "extra": "avg cpu: 20.21397788495145, max cpu: 46.376812, count: 55459"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 178.71484375,
            "unit": "median mem",
            "extra": "avg mem: 176.78876294030275, max mem: 179.10546875, count: 55459"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 7.629670588442259, max cpu: 27.87996, count: 55459"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 120.65234375,
            "unit": "median mem",
            "extra": "avg mem: 119.51352040471339, max mem: 120.8828125, count: 55459"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 6.099630287315251, max cpu: 23.323614, count: 55459"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 173.9765625,
            "unit": "median mem",
            "extra": "avg mem: 148.0949211213464, max mem: 180.94140625, count: 55459"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 16524,
            "unit": "median block_count",
            "extra": "avg block_count: 16772.00728466074, max block_count: 31233.0, count: 55459"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.3839001601556955, max cpu: 4.6875, count: 55459"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 110.73828125,
            "unit": "median mem",
            "extra": "avg mem: 97.68748196866153, max mem: 138.5625, count: 55459"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 26,
            "unit": "median segment_count",
            "extra": "avg segment_count: 25.722497701004347, max segment_count: 37.0, count: 55459"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.257474,
            "unit": "median cpu",
            "extra": "avg cpu: 9.285889505207543, max cpu: 32.24568, count: 110918"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 180.078125,
            "unit": "median mem",
            "extra": "avg mem: 163.96207703128664, max mem: 184.5546875, count: 110918"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.872832,
            "unit": "median cpu",
            "extra": "avg cpu: 12.362905586321125, max cpu: 28.152493, count: 55459"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 174.38671875,
            "unit": "median mem",
            "extra": "avg mem: 171.7574072879289, max mem: 175.11328125, count: 55459"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "stuhood@paradedb.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "713eaca82145388378043ddb2c750c41e9c632d9",
          "message": "chore: Revert #4905 (#5066)\n\nRevert #4905 due to performance regressions.",
          "timestamp": "2026-05-13T13:34:27-07:00",
          "tree_id": "2894402ba71bbb0b86d5990862fb2adba4fcbf49",
          "url": "https://github.com/paradedb/paradedb/commit/713eaca82145388378043ddb2c750c41e9c632d9"
        },
        "date": 1778708351363,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.658894,
            "unit": "median cpu",
            "extra": "avg cpu: 20.1436753451237, max cpu: 46.332047, count: 55735"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 172.234375,
            "unit": "median mem",
            "extra": "avg mem: 150.23035415919082, max mem: 179.1171875, count: 55735"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 7.604867164636882, max cpu: 28.125, count: 55735"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 119.76171875,
            "unit": "median mem",
            "extra": "avg mem: 118.59954864537544, max mem: 119.83984375, count: 55735"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 6.185734289607619, max cpu: 18.658894, count: 55735"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 168.60546875,
            "unit": "median mem",
            "extra": "avg mem: 145.00633452386293, max mem: 180.56640625, count: 55735"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 16596,
            "unit": "median block_count",
            "extra": "avg block_count: 16927.916390060105, max block_count: 31469.0, count: 55735"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.135359858869017, max cpu: 4.669261, count: 55735"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 106.9453125,
            "unit": "median mem",
            "extra": "avg mem: 94.37988395139948, max mem: 137.203125, count: 55735"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 25,
            "unit": "median segment_count",
            "extra": "avg segment_count: 24.926060823539967, max segment_count: 39.0, count: 55735"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.275363,
            "unit": "median cpu",
            "extra": "avg cpu: 9.346277576625214, max cpu: 28.346458, count: 111470"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 180.15234375,
            "unit": "median mem",
            "extra": "avg mem: 162.18272783596484, max mem: 184.01171875, count: 111470"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.88621,
            "unit": "median cpu",
            "extra": "avg cpu: 12.051544141495908, max cpu: 32.40116, count: 55735"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 173.94140625,
            "unit": "median mem",
            "extra": "avg mem: 170.89538398784427, max mem: 174.81640625, count: 55735"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "035982deb7c1e630e0d8a13e4ca1910b979b08f6",
          "message": "ci: add logical-replication-merge stressgres suite to cover FSM race (#4935) (#5072)\n\nRe-opened from #5068 (originally from a fork, where CI could not access\nworkflow secrets — see\nhttps://github.com/paradedb/paradedb/actions/runs/25830120431/job/75892835684?pr=5068).\nSame intent, from a branch in this repo, with the FSM-race suite landing\nas its own file rather than folded into `logical-replication.toml`.\n\n## Changes\n\n- `stressgres/suites/logical-replication-merge.toml` (new): standalone\nsuite that reliably reproduces the FSM race from #4935 (fixed in #5067).\nLogical-replication subscriber with aggressive autovacuum (`naptime=1s`,\n`threshold=50`), small `layer_sizes = '10kb, 100kb, 1mb, 100mb'`,\nmultiple concurrent BM25 readers, and sustained UPDATE/INSERT/DELETE\ntraffic on the publisher. The key difference from\n`logical-replication.toml` is the writer: `message = message || ' ' ||\ntxid_current()` grows each row's terms unbounded, generating ~10× more\nmerge/GC pressure and reliably opening the race window — folding the\nsame churn into `logical-replication.toml` (which strips-then-appends,\nkeeping row size constant) did not reproduce the bug.\n- `.github/workflows/benchmark-pg_search-stressgres.yml`:\n- Comment out single-server, bulk-updates, wide-table, and\nbackground-merge so CI focuses on the two replication suites while we\niterate. To be re-enabled before final merge.\n- Run `logical-replication-merge.toml` **before**\n`logical-replication.toml`.\n\n`stressgres/suites/logical-replication.toml` is unchanged from `main`.\n\n## Expected behavior\n\n- Without #5067: SIGSEGV or `SegmentMetaEntryHeader: UnexpectedEnd`\nwithin minutes.\n- With #5067: runs the full duration without errors.\n\n## Follow-ups\n\n- Re-enable the four commented-out suites before final merge.\n- Antithesis wiring for this suite belongs in `paradedb-enterprise` next\nto the existing `physical-logical-replication` driver, since the OSS\nAntithesis manifest only stands up a single paradedb cluster.\n\nRef: #4935\nRelated: #5067\nSupersedes: #5068",
          "timestamp": "2026-05-13T19:51:05-04:00",
          "tree_id": "c71af69d7df60d54d1631876f7e3c7af0782c3e3",
          "url": "https://github.com/paradedb/paradedb/commit/035982deb7c1e630e0d8a13e4ca1910b979b08f6"
        },
        "date": 1778720138419,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.622696,
            "unit": "median cpu",
            "extra": "avg cpu: 19.943554359380567, max cpu: 46.60194, count: 55494"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 178.7109375,
            "unit": "median mem",
            "extra": "avg mem: 176.6471857458689, max mem: 178.921875, count: 55494"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 7.6182773813645, max cpu: 28.180038, count: 55494"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 120.703125,
            "unit": "median mem",
            "extra": "avg mem: 119.438243816336, max mem: 120.875, count: 55494"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 6.127808300893938, max cpu: 18.713451, count: 55494"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 170.19921875,
            "unit": "median mem",
            "extra": "avg mem: 144.88610987223845, max mem: 178.91015625, count: 55494"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 16540,
            "unit": "median block_count",
            "extra": "avg block_count: 16928.26085702959, max block_count: 31620.0, count: 55494"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.065397538131852, max cpu: 4.6647234, count: 55494"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 108.65625,
            "unit": "median mem",
            "extra": "avg mem: 95.35415068802483, max mem: 136.9140625, count: 55494"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 26,
            "unit": "median segment_count",
            "extra": "avg segment_count: 25.568079432010666, max segment_count: 37.0, count: 55494"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.257474,
            "unit": "median cpu",
            "extra": "avg cpu: 9.099141056624307, max cpu: 28.318584, count: 110988"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 182.5546875,
            "unit": "median mem",
            "extra": "avg mem: 164.48018285901404, max mem: 184.61328125, count: 110988"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.88621,
            "unit": "median cpu",
            "extra": "avg cpu: 12.400360963001708, max cpu: 27.934044, count: 55494"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 173.765625,
            "unit": "median mem",
            "extra": "avg mem: 171.35421608079253, max mem: 174.75390625, count: 55494"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "36623265+daniel3303@users.noreply.github.com",
            "name": "Daniel Oliveira",
            "username": "daniel3303"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "b9c06c5f75ca94cf815eb57c71c46180e84b8038",
          "message": "fix(mlt): quote key_field identifier in internal SPI lookup (#5078)\n\n## Summary\n\n- `pdb.more_like_this(key_value)` raises `ERROR: column \"id\" does not\nexist` whenever the index's `key_field` column is a mixed-case\nPostgreSQL identifier (e.g. `\"Id\"`, `\"DocumentId\"`). Direct `@@@`-on-LHS\nsearches (`\"Content\" @@@ 'foo'`) are unaffected because they don't go\nthrough the internal SPI lookup. Repro in #5065.\n- Root cause: `pg_search/src/query/more_like_this.rs:152-157` builds the\nSPI `SELECT * FROM <ns>.<rel> WHERE <key_field> = $1` with `<ns>` and\n`<rel>` already routed through `pgrx::spi::quote_identifier`, but\ninterpolates `<key_field>` verbatim via `Display`. PostgreSQL folds the\nunquoted reference to lowercase, so a column named `\"Id\"` is looked up\nas `id` and the SPI call fails before MLT ever runs.\n- Fix: send the key field through\n`pgrx::spi::quote_identifier(key_field_name.root())`, matching how the\nnamespace and relation names are already quoted on the lines immediately\nabove. `.root()` strips the JSON sub-path (`key_field` is always a\ntop-level column).\n\n## Scope\n\nThe linked issue also lists JSON `term` filters (`@@@\n'{\"term\":{\"field\":\"Category\",…}}'::jsonb`) as failing on mixed-case\ncolumns. That path does **not** go through SPI — `term()` in\n`pg_search/src/query/pdb_query.rs:792` resolves the field via\n`schema.search_field(field.root())`, a pure Tantivy schema lookup — so\nit isn't fixed here and I couldn't find a corresponding\nunquoted-identifier hazard. If it reproduces on `0.23.x` it's a separate\nbug; tracking it on its own issue is cleaner than bundling a speculative\nfix.\n\n## Test plan\n\n- [x] `cargo test -p tests --test mlt --\nmlt_mixed_case_key_field_issue5065` — new regression test: `\"Id\"` /\n`\"Content\"` table, `key_field='Id'`, asserts `pdb.more_like_this(1)`\nreturns rows. Fails on `main` with `column \"id\" does not exist`, passes\nwith this change.\n- [x] `cargo test -p tests --test mlt` — existing\n`mlt_enables_scoring_issue1747`, `mlt_datetime_key`,\n`mlt_scoring_nested` still pass.\n- [x] `cargo pgrx regress -p pg_search --auto -- pg18 more_like_this` —\ngolden output unchanged (`quote_identifier(\"id\")` is a no-op for\nalready-lowercase identifiers).\n- [x] Manual repro from #5065 (`CREATE TABLE items (\"Id\" int primary\nkey, \"Content\" text); … pdb.more_like_this(1)`) returns rows instead of\nerroring.\n\nCloses #5065.",
          "timestamp": "2026-05-14T14:45:18-04:00",
          "tree_id": "812b4a66ffce7bc074cb919986f1de3b6474813f",
          "url": "https://github.com/paradedb/paradedb/commit/b9c06c5f75ca94cf815eb57c71c46180e84b8038"
        },
        "date": 1778788207494,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.60465,
            "unit": "median cpu",
            "extra": "avg cpu: 19.839973004471233, max cpu: 46.28737, count: 55509"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 178.89453125,
            "unit": "median mem",
            "extra": "avg mem: 176.8456074549848, max mem: 179.08984375, count: 55509"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 7.708383136090712, max cpu: 28.042841, count: 55509"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 119.984375,
            "unit": "median mem",
            "extra": "avg mem: 118.67777234040877, max mem: 120.1484375, count: 55509"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 6.1576999264611585, max cpu: 18.731707, count: 55509"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 172.18359375,
            "unit": "median mem",
            "extra": "avg mem: 146.52505400306708, max mem: 180.71875, count: 55509"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 16548,
            "unit": "median block_count",
            "extra": "avg block_count: 16857.340593417284, max block_count: 31551.0, count: 55509"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.267633514426484, max cpu: 4.6647234, count: 55509"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 109.25,
            "unit": "median mem",
            "extra": "avg mem: 95.80017514051775, max mem: 136.75390625, count: 55509"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 25,
            "unit": "median segment_count",
            "extra": "avg segment_count: 25.42991226647931, max segment_count: 37.0, count: 55509"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.257474,
            "unit": "median cpu",
            "extra": "avg cpu: 9.070541565048964, max cpu: 32.495163, count: 111018"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 181.015625,
            "unit": "median mem",
            "extra": "avg mem: 163.55263184376633, max mem: 184.3828125, count: 111018"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.88621,
            "unit": "median cpu",
            "extra": "avg cpu: 12.754426643546875, max cpu: 27.745665, count: 55509"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 173.9375,
            "unit": "median mem",
            "extra": "avg mem: 171.2731489769677, max mem: 174.74609375, count: 55509"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "balavignesh449@gmail.com",
            "name": "S Bala Vignesh",
            "username": "SBALAVIGNESH123"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "d0039460372b22539b33262920b0194c694b7205",
          "message": "fix: use fixed-length updates in stressgres to avoid TOAST and expose FSM race (#5080)\n\nThe old UPDATE pattern in the logical-replication stressgres suite\nappended txid_current() to the message column every iteration, growing\nit past the TOAST threshold (~2KB). This caused the suite to hit the\nunrelated TOAST visibility race (#5076) before the FSM segment metadata\nrace (#4935) could surface.\n\nChanged to fixed-length updates that keep the first search term and\nappend a small txid-derived number, staying well under the TOAST\nthreshold. This way the suite can run long enough to exercise the FSM\npath.\n\nRelated: #5067 (FSM race fix), #5076 (TOAST bug)",
          "timestamp": "2026-05-14T14:57:31-04:00",
          "tree_id": "96511f40645bb3416046b0914f9758c60c159a20",
          "url": "https://github.com/paradedb/paradedb/commit/d0039460372b22539b33262920b0194c694b7205"
        },
        "date": 1778789012094,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.658894,
            "unit": "median cpu",
            "extra": "avg cpu: 19.92473269991633, max cpu: 46.466602, count: 55653"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 178.8125,
            "unit": "median mem",
            "extra": "avg mem: 176.74653524124935, max mem: 179.17578125, count: 55653"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 7.603047056980902, max cpu: 37.75811, count: 55653"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 120.12890625,
            "unit": "median mem",
            "extra": "avg mem: 118.90898275362514, max mem: 120.2265625, count: 55653"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 6.151571422164445, max cpu: 18.640776, count: 55653"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 166.65625,
            "unit": "median mem",
            "extra": "avg mem: 146.4940329193844, max mem: 180.54296875, count: 55653"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 16606,
            "unit": "median block_count",
            "extra": "avg block_count: 16910.34968465312, max block_count: 31373.0, count: 55653"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.673807,
            "unit": "median cpu",
            "extra": "avg cpu: 4.62218861259927, max cpu: 4.7477746, count: 55653"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 105.7265625,
            "unit": "median mem",
            "extra": "avg mem: 95.12719250590713, max mem: 134.92578125, count: 55653"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 25,
            "unit": "median segment_count",
            "extra": "avg segment_count: 24.73571954791296, max segment_count: 35.0, count: 55653"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.284333,
            "unit": "median cpu",
            "extra": "avg cpu: 9.130002325770091, max cpu: 37.75811, count: 111306"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 181.5,
            "unit": "median mem",
            "extra": "avg mem: 163.34550578753615, max mem: 184.5546875, count: 111306"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.9265,
            "unit": "median cpu",
            "extra": "avg cpu: 13.06273444896406, max cpu: 28.042841, count: 55653"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 174.1796875,
            "unit": "median mem",
            "extra": "avg mem: 171.1924458952123, max mem: 174.73828125, count: 55653"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "balavignesh449@gmail.com",
            "name": "S Bala Vignesh",
            "username": "SBALAVIGNESH123"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "b597b183d25a561cf8c81d81decc7f07b7866e55",
          "message": "fix: hold header shared lock during LinkedItemList iteration to prevent FSM race (#4935) (#5067)\n\n## Problem\n\nLinkedItemList read-only iteration methods (list(), is_empty(),\nor_each(), lookup()/lookup_ex()) were releasing the header shared lock\nbefore iterating through the linked list blocks. This allowed\nAtomicGuard::commit() to:\n\n1. Swap the header pointer to a new list\n2. Immediately recycle old blocks to the FSM with\nReadNextFullTransactionId()\n3. The next transaction pops a recycled block via RBM_ZERO_AND_LOCK,\nzeroing it\n4. A concurrent reader still traversing the old list hits the\nzeroed/repurposed block\n5. **SIGSEGV** or SegmentMetaEntryHeader: UnexpectedEnd deserialization\nerror\n\nThis manifests as periodic crashes and durable index corruption under\nsustained write traffic, particularly on logical replication subscribers\nat high apply rates (~395 commits/sec). Correlates strongly with\nautovacuum events that trigger garbage_collect_index().\n\n## Root Cause\n\nThe race window exists because or_each(), list(), is_empty(), and\nlookup_ex() call get_start_blockno() which acquires a shared lock on the\nheader, reads start_blockno, then immediately releases the header lock\nwhen exchanging to the first data block. After that point, \u0007tomically()\ncan take an exclusive header lock and proceed with the swap+recycle\nwhile the reader is deep in the old list.\n\n**This was already a known pattern in the codebase** — emove_item() and\nupdate_item() in the same file both hold the header shared lock for\ntheir entire operation with this comment:\n\n\\\\\\\rust\n// Acquire and hold a shared lock on the header for the entire\noperation, preventing the\n// list from being swapped out from under us by atomically between our\nread locks and\n// our write locks.\nlet header_lock = self.bman.get_buffer(self.header_blockno);\n\\\\\\\n\nThe read-only methods simply weren't given the same treatment.\n\n## Fix\n\nHold a shared lock on the header for the entire duration of iteration in\nall 4 methods, matching the existing emove_item()/update_item() pattern:\n\n- **list()** — hold header_lock from start to end of iteration\n- **is_empty()** — same\n- **\for_each()** — same\n- **lookup_ex()** — conditionally: only when \blockno is None (top-level\ncall). When \blockno is Some, the caller ( emove_item/update_item)\nalready holds the header lock\n\nRead start_blockno directly from the already-held header_lock instead of\ncalling get_start_blockno(), avoiding a double shared-lock acquisition\non the same block (which would trigger a panic under the \block_tracker\ndebug feature).\n\n## Why This Is Safe\n\n- **No deadlock**: Header block is always locked first, content blocks\nin ascending order — consistent lock ordering\n- **No reader-reader blocking**: Multiple readers hold shared locks\nconcurrently (shared locks are compatible)\n- **Writer waits for readers**: \u0007tomically() takes an exclusive header\nlock, which blocks until all shared locks are released — correct\nserialization\n- **Minimal performance impact**: The header lock was already acquired;\nwe just hold it slightly longer\n\n## Verification\n\n- \rustfmt --check passes\n- Pattern matches the proven emove_item()/update_item() implementation\nin the same file\n- Full cargo check requires pgrx setup (PostgreSQL extension); the\nchange is limited to lock lifetime management with no new APIs\n\nCloses #4935",
          "timestamp": "2026-05-14T15:00:05-04:00",
          "tree_id": "46e474245958de09c21d0198195343d8a87fb72d",
          "url": "https://github.com/paradedb/paradedb/commit/b597b183d25a561cf8c81d81decc7f07b7866e55"
        },
        "date": 1778789096616,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.640776,
            "unit": "median cpu",
            "extra": "avg cpu: 19.92716142307539, max cpu: 46.421665, count: 55579"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 176.3203125,
            "unit": "median mem",
            "extra": "avg mem: 165.05520998263734, max mem: 179.4140625, count: 55579"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 7.6922813110711905, max cpu: 28.015566, count: 55579"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 119.9453125,
            "unit": "median mem",
            "extra": "avg mem: 118.76979474036956, max mem: 120.0546875, count: 55579"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 6.174320958567713, max cpu: 18.695229, count: 55579"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 175.34375,
            "unit": "median mem",
            "extra": "avg mem: 147.50089280292465, max mem: 180.97265625, count: 55579"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 16521,
            "unit": "median block_count",
            "extra": "avg block_count: 16938.73738282445, max block_count: 31663.0, count: 55579"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.548184726406851, max cpu: 4.738401, count: 55579"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 111.12109375,
            "unit": "median mem",
            "extra": "avg mem: 96.34328613325177, max mem: 136.9375, count: 55579"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 25,
            "unit": "median segment_count",
            "extra": "avg segment_count: 25.459094262221342, max segment_count: 39.0, count: 55579"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.266409,
            "unit": "median cpu",
            "extra": "avg cpu: 9.072281067302626, max cpu: 28.070175, count: 111158"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 182,
            "unit": "median mem",
            "extra": "avg mem: 164.3412037229664, max mem: 183.41796875, count: 111158"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.899614,
            "unit": "median cpu",
            "extra": "avg cpu: 12.655229674071151, max cpu: 27.906979, count: 55579"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 175.1328125,
            "unit": "median mem",
            "extra": "avg mem: 172.30263221889112, max mem: 175.6640625, count: 55579"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "652b0952a7f3be58ea4b9fa80a863426cf03185c",
          "message": "chore(stressgres): remove dormant suites not wired into CI (#5085)\n\n## Summary\n\n- Removes 8 Stressgres suites that have not been touched since\nStressgres was added (#3821), are not referenced by any CI workflow or\ndriver script, and use the legacy `[setup_primary]` schema rather than\nthe current `[[server]]` schema.\n- No CI workflow changes; the 6 suites currently exercised by\n`benchmark-pg_search-stressgres.yml` (`single-server`, `bulk-updates`,\n`wide-table`, `background-merge`, `logical-replication`,\n`logical-replication-merge`) and `vanilla-postgres.toml` (used by the\nantithesis singleton driver) are kept.\n\n### Removed\n- `large-inserts.toml`\n- `lr.toml` — predecessor of `logical-replication.toml`\n- `lr-graphable.toml`\n- `lr-large-inserts.toml`\n- `lr-no-pg_search.toml`\n- `many-updates.toml` — conceptually useful (MVCC correctness with\n`assert(count, expected)` under concurrency), but on legacy schema; if\nwe want this coverage in CI it should be a deliberate port, not a\nrevival of dead config\n- `read-write.toml` — contained placeholder `<password>` literal, not\nCI-runnable\n- `topk-crash.toml`\n\n### Why now\nCompanion to #5080 (which adjusts `logical-replication.toml` to expose\nthe FSM race instead of being masked by the TOAST bug already covered by\n`logical-replication-merge.toml`). Each CI suite should own a distinct\nrepro; dormant files muddy that mapping.\n\n## Test plan\n- [x] `benchmark-pg_search-stressgres` workflow still kicks off for all\n6 in-CI suites\n- [x] Antithesis `singleton_driver_vanilla-postgres.sh` still resolves\nits suite path",
          "timestamp": "2026-05-14T15:45:37-04:00",
          "tree_id": "090e72f7bbb9817642fb0c7495c2b12e0eed2fe7",
          "url": "https://github.com/paradedb/paradedb/commit/652b0952a7f3be58ea4b9fa80a863426cf03185c"
        },
        "date": 1778791949003,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.640776,
            "unit": "median cpu",
            "extra": "avg cpu: 19.819339318505794, max cpu: 46.466602, count: 55633"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 173.3515625,
            "unit": "median mem",
            "extra": "avg mem: 159.07897796721372, max mem: 179.1484375, count: 55633"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 7.606618452656046, max cpu: 37.029896, count: 55633"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 121.48046875,
            "unit": "median mem",
            "extra": "avg mem: 120.1836537835008, max mem: 121.74609375, count: 55633"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 6.0862133570471855, max cpu: 18.622696, count: 55633"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 165.2890625,
            "unit": "median mem",
            "extra": "avg mem: 144.41604880140832, max mem: 179.96875, count: 55633"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 16589,
            "unit": "median block_count",
            "extra": "avg block_count: 16942.29378246724, max block_count: 31447.0, count: 55633"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 4.5444323569383585, max cpu: 4.6966734, count: 55633"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 105.203125,
            "unit": "median mem",
            "extra": "avg mem: 95.20751362444503, max mem: 137.08984375, count: 55633"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 25,
            "unit": "median segment_count",
            "extra": "avg segment_count: 25.466521668793703, max segment_count: 37.0, count: 55633"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.266409,
            "unit": "median cpu",
            "extra": "avg cpu: 9.107434647484109, max cpu: 37.029896, count: 111266"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 180.98046875,
            "unit": "median mem",
            "extra": "avg mem: 162.95870660596003, max mem: 182.4296875, count: 111266"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.913043,
            "unit": "median cpu",
            "extra": "avg cpu: 12.925844155510616, max cpu: 23.483368, count: 55633"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 174.65625,
            "unit": "median mem",
            "extra": "avg mem: 172.22004180859383, max mem: 175.1640625, count: 55633"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ming.ying.nyc@gmail.com",
            "name": "Ming",
            "username": "rebasedming"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "21bdc9a753582039cc25a38cb07111026c0fd378",
          "message": "feat: Crash recovery via WAL (#4901)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nPorts WAL integration over to community, which gives `pg_search` crash\nrecovery.\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2026-05-14T14:59:53-07:00",
          "tree_id": "7580b5f2bfcc1a95ed1b62c4628f2ec6129b7c00",
          "url": "https://github.com/paradedb/paradedb/commit/21bdc9a753582039cc25a38cb07111026c0fd378"
        },
        "date": 1778799996513,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.640776,
            "unit": "median cpu",
            "extra": "avg cpu: 19.951478721354327, max cpu: 50.96525, count: 55542"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 178.4609375,
            "unit": "median mem",
            "extra": "avg mem: 165.14286296012477, max mem: 179.5234375, count: 55542"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 7.683469632998179, max cpu: 28.042841, count: 55542"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 120.80859375,
            "unit": "median mem",
            "extra": "avg mem: 119.60547296977963, max mem: 120.9765625, count: 55542"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 6.143382541394475, max cpu: 18.622696, count: 55542"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 172.0625,
            "unit": "median mem",
            "extra": "avg mem: 147.0359837487847, max mem: 180.4140625, count: 55542"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 16264,
            "unit": "median block_count",
            "extra": "avg block_count: 16647.66211155522, max block_count: 31064.0, count: 55542"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.604013119520808, max cpu: 4.669261, count: 55542"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 110.36328125,
            "unit": "median mem",
            "extra": "avg mem: 96.42721562342462, max mem: 137.875, count: 55542"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 25,
            "unit": "median segment_count",
            "extra": "avg segment_count: 25.46896042634403, max segment_count: 37.0, count: 55542"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.275363,
            "unit": "median cpu",
            "extra": "avg cpu: 9.334446254386119, max cpu: 32.589718, count: 111084"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 182.30078125,
            "unit": "median mem",
            "extra": "avg mem: 164.74768545087503, max mem: 184.43359375, count: 111084"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.88621,
            "unit": "median cpu",
            "extra": "avg cpu: 12.461451751707411, max cpu: 23.59882, count: 55542"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 174.39453125,
            "unit": "median mem",
            "extra": "avg mem: 171.928306647447, max mem: 175.109375, count: 55542"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "d99908a1f58307567dd34698345b3fc836be6135",
          "message": "feat(antithesis): add singleton drivers for logical-replication suites (#5087)\n\n## Summary\n\nAdds OSS Antithesis singleton drivers for the two CI\n`logical-replication` suites that previously had no singleton\n(`single-server`, `bulk-updates`, `wide-table`, `background-merge`, and\n`vanilla-postgres` already had one).\n\nMirrors the enterprise pattern used for `physical-logical-replication`:\n- A **vanilla Postgres 18** publisher pod (with `wal_level=logical`)\nthat lives outside the CNPG cluster, reflecting real-world\nlogical-replication topologies where the upstream primary is not under\nour control.\n- Subscriber points at `paradedb-rw` (the CNPG primary, which has\n`pg_search`).\n\n## Changes\n- `docker/manifests/antithesis-paradedb.yaml` — add\n`logical-replication-publisher` Service + Deployment (vanilla Postgres\n18 with `wal_level=logical`), reusing the existing `paradedb-superuser`\nsecret.\n- `stressgres/suites/logical-replication.toml`,\n`stressgres/suites/logical-replication-merge.toml` — drop `CREATE\nEXTENSION pg_search` from the **Publisher** setup. Only the Subscriber\nuses `pg_search`; the line was cosmetic and incompatible with a vanilla\nPostgres publisher (the line in the Subscriber setup is unchanged).\n-\n`stressgres/suites/antithesis/singleton_driver_logical-replication.sh`,\n`singleton_driver_logical-replication-merge.sh` — new drivers that\nperform per-block `sed -z` rewrites of the `[server.style.Automatic]`\nblocks into `[server.style.With]` connection strings (Publisher →\n`logical-replication-publisher:5432`, Subscriber → `paradedb-rw:5432`).\n- `.github/workflows/antithesis-trigger-test-run.yml` — add\n`logical-replication-publisher` to\n`container_faults_stop_exclusion_patterns` and\n`container_faults_kill_exclusion_patterns`, matching enterprise. Network\nfaults to/from the publisher are intentionally still injected.\n\n## Why\nWithout these, the FSM race repro in `logical-replication-merge.toml`\n(issue #4935, fixed by #5067) and the broader logical-replication\ncoverage in `logical-replication.toml` were running in\n`benchmark-pg_search-stressgres` but had no Antithesis fault-injection\nequivalent — that's the half of the matrix where the bugs originally\nsurfaced.\n\n## Test plan\n- [x] Antithesis trigger workflow picks up both new singleton drivers\nfrom `/opt/antithesis/test/v1/quickstart/`\n- [x] Publisher pod (`logical-replication-publisher`) starts with\n`wal_level=logical` and is reachable from the stressgres-runner pod\n- [x] Subscriber's `CREATE SUBSCRIPTION ... CONNECTION\n'@Publisher_CONNSTR@'` resolves to the publisher pod after the `sed`\nrewrite\n- [x] `logical-replication-merge.toml` still reproduces the FSM race\nwhen run against a build without #5067\n- [x] `benchmark-pg_search-stressgres` (local Stressgres, not\nAntithesis) still runs both suites unchanged",
          "timestamp": "2026-05-14T18:54:16-04:00",
          "tree_id": "8c7a6dab334db43248e88e7a4ebc75fd5840e446",
          "url": "https://github.com/paradedb/paradedb/commit/d99908a1f58307567dd34698345b3fc836be6135"
        },
        "date": 1778803164420,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.640776,
            "unit": "median cpu",
            "extra": "avg cpu: 19.89719151371851, max cpu: 46.28737, count: 55578"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 178.62890625,
            "unit": "median mem",
            "extra": "avg mem: 167.75874088274588, max mem: 179.49609375, count: 55578"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 7.61307361400379, max cpu: 37.944664, count: 55578"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 119.90234375,
            "unit": "median mem",
            "extra": "avg mem: 118.72717776705711, max mem: 120.0859375, count: 55578"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 6.332112715723434, max cpu: 23.346306, count: 55578"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 172.27734375,
            "unit": "median mem",
            "extra": "avg mem: 146.5551397782171, max mem: 180.45703125, count: 55578"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 16308,
            "unit": "median block_count",
            "extra": "avg block_count: 16644.283187592213, max block_count: 31107.0, count: 55578"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.431624483617591, max cpu: 4.678363, count: 55578"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 108.77734375,
            "unit": "median mem",
            "extra": "avg mem: 95.55844102263936, max mem: 138.03515625, count: 55578"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 25,
            "unit": "median segment_count",
            "extra": "avg segment_count: 24.590359494764115, max segment_count: 36.0, count: 55578"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 9.275363,
            "unit": "median cpu",
            "extra": "avg cpu: 9.190228856114038, max cpu: 42.687748, count: 111156"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 182.2265625,
            "unit": "median mem",
            "extra": "avg mem: 163.91654113925475, max mem: 183.66796875, count: 111156"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.913043,
            "unit": "median cpu",
            "extra": "avg cpu: 12.697247457224524, max cpu: 23.529411, count: 55578"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 174.390625,
            "unit": "median mem",
            "extra": "avg mem: 171.58082826725501, max mem: 175.234375, count: 55578"
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
        "date": 1778655759449,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - tps",
            "value": 592.9952127613484,
            "unit": "median tps",
            "extra": "avg tps: 599.9854515905942, max tps: 733.6624441863133, count: 53875"
          },
          {
            "name": "Index Only Scan - Subscriber - tps",
            "value": 602.6823879149085,
            "unit": "median tps",
            "extra": "avg tps: 605.16276426073, max tps: 814.9468060141741, count: 53875"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - tps",
            "value": 86.47488620990502,
            "unit": "median tps",
            "extra": "avg tps: 86.58913915551314, max tps: 93.4772611664817, count: 53875"
          },
          {
            "name": "Top K - Subscriber - tps",
            "value": 245.78112942699045,
            "unit": "median tps",
            "extra": "avg tps: 247.84743236606508, max tps: 553.6909740586984, count: 107750"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "59696464+saadtajwar@users.noreply.github.com",
            "name": "Saad Tajwar",
            "username": "saadtajwar"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "bc20c3dd4b36a13dbcc74d03499f966a4dc93fe6",
          "message": "refactor: Deduplicate deferred materialization request partitioning between visibility and lookup (#4903)\n\n# Ticket(s) Closed\n- Closes https://github.com/paradedb/paradedb/issues/4568\n\n## What\nDeduplicated the segment-grouping/materialization loop shared between\n`materialize_deferred_ctid()` in `visibility_filter.rs` and\n`materialize_deferred_column()` in `tantivy_lookup_exec.rs`.\n\n## Why\nBoth functions implemented the same pattern of partitioning packed doc\naddresses by segment ordinal, batch-reading via `FFHelper`, and writing\nresults back in row order. This duplication made the two paths harder to\nkeep consistent and made future optimization work more tedious.\n\n## How\n\n### Shared helper in `fast_fields_helper.rs`\n- Added `for_each_segment`: partitions an iterator of `(row_index,\npacked_doc_address)` pairs into per-segment buckets and invokes a\ncaller-supplied closure once per non-empty segment, in segment-ordinal\norder. Backed by a `Vec<Vec<(usize, DocId)>>` indexed by segment ordinal\n(dense in practice; cheaper than a hash map for typical segment counts).\n- Added `FFHelper::num_segments()` so callers can size the bucket vector\nwithout reaching into private fields.\n\n### `materialize_deferred_ctid` (visibility_filter.rs)\n- Replaced the manual sort + partition + per-segment slice loop with a\nsingle `for_each_segment` call.\n- Kept `DeferredCtidMaterializationState` for buffer reuse across calls,\nbut removed its now-unused `requests` field; the per-segment\npartitioning lives inside `for_each_segment`.\n- Removed the TODO comment that flagged this duplication.\n\n### `materialize_deferred_column` (tantivy_lookup_exec.rs)\nFunction body shrank from ~160 lines to ~50 by extracting three\nsingle-responsibility helpers:\n- `resolve_doc_addresses_to_term_ords` — resolves State 0 (packed doc\naddresses) into per-segment `(row_index, Option<TermOrdinal>)` pairs via\n`for_each_segment`.\n- `extract_term_ords` — parses State 1 (pre-resolved `(segment_ord,\nterm_ord)` pairs from the dense union's `StructArray` child) into the\nsame per-segment shape.\n- `decode_term_ordinals` — takes the merged per-segment ordinals and\nperforms the bulk dictionary lookup once per segment, recording\npositions for the final `interleave`.\n\nState 0 and State 1 are now merged into a single `Vec<Vec<(row_index,\nOption<TermOrdinal>)>>` indexed by segment ordinal, then decoded in one\npass — previously each state was iterated and decoded separately,\nproducing two `segment_arrays` entries per segment touched by both. The\nfinal interleaved output is identical.\n\nReplaced the `(ff_index: usize, is_bytes: bool)` parameter pair with a\n`DeferredColumnKind { Text { ff_index }, Bytes { ff_index } }` enum to\nmake the `is_bytes && wrong-ff-type` mismatch unrepresentable.\n\n## Tests\nNo new tests; behavior is unchanged. Existing coverage exercises both\npaths end-to-end:\n- `pg_search/tests/pg_regress/sql/join_deferred_visibility.sql` —\n`materialize_deferred_ctid`.\n- `pg_search/tests/pg_regress/sql/segmented_topk.sql` plus joinscan\ntests — `materialize_deferred_column` (the segmented top-K rule is what\nproduces State 1 rows below `TantivyLookupExec`).\n\n---------\n\nCo-authored-by: Mithun Chicklore Yogendra <mithun.cy@gmail.com>",
          "timestamp": "2026-05-13T10:07:59-07:00",
          "tree_id": "cc9e05e63ed6052c202d00901357d2c5026923d0",
          "url": "https://github.com/paradedb/paradedb/commit/bc20c3dd4b36a13dbcc74d03499f966a4dc93fe6"
        },
        "date": 1778696622584,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - tps",
            "value": 589.5835190079488,
            "unit": "median tps",
            "extra": "avg tps: 592.222063350532, max tps: 751.5434824270345, count: 53864"
          },
          {
            "name": "Index Only Scan - Subscriber - tps",
            "value": 614.180501846881,
            "unit": "median tps",
            "extra": "avg tps: 616.8351362225699, max tps: 781.9285368142707, count: 53864"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - tps",
            "value": 86.47625113220047,
            "unit": "median tps",
            "extra": "avg tps: 86.5568882275466, max tps: 96.21274118752775, count: 53864"
          },
          {
            "name": "Top K - Subscriber - tps",
            "value": 248.92628403523338,
            "unit": "median tps",
            "extra": "avg tps: 248.70071167973938, max tps: 607.813053115814, count: 107728"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "stuhood@paradedb.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "713eaca82145388378043ddb2c750c41e9c632d9",
          "message": "chore: Revert #4905 (#5066)\n\nRevert #4905 due to performance regressions.",
          "timestamp": "2026-05-13T13:34:27-07:00",
          "tree_id": "2894402ba71bbb0b86d5990862fb2adba4fcbf49",
          "url": "https://github.com/paradedb/paradedb/commit/713eaca82145388378043ddb2c750c41e9c632d9"
        },
        "date": 1778708998715,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - tps",
            "value": 616.5307830808171,
            "unit": "median tps",
            "extra": "avg tps: 615.4809966784103, max tps: 814.5346801128584, count: 53907"
          },
          {
            "name": "Index Only Scan - Subscriber - tps",
            "value": 615.6438289316054,
            "unit": "median tps",
            "extra": "avg tps: 614.772182296459, max tps: 745.1116577139845, count: 53907"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - tps",
            "value": 91.95732843748326,
            "unit": "median tps",
            "extra": "avg tps: 91.98768962399424, max tps: 100.89440451319504, count: 53907"
          },
          {
            "name": "Top K - Subscriber - tps",
            "value": 279.8465144944984,
            "unit": "median tps",
            "extra": "avg tps: 275.08730031673963, max tps: 614.2211754831775, count: 107814"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "035982deb7c1e630e0d8a13e4ca1910b979b08f6",
          "message": "ci: add logical-replication-merge stressgres suite to cover FSM race (#4935) (#5072)\n\nRe-opened from #5068 (originally from a fork, where CI could not access\nworkflow secrets — see\nhttps://github.com/paradedb/paradedb/actions/runs/25830120431/job/75892835684?pr=5068).\nSame intent, from a branch in this repo, with the FSM-race suite landing\nas its own file rather than folded into `logical-replication.toml`.\n\n## Changes\n\n- `stressgres/suites/logical-replication-merge.toml` (new): standalone\nsuite that reliably reproduces the FSM race from #4935 (fixed in #5067).\nLogical-replication subscriber with aggressive autovacuum (`naptime=1s`,\n`threshold=50`), small `layer_sizes = '10kb, 100kb, 1mb, 100mb'`,\nmultiple concurrent BM25 readers, and sustained UPDATE/INSERT/DELETE\ntraffic on the publisher. The key difference from\n`logical-replication.toml` is the writer: `message = message || ' ' ||\ntxid_current()` grows each row's terms unbounded, generating ~10× more\nmerge/GC pressure and reliably opening the race window — folding the\nsame churn into `logical-replication.toml` (which strips-then-appends,\nkeeping row size constant) did not reproduce the bug.\n- `.github/workflows/benchmark-pg_search-stressgres.yml`:\n- Comment out single-server, bulk-updates, wide-table, and\nbackground-merge so CI focuses on the two replication suites while we\niterate. To be re-enabled before final merge.\n- Run `logical-replication-merge.toml` **before**\n`logical-replication.toml`.\n\n`stressgres/suites/logical-replication.toml` is unchanged from `main`.\n\n## Expected behavior\n\n- Without #5067: SIGSEGV or `SegmentMetaEntryHeader: UnexpectedEnd`\nwithin minutes.\n- With #5067: runs the full duration without errors.\n\n## Follow-ups\n\n- Re-enable the four commented-out suites before final merge.\n- Antithesis wiring for this suite belongs in `paradedb-enterprise` next\nto the existing `physical-logical-replication` driver, since the OSS\nAntithesis manifest only stands up a single paradedb cluster.\n\nRef: #4935\nRelated: #5067\nSupersedes: #5068",
          "timestamp": "2026-05-13T19:51:05-04:00",
          "tree_id": "c71af69d7df60d54d1631876f7e3c7af0782c3e3",
          "url": "https://github.com/paradedb/paradedb/commit/035982deb7c1e630e0d8a13e4ca1910b979b08f6"
        },
        "date": 1778720785526,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - tps",
            "value": 592.8699460421997,
            "unit": "median tps",
            "extra": "avg tps: 597.6943983725522, max tps: 753.4985881320205, count: 53853"
          },
          {
            "name": "Index Only Scan - Subscriber - tps",
            "value": 618.6260221339546,
            "unit": "median tps",
            "extra": "avg tps: 623.8986396159858, max tps: 793.4601323179406, count: 53853"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - tps",
            "value": 89.79938000918261,
            "unit": "median tps",
            "extra": "avg tps: 89.95463584675947, max tps: 97.08695683093998, count: 53853"
          },
          {
            "name": "Top K - Subscriber - tps",
            "value": 275.0943576662008,
            "unit": "median tps",
            "extra": "avg tps: 271.50947759560665, max tps: 557.714023582793, count: 107706"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "36623265+daniel3303@users.noreply.github.com",
            "name": "Daniel Oliveira",
            "username": "daniel3303"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "b9c06c5f75ca94cf815eb57c71c46180e84b8038",
          "message": "fix(mlt): quote key_field identifier in internal SPI lookup (#5078)\n\n## Summary\n\n- `pdb.more_like_this(key_value)` raises `ERROR: column \"id\" does not\nexist` whenever the index's `key_field` column is a mixed-case\nPostgreSQL identifier (e.g. `\"Id\"`, `\"DocumentId\"`). Direct `@@@`-on-LHS\nsearches (`\"Content\" @@@ 'foo'`) are unaffected because they don't go\nthrough the internal SPI lookup. Repro in #5065.\n- Root cause: `pg_search/src/query/more_like_this.rs:152-157` builds the\nSPI `SELECT * FROM <ns>.<rel> WHERE <key_field> = $1` with `<ns>` and\n`<rel>` already routed through `pgrx::spi::quote_identifier`, but\ninterpolates `<key_field>` verbatim via `Display`. PostgreSQL folds the\nunquoted reference to lowercase, so a column named `\"Id\"` is looked up\nas `id` and the SPI call fails before MLT ever runs.\n- Fix: send the key field through\n`pgrx::spi::quote_identifier(key_field_name.root())`, matching how the\nnamespace and relation names are already quoted on the lines immediately\nabove. `.root()` strips the JSON sub-path (`key_field` is always a\ntop-level column).\n\n## Scope\n\nThe linked issue also lists JSON `term` filters (`@@@\n'{\"term\":{\"field\":\"Category\",…}}'::jsonb`) as failing on mixed-case\ncolumns. That path does **not** go through SPI — `term()` in\n`pg_search/src/query/pdb_query.rs:792` resolves the field via\n`schema.search_field(field.root())`, a pure Tantivy schema lookup — so\nit isn't fixed here and I couldn't find a corresponding\nunquoted-identifier hazard. If it reproduces on `0.23.x` it's a separate\nbug; tracking it on its own issue is cleaner than bundling a speculative\nfix.\n\n## Test plan\n\n- [x] `cargo test -p tests --test mlt --\nmlt_mixed_case_key_field_issue5065` — new regression test: `\"Id\"` /\n`\"Content\"` table, `key_field='Id'`, asserts `pdb.more_like_this(1)`\nreturns rows. Fails on `main` with `column \"id\" does not exist`, passes\nwith this change.\n- [x] `cargo test -p tests --test mlt` — existing\n`mlt_enables_scoring_issue1747`, `mlt_datetime_key`,\n`mlt_scoring_nested` still pass.\n- [x] `cargo pgrx regress -p pg_search --auto -- pg18 more_like_this` —\ngolden output unchanged (`quote_identifier(\"id\")` is a no-op for\nalready-lowercase identifiers).\n- [x] Manual repro from #5065 (`CREATE TABLE items (\"Id\" int primary\nkey, \"Content\" text); … pdb.more_like_this(1)`) returns rows instead of\nerroring.\n\nCloses #5065.",
          "timestamp": "2026-05-14T14:45:18-04:00",
          "tree_id": "812b4a66ffce7bc074cb919986f1de3b6474813f",
          "url": "https://github.com/paradedb/paradedb/commit/b9c06c5f75ca94cf815eb57c71c46180e84b8038"
        },
        "date": 1778788855202,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - tps",
            "value": 606.9098362591045,
            "unit": "median tps",
            "extra": "avg tps: 608.3753253302652, max tps: 775.6821930967739, count: 53860"
          },
          {
            "name": "Index Only Scan - Subscriber - tps",
            "value": 634.7505314209125,
            "unit": "median tps",
            "extra": "avg tps: 637.0449470370172, max tps: 795.223894926151, count: 53860"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - tps",
            "value": 89.35731503719748,
            "unit": "median tps",
            "extra": "avg tps: 89.58344241850413, max tps: 97.26909767143509, count: 53860"
          },
          {
            "name": "Top K - Subscriber - tps",
            "value": 275.2500801879538,
            "unit": "median tps",
            "extra": "avg tps: 267.14663182869276, max tps: 578.5988609925361, count: 107720"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "balavignesh449@gmail.com",
            "name": "S Bala Vignesh",
            "username": "SBALAVIGNESH123"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "d0039460372b22539b33262920b0194c694b7205",
          "message": "fix: use fixed-length updates in stressgres to avoid TOAST and expose FSM race (#5080)\n\nThe old UPDATE pattern in the logical-replication stressgres suite\nappended txid_current() to the message column every iteration, growing\nit past the TOAST threshold (~2KB). This caused the suite to hit the\nunrelated TOAST visibility race (#5076) before the FSM segment metadata\nrace (#4935) could surface.\n\nChanged to fixed-length updates that keep the first search term and\nappend a small txid-derived number, staying well under the TOAST\nthreshold. This way the suite can run long enough to exercise the FSM\npath.\n\nRelated: #5067 (FSM race fix), #5076 (TOAST bug)",
          "timestamp": "2026-05-14T14:57:31-04:00",
          "tree_id": "96511f40645bb3416046b0914f9758c60c159a20",
          "url": "https://github.com/paradedb/paradedb/commit/d0039460372b22539b33262920b0194c694b7205"
        },
        "date": 1778789659893,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - tps",
            "value": 562.9798481147266,
            "unit": "median tps",
            "extra": "avg tps: 566.0605305605943, max tps: 818.6717347385066, count: 53674"
          },
          {
            "name": "Index Only Scan - Subscriber - tps",
            "value": 586.4977230885117,
            "unit": "median tps",
            "extra": "avg tps: 589.9401557030228, max tps: 878.0936008857639, count: 53674"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - tps",
            "value": 95.07354478395298,
            "unit": "median tps",
            "extra": "avg tps: 95.09835366446636, max tps: 97.63837876527559, count: 53674"
          },
          {
            "name": "Top K - Subscriber - tps",
            "value": 262.62798872314795,
            "unit": "median tps",
            "extra": "avg tps: 259.6961209297721, max tps: 595.8245686711748, count: 107348"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "balavignesh449@gmail.com",
            "name": "S Bala Vignesh",
            "username": "SBALAVIGNESH123"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "b597b183d25a561cf8c81d81decc7f07b7866e55",
          "message": "fix: hold header shared lock during LinkedItemList iteration to prevent FSM race (#4935) (#5067)\n\n## Problem\n\nLinkedItemList read-only iteration methods (list(), is_empty(),\nor_each(), lookup()/lookup_ex()) were releasing the header shared lock\nbefore iterating through the linked list blocks. This allowed\nAtomicGuard::commit() to:\n\n1. Swap the header pointer to a new list\n2. Immediately recycle old blocks to the FSM with\nReadNextFullTransactionId()\n3. The next transaction pops a recycled block via RBM_ZERO_AND_LOCK,\nzeroing it\n4. A concurrent reader still traversing the old list hits the\nzeroed/repurposed block\n5. **SIGSEGV** or SegmentMetaEntryHeader: UnexpectedEnd deserialization\nerror\n\nThis manifests as periodic crashes and durable index corruption under\nsustained write traffic, particularly on logical replication subscribers\nat high apply rates (~395 commits/sec). Correlates strongly with\nautovacuum events that trigger garbage_collect_index().\n\n## Root Cause\n\nThe race window exists because or_each(), list(), is_empty(), and\nlookup_ex() call get_start_blockno() which acquires a shared lock on the\nheader, reads start_blockno, then immediately releases the header lock\nwhen exchanging to the first data block. After that point, \u0007tomically()\ncan take an exclusive header lock and proceed with the swap+recycle\nwhile the reader is deep in the old list.\n\n**This was already a known pattern in the codebase** — emove_item() and\nupdate_item() in the same file both hold the header shared lock for\ntheir entire operation with this comment:\n\n\\\\\\\rust\n// Acquire and hold a shared lock on the header for the entire\noperation, preventing the\n// list from being swapped out from under us by atomically between our\nread locks and\n// our write locks.\nlet header_lock = self.bman.get_buffer(self.header_blockno);\n\\\\\\\n\nThe read-only methods simply weren't given the same treatment.\n\n## Fix\n\nHold a shared lock on the header for the entire duration of iteration in\nall 4 methods, matching the existing emove_item()/update_item() pattern:\n\n- **list()** — hold header_lock from start to end of iteration\n- **is_empty()** — same\n- **\for_each()** — same\n- **lookup_ex()** — conditionally: only when \blockno is None (top-level\ncall). When \blockno is Some, the caller ( emove_item/update_item)\nalready holds the header lock\n\nRead start_blockno directly from the already-held header_lock instead of\ncalling get_start_blockno(), avoiding a double shared-lock acquisition\non the same block (which would trigger a panic under the \block_tracker\ndebug feature).\n\n## Why This Is Safe\n\n- **No deadlock**: Header block is always locked first, content blocks\nin ascending order — consistent lock ordering\n- **No reader-reader blocking**: Multiple readers hold shared locks\nconcurrently (shared locks are compatible)\n- **Writer waits for readers**: \u0007tomically() takes an exclusive header\nlock, which blocks until all shared locks are released — correct\nserialization\n- **Minimal performance impact**: The header lock was already acquired;\nwe just hold it slightly longer\n\n## Verification\n\n- \rustfmt --check passes\n- Pattern matches the proven emove_item()/update_item() implementation\nin the same file\n- Full cargo check requires pgrx setup (PostgreSQL extension); the\nchange is limited to lock lifetime management with no new APIs\n\nCloses #4935",
          "timestamp": "2026-05-14T15:00:05-04:00",
          "tree_id": "46e474245958de09c21d0198195343d8a87fb72d",
          "url": "https://github.com/paradedb/paradedb/commit/b597b183d25a561cf8c81d81decc7f07b7866e55"
        },
        "date": 1778789745513,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - tps",
            "value": 601.2145437687412,
            "unit": "median tps",
            "extra": "avg tps: 610.4136173790109, max tps: 858.9028718276422, count: 53854"
          },
          {
            "name": "Index Only Scan - Subscriber - tps",
            "value": 622.6324344814624,
            "unit": "median tps",
            "extra": "avg tps: 633.3534858205228, max tps: 829.1429227339847, count: 53854"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - tps",
            "value": 91.19412896572905,
            "unit": "median tps",
            "extra": "avg tps: 91.48213146743981, max tps: 97.91503837867644, count: 53854"
          },
          {
            "name": "Top K - Subscriber - tps",
            "value": 272.3539761221426,
            "unit": "median tps",
            "extra": "avg tps: 268.0613177079677, max tps: 563.5156524361684, count: 107708"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "652b0952a7f3be58ea4b9fa80a863426cf03185c",
          "message": "chore(stressgres): remove dormant suites not wired into CI (#5085)\n\n## Summary\n\n- Removes 8 Stressgres suites that have not been touched since\nStressgres was added (#3821), are not referenced by any CI workflow or\ndriver script, and use the legacy `[setup_primary]` schema rather than\nthe current `[[server]]` schema.\n- No CI workflow changes; the 6 suites currently exercised by\n`benchmark-pg_search-stressgres.yml` (`single-server`, `bulk-updates`,\n`wide-table`, `background-merge`, `logical-replication`,\n`logical-replication-merge`) and `vanilla-postgres.toml` (used by the\nantithesis singleton driver) are kept.\n\n### Removed\n- `large-inserts.toml`\n- `lr.toml` — predecessor of `logical-replication.toml`\n- `lr-graphable.toml`\n- `lr-large-inserts.toml`\n- `lr-no-pg_search.toml`\n- `many-updates.toml` — conceptually useful (MVCC correctness with\n`assert(count, expected)` under concurrency), but on legacy schema; if\nwe want this coverage in CI it should be a deliberate port, not a\nrevival of dead config\n- `read-write.toml` — contained placeholder `<password>` literal, not\nCI-runnable\n- `topk-crash.toml`\n\n### Why now\nCompanion to #5080 (which adjusts `logical-replication.toml` to expose\nthe FSM race instead of being masked by the TOAST bug already covered by\n`logical-replication-merge.toml`). Each CI suite should own a distinct\nrepro; dormant files muddy that mapping.\n\n## Test plan\n- [x] `benchmark-pg_search-stressgres` workflow still kicks off for all\n6 in-CI suites\n- [x] Antithesis `singleton_driver_vanilla-postgres.sh` still resolves\nits suite path",
          "timestamp": "2026-05-14T15:45:37-04:00",
          "tree_id": "090e72f7bbb9817642fb0c7495c2b12e0eed2fe7",
          "url": "https://github.com/paradedb/paradedb/commit/652b0952a7f3be58ea4b9fa80a863426cf03185c"
        },
        "date": 1778792596819,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - tps",
            "value": 592.5760675795299,
            "unit": "median tps",
            "extra": "avg tps: 598.3799918800473, max tps: 824.5422642759576, count: 53870"
          },
          {
            "name": "Index Only Scan - Subscriber - tps",
            "value": 613.1584972836808,
            "unit": "median tps",
            "extra": "avg tps: 620.9749794450688, max tps: 900.3103439010068, count: 53870"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - tps",
            "value": 93.46735848570242,
            "unit": "median tps",
            "extra": "avg tps: 93.61326490313851, max tps: 99.96730369398081, count: 53870"
          },
          {
            "name": "Top K - Subscriber - tps",
            "value": 275.23918239089903,
            "unit": "median tps",
            "extra": "avg tps: 267.7008836240167, max tps: 586.4173225255738, count: 107740"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ming.ying.nyc@gmail.com",
            "name": "Ming",
            "username": "rebasedming"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "21bdc9a753582039cc25a38cb07111026c0fd378",
          "message": "feat: Crash recovery via WAL (#4901)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nPorts WAL integration over to community, which gives `pg_search` crash\nrecovery.\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2026-05-14T14:59:53-07:00",
          "tree_id": "7580b5f2bfcc1a95ed1b62c4628f2ec6129b7c00",
          "url": "https://github.com/paradedb/paradedb/commit/21bdc9a753582039cc25a38cb07111026c0fd378"
        },
        "date": 1778800644073,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - tps",
            "value": 595.6374050430177,
            "unit": "median tps",
            "extra": "avg tps: 598.741366221032, max tps: 798.4029119499545, count: 53868"
          },
          {
            "name": "Index Only Scan - Subscriber - tps",
            "value": 613.1254948847361,
            "unit": "median tps",
            "extra": "avg tps: 616.654782358617, max tps: 875.3109970458254, count: 53868"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - tps",
            "value": 96.04612170814892,
            "unit": "median tps",
            "extra": "avg tps: 96.02092628712133, max tps: 103.14632474193625, count: 53868"
          },
          {
            "name": "Top K - Subscriber - tps",
            "value": 294.3820056766177,
            "unit": "median tps",
            "extra": "avg tps: 291.3984013057396, max tps: 605.2091867728507, count: 107736"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "d99908a1f58307567dd34698345b3fc836be6135",
          "message": "feat(antithesis): add singleton drivers for logical-replication suites (#5087)\n\n## Summary\n\nAdds OSS Antithesis singleton drivers for the two CI\n`logical-replication` suites that previously had no singleton\n(`single-server`, `bulk-updates`, `wide-table`, `background-merge`, and\n`vanilla-postgres` already had one).\n\nMirrors the enterprise pattern used for `physical-logical-replication`:\n- A **vanilla Postgres 18** publisher pod (with `wal_level=logical`)\nthat lives outside the CNPG cluster, reflecting real-world\nlogical-replication topologies where the upstream primary is not under\nour control.\n- Subscriber points at `paradedb-rw` (the CNPG primary, which has\n`pg_search`).\n\n## Changes\n- `docker/manifests/antithesis-paradedb.yaml` — add\n`logical-replication-publisher` Service + Deployment (vanilla Postgres\n18 with `wal_level=logical`), reusing the existing `paradedb-superuser`\nsecret.\n- `stressgres/suites/logical-replication.toml`,\n`stressgres/suites/logical-replication-merge.toml` — drop `CREATE\nEXTENSION pg_search` from the **Publisher** setup. Only the Subscriber\nuses `pg_search`; the line was cosmetic and incompatible with a vanilla\nPostgres publisher (the line in the Subscriber setup is unchanged).\n-\n`stressgres/suites/antithesis/singleton_driver_logical-replication.sh`,\n`singleton_driver_logical-replication-merge.sh` — new drivers that\nperform per-block `sed -z` rewrites of the `[server.style.Automatic]`\nblocks into `[server.style.With]` connection strings (Publisher →\n`logical-replication-publisher:5432`, Subscriber → `paradedb-rw:5432`).\n- `.github/workflows/antithesis-trigger-test-run.yml` — add\n`logical-replication-publisher` to\n`container_faults_stop_exclusion_patterns` and\n`container_faults_kill_exclusion_patterns`, matching enterprise. Network\nfaults to/from the publisher are intentionally still injected.\n\n## Why\nWithout these, the FSM race repro in `logical-replication-merge.toml`\n(issue #4935, fixed by #5067) and the broader logical-replication\ncoverage in `logical-replication.toml` were running in\n`benchmark-pg_search-stressgres` but had no Antithesis fault-injection\nequivalent — that's the half of the matrix where the bugs originally\nsurfaced.\n\n## Test plan\n- [x] Antithesis trigger workflow picks up both new singleton drivers\nfrom `/opt/antithesis/test/v1/quickstart/`\n- [x] Publisher pod (`logical-replication-publisher`) starts with\n`wal_level=logical` and is reachable from the stressgres-runner pod\n- [x] Subscriber's `CREATE SUBSCRIPTION ... CONNECTION\n'@Publisher_CONNSTR@'` resolves to the publisher pod after the `sed`\nrewrite\n- [x] `logical-replication-merge.toml` still reproduces the FSM race\nwhen run against a build without #5067\n- [x] `benchmark-pg_search-stressgres` (local Stressgres, not\nAntithesis) still runs both suites unchanged",
          "timestamp": "2026-05-14T18:54:16-04:00",
          "tree_id": "8c7a6dab334db43248e88e7a4ebc75fd5840e446",
          "url": "https://github.com/paradedb/paradedb/commit/d99908a1f58307567dd34698345b3fc836be6135"
        },
        "date": 1778803812170,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - tps",
            "value": 619.6261624324003,
            "unit": "median tps",
            "extra": "avg tps: 618.7741074135282, max tps: 764.5624113274296, count: 53878"
          },
          {
            "name": "Index Only Scan - Subscriber - tps",
            "value": 647.5138442073119,
            "unit": "median tps",
            "extra": "avg tps: 646.6838165870267, max tps: 819.7719679148155, count: 53878"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - tps",
            "value": 90.1443059370128,
            "unit": "median tps",
            "extra": "avg tps: 90.16334287930128, max tps: 101.12687531152345, count: 53878"
          },
          {
            "name": "Top K - Subscriber - tps",
            "value": 282.11382457593027,
            "unit": "median tps",
            "extra": "avg tps: 275.68372421444934, max tps: 644.3398505463382, count: 107756"
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
        "date": 1778655794157,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - cpu",
            "value": 4.5845275,
            "unit": "median cpu",
            "extra": "avg cpu: 5.093064852579983, max cpu: 9.284333, count: 53875"
          },
          {
            "name": "Custom Scan - Subscriber - mem",
            "value": 53.26171875,
            "unit": "median mem",
            "extra": "avg mem: 53.276453813805105, max mem: 59.3828125, count: 53875"
          },
          {
            "name": "Delete values - Publisher - cpu",
            "value": 4.5801525,
            "unit": "median cpu",
            "extra": "avg cpu: 2.4910544106260595, max cpu: 4.6065254, count: 53875"
          },
          {
            "name": "Delete values - Publisher - mem",
            "value": 32.1875,
            "unit": "median mem",
            "extra": "avg mem: 31.445576058584688, max mem: 32.50390625, count: 53875"
          },
          {
            "name": "Find by ctid - Subscriber - cpu",
            "value": 9.142857,
            "unit": "median cpu",
            "extra": "avg cpu: 8.543918860535676, max cpu: 18.461538, count: 53875"
          },
          {
            "name": "Find by ctid - Subscriber - mem",
            "value": 55.90234375,
            "unit": "median mem",
            "extra": "avg mem: 55.55137797273782, max mem: 61.92578125, count: 53875"
          },
          {
            "name": "Index Only Scan - Subscriber - cpu",
            "value": 4.5845275,
            "unit": "median cpu",
            "extra": "avg cpu: 5.093439674504756, max cpu: 9.275363, count: 53875"
          },
          {
            "name": "Index Only Scan - Subscriber - mem",
            "value": 52.75,
            "unit": "median mem",
            "extra": "avg mem: 52.7553124274942, max mem: 58.87109375, count: 53875"
          },
          {
            "name": "Index Size Info - Subscriber - cpu",
            "value": 4.5801525,
            "unit": "median cpu",
            "extra": "avg cpu: 4.593974366523301, max cpu: 9.204219, count: 53875"
          },
          {
            "name": "Index Size Info - Subscriber - mem",
            "value": 33.53125,
            "unit": "median mem",
            "extra": "avg mem: 33.60742691415313, max mem: 38.890625, count: 53875"
          },
          {
            "name": "Index Size Info - Subscriber - pages",
            "value": 1144,
            "unit": "median pages",
            "extra": "avg pages: 1141.116807424594, max pages: 1896.0, count: 53875"
          },
          {
            "name": "Index Size Info - Subscriber - relation_size:MB",
            "value": 8.9375,
            "unit": "median relation_size:MB",
            "extra": "avg relation_size:MB: 8.914975493039444, max relation_size:MB: 14.8125, count: 53875"
          },
          {
            "name": "Index Size Info - Subscriber - segment_count",
            "value": 11,
            "unit": "median segment_count",
            "extra": "avg segment_count: 11.58245939675174, max segment_count: 22.0, count: 53875"
          },
          {
            "name": "Insert value A - Publisher - cpu",
            "value": 4.507042,
            "unit": "median cpu",
            "extra": "avg cpu: 3.416494555239076, max cpu: 4.5933013, count: 53875"
          },
          {
            "name": "Insert value A - Publisher - mem",
            "value": 29.40625,
            "unit": "median mem",
            "extra": "avg mem: 28.69406895301624, max mem: 29.7578125, count: 53875"
          },
          {
            "name": "Insert value B - Publisher - cpu",
            "value": 4.5933013,
            "unit": "median cpu",
            "extra": "avg cpu: 4.502906823497388, max cpu: 4.610951, count: 53875"
          },
          {
            "name": "Insert value B - Publisher - mem",
            "value": 29.4609375,
            "unit": "median mem",
            "extra": "avg mem: 28.702378335266822, max mem: 29.8046875, count: 53875"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - cpu",
            "value": 9.17782,
            "unit": "median cpu",
            "extra": "avg cpu: 10.188320874672202, max cpu: 23.506365, count: 53875"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - mem",
            "value": 52.06640625,
            "unit": "median mem",
            "extra": "avg mem: 52.02861057134571, max mem: 58.16015625, count: 53875"
          },
          {
            "name": "SELECT\n  pid,\n  pg_wal_lsn_diff(sent_lsn, replay_lsn) AS replication_lag,\n  application_name::text,\n  state::text\nFROM pg_stat_replication; - Publisher - replication_lag:MB",
            "value": 0,
            "unit": "median replication_lag:MB",
            "extra": "avg replication_lag:MB: 0.000023684472882830626, max replication_lag:MB: 0.14870452880859375, count: 53875"
          },
          {
            "name": "Top K - Subscriber - cpu",
            "value": 4.5845275,
            "unit": "median cpu",
            "extra": "avg cpu: 5.259570511493129, max cpu: 13.846154, count: 107750"
          },
          {
            "name": "Top K - Subscriber - mem",
            "value": 52.046875,
            "unit": "median mem",
            "extra": "avg mem: 51.96822041763341, max mem: 58.390625, count: 107750"
          },
          {
            "name": "Update 1..9 - Publisher - cpu",
            "value": 4.5584044,
            "unit": "median cpu",
            "extra": "avg cpu: 4.0645801962298815, max cpu: 4.6021094, count: 53875"
          },
          {
            "name": "Update 1..9 - Publisher - mem",
            "value": 32.83203125,
            "unit": "median mem",
            "extra": "avg mem: 32.12121091937355, max mem: 33.1484375, count: 53875"
          },
          {
            "name": "Update 10,11 - Publisher - cpu",
            "value": 4.562738,
            "unit": "median cpu",
            "extra": "avg cpu: 4.115904919197946, max cpu: 4.6065254, count: 53875"
          },
          {
            "name": "Update 10,11 - Publisher - mem",
            "value": 32.8125,
            "unit": "median mem",
            "extra": "avg mem: 32.083466864849186, max mem: 32.8984375, count: 53875"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "59696464+saadtajwar@users.noreply.github.com",
            "name": "Saad Tajwar",
            "username": "saadtajwar"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "bc20c3dd4b36a13dbcc74d03499f966a4dc93fe6",
          "message": "refactor: Deduplicate deferred materialization request partitioning between visibility and lookup (#4903)\n\n# Ticket(s) Closed\n- Closes https://github.com/paradedb/paradedb/issues/4568\n\n## What\nDeduplicated the segment-grouping/materialization loop shared between\n`materialize_deferred_ctid()` in `visibility_filter.rs` and\n`materialize_deferred_column()` in `tantivy_lookup_exec.rs`.\n\n## Why\nBoth functions implemented the same pattern of partitioning packed doc\naddresses by segment ordinal, batch-reading via `FFHelper`, and writing\nresults back in row order. This duplication made the two paths harder to\nkeep consistent and made future optimization work more tedious.\n\n## How\n\n### Shared helper in `fast_fields_helper.rs`\n- Added `for_each_segment`: partitions an iterator of `(row_index,\npacked_doc_address)` pairs into per-segment buckets and invokes a\ncaller-supplied closure once per non-empty segment, in segment-ordinal\norder. Backed by a `Vec<Vec<(usize, DocId)>>` indexed by segment ordinal\n(dense in practice; cheaper than a hash map for typical segment counts).\n- Added `FFHelper::num_segments()` so callers can size the bucket vector\nwithout reaching into private fields.\n\n### `materialize_deferred_ctid` (visibility_filter.rs)\n- Replaced the manual sort + partition + per-segment slice loop with a\nsingle `for_each_segment` call.\n- Kept `DeferredCtidMaterializationState` for buffer reuse across calls,\nbut removed its now-unused `requests` field; the per-segment\npartitioning lives inside `for_each_segment`.\n- Removed the TODO comment that flagged this duplication.\n\n### `materialize_deferred_column` (tantivy_lookup_exec.rs)\nFunction body shrank from ~160 lines to ~50 by extracting three\nsingle-responsibility helpers:\n- `resolve_doc_addresses_to_term_ords` — resolves State 0 (packed doc\naddresses) into per-segment `(row_index, Option<TermOrdinal>)` pairs via\n`for_each_segment`.\n- `extract_term_ords` — parses State 1 (pre-resolved `(segment_ord,\nterm_ord)` pairs from the dense union's `StructArray` child) into the\nsame per-segment shape.\n- `decode_term_ordinals` — takes the merged per-segment ordinals and\nperforms the bulk dictionary lookup once per segment, recording\npositions for the final `interleave`.\n\nState 0 and State 1 are now merged into a single `Vec<Vec<(row_index,\nOption<TermOrdinal>)>>` indexed by segment ordinal, then decoded in one\npass — previously each state was iterated and decoded separately,\nproducing two `segment_arrays` entries per segment touched by both. The\nfinal interleaved output is identical.\n\nReplaced the `(ff_index: usize, is_bytes: bool)` parameter pair with a\n`DeferredColumnKind { Text { ff_index }, Bytes { ff_index } }` enum to\nmake the `is_bytes && wrong-ff-type` mismatch unrepresentable.\n\n## Tests\nNo new tests; behavior is unchanged. Existing coverage exercises both\npaths end-to-end:\n- `pg_search/tests/pg_regress/sql/join_deferred_visibility.sql` —\n`materialize_deferred_ctid`.\n- `pg_search/tests/pg_regress/sql/segmented_topk.sql` plus joinscan\ntests — `materialize_deferred_column` (the segmented top-K rule is what\nproduces State 1 rows below `TantivyLookupExec`).\n\n---------\n\nCo-authored-by: Mithun Chicklore Yogendra <mithun.cy@gmail.com>",
          "timestamp": "2026-05-13T10:07:59-07:00",
          "tree_id": "cc9e05e63ed6052c202d00901357d2c5026923d0",
          "url": "https://github.com/paradedb/paradedb/commit/bc20c3dd4b36a13dbcc74d03499f966a4dc93fe6"
        },
        "date": 1778696654513,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - cpu",
            "value": 4.567079,
            "unit": "median cpu",
            "extra": "avg cpu: 5.072662982117275, max cpu: 9.257474, count: 53864"
          },
          {
            "name": "Custom Scan - Subscriber - mem",
            "value": 52.6640625,
            "unit": "median mem",
            "extra": "avg mem: 52.76543615633818, max mem: 58.890625, count: 53864"
          },
          {
            "name": "Delete values - Publisher - cpu",
            "value": 4.5540795,
            "unit": "median cpu",
            "extra": "avg cpu: 4.137016787146727, max cpu: 4.5801525, count: 53864"
          },
          {
            "name": "Delete values - Publisher - mem",
            "value": 32.453125,
            "unit": "median mem",
            "extra": "avg mem: 31.787727395616738, max mem: 33.125, count: 53864"
          },
          {
            "name": "Find by ctid - Subscriber - cpu",
            "value": 9.116809,
            "unit": "median cpu",
            "extra": "avg cpu: 8.539325465475969, max cpu: 18.33811, count: 53864"
          },
          {
            "name": "Find by ctid - Subscriber - mem",
            "value": 55.5234375,
            "unit": "median mem",
            "extra": "avg mem: 55.25188843661815, max mem: 61.62109375, count: 53864"
          },
          {
            "name": "Index Only Scan - Subscriber - cpu",
            "value": 4.567079,
            "unit": "median cpu",
            "extra": "avg cpu: 5.065430960447016, max cpu: 9.230769, count: 53864"
          },
          {
            "name": "Index Only Scan - Subscriber - mem",
            "value": 51.8828125,
            "unit": "median mem",
            "extra": "avg mem: 51.970352850465986, max mem: 58.0859375, count: 53864"
          },
          {
            "name": "Index Size Info - Subscriber - cpu",
            "value": 4.562738,
            "unit": "median cpu",
            "extra": "avg cpu: 4.58080178317347, max cpu: 9.213051, count: 53864"
          },
          {
            "name": "Index Size Info - Subscriber - mem",
            "value": 32.94921875,
            "unit": "median mem",
            "extra": "avg mem: 33.04980990898374, max mem: 38.23046875, count: 53864"
          },
          {
            "name": "Index Size Info - Subscriber - pages",
            "value": 1121,
            "unit": "median pages",
            "extra": "avg pages: 1129.3315201247588, max pages: 1882.0, count: 53864"
          },
          {
            "name": "Index Size Info - Subscriber - relation_size:MB",
            "value": 8.7578125,
            "unit": "median relation_size:MB",
            "extra": "avg relation_size:MB: 8.822902646015892, max relation_size:MB: 14.703125, count: 53864"
          },
          {
            "name": "Index Size Info - Subscriber - segment_count",
            "value": 11,
            "unit": "median segment_count",
            "extra": "avg segment_count: 10.79318283083321, max segment_count: 19.0, count: 53864"
          },
          {
            "name": "Insert value A - Publisher - cpu",
            "value": 4.5714283,
            "unit": "median cpu",
            "extra": "avg cpu: 4.135433990221484, max cpu: 4.6153846, count: 53864"
          },
          {
            "name": "Insert value A - Publisher - mem",
            "value": 29.20703125,
            "unit": "median mem",
            "extra": "avg mem: 28.558138175543963, max mem: 29.6015625, count: 53864"
          },
          {
            "name": "Insert value B - Publisher - cpu",
            "value": 4.5368624,
            "unit": "median cpu",
            "extra": "avg cpu: 3.3959431196506045, max cpu: 4.58891, count: 53864"
          },
          {
            "name": "Insert value B - Publisher - mem",
            "value": 29.29296875,
            "unit": "median mem",
            "extra": "avg mem: 28.657318228547823, max mem: 29.85546875, count: 53864"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - cpu",
            "value": 9.142857,
            "unit": "median cpu",
            "extra": "avg cpu: 10.182340970706203, max cpu: 27.246925, count: 53864"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - mem",
            "value": 50.78515625,
            "unit": "median mem",
            "extra": "avg mem: 50.89196315024878, max mem: 56.9453125, count: 53864"
          },
          {
            "name": "SELECT\n  pid,\n  pg_wal_lsn_diff(sent_lsn, replay_lsn) AS replication_lag,\n  application_name::text,\n  state::text\nFROM pg_stat_replication; - Publisher - replication_lag:MB",
            "value": 0,
            "unit": "median replication_lag:MB",
            "extra": "avg replication_lag:MB: 0.00002672455205652488, max replication_lag:MB: 0.1368560791015625, count: 53864"
          },
          {
            "name": "Top K - Subscriber - cpu",
            "value": 4.567079,
            "unit": "median cpu",
            "extra": "avg cpu: 5.223940707935863, max cpu: 13.766731, count: 107728"
          },
          {
            "name": "Top K - Subscriber - mem",
            "value": 51.71875,
            "unit": "median mem",
            "extra": "avg mem: 51.77521160788049, max mem: 58.046875, count: 107728"
          },
          {
            "name": "Update 1..9 - Publisher - cpu",
            "value": 4.562738,
            "unit": "median cpu",
            "extra": "avg cpu: 4.402011915539643, max cpu: 4.5933013, count: 53864"
          },
          {
            "name": "Update 1..9 - Publisher - mem",
            "value": 32.7578125,
            "unit": "median mem",
            "extra": "avg mem: 32.00510189725512, max mem: 32.76953125, count: 53864"
          },
          {
            "name": "Update 10,11 - Publisher - cpu",
            "value": 4.567079,
            "unit": "median cpu",
            "extra": "avg cpu: 3.8832999511081194, max cpu: 4.5801525, count: 53864"
          },
          {
            "name": "Update 10,11 - Publisher - mem",
            "value": 33.03515625,
            "unit": "median mem",
            "extra": "avg mem: 32.348674946972935, max mem: 33.4375, count: 53864"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "stuhood@paradedb.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "713eaca82145388378043ddb2c750c41e9c632d9",
          "message": "chore: Revert #4905 (#5066)\n\nRevert #4905 due to performance regressions.",
          "timestamp": "2026-05-13T13:34:27-07:00",
          "tree_id": "2894402ba71bbb0b86d5990862fb2adba4fcbf49",
          "url": "https://github.com/paradedb/paradedb/commit/713eaca82145388378043ddb2c750c41e9c632d9"
        },
        "date": 1778709030467,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - cpu",
            "value": 4.5714283,
            "unit": "median cpu",
            "extra": "avg cpu: 5.0543948824229075, max cpu: 9.266409, count: 53907"
          },
          {
            "name": "Custom Scan - Subscriber - mem",
            "value": 53.36328125,
            "unit": "median mem",
            "extra": "avg mem: 53.368353425807406, max mem: 59.16796875, count: 53907"
          },
          {
            "name": "Delete values - Publisher - cpu",
            "value": 4.562738,
            "unit": "median cpu",
            "extra": "avg cpu: 3.621148403469874, max cpu: 4.58891, count: 53907"
          },
          {
            "name": "Delete values - Publisher - mem",
            "value": 32.59765625,
            "unit": "median mem",
            "extra": "avg mem: 31.917860273248372, max mem: 33.32421875, count: 53907"
          },
          {
            "name": "Find by ctid - Subscriber - cpu",
            "value": 9.116809,
            "unit": "median cpu",
            "extra": "avg cpu: 8.207306850847072, max cpu: 18.426102, count: 53907"
          },
          {
            "name": "Find by ctid - Subscriber - mem",
            "value": 55.75390625,
            "unit": "median mem",
            "extra": "avg mem: 55.39522703742093, max mem: 61.53515625, count: 53907"
          },
          {
            "name": "Index Only Scan - Subscriber - cpu",
            "value": 4.5714283,
            "unit": "median cpu",
            "extra": "avg cpu: 5.059698283335458, max cpu: 9.266409, count: 53907"
          },
          {
            "name": "Index Only Scan - Subscriber - mem",
            "value": 52.109375,
            "unit": "median mem",
            "extra": "avg mem: 52.125818104559706, max mem: 57.93359375, count: 53907"
          },
          {
            "name": "Index Size Info - Subscriber - cpu",
            "value": 4.567079,
            "unit": "median cpu",
            "extra": "avg cpu: 4.59899597620134, max cpu: 9.17782, count: 53907"
          },
          {
            "name": "Index Size Info - Subscriber - mem",
            "value": 33.51171875,
            "unit": "median mem",
            "extra": "avg mem: 33.54517669029532, max mem: 38.67578125, count: 53907"
          },
          {
            "name": "Index Size Info - Subscriber - pages",
            "value": 1118,
            "unit": "median pages",
            "extra": "avg pages: 1117.726158012874, max pages: 1842.0, count: 53907"
          },
          {
            "name": "Index Size Info - Subscriber - relation_size:MB",
            "value": 8.734375,
            "unit": "median relation_size:MB",
            "extra": "avg relation_size:MB: 8.732235609475579, max relation_size:MB: 14.390625, count: 53907"
          },
          {
            "name": "Index Size Info - Subscriber - segment_count",
            "value": 9,
            "unit": "median segment_count",
            "extra": "avg segment_count: 8.899419370397165, max segment_count: 16.0, count: 53907"
          },
          {
            "name": "Insert value A - Publisher - cpu",
            "value": 4.5714283,
            "unit": "median cpu",
            "extra": "avg cpu: 4.342222570806338, max cpu: 4.5845275, count: 53907"
          },
          {
            "name": "Insert value A - Publisher - mem",
            "value": 29.40625,
            "unit": "median mem",
            "extra": "avg mem: 28.78113355234478, max mem: 29.90234375, count: 53907"
          },
          {
            "name": "Insert value B - Publisher - cpu",
            "value": 4.5801525,
            "unit": "median cpu",
            "extra": "avg cpu: 4.35690127771939, max cpu: 4.5933013, count: 53907"
          },
          {
            "name": "Insert value B - Publisher - mem",
            "value": 29.5546875,
            "unit": "median mem",
            "extra": "avg mem: 28.91315657057525, max mem: 30.0390625, count: 53907"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - cpu",
            "value": 4.5845275,
            "unit": "median cpu",
            "extra": "avg cpu: 6.3179098518343215, max cpu: 23.121387, count: 53907"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - mem",
            "value": 51.67578125,
            "unit": "median mem",
            "extra": "avg mem: 51.66356591270614, max mem: 57.484375, count: 53907"
          },
          {
            "name": "SELECT\n  pid,\n  pg_wal_lsn_diff(sent_lsn, replay_lsn) AS replication_lag,\n  application_name::text,\n  state::text\nFROM pg_stat_replication; - Publisher - replication_lag:MB",
            "value": 0,
            "unit": "median replication_lag:MB",
            "extra": "avg replication_lag:MB: 0.000040318732521982304, max replication_lag:MB: 0.14935302734375, count: 53907"
          },
          {
            "name": "Top K - Subscriber - cpu",
            "value": 4.5757866,
            "unit": "median cpu",
            "extra": "avg cpu: 5.238919730528756, max cpu: 13.832853, count: 107814"
          },
          {
            "name": "Top K - Subscriber - mem",
            "value": 51.43359375,
            "unit": "median mem",
            "extra": "avg mem: 51.43432573256952, max mem: 57.5234375, count: 107814"
          },
          {
            "name": "Update 1..9 - Publisher - cpu",
            "value": 4.58891,
            "unit": "median cpu",
            "extra": "avg cpu: 4.3708178468759336, max cpu: 4.5933013, count: 53907"
          },
          {
            "name": "Update 1..9 - Publisher - mem",
            "value": 32.98828125,
            "unit": "median mem",
            "extra": "avg mem: 32.266170934433376, max mem: 33.0, count: 53907"
          },
          {
            "name": "Update 10,11 - Publisher - cpu",
            "value": 4.5584044,
            "unit": "median cpu",
            "extra": "avg cpu: 4.489791009697886, max cpu: 4.5757866, count: 53907"
          },
          {
            "name": "Update 10,11 - Publisher - mem",
            "value": 33.265625,
            "unit": "median mem",
            "extra": "avg mem: 32.59773190112138, max mem: 33.6875, count: 53907"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "035982deb7c1e630e0d8a13e4ca1910b979b08f6",
          "message": "ci: add logical-replication-merge stressgres suite to cover FSM race (#4935) (#5072)\n\nRe-opened from #5068 (originally from a fork, where CI could not access\nworkflow secrets — see\nhttps://github.com/paradedb/paradedb/actions/runs/25830120431/job/75892835684?pr=5068).\nSame intent, from a branch in this repo, with the FSM-race suite landing\nas its own file rather than folded into `logical-replication.toml`.\n\n## Changes\n\n- `stressgres/suites/logical-replication-merge.toml` (new): standalone\nsuite that reliably reproduces the FSM race from #4935 (fixed in #5067).\nLogical-replication subscriber with aggressive autovacuum (`naptime=1s`,\n`threshold=50`), small `layer_sizes = '10kb, 100kb, 1mb, 100mb'`,\nmultiple concurrent BM25 readers, and sustained UPDATE/INSERT/DELETE\ntraffic on the publisher. The key difference from\n`logical-replication.toml` is the writer: `message = message || ' ' ||\ntxid_current()` grows each row's terms unbounded, generating ~10× more\nmerge/GC pressure and reliably opening the race window — folding the\nsame churn into `logical-replication.toml` (which strips-then-appends,\nkeeping row size constant) did not reproduce the bug.\n- `.github/workflows/benchmark-pg_search-stressgres.yml`:\n- Comment out single-server, bulk-updates, wide-table, and\nbackground-merge so CI focuses on the two replication suites while we\niterate. To be re-enabled before final merge.\n- Run `logical-replication-merge.toml` **before**\n`logical-replication.toml`.\n\n`stressgres/suites/logical-replication.toml` is unchanged from `main`.\n\n## Expected behavior\n\n- Without #5067: SIGSEGV or `SegmentMetaEntryHeader: UnexpectedEnd`\nwithin minutes.\n- With #5067: runs the full duration without errors.\n\n## Follow-ups\n\n- Re-enable the four commented-out suites before final merge.\n- Antithesis wiring for this suite belongs in `paradedb-enterprise` next\nto the existing `physical-logical-replication` driver, since the OSS\nAntithesis manifest only stands up a single paradedb cluster.\n\nRef: #4935\nRelated: #5067\nSupersedes: #5068",
          "timestamp": "2026-05-13T19:51:05-04:00",
          "tree_id": "c71af69d7df60d54d1631876f7e3c7af0782c3e3",
          "url": "https://github.com/paradedb/paradedb/commit/035982deb7c1e630e0d8a13e4ca1910b979b08f6"
        },
        "date": 1778720818991,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - cpu",
            "value": 4.5801525,
            "unit": "median cpu",
            "extra": "avg cpu: 5.081747298792209, max cpu: 13.806328, count: 53853"
          },
          {
            "name": "Custom Scan - Subscriber - mem",
            "value": 53.0078125,
            "unit": "median mem",
            "extra": "avg mem: 53.03036521294079, max mem: 58.96484375, count: 53853"
          },
          {
            "name": "Delete values - Publisher - cpu",
            "value": 4.562738,
            "unit": "median cpu",
            "extra": "avg cpu: 3.490042551711338, max cpu: 4.619827, count: 53853"
          },
          {
            "name": "Delete values - Publisher - mem",
            "value": 32.78125,
            "unit": "median mem",
            "extra": "avg mem: 32.10156583662934, max mem: 33.43359375, count: 53853"
          },
          {
            "name": "Find by ctid - Subscriber - cpu",
            "value": 9.142857,
            "unit": "median cpu",
            "extra": "avg cpu: 8.342089038766028, max cpu: 18.461538, count: 53853"
          },
          {
            "name": "Find by ctid - Subscriber - mem",
            "value": 55.8828125,
            "unit": "median mem",
            "extra": "avg mem: 55.562831051659145, max mem: 61.80859375, count: 53853"
          },
          {
            "name": "Index Only Scan - Subscriber - cpu",
            "value": 4.5801525,
            "unit": "median cpu",
            "extra": "avg cpu: 5.080209018874647, max cpu: 9.284333, count: 53853"
          },
          {
            "name": "Index Only Scan - Subscriber - mem",
            "value": 52.27734375,
            "unit": "median mem",
            "extra": "avg mem: 52.28694156430468, max mem: 58.22265625, count: 53853"
          },
          {
            "name": "Index Size Info - Subscriber - cpu",
            "value": 4.5801525,
            "unit": "median cpu",
            "extra": "avg cpu: 4.621467531364052, max cpu: 9.204219, count: 53853"
          },
          {
            "name": "Index Size Info - Subscriber - mem",
            "value": 34.01171875,
            "unit": "median mem",
            "extra": "avg mem: 34.036072735037976, max mem: 39.24609375, count: 53853"
          },
          {
            "name": "Index Size Info - Subscriber - pages",
            "value": 1132,
            "unit": "median pages",
            "extra": "avg pages: 1132.1565186711975, max pages: 1873.0, count: 53853"
          },
          {
            "name": "Index Size Info - Subscriber - relation_size:MB",
            "value": 8.84375,
            "unit": "median relation_size:MB",
            "extra": "avg relation_size:MB: 8.844972947189571, max relation_size:MB: 14.6328125, count: 53853"
          },
          {
            "name": "Index Size Info - Subscriber - segment_count",
            "value": 10,
            "unit": "median segment_count",
            "extra": "avg segment_count: 9.753569903255158, max segment_count: 17.0, count: 53853"
          },
          {
            "name": "Insert value A - Publisher - cpu",
            "value": 4.5368624,
            "unit": "median cpu",
            "extra": "avg cpu: 2.7649570615362276, max cpu: 4.5801525, count: 53853"
          },
          {
            "name": "Insert value A - Publisher - mem",
            "value": 29.6015625,
            "unit": "median mem",
            "extra": "avg mem: 28.924278678764413, max mem: 30.03515625, count: 53853"
          },
          {
            "name": "Insert value B - Publisher - cpu",
            "value": 4.567079,
            "unit": "median cpu",
            "extra": "avg cpu: 4.116485473221473, max cpu: 4.5933013, count: 53853"
          },
          {
            "name": "Insert value B - Publisher - mem",
            "value": 29.609375,
            "unit": "median mem",
            "extra": "avg mem: 29.0038740442733, max mem: 30.08203125, count: 53853"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - cpu",
            "value": 4.597701,
            "unit": "median cpu",
            "extra": "avg cpu: 6.551257780264088, max cpu: 23.121387, count: 53853"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - mem",
            "value": 51.01171875,
            "unit": "median mem",
            "extra": "avg mem: 51.060454501142, max mem: 57.03125, count: 53853"
          },
          {
            "name": "SELECT\n  pid,\n  pg_wal_lsn_diff(sent_lsn, replay_lsn) AS replication_lag,\n  application_name::text,\n  state::text\nFROM pg_stat_replication; - Publisher - replication_lag:MB",
            "value": 0,
            "unit": "median replication_lag:MB",
            "extra": "avg replication_lag:MB: 0.000027220616590243348, max replication_lag:MB: 0.18209075927734375, count: 53853"
          },
          {
            "name": "Top K - Subscriber - cpu",
            "value": 4.5845275,
            "unit": "median cpu",
            "extra": "avg cpu: 5.264929663620596, max cpu: 13.753581, count: 107706"
          },
          {
            "name": "Top K - Subscriber - mem",
            "value": 51.89453125,
            "unit": "median mem",
            "extra": "avg mem: 51.905371305916105, max mem: 58.33203125, count: 107706"
          },
          {
            "name": "Update 1..9 - Publisher - cpu",
            "value": 4.5845275,
            "unit": "median cpu",
            "extra": "avg cpu: 4.379002484665478, max cpu: 4.6021094, count: 53853"
          },
          {
            "name": "Update 1..9 - Publisher - mem",
            "value": 33.16015625,
            "unit": "median mem",
            "extra": "avg mem: 32.419595792017155, max mem: 33.171875, count: 53853"
          },
          {
            "name": "Update 10,11 - Publisher - cpu",
            "value": 4.567079,
            "unit": "median cpu",
            "extra": "avg cpu: 4.112222184413526, max cpu: 4.597701, count: 53853"
          },
          {
            "name": "Update 10,11 - Publisher - mem",
            "value": 33.34375,
            "unit": "median mem",
            "extra": "avg mem: 32.66242283681967, max mem: 33.75390625, count: 53853"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "36623265+daniel3303@users.noreply.github.com",
            "name": "Daniel Oliveira",
            "username": "daniel3303"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "b9c06c5f75ca94cf815eb57c71c46180e84b8038",
          "message": "fix(mlt): quote key_field identifier in internal SPI lookup (#5078)\n\n## Summary\n\n- `pdb.more_like_this(key_value)` raises `ERROR: column \"id\" does not\nexist` whenever the index's `key_field` column is a mixed-case\nPostgreSQL identifier (e.g. `\"Id\"`, `\"DocumentId\"`). Direct `@@@`-on-LHS\nsearches (`\"Content\" @@@ 'foo'`) are unaffected because they don't go\nthrough the internal SPI lookup. Repro in #5065.\n- Root cause: `pg_search/src/query/more_like_this.rs:152-157` builds the\nSPI `SELECT * FROM <ns>.<rel> WHERE <key_field> = $1` with `<ns>` and\n`<rel>` already routed through `pgrx::spi::quote_identifier`, but\ninterpolates `<key_field>` verbatim via `Display`. PostgreSQL folds the\nunquoted reference to lowercase, so a column named `\"Id\"` is looked up\nas `id` and the SPI call fails before MLT ever runs.\n- Fix: send the key field through\n`pgrx::spi::quote_identifier(key_field_name.root())`, matching how the\nnamespace and relation names are already quoted on the lines immediately\nabove. `.root()` strips the JSON sub-path (`key_field` is always a\ntop-level column).\n\n## Scope\n\nThe linked issue also lists JSON `term` filters (`@@@\n'{\"term\":{\"field\":\"Category\",…}}'::jsonb`) as failing on mixed-case\ncolumns. That path does **not** go through SPI — `term()` in\n`pg_search/src/query/pdb_query.rs:792` resolves the field via\n`schema.search_field(field.root())`, a pure Tantivy schema lookup — so\nit isn't fixed here and I couldn't find a corresponding\nunquoted-identifier hazard. If it reproduces on `0.23.x` it's a separate\nbug; tracking it on its own issue is cleaner than bundling a speculative\nfix.\n\n## Test plan\n\n- [x] `cargo test -p tests --test mlt --\nmlt_mixed_case_key_field_issue5065` — new regression test: `\"Id\"` /\n`\"Content\"` table, `key_field='Id'`, asserts `pdb.more_like_this(1)`\nreturns rows. Fails on `main` with `column \"id\" does not exist`, passes\nwith this change.\n- [x] `cargo test -p tests --test mlt` — existing\n`mlt_enables_scoring_issue1747`, `mlt_datetime_key`,\n`mlt_scoring_nested` still pass.\n- [x] `cargo pgrx regress -p pg_search --auto -- pg18 more_like_this` —\ngolden output unchanged (`quote_identifier(\"id\")` is a no-op for\nalready-lowercase identifiers).\n- [x] Manual repro from #5065 (`CREATE TABLE items (\"Id\" int primary\nkey, \"Content\" text); … pdb.more_like_this(1)`) returns rows instead of\nerroring.\n\nCloses #5065.",
          "timestamp": "2026-05-14T14:45:18-04:00",
          "tree_id": "812b4a66ffce7bc074cb919986f1de3b6474813f",
          "url": "https://github.com/paradedb/paradedb/commit/b9c06c5f75ca94cf815eb57c71c46180e84b8038"
        },
        "date": 1778788887117,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - cpu",
            "value": 4.5933013,
            "unit": "median cpu",
            "extra": "avg cpu: 5.047711540903169, max cpu: 9.402546, count: 53860"
          },
          {
            "name": "Custom Scan - Subscriber - mem",
            "value": 52.76171875,
            "unit": "median mem",
            "extra": "avg mem: 52.85444780159209, max mem: 58.93359375, count: 53860"
          },
          {
            "name": "Delete values - Publisher - cpu",
            "value": 4.5801525,
            "unit": "median cpu",
            "extra": "avg cpu: 3.9291705984804612, max cpu: 4.610951, count: 53860"
          },
          {
            "name": "Delete values - Publisher - mem",
            "value": 32.453125,
            "unit": "median mem",
            "extra": "avg mem: 31.77339550744987, max mem: 33.18359375, count: 53860"
          },
          {
            "name": "Find by ctid - Subscriber - cpu",
            "value": 9.151573,
            "unit": "median cpu",
            "extra": "avg cpu: 8.261153963461654, max cpu: 18.461538, count: 53860"
          },
          {
            "name": "Find by ctid - Subscriber - mem",
            "value": 55.5546875,
            "unit": "median mem",
            "extra": "avg mem: 55.309295946667284, max mem: 61.71484375, count: 53860"
          },
          {
            "name": "Index Only Scan - Subscriber - cpu",
            "value": 4.5933013,
            "unit": "median cpu",
            "extra": "avg cpu: 5.07798161104245, max cpu: 9.302325, count: 53860"
          },
          {
            "name": "Index Only Scan - Subscriber - mem",
            "value": 52.1640625,
            "unit": "median mem",
            "extra": "avg mem: 52.25051943116413, max mem: 58.3671875, count: 53860"
          },
          {
            "name": "Index Size Info - Subscriber - cpu",
            "value": 4.58891,
            "unit": "median cpu",
            "extra": "avg cpu: 4.645844990827023, max cpu: 9.221902, count: 53860"
          },
          {
            "name": "Index Size Info - Subscriber - mem",
            "value": 34.10546875,
            "unit": "median mem",
            "extra": "avg mem: 34.14847775192629, max mem: 39.34765625, count: 53860"
          },
          {
            "name": "Index Size Info - Subscriber - pages",
            "value": 1121,
            "unit": "median pages",
            "extra": "avg pages: 1129.589974006684, max pages: 1890.0, count: 53860"
          },
          {
            "name": "Index Size Info - Subscriber - relation_size:MB",
            "value": 8.7578125,
            "unit": "median relation_size:MB",
            "extra": "avg relation_size:MB: 8.82492167192722, max relation_size:MB: 14.765625, count: 53860"
          },
          {
            "name": "Index Size Info - Subscriber - segment_count",
            "value": 9,
            "unit": "median segment_count",
            "extra": "avg segment_count: 8.781433345711102, max segment_count: 17.0, count: 53860"
          },
          {
            "name": "Insert value A - Publisher - cpu",
            "value": 4.5801525,
            "unit": "median cpu",
            "extra": "avg cpu: 4.22253345243212, max cpu: 4.6153846, count: 53860"
          },
          {
            "name": "Insert value A - Publisher - mem",
            "value": 29.22265625,
            "unit": "median mem",
            "extra": "avg mem: 28.55854871135815, max mem: 29.69921875, count: 53860"
          },
          {
            "name": "Insert value B - Publisher - cpu",
            "value": 4.5454545,
            "unit": "median cpu",
            "extra": "avg cpu: 2.78078782469151, max cpu: 4.610951, count: 53860"
          },
          {
            "name": "Insert value B - Publisher - mem",
            "value": 29.21875,
            "unit": "median mem",
            "extra": "avg mem: 28.594308014992574, max mem: 29.74609375, count: 53860"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - cpu",
            "value": 4.6065254,
            "unit": "median cpu",
            "extra": "avg cpu: 6.5857090811865575, max cpu: 27.480915, count: 53860"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - mem",
            "value": 51.4921875,
            "unit": "median mem",
            "extra": "avg mem: 51.63217463388879, max mem: 57.765625, count: 53860"
          },
          {
            "name": "SELECT\n  pid,\n  pg_wal_lsn_diff(sent_lsn, replay_lsn) AS replication_lag,\n  application_name::text,\n  state::text\nFROM pg_stat_replication; - Publisher - replication_lag:MB",
            "value": 0,
            "unit": "median replication_lag:MB",
            "extra": "avg replication_lag:MB: 0.000024277934610201796, max replication_lag:MB: 0.063232421875, count: 53860"
          },
          {
            "name": "Top K - Subscriber - cpu",
            "value": 4.5933013,
            "unit": "median cpu",
            "extra": "avg cpu: 5.300384641976074, max cpu: 13.806328, count: 107720"
          },
          {
            "name": "Top K - Subscriber - mem",
            "value": 51.53125,
            "unit": "median mem",
            "extra": "avg mem: 51.61655115693465, max mem: 57.95703125, count: 107720"
          },
          {
            "name": "Update 1..9 - Publisher - cpu",
            "value": 4.494382,
            "unit": "median cpu",
            "extra": "avg cpu: 3.4808595238927817, max cpu: 4.628737, count: 53860"
          },
          {
            "name": "Update 1..9 - Publisher - mem",
            "value": 32.9140625,
            "unit": "median mem",
            "extra": "avg mem: 32.136263431813965, max mem: 32.96875, count: 53860"
          },
          {
            "name": "Update 10,11 - Publisher - cpu",
            "value": 4.567079,
            "unit": "median cpu",
            "extra": "avg cpu: 3.991236802854422, max cpu: 4.619827, count: 53860"
          },
          {
            "name": "Update 10,11 - Publisher - mem",
            "value": 33.203125,
            "unit": "median mem",
            "extra": "avg mem: 32.510448312175086, max mem: 33.62890625, count: 53860"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "balavignesh449@gmail.com",
            "name": "S Bala Vignesh",
            "username": "SBALAVIGNESH123"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "d0039460372b22539b33262920b0194c694b7205",
          "message": "fix: use fixed-length updates in stressgres to avoid TOAST and expose FSM race (#5080)\n\nThe old UPDATE pattern in the logical-replication stressgres suite\nappended txid_current() to the message column every iteration, growing\nit past the TOAST threshold (~2KB). This caused the suite to hit the\nunrelated TOAST visibility race (#5076) before the FSM segment metadata\nrace (#4935) could surface.\n\nChanged to fixed-length updates that keep the first search term and\nappend a small txid-derived number, staying well under the TOAST\nthreshold. This way the suite can run long enough to exercise the FSM\npath.\n\nRelated: #5067 (FSM race fix), #5076 (TOAST bug)",
          "timestamp": "2026-05-14T14:57:31-04:00",
          "tree_id": "96511f40645bb3416046b0914f9758c60c159a20",
          "url": "https://github.com/paradedb/paradedb/commit/d0039460372b22539b33262920b0194c694b7205"
        },
        "date": 1778789691817,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - cpu",
            "value": 4.597701,
            "unit": "median cpu",
            "extra": "avg cpu: 5.087363520582067, max cpu: 9.375, count: 53674"
          },
          {
            "name": "Custom Scan - Subscriber - mem",
            "value": 52.87890625,
            "unit": "median mem",
            "extra": "avg mem: 52.90439359606141, max mem: 58.8125, count: 53674"
          },
          {
            "name": "Delete values - Publisher - cpu",
            "value": 4.5933013,
            "unit": "median cpu",
            "extra": "avg cpu: 2.8497973151266196, max cpu: 4.610951, count: 53674"
          },
          {
            "name": "Delete values - Publisher - mem",
            "value": 32.046875,
            "unit": "median mem",
            "extra": "avg mem: 31.394718797159705, max mem: 32.45703125, count: 53674"
          },
          {
            "name": "Find by ctid - Subscriber - cpu",
            "value": 9.186603,
            "unit": "median cpu",
            "extra": "avg cpu: 9.126125619832562, max cpu: 23.4375, count: 53674"
          },
          {
            "name": "Find by ctid - Subscriber - mem",
            "value": 55.88671875,
            "unit": "median mem",
            "extra": "avg mem: 55.54191158661549, max mem: 61.73828125, count: 53674"
          },
          {
            "name": "Index Only Scan - Subscriber - cpu",
            "value": 4.597701,
            "unit": "median cpu",
            "extra": "avg cpu: 5.110877929657153, max cpu: 9.375, count: 53674"
          },
          {
            "name": "Index Only Scan - Subscriber - mem",
            "value": 52.3046875,
            "unit": "median mem",
            "extra": "avg mem: 52.2826965220591, max mem: 58.2109375, count: 53674"
          },
          {
            "name": "Index Size Info - Subscriber - cpu",
            "value": 4.597701,
            "unit": "median cpu",
            "extra": "avg cpu: 4.639914989259452, max cpu: 9.275363, count: 53674"
          },
          {
            "name": "Index Size Info - Subscriber - mem",
            "value": 33.35546875,
            "unit": "median mem",
            "extra": "avg mem: 33.37421851711257, max mem: 38.53515625, count: 53674"
          },
          {
            "name": "Index Size Info - Subscriber - pages",
            "value": 1130,
            "unit": "median pages",
            "extra": "avg pages: 1128.6634124529567, max pages: 1864.0, count: 53674"
          },
          {
            "name": "Index Size Info - Subscriber - relation_size:MB",
            "value": 8.828125,
            "unit": "median relation_size:MB",
            "extra": "avg relation_size:MB: 8.81768305534337, max relation_size:MB: 14.5625, count: 53674"
          },
          {
            "name": "Index Size Info - Subscriber - segment_count",
            "value": 12,
            "unit": "median segment_count",
            "extra": "avg segment_count: 11.806833848790848, max segment_count: 20.0, count: 53674"
          },
          {
            "name": "Insert value A - Publisher - cpu",
            "value": 4.5933013,
            "unit": "median cpu",
            "extra": "avg cpu: 4.526772442370834, max cpu: 4.6376815, count: 53674"
          },
          {
            "name": "Insert value A - Publisher - mem",
            "value": 29.359375,
            "unit": "median mem",
            "extra": "avg mem: 28.617628603350784, max mem: 29.83984375, count: 53674"
          },
          {
            "name": "Insert value B - Publisher - cpu",
            "value": 4.562738,
            "unit": "median cpu",
            "extra": "avg cpu: 3.696220174487456, max cpu: 4.6153846, count: 53674"
          },
          {
            "name": "Insert value B - Publisher - mem",
            "value": 29.21484375,
            "unit": "median mem",
            "extra": "avg mem: 28.54259176347021, max mem: 29.55078125, count: 53674"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - cpu",
            "value": 4.610951,
            "unit": "median cpu",
            "extra": "avg cpu: 6.198590813230149, max cpu: 23.099133, count: 53674"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - mem",
            "value": 51.015625,
            "unit": "median mem",
            "extra": "avg mem: 51.02103512058911, max mem: 56.93359375, count: 53674"
          },
          {
            "name": "SELECT\n  pid,\n  pg_wal_lsn_diff(sent_lsn, replay_lsn) AS replication_lag,\n  application_name::text,\n  state::text\nFROM pg_stat_replication; - Publisher - replication_lag:MB",
            "value": 0,
            "unit": "median replication_lag:MB",
            "extra": "avg replication_lag:MB: 0.00001986422899268035, max replication_lag:MB: 0.11859893798828125, count: 53674"
          },
          {
            "name": "Top K - Subscriber - cpu",
            "value": 4.6021094,
            "unit": "median cpu",
            "extra": "avg cpu: 5.389240587261424, max cpu: 14.0625, count: 107348"
          },
          {
            "name": "Top K - Subscriber - mem",
            "value": 52.3359375,
            "unit": "median mem",
            "extra": "avg mem: 52.262934058505984, max mem: 58.9375, count: 107348"
          },
          {
            "name": "Update 1..9 - Publisher - cpu",
            "value": 4.58891,
            "unit": "median cpu",
            "extra": "avg cpu: 4.453653511134155, max cpu: 4.6332045, count: 53674"
          },
          {
            "name": "Update 1..9 - Publisher - mem",
            "value": 33.10546875,
            "unit": "median mem",
            "extra": "avg mem: 32.38584207437493, max mem: 33.42578125, count: 53674"
          },
          {
            "name": "Update 10,11 - Publisher - cpu",
            "value": 4.5845275,
            "unit": "median cpu",
            "extra": "avg cpu: 4.199617696942711, max cpu: 4.628737, count: 53674"
          },
          {
            "name": "Update 10,11 - Publisher - mem",
            "value": 32.9140625,
            "unit": "median mem",
            "extra": "avg mem: 32.21558658812926, max mem: 33.00390625, count: 53674"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "balavignesh449@gmail.com",
            "name": "S Bala Vignesh",
            "username": "SBALAVIGNESH123"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "b597b183d25a561cf8c81d81decc7f07b7866e55",
          "message": "fix: hold header shared lock during LinkedItemList iteration to prevent FSM race (#4935) (#5067)\n\n## Problem\n\nLinkedItemList read-only iteration methods (list(), is_empty(),\nor_each(), lookup()/lookup_ex()) were releasing the header shared lock\nbefore iterating through the linked list blocks. This allowed\nAtomicGuard::commit() to:\n\n1. Swap the header pointer to a new list\n2. Immediately recycle old blocks to the FSM with\nReadNextFullTransactionId()\n3. The next transaction pops a recycled block via RBM_ZERO_AND_LOCK,\nzeroing it\n4. A concurrent reader still traversing the old list hits the\nzeroed/repurposed block\n5. **SIGSEGV** or SegmentMetaEntryHeader: UnexpectedEnd deserialization\nerror\n\nThis manifests as periodic crashes and durable index corruption under\nsustained write traffic, particularly on logical replication subscribers\nat high apply rates (~395 commits/sec). Correlates strongly with\nautovacuum events that trigger garbage_collect_index().\n\n## Root Cause\n\nThe race window exists because or_each(), list(), is_empty(), and\nlookup_ex() call get_start_blockno() which acquires a shared lock on the\nheader, reads start_blockno, then immediately releases the header lock\nwhen exchanging to the first data block. After that point, \u0007tomically()\ncan take an exclusive header lock and proceed with the swap+recycle\nwhile the reader is deep in the old list.\n\n**This was already a known pattern in the codebase** — emove_item() and\nupdate_item() in the same file both hold the header shared lock for\ntheir entire operation with this comment:\n\n\\\\\\\rust\n// Acquire and hold a shared lock on the header for the entire\noperation, preventing the\n// list from being swapped out from under us by atomically between our\nread locks and\n// our write locks.\nlet header_lock = self.bman.get_buffer(self.header_blockno);\n\\\\\\\n\nThe read-only methods simply weren't given the same treatment.\n\n## Fix\n\nHold a shared lock on the header for the entire duration of iteration in\nall 4 methods, matching the existing emove_item()/update_item() pattern:\n\n- **list()** — hold header_lock from start to end of iteration\n- **is_empty()** — same\n- **\for_each()** — same\n- **lookup_ex()** — conditionally: only when \blockno is None (top-level\ncall). When \blockno is Some, the caller ( emove_item/update_item)\nalready holds the header lock\n\nRead start_blockno directly from the already-held header_lock instead of\ncalling get_start_blockno(), avoiding a double shared-lock acquisition\non the same block (which would trigger a panic under the \block_tracker\ndebug feature).\n\n## Why This Is Safe\n\n- **No deadlock**: Header block is always locked first, content blocks\nin ascending order — consistent lock ordering\n- **No reader-reader blocking**: Multiple readers hold shared locks\nconcurrently (shared locks are compatible)\n- **Writer waits for readers**: \u0007tomically() takes an exclusive header\nlock, which blocks until all shared locks are released — correct\nserialization\n- **Minimal performance impact**: The header lock was already acquired;\nwe just hold it slightly longer\n\n## Verification\n\n- \rustfmt --check passes\n- Pattern matches the proven emove_item()/update_item() implementation\nin the same file\n- Full cargo check requires pgrx setup (PostgreSQL extension); the\nchange is limited to lock lifetime management with no new APIs\n\nCloses #4935",
          "timestamp": "2026-05-14T15:00:05-04:00",
          "tree_id": "46e474245958de09c21d0198195343d8a87fb72d",
          "url": "https://github.com/paradedb/paradedb/commit/b597b183d25a561cf8c81d81decc7f07b7866e55"
        },
        "date": 1778789777585,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - cpu",
            "value": 4.5845275,
            "unit": "median cpu",
            "extra": "avg cpu: 5.06465576989025, max cpu: 9.275363, count: 53854"
          },
          {
            "name": "Custom Scan - Subscriber - mem",
            "value": 53.2421875,
            "unit": "median mem",
            "extra": "avg mem: 53.26656772535466, max mem: 58.7421875, count: 53854"
          },
          {
            "name": "Delete values - Publisher - cpu",
            "value": 4.5540795,
            "unit": "median cpu",
            "extra": "avg cpu: 4.173810310312371, max cpu: 4.567079, count: 53854"
          },
          {
            "name": "Delete values - Publisher - mem",
            "value": 32.1015625,
            "unit": "median mem",
            "extra": "avg mem: 31.402349335123667, max mem: 32.453125, count: 53854"
          },
          {
            "name": "Find by ctid - Subscriber - cpu",
            "value": 9.142857,
            "unit": "median cpu",
            "extra": "avg cpu: 8.417443329890002, max cpu: 18.408438, count: 53854"
          },
          {
            "name": "Find by ctid - Subscriber - mem",
            "value": 55.55859375,
            "unit": "median mem",
            "extra": "avg mem: 55.204880324581275, max mem: 61.00390625, count: 53854"
          },
          {
            "name": "Index Only Scan - Subscriber - cpu",
            "value": 4.5845275,
            "unit": "median cpu",
            "extra": "avg cpu: 5.072619791359374, max cpu: 9.284333, count: 53854"
          },
          {
            "name": "Index Only Scan - Subscriber - mem",
            "value": 51.8515625,
            "unit": "median mem",
            "extra": "avg mem: 51.90177312444294, max mem: 57.37109375, count: 53854"
          },
          {
            "name": "Index Size Info - Subscriber - cpu",
            "value": 4.5801525,
            "unit": "median cpu",
            "extra": "avg cpu: 4.651626955549621, max cpu: 9.230769, count: 53854"
          },
          {
            "name": "Index Size Info - Subscriber - mem",
            "value": 32.89453125,
            "unit": "median mem",
            "extra": "avg mem: 32.89963365940784, max mem: 37.65625, count: 53854"
          },
          {
            "name": "Index Size Info - Subscriber - pages",
            "value": 1068,
            "unit": "median pages",
            "extra": "avg pages: 1072.463976677684, max pages: 1754.0, count: 53854"
          },
          {
            "name": "Index Size Info - Subscriber - relation_size:MB",
            "value": 8.34375,
            "unit": "median relation_size:MB",
            "extra": "avg relation_size:MB: 8.378624817794407, max relation_size:MB: 13.703125, count: 53854"
          },
          {
            "name": "Index Size Info - Subscriber - segment_count",
            "value": 9,
            "unit": "median segment_count",
            "extra": "avg segment_count: 8.92006164815984, max segment_count: 15.0, count: 53854"
          },
          {
            "name": "Insert value A - Publisher - cpu",
            "value": 4.597701,
            "unit": "median cpu",
            "extra": "avg cpu: 3.6361638430259333, max cpu: 4.610951, count: 53854"
          },
          {
            "name": "Insert value A - Publisher - mem",
            "value": 29.26171875,
            "unit": "median mem",
            "extra": "avg mem: 28.571706387292494, max mem: 29.625, count: 53854"
          },
          {
            "name": "Insert value B - Publisher - cpu",
            "value": 4.5845275,
            "unit": "median cpu",
            "extra": "avg cpu: 4.290881248683797, max cpu: 4.6153846, count: 53854"
          },
          {
            "name": "Insert value B - Publisher - mem",
            "value": 29.359375,
            "unit": "median mem",
            "extra": "avg mem: 28.62782201066773, max mem: 29.7265625, count: 53854"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - cpu",
            "value": 4.597701,
            "unit": "median cpu",
            "extra": "avg cpu: 6.367590921338712, max cpu: 23.010548, count: 53854"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - mem",
            "value": 50.578125,
            "unit": "median mem",
            "extra": "avg mem: 50.637789715596796, max mem: 56.0703125, count: 53854"
          },
          {
            "name": "SELECT\n  pid,\n  pg_wal_lsn_diff(sent_lsn, replay_lsn) AS replication_lag,\n  application_name::text,\n  state::text\nFROM pg_stat_replication; - Publisher - replication_lag:MB",
            "value": 0,
            "unit": "median replication_lag:MB",
            "extra": "avg replication_lag:MB: 0.000014919777269257506, max replication_lag:MB: 0.10605621337890625, count: 53854"
          },
          {
            "name": "Top K - Subscriber - cpu",
            "value": 4.5845275,
            "unit": "median cpu",
            "extra": "avg cpu: 5.319799923371453, max cpu: 13.740458, count: 107708"
          },
          {
            "name": "Top K - Subscriber - mem",
            "value": 51.375,
            "unit": "median mem",
            "extra": "avg mem: 51.42435051539811, max mem: 57.1875, count: 107708"
          },
          {
            "name": "Update 1..9 - Publisher - cpu",
            "value": 4.5801525,
            "unit": "median cpu",
            "extra": "avg cpu: 4.1866271302751015, max cpu: 4.610951, count: 53854"
          },
          {
            "name": "Update 1..9 - Publisher - mem",
            "value": 32.96875,
            "unit": "median mem",
            "extra": "avg mem: 32.32353867052586, max mem: 33.33203125, count: 53854"
          },
          {
            "name": "Update 10,11 - Publisher - cpu",
            "value": 4.58891,
            "unit": "median cpu",
            "extra": "avg cpu: 4.129032005396621, max cpu: 4.6065254, count: 53854"
          },
          {
            "name": "Update 10,11 - Publisher - mem",
            "value": 32.83203125,
            "unit": "median mem",
            "extra": "avg mem: 32.12852501090912, max mem: 32.9375, count: 53854"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "652b0952a7f3be58ea4b9fa80a863426cf03185c",
          "message": "chore(stressgres): remove dormant suites not wired into CI (#5085)\n\n## Summary\n\n- Removes 8 Stressgres suites that have not been touched since\nStressgres was added (#3821), are not referenced by any CI workflow or\ndriver script, and use the legacy `[setup_primary]` schema rather than\nthe current `[[server]]` schema.\n- No CI workflow changes; the 6 suites currently exercised by\n`benchmark-pg_search-stressgres.yml` (`single-server`, `bulk-updates`,\n`wide-table`, `background-merge`, `logical-replication`,\n`logical-replication-merge`) and `vanilla-postgres.toml` (used by the\nantithesis singleton driver) are kept.\n\n### Removed\n- `large-inserts.toml`\n- `lr.toml` — predecessor of `logical-replication.toml`\n- `lr-graphable.toml`\n- `lr-large-inserts.toml`\n- `lr-no-pg_search.toml`\n- `many-updates.toml` — conceptually useful (MVCC correctness with\n`assert(count, expected)` under concurrency), but on legacy schema; if\nwe want this coverage in CI it should be a deliberate port, not a\nrevival of dead config\n- `read-write.toml` — contained placeholder `<password>` literal, not\nCI-runnable\n- `topk-crash.toml`\n\n### Why now\nCompanion to #5080 (which adjusts `logical-replication.toml` to expose\nthe FSM race instead of being masked by the TOAST bug already covered by\n`logical-replication-merge.toml`). Each CI suite should own a distinct\nrepro; dormant files muddy that mapping.\n\n## Test plan\n- [x] `benchmark-pg_search-stressgres` workflow still kicks off for all\n6 in-CI suites\n- [x] Antithesis `singleton_driver_vanilla-postgres.sh` still resolves\nits suite path",
          "timestamp": "2026-05-14T15:45:37-04:00",
          "tree_id": "090e72f7bbb9817642fb0c7495c2b12e0eed2fe7",
          "url": "https://github.com/paradedb/paradedb/commit/652b0952a7f3be58ea4b9fa80a863426cf03185c"
        },
        "date": 1778792631705,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - cpu",
            "value": 4.5845275,
            "unit": "median cpu",
            "extra": "avg cpu: 5.0675756210946155, max cpu: 9.275363, count: 53870"
          },
          {
            "name": "Custom Scan - Subscriber - mem",
            "value": 53.1328125,
            "unit": "median mem",
            "extra": "avg mem: 53.135238914284386, max mem: 59.03515625, count: 53870"
          },
          {
            "name": "Delete values - Publisher - cpu",
            "value": 4.58891,
            "unit": "median cpu",
            "extra": "avg cpu: 4.46079145620146, max cpu: 4.58891, count: 53870"
          },
          {
            "name": "Delete values - Publisher - mem",
            "value": 32.04296875,
            "unit": "median mem",
            "extra": "avg mem: 31.326078623886207, max mem: 32.32421875, count: 53870"
          },
          {
            "name": "Find by ctid - Subscriber - cpu",
            "value": 9.116809,
            "unit": "median cpu",
            "extra": "avg cpu: 7.7211704764018165, max cpu: 18.461538, count: 53870"
          },
          {
            "name": "Find by ctid - Subscriber - mem",
            "value": 55.96484375,
            "unit": "median mem",
            "extra": "avg mem: 55.63185794505291, max mem: 61.92578125, count: 53870"
          },
          {
            "name": "Index Only Scan - Subscriber - cpu",
            "value": 4.5845275,
            "unit": "median cpu",
            "extra": "avg cpu: 5.088227265746156, max cpu: 9.266409, count: 53870"
          },
          {
            "name": "Index Only Scan - Subscriber - mem",
            "value": 52.27734375,
            "unit": "median mem",
            "extra": "avg mem: 52.25836823603119, max mem: 58.1796875, count: 53870"
          },
          {
            "name": "Index Size Info - Subscriber - cpu",
            "value": 4.5801525,
            "unit": "median cpu",
            "extra": "avg cpu: 4.622492382768815, max cpu: 9.195402, count: 53870"
          },
          {
            "name": "Index Size Info - Subscriber - mem",
            "value": 34.03125,
            "unit": "median mem",
            "extra": "avg mem: 33.99163952280954, max mem: 39.0859375, count: 53870"
          },
          {
            "name": "Index Size Info - Subscriber - pages",
            "value": 1118,
            "unit": "median pages",
            "extra": "avg pages: 1115.9302209021719, max pages: 1855.0, count: 53870"
          },
          {
            "name": "Index Size Info - Subscriber - relation_size:MB",
            "value": 8.734375,
            "unit": "median relation_size:MB",
            "extra": "avg relation_size:MB: 8.718204850798218, max relation_size:MB: 14.4921875, count: 53870"
          },
          {
            "name": "Index Size Info - Subscriber - segment_count",
            "value": 8,
            "unit": "median segment_count",
            "extra": "avg segment_count: 8.055541117505104, max segment_count: 13.0, count: 53870"
          },
          {
            "name": "Insert value A - Publisher - cpu",
            "value": 4.58891,
            "unit": "median cpu",
            "extra": "avg cpu: 4.288567152151915, max cpu: 4.6153846, count: 53870"
          },
          {
            "name": "Insert value A - Publisher - mem",
            "value": 29.1796875,
            "unit": "median mem",
            "extra": "avg mem: 28.46049727642937, max mem: 29.55078125, count: 53870"
          },
          {
            "name": "Insert value B - Publisher - cpu",
            "value": 4.5540795,
            "unit": "median cpu",
            "extra": "avg cpu: 4.512721107362955, max cpu: 4.6065254, count: 53870"
          },
          {
            "name": "Insert value B - Publisher - mem",
            "value": 29.1328125,
            "unit": "median mem",
            "extra": "avg mem: 28.419054045038983, max mem: 29.484375, count: 53870"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - cpu",
            "value": 4.5933013,
            "unit": "median cpu",
            "extra": "avg cpu: 6.146690856494597, max cpu: 22.900763, count: 53870"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - mem",
            "value": 51.20703125,
            "unit": "median mem",
            "extra": "avg mem: 51.161940130754594, max mem: 57.07421875, count: 53870"
          },
          {
            "name": "SELECT\n  pid,\n  pg_wal_lsn_diff(sent_lsn, replay_lsn) AS replication_lag,\n  application_name::text,\n  state::text\nFROM pg_stat_replication; - Publisher - replication_lag:MB",
            "value": 0,
            "unit": "median replication_lag:MB",
            "extra": "avg replication_lag:MB: 0.00003479765854320877, max replication_lag:MB: 0.31896209716796875, count: 53870"
          },
          {
            "name": "Top K - Subscriber - cpu",
            "value": 4.58891,
            "unit": "median cpu",
            "extra": "avg cpu: 5.338102041503427, max cpu: 13.899614, count: 107740"
          },
          {
            "name": "Top K - Subscriber - mem",
            "value": 51.9140625,
            "unit": "median mem",
            "extra": "avg mem: 51.85302362748283, max mem: 58.1015625, count: 107740"
          },
          {
            "name": "Update 1..9 - Publisher - cpu",
            "value": 4.5714283,
            "unit": "median cpu",
            "extra": "avg cpu: 4.4915020219304616, max cpu: 4.6021094, count: 53870"
          },
          {
            "name": "Update 1..9 - Publisher - mem",
            "value": 32.98828125,
            "unit": "median mem",
            "extra": "avg mem: 32.26220364128921, max mem: 33.328125, count: 53870"
          },
          {
            "name": "Update 10,11 - Publisher - cpu",
            "value": 4.5454545,
            "unit": "median cpu",
            "extra": "avg cpu: 4.2556883986337555, max cpu: 4.6065254, count: 53870"
          },
          {
            "name": "Update 10,11 - Publisher - mem",
            "value": 33.05078125,
            "unit": "median mem",
            "extra": "avg mem: 32.34186133863932, max mem: 33.16796875, count: 53870"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ming.ying.nyc@gmail.com",
            "name": "Ming",
            "username": "rebasedming"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "21bdc9a753582039cc25a38cb07111026c0fd378",
          "message": "feat: Crash recovery via WAL (#4901)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nPorts WAL integration over to community, which gives `pg_search` crash\nrecovery.\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2026-05-14T14:59:53-07:00",
          "tree_id": "7580b5f2bfcc1a95ed1b62c4628f2ec6129b7c00",
          "url": "https://github.com/paradedb/paradedb/commit/21bdc9a753582039cc25a38cb07111026c0fd378"
        },
        "date": 1778800678556,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - cpu",
            "value": 4.5801525,
            "unit": "median cpu",
            "extra": "avg cpu: 5.070768995454957, max cpu: 9.266409, count: 53868"
          },
          {
            "name": "Custom Scan - Subscriber - mem",
            "value": 53.29296875,
            "unit": "median mem",
            "extra": "avg mem: 53.32245771782413, max mem: 59.3359375, count: 53868"
          },
          {
            "name": "Delete values - Publisher - cpu",
            "value": 4.5801525,
            "unit": "median cpu",
            "extra": "avg cpu: 3.215519993139449, max cpu: 4.597701, count: 53868"
          },
          {
            "name": "Delete values - Publisher - mem",
            "value": 32.1640625,
            "unit": "median mem",
            "extra": "avg mem: 31.510534358872615, max mem: 32.6484375, count: 53868"
          },
          {
            "name": "Find by ctid - Subscriber - cpu",
            "value": 9.142857,
            "unit": "median cpu",
            "extra": "avg cpu: 8.629306453566093, max cpu: 23.054754, count: 53868"
          },
          {
            "name": "Find by ctid - Subscriber - mem",
            "value": 55.95703125,
            "unit": "median mem",
            "extra": "avg mem: 55.667464044052124, max mem: 61.921875, count: 53868"
          },
          {
            "name": "Index Only Scan - Subscriber - cpu",
            "value": 4.5801525,
            "unit": "median cpu",
            "extra": "avg cpu: 5.0888131754670365, max cpu: 9.284333, count: 53868"
          },
          {
            "name": "Index Only Scan - Subscriber - mem",
            "value": 52.40625,
            "unit": "median mem",
            "extra": "avg mem: 52.43041642301552, max mem: 58.453125, count: 53868"
          },
          {
            "name": "Index Size Info - Subscriber - cpu",
            "value": 4.5757866,
            "unit": "median cpu",
            "extra": "avg cpu: 4.638934309961094, max cpu: 9.17782, count: 53868"
          },
          {
            "name": "Index Size Info - Subscriber - mem",
            "value": 33.21875,
            "unit": "median mem",
            "extra": "avg mem: 33.26824497498515, max mem: 38.5859375, count: 53868"
          },
          {
            "name": "Index Size Info - Subscriber - pages",
            "value": 1110,
            "unit": "median pages",
            "extra": "avg pages: 1111.2501113833816, max pages: 1856.0, count: 53868"
          },
          {
            "name": "Index Size Info - Subscriber - relation_size:MB",
            "value": 8.671875,
            "unit": "median relation_size:MB",
            "extra": "avg relation_size:MB: 8.681641495182669, max relation_size:MB: 14.5, count: 53868"
          },
          {
            "name": "Index Size Info - Subscriber - segment_count",
            "value": 10,
            "unit": "median segment_count",
            "extra": "avg segment_count: 10.021218534194698, max segment_count: 15.0, count: 53868"
          },
          {
            "name": "Insert value A - Publisher - cpu",
            "value": 4.58891,
            "unit": "median cpu",
            "extra": "avg cpu: 3.4246876127099624, max cpu: 4.5933013, count: 53868"
          },
          {
            "name": "Insert value A - Publisher - mem",
            "value": 29.47265625,
            "unit": "median mem",
            "extra": "avg mem: 28.849195458110568, max mem: 29.90625, count: 53868"
          },
          {
            "name": "Insert value B - Publisher - cpu",
            "value": 4.5714283,
            "unit": "median cpu",
            "extra": "avg cpu: 4.475603385841937, max cpu: 4.5933013, count: 53868"
          },
          {
            "name": "Insert value B - Publisher - mem",
            "value": 29.49609375,
            "unit": "median mem",
            "extra": "avg mem: 28.830158254320747, max mem: 29.86328125, count: 53868"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - cpu",
            "value": 4.58891,
            "unit": "median cpu",
            "extra": "avg cpu: 6.035548208854177, max cpu: 27.195469, count: 53868"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - mem",
            "value": 51.85546875,
            "unit": "median mem",
            "extra": "avg mem: 51.86564763925243, max mem: 57.84765625, count: 53868"
          },
          {
            "name": "SELECT\n  pid,\n  pg_wal_lsn_diff(sent_lsn, replay_lsn) AS replication_lag,\n  application_name::text,\n  state::text\nFROM pg_stat_replication; - Publisher - replication_lag:MB",
            "value": 0,
            "unit": "median replication_lag:MB",
            "extra": "avg replication_lag:MB: 0.00001874051115653948, max replication_lag:MB: 0.05022430419921875, count: 53868"
          },
          {
            "name": "Top K - Subscriber - cpu",
            "value": 4.5801525,
            "unit": "median cpu",
            "extra": "avg cpu: 5.232830709810732, max cpu: 13.832853, count: 107736"
          },
          {
            "name": "Top K - Subscriber - mem",
            "value": 51.859375,
            "unit": "median mem",
            "extra": "avg mem: 51.9289939281554, max mem: 58.44140625, count: 107736"
          },
          {
            "name": "Update 1..9 - Publisher - cpu",
            "value": 4.5714283,
            "unit": "median cpu",
            "extra": "avg cpu: 4.386891571463513, max cpu: 4.610951, count: 53868"
          },
          {
            "name": "Update 1..9 - Publisher - mem",
            "value": 33.171875,
            "unit": "median mem",
            "extra": "avg mem: 32.544016304902726, max mem: 33.67578125, count: 53868"
          },
          {
            "name": "Update 10,11 - Publisher - cpu",
            "value": 4.6021094,
            "unit": "median cpu",
            "extra": "avg cpu: 4.55475455294629, max cpu: 4.610951, count: 53868"
          },
          {
            "name": "Update 10,11 - Publisher - mem",
            "value": 32.9921875,
            "unit": "median mem",
            "extra": "avg mem: 32.32203886989957, max mem: 33.15625, count: 53868"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "d99908a1f58307567dd34698345b3fc836be6135",
          "message": "feat(antithesis): add singleton drivers for logical-replication suites (#5087)\n\n## Summary\n\nAdds OSS Antithesis singleton drivers for the two CI\n`logical-replication` suites that previously had no singleton\n(`single-server`, `bulk-updates`, `wide-table`, `background-merge`, and\n`vanilla-postgres` already had one).\n\nMirrors the enterprise pattern used for `physical-logical-replication`:\n- A **vanilla Postgres 18** publisher pod (with `wal_level=logical`)\nthat lives outside the CNPG cluster, reflecting real-world\nlogical-replication topologies where the upstream primary is not under\nour control.\n- Subscriber points at `paradedb-rw` (the CNPG primary, which has\n`pg_search`).\n\n## Changes\n- `docker/manifests/antithesis-paradedb.yaml` — add\n`logical-replication-publisher` Service + Deployment (vanilla Postgres\n18 with `wal_level=logical`), reusing the existing `paradedb-superuser`\nsecret.\n- `stressgres/suites/logical-replication.toml`,\n`stressgres/suites/logical-replication-merge.toml` — drop `CREATE\nEXTENSION pg_search` from the **Publisher** setup. Only the Subscriber\nuses `pg_search`; the line was cosmetic and incompatible with a vanilla\nPostgres publisher (the line in the Subscriber setup is unchanged).\n-\n`stressgres/suites/antithesis/singleton_driver_logical-replication.sh`,\n`singleton_driver_logical-replication-merge.sh` — new drivers that\nperform per-block `sed -z` rewrites of the `[server.style.Automatic]`\nblocks into `[server.style.With]` connection strings (Publisher →\n`logical-replication-publisher:5432`, Subscriber → `paradedb-rw:5432`).\n- `.github/workflows/antithesis-trigger-test-run.yml` — add\n`logical-replication-publisher` to\n`container_faults_stop_exclusion_patterns` and\n`container_faults_kill_exclusion_patterns`, matching enterprise. Network\nfaults to/from the publisher are intentionally still injected.\n\n## Why\nWithout these, the FSM race repro in `logical-replication-merge.toml`\n(issue #4935, fixed by #5067) and the broader logical-replication\ncoverage in `logical-replication.toml` were running in\n`benchmark-pg_search-stressgres` but had no Antithesis fault-injection\nequivalent — that's the half of the matrix where the bugs originally\nsurfaced.\n\n## Test plan\n- [x] Antithesis trigger workflow picks up both new singleton drivers\nfrom `/opt/antithesis/test/v1/quickstart/`\n- [x] Publisher pod (`logical-replication-publisher`) starts with\n`wal_level=logical` and is reachable from the stressgres-runner pod\n- [x] Subscriber's `CREATE SUBSCRIPTION ... CONNECTION\n'@Publisher_CONNSTR@'` resolves to the publisher pod after the `sed`\nrewrite\n- [x] `logical-replication-merge.toml` still reproduces the FSM race\nwhen run against a build without #5067\n- [x] `benchmark-pg_search-stressgres` (local Stressgres, not\nAntithesis) still runs both suites unchanged",
          "timestamp": "2026-05-14T18:54:16-04:00",
          "tree_id": "8c7a6dab334db43248e88e7a4ebc75fd5840e446",
          "url": "https://github.com/paradedb/paradedb/commit/d99908a1f58307567dd34698345b3fc836be6135"
        },
        "date": 1778803844321,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Subscriber - cpu",
            "value": 4.5845275,
            "unit": "median cpu",
            "extra": "avg cpu: 5.055622691767608, max cpu: 9.284333, count: 53878"
          },
          {
            "name": "Custom Scan - Subscriber - mem",
            "value": 53.06640625,
            "unit": "median mem",
            "extra": "avg mem: 53.09201155272096, max mem: 58.921875, count: 53878"
          },
          {
            "name": "Delete values - Publisher - cpu",
            "value": 4.5584044,
            "unit": "median cpu",
            "extra": "avg cpu: 3.027860820851554, max cpu: 4.610951, count: 53878"
          },
          {
            "name": "Delete values - Publisher - mem",
            "value": 29.8984375,
            "unit": "median mem",
            "extra": "avg mem: 29.202233083308585, max mem: 30.34765625, count: 53878"
          },
          {
            "name": "Find by ctid - Subscriber - cpu",
            "value": 9.142857,
            "unit": "median cpu",
            "extra": "avg cpu: 8.322599297442848, max cpu: 18.461538, count: 53878"
          },
          {
            "name": "Find by ctid - Subscriber - mem",
            "value": 55.7734375,
            "unit": "median mem",
            "extra": "avg mem: 55.53070493174394, max mem: 61.62109375, count: 53878"
          },
          {
            "name": "Index Only Scan - Subscriber - cpu",
            "value": 4.5845275,
            "unit": "median cpu",
            "extra": "avg cpu: 5.073312114210744, max cpu: 9.284333, count: 53878"
          },
          {
            "name": "Index Only Scan - Subscriber - mem",
            "value": 52.125,
            "unit": "median mem",
            "extra": "avg mem: 52.1740646257517, max mem: 58.04296875, count: 53878"
          },
          {
            "name": "Index Size Info - Subscriber - cpu",
            "value": 4.5845275,
            "unit": "median cpu",
            "extra": "avg cpu: 4.655208964307286, max cpu: 9.239654, count: 53878"
          },
          {
            "name": "Index Size Info - Subscriber - mem",
            "value": 33.1640625,
            "unit": "median mem",
            "extra": "avg mem: 33.1941181784541, max mem: 38.125, count: 53878"
          },
          {
            "name": "Index Size Info - Subscriber - pages",
            "value": 1088,
            "unit": "median pages",
            "extra": "avg pages: 1087.967129440588, max pages: 1814.0, count: 53878"
          },
          {
            "name": "Index Size Info - Subscriber - relation_size:MB",
            "value": 8.5,
            "unit": "median relation_size:MB",
            "extra": "avg relation_size:MB: 8.49974334375812, max relation_size:MB: 14.171875, count: 53878"
          },
          {
            "name": "Index Size Info - Subscriber - segment_count",
            "value": 8,
            "unit": "median segment_count",
            "extra": "avg segment_count: 9.208916440847842, max segment_count: 20.0, count: 53878"
          },
          {
            "name": "Insert value A - Publisher - cpu",
            "value": 4.562738,
            "unit": "median cpu",
            "extra": "avg cpu: 3.6589385186826866, max cpu: 4.6021094, count: 53878"
          },
          {
            "name": "Insert value A - Publisher - mem",
            "value": 29.15234375,
            "unit": "median mem",
            "extra": "avg mem: 28.409559487986748, max mem: 29.55078125, count: 53878"
          },
          {
            "name": "Insert value B - Publisher - cpu",
            "value": 4.567079,
            "unit": "median cpu",
            "extra": "avg cpu: 4.344290110958144, max cpu: 4.58891, count: 53878"
          },
          {
            "name": "Insert value B - Publisher - mem",
            "value": 29.09375,
            "unit": "median mem",
            "extra": "avg mem: 28.424835029487916, max mem: 29.5390625, count: 53878"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - cpu",
            "value": 4.6021094,
            "unit": "median cpu",
            "extra": "avg cpu: 6.502423798528388, max cpu: 23.188406, count: 53878"
          },
          {
            "name": "Parallel Custom Scan - Subscriber - mem",
            "value": 51.5625,
            "unit": "median mem",
            "extra": "avg mem: 51.637476103418834, max mem: 57.51171875, count: 53878"
          },
          {
            "name": "SELECT\n  pid,\n  pg_wal_lsn_diff(sent_lsn, replay_lsn) AS replication_lag,\n  application_name::text,\n  state::text\nFROM pg_stat_replication; - Publisher - replication_lag:MB",
            "value": 0,
            "unit": "median replication_lag:MB",
            "extra": "avg replication_lag:MB: 0.0000405474607236198, max replication_lag:MB: 0.31899261474609375, count: 53878"
          },
          {
            "name": "Top K - Subscriber - cpu",
            "value": 4.58891,
            "unit": "median cpu",
            "extra": "avg cpu: 5.256194703650474, max cpu: 13.766731, count: 107756"
          },
          {
            "name": "Top K - Subscriber - mem",
            "value": 51.7109375,
            "unit": "median mem",
            "extra": "avg mem: 51.8023479768528, max mem: 57.93359375, count: 107756"
          },
          {
            "name": "Update 1..9 - Publisher - cpu",
            "value": 4.5933013,
            "unit": "median cpu",
            "extra": "avg cpu: 4.445348401510888, max cpu: 4.624277, count: 53878"
          },
          {
            "name": "Update 1..9 - Publisher - mem",
            "value": 30.86328125,
            "unit": "median mem",
            "extra": "avg mem: 30.143274214428896, max mem: 31.2734375, count: 53878"
          },
          {
            "name": "Update 10,11 - Publisher - cpu",
            "value": 4.5714283,
            "unit": "median cpu",
            "extra": "avg cpu: 4.759046789051599, max cpu: 9.17782, count: 53878"
          },
          {
            "name": "Update 10,11 - Publisher - mem",
            "value": 30.94140625,
            "unit": "median mem",
            "extra": "avg mem: 30.211653092403207, max mem: 31.0546875, count: 53878"
          }
        ]
      }
    ]
  }
}