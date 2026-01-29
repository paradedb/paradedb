-- Advanced Composite Type Tests for pg_search
-- Tests: parallel build, MVCC visibility, catchup indexing, fast fields

\i common/composite_advanced_setup.sql

------------------------------------------------------------
-- TEST: Parallel index build with composite types
------------------------------------------------------------

-- Enable parallel workers for this test
SET max_parallel_workers_per_gather = 4;
SET max_parallel_maintenance_workers = 4;
SET maintenance_work_mem = '256MB';

CREATE TYPE parallel_comp AS (f1 TEXT, f2 TEXT, f3 TEXT);

CREATE TABLE parallel_test (
    id SERIAL PRIMARY KEY,
    f1 TEXT,
    f2 TEXT,
    f3 TEXT
);

-- Insert enough rows with large text to trigger parallel build (35000 rows, >15MB)
-- Each row ~500 bytes to exceed the 15MB threshold for parallel segment creation
INSERT INTO parallel_test (f1, f2, f3)
SELECT
    'field1_' || i || ' lorem ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod tempor incididunt ut labore et dolore magna aliqua',
    'field2_' || i || ' ut enim ad minim veniam quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat duis aute irure',
    'field3_' || i || ' dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur excepteur sint occaecat cupidatat non'
FROM generate_series(1, 35000) AS i;

-- Create index with target_segment_count to verify parallel build
-- Multiple segments indicate parallel workers were used
CREATE INDEX idx_parallel ON parallel_test USING bm25 (
    id, (ROW(f1, f2, f3)::parallel_comp)
) WITH (key_field='id', target_segment_count=4);

-- Verify parallel build created multiple segments (proves parallel workers were used)
SELECT COUNT(*) AS segment_count FROM paradedb.index_info('idx_parallel');
SELECT SUM(num_docs) AS total_docs FROM paradedb.index_info('idx_parallel');

-- Verify search works on parallel-built index
-- Note: EXPLAIN omitted for parallel queries as plans vary with parallel workers
SELECT COUNT(*) FROM parallel_test WHERE id @@@ pdb.parse('f1:field1_5000');
SELECT COUNT(*) FROM parallel_test WHERE id @@@ pdb.parse('f2:field2_1');
SELECT COUNT(*) FROM parallel_test WHERE id @@@ pdb.parse('f3:field3_35000');

-- Verify bulk search works (confirms parallel build indexed all rows correctly)
SELECT COUNT(*) AS rows_1_to_100 FROM parallel_test
WHERE id @@@ pdb.parse('f1:field1_1 OR f1:field1_50 OR f1:field1_100');
SELECT COUNT(*) AS rows_high_range FROM parallel_test
WHERE id @@@ pdb.parse('f1:field1_34998 OR f1:field1_34999 OR f1:field1_35000');

-- Reset parallel settings
SET max_parallel_workers_per_gather = 0;

------------------------------------------------------------
-- TEST: MVCC visibility with composite types
------------------------------------------------------------

CREATE TYPE mvcc_comp AS (content TEXT);

CREATE TABLE mvcc_test (
    id SERIAL PRIMARY KEY,
    content TEXT
);

-- Insert initial rows
INSERT INTO mvcc_test (content) VALUES
    ('unique_alpha_one'),
    ('unique_beta_two'),
    ('unique_gamma_three');

-- Create index
CREATE INDEX idx_mvcc ON mvcc_test USING bm25 (
    id, (ROW(content)::mvcc_comp)
) WITH (key_field='id');

-- Verify segment count after initial insert (should be at least 1)
SELECT COUNT(*) AS initial_segment_count FROM paradedb.index_info('idx_mvcc');

-- Modify data: UPDATE row 1, DELETE row 2, INSERT row 4
UPDATE mvcc_test SET content = 'unique_delta_updated' WHERE id = 1;
DELETE FROM mvcc_test WHERE id = 2;
INSERT INTO mvcc_test (content) VALUES ('unique_epsilon_new');

-- DO NOT VACUUM - forces executor to check heap visibility

-- Test 1: Deleted row's content should NOT be visible
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) AS deleted_not_visible FROM mvcc_test WHERE id @@@ pdb.parse('content:unique_beta_two');
SELECT COUNT(*) AS deleted_not_visible FROM mvcc_test WHERE id @@@ pdb.parse('content:unique_beta_two');

-- Test 2: Updated row's OLD content should NOT be visible
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) AS old_content_not_visible FROM mvcc_test WHERE id @@@ pdb.parse('content:unique_alpha_one');
SELECT COUNT(*) AS old_content_not_visible FROM mvcc_test WHERE id @@@ pdb.parse('content:unique_alpha_one');

-- Test 3: Updated row's NEW content SHOULD be visible
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) AS new_content_visible FROM mvcc_test WHERE id @@@ pdb.parse('content:unique_delta_updated');
SELECT COUNT(*) AS new_content_visible FROM mvcc_test WHERE id @@@ pdb.parse('content:unique_delta_updated');

-- Test 4: Unchanged row SHOULD still be visible
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) AS unchanged_visible FROM mvcc_test WHERE id @@@ pdb.parse('content:unique_gamma_three');
SELECT COUNT(*) AS unchanged_visible FROM mvcc_test WHERE id @@@ pdb.parse('content:unique_gamma_three');

-- Test 5: Newly inserted row SHOULD be visible
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) AS new_row_visible FROM mvcc_test WHERE id @@@ pdb.parse('content:unique_epsilon_new');
SELECT COUNT(*) AS new_row_visible FROM mvcc_test WHERE id @@@ pdb.parse('content:unique_epsilon_new');

-- Test 6: Count all visible rows (should be 3)
SELECT COUNT(*) AS total_visible FROM mvcc_test;

-- Verify segment count after modifications (should not decrease)
SELECT COUNT(*) AS post_modify_segment_count FROM paradedb.index_info('idx_mvcc');

-- Verify total docs in index (may include deleted docs not yet vacuumed, should be >= 3)
SELECT SUM(num_docs) AS total_index_docs FROM paradedb.index_info('idx_mvcc');

------------------------------------------------------------
-- TEST: CREATE INDEX with existing modifications (catchup)
------------------------------------------------------------

CREATE TYPE catchup_comp AS (content TEXT);

CREATE TABLE catchup_test (
    id SERIAL PRIMARY KEY,
    content TEXT
);

-- Insert initial data BEFORE index creation
INSERT INTO catchup_test (content) VALUES
    ('original_one'),
    ('original_two'),
    ('original_three'),
    ('original_four'),
    ('original_five');

-- Make modifications BEFORE index exists
UPDATE catchup_test SET content = 'modified_one' WHERE id = 1;
DELETE FROM catchup_test WHERE id = 2;
INSERT INTO catchup_test (content) VALUES ('inserted_six');

-- NOW create index - must catch up with all modifications
CREATE INDEX idx_catchup ON catchup_test USING bm25 (
    id, (ROW(content)::catchup_comp)
) WITH (key_field='id');

-- Verify index structure via paradedb.index_info (should have at least 1 segment)
SELECT COUNT(*) AS segment_count FROM paradedb.index_info('idx_catchup');

-- Verify index reflects current state, not original state

-- Modified row should have new content indexed
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) AS modified_found FROM catchup_test WHERE id @@@ pdb.parse('content:modified_one');
SELECT COUNT(*) AS modified_found FROM catchup_test WHERE id @@@ pdb.parse('content:modified_one');

-- Original content should NOT be found
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) AS original_not_found FROM catchup_test WHERE id @@@ pdb.parse('content:original_one');
SELECT COUNT(*) AS original_not_found FROM catchup_test WHERE id @@@ pdb.parse('content:original_one');

-- Deleted row should not be in index
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) AS deleted_not_found FROM catchup_test WHERE id @@@ pdb.parse('content:original_two');
SELECT COUNT(*) AS deleted_not_found FROM catchup_test WHERE id @@@ pdb.parse('content:original_two');

-- Newly inserted row should be in index
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) AS inserted_found FROM catchup_test WHERE id @@@ pdb.parse('content:inserted_six');
SELECT COUNT(*) AS inserted_found FROM catchup_test WHERE id @@@ pdb.parse('content:inserted_six');

-- Unchanged rows should be in index
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) AS unchanged_found FROM catchup_test WHERE id @@@ pdb.parse('content:original_three');
SELECT COUNT(*) AS unchanged_found FROM catchup_test WHERE id @@@ pdb.parse('content:original_three');

-- Total visible rows
SELECT COUNT(*) AS total_rows FROM catchup_test;

------------------------------------------------------------
-- TEST: Fast fields configuration with 50 composite fields
------------------------------------------------------------

-- Create composite type with 50 fields (40 literal/keyword + 10 numeric)
-- Uses pdb.literal for fast keyword fields (v2 API), numeric fields are automatically fast
CREATE TYPE fast_comp_50 AS (
    t01 pdb.literal, t02 pdb.literal, t03 pdb.literal, t04 pdb.literal, t05 pdb.literal,
    t06 pdb.literal, t07 pdb.literal, t08 pdb.literal, t09 pdb.literal, t10 pdb.literal,
    t11 pdb.literal, t12 pdb.literal, t13 pdb.literal, t14 pdb.literal, t15 pdb.literal,
    t16 pdb.literal, t17 pdb.literal, t18 pdb.literal, t19 pdb.literal, t20 pdb.literal,
    t21 pdb.literal, t22 pdb.literal, t23 pdb.literal, t24 pdb.literal, t25 pdb.literal,
    t26 pdb.literal, t27 pdb.literal, t28 pdb.literal, t29 pdb.literal, t30 pdb.literal,
    t31 pdb.literal, t32 pdb.literal, t33 pdb.literal, t34 pdb.literal, t35 pdb.literal,
    t36 pdb.literal, t37 pdb.literal, t38 pdb.literal, t39 pdb.literal, t40 pdb.literal,
    n01 NUMERIC(18,2), n02 NUMERIC(18,2), n03 NUMERIC(18,2), n04 NUMERIC(18,2), n05 NUMERIC(18,2),
    n06 NUMERIC(18,2), n07 NUMERIC(18,2), n08 NUMERIC(18,2), n09 NUMERIC(18,2), n10 NUMERIC(18,2)
);

CREATE TABLE fast_test_50 (
    id SERIAL PRIMARY KEY,
    t01 TEXT, t02 TEXT, t03 TEXT, t04 TEXT, t05 TEXT, t06 TEXT, t07 TEXT, t08 TEXT, t09 TEXT, t10 TEXT,
    t11 TEXT, t12 TEXT, t13 TEXT, t14 TEXT, t15 TEXT, t16 TEXT, t17 TEXT, t18 TEXT, t19 TEXT, t20 TEXT,
    t21 TEXT, t22 TEXT, t23 TEXT, t24 TEXT, t25 TEXT, t26 TEXT, t27 TEXT, t28 TEXT, t29 TEXT, t30 TEXT,
    t31 TEXT, t32 TEXT, t33 TEXT, t34 TEXT, t35 TEXT, t36 TEXT, t37 TEXT, t38 TEXT, t39 TEXT, t40 TEXT,
    n01 NUMERIC(18,2), n02 NUMERIC(18,2), n03 NUMERIC(18,2), n04 NUMERIC(18,2), n05 NUMERIC(18,2),
    n06 NUMERIC(18,2), n07 NUMERIC(18,2), n08 NUMERIC(18,2), n09 NUMERIC(18,2), n10 NUMERIC(18,2)
);

-- Create index with fast fields via v2 API (pdb.literal in composite type)
CREATE INDEX idx_fast_50 ON fast_test_50 USING bm25 (
    id, (ROW(t01,t02,t03,t04,t05,t06,t07,t08,t09,t10,t11,t12,t13,t14,t15,t16,t17,t18,t19,t20,
             t21,t22,t23,t24,t25,t26,t27,t28,t29,t30,t31,t32,t33,t34,t35,t36,t37,t38,t39,t40,
             n01,n02,n03,n04,n05,n06,n07,n08,n09,n10)::fast_comp_50)
) WITH (key_field='id');

-- Insert test data with values in first, middle, and last fields
INSERT INTO fast_test_50 (t01, t20, t40, n01, n05, n10) VALUES
    ('first_text', 'middle_text', 'last_text', 100.00, 500.00, 1000.00),
    ('alpha', 'beta', 'gamma', 10.00, 50.00, 100.00),
    ('first_text', 'other', 'delta', 200.00, 250.00, 300.00);

-- Verify search works on first text field
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) AS t01_count FROM fast_test_50 WHERE id @@@ pdb.parse('t01:first_text');
SELECT COUNT(*) AS t01_count FROM fast_test_50 WHERE id @@@ pdb.parse('t01:first_text');

-- Verify search works on middle text field
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) AS t20_count FROM fast_test_50 WHERE id @@@ pdb.parse('t20:middle_text');
SELECT COUNT(*) AS t20_count FROM fast_test_50 WHERE id @@@ pdb.parse('t20:middle_text');

-- Verify search works on last text field
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) AS t40_count FROM fast_test_50 WHERE id @@@ pdb.parse('t40:last_text');
SELECT COUNT(*) AS t40_count FROM fast_test_50 WHERE id @@@ pdb.parse('t40:last_text');

-- Test ordering by first numeric field (n01)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT t01, n01 FROM fast_test_50
WHERE id @@@ pdb.parse('t01:first_text OR t01:alpha')
ORDER BY n01
LIMIT 3;
SELECT t01, n01 FROM fast_test_50
WHERE id @@@ pdb.parse('t01:first_text OR t01:alpha')
ORDER BY n01
LIMIT 3;

-- Test ordering by last numeric field (n10) with paradedb search
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT t01, n10 FROM fast_test_50
WHERE id @@@ pdb.parse('t01:first_text OR t01:alpha')
ORDER BY n10 DESC
LIMIT 2;
SELECT t01, n10 FROM fast_test_50
WHERE id @@@ pdb.parse('t01:first_text OR t01:alpha')
ORDER BY n10 DESC
LIMIT 2;

------------------------------------------------------------
-- TEST: Maximum field limit test (500 fields)
------------------------------------------------------------

-- Test well beyond the historical Tantivy u8 limit (254 fields)
-- This verifies pg_search can handle very large composite types

DO $$
DECLARE
    type_sql TEXT;
    table_sql TEXT;
    row_expr TEXT;
    idx_sql TEXT;
    i INT;
BEGIN
    -- Build composite type with 500 fields
    type_sql := 'CREATE TYPE max_fields_comp AS (';
    FOR i IN 1..500 LOOP
        type_sql := type_sql || format('f%s TEXT', lpad(i::text, 3, '0'));
        IF i < 500 THEN
            type_sql := type_sql || ', ';
        END IF;
    END LOOP;
    type_sql := type_sql || ')';
    EXECUTE type_sql;

    -- Build table with 500 columns
    table_sql := 'CREATE TABLE max_fields_test (id SERIAL PRIMARY KEY, ';
    FOR i IN 1..500 LOOP
        table_sql := table_sql || format('f%s TEXT', lpad(i::text, 3, '0'));
        IF i < 500 THEN
            table_sql := table_sql || ', ';
        END IF;
    END LOOP;
    table_sql := table_sql || ')';
    EXECUTE table_sql;

    -- Build ROW expression
    row_expr := 'ROW(';
    FOR i IN 1..500 LOOP
        row_expr := row_expr || format('f%s', lpad(i::text, 3, '0'));
        IF i < 500 THEN
            row_expr := row_expr || ', ';
        END IF;
    END LOOP;
    row_expr := row_expr || ')::max_fields_comp';

    -- Create index
    idx_sql := format('CREATE INDEX idx_max_fields ON max_fields_test USING bm25 (id, (%s)) WITH (key_field=''id'')', row_expr);
    EXECUTE idx_sql;

    -- Insert test data
    EXECUTE 'INSERT INTO max_fields_test (f001, f250, f500) VALUES (''first_field'', ''middle_field'', ''last_field'')';
END $$;

-- Verify search works on first, middle, and last fields
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) AS first_field_found FROM max_fields_test WHERE id @@@ pdb.parse('f001:first_field');
SELECT COUNT(*) AS first_field_found FROM max_fields_test WHERE id @@@ pdb.parse('f001:first_field');

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) AS middle_field_found FROM max_fields_test WHERE id @@@ pdb.parse('f250:middle_field');
SELECT COUNT(*) AS middle_field_found FROM max_fields_test WHERE id @@@ pdb.parse('f250:middle_field');

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) AS last_field_found FROM max_fields_test WHERE id @@@ pdb.parse('f500:last_field');
SELECT COUNT(*) AS last_field_found FROM max_fields_test WHERE id @@@ pdb.parse('f500:last_field');

------------------------------------------------------------
-- TEST: Dual composite field limit (400 + 400 = 800 fields)
------------------------------------------------------------

-- Test TWO composite types combined in one index
-- 400 fields in type A + 400 fields in type B = 800 total index fields
-- This tests multiple composites in a single index while staying under
-- PostgreSQL 17's catalog tuple size limit (~8KB for pg_index.indexprs)

-- Create two composite types (400 fields each)
DO $$
DECLARE
    type_sql TEXT;
    i INT;
BEGIN
    -- Create type A with 400 fields (a0001 to a0400)
    type_sql := 'CREATE TYPE composite_adv.comp_a_400 AS (';
    FOR i IN 1..400 LOOP
        type_sql := type_sql || format('a%s TEXT', lpad(i::text, 4, '0'));
        IF i < 400 THEN type_sql := type_sql || ', '; END IF;
    END LOOP;
    type_sql := type_sql || ')';
    EXECUTE type_sql;

    -- Create type B with 400 fields (b0001 to b0400)
    type_sql := 'CREATE TYPE composite_adv.comp_b_400 AS (';
    FOR i IN 1..400 LOOP
        type_sql := type_sql || format('b%s TEXT', lpad(i::text, 4, '0'));
        IF i < 400 THEN type_sql := type_sql || ', '; END IF;
    END LOOP;
    type_sql := type_sql || ')';
    EXECUTE type_sql;
END $$;

-- Create table with columns for both composites (400 for A + 400 for B = 800 + 1 id = 801)
DO $$
DECLARE
    table_sql TEXT;
    i INT;
BEGIN
    table_sql := 'CREATE TABLE dual_comp_test (id SERIAL PRIMARY KEY, ';
    -- Add 400 columns for type A
    FOR i IN 1..400 LOOP
        table_sql := table_sql || format('a%s TEXT, ', lpad(i::text, 4, '0'));
    END LOOP;
    -- Add 400 columns for type B (last one without comma)
    FOR i IN 1..400 LOOP
        table_sql := table_sql || format('b%s TEXT', lpad(i::text, 4, '0'));
        IF i < 400 THEN table_sql := table_sql || ', '; END IF;
    END LOOP;
    table_sql := table_sql || ')';
    EXECUTE table_sql;
END $$;

-- Create index with BOTH composite types (400 + 400 = 800 index fields)
DO $$
DECLARE
    row_a TEXT;
    row_b TEXT;
    idx_sql TEXT;
    i INT;
BEGIN
    -- Build ROW expression for type A (400 fields)
    row_a := 'ROW(';
    FOR i IN 1..400 LOOP
        row_a := row_a || format('a%s', lpad(i::text, 4, '0'));
        IF i < 400 THEN row_a := row_a || ', '; END IF;
    END LOOP;
    row_a := row_a || ')::composite_adv.comp_a_400';

    -- Build ROW expression for type B (400 fields)
    row_b := 'ROW(';
    FOR i IN 1..400 LOOP
        row_b := row_b || format('b%s', lpad(i::text, 4, '0'));
        IF i < 400 THEN row_b := row_b || ', '; END IF;
    END LOOP;
    row_b := row_b || ')::composite_adv.comp_b_400';

    -- Create index with both composites
    idx_sql := format('CREATE INDEX idx_dual_comp ON dual_comp_test USING bm25 (id, (%s), (%s)) WITH (key_field=''id'')', row_a, row_b);
    EXECUTE idx_sql;
END $$;

-- Insert test data (a0400 is the last field in type A)
INSERT INTO dual_comp_test (a0001, a0200, a0400, b0001, b0200, b0400)
VALUES ('first_a', 'mid_a', 'last_a', 'first_b', 'mid_b', 'last_b');

-- Test search on both composite types using pdb.parse
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) AS a_first FROM dual_comp_test WHERE id @@@ pdb.parse('a0001:first_a');
SELECT COUNT(*) AS a_first FROM dual_comp_test WHERE id @@@ pdb.parse('a0001:first_a');

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) AS a_last FROM dual_comp_test WHERE id @@@ pdb.parse('a0400:last_a');
SELECT COUNT(*) AS a_last FROM dual_comp_test WHERE id @@@ pdb.parse('a0400:last_a');

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) AS b_first FROM dual_comp_test WHERE id @@@ pdb.parse('b0001:first_b');
SELECT COUNT(*) AS b_first FROM dual_comp_test WHERE id @@@ pdb.parse('b0001:first_b');

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) AS b_last FROM dual_comp_test WHERE id @@@ pdb.parse('b0400:last_b');
SELECT COUNT(*) AS b_last FROM dual_comp_test WHERE id @@@ pdb.parse('b0400:last_b');

------------------------------------------------------------
-- TEST: pdb functions on composite fields (field @@@ pdb.function())
------------------------------------------------------------

-- pdb.term() on composite field from type A
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) AS term_a FROM dual_comp_test WHERE a0001 @@@ pdb.term('first_a');
SELECT COUNT(*) AS term_a FROM dual_comp_test WHERE a0001 @@@ pdb.term('first_a');

-- pdb.term() on composite field from type B
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) AS term_b FROM dual_comp_test WHERE b0001 @@@ pdb.term('first_b');
SELECT COUNT(*) AS term_b FROM dual_comp_test WHERE b0001 @@@ pdb.term('first_b');

-- pdb.match() on composite field
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) AS match_a FROM dual_comp_test WHERE a0200 @@@ pdb.match('mid_a');
SELECT COUNT(*) AS match_a FROM dual_comp_test WHERE a0200 @@@ pdb.match('mid_a');

-- pdb.regex() on composite field
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) AS regex_b FROM dual_comp_test WHERE b0400 @@@ pdb.regex('last.*');
SELECT COUNT(*) AS regex_b FROM dual_comp_test WHERE b0400 @@@ pdb.regex('last.*');

-- Test pdb functions on MVCC test table composite field
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) AS mvcc_term FROM mvcc_test WHERE content @@@ pdb.term('unique_gamma_three');
SELECT COUNT(*) AS mvcc_term FROM mvcc_test WHERE content @@@ pdb.term('unique_gamma_three');

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) AS mvcc_match FROM mvcc_test WHERE content @@@ pdb.match('unique_delta_updated');
SELECT COUNT(*) AS mvcc_match FROM mvcc_test WHERE content @@@ pdb.match('unique_delta_updated');

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) AS mvcc_regex FROM mvcc_test WHERE content @@@ pdb.regex('unique_.*_new');
SELECT COUNT(*) AS mvcc_regex FROM mvcc_test WHERE content @@@ pdb.regex('unique_.*_new');

-- Test pdb functions on parallel test table composite fields
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) AS parallel_term FROM parallel_test WHERE f1 @@@ pdb.term('field1_100');
SELECT COUNT(*) AS parallel_term FROM parallel_test WHERE f1 @@@ pdb.term('field1_100');

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) AS parallel_match FROM parallel_test WHERE f2 @@@ pdb.match('field2_500');
SELECT COUNT(*) AS parallel_match FROM parallel_test WHERE f2 @@@ pdb.match('field2_500');

-- Test pdb functions on fast fields table
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) AS fast_term FROM fast_test_50 WHERE t01 @@@ pdb.term('first_text');
SELECT COUNT(*) AS fast_term FROM fast_test_50 WHERE t01 @@@ pdb.term('first_text');

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) AS fast_match FROM fast_test_50 WHERE t20 @@@ pdb.match('middle_text');
SELECT COUNT(*) AS fast_match FROM fast_test_50 WHERE t20 @@@ pdb.match('middle_text');

------------------------------------------------------------
-- TEST: TopN queries with pdb functions on composite fields
------------------------------------------------------------

-- TopN with pdb.term() on composite field, ORDER BY score
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, t01, pdb.score(id) as score
FROM fast_test_50 WHERE t01 @@@ pdb.term('first_text')
ORDER BY score DESC, id LIMIT 2;
SELECT id, t01, pdb.score(id) as score
FROM fast_test_50 WHERE t01 @@@ pdb.term('first_text')
ORDER BY score DESC, id LIMIT 2;

-- TopN with pdb.match() on composite field, ORDER BY n01 (fast numeric field)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, t01, n01
FROM fast_test_50 WHERE t01 @@@ pdb.match('first_text OR alpha')
ORDER BY n01 DESC, id LIMIT 3;
SELECT id, t01, n01
FROM fast_test_50 WHERE t01 @@@ pdb.match('first_text OR alpha')
ORDER BY n01 DESC, id LIMIT 3;

-- TopN with pdb functions on parallel test (large table)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, pdb.score(id) as score
FROM parallel_test WHERE f1 @@@ pdb.term('field1_1000')
ORDER BY score DESC, id LIMIT 1;
SELECT id, pdb.score(id) as score
FROM parallel_test WHERE f1 @@@ pdb.term('field1_1000')
ORDER BY score DESC, id LIMIT 1;

\i common/composite_advanced_cleanup.sql
