SET paradedb.vector_adaptive_probing = on;
SET paradedb.vector_adaptive_recall_target = {{ pg_search_recall_target }};
SET paradedb.vector_adaptive_recall_exponent = {{ pg_search_recall_exponent_10pct }};
SELECT _id, title FROM cohere_wiki
WHERE text @@@ current_setting('cohere.titles_10pct')
ORDER BY emb <=> current_setting('cohere.qvec')::vector(1024)
LIMIT 10;
