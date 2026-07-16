-- vchordrq prefilter ON: the ANN scan evaluates the tsvector predicate during traversal, so the
-- LIMIT budget isn't spent on rows that fail the filter.
SET enable_seqscan=off; SET enable_bitmapscan=off; SET enable_sort=off; SET vchordrq.prefilter=on; SET vchordrq.probes={{ vchord_probes_10pct_on }}; SET vchordrq.epsilon={{ vchord_epsilon }}; SELECT _id, title FROM cohere_wiki
WHERE to_tsvector('english', text) @@ websearch_to_tsquery('english', current_setting('cohere.titles_10pct'))
ORDER BY emb <=> current_setting('cohere.qvec')::vector(1024)
LIMIT 10;
