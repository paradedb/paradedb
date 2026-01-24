-- Comprehensive tests for sort_by option in CREATE INDEX
-- Covers: syntax validation, sorted fetch, sawtooth patterns, errors

\i common/common_setup.sql

-- ============================================================================
-- SECTION 1: Basic syntax validation
-- ============================================================================
\echo '=== SECTION 1: Basic syntax validation ==='

DROP TABLE IF EXISTS sort_by_test CASCADE;
CREATE TABLE sort_by_test (
    id SERIAL PRIMARY KEY,
    name TEXT,
    score INTEGER,
    created_at TIMESTAMP
);

INSERT INTO sort_by_test (name, score, created_at) VALUES
    ('Alice', 100, '2023-01-01'),
    ('Bob', 200, '2023-06-01'),
    ('Charlie', 150, '2023-12-01');

-- Test 1.1: sort_by with ASC
\echo 'Test 1.1: sort_by with id ASC'
CREATE INDEX sort_by_test_idx ON sort_by_test
    USING bm25 (id, name, score)
    WITH (key_field='id', sort_by='id ASC');

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, name FROM sort_by_test WHERE sort_by_test @@@ 'name:Alice OR name:Bob' ORDER BY id;

SELECT id, name FROM sort_by_test WHERE sort_by_test @@@ 'name:Alice OR name:Bob' ORDER BY id;

DROP INDEX sort_by_test_idx;

-- Test 1.2: sort_by with DESC
\echo 'Test 1.2: sort_by with id DESC'
CREATE INDEX sort_by_test_idx ON sort_by_test
    USING bm25 (id, name, score)
    WITH (key_field='id', sort_by='id DESC');

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, name FROM sort_by_test WHERE sort_by_test @@@ 'name:Alice OR name:Bob' ORDER BY id;

SELECT id, name FROM sort_by_test WHERE sort_by_test @@@ 'name:Alice OR name:Bob' ORDER BY id;

DROP INDEX sort_by_test_idx;

-- Test 1.3: sort_by = 'none'
\echo 'Test 1.3: sort_by = none (disables segment sorting)'
CREATE INDEX sort_by_test_idx ON sort_by_test
    USING bm25 (id, name, score)
    WITH (key_field='id', sort_by='none');

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, name FROM sort_by_test WHERE sort_by_test @@@ 'name:Alice' ORDER BY id;

SELECT id, name FROM sort_by_test WHERE sort_by_test @@@ 'name:Alice' ORDER BY id;

DROP INDEX sort_by_test_idx;

DROP TABLE sort_by_test CASCADE;

-- ============================================================================
-- SECTION 2: Single segment sorted fetch
-- ============================================================================
\echo '=== SECTION 2: Single segment sorted fetch ==='

DROP TABLE IF EXISTS sort_by_test CASCADE;

CREATE TABLE sort_by_test (
    id SERIAL PRIMARY KEY,
    name TEXT,
    score INTEGER
);

-- Insert data in RANDOM order (not sorted by score)
INSERT INTO sort_by_test (name, score) VALUES
    ('Charlie', 50),
    ('Alice', 100),
    ('Eve', 30),
    ('Bob', 80),
    ('Diana', 60);

-- Test 2.1: sort_by score DESC - verify Tantivy returns sorted
\echo 'Test 2.1: sort_by score DESC - single segment'
CREATE INDEX sort_by_test_idx ON sort_by_test
    USING bm25 (id, name, score)
    WITH (key_field='id', sort_by='score DESC');

\echo 'Query without ORDER BY - should return in segment sorted order (DESC)'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, name, score FROM sort_by_test
WHERE sort_by_test @@@ 'name:Alice OR name:Bob OR name:Charlie OR name:Diana OR name:Eve';

SELECT id, name, score FROM sort_by_test
WHERE sort_by_test @@@ 'name:Alice OR name:Bob OR name:Charlie OR name:Diana OR name:Eve';

\echo 'Query with ORDER BY score DESC (matches sort_by)'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, name, score FROM sort_by_test
WHERE sort_by_test @@@ 'name:Alice OR name:Bob OR name:Charlie OR name:Diana OR name:Eve'
ORDER BY score DESC;

SELECT id, name, score FROM sort_by_test
WHERE sort_by_test @@@ 'name:Alice OR name:Bob OR name:Charlie OR name:Diana OR name:Eve'
ORDER BY score DESC;

\echo 'Query with ORDER BY score ASC (opposite of sort_by)'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, name, score FROM sort_by_test
WHERE sort_by_test @@@ 'name:Alice OR name:Bob OR name:Charlie OR name:Diana OR name:Eve'
ORDER BY score ASC;

SELECT id, name, score FROM sort_by_test
WHERE sort_by_test @@@ 'name:Alice OR name:Bob OR name:Charlie OR name:Diana OR name:Eve'
ORDER BY score ASC;

\echo 'Query with ORDER BY score DESC LIMIT 3'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, name, score FROM sort_by_test
WHERE sort_by_test @@@ 'name:Alice OR name:Bob OR name:Charlie OR name:Diana OR name:Eve'
ORDER BY score DESC
LIMIT 3;

SELECT id, name, score FROM sort_by_test
WHERE sort_by_test @@@ 'name:Alice OR name:Bob OR name:Charlie OR name:Diana OR name:Eve'
ORDER BY score DESC
LIMIT 3;

DROP INDEX sort_by_test_idx;

-- Test 2.2: sort_by score ASC
\echo 'Test 2.2: sort_by score ASC - single segment'
CREATE INDEX sort_by_test_idx ON sort_by_test
    USING bm25 (id, name, score)
    WITH (key_field='id', sort_by='score ASC');

\echo 'Query without ORDER BY - should return in ASC order'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, name, score FROM sort_by_test
WHERE sort_by_test @@@ 'name:Alice OR name:Bob OR name:Charlie OR name:Diana OR name:Eve';

SELECT id, name, score FROM sort_by_test
WHERE sort_by_test @@@ 'name:Alice OR name:Bob OR name:Charlie OR name:Diana OR name:Eve';

DROP TABLE sort_by_test CASCADE;

-- ============================================================================
-- SECTION 3: Multi-segment sawtooth pattern
-- ============================================================================
\echo '=== SECTION 3: Multi-segment sawtooth pattern ==='

DROP TABLE IF EXISTS sort_by_test CASCADE;

CREATE TABLE sort_by_test (
    id SERIAL PRIMARY KEY,
    category TEXT,
    score INTEGER
);

-- Insert first batch
INSERT INTO sort_by_test (category, score) VALUES
    ('A', 100), ('A', 90), ('A', 80), ('A', 70), ('A', 60);

-- Create index with sort_by score DESC
CREATE INDEX sort_by_test_idx ON sort_by_test
    USING bm25 (id, category, score)
    WITH (key_field='id', sort_by='score DESC');

\echo 'Segment count after first batch:'
SELECT count(*) as segment_count FROM paradedb.index_info('sort_by_test_idx');

-- Insert second batch (creates second segment)
INSERT INTO sort_by_test (category, score) VALUES
    ('A', 95), ('A', 85), ('A', 75), ('A', 65), ('A', 55);

\echo 'Segment count after second batch:'
SELECT count(*) as segment_count FROM paradedb.index_info('sort_by_test_idx');

-- Parameterized test helper function for ORDER BY variations
CREATE OR REPLACE FUNCTION test_sort_by_order(
    test_name TEXT,
    order_clause TEXT DEFAULT ''
) RETURNS SETOF RECORD AS $$
BEGIN
    RAISE NOTICE '%', test_name;
    RETURN QUERY EXECUTE format(
        'SELECT id, category, score FROM sort_by_test WHERE sort_by_test @@@ ''category:A'' %s',
        order_clause
    );
END;
$$ LANGUAGE plpgsql;

-- Test 3.1: Sawtooth pattern with DESC
\echo 'Test 3.1: Query without ORDER BY shows sawtooth pattern (DESC)'
\echo 'Expected: seg2 [95,85,75,65,55] then seg1 [100,90,80,70,60]'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, category, score FROM sort_by_test
WHERE sort_by_test @@@ 'category:A';

SELECT * FROM test_sort_by_order('Test 3.1', '') AS t(id INT, category TEXT, score INT);

\echo 'Test 3.2: Query with ORDER BY score DESC (matches sort_by) - global sort'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, category, score FROM sort_by_test
WHERE sort_by_test @@@ 'category:A'
ORDER BY score DESC;

SELECT * FROM test_sort_by_order('Test 3.2', 'ORDER BY score DESC') AS t(id INT, category TEXT, score INT);

\echo 'Test 3.3: Query with ORDER BY score ASC (opposite of sort_by)'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, category, score FROM sort_by_test
WHERE sort_by_test @@@ 'category:A'
ORDER BY score ASC;

SELECT * FROM test_sort_by_order('Test 3.3', 'ORDER BY score ASC') AS t(id INT, category TEXT, score INT);

DROP FUNCTION test_sort_by_order;

DROP TABLE sort_by_test CASCADE;

-- Test 3.4: Sawtooth pattern with ASC
\echo 'Test 3.4: Sawtooth pattern with ASC'
DROP TABLE IF EXISTS sort_by_test CASCADE;

CREATE TABLE sort_by_test (
    id SERIAL PRIMARY KEY,
    category TEXT,
    score INTEGER
);

INSERT INTO sort_by_test (category, score) VALUES
    ('A', 60), ('A', 70), ('A', 80), ('A', 90), ('A', 100);

CREATE INDEX sort_by_test_idx ON sort_by_test
    USING bm25 (id, category, score)
    WITH (key_field='id', sort_by='score ASC');

INSERT INTO sort_by_test (category, score) VALUES
    ('A', 55), ('A', 65), ('A', 75), ('A', 85), ('A', 95);

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, category, score FROM sort_by_test
WHERE sort_by_test @@@ 'category:A';

SELECT id, category, score FROM sort_by_test
WHERE sort_by_test @@@ 'category:A';

DROP TABLE sort_by_test CASCADE;

-- ============================================================================
-- SECTION 4: Default behavior (no sort_by specified)
-- ============================================================================
\echo '=== SECTION 4: Default behavior ==='

-- Test 4.1: No sort_by specified (defaults to ctid ASC)
\echo 'Test 4.1: No sort_by specified - defaults to ctid ASC'
DROP TABLE IF EXISTS sort_by_test CASCADE;

CREATE TABLE sort_by_test (
    id SERIAL PRIMARY KEY,
    category TEXT,
    score INTEGER
);

INSERT INTO sort_by_test (category, score) VALUES
    ('A', 100), ('A', 90), ('A', 80);

CREATE INDEX sort_by_test_idx ON sort_by_test
    USING bm25 (id, category, score)
    WITH (key_field='id');

INSERT INTO sort_by_test (category, score) VALUES
    ('A', 95), ('A', 85), ('A', 75);

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, category, score FROM sort_by_test
WHERE sort_by_test @@@ 'category:A';

SELECT id, category, score FROM sort_by_test
WHERE sort_by_test @@@ 'category:A';

DROP TABLE sort_by_test CASCADE;

-- Test 4.2: sort_by = 'none' (no segment sorting)
\echo 'Test 4.2: sort_by = none - no segment sorting'
DROP TABLE IF EXISTS sort_by_test CASCADE;

CREATE TABLE sort_by_test (
    id SERIAL PRIMARY KEY,
    category TEXT,
    score INTEGER
);

INSERT INTO sort_by_test (category, score) VALUES
    ('A', 100), ('A', 90), ('A', 80);

CREATE INDEX sort_by_test_idx ON sort_by_test
    USING bm25 (id, category, score)
    WITH (key_field='id', sort_by='none');

INSERT INTO sort_by_test (category, score) VALUES
    ('A', 95), ('A', 85), ('A', 75);

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, category, score FROM sort_by_test
WHERE sort_by_test @@@ 'category:A';

SELECT id, category, score FROM sort_by_test
WHERE sort_by_test @@@ 'category:A';

DROP TABLE sort_by_test CASCADE;

-- ============================================================================
-- SECTION 5: Error cases
-- ============================================================================
\echo '=== SECTION 5: Error cases ==='

DROP TABLE IF EXISTS sort_by_test CASCADE;

CREATE TABLE sort_by_test (
    id SERIAL PRIMARY KEY,
    name TEXT,
    score INTEGER
);

-- Test 5.1: Nonexistent field
\echo 'Test 5.1: sort_by with nonexistent field (should error)'
CREATE INDEX sort_by_test_idx ON sort_by_test
    USING bm25 (id, name, score)
    WITH (key_field='id', sort_by='nonexistent ASC');

-- Test 5.2: Non-fast field
\echo 'Test 5.2: sort_by with non-fast field (should error)'
CREATE INDEX sort_by_test_idx ON sort_by_test
    USING bm25 (id, name, score)
    WITH (key_field='id', sort_by='name ASC');

-- Test 5.3: Invalid syntax
\echo 'Test 5.3: sort_by with invalid syntax (should error)'
CREATE INDEX sort_by_test_idx ON sort_by_test
    USING bm25 (id, name, score)
    WITH (key_field='id', sort_by='id ASCENDING');

-- Test 5.4: NULLS FIRST/LAST not supported
\echo 'Test 5.4: sort_by with NULLS FIRST/LAST (should error - not supported)'
CREATE INDEX sort_by_test_idx ON sort_by_test
    USING bm25 (id, name, score)
    WITH (key_field='id', sort_by='id ASC NULLS FIRST');

-- Test 5.5: Multiple fields not supported
\echo 'Test 5.5: sort_by with multiple fields (should error - not supported)'
CREATE INDEX sort_by_test_idx ON sort_by_test
    USING bm25 (id, name, score)
    WITH (key_field='id', sort_by='score DESC, id ASC');

DROP TABLE sort_by_test CASCADE;
