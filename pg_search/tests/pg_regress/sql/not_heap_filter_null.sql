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
SELECT id FROM not_heap_null WHERE body @@@ 'hello' AND NOT (flag AND body @@@ 'null') ORDER BY id;
SELECT id FROM not_heap_null WHERE body @@@ 'hello' AND flag IS NOT TRUE ORDER BY id;

SET paradedb.enable_custom_scan = off;
SELECT id FROM not_heap_null WHERE body @@@ 'hello' AND NOT flag ORDER BY id;
SELECT id FROM not_heap_null WHERE body @@@ 'hello' AND NOT (flag AND body @@@ 'null') ORDER BY id;
RESET paradedb.enable_custom_scan;
RESET paradedb.enable_filter_pushdown;

DROP TABLE not_heap_null;

\i common/common_cleanup.sql
