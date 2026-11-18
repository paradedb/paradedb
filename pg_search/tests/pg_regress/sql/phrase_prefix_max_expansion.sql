-- Negative expansion limits must not become huge unsigned limits.
SELECT pdb.phrase_prefix(ARRAY['running', 'sh'])::text;
SELECT pdb.phrase_prefix(ARRAY['running', 'sh'], max_expansion => NULL)::text;
SELECT pdb.phrase_prefix(ARRAY['running', 'sh'], max_expansion => 0)::text;
SELECT pdb.phrase_prefix(ARRAY['running', 'sh'], max_expansion => 1)::text;
SELECT pdb.phrase_prefix(ARRAY['running', 'sh'], max_expansion => 2147483647)::text;

SELECT pdb.phrase_prefix(ARRAY['running', 'sh'], max_expansion => -1)::text;
\echo :SQLSTATE
SELECT pdb.phrase_prefix(ARRAY['running', 'sh'], max_expansion => -2147483648)::text;
\echo :SQLSTATE

-- A valid call still works after the errors.
SELECT pdb.phrase_prefix(ARRAY['running', 'sh'], max_expansion => 1)::text;
