\i common/docs_setup.sql

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
