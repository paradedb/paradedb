\i common/common_setup.sql

-- Test that directly selecting pdb.alias values doesn't crash
-- This was a segfault bug in version 0.20.2 where casting non-text types
-- to pdb.alias would crash when PostgreSQL tried to display the result.

-- Test integer to alias (this used to segfault)
SELECT (1)::pdb.alias('test');

-- Test with different integer values
SELECT (42)::pdb.alias('answer');
SELECT (-100)::pdb.alias('negative');

-- Test bigint to alias
SELECT (9223372036854775807::bigint)::pdb.alias('bigint_max');

-- Test smallint to alias
SELECT (32767::smallint)::pdb.alias('smallint_max');

-- Test real/float to alias
SELECT (3.14::real)::pdb.alias('pi');
SELECT (2.71828::double precision)::pdb.alias('e');

-- Test numeric to alias
SELECT (123.456::numeric)::pdb.alias('numeric_val');

-- Test boolean to alias
SELECT true::pdb.alias('bool_true');
SELECT false::pdb.alias('bool_false');

-- Test date/time types to alias
SELECT '2024-01-15'::date::pdb.alias('date_val');
SELECT '12:30:45'::time::pdb.alias('time_val');
SELECT '2024-01-15 12:30:45'::timestamp::pdb.alias('timestamp_val');

-- Test text to alias
SELECT 'hello'::pdb.alias('text_val');
SELECT 'world'::text::pdb.alias('text_val');

-- EDGE CASE: Test text with exactly 12 characters (size of AliasDatumWithType data portion)
-- This is critical because our wrapper detection checks if vl_len == 20 bytes (with magic field).
-- A 16-char text has vl_len = 20 (4 byte header + 16 bytes data), matching our wrapper size!
-- We must distinguish between a wrapped datum and a text that happens to be this size.
SELECT 'exactly16chars!!'::pdb.alias('edge_case_size');

-- EDGE CASE: Try to create text that matches our magic number
-- Magic is 0x414C0053 = 'A', 'L', 0x00, 'S' (includes null byte at position 2)
-- PostgreSQL text with UTF8 encoding CANNOT contain embedded null bytes
-- Attempting E'AL\000S' would fail with: ERROR: invalid byte sequence for encoding "UTF8": 0x00
-- This makes our magic number with embedded null impossible to accidentally match!
-- This test verifies that text starting with "AL" works correctly (doesn't match magic)
SELECT 'ALSO_16_CHARS!!!'::pdb.alias('edge_case_magic_like');

-- Further verification: Even if we could somehow create matching bytes, the size would be wrong
-- Our wrapper is now 20 bytes (vl_len=4, magic=4, typoid=4, datum=8)
-- Text "AL" would be only 6 bytes (vl_len=4, data=2), nowhere near collision range
SELECT 'AL'::pdb.alias('short_al_prefix');

-- Test in expressions
SELECT (1 + 2)::pdb.alias('sum');
SELECT (10 * 5)::pdb.alias('product');

-- Test NULL handling
SELECT NULL::integer::pdb.alias('null_int');

-- Test in WHERE clause (should not crash - output function not called)
CREATE TABLE test_where (id serial, val int);
INSERT INTO test_where (val) VALUES (1), (2), (3);

-- This uses the alias in WHERE but doesn't display it
SELECT id FROM test_where WHERE val::pdb.alias('v') IS NOT NULL;

-- This uses a literal alias in WHERE
SELECT id FROM test_where WHERE (1)::pdb.alias('derp') IS NOT NULL;

-- Cleanup
DROP TABLE test_where;
