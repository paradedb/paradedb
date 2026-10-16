# HN vector dataset preparation

HN v1 embeds the existing `hn-ci` dataset's `text` column using
`text-embedding-3-small`, 1,536 dimensions, and cosine distance. The source is
`s3://paradedb-benchmarker/datasets/hn-ci.tar.gz`; `prepare.toml` defines the
preparation contract and `create_tables.sql` preserves the HN schema with a nullable
`emb vector(1536)` column.

## Profile and select queries

Run from the repository root with [uv](https://docs.astral.sh/uv/):

```sh
uv run benchmarks/scripts/prepare_hn.py \
  --source /path/to/extracted/data.csv \
  --output /path/to/hn-v1-profile \
  --workers 4
```

This is a local dry run: it makes no embedding API calls and does not upload to
S3 or change the source CSV. It writes `manifest.json` and
`queries/holdout.parquet`. The output directory must be empty. Temporary profile
shards and DuckDB spill files live underneath it and are removed when the run
exits; allow disk space for the per-row hashes and aggregation spill. Normalized
text is not copied to disk: a second CSV pass retrieves just the held-out inputs.
Memory usage is bounded by worker batches and DuckDB's default 2 GB memory limit;
`--memory-limit 8GB` reduces spill on machines with sufficient RAM.

The manifest records the exact CSV SHA-256, configuration, preparer checksum,
dependency versions, row/token/duplicate counts, estimated API cost after input
deduplication, and raw vector storage. Costs exclude retries, S3, and compute;
the dated prices are explicit in `prepare.toml`. A supplied local CSV is not
automatically verified against the configured S3 archive: the manifest records
`archive_verified: false`. Verify its checksum against the archived CSV before
publishing a dataset version.

## Text and holdout contract

- Preserve the original `text` and all other HN columns in the eventual corpus.
- Parse HTML with Python's `HTMLParser`, decode entities once, separate block
  elements, omit comments/script/style contents, and collapse Unicode whitespace.
- Empty or `\N` inputs, including text empty after normalization, get NULL vectors.
- Encode normalized text with `cl100k_base`, treating special-token spellings as
  ordinary text. The API input is the first 8,192 **token IDs**, avoiding a lossy
  decode/re-encode at a truncated UTF-8 boundary.
- Hash normalized UTF-8 text and the final little-endian uint32 token sequence
  separately. Cache embeddings by model, dimensions, preprocessing version, and
  input hash. Duplicate rows retain separate HN IDs.
- Choose 1,000 rows with unique final token sequences, which also guarantees
  unique normalized text and excludes collisions caused by truncation. Sort by
  SHA-256 of `"<seed>:<hn_id>"`, then numeric HN ID, and take the first 1,000.
  Assign query IDs 0–999 in that order. Selection is independent of CSV order,
  batching, and worker count.
- `holdout.parquet` contains `id` (int32 query ID), `hn_id` (int64),
  `normalized_sha256`, `input_sha256`, and `token_ids` (list of int32).
- Corpus assembly excludes every held-out `hn_id` before sampling, GT generation,
  or heap loading. The dry run only records this split;
  it does not delete any source rows.

## Artifact destinations

The agreed portable artifact layout is:

```text
s3://paradedb-benchmarks/datasets/hn/v1/
  embedding-cache/<preparation>/
    plan.json
    profile.json
    state.json
    embeddings.json
    requests/*.jsonl
    submissions/*.jsonl
    vectors/*.parquet
    queries/holdout.parquet
  manifest.json
  assembly-intent.json
  provenance/{profile,plan,embeddings}.json
  queries/
    holdout.parquet
    hn_queries.parquet
    ground_truth_<query>_<size>.parquet
  sampled/<size>/parquet/hn/
    part-*.parquet
```

Heap snapshots belong under
`s3://paradedb-ci-benchmarks/snapshots/hn/v1/<run-id>-<attempt>/full/`.
Each run uses a new prefix, leaving earlier snapshots intact.

The profile manifest has state `profiled`; it is not a complete benchmark dataset.
The embedding cache is also an intermediate artifact. Assembly produces a manifest
with state `assembled`, covering the corpus and query files. GT generation,
snapshot publication, and benchmarking run in the subsequent workflows below.
Keep published versions immutable and bind each snapshot and fixture set to the
same assembled manifest.

## Generate embeddings with the Batch API

Build request files from the exact CSV used for profiling:

```sh
uv run benchmarks/scripts/plan_hn_embeddings.py \
  --source /path/to/extracted/data.csv \
  --profile /path/to/hn-v1-profile/manifest.json \
  --output /path/to/hn-v1-embeddings
```

Planning makes no API calls. It verifies the source checksum and profile counts,
deduplicates exact token inputs with a temporary SQLite index, and creates bounded
JSONL files. Defaults are 128 inputs and 250,000 tokens per request, and 50,000
inputs, 5 million tokens, or 180 MB per batch, whichever is reached first. These
fit the [embedding request limits](https://developers.openai.com/api/reference/resources/embeddings/methods/create)
and [Batch limits](https://developers.openai.com/api/docs/guides/batch).

With `OPENAI_API_KEY` in the environment, submit and collect a local cache:

```sh
uv run benchmarks/scripts/embed_hn.py advance \
  --directory /path/to/hn-v1-embeddings
```

The default performs one collection/submission pass and exits. Rerun the same
command to collect completed jobs and submit more. `--max-seconds 18000` keeps
advancing for up to five hours, polling every 60 seconds. Pending work is a normal
exit; completion is indicated by `embeddings.json`, not by process exit status.

For durable CI checkpoints and to avoid retaining all vectors on the runner:

```sh
uv run benchmarks/scripts/embed_hn.py publish \
  --directory /path/to/hn-v1-embeddings \
  --s3-prefix s3://paradedb-benchmarks/datasets/hn/v1/embedding-cache/initial

uv run benchmarks/scripts/embed_hn.py advance \
  --directory /path/to/hn-v1-embeddings \
  --s3-prefix s3://paradedb-benchmarks/datasets/hn/v1/embedding-cache/initial \
  --max-seconds 18000
```

`publish` uploads immutable requests and publishes `plan.json` last. `advance`
can start in an empty local directory: it restores the plan and state from S3,
downloads request shards as needed, and uploads each validated vector shard before
checkpointing its completion. Uploaded vector shards are then removed locally.
Use one writer per S3 prefix, and the same OpenAI project for submission and
collection. The workflow enforces a single writer; local commands also acquire a
process lock. Local-only runs retain the full vector cache on disk.

Successful responses are matched by request ID and embedding index, then checked
for the configured model, dimensions, and finite nonzero float32 values. Cache
Parquet files contain `input_sha256` and `emb` (a fixed-size list of 1,536 float32
values). `embeddings.json` records their checksums and counts only after every
unique input succeeds. Empty source rows have no cache entry. Corpus and held-out
query rows both join to this cache during assembly.

Partial jobs retain successes and retry only transient failures, with three
attempts per shard by default. Permanent failures and incomplete/malformed
responses stop the run. Before creating a paid batch, the runner checkpoints a
unique submission token and disables automatic retries of that create call. On
restart it searches batch metadata to recover an accepted job. If the outcome
cannot be reconciled, it stops for operator investigation instead of submitting
another copy; inspect `state.json` and the OpenAI batch dashboard before changing
that checkpoint. Collect results before the API's output files expire.

## Assemble corpus and query files

Once `embeddings.json` exists, assemble from the exact original CSV and completed
cache. No OpenAI calls are made by this step:

```sh
uv run benchmarks/scripts/assemble_hn.py assemble \
  --source /path/to/extracted/data.csv \
  --cache-directory /path/to/hn-v1-embeddings \
  --output /path/to/hn-v1-dataset
```

Add `--cache-s3-prefix
s3://paradedb-benchmarks/datasets/hn/v1/embedding-cache/initial` to restore a remote
cache into a local directory. Assembly verifies the source, plan, profile,
holdout, and every vector shard against their recorded checksums. It rejects
missing, duplicate, unexpected, null, zero, nonfinite, or incorrectly dimensioned
cache vectors. Original HN columns retain their types and values; `\N` becomes
NULL, empty strings remain empty strings, and timestamps are interpreted in UTC.
Rows with empty normalized text have NULL embeddings. Duplicate inputs share a
vector while retaining their separate HN IDs and original text.

The output directory must be empty. Each corpus contains the original 15 columns
in table order followed by `emb` (list of float32). The query fixture contains
exactly `id` (int32) and `emb`, matching pdbench's loader. The separate
`queries/holdout.parquet` maps query IDs back to source HN IDs and input hashes.
Held-out rows are excluded before sampling; no source CSV or database is mutated.

By default, assembly produces only `full`: every source row except the 1,000
held-out queries. This is the corpus used by the CI preparation workflow.

Optional smaller corpora can be requested explicitly with `--sizes`, for example
`--sizes full 100k` for local checks. Samples have exact row counts and use the
first N corpus IDs ordered by SHA-256 of `"<sampling_seed>:<hn_id>"`, then numeric
HN ID, with the seed from `config.toml`. Smaller samples are subsets of larger
ones, and all share the same query fixture. Empty-text rows participate in
sampling. The manifest records both row and vector counts for each size; a size
larger than the available corpus fails.

Assembly uses hash partitions and a disk-backed DuckDB database with a default
4 GB memory limit. It temporarily holds downloaded cache shards and partitioned
vectors, then deletes downloaded shards and consumes partitions as output is
written. Existing local cache files are preserved. Allow roughly 600 GB of free
disk for full-scale assembly and spill; the CI assembly runner provisions a
600 GB volume. `--memory-limit` controls DuckDB, and `--workers` controls the text
profiling processes. The full-scale resource/runtime requirements have not yet
been measured with real embeddings.

Publish a successfully assembled dataset separately:

```sh
uv run benchmarks/scripts/assemble_hn.py publish \
  --output /path/to/hn-v1-dataset \
  --s3-prefix s3://paradedb-benchmarks/datasets/hn/v1
```

Publication requires an archive-verified source profile, checks local artifact
checksums, and uploads `manifest.json` last. An intent object reserves the prefix
for that exact manifest before artifact uploads start. Rerunning publication of
the same local output resumes safely; a different manifest is refused. Use one
writer per dataset prefix. Assembly itself has no checkpoints: if interrupted,
restart with a new empty output directory and the same completed embedding cache.
Failed assembly never writes a completion manifest. A partial S3 publication must
be resumed with the identical artifacts, not replaced by a different assembly.

## Run in CI

Use the manual **Generate HN Embeddings** workflow with a preparation name such as
`initial`:

1. Choose `prepare` to download the configured HN archive, extract only its CSV,
   profile it, verify the CSV against the downloaded archive, and publish request
   files. This action does not call OpenAI. An existing plan is reused without
   overwriting it; choose a new preparation name to build a different plan.
2. Choose `advance` with the same name to submit/collect Batch jobs. This action
   requires the `OPENAI_API_KEY` repository or organization secret and incurs API
   charges. AWS credentials use the existing CI secrets and need read access to
   the source bucket and read/write access to the embedding-cache prefix.
3. Repeat `advance` until `embeddings.json` is published. Jobs can outlive the CI
   run; progress is saved after each transition. The default queue holds at most
   four batches and 20 million input tokens for this cache. Set lower limits if
   other jobs share the project's Batch quota, or raise them within that quota.
4. Choose `assemble` with the same preparation name to download the source/cache,
   build the full corpus and the query fixture, and publish the
   assembled dataset. This requires read/write access to the dataset prefix, no
   OpenAI key, and makes no paid embedding requests.

Normal benchmark CI does not generate embeddings. This manual workflow prepares
the embedding cache and portable dataset artifacts. It does not load a database or
create heap snapshots. Ground truth is generated by the separate workflow below.

## Filtered ground truth

After the full corpus and query fixture have been published, run the manual
**Generate HN Ground Truth** workflow. It makes no OpenAI requests. It restores
only the full corpus, freezes PostgreSQL filters, runs pinned Supernova
`nova-bf`, and publishes pdbench-compatible top-10 fixtures under:

```text
s3://paradedb-benchmarks/datasets/hn/v1/
  queries/ground_truth_<query_name>_full.parquet
  ground_truth/manifest.json
  ground_truth/filters.json
  ground_truth/queries/<query_name>.sql
  ground_truth/index.sql
  ground_truth/membership.parquet
  ground_truth/<query_name>_boundary.parquet
  ground_truth/supernova/...
```

`filters.toml` defines the workload: unfiltered, score, story type, story plus
score, three time cutoffs targeting 10%/1%/0.1% selectivity, and three distinct
author equality filters targeting 0.1%/0.01%/0.001%. Author frequencies and date
quantiles determine the actual values. Every filter records its exact SQL,
matched vector count, and actual selectivity; targets are not guarantees.
Author selection chooses the nearest frequency with at least 11 rows, breaking
ties by the author string under PostgreSQL C collation. Text comparisons are
case-sensitive whole-value equality. The index defines `type` and `by` with
`pdb.literal`; the queries use `=`, not substring matching.

PostgreSQL evaluates the predicates in a temporary metadata table. The exported
boolean masks carry those exact memberships into Supernova. NULL vectors never
enter the GT corpus; a NULL predicate evaluates as false. Checksums bind the GT
to the full assembled dataset, held-out queries, frozen masks, and tool revision.
Every filter must have at least 11 eligible vectors or preparation fails.

Supernova scans every vector and searches every held-out query with float32
cosine scoring, TF32 disabled, and numeric HN ID ordering for equal scores. It
computes 11 neighbors so the 10th/11th score boundary can be audited. Exported
fixtures contain exactly `query_id` (int32) and `gt_ids` (list of strings), as
pdbench expects. Raw scores and per-query boundary diagnostics are retained.
Duplicate HN inputs can produce tied neighbors: pdbench's current strict ID-set
recall can penalize an equally close alternative ID. The boundary files flag
exact ties and gaps up to 1e-6; they do not make pdbench tie-aware. Benchmark
registration must account for those diagnostics when interpreting recall.

Local execution uses Python 3.12, a scratch PostgreSQL database, and an unmodified
Supernova checkout at `695841a0fc29f8da303e8dccb80eb21cb732e280`:

```sh
python -m pip install -r benchmarks/scripts/requirements-hn-gt.txt
export HN_GT_DATABASE_URL=postgresql://localhost/hn_gt

python benchmarks/scripts/ground_truth_hn.py prepare \
  --dataset-directory /path/to/hn-v1-dataset \
  --output /path/to/hn-v1-gt

NOVA_BF_DEVICE=cpu python benchmarks/scripts/ground_truth_hn.py compute \
  --output /path/to/hn-v1-gt --supernova /path/to/supernova

python benchmarks/scripts/ground_truth_hn.py publish \
  --output /path/to/hn-v1-gt \
  --s3-prefix s3://paradedb-benchmarks/datasets/hn/v1
```

Add `--s3-prefix` to `prepare` to restore missing dataset files. PostgreSQL needs
no extensions for mask preparation, and all metadata tables are temporary. The
separate benchmark index requires a pg_search build with native vector support.
On an appropriate CUDA host, install a CUDA-enabled Torch build and select
`NOVA_BF_DEVICE=cuda` for compute.

The CI runner provides 16 CPUs, 64 GB RAM, and 600 GB disk. Staging retains both
the assembled full corpus and a copy of its vectors with masks. Full-scale
runtime and peak resources have not yet been measured with real embeddings.
Preparation requires an empty output directory; interruption requires starting
preparation again in a new empty directory. To retry failed compute without
restaging vectors, remove only `ground_truth/supernova/` from the failed output
and rerun `compute` with the same script and prepared inputs. A completed run is
immutable and should instead be published or reused.

Publication requires the already-published, archive-verified dataset manifest to
match. It reserves `ground_truth/intent.json`, publishes fixtures and provenance,
then uploads `ground_truth/manifest.json` last. A different GT run is refused at
the same version. Use one writer per prefix. If publication is interrupted, the
workflow retains `ground_truth/` and `queries/` as a recovery artifact for 14 days.
Download that artifact, verify it contains `ground_truth/manifest.json`, and run
`publish` against its root to resume the identical output. Do not recompute a new
run into a prefix reserved by the interrupted publication.

## Run with pdbench

The manual **Benchmark HN Vectors (pdbench)** workflow uses the unchanged
`paradedb/pdbench` runner pinned to
`059615191b0890fea8c3693567b145d0bc63086e`. It requires the existing organization
GitHub App to have contents-read access to that private repository. AWS credentials
need read access to the dataset bucket and read/write access to the snapshot bucket.

Run the workflows in this order:

1. **Generate HN Embeddings**: `prepare`, then repeat `advance` with the same
   preparation name until `embeddings.json` is published, then `assemble`.
2. **Generate HN Ground Truth**: freeze all ten filters and publish the exhaustive
   top-10 fixtures and boundary diagnostics.
3. **Benchmark HN Vectors (pdbench)**: `action=snapshot` loads the full verified
   corpus into PostgreSQL 18 and captures a heap-only pgBackRest snapshot. Its
   summary prints the snapshot ID. This uses the same ARM64 metal cluster setup
   as benchmark runs. Each snapshot includes the assembled manifest hash and
   requires exact corpus/vector counts and exclusion of all held-out IDs.
4. **Benchmark HN Vectors (pdbench)**: `action=benchmark`, `heap_source=snapshot`,
   and the snapshot ID builds the index and benchmarks all ten frozen workloads.
   `heap_source=parquet` instead loads the checked Parquet files into a fresh heap.

The pdbench profile is generated from the assembled manifest, including the exact
full row count. Its dataset name is `hn`, so pdbench expects the corpus directory
`sampled/full/parquet/hn/`; its table remains `hn_items`. It creates the default
index `hn_items_bm25_idx` using `paradedb`, with `training_sample_ratio = 0.32`,
`target_segment_count = 4`, and no `key_field`. The cluster-wide segment override
is zero so the index setting takes effect. There are no 1M/10M benchmark variants.

`pdbench_hn.py prepare` checks dataset/GT manifest identity and fixture checksums,
copies the exact frozen SQL predicates, and builds the profile and SQL assets.
It uses a verified local artifact cache as pdbench's data path. Snapshot restores
need only the small fixtures; new heaps also download and checksum the corpus.
The schema asset is safe for pdbench's temporary schema probe and contains no
`DROP TABLE`. A snapshot with a different manifest identity or row/vector counts
is rejected before building the index.

Before measuring, an `EXPLAIN (ANALYZE, VERBOSE, BUFFERS, FORMAT JSON)` gate requires
native cosine TopK and all filter fields inside the ParadeDB scan, with no residual
Postgres or internal heap filter. This runs against the actual CI extension build.
Failure preserves the plans for inspection and prevents reporting the workload
as a filtered-vector benchmark. Text filters stay exact SQL equality on columns
indexed with `pdb.literal`.

Pdbench uses all 1,000 query vectors, three measured repetitions and one warmup,
and sweeps `paradedb.vector_cluster_max_probe` from 0.0001 to 1.0 for 90%, 95%, and
99% recall targets. Each workload loads its own frozen top-10 GT fixture. Reports
use strict ID recall; duplicate-vector ties can prevent reaching a target even
with exhaustive search. The GitHub artifact retains JSON/HTML reports, plans,
profile, frozen SQL, dataset/GT manifests, and per-query boundary diagnostics for
30 days. `runs/complete.json` appears only after all workloads succeed.

These workflows do not automatically chain. A successful `advance` job may still
have Batch work pending; always check the completion marker before assembling.
The workflows are registered with GitHub and can be dispatched from this branch:

```sh
gh workflow run generate-hn-embeddings.yml --repo paradedb/paradedb \
  --ref codex/hn-vector-preparation -f action=prepare -f preparation=initial
```

For a brand-new workflow, a temporary PR-triggered registration job can make it
available to the dispatch API before merge. Keep generation jobs restricted to
`workflow_dispatch` during registration, then remove the temporary trigger.

## Checks

```sh
python -m pip install -r benchmarks/scripts/requirements-hn-gt.txt
export HN_GT_DATABASE_URL=postgresql://localhost/hn_gt
export HN_GT_SUPERNOVA=/path/to/pinned/supernova
NOVA_BF_DEVICE=cpu python -m unittest discover -s benchmarks/scripts -p 'test_*hn*.py'
```

Tests use synthetic vectors, simulated embedding responses, and simulated S3
copies. They make no paid API calls and require no AWS credentials. GT integration
tests use real PostgreSQL and the pinned Supernova implementation; they are
skipped unless both GT environment variables are set. CI sets both. The tests
compare exhaustive neighbors against an independent cosine calculation and
cover equality semantics, ties, query/corpus separation, incomplete scans,
invalid membership, checksums, immutable publication, pdbench assets, heap loading,
snapshot identity, and rejection of plans without indexed filter pushdown.
