# RunsOn Spot retries

All RunsOn jobs explicitly use `retry=when-interrupted`, including publishing,
benchmarks, snapshot generation, and Antithesis. RunsOn performs the retry; the
workflow token does not need `actions: write` for this. The control plane was
upgraded to v3.3.1 as recorded in [#6255](https://github.com/paradedb/paradedb/issues/6255).

Starting with v3.3.0, RunsOn marks interrupted jobs as failed and adds the
`EC2 Spot interruption` error annotation. It can rerun failed jobs after workflow
attempts 1 and 2; attempt 3 is final. Successful jobs are carried forward, so
matrix jobs use `fail-fast: false` to avoid cancelling healthy siblings.
Artifact uploads from RunsOn jobs use `overwrite: true` so an interruption after
an upload does not make the retry fail on an existing artifact name. Each matrix
row retains its own artifact name.

## Merge queue gates

The seven regular RunsOn PR validation workflows also run on `merge_group` and
call [Cyril's reusable gate](https://github.com/runs-on/spot-retry-gate) with all
validation jobs in `needs`, `always()`, and only `actions: read` / `checks: read`.
The gate runs on GitHub-hosted runners. Each workflow has a distinct gate name
so one workflow's success cannot satisfy another workflow's required check.

| Workflow                 | Normal check name                      |
| ------------------------ | -------------------------------------- |
| Lint Rust                | `Lint Rust Gate / pass`                |
| Test ParadeDB (Docs)     | `Test ParadeDB (Docs) Gate / pass`     |
| Test pg_search           | `Test pg_search Gate / pass`           |
| Test pg_search (Docker)  | `Test pg_search (Docker) Gate / pass`  |
| Test pg_search (Schema)  | `Test pg_search (Schema) Gate / pass`  |
| Test pg_search (Upgrade) | `Test pg_search (Upgrade) Gate / pass` |
| Test Stressgres (Docker) | `Test Stressgres (Docker) Gate / pass` |

On a retryable annotated interruption, the gate publishes `<workflow> Gate /
interrupted` successfully instead of `pass`. A required `pass` check stays
pending until a retry completes. Ordinary failures, cancellation without an
interruption annotation, annotation API errors, and exhausted retries fail the
normal check. Mixed ordinary and Spot failures remain pending during a retryable
interruption; a persistent ordinary failure fails the subsequent attempt.

Publishing, benchmarks, snapshot generation, and Antithesis retain their existing
triggers and use the same automatic retry policy. They are not required merge
checks and do not need a merge queue gate. The Docker publishing workflow also
inherits retries through its call to the Docker test workflow; its caller grants
the read permissions needed by the nested gate.

## Activating required checks

The rules applying to `main` had no required status checks or merge queue when
inspected on 2026-09-11. This repository change does not modify organization
rulesets. When enabling the queue:

1. Validate the gates on a PR and a disposable Spot interruption run before
   changing required checks.
2. The PR workflows retain their existing path filters. Before making any gate
   unconditionally required, remove that workflow's PR path filter or implement
   an always-running path-aware gate. Otherwise an unrelated PR will wait forever
   for a workflow that never ran. `merge_group` runs are not path-filtered.
3. Require the applicable names above instead of their individual RunsOn jobs.
   Preserve unrelated checks and ensure they also run on `merge_group`. Do not
   require the `interrupted` or `Detect Spot interruption` checks.
4. Allow enough merge queue status-check time for the entire first attempt and
   up to two retries. RunsOn waits for the workflow attempt, including the gate,
   to finish before requesting a retry. Do not cancel an interrupted attempt
   while its gate is still running.

## Live validation

Use a disposable job and an EC2 Spot interruption simulation that allows the
runner's post-job hook to emit the annotation. Simply terminating an instance can
prevent that annotation from being reported and cannot validate this gate.

Check normal success, an annotated interruption on attempts 1 and 2, successful
retry, ordinary failure, mixed ordinary/Spot failures, and exhausted retries.
Confirm that only failed jobs rerun, the expected normal check passes after a
successful retry, and the PR remains queued while an interrupted attempt is being
retried. Local gate unit tests exercise the result and annotation logic, but do
not verify AWS interruption delivery or GitHub merge queue behavior.

See the [v3.3.0 release notes](https://github.com/runs-on/runs-on/releases/tag/v3.3.0)
and [RunsOn job labels](https://runs-on.com/docs/runners/labels/).
