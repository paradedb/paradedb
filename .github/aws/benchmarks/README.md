# Benchmark AWS access

Downloads from `datasets/`, `benchmarker/`, and `snapshots/` in the CI account's
`paradedb-benchmark` bucket are public. CLI and Benchmarker downloads are
anonymous; native pgBackRest restores use a scoped read-only OIDC role because
pgBackRest requires S3 credentials. Snapshot uploads and cleanup use a separate
OIDC write role. None of these workflows use the `GITHUB_ACTIONS` IAM user's keys.

| Role | Permissions | GitHub subjects |
| --- | --- | --- |
| `paradedb-benchmark-read` | Read benchmark inputs; list their prefixes | `main` branch and pull-request jobs in `paradedb/paradedb` and `paradedb/paradedb-enterprise` |
| `paradedb-benchmark-write` | The same reads, plus upload, replacement, and multipart cleanup under `snapshots/` in the CI bucket | `main` branch jobs in those two repositories |

Enterprise is included in the trusts because its workflows are synced from the
community repository. No other repositories, tags, or GitHub environments are
trusted. Dispatch workflows from `main` (the benchmark commit input can select other source refs); PR benchmark jobs
cannot assume the write role. The OIDC audience must be `sts.amazonaws.com`.

The policies only access `paradedb-benchmark`, owned by the CI account. Reads
cover `datasets/`, `benchmarker/` config packages, and `snapshots/`; only
`snapshots/` is writable. The workflows use the versioned layout introduced in
#6980, including the migrated Benchmarker configs and heap archives. Legacy
Management buckets and preserved `old/` objects are outside these roles.

Dataset uploads, bucket configuration, IAM, compute, Terraform state, and runner
caches are outside these roles. `ListBucketMultipartUploads` is bucket-wide
because S3 does not support a prefix condition for that action; abort and object
write/delete permissions remain restricted to `snapshots/`.

The roles allow six-hour sessions to match the benchmark jobs' six-hour runtime
limit. The credential action exports `AWS_SESSION_TOKEN` along with the access
key and secret. The existing pgBackRest configuration generator forwards that
token as `repo1-s3-token`; `shared` therefore works with these temporary credentials.

## Live configuration

Both roles and the public-read bucket policy are already provisioned in CI
account `858265286872`. The role permissions use the inline policy
`BenchmarkS3Access`. These workflows consume the existing roles; they do not
provision AWS resources.

The bucket policy permits anonymous reads/listings only under the three public
prefixes, denies anonymous uploads/deletes, and requires HTTPS. Bucket ACLs
remain blocked/ignored, and object ownership remains `BucketOwnerEnforced`.
No account-wide public-access settings were changed. Neither `old/` nor bucket
administration is public. Public downloads incur the bucket owner's S3 request
and applicable data-transfer charges.

## Cutover

1. The roles and public-read bucket configuration are already provisioned and validated.
2. Verify a PR benchmark can read snapshots through OIDC. After merge, verify a
   snapshot-generation run on `main` can upload and clean up snapshots.
3. Sync the workflow change to Enterprise and verify its runs too.
4. Once all consumers are migrated, disable the old key, verify scheduled and
   manual jobs still work, then delete the key and `GITHUB_ACTIONS` user. Remove
   the two `AWS_CI_SUBACCOUNT_GITHUB_ACTIONS_USER_*` organization secrets too.
   The archived `terraform-paradedb-byoc` workflows still reference those secrets;
   they need migration if that repository is ever reactivated.

Changing these workflows alone does not revoke the old user or its credentials.
