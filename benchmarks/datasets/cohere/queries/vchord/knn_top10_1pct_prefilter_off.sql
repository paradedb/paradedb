-- vchordrq prefilter OFF (post-filter): the ANN scan returns candidates, the tsvector predicate
-- filters them, and iterative scan refills to reach LIMIT. Kept in its own file (not a second
-- variant of the prefilter-on query) so `recall` -- which scores only a file's first variant --
-- measures this operating point too.
SET enable_seqscan=off; SET enable_bitmapscan=off; SET enable_sort=off; SET vchordrq.prefilter=off; SET vchordrq.probes={{ vchord_probes_1pct_off }}; SET vchordrq.epsilon={{ vchord_epsilon }}; SELECT _id, title FROM cohere_wiki
WHERE to_tsvector('english', text) @@ websearch_to_tsquery('english', current_setting('cohere.titles_1pct'))
ORDER BY emb <=> current_setting('cohere.qvec')::vector(1024)
LIMIT 10;
