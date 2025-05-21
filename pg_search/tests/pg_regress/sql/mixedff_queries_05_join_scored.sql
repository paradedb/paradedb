-- Tests complex join queries with mixed fields and scores

\i common/mixedff_queries_setup.sql

\echo 'Test: join with mixed fast fields and scores'

SET paradedb.enable_mixed_fast_field_exec = true;


-- Test ordering by only a single score.

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT
    documents.id,
    paradedb.score(files.id) AS score
FROM
    documents JOIN files ON documents.id = files.documentId
WHERE
    documents.content @@@ '2023' AND files.title @@@ 'Receipt'
ORDER BY score DESC
LIMIT 10;

SELECT
    documents.id,
    paradedb.score(files.id) AS score
FROM
    documents JOIN files ON documents.id = files.documentId
WHERE
    documents.content @@@ '2023' AND files.title @@@ 'Receipt'
ORDER BY score DESC
LIMIT 10;


-- Test ordering by a summed score.

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT
    documents.id,
    files.id,
    paradedb.score(documents.id) + paradedb.score(files.id) AS score
FROM
    documents JOIN files ON documents.id = files.documentId
WHERE
    documents.content @@@ '2023' AND files.title @@@ 'Receipt'
ORDER BY score DESC
LIMIT 10;

SELECT
    documents.id,
    files.id,
    paradedb.score(documents.id) + paradedb.score(files.id) AS score
FROM
    documents JOIN files ON documents.id = files.documentId
WHERE
    documents.content @@@ '2023' AND files.title @@@ 'Receipt'
ORDER BY score DESC
LIMIT 10;

\i common/mixedff_queries_cleanup.sql
