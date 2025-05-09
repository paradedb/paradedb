-- Tests type conversion edge cases with mixed fast fields

\i common/mixedff_advanced_setup.sql

\echo 'Test: Type conversion edge cases'

-- Test 1: Basic text to text conversions
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT text_field, varchar_field, char_field
FROM type_conversion_test
WHERE text_field @@@ 'text' OR varchar_field @@@ 'varchar';

-- Test 2: Converting numeric string to number
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT text_field::numeric as converted_num, numeric_field
FROM type_conversion_test
WHERE text_field ~ '^[0-9.]+$';

-- Test 3: Numeric range filtering with casts
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT int_field, bigint_field, float_field
FROM type_conversion_test
WHERE int_field::float > 100 AND float_field::int < 12346;

-- Test 4: String concatenation with different types
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT text_field || ' - ' || int_field::text as text_with_num
FROM type_conversion_test
WHERE bool_field = true;

-- Test 5: Mixed type expressions in filtering
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT text_field, int_field, float_field
FROM type_conversion_test
WHERE (int_field::text = '100' OR text_field = '123') 
  AND float_field BETWEEN 2 AND 10000;

-- Test 6: Date conversions
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT date_field, timestamp_field
FROM type_conversion_test
WHERE date_field = timestamp_field::date;

-- Test 7: CASE expression with type conversion
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT 
    id,
    CASE 
        WHEN text_field ~ '^[0-9]+$' THEN text_field::integer * 2
        ELSE int_field
    END as converted_value
FROM type_conversion_test;

-- Test 8: JSON extraction with type conversion
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT 
    id,
    json_field,
    (json_field->>'number')::numeric AS extracted_number
FROM type_conversion_test
WHERE json_field ? 'number';

-- Test 9: Complex mixed type filtering
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT text_field, int_field, bool_field
FROM type_conversion_test
WHERE 
    CASE 
        WHEN bool_field THEN int_field > 50
        ELSE text_field @@@ 'text'
    END;

-- Verify actual conversion results
SELECT 
    id,
    text_field,
    text_field::numeric as text_to_num,
    int_field,
    int_field::text as int_to_text,
    bool_field,
    CASE WHEN bool_field THEN 'Yes' ELSE 'No' END as bool_to_text
FROM type_conversion_test
WHERE text_field ~ '^[0-9.]+$' OR int_field > 1000;

-- Test character set conversion issues
SELECT 
    id,
    text_field,
    varchar_field,
    char_field,
    TRIM(char_field) as trimmed_char,
    LENGTH(char_field) as char_length,
    LENGTH(TRIM(char_field)) as trimmed_length
FROM type_conversion_test
WHERE char_field <> '';

\i common/mixedff_advanced_cleanup.sql 
