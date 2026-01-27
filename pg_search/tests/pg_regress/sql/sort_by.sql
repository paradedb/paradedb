-- Tests for sort_by option in CREATE INDEX

\i common/common_setup.sql

-- SECTION 1: Basic syntax validation
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

\echo 'Test 1.1: sort_by with id ASC'
CREATE INDEX sort_by_test_idx ON sort_by_test
    USING bm25 (id, name, score)
    WITH (key_field='id', sort_by='id ASC NULLS FIRST');

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, name FROM sort_by_test WHERE sort_by_test @@@ 'name:Alice OR name:Bob' ORDER BY id;

SELECT id, name FROM sort_by_test WHERE sort_by_test @@@ 'name:Alice OR name:Bob' ORDER BY id;

DROP INDEX sort_by_test_idx;

\echo 'Test 1.2: sort_by with id DESC'
CREATE INDEX sort_by_test_idx ON sort_by_test
    USING bm25 (id, name, score)
    WITH (key_field='id', sort_by='id DESC NULLS LAST');

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, name FROM sort_by_test WHERE sort_by_test @@@ 'name:Alice OR name:Bob' ORDER BY id;

SELECT id, name FROM sort_by_test WHERE sort_by_test @@@ 'name:Alice OR name:Bob' ORDER BY id;

DROP INDEX sort_by_test_idx;

\echo 'Test 1.3: sort_by = none (disables segment sorting)'
CREATE INDEX sort_by_test_idx ON sort_by_test
    USING bm25 (id, name, score)
    WITH (key_field='id', sort_by='none');

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, name FROM sort_by_test WHERE sort_by_test @@@ 'name:Alice' ORDER BY id;

SELECT id, name FROM sort_by_test WHERE sort_by_test @@@ 'name:Alice' ORDER BY id;

DROP INDEX sort_by_test_idx;

DROP TABLE sort_by_test CASCADE;

-- SECTION 2: Single segment sorted fetch
\echo '=== SECTION 2: Single segment sorted fetch ==='

DROP TABLE IF EXISTS sort_by_test CASCADE;

CREATE TABLE sort_by_test (
    id SERIAL PRIMARY KEY,
    name TEXT,
    score INTEGER
);

INSERT INTO sort_by_test (name, score) VALUES
    ('Charlie', 50),
    ('Alice', 100),
    ('Eve', 30),
    ('Bob', 80),
    ('Diana', 60);

\echo 'Test 2.1: sort_by score DESC - single segment'
CREATE INDEX sort_by_test_idx ON sort_by_test
    USING bm25 (id, name, score)
    WITH (key_field='id', sort_by='score DESC NULLS LAST');

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

\echo 'Test 2.2: sort_by score ASC - single segment'
CREATE INDEX sort_by_test_idx ON sort_by_test
    USING bm25 (id, name, score)
    WITH (key_field='id', sort_by='score ASC NULLS FIRST');

\echo 'Query without ORDER BY - should return in ASC order'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, name, score FROM sort_by_test
WHERE sort_by_test @@@ 'name:Alice OR name:Bob OR name:Charlie OR name:Diana OR name:Eve';

SELECT id, name, score FROM sort_by_test
WHERE sort_by_test @@@ 'name:Alice OR name:Bob OR name:Charlie OR name:Diana OR name:Eve';

DROP TABLE sort_by_test CASCADE;

-- SECTION 3: Multi-segment sawtooth pattern
\echo '=== SECTION 3: Multi-segment sawtooth pattern ==='

DROP TABLE IF EXISTS sort_by_test CASCADE;

CREATE TABLE sort_by_test (
    id SERIAL PRIMARY KEY,
    category TEXT,
    score INTEGER
);

INSERT INTO sort_by_test (category, score) VALUES
    ('A', 100), ('A', 90), ('A', 80), ('A', 70), ('A', 60);

CREATE INDEX sort_by_test_idx ON sort_by_test
    USING bm25 (id, category, score)
    WITH (key_field='id', sort_by='score DESC NULLS LAST');

\echo 'Segment count after first batch:'
SELECT count(*) as segment_count FROM paradedb.index_info('sort_by_test_idx');

INSERT INTO sort_by_test (category, score) VALUES
    ('A', 95), ('A', 85), ('A', 75), ('A', 65), ('A', 55);

\echo 'Segment count after second batch:'
SELECT count(*) as segment_count FROM paradedb.index_info('sort_by_test_idx');

-- Helper: queries table with (id, category, score) columns
CREATE OR REPLACE FUNCTION query_sort_by(
    table_name TEXT,
    order_clause TEXT DEFAULT ''
) RETURNS SETOF RECORD AS $$
BEGIN
    RETURN QUERY EXECUTE format(
        'SELECT id, category, score FROM %I WHERE %I @@@ ''category:A'' %s',
        table_name, table_name, order_clause
    );
END;
$$ LANGUAGE plpgsql;

\echo 'Test 3.1: Query without ORDER BY shows sawtooth pattern (DESC)'
\echo 'Expected: seg2 [95,85,75,65,55] then seg1 [100,90,80,70,60]'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, category, score FROM sort_by_test
WHERE sort_by_test @@@ 'category:A';

SELECT * FROM query_sort_by('sort_by_test', '') AS t(id INT, category TEXT, score INT);

\echo 'Test 3.2: Query with ORDER BY score DESC (matches sort_by) - global sort'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, category, score FROM sort_by_test
WHERE sort_by_test @@@ 'category:A'
ORDER BY score DESC;

SELECT * FROM query_sort_by('sort_by_test', 'ORDER BY score DESC') AS t(id INT, category TEXT, score INT);

\echo 'Test 3.3: Query with ORDER BY score ASC (opposite of sort_by)'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, category, score FROM sort_by_test
WHERE sort_by_test @@@ 'category:A'
ORDER BY score ASC;

SELECT * FROM query_sort_by('sort_by_test', 'ORDER BY score ASC') AS t(id INT, category TEXT, score INT);

DROP TABLE sort_by_test CASCADE;

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
    WITH (key_field='id', sort_by='score ASC NULLS FIRST');

INSERT INTO sort_by_test (category, score) VALUES
    ('A', 55), ('A', 65), ('A', 75), ('A', 85), ('A', 95);

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, category, score FROM sort_by_test
WHERE sort_by_test @@@ 'category:A';

SELECT id, category, score FROM sort_by_test
WHERE sort_by_test @@@ 'category:A';

DROP TABLE sort_by_test CASCADE;

-- SECTION 4: Default behavior (no sort_by specified)
\echo '=== SECTION 4: Default behavior ==='

\echo 'Test 4.1: No sort_by specified - defaults to none'
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

-- SECTION 5: Error cases
\echo '=== SECTION 5: Error cases ==='

DROP TABLE IF EXISTS sort_by_test CASCADE;

CREATE TABLE sort_by_test (
    id SERIAL PRIMARY KEY,
    name TEXT,
    score INTEGER
);

\echo 'Test 5.1: sort_by with nonexistent field (should error)'
CREATE INDEX sort_by_test_idx ON sort_by_test
    USING bm25 (id, name, score)
    WITH (key_field='id', sort_by='nonexistent ASC NULLS FIRST');

\echo 'Test 5.2: sort_by with non-fast field (should error)'
CREATE INDEX sort_by_test_idx ON sort_by_test
    USING bm25 (id, name, score)
    WITH (key_field='id', sort_by='name ASC NULLS FIRST');

\echo 'Test 5.3: sort_by with invalid syntax (should error)'
CREATE INDEX sort_by_test_idx ON sort_by_test
    USING bm25 (id, name, score)
    WITH (key_field='id', sort_by='id ASCENDING');

\echo 'Test 5.4a: sort_by with ASC NULLS LAST (should error - Tantivy uses NULLS FIRST for ASC)'
CREATE INDEX sort_by_test_idx ON sort_by_test
    USING bm25 (id, name, score)
    WITH (key_field='id', sort_by='id ASC NULLS LAST');

\echo 'Test 5.4b: sort_by with DESC NULLS FIRST (should error - Tantivy uses NULLS LAST for DESC)'
CREATE INDEX sort_by_test_idx ON sort_by_test
    USING bm25 (id, name, score)
    WITH (key_field='id', sort_by='id DESC NULLS FIRST');

\echo 'Test 5.5: sort_by with multiple fields (should error - not supported)'
CREATE INDEX sort_by_test_idx ON sort_by_test
    USING bm25 (id, name, score)
    WITH (key_field='id', sort_by='score DESC NULLS LAST, id ASC NULLS FIRST');

DROP TABLE sort_by_test CASCADE;

-- SECTION 6: Mutable segment sort_by behavior
\echo '=== SECTION 6: Mutable segment sort_by behavior ==='

DROP TABLE IF EXISTS mutable_sort_test CASCADE;

CREATE TABLE mutable_sort_test (
    id SERIAL PRIMARY KEY,
    category TEXT,
    score INTEGER
);

\echo 'Creating index with sort_by=score DESC'
CREATE INDEX mutable_sort_test_idx ON mutable_sort_test
    USING bm25 (id, category, score)
    WITH (key_field='id', sort_by='score DESC NULLS LAST');

\echo 'Inserting data AFTER index creation (goes to mutable segment)'
INSERT INTO mutable_sort_test (category, score) VALUES
    ('A', 50),   -- id=1
    ('A', 100),  -- id=2
    ('A', 30),   -- id=3
    ('A', 80),   -- id=4
    ('A', 60);   -- id=5

\echo 'Test 6.1: Query mutable segment without ORDER BY'
\echo 'Expected: score DESC order (100, 80, 60, 50, 30)'
SELECT id, category, score FROM mutable_sort_test
WHERE mutable_sort_test @@@ 'category:A';

DROP TABLE mutable_sort_test CASCADE;

-- SECTION 7: NULL value handling
\echo '=== SECTION 7: NULL value handling ==='

DROP TABLE IF EXISTS sort_by_null_test CASCADE;
CREATE TABLE sort_by_null_test (
    id SERIAL PRIMARY KEY,
    category TEXT,
    score INTEGER
);

INSERT INTO sort_by_null_test (category, score) VALUES
    ('A', 100), ('A', NULL), ('A', 50), ('A', NULL), ('A', 75);

CREATE INDEX sort_by_null_test_idx ON sort_by_null_test
    USING bm25 (id, category, score) WITH (key_field='id', sort_by='score ASC NULLS FIRST');

\echo 'Test 7.1: ASC NULLS FIRST - Expected: NULL, NULL, 50, 75, 100'
SELECT * FROM query_sort_by('sort_by_null_test', '') AS t(id INT, category TEXT, score INT);

\echo 'Test 7.2: ALTER to DESC NULLS LAST - Expected: 100, 75, 50, NULL, NULL'
ALTER INDEX sort_by_null_test_idx SET (sort_by='score DESC NULLS LAST');
REINDEX INDEX sort_by_null_test_idx;
SELECT * FROM query_sort_by('sort_by_null_test', '') AS t(id INT, category TEXT, score INT);

DROP TABLE sort_by_null_test CASCADE;

-- SECTION 8: Different field types
\echo '=== SECTION 8: Different field types ==='
DROP TABLE IF EXISTS sort_by_types_test CASCADE;
CREATE TABLE sort_by_types_test (
    id SERIAL PRIMARY KEY,
    category TEXT,
    ts_val TIMESTAMP,
    real_val REAL,
    bigint_val BIGINT
);

INSERT INTO sort_by_types_test (category, ts_val, real_val, bigint_val) VALUES
    ('A', '2023-03-15 10:00:00', 19.99, 9223372036854775800),
    ('A', '2023-01-01 10:00:00', 5.50, 1000000000000),
    ('A', '2023-06-20 10:00:00', 99.95, 9223372036854775807),
    ('A', '2023-02-10 10:00:00', 25.00, 5000000000000);

CREATE OR REPLACE FUNCTION test_sort_by_type(sort_by_option TEXT)
RETURNS SETOF RECORD AS $$
BEGIN
    DROP INDEX IF EXISTS sort_by_types_test_idx;
    EXECUTE format(
        'CREATE INDEX sort_by_types_test_idx ON sort_by_types_test
         USING bm25 (id, category, ts_val, real_val, bigint_val)
         WITH (key_field=''id'', sort_by=%L)',
        sort_by_option
    );
    RETURN QUERY SELECT id, category, ts_val, real_val, bigint_val
        FROM sort_by_types_test WHERE sort_by_types_test @@@ 'category:A';
END;
$$ LANGUAGE plpgsql;

\echo 'Test 8.1: TIMESTAMP DESC - Expected: 2023-06-20, 2023-03-15, 2023-02-10, 2023-01-01'
SELECT * FROM test_sort_by_type('ts_val DESC NULLS LAST')
    AS t(id INT, category TEXT, ts_val TIMESTAMP, real_val REAL, bigint_val BIGINT);

\echo 'Test 8.2: REAL ASC - Expected: 5.50, 19.99, 25.00, 99.95'
SELECT * FROM test_sort_by_type('real_val ASC NULLS FIRST')
    AS t(id INT, category TEXT, ts_val TIMESTAMP, real_val REAL, bigint_val BIGINT);

\echo 'Test 8.3: BIGINT DESC - Expected: max BIGINT values descending'
SELECT * FROM test_sort_by_type('bigint_val DESC NULLS LAST')
    AS t(id INT, category TEXT, ts_val TIMESTAMP, real_val REAL, bigint_val BIGINT);

DROP FUNCTION test_sort_by_type;
DROP TABLE sort_by_types_test CASCADE;

-- SECTION 9: REINDEX behavior
\echo '=== SECTION 9: REINDEX behavior ==='

DROP TABLE IF EXISTS sort_by_reindex_test CASCADE;
CREATE TABLE sort_by_reindex_test (
    id SERIAL PRIMARY KEY,
    category TEXT,
    score INTEGER
);

INSERT INTO sort_by_reindex_test (category, score) VALUES
    ('A', 30), ('A', 100), ('A', 50), ('A', 80);

CREATE INDEX sort_by_reindex_test_idx ON sort_by_reindex_test
    USING bm25 (id, category, score) WITH (key_field='id', sort_by='score DESC NULLS LAST');

\echo 'Test 9.1: DESC after REINDEX - Expected: 100, 80, 50, 30'
REINDEX INDEX sort_by_reindex_test_idx;
SELECT * FROM query_sort_by('sort_by_reindex_test', '') AS t(id INT, category TEXT, score INT);

\echo 'Test 9.2: ALTER to ASC + REINDEX - Expected: 30, 50, 80, 100'
ALTER INDEX sort_by_reindex_test_idx SET (sort_by='score ASC NULLS FIRST');
REINDEX INDEX sort_by_reindex_test_idx;
SELECT * FROM query_sort_by('sort_by_reindex_test', '') AS t(id INT, category TEXT, score INT);

DROP TABLE sort_by_reindex_test CASCADE;

-- Cleanup helper function
DROP FUNCTION query_sort_by;
