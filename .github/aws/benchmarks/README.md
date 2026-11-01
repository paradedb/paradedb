# Benchmark AWS access

Benchmark jobs use GitHub OIDC in the CI account (`858265286872`) instead of the
`GITHUB_ACTIONS` IAM user's access keys.

| Role | Permissions | GitHub subjects |
| --- | --- | --- |
| `paradedb-benchmark-read` | Read datasets and snapshots; list those prefixes | `main` branch and pull-request jobs in `paradedb/paradedb` and `paradedb/paradedb-enterprise` |
| `paradedb-benchmark-write` | The same reads, plus upload, replacement, and multipart cleanup under `snapshots/` in the two CI buckets | `main` branch jobs in those two repositories |

Enterprise is included in the trusts because its workflows are synced from the
community repository. No other repositories, tags, or GitHub environments are
trusted. Dispatch workflows from `main` (the benchmark commit input can select other source refs); PR benchmark jobs
cannot assume the write role. The OIDC audience must be `sts.amazonaws.com`.

The policies retain reads from `paradedb-ci-benchmarks`, `paradedb-benchmarks`, and
`paradedb-benchmarker` while datasets move to `paradedb-benchmark` in #6980.
Only `paradedb-ci-benchmarks/snapshots/` and `paradedb-benchmark/snapshots/` are
writable. Dataset uploads, bucket configuration, IAM, compute, Terraform state,
and runner caches are outside these roles. `ListBucketMultipartUploads` is
bucket-wide because S3 does not support a prefix condition for that action;
abort and object write/delete permissions remain restricted to `snapshots/`.

The roles allow six-hour sessions to match the benchmark jobs' six-hour runtime
limit. The credential action exports `AWS_SESSION_TOKEN` along with the access
key and secret. The existing pgBackRest configuration generator forwards that
token as `repo1-s3-token`; `shared` therefore works with these temporary credentials.

## Provisioning

The four JSON files in this directory are the role trust and inline permission
policies. They use the existing GitHub OIDC provider in the CI account. Run from
the repository root with an administrator session for that account:

```bash
test "$(aws sts get-caller-identity --profile ci --query Account --output text)" = 858265286872
for access in read write; do
  aws iam create-role --profile ci \
    --role-name "paradedb-benchmark-${access}" \
    --max-session-duration 21600 \
    --assume-role-policy-document "file://.github/aws/benchmarks/${access}-trust.json"
  aws iam put-role-policy --profile ci \
    --role-name "paradedb-benchmark-${access}" \
    --policy-name BenchmarkS3Access \
    --policy-document "file://.github/aws/benchmarks/${access}-policy.json"
done
```

For existing roles, use `update-assume-role-policy`, `update-role` with
`--max-session-duration 21600`, and `put-role-policy` with the same files. Keep
live policies aligned with the checked-in documents.

## Cutover

1. Provision and validate both roles before merging the workflow change.
2. Verify a PR benchmark can read snapshots through OIDC. After merge, verify a
   snapshot-generation run on `main` can upload and clean up snapshots.
3. Sync the workflow change to Enterprise and verify its runs too.
4. Once all consumers are migrated, disable the old key, verify scheduled and
   manual jobs still work, then delete the key and `GITHUB_ACTIONS` user. Remove
   the two `AWS_CI_SUBACCOUNT_GITHUB_ACTIONS_USER_*` organization secrets too.
   The archived `terraform-paradedb-byoc` workflows still reference those secrets;
   they need migration if that repository is ever reactivated.

Changing these workflows alone does not revoke the old user or its credentials.
