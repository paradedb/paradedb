-- Shape: Permissioned Search (Score Sort)
-- Join: Single Feature (BM25 Score)
-- Description: A Full Text Search (BM25) drives the ranking, but the result set must be restricted by a JOIN (e.g., checking permissions or document isolation). The score comes purely from the files table, but the validity of the row depends on the documents table.

SET paradedb.enable_mixed_fast_field_sort TO off; SET paradedb.enable_join_custom_scan TO off; SELECT
    f.id,
    f.title,
    paradedb.score(f.id) as relevance
FROM files f
JOIN documents d ON f."documentId" = d.id
WHERE
    f.title @@@ 'File'              -- Driving the Sort (Single Feature)
    AND d.parents LIKE 'PARENT_GROUP_10%' -- Hard Constraint (Permission)
ORDER BY
    relevance DESC
LIMIT 10;

SET work_mem TO '4GB'; SET paradedb.enable_mixed_fast_field_sort TO off; SET paradedb.enable_join_custom_scan TO on; SELECT
    f.id,
    f.title,
    paradedb.score(f.id) as relevance
FROM files f
JOIN documents d ON f."documentId" = d.id
WHERE
    f.title @@@ 'File'              -- Driving the Sort (Single Feature)
    AND d.parents LIKE 'PARENT_GROUP_10%' -- Hard Constraint (Permission)
ORDER BY
    relevance DESC
LIMIT 10;

-- Sortedness enabled (no join scan).
SET paradedb.enable_mixed_fast_field_sort TO on; SET paradedb.enable_join_custom_scan TO off; SELECT
    f.id,
    f.title,
    paradedb.score(f.id) as relevance
FROM files f
JOIN documents d ON f."documentId" = d.id
WHERE
    f.title @@@ 'File'              -- Driving the Sort (Single Feature)
    AND d.parents LIKE 'PARENT_GROUP_10%' -- Hard Constraint (Permission)
ORDER BY
    relevance DESC
LIMIT 10;
