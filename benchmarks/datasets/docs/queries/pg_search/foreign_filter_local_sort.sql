-- Shape: Foreign Filter, Local Sort
-- Join: Single Feature (Fast Field)
-- Description: A standard join where the user filters by a property of the parent table (documents), but sorts by a deterministic "fast field" on the child table (files). The challenge is balancing the selectivity of the foreign filter against the sort order of the local table.

SET paradedb.enable_join_custom_scan TO off; SELECT
    f.id,
    f.title,
    f."createdAt",
    d.title as document_title,
    d.parents as document_parents
FROM files f
JOIN documents d ON f."documentId" = d.id
WHERE
    d.parents @@@ 'parent group'
    AND f.title @@@ 'collab12'
ORDER BY
    f."createdAt" DESC                -- Single Feature Sort (Local Fast Field)
LIMIT 20;

SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT
    f.id,
    f.title,
    f."createdAt",
    d.title as document_title
FROM files f
JOIN documents d ON f."documentId" = d.id
WHERE
    d.parents @@@ 'parent group'
    AND f.title @@@ 'collab12'
ORDER BY
    f."createdAt" DESC                -- Single Feature Sort (Local Fast Field)
LIMIT 20;

-- Lower bound: uses denormalized matview
SELECT
    fid.file_id,
    fid.file_title,
    fid.file_created_at,
    fid.doc_title as document_title
FROM files_inner_join_documents fid
WHERE
    fid.doc_parents @@@ 'parent group'
    AND fid.file_title @@@ 'collab12'
ORDER BY
    fid.file_created_at DESC
LIMIT 20;
