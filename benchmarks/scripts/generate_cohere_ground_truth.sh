#!/usr/bin/env bash
#
# Generate exact recall@10 ground truth for the cohere vector-update benchmark, in DuckDB, and upload
# it to S3 for `recall --ground-truth-path` to consume (see benchmark-pg_search-updates.yml).
#
# For a target SIZE (e.g. 1m) split into chunks by split_cohere_chunks.sh, this computes, for each
# cumulative step k (corpus = chunks 0..k) and each query file, the exact top-10 nearest neighbors of
# every held-out query vector and writes them to:
#     {base}/queries/ground_truth_{query}_upd_{SIZE}_{cum}.parquet   (schema: query_id, gt_ids)
# matching the schema `recall` reads. "Exact" = brute-force cosine over the corpus, i.e. what the ANN
# index is measured against; this reproduces the method used for the existing standalone
# ground_truth_{query}_1m.parquet (verified to match to the id, in order).
#
# Query semantics reproduced:
#   knn_top10_unfiltered : top-10 by cosine distance over the whole corpus.
#   knn_top10_1pct       : same, restricted to docs whose text matches 'battle'
#   knn_top10_10pct      : same, restricted to docs whose text matches 'year'
# The text filter mirrors Postgres `to_tsvector('english',text) @@ websearch_to_tsquery('english',W)`
# (the filter terms are the cohere.titles_* GUCs in datasets/cohere/after_create_index.sql): the text
# is lowercased, tokenized, and Snowball-stemmed, and matches iff stem(W) is present -- DuckDB's
# 'english' stemmer is the same Snowball stemmer Postgres uses.
#
# Requires the DuckDB CLI (with the fts extension) and AWS credentials on the standard chain. Usage:
#   ./generate_cohere_ground_truth.sh <size> [chunks] [s3://bucket/datasets/cohere]
set -euo pipefail

SIZE="${1:?usage: generate_cohere_ground_truth.sh <size> [chunks] [base]  (e.g. 1m)}"
CHUNKS="${2:-10}"
BASE="${3:-s3://paradedb-benchmarks/datasets/cohere}"
BASE="${BASE%/}"
CHUNK_GLOB="${BASE}/sampled/${SIZE}_chunk*/parquet/cohere_wiki/*.parquet"
QUERIES="${BASE}/queries/cohere_queries.parquet"

SPILL="$(mktemp -d)"
trap 'rm -rf "${SPILL}"' EXIT

# Total rows in the target, to label each cumulative step (chunk 0..k -> (k+1)/CHUNKS of the corpus).
case "$SIZE" in
  *m) TOTAL=$(( ${SIZE%m} * 1000000 )) ;;
  *k) TOTAL=$(( ${SIZE%k} * 1000 )) ;;
  *)  echo "unsupported size '${SIZE}'"; exit 1 ;;
esac
PER=$(( TOTAL / CHUNKS ))

humanize() { # rows -> label (e.g. 100000->100k, 1000000->1m)
  local n=$1
  if (( n % 1000000 == 0 )); then echo "$(( n / 1000000 ))m"; else echo "$(( n / 1000 ))k"; fi
}

QUERY_TYPES=(knn_top10_unfiltered knn_top10_1pct knn_top10_10pct)
FILTER_WORDS=("" "battle" "year")   # "" = no filter; else stem(word) membership

# A doc matches filter word W iff stem(W) is one of its stemmed tokens. stem(W) is a substring of any
# word that stems to it, so `text ILIKE '%stem(W)%'` is a cheap superset filter -- stemming (the
# expensive part) then runs only on those few candidates, not the whole corpus. Build the candidate
# prefilter (union over the filter words) and a per-word id set.
ilike_terms=""
filter_ddl=""
for w in "${FILTER_WORDS[@]}"; do
  [ -z "$w" ] && continue
  [ -n "$ilike_terms" ] && ilike_terms="${ilike_terms} OR "
  ilike_terms="${ilike_terms}text ILIKE '%' || stem('${w}','english') || '%'"
  filter_ddl="${filter_ddl}
    CREATE TEMP TABLE ${w}_docs AS
      SELECT _id FROM cand WHERE list_contains(stems, stem('${w}','english'));"
done

# One bounded-top-10 (arg_min, so no billion-element lists) COPY per (query, step); filtered queries
# restrict the corpus to their id set first via a semi-join.
copies=""
for i in "${!QUERY_TYPES[@]}"; do
  q="${QUERY_TYPES[$i]}"
  word="${FILTER_WORDS[$i]}"
  if [ -z "$word" ]; then filt=""; else filt="AND c._id IN (SELECT _id FROM ${word}_docs)"; fi
  for k in $(seq 0 $((CHUNKS - 1))); do
    cum=$(humanize $(( PER * (k + 1) )))
    out="${BASE}/queries/ground_truth_${q}_upd_${SIZE}_${cum}.parquet"
    copies="${copies}
      COPY (
        SELECT qq.id AS query_id,
               arg_min(c._id, array_cosine_distance(c.emb, qq.emb), 10) AS gt_ids
        FROM q qq CROSS JOIN corpus c
        WHERE c.chunk <= ${k} ${filt}
        GROUP BY qq.id
      ) TO '${out}' (FORMAT PARQUET);"
  done
done

echo "Generating ground truth for ${SIZE}: ${#QUERY_TYPES[@]} queries x ${CHUNKS} steps ..."
duckdb -c "
  INSTALL httpfs; LOAD httpfs; INSTALL fts; LOAD fts;
  CREATE OR REPLACE SECRET s3_secret (TYPE s3, PROVIDER credential_chain);
  SET http_timeout=300; SET http_retries=10; SET temp_directory='${SPILL}';
  SET lambda_syntax='ENABLE_SINGLE_ARROW';
  CREATE TEMP TABLE corpus AS
    SELECT CAST(regexp_extract(filename, '_chunk([0-9]+)/', 1) AS INT) AS chunk, _id, emb::FLOAT[1024] AS emb
    FROM read_parquet('${CHUNK_GLOB}', filename=true);
  CREATE TEMP TABLE cand AS
    SELECT _id, list_transform(regexp_split_to_array(lower(text), '[^a-z0-9]+'), x -> stem(x,'english')) AS stems
    FROM read_parquet('${CHUNK_GLOB}')
    WHERE ${ilike_terms};
  ${filter_ddl}
  CREATE TEMP TABLE q AS
    SELECT id, emb::FLOAT[1024] AS emb FROM read_parquet('${QUERIES}');
  ${copies}
"

echo "Done. Wrote ground_truth_{query}_upd_${SIZE}_{cum}.parquet for cum in each cumulative step."
