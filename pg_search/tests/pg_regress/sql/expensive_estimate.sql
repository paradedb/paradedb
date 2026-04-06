-- Tests that queries with expensive scorer construction (fuzzy, regex)
-- use heuristic selectivity and still produce correct results.
-- Range queries use accurate scorer-based estimation (cheap on fast fields).

\i common/common_setup.sql

CALL paradedb.create_bm25_test_table(
        schema_name => 'public',
        table_name => 'mock_items'
);

CREATE INDEX idx_mock_items
ON mock_items
    USING bm25 (id, description, rating, category, in_stock, created_at, weight_range)
WITH (key_field='id');

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
-- Cleanup
-- ============================================================================

DROP TABLE mock_items CASCADE;
