\i common/common_setup.sql

-- Single-index guard: because `paradedb` and `bm25` share a handler, they count
-- as the same index for the "one index per relation" rule. With a `bm25` index
-- already present, creating a `paradedb` index non-concurrently is rejected...
CREATE TABLE guard_test (id INTEGER PRIMARY KEY, content TEXT);
INSERT INTO guard_test (id, content) VALUES (1, 'wireless keyboard'), (2, 'wired mouse');

CREATE INDEX guard_bm25_idx ON guard_test USING bm25 (id, content);

CREATE INDEX guard_paradedb_idx ON guard_test USING paradedb (id, content);

-- ...but CREATE INDEX CONCURRENTLY bypasses the guard, so the two coexist.
CREATE INDEX CONCURRENTLY guard_paradedb_idx ON guard_test USING paradedb (id, content);

SELECT c.relname, am.amname
FROM pg_class c
JOIN pg_am am ON c.relam = am.oid
WHERE c.relname IN ('guard_bm25_idx', 'guard_paradedb_idx')
ORDER BY c.relname;

DROP TABLE guard_test CASCADE;
