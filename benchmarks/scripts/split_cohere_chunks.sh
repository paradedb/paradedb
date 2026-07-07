#!/usr/bin/env bash
#
# Split a cohere `sampled/{size}` corpus into N disjoint, prefix-ordered chunks for the
# vector-update benchmark (see .github/workflows/benchmark-pg_search-updates.yml).
#
# The update benchmark grows the corpus one chunk at a time and measures latency + recall between
# each insert. The chunks partition `sampled/{size}` exactly, so the union of all chunks equals
# `sampled/{size}` and the final step's corpus matches the one-shot benchmark. Ground truth is
# produced separately by generate_cohere_ground_truth.sh.
#
# Chunks are cut by ntile(N) over ORDER BY _id, a stable total order (ids are unique), so the split is
# deterministic. With the default 10 chunks, 1m -> ten ~100k chunks.
#
# Each chunk is written as a SINGLE exact key `sampled/{size}_chunkK/parquet/cohere_wiki/data.parquet`
# (not a directory of shards). This lets `load-heap --single-file` read it by exact key, which needs
# only s3:GetObject -- a `*.parquet` glob would need s3:ListBucket, which the CI cross-account user
# does not have on the public-GetObject datasets bucket.
#
# Requires the DuckDB CLI and AWS credentials on the standard chain. Usage:
#   ./split_cohere_chunks.sh <size> [chunks] [s3://bucket/datasets/cohere]
#   ./split_cohere_chunks.sh 1m
set -euo pipefail

SIZE="${1:?usage: split_cohere_chunks.sh <size> [chunks] [base]  (e.g. 1m)}"
CHUNKS="${2:-10}"
BASE="${3:-s3://paradedb-benchmarks/datasets/cohere}"
BASE="${BASE%/}"
SRC="${BASE}/sampled/${SIZE}/parquet/cohere_wiki/*.parquet"

SPILL="$(mktemp -d)"
trap 'rm -rf "${SPILL}"' EXIT

echo "Splitting ${SRC} into ${CHUNKS} chunks under ${BASE}/sampled/${SIZE}_chunk{0..$((CHUNKS - 1))}/ ..."

# Materialize the corpus once with its chunk assignment (ntile over ORDER BY _id), then write each
# chunk to a single exact `data.parquet` key. http_timeout is raised from the 30s default so large
# multi-file S3 reads don't time out.
copies=""
for k in $(seq 0 $((CHUNKS - 1))); do
  dst="${BASE}/sampled/${SIZE}_chunk${k}/parquet/cohere_wiki/data.parquet"
  echo "  chunk ${k} -> ${dst}"
  copies="${copies}
    COPY (SELECT _id, url, title, text, emb FROM corpus WHERE chunk = ${k})
      TO '${dst}' (FORMAT PARQUET, COMPRESSION 'zstd');"
done

duckdb -c "
  INSTALL httpfs; LOAD httpfs;
  CREATE OR REPLACE SECRET s3_secret (TYPE s3, PROVIDER credential_chain);
  SET http_timeout=300; SET http_retries=10; SET temp_directory='${SPILL}';
  CREATE TEMP TABLE corpus AS
    SELECT _id, url, title, text, emb, ntile(${CHUNKS}) OVER (ORDER BY _id) - 1 AS chunk
    FROM read_parquet('${SRC}');
  ${copies}
"

echo "Done. Next: generate ground truth with ./scripts/generate_cohere_ground_truth.sh ${SIZE}"
