-- Shape: Semi-Join Filter (Term Set)
-- Join: Single Feature (Fast Field)
-- Description: The sort is simple (e.g., by Title), but the filter involves a "List" logic where the valid IDs are derived from a complex subquery or list. This was previously implemented using a term_set, effectively pushing a semi-join down to the search index.

-- term_set workaround, no join
SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO off; SELECT
    f.id,
    f.title,
    f."createdAt"
FROM files f
WHERE
    f."documentId" @@@ pdb.term_set((
        SELECT array_agg(id) FROM documents WHERE parents @@@ 'PROJECT_ALPHA' AND title @@@ 'Document Title 1'
    ))
ORDER BY
    f.title ASC
LIMIT 25;
