-- Tests type conversion edge cases with mixed fast fields

\i common/mixedff_advanced_setup.sql

\echo 'Test: Type conversion edge cases'

-- Test implicit type conversions
EXPLAIN (FORMAT TEXT, COSTS OFF)
SELECT id, smallint_field, integer_field, bigint_field
FROM conversion_test
WHERE content @@@ 'conversion test';

-- Check values for different integer types
SELECT id, smallint_field, integer_field, bigint_field
FROM conversion_test
WHERE content @@@ 'conversion test'
ORDER BY id;

-- Check floating point type conversions
SELECT id, real_field, double_field
FROM conversion_test
WHERE content @@@ 'conversion test'
ORDER BY id;

-- Check boolean conversions
SELECT id, bool_from_int
FROM conversion_test
WHERE content @@@ 'conversion test'
ORDER BY id;

-- Check timestamp field conversions
SELECT id, timestamp_field
FROM conversion_test
WHERE content @@@ 'conversion test'
ORDER BY id;

-- Test type coercion in operations
SELECT 
    id,
    smallint_field + integer_field as addIntegers,
    smallint_field::float / NULLIF(integer_field, 0) as divideIntegers
FROM conversion_test
WHERE content @@@ 'conversion test'
ORDER BY id;

-- Test numeric string conversion
SELECT 
    id,
    string_field1,
    CASE 
        WHEN string_field1 ~ '^[0-9]+$' THEN string_field1::integer * 2
        ELSE numeric_field1
    END as converted_value
FROM mixed_numeric_string_test
WHERE string_field1 @@@ 'Unique'
ORDER BY id;

-- Test string concatenation with numbers
SELECT 
    id,
    string_field1 || ' - ' || numeric_field1::text as text_with_num
FROM mixed_numeric_string_test
WHERE numeric_field1 > 0 AND string_field1 @@@ 'Apple'
ORDER BY id;

-- Test date conversions with text output
SELECT 
    id,
    timestamp_field,
    timestamp_field::date as just_date,
    timestamp_field::time as just_time,
    to_char(timestamp_field, 'YYYY-MM-DD') as formatted_date
FROM conversion_test
WHERE content @@@ 'conversion test'
ORDER BY timestamp_field
LIMIT 2;

-- Test complex type conversion in a CASE expression
SELECT 
    id,
    CASE
        WHEN numeric_field1 > 300 THEN 'High Value'
        WHEN numeric_field1 > 100 THEN 'Medium Value'
        ELSE 'Low Value'
    END as numeric_category,
    string_field1
FROM mixed_numeric_string_test
WHERE content @@@ 'is'
ORDER BY numeric_field1
LIMIT 3;

\i common/mixedff_advanced_cleanup.sql 
