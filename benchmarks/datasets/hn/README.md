# HN heap snapshot

HN uses the same PostgreSQL 18 cluster, parallel Parquet loader, and pgBackRest
snapshot workflow as MS MARCO and Cohere. Select it explicitly; `all` excludes HN.

```sh
gh workflow run generate-benchmark-snapshots.yml --repo paradedb/paradedb \
  --ref main -f dataset=hn -f size=full
```

The assembled dataset must first be available at
`s3://paradedb-benchmark/datasets/hn/v1/`. The existing production artifacts were
published by [the HN preparation PR](https://github.com/paradedb/paradedb/pull/6853)
under `s3://paradedb-ci-benchmarks/datasets/hn/v1/`. Preserve the original manifest
and artifact bytes when copying them to the current benchmark bucket. No new
embedding generation is required.

The published corpus has 28,736,557 rows, including 24,138,153 non-NULL vectors
with 1,536 dimensions. It excludes 1,000 held-out queries. The manifest supplies
the exact counts and SHA-256 checksums used by the workflow.

The preparation step downloads only manifest-listed full-corpus shards and query
fixtures, verifies every checksum, and checks Parquet and query counts. A local
directory alias maps the published `sampled/full/parquet/hn/` directory to the
native loader's `hn_items` table without changing the immutable artifacts.
Allow space for both the staged Parquet and loaded heap on the runner's NVMe.

Loading uses 16 workers by default; `-f load_workers=8` overrides that setting.
`pg_null_byte_replacement = ""` handles PostgreSQL-incompatible NUL bytes through
the existing MS MARCO loader. All original columns and nullable embeddings remain
in the heap. The workflow verifies row/vector counts and holdout exclusion,
records the source manifest hash in `hn_dataset_identity`, and runs
`VACUUM (FREEZE, ANALYZE)` before the cold backup. The primary key is retained;
there is no search index.

Snapshots use the standard destination:

```text
s3://paradedb-benchmark/snapshots/hn/v1/full/
  pg_backrest/
  dataset-manifest.json
  verify_heap.sql
```

The shared workflow runs only from main using the benchmark write role.
Regeneration replaces the backup at that destination. A successful job records
provenance after pgBackRest completes. Benchmark jobs can use the shared
`restore-heap --dataset hn --size full` command with bucket `paradedb-benchmark`
and path prefix `/snapshots`, then rebuild their search index.

Run preparation checks with:

```sh
uv run --with duckdb==2.0.0 python -m unittest discover \
  -s .github/scripts -p test_prepare_hn_heap.py
```

Set `HN_TEST_DATABASE_URL` to an empty scratch PostgreSQL database with pgvector
installed to include the actual heap-loading and verification checks. The
integration test creates and drops `hn_items` and `hn_dataset_identity` there.
