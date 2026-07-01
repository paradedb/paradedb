-- Filtered kNN, forced onto the ANN index and tuned for ~95% recall@10. enable_*=off forces the
-- planner off the exact pre-filter plan; probes/ef_search come from config.toml [params] (scaled by
-- dataset_size). Both index GUCs are set so one file works for either index -- the unused index
-- ignores its GUC. SETs are inline with the SELECT (single statement) so the harness applies them.
SET enable_seqscan=off; SET enable_bitmapscan=off; SET enable_sort=off; SET ivfflat.iterative_scan=relaxed_order; SET hnsw.iterative_scan=relaxed_order; SET ivfflat.probes={{ probes_1pct }}; SET hnsw.ef_search={{ ef_search_1pct }}; SELECT _id, title FROM cohere_wiki
WHERE to_tsvector('english', text) @@ websearch_to_tsquery('english', current_setting('cohere.titles_1pct'))
ORDER BY emb <=> current_setting('cohere.qvec')::vector(1024)
LIMIT 10;
