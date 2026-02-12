-- Shape: Distinct Parent Sort (Join Explosion)
-- Join: Single Feature (Local Field with Deduplication)
-- Description: The user wants to find documents that match criteria in the pages table (a "Many" side join), ordered by a field on the documents table. Because joining to pages explodes the row count (1 Document -> N Pages), the query requires DISTINCT. This tests the engine's ability to maintain the sort order while deduplicating the join results.

SET paradedb.enable_join_custom_scan TO off; SELECT DISTINCT
    d.id,
    d.title,
    d.parents
FROM documents d
JOIN files f ON d.id = f."documentId"
JOIN pages p ON f.id = p."fileId"
WHERE
    p."sizeInBytes" > 5000            -- Filter on the "Many" side
    AND d.parents @@@ 'parent group'
ORDER BY
    d.title ASC                     -- Single Feature Sort (Parent Field)
LIMIT 50;

SET paradedb.enable_join_custom_scan TO on; SELECT DISTINCT
    d.id,
    d.title,
    d.parents
FROM documents d
JOIN files f ON d.id = f."documentId"
JOIN pages p ON f.id = p."fileId"
WHERE
    p."sizeInBytes" > 5000            -- Filter on the "Many" side
    AND d.parents @@@ 'parent group'
ORDER BY
    d.title ASC                     -- Single Feature Sort (Parent Field)
LIMIT 50;

-- Lower bound: uses denormalized matview
SELECT DISTINCT
    djfp.doc_id,
    djfp.doc_title,
    djfp.doc_parents
FROM documents_inner_join_files_inner_join_pages djfp
WHERE
    djfp.page_size_in_bytes > 5000
    AND djfp.doc_parents @@@ 'parent group'
ORDER BY
    djfp.doc_title ASC
LIMIT 50;
