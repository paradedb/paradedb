\i common/common_setup.sql

-- ============================================================================
-- PART 1: Regular numeric column tests (non-JSON)
-- ============================================================================

CREATE TABLE numeric_pushdown(
    id SERIAL PRIMARY KEY,
    text_col TEXT,
    numeric_col NUMERIC,
    float_col FLOAT4,
    int_col INTEGER
);

INSERT INTO numeric_pushdown(text_col, numeric_col, float_col, int_col)
SELECT
    (ARRAY['Alice', 'Bob', 'Charlie', 'David', 'Eve'])[i % 5 + 1],
    (i % 5)::numeric,
    (i % 5)::float4,
    (i % 5)::integer
FROM generate_series(1, 100) i;

CREATE INDEX numeric_pushdown_idx ON numeric_pushdown USING bm25 (
    id, text_col, numeric_col, float_col, int_col
) WITH (key_field = 'id');

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT * FROM numeric_pushdown
WHERE id @@@ paradedb.all()
AND numeric_col = 1
ORDER BY id LIMIT 10;

SELECT * FROM numeric_pushdown
WHERE id @@@ paradedb.all()
AND numeric_col = 1
ORDER BY id LIMIT 10;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT * FROM numeric_pushdown
WHERE id @@@ paradedb.all()
AND numeric_col > 1
ORDER BY id LIMIT 10;

SELECT * FROM numeric_pushdown
WHERE id @@@ paradedb.all()
AND numeric_col > 1
ORDER BY id LIMIT 10;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT * FROM numeric_pushdown
WHERE id @@@ paradedb.all()
AND numeric_col::int > 1
ORDER BY id LIMIT 10;

SELECT * FROM numeric_pushdown
WHERE id @@@ paradedb.all()
AND numeric_col::int > 1
ORDER BY id LIMIT 10;

DROP TABLE numeric_pushdown;

-- ============================================================================
-- PART 2: JSON Numeric Multi-Type Expansion Tests
-- ============================================================================
-- Tests multi-type expansion for JSON fields across I64, U64, and F64
-- Single JSONB column with all numeric test values
-- ============================================================================

CREATE TABLE json_numeric_types (
    id SERIAL PRIMARY KEY,
    data JSONB
);

-- ============================================================================
-- Test Data Setup
-- ============================================================================
-- Group 1: ONLY I64 - Integers without F64 variants (negative and positive)
-- Group 2: ONLY U64 - Beyond i64::MAX (cannot be I64)
-- Group 3: ONLY F64 - Decimal values (cannot be I64 or U64)
-- Group 4: Cross-type - Same value stored as BOTH I64 and F64
-- Group 5: Boundary values (2^53 boundaries)

INSERT INTO json_numeric_types (data) VALUES
    -- Group 1: ONLY I64 - Negative and positive integers (no F64 variant)
    ('{"num": -9223372036854775808}'::jsonb),  -- i64::MIN
    ('{"num": -1000}'::jsonb),
    ('{"num": -42}'::jsonb),
    ('{"num": -1}'::jsonb),
    ('{"num": 0}'::jsonb),
    ('{"num": 1}'::jsonb),
    ('{"num": 42}'::jsonb),
    ('{"num": 1000}'::jsonb),
    ('{"num": 9223372036854775807}'::jsonb),  -- i64::MAX

    -- Group 2: ONLY U64 - Beyond i64::MAX (cannot be I64)
    ('{"num": 9223372036854775808}'::jsonb),  -- i64::MAX + 1
    ('{"num": 10000000000000000000}'::jsonb),
    ('{"num": 18446744073709551613}'::jsonb),  -- u64::MAX - 2
    ('{"num": 18446744073709551614}'::jsonb),  -- u64::MAX - 1
    ('{"num": 18446744073709551615}'::jsonb),  -- u64::MAX

    -- Group 3: ONLY F64 - Decimal values (cannot be I64 or U64)
    ('{"num": -42.5}'::jsonb),
    ('{"num": -3.14159}'::jsonb),
    ('{"num": 0.5}'::jsonb),
    ('{"num": 3.14159}'::jsonb),
    ('{"num": 42.5}'::jsonb),

    -- Group 4: Cross-type - Same value stored as BOTH I64 and F64
    -- These rows demonstrate multi-type expansion: query for 100 matches both
    ('{"num": 100}'::jsonb),     -- Stored as I64
    ('{"num": 100.0}'::jsonb),   -- Stored as F64
    ('{"num": 999}'::jsonb),     -- Stored as I64
    ('{"num": 999.0}'::jsonb),   -- Stored as F64

    -- Group 5: Boundary values (2^53 - precision boundary for F64)
    ('{"num": -9007199254740992}'::jsonb),  -- -2^53
    ('{"num": -9007199254740991}'::jsonb),  -- -(2^53 - 1)
    ('{"num": 9007199254740991}'::jsonb),   -- 2^53 - 1
    ('{"num": 9007199254740992}'::jsonb),   -- 2^53
    ('{"num": 9007199254740993}'::jsonb),   -- 2^53 + 1
    ('{"num": 9007199254740994}'::jsonb),   -- 2^53 + 2
    ('{"num": 9007199254740995}'::jsonb),   -- 2^53 + 3

    -- Group 6: Edge case test values for range queries
    ('{"num": 50}'::jsonb),
    ('{"num": 99}'::jsonb),
    ('{"num": 99.5}'::jsonb),
    ('{"num": 100.5}'::jsonb),
    ('{"num": 101}'::jsonb),
    ('{"num": 150}'::jsonb),
    ('{"num": 200}'::jsonb);

CREATE INDEX json_numeric_idx ON json_numeric_types
    USING bm25 (id, data)
    WITH (key_field = 'id');

-- ============================================================================
-- SECTION A: EQUALITY (=) OPERATOR
-- Tests single-value queries with multi-type expansion
-- ============================================================================

-- Test A1: Query value that exists ONLY in I64 (negative integer)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, data->>'num' as value
FROM json_numeric_types
WHERE id @@@ paradedb.all()
AND (data->>'num')::numeric = -42
ORDER BY id;

SELECT id, data->>'num' as value
FROM json_numeric_types
WHERE id @@@ paradedb.all()
AND (data->>'num')::numeric = -42
ORDER BY id;

-- Test A2: Query value that exists ONLY in U64 (beyond i64::MAX)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, data->>'num' as value
FROM json_numeric_types
WHERE id @@@ paradedb.all()
AND (data->>'num')::numeric = 9223372036854775808
ORDER BY id;

SELECT id, data->>'num' as value
FROM json_numeric_types
WHERE id @@@ paradedb.all()
AND (data->>'num')::numeric = 9223372036854775808
ORDER BY id;

-- Test A3: Query value that exists ONLY in F64 (decimal)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, data->>'num' as value
FROM json_numeric_types
WHERE id @@@ paradedb.all()
AND (data->>'num')::numeric = 3.14159
ORDER BY id;

SELECT id, data->>'num' as value
FROM json_numeric_types
WHERE id @@@ paradedb.all()
AND (data->>'num')::numeric = 3.14159
ORDER BY id;

-- Test A4: Cross-type matching - query 100 should match BOTH 100 and 100.0
-- Demonstrates multi-type expansion working correctly
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, data->>'num' as value, jsonb_typeof(data->'num') as json_type
FROM json_numeric_types
WHERE id @@@ paradedb.all()
AND (data->>'num')::numeric = 100
ORDER BY id;

SELECT id, data->>'num' as value, jsonb_typeof(data->'num') as json_type
FROM json_numeric_types
WHERE id @@@ paradedb.all()
AND (data->>'num')::numeric = 100
ORDER BY id;

-- Test A5: Cross-type matching - query 999.0 should match BOTH 999 and 999.0
SELECT id, data->>'num' as value, jsonb_typeof(data->'num') as json_type
FROM json_numeric_types
WHERE id @@@ paradedb.all()
AND (data->>'num')::numeric = 999.0
ORDER BY id;

-- Test A6: Query i64::MAX boundary
SELECT id, data->>'num' as value
FROM json_numeric_types
WHERE id @@@ paradedb.all()
AND (data->>'num')::numeric = 9223372036854775807
ORDER BY id;

-- Test A7: Query u64::MAX boundary
SELECT id, data->>'num' as value
FROM json_numeric_types
WHERE id @@@ paradedb.all()
AND (data->>'num')::numeric = 18446744073709551615
ORDER BY id;

-- Test A8: Query zero (should work across types)
SELECT id, data->>'num' as value
FROM json_numeric_types
WHERE id @@@ paradedb.all()
AND (data->>'num')::numeric = 0
ORDER BY id;

-- ============================================================================
-- SECTION B: GREATER THAN (>) OPERATOR
-- Tests range queries with multi-type expansion
-- ============================================================================

-- Test B1: Greater than with I64 value
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, data->>'num' as value
FROM json_numeric_types
WHERE id @@@ paradedb.all()
AND (data->>'num')::numeric > 1000
ORDER BY id;

SELECT id, data->>'num' as value
FROM json_numeric_types
WHERE id @@@ paradedb.all()
AND (data->>'num')::numeric > 1000
ORDER BY id;

-- Test B2: Greater than with U64 boundary (should find u64::MAX - 1 and u64::MAX)
SELECT id, data->>'num' as value
FROM json_numeric_types
WHERE id @@@ paradedb.all()
AND (data->>'num')::numeric > 18446744073709551613
ORDER BY id;

-- Test B3: Greater than with F64 value
SELECT id, data->>'num' as value
FROM json_numeric_types
WHERE id @@@ paradedb.all()
AND (data->>'num')::numeric > 3.0
ORDER BY id;

-- Test B4: Greater than negative (tests I64 negative range)
SELECT id, data->>'num' as value
FROM json_numeric_types
WHERE id @@@ paradedb.all()
AND (data->>'num')::numeric > -100
ORDER BY id;

-- ============================================================================
-- SECTION C: LESS THAN (<) OPERATOR
-- ============================================================================

-- Test C1: Less than with positive I64 value
SELECT id, data->>'num' as value
FROM json_numeric_types
WHERE id @@@ paradedb.all()
AND (data->>'num')::numeric < 10
ORDER BY id;

-- Test C2: Less than i64::MAX boundary (should exclude U64 values)
SELECT id, data->>'num' as value
FROM json_numeric_types
WHERE id @@@ paradedb.all()
AND (data->>'num')::numeric < 9223372036854775808
ORDER BY id;

-- Test C3: Less than with F64 value
SELECT id, data->>'num' as value
FROM json_numeric_types
WHERE id @@@ paradedb.all()
AND (data->>'num')::numeric < 1.0
ORDER BY id;

-- Test C4: Less than negative (tests I64 negative range)
SELECT id, data->>'num' as value
FROM json_numeric_types
WHERE id @@@ paradedb.all()
AND (data->>'num')::numeric < -1000
ORDER BY id;

-- ============================================================================
-- SECTION D: GREATER THAN OR EQUAL (>=) OPERATOR
-- ============================================================================

-- Test D1: Greater than or equal with U64 boundary
SELECT id, data->>'num' as value
FROM json_numeric_types
WHERE id @@@ paradedb.all()
AND (data->>'num')::numeric >= 18446744073709551614
ORDER BY id;

-- Test D2: Greater than or equal with I64 value
SELECT id, data->>'num' as value
FROM json_numeric_types
WHERE id @@@ paradedb.all()
AND (data->>'num')::numeric >= 1000
ORDER BY id;

-- Test D3: Greater than or equal to zero
SELECT id, data->>'num' as value
FROM json_numeric_types
WHERE id @@@ paradedb.all()
AND (data->>'num')::numeric >= 0
ORDER BY id;

-- ============================================================================
-- SECTION E: LESS THAN OR EQUAL (<=) OPERATOR
-- ============================================================================

-- Test E1: Less than or equal with small I64 value
SELECT id, data->>'num' as value
FROM json_numeric_types
WHERE id @@@ paradedb.all()
AND (data->>'num')::numeric <= 1
ORDER BY id;

-- Test E2: Less than or equal to i64::MAX + 1 (first U64 value)
SELECT id, data->>'num' as value
FROM json_numeric_types
WHERE id @@@ paradedb.all()
AND (data->>'num')::numeric <= 9223372036854775808
ORDER BY id;

-- Test E3: Less than or equal to zero
SELECT id, data->>'num' as value
FROM json_numeric_types
WHERE id @@@ paradedb.all()
AND (data->>'num')::numeric <= 0
ORDER BY id;

-- ============================================================================
-- SECTION F: BETWEEN OPERATOR
-- Tests range queries with lower and upper bounds
-- ============================================================================

-- Test F1: BETWEEN with I64 range
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, data->>'num' as value
FROM json_numeric_types
WHERE id @@@ paradedb.all()
AND (data->>'num')::numeric BETWEEN 0 AND 100
ORDER BY id;

SELECT id, data->>'num' as value
FROM json_numeric_types
WHERE id @@@ paradedb.all()
AND (data->>'num')::numeric BETWEEN 0 AND 100
ORDER BY id;

-- Test F2: BETWEEN crossing type boundaries (I64 to U64)
SELECT id, data->>'num' as value
FROM json_numeric_types
WHERE id @@@ paradedb.all()
AND (data->>'num')::numeric BETWEEN 9223372036854775807 AND 9223372036854775808
ORDER BY id;

-- Test F3: BETWEEN with U64 range
SELECT id, data->>'num' as value
FROM json_numeric_types
WHERE id @@@ paradedb.all()
AND (data->>'num')::numeric BETWEEN 18446744073709551613 AND 18446744073709551615
ORDER BY id;

-- Test F4: BETWEEN with negative range (I64)
SELECT id, data->>'num' as value
FROM json_numeric_types
WHERE id @@@ paradedb.all()
AND (data->>'num')::numeric BETWEEN -1000 AND -1
ORDER BY id;

-- Test F5: BETWEEN crossing zero
SELECT id, data->>'num' as value
FROM json_numeric_types
WHERE id @@@ paradedb.all()
AND (data->>'num')::numeric BETWEEN -10 AND 10
ORDER BY id;

-- Test F6: BETWEEN around cross-type values (should match both 100 and 100.0)
SELECT id, data->>'num' as value, jsonb_typeof(data->'num') as json_type
FROM json_numeric_types
WHERE id @@@ paradedb.all()
AND (data->>'num')::numeric BETWEEN 99 AND 101
ORDER BY id;

-- ============================================================================
-- SECTION G: IN OPERATOR
-- Tests term set queries with multi-type expansion
-- ============================================================================

-- Test G1: IN with ONLY I64 values
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, data->>'num' as value
FROM json_numeric_types
WHERE id @@@ paradedb.all()
AND (data->>'num')::numeric IN (-42, -1, 42)
ORDER BY id;

SELECT id, data->>'num' as value
FROM json_numeric_types
WHERE id @@@ paradedb.all()
AND (data->>'num')::numeric IN (-42, -1, 42)
ORDER BY id;

-- Test G2: IN with ONLY U64 values
SELECT id, data->>'num' as value
FROM json_numeric_types
WHERE id @@@ paradedb.all()
AND (data->>'num')::numeric IN (9223372036854775808, 18446744073709551615)
ORDER BY id;

-- Test G3: IN with ONLY F64 values
SELECT id, data->>'num' as value
FROM json_numeric_types
WHERE id @@@ paradedb.all()
AND (data->>'num')::numeric IN (0.5, 3.14159, 42.5)
ORDER BY id;

-- Test G4: IN with mixed types (I64, U64, F64)
SELECT id, data->>'num' as value
FROM json_numeric_types
WHERE id @@@ paradedb.all()
AND (data->>'num')::numeric IN (
    -42,                      -- I64
    42.5,                     -- F64
    9223372036854775808,      -- U64
    18446744073709551615      -- U64
)
ORDER BY id;

-- Test G5: IN with cross-type values (should match both int and float)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, data->>'num' as value, jsonb_typeof(data->'num') as json_type
FROM json_numeric_types
WHERE id @@@ paradedb.all()
AND (data->>'num')::numeric IN (100, 999)
ORDER BY id;

SELECT id, data->>'num' as value, jsonb_typeof(data->'num') as json_type
FROM json_numeric_types
WHERE id @@@ paradedb.all()
AND (data->>'num')::numeric IN (100, 999)
ORDER BY id;

-- Test G6: IN with boundary values
SELECT id, data->>'num' as value
FROM json_numeric_types
WHERE id @@@ paradedb.all()
AND (data->>'num')::numeric IN (
    -9223372036854775808,     -- i64::MIN
    9223372036854775807,      -- i64::MAX
    18446744073709551615      -- u64::MAX
)
ORDER BY id;

-- ============================================================================
-- SECTION H: NOT IN OPERATOR
-- Tests exclusion with multi-type expansion
-- ============================================================================

-- Test H1: NOT IN with cross-type values (should exclude both 100 and 100.0)
SELECT COUNT(*) as count_not_in_list
FROM json_numeric_types
WHERE id @@@ paradedb.all()
AND (data->>'num')::numeric NOT IN (100, 999);

-- Test H2: NOT IN with specific values
SELECT id, data->>'num' as value
FROM json_numeric_types
WHERE id @@@ paradedb.all()
AND (data->>'num')::numeric NOT IN (0, -1, 1, 42)
ORDER BY id;

-- ============================================================================
-- SECTION I: BOUNDARY AND EDGE CASES
-- ============================================================================

-- Test I1: F64 precision boundary (2^53 - 1)
SELECT id, data->>'num' as value
FROM json_numeric_types
WHERE id @@@ paradedb.all()
AND (data->>'num')::numeric = 9007199254740991
ORDER BY id;

-- Test I2: Beyond F64 safe precision (2^53 + 1)
SELECT id, data->>'num' as value
FROM json_numeric_types
WHERE id @@@ paradedb.all()
AND (data->>'num')::numeric = 9007199254740993
ORDER BY id;

-- Test I3: i64::MIN boundary
SELECT id, data->>'num' as value
FROM json_numeric_types
WHERE id @@@ paradedb.all()
AND (data->>'num')::numeric = -9223372036854775808
ORDER BY id;

-- Test I4: Range query around 2^53 boundary
SELECT id, data->>'num' as value
FROM json_numeric_types
WHERE id @@@ paradedb.all()
AND (data->>'num')::numeric BETWEEN 9007199254740991 AND 9007199254740993
ORDER BY id;

-- ============================================================================
-- SECTION J: EDGE CASES AND ERROR HANDLING
-- ============================================================================

-- Test J1: Empty range (inverted bounds) - should return 0 rows
-- Tests is_empty_range() function
SELECT COUNT(*) as count_should_be_zero
FROM json_numeric_types
WHERE id @@@ paradedb.all()
AND (data->>'num')::numeric BETWEEN 200 AND 100;

-- Test J2: Boundary-exact range (100 to 100) - should match 100 and 100.0
-- Tests Bound::Included handling
SELECT id, data->>'num' as value
FROM json_numeric_types
WHERE id @@@ paradedb.all()
AND (data->>'num')::numeric BETWEEN 100 AND 100
ORDER BY id;

-- Test J3: Excluded bounds test (> 99 AND < 101)
-- Tests Bound::Excluded handling
SELECT id, data->>'num' as value
FROM json_numeric_types
WHERE id @@@ paradedb.all()
AND (data->>'num')::numeric > 99
AND (data->>'num')::numeric < 101
ORDER BY id;

-- Test J4: Single-type optimization (U64 only beyond 2^53)
-- Tests single RangeQuery optimization path
SELECT id, data->>'num' as value
FROM json_numeric_types
WHERE id @@@ paradedb.all()
AND (data->>'num')::numeric BETWEEN 9007199254740993 AND 18446744073709551615
ORDER BY id;

-- Test J5: F64-only range (decimals only)
-- Tests single F64 RangeQuery when no integers match
SELECT id, data->>'num' as value
FROM json_numeric_types
WHERE id @@@ paradedb.all()
AND (data->>'num')::numeric BETWEEN 99.5 AND 100.5
ORDER BY id;

-- Test J6: Range at exact 2^53 boundary
-- Tests precision boundary F64_SAFE_INTEGER_MAX
SELECT id, data->>'num' as value
FROM json_numeric_types
WHERE id @@@ paradedb.all()
AND (data->>'num')::numeric BETWEEN 9007199254740991 AND 9007199254740992
ORDER BY id;

-- Test J7: Range crossing 2^53 boundary
-- Tests type transitions at precision limit
SELECT id, data->>'num' as value
FROM json_numeric_types
WHERE id @@@ paradedb.all()
AND (data->>'num')::numeric BETWEEN 9007199254740992 AND 9007199254740994
ORDER BY id;

-- Test J8: Negative empty range (inverted)
SELECT COUNT(*) as count_should_be_zero
FROM json_numeric_types
WHERE id @@@ paradedb.all()
AND (data->>'num')::numeric BETWEEN 100 AND 50;

-- Test J9: Narrow range (100 to 101)
-- Tests multi-type matching in narrow window
SELECT id, data->>'num' as value
FROM json_numeric_types
WHERE id @@@ paradedb.all()
AND (data->>'num')::numeric BETWEEN 100 AND 101
ORDER BY id;

-- Test J10: Excluded upper bound at exact match
-- Tests (>= 100 AND < 100) - should be empty
SELECT COUNT(*) as count_should_be_zero
FROM json_numeric_types
WHERE id @@@ paradedb.all()
AND (data->>'num')::numeric >= 100
AND (data->>'num')::numeric < 100;

-- ============================================================================
-- Cleanup
-- ============================================================================
DROP TABLE json_numeric_types;

-- ============================================================================
-- PART 3: JSON Fast Field Numeric Range Query Tests
-- ============================================================================
-- Tests JSON numeric range queries on fast fields with both NUMERIC and FLOAT8
-- pushdown. Both produce the same behavior because fast fields store values
-- as F64 when mixed int/float data exists.
-- ============================================================================

CREATE TABLE json_fast_field_test (
    id SERIAL PRIMARY KEY,
    data JSONB
);

-- Insert test data with mixed integers and floats
-- The presence of float values causes the fast field column to store ALL values as F64
INSERT INTO json_fast_field_test (data) VALUES
    ('{"value": 100}'),
    ('{"value": 200}'),
    ('{"value": 500}'),
    ('{"value": 1000}'),
    ('{"value": 1.5}'),                 -- Float triggers F64 storage
    ('{"value": 99.9}'),
    ('{"value": 9007199254740991}'),    -- 2^53 - 1 (safe in F64)
    ('{"value": 9007199254740992}'),    -- 2^53 (exact in F64)
    ('{"value": 9007199254740993}'),    -- 2^53 + 1 (precision loss in F64)
    ('{"value": 9007199254740994}');    -- 2^53 + 2

CREATE INDEX json_fast_idx ON json_fast_field_test
USING bm25 (id, data)
WITH (key_field = 'id', text_fields = '{}', json_fields = '{"data": {"fast": true}}');

-- ============================================================================
-- SECTION K: Comparison of NUMERIC vs FLOAT8 pushdown on JSON fast fields
-- Both should produce IDENTICAL results since fast fields use F64 internally
-- ============================================================================

-- Test K1a: NUMERIC pushdown - basic range
SELECT id, data->>'value' as value
FROM json_fast_field_test
WHERE id @@@ paradedb.all()
AND (data->>'value')::numeric >= 100 AND (data->>'value')::numeric <= 500
ORDER BY id;

-- Test K1b: FLOAT8 pushdown - same range (should match K1a exactly)
SELECT id, data->>'value' as value
FROM json_fast_field_test
WHERE id @@@ paradedb.all()
AND (data->>'value')::float8 >= 100 AND (data->>'value')::float8 <= 500
ORDER BY id;

-- Test K2a: NUMERIC pushdown - 2^53 boundary precision test
SELECT id, data->>'value' as value
FROM json_fast_field_test
WHERE id @@@ paradedb.all()
AND (data->>'value')::numeric >= 9007199254740993 AND (data->>'value')::numeric < 9007199254740994
ORDER BY id;

-- Test K2b: FLOAT8 pushdown - same 2^53 boundary (should match K2a exactly)
SELECT id, data->>'value' as value
FROM json_fast_field_test
WHERE id @@@ paradedb.all()
AND (data->>'value')::float8 >= 9007199254740993 AND (data->>'value')::float8 < 9007199254740994
ORDER BY id;

-- ============================================================================
-- SECTION L: Complete range query tests on JSON fast fields
-- ============================================================================

-- Test L1: Range including both integer and float values
SELECT id, data->>'value' as value
FROM json_fast_field_test
WHERE id @@@ paradedb.all()
AND (data->>'value')::numeric >= 1 AND (data->>'value')::numeric <= 200
ORDER BY id;

-- Test L2: Greater than comparison
SELECT id, data->>'value' as value
FROM json_fast_field_test
WHERE id @@@ paradedb.all()
AND (data->>'value')::numeric > 500
ORDER BY id;

-- Test L3: Less than comparison
SELECT id, data->>'value' as value
FROM json_fast_field_test
WHERE id @@@ paradedb.all()
AND (data->>'value')::numeric < 200
ORDER BY id;

-- Test L4: Range at 2^53 boundary (all values in this range)
SELECT id, data->>'value' as value
FROM json_fast_field_test
WHERE id @@@ paradedb.all()
AND (data->>'value')::numeric >= 9007199254740991 AND (data->>'value')::numeric <= 9007199254740994
ORDER BY id;

-- Test L5: Exact match at 2^53 (uses F64 comparison)
SELECT id, data->>'value' as value
FROM json_fast_field_test
WHERE id @@@ paradedb.all()
AND (data->>'value')::numeric = 9007199254740992
ORDER BY id;

-- Test L6: BETWEEN syntax
SELECT id, data->>'value' as value
FROM json_fast_field_test
WHERE id @@@ paradedb.all()
AND (data->>'value')::numeric BETWEEN 100 AND 1000
ORDER BY id;

-- ============================================================================
-- Cleanup
-- ============================================================================
DROP TABLE json_fast_field_test;
