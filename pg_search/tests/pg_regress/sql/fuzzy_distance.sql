-- Out-of-range distances must be rejected before they can wrap to a u8.
SELECT pdb.fuzzy_term('shoe', distance => -1)::text;
\echo :SQLSTATE
SELECT pdb.fuzzy_term('shoe', distance => 256)::text;
\echo :SQLSTATE
SELECT pdb.match('shoe', distance => -1)::text;
\echo :SQLSTATE
SELECT pdb.match('shoe', distance => 256)::text;
\echo :SQLSTATE

-- The builder accepts representable distances and leaves the search limit to Tantivy.
\pset format unaligned
SELECT pdb.fuzzy_term('shoe', distance => 3)::text::jsonb #>> '{fuzzy_term,distance}' AS distance;
SELECT pdb.match('shoe', distance => 3)::text::jsonb #>> '{match,distance}' AS distance;
