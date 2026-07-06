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
# The chunk assignment sorts only the id list (cheap); the corpus itself is streamed through a join
# and written partitioned in one pass, so this stays feasible at larger sizes without sorting the full
# set of vectors.
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

STAGE="$(mktemp -d)/chunks"
mkdir -p "${STAGE}.spill"
trap 'rm -rf "$(dirname "${STAGE}")"' EXIT

echo "Splitting ${SRC} into ${CHUNKS} chunks under ${BASE}/sampled/${SIZE}_chunk{0..$((CHUNKS - 1))}/ ..."

# Assign each row a chunk (ntile over the id list -- a small sort), then stream the corpus through a
# join on _id and write it partitioned by chunk to local parquet in one pass. http_timeout is raised
# from the 30s default so large multi-file S3 reads don't time out.
duckdb -c "
  INSTALL httpfs; LOAD httpfs;
  CREATE OR REPLACE SECRET s3_secret (TYPE s3, PROVIDER credential_chain);
  SET http_timeout=300; SET http_retries=10; SET temp_directory='${STAGE}.spill';
  CREATE TEMP TABLE assign AS
    SELECT _id, ntile(${CHUNKS}) OVER (ORDER BY _id) - 1 AS chunk FROM read_parquet('${SRC}');
  COPY (
    SELECT c._id, c.url, c.title, c.text, c.emb, a.chunk
    FROM read_parquet('${SRC}') c JOIN assign a USING (_id)
  ) TO '${STAGE}' (FORMAT PARQUET, COMPRESSION 'zstd', PARTITION_BY (chunk), OVERWRITE_OR_IGNORE true);
"

# Upload each local partition (`chunk=k/`) to its chunk path.
for k in $(seq 0 $((CHUNKS - 1))); do
  dst="${BASE}/sampled/${SIZE}_chunk${k}/parquet/cohere_wiki/"
  echo "  chunk ${k} -> ${dst}"
  aws s3 cp "${STAGE}/chunk=${k}/" "${dst}" --recursive --exclude '*' --include '*.parquet'
done

echo "Done. Next: generate ground truth with ./scripts/generate_cohere_ground_truth.sh ${SIZE}"
