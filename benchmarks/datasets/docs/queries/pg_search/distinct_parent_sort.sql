-- Shape: Distinct Parent Sort (Join Explosion)
-- Join: Single Feature (Local Field with Deduplication)
-- Description: The user wants to find documents that match criteria in the pages table (a "Many" side join), ordered by a field on the documents table. Because joining to pages explodes the row count (1 Document -> N Pages), the query requires DISTINCT. This tests the engine's ability to maintain the sort order while deduplicating the join results.

SET paradedb.enable_mixed_fast_field_sort TO off; SET paradedb.enable_join_custom_scan TO off; SELECT DISTINCT
    d.id,
    d.title,
    d.parents
FROM documents d
JOIN files f ON d.id = f."documentId"
JOIN pages p ON f.id = p."fileId"
WHERE
    p."sizeInBytes" > 5000            -- Filter on the "Many" side
    AND d.parents LIKE 'SFR%'       -- Local Filter
ORDER BY
    d.title ASC                     -- Single Feature Sort (Parent Field)
LIMIT 50;

SET paradedb.enable_mixed_fast_field_sort TO off; SET paradedb.enable_join_custom_scan TO on; SELECT DISTINCT
    d.id,
    d.title,
    d.parents
FROM documents d
JOIN files f ON d.id = f."documentId"
JOIN pages p ON f.id = p."fileId"
WHERE
    p."sizeInBytes" > 5000            -- Filter on the "Many" side
    AND d.parents LIKE 'SFR%'       -- Local Filter
ORDER BY
    d.title ASC                     -- Single Feature Sort (Parent Field)
LIMIT 50;

-- Sortedness enabled (no join scan).
SET paradedb.enable_mixed_fast_field_sort TO on; SET paradedb.enable_join_custom_scan TO off; SELECT DISTINCT
    d.id,
    d.title,
    d.parents
FROM documents d
JOIN files f ON d.id = f."documentId"
JOIN pages p ON f.id = p."fileId"
WHERE
    p."sizeInBytes" > 5000            -- Filter on the "Many" side
    AND d.parents LIKE 'SFR%'       -- Local Filter
ORDER BY
    d.title ASC                     -- Single Feature Sort (Parent Field)
LIMIT 50;
