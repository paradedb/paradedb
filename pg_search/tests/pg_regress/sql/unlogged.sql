\i common/common_setup.sql

-- Test: create bm25 index on unlogged table
CREATE UNLOGGED TABLE test_unlogged (
    id SERIAL PRIMARY KEY,
    description TEXT
);
INSERT INTO test_unlogged (description) VALUES
    ('keyboard'), ('mouse'), ('monitor');

CREATE INDEX ON test_unlogged
USING paradedb (id, description);

-- Test: search on unlogged table
SELECT id FROM test_unlogged WHERE description ||| 'keyboard' ORDER BY id;

-- Test: insert after index creation
INSERT INTO test_unlogged (description) VALUES ('headphones');
SELECT id FROM test_unlogged WHERE description ||| 'headphones' ORDER BY id;

-- Test: update
UPDATE test_unlogged SET description = 'trackpad' WHERE description = 'mouse';
SELECT id FROM test_unlogged WHERE description ||| 'mouse' ORDER BY id;
SELECT id FROM test_unlogged WHERE description ||| 'trackpad' ORDER BY id;

-- Test: delete
DELETE FROM test_unlogged WHERE description = 'trackpad';
SELECT id FROM test_unlogged WHERE description ||| 'trackpad' ORDER BY id;

-- Cleanup
DROP TABLE test_unlogged;

\i common/common_cleanup.sql
