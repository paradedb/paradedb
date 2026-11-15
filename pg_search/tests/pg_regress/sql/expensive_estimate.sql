-- Tests that queries with expensive scorer construction (fuzzy, regex)
-- use heuristic selectivity and still produce correct results.
-- Range queries use accurate scorer-based estimation (cheap on fast fields).

\i common/common_setup.sql

CALL paradedb.create_paradedb_test_table(
        schema_name => 'public',
        table_name => 'mock_items'
);

-- These tests exercise BM25 estimation, not vector search. Drop the embedding column
-- and rewrite the heap so it stays out of the row-width and cost estimates.
ALTER TABLE mock_items DROP COLUMN embedding;
VACUUM FULL mock_items;

CREATE INDEX idx_mock_items
ON mock_items
    USING paradedb (id, description, rating, category, in_stock, created_at, weight_range);

ANALYZE mock_items;

-- ============================================================================
-- Fuzzy term query
-- ============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF) SELECT * FROM mock_items WHERE description @@@ paradedb.fuzzy_term(
    field => 'description',
    value => 'sheos',
    distance => 1::integer
);
SELECT id, description, rating, category FROM mock_items WHERE description @@@ paradedb.fuzzy_term(
    field => 'description',
    value => 'sheos',
    distance => 1::integer
) ORDER BY id;

-- Fuzzy term with higher distance
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF) SELECT * FROM mock_items WHERE description @@@ paradedb.fuzzy_term(
    field => 'description',
    value => 'sheos',
    distance => 2::integer
);
SELECT id, description, rating, category FROM mock_items WHERE description @@@ paradedb.fuzzy_term(
    field => 'description',
    value => 'sheos',
    distance => 2::integer
) ORDER BY id;

-- ============================================================================
-- Regex query
-- ============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF) SELECT * FROM mock_items WHERE description @@@ paradedb.regex(
    field => 'description',
    pattern => 'sh.*es'
);
SELECT id, description, rating, category FROM mock_items WHERE description @@@ paradedb.regex(
    field => 'description',
    pattern => 'sh.*es'
) ORDER BY id;

-- ============================================================================
-- Range query on numeric field
-- ============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF) SELECT * FROM mock_items WHERE id @@@ paradedb.range(
    field => 'rating',
    range => int4range(3, 5)
);
SELECT id, description, rating, category FROM mock_items WHERE id @@@ paradedb.range(
    field => 'rating',
    range => int4range(3, 5)
) ORDER BY id;

-- ============================================================================
-- Range query on date field
-- ============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF) SELECT * FROM mock_items WHERE id @@@ paradedb.range(
    field => 'created_at',
    range => tsrange('2023-04-01', '2023-06-01')
);
SELECT id, description, rating, category FROM mock_items WHERE id @@@ paradedb.range(
    field => 'created_at',
    range => tsrange('2023-04-01', '2023-06-01')
) ORDER BY id;

-- ============================================================================
-- Boolean query mixing cheap and expensive sub-queries
-- ============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF) SELECT * FROM mock_items WHERE description @@@ paradedb.boolean(
    must := ARRAY[
        paradedb.term('description', 'shoes'),
        paradedb.fuzzy_term(field => 'description', value => 'runing', distance => 1::integer)
    ]
);
SELECT id, description, rating, category FROM mock_items WHERE description @@@ paradedb.boolean(
    must := ARRAY[
        paradedb.term('description', 'shoes'),
        paradedb.fuzzy_term(field => 'description', value => 'runing', distance => 1::integer)
    ]
) ORDER BY id;

-- ============================================================================
-- Fuzzy match query
-- ============================================================================

CREATE TABLE expensive_estimate_titles (id int PRIMARY KEY, title text, lang text);
INSERT INTO expensive_estimate_titles
SELECT g, md5(g::text), CASE WHEN g % 2 = 0 THEN 'en' ELSE 'ja' END FROM generate_series(1, 10000) g;
INSERT INTO expensive_estimate_titles VALUES (10001, 'naruto', 'ja'), (10002, 'narutoo', 'ja'), (10003, 'naruta', 'ja');
CREATE INDEX expensive_estimate_titles_idx ON expensive_estimate_titles
    USING bm25 (id, (title::pdb.literal_normalized), lang);
ANALYZE expensive_estimate_titles;

CREATE FUNCTION expensive_estimate_plan_rows(q text) RETURNS text LANGUAGE plpgsql AS $$
DECLARE plan json;
BEGIN
    EXECUTE 'EXPLAIN (FORMAT JSON) ' || q INTO plan;
    RETURN plan -> 0 -> 'Plan' ->> 'Plan Rows';
END
$$;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF) SELECT * FROM expensive_estimate_titles WHERE title ||| 'naruto'::pdb.fuzzy(1);
SELECT * FROM expensive_estimate_titles WHERE title ||| 'naruto'::pdb.fuzzy(1) ORDER BY id;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF) SELECT * FROM expensive_estimate_titles WHERE title ||| ARRAY['naruto', 'narutoo']::pdb.fuzzy(1);
SELECT * FROM expensive_estimate_titles WHERE title ||| ARRAY['naruto', 'narutoo']::pdb.fuzzy(1) ORDER BY id;

-- A fuzzy match gets the same estimate as a fuzzy term, 1% of the rows at distance 1 and 5% at distance 2.
SELECT expensive_estimate_plan_rows($$SELECT id FROM expensive_estimate_titles WHERE title ||| 'naruto'::pdb.fuzzy(1)$$) AS match_op,
       expensive_estimate_plan_rows($$SELECT id FROM expensive_estimate_titles WHERE title @@@ pdb.match('naruto', distance => 1)$$) AS match_fn,
       expensive_estimate_plan_rows($$SELECT id FROM expensive_estimate_titles WHERE title ||| ARRAY['naruto', 'narutoo']::pdb.fuzzy(1)$$) AS match_array,
       expensive_estimate_plan_rows($$SELECT id FROM expensive_estimate_titles WHERE title === 'naruto'::pdb.fuzzy(1)$$) AS term_op;
SELECT expensive_estimate_plan_rows($$SELECT id FROM expensive_estimate_titles WHERE title ||| 'naruto'::pdb.fuzzy(2)$$) AS match_op,
       expensive_estimate_plan_rows($$SELECT id FROM expensive_estimate_titles WHERE title === 'naruto'::pdb.fuzzy(2)$$) AS term_op;

-- A match at distance 0 builds no automaton, so its estimate comes from the index.
SELECT expensive_estimate_plan_rows($$SELECT id FROM expensive_estimate_titles WHERE title ||| 'naruto'::pdb.fuzzy(0)$$) AS match_op;

-- The heuristic has no estimate for a negated fuzzy match, so the estimate comes from the index.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF) SELECT count(*) FROM expensive_estimate_titles WHERE lang === 'ja' AND NOT (title ||| 'naruto'::pdb.fuzzy(1));
SELECT count(*) FROM expensive_estimate_titles WHERE lang === 'ja' AND NOT (title ||| 'naruto'::pdb.fuzzy(1));
SELECT expensive_estimate_plan_rows($$SELECT id FROM expensive_estimate_titles WHERE lang === 'ja' AND NOT (title ||| 'naruto'::pdb.fuzzy(1))$$) AS negated_match,
       expensive_estimate_plan_rows($$SELECT id FROM expensive_estimate_titles WHERE lang === 'ja' AND NOT (title === 'naruto'::pdb.fuzzy(1))$$) AS negated_term;

-- With the heuristic off, every estimate comes from the index.
SET paradedb.enable_heuristic_selectivity = off;
SELECT expensive_estimate_plan_rows($$SELECT id FROM expensive_estimate_titles WHERE title ||| 'naruto'::pdb.fuzzy(1)$$) AS match_op,
       expensive_estimate_plan_rows($$SELECT id FROM expensive_estimate_titles WHERE title === 'naruto'::pdb.fuzzy(1)$$) AS term_op;
RESET paradedb.enable_heuristic_selectivity;

DROP FUNCTION expensive_estimate_plan_rows(text);
DROP TABLE expensive_estimate_titles;

-- ============================================================================
-- Cleanup
-- ============================================================================

DROP TABLE mock_items CASCADE;
