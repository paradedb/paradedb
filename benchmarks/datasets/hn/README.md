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

The current manifest has state `profiled`; it is not a complete benchmark dataset.
Embedding generation, corpus assembly, GT, snapshot publication, and benchmark
workflow registration follow in subsequent steps. Keep published versions
immutable and bind each snapshot and fixture set to the same completed manifest.

## Checks

```sh
uv run --python 3.12 \
  --with duckdb==1.4.3 --with pyarrow==21.0.0 --with tiktoken==0.12.0 \
  python -m unittest discover -s benchmarks/scripts -p 'test_prepare_hn.py'
```
