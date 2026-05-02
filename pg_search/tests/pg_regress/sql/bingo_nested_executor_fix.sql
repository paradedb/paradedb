-- Regression test for issue #4843
-- https://github.com/paradedb/paradedb/issues/4843
--
-- Verifies that a multi-row INSERT into a table with a BM25 expression index
-- that calls a bingo function does not panic with:
--   "entered unreachable code: get_insert_state"
--
-- Root cause: bingo.cansmiles evaluates via internal C++ APIs that call
-- ExecutorRun() directly (not via SPI), triggering a nested ExecutorRun_hook
-- while the outer stack slot is still live. The depth-counter fix in
-- fake_aminsertcleanup.rs absorbs the nested call instead of pushing a new
-- None that shadows the outer Some.
--
-- Requirements: pg_search + bingo 1.40.0+ installed on PG16.
-- Bingo is installed via bingo_install.sql (not CREATE EXTENSION).

CREATE TABLE bingo_nested_exec_test (
    id      SERIAL PRIMARY KEY,
    smiles  TEXT NOT NULL
);

CREATE INDEX bingo_nested_exec_test_bm25_idx
    ON bingo_nested_exec_test
    USING bm25 (id, (bingo.cansmiles(smiles)::pdb.simple))
    WITH (key_field = 'id');

-- Before the fix this panicked. After the fix: 5 rows insert cleanly.
INSERT INTO bingo_nested_exec_test (smiles)
SELECT repeat('C', n) FROM generate_series(1, 5) AS n;

SELECT COUNT(*) AS row_count FROM bingo_nested_exec_test;
-- Expected: 5

DROP TABLE bingo_nested_exec_test;