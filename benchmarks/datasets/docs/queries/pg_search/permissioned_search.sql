-- Shape: Permissioned Search (Score Sort)
-- Join: Single Feature (BM25 Score)
-- Description: A Full Text Search (BM25) drives the ranking, but the result set must be restricted by a JOIN (e.g., checking permissions or document isolation). The score comes purely from the files table, but the validity of the row depends on the documents table.

SET paradedb.dynamic_filter_batch_size = 128; SET work_mem TO '4GB'; SET paradedb.enable_columnar_sort TO on; SET paradedb.enable_join_custom_scan TO on; EXPLAIN ANALYZE SELECT
    f.id,
    f.title,
    paradedb.score(f.id) as relevance
FROM files f
JOIN documents d ON f."documentId" = d.id
WHERE
    f.title @@@ 'File'              -- Driving the Sort (Single Feature)
    AND d.parents @@@ 'parent group'
ORDER BY
    relevance DESC
LIMIT 10;

SET paradedb.dynamic_filter_batch_size = 8192; SET work_mem TO '4GB'; SET paradedb.enable_columnar_sort TO on; SET paradedb.enable_join_custom_scan TO on; EXPLAIN ANALYZE SELECT
    f.id,
    f.title,
    paradedb.score(f.id) as relevance
FROM files f
JOIN documents d ON f."documentId" = d.id
WHERE
    f.title @@@ 'File'              -- Driving the Sort (Single Feature)
    AND d.parents @@@ 'parent group'
ORDER BY
    relevance DESC
LIMIT 10;
