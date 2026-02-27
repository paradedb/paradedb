\i common/common_setup.sql

-- Ensure clean state
DROP TABLE IF EXISTS booltest_simple;

-- Create table
CREATE TABLE booltest_simple (
                                 id SERIAL PRIMARY KEY,
                                 description TEXT,
                                 flag BOOLEAN
);

-- Insert data
INSERT INTO booltest_simple (description, flag)
VALUES
    ('hello world', true),
    ('hello parade', false),
    ('other text', true);

-- Create ParadeDB index
CREATE INDEX booltest_simple_idx
    ON booltest_simple
    USING bm25 (id, description)
    WITH (key_field = 'id');

-- Force ParadeDB planner path
SET enable_seqscan = off;
SET enable_indexscan = off;
SET enable_bitmapscan = off;
SET paradedb.enable_filter_pushdown = on;

-- Verify BoolTest fallback stays inside Custom Scan
EXPLAIN (COSTS OFF)
SELECT *
FROM booltest_simple
WHERE description @@@ 'hello'
  AND flag IS TRUE;

SELECT *
FROM booltest_simple
WHERE description @@@ 'hello'
  AND flag IS TRUE
ORDER BY id;

-- Cleanup
DROP INDEX IF EXISTS booltest_simple_idx;
DROP TABLE booltest_simple;

\i common/common_cleanup.sql