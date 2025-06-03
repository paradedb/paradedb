-- Test use of the pg_ivm extension.

-- Setup
\i common/common_setup.sql

CREATE EXTENSION IF NOT EXISTS pg_ivm;

DROP TABLE IF EXISTS test CASCADE;
CREATE TABLE test (
    id int,
    content TEXT
);

DROP TABLE IF EXISTS test_view CASCADE;
SELECT pgivm.create_immv('test_view', 'SELECT test.*, test.id + 1 as derived FROM test;');

CREATE INDEX test_search_idx ON test_view
USING bm25 (id, content)
WITH (key_field='id');

-- Validate that DML works with/without the custom scan.
SET paradedb.enable_custom_scan = false;
INSERT INTO test VALUES (1, 'pineapple sauce');
UPDATE test SET id = id;
SET paradedb.enable_custom_scan = true;
INSERT INTO test VALUES (2, 'mango sauce');
UPDATE test SET id = id;

-- Confirm that the indexed view is queryable.
SELECT * from test;
SELECT * FROM test_view WHERE test_view.content @@@ 'pineapple';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT * FROM test_view WHERE test_view.content @@@ 'pineapple';

DROP TABLE IF EXISTS test_view CASCADE;
DROP TABLE IF EXISTS test CASCADE;

\i common/common_cleanup.sql 
