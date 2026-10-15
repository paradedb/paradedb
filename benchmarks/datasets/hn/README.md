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
- The later corpus assembly step must exclude every held-out `hn_id` before
  sampling, GT generation, or heap loading. The dry run only records this split;
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
  queries/
    holdout.parquet
    hn_queries.parquet
    ground_truth_<query>_<size>.parquet
  sampled/<size>/parquet/hn_items/
    part-*.parquet
```

Heap snapshots belong under
`s3://paradedb-ci-benchmarks/snapshots/hn/v1/<size>/`.

The profile manifest has state `profiled`; it is not a complete benchmark dataset.
The embedding cache is also an intermediate artifact. Corpus assembly, GT,
snapshot publication, and benchmark workflow registration follow in subsequent
steps. Keep published versions immutable and bind each snapshot and fixture set
to the same completed manifest.

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
query rows will both join to this cache in the subsequent assembly step.

Partial jobs retain successes and retry only transient failures, with three
attempts per shard by default. Permanent failures and incomplete/malformed
responses stop the run. Before creating a paid batch, the runner checkpoints a
unique submission token and disables automatic retries of that create call. On
restart it searches batch metadata to recover an accepted job. If the outcome
cannot be reconciled, it stops for operator investigation instead of submitting
another copy; inspect `state.json` and the OpenAI batch dashboard before changing
that checkpoint. Collect results before the API's output files expire.

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

Normal benchmark CI does not generate embeddings. This workflow builds only the
embedding cache; it does not load a database, remove holdout rows from the source,
generate GT, or create heap snapshots.

## Checks

```sh
uv run --python 3.12 \
  --with duckdb==1.4.3 --with pyarrow==21.0.0 --with tiktoken==0.12.0 \
  --with openai==3.31.0 \
  python -m unittest discover -s benchmarks/scripts -p 'test_*hn*.py'
```

Tests use simulated API responses and do not require credentials or incur charges.
