-- Test use of the pg_ivm extension.

-- Setup
\i common/common_setup.sql

CREATE EXTENSION IF NOT EXISTS pg_ivm;

DROP TABLE IF EXISTS test CASCADE;
CREATE TABLE test (
    id int
);

INSERT INTO test VALUES (1);

DROP TABLE IF EXISTS test_view CASCADE;
SELECT pgivm.create_immv('test_view', 'SELECT * FROM test;');

CREATE INDEX test_search_idx ON test_view
USING bm25 (id)
WITH (key_field='id');

-- works with custom scans disabled
SET paradedb.enable_custom_scan = false;
UPDATE test SET id = id;

-- fails with custom scans enabled
SET paradedb.enable_custom_scan = true;
UPDATE test SET id = id;

SELECT * from test;
SELECT * from test_view;

DROP TABLE IF EXISTS test_view CASCADE;
DROP TABLE IF EXISTS test CASCADE;

\i common/common_cleanup.sql 
