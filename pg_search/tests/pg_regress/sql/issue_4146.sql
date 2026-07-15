\i common/docs_setup.sql

-- Deferring DISTINCT columns through doc addresses deepens this join plan enough
-- that serializing it overflows the 2MB default stack in debug builds. The real
-- fix is bounding that recursion in the plan serialization path; until it lands,
-- give the session headroom.
SET max_stack_depth = '6MB';
SET paradedb.enable_join_custom_scan TO on;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT DISTINCT
    d.id,
    d.title,
    d.parents
FROM documents d
JOIN files f ON d.id = f."documentId"
JOIN pages p ON f.id = p."fileId"
WHERE
    p."sizeInBytes" > 5000
    AND d.parents LIKE 'SFR%'
    AND d.id @@@ pdb.all()
ORDER BY
    d.title ASC
LIMIT 50;

SELECT DISTINCT
    d.id,
    d.title,
    d.parents
FROM documents d
JOIN files f ON d.id = f."documentId"
JOIN pages p ON f.id = p."fileId"
WHERE
    p."sizeInBytes" > 5000
    AND d.parents LIKE 'SFR%'
    AND d.id @@@ pdb.all()
ORDER BY
    d.title ASC
LIMIT 50;

\i common/docs_cleanup.sql
