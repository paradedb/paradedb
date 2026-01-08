-- Test to verify the return type of pdb.snippet_positions
-- Note: PostgreSQL doesn't differentiate between integer[] and integer[][] at the type level.
-- Both 1D and 2D integer arrays have the same type signature (integer[] / _int4).
-- The dimensionality is stored in the array metadata, not the type system.

-- Show the function signature
\df pdb.snippet_positions

-- Show the actual return type in pg_catalog
SELECT 
    p.proname as function_name,
    pg_catalog.format_type(p.prorettype, NULL) as return_type,
    p.prorettype::regtype as return_type_regtype,
    t.typname as type_name,
    t.typtype as type_type,
    t.typlen as type_len,
    CASE 
        WHEN t.typelem != 0 THEN pg_catalog.format_type(t.typelem, NULL)
        ELSE NULL
    END as element_type
FROM pg_catalog.pg_proc p
JOIN pg_catalog.pg_namespace n ON p.pronamespace = n.oid
JOIN pg_catalog.pg_type t ON p.prorettype = t.oid
WHERE n.nspname = 'pdb' 
  AND p.proname = 'snippet_positions';

-- Create a test table and index
CREATE TABLE snippet_type_test (
    id SERIAL PRIMARY KEY,
    content TEXT
);

INSERT INTO snippet_type_test (content) VALUES 
    ('This is a test document for snippet positions'),
    ('Another test with multiple test occurrences');

CREATE INDEX snippet_type_test_idx ON snippet_type_test
USING bm25 (id, content)
WITH (key_field = 'id');

-- Test the actual usage and output
SELECT 
    id, 
    pdb.snippet_positions(content) as positions,
    pg_typeof(pdb.snippet_positions(content)) as actual_type
FROM snippet_type_test 
WHERE content @@@ 'test'
ORDER BY id;

-- Test that we can treat it as a 2D array
-- array_length(arr, 1) returns the number of rows (first dimension)
-- array_length(arr, 2) returns the number of columns (second dimension = 2 for [start, end])
SELECT 
    id,
    pdb.snippet_positions(content) as positions,
    array_length(pdb.snippet_positions(content), 1) as num_positions,
    array_length(pdb.snippet_positions(content), 2) as inner_dimension
FROM snippet_type_test 
WHERE content @@@ 'test'
ORDER BY id;

-- Cleanup
DROP TABLE snippet_type_test CASCADE;

