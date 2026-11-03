\i common/common_setup.sql

-- NOT over a heap filter must follow three-valued logic: NOT NULL is NULL (#7167)
CREATE TABLE not_heap_null (id int PRIMARY KEY, body text, flag boolean);
INSERT INTO not_heap_null VALUES
    (1, 'hello true', true),
    (2, 'hello false', false),
    (3, 'hello null', NULL),
    (4, 'other', false);
CREATE INDEX not_heap_null_idx ON not_heap_null USING bm25 (id, body) WITH (key_field = 'id');

SET paradedb.enable_filter_pushdown = on;

EXPLAIN (COSTS OFF) SELECT id FROM not_heap_null WHERE body @@@ 'hello' AND NOT flag;
SELECT id FROM not_heap_null WHERE body @@@ 'hello' AND NOT flag ORDER BY id;
-- Both search predicates stay in Tantivy while the non-indexed NOT uses a heap filter.
SELECT line LIKE '%"query_string":"hello"%'
   AND line LIKE '%"query_string":"null"%'
   AND line LIKE '%"must_not":%'
   AND line LIKE '%"heap_filter":"(NOT flag)"%' AS mixed_not_pushdown
FROM explain_redact_oid($q$
    SELECT id FROM not_heap_null
    WHERE body @@@ 'hello' AND NOT (flag AND body @@@ 'null')
$q$) AS plan(line)
WHERE line LIKE '%Tantivy Query:%';
SELECT id FROM not_heap_null WHERE body @@@ 'hello' AND NOT (flag AND body @@@ 'null') ORDER BY id;
SELECT id FROM not_heap_null WHERE body @@@ 'hello' AND flag IS NOT TRUE ORDER BY id;

SET paradedb.enable_custom_scan = off;
SELECT id FROM not_heap_null WHERE body @@@ 'hello' AND NOT flag ORDER BY id;
SELECT id FROM not_heap_null WHERE body @@@ 'hello' AND NOT (flag AND body @@@ 'null') ORDER BY id;
RESET paradedb.enable_custom_scan;

-- Index flag as well: NOT must remain an index predicate with no heap filter.
DROP INDEX not_heap_null_idx;
CREATE INDEX not_heap_null_idx ON not_heap_null USING bm25 (id, body, flag);
SELECT line LIKE '%"query_string":"hello"%'
   AND line LIKE '%"field":"flag"%'
   AND line LIKE '%"value":false%'
   AND line NOT LIKE '%heap_filter%' AS indexed_not_pushdown
FROM explain_redact_oid($q$
    SELECT id FROM not_heap_null WHERE body @@@ 'hello' AND NOT flag
$q$) AS plan(line)
WHERE line LIKE '%Tantivy Query:%';
SELECT id FROM not_heap_null WHERE body @@@ 'hello' AND NOT flag ORDER BY id;

RESET paradedb.enable_filter_pushdown;
DROP TABLE not_heap_null;

\i common/common_cleanup.sql
