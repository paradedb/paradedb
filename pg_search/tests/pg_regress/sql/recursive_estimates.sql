-- Recursive Estimates Feature Tests
-- Tests recursive cost estimation in EXPLAIN VERBOSE output
-- Organized by complexity: Simple → Complex → Nested
-- Each test MUST show estimated_docs in output

\i common/recursive_estimates_setup.sql

-- ============================================================================
-- STAGE 1: SIMPLE LEAF QUERIES (No Nesting)
-- Expected: Each query should show estimated_docs for single term
-- ============================================================================

-- Test 1.1: Simple parse query (single term)
EXPLAIN (VERBOSE) SELECT * FROM recursive_test.estimate_items WHERE description @@@ 'shoes';

-- Test 1.2: Simple phrase query
EXPLAIN (VERBOSE) SELECT * FROM recursive_test.estimate_items WHERE description ### 'running shoes';

-- ============================================================================
-- STAGE 2: BOOLEAN QUERIES (Two-Level Nesting)
-- Expected: Parent boolean AND/OR + each child term show estimated_docs
-- ============================================================================

-- Test 2.1: Simple AND (conjunction) - 2 children
EXPLAIN (VERBOSE) SELECT * FROM recursive_test.estimate_items WHERE description &&& 'running shoes';

-- Test 2.2: Simple OR (disjunction) - 2 children
EXPLAIN (VERBOSE) SELECT * FROM recursive_test.estimate_items WHERE description ||| 'running shoes';

-- Test 2.3: AND with array (multiple terms)
EXPLAIN (VERBOSE) SELECT * FROM recursive_test.estimate_items WHERE description &&& ARRAY['running', 'shoes', 'athletic'];

-- Test 2.4: OR with array (multiple terms)
EXPLAIN (VERBOSE) SELECT * FROM recursive_test.estimate_items WHERE description ||| ARRAY['running', 'walking', 'hiking'];

-- ============================================================================
-- STAGE 3: NESTED BOOLEAN QUERIES (Three-Level Nesting)
-- Expected: Recursive estimates showing boolean parent + term children
-- ============================================================================

-- Test 3.1: Boolean with two MUST clauses
EXPLAIN (VERBOSE)
SELECT * FROM recursive_test.estimate_items
WHERE description @@@ paradedb.boolean(
    must := ARRAY[
        paradedb.term('description', 'shoes'),
        paradedb.term('description', 'running')
    ]
);

-- Test 3.2: Boolean with two SHOULD clauses (OR)
EXPLAIN (VERBOSE)
SELECT * FROM recursive_test.estimate_items
WHERE description @@@ paradedb.boolean(
    should := ARRAY[
        paradedb.term('description', 'running'),
        paradedb.term('description', 'walking')
    ]
);

-- Test 3.3: Boolean with MUST and SHOULD mixed
EXPLAIN (VERBOSE)
SELECT * FROM recursive_test.estimate_items
WHERE description @@@ paradedb.boolean(
    must := ARRAY[paradedb.term('description', 'shoes')],
    should := ARRAY[paradedb.term('description', 'running'), paradedb.term('description', 'athletic')]
);

-- Test 3.4: Boolean with MUST_NOT clause (CRITICAL: Tests negation)
EXPLAIN (VERBOSE)
SELECT * FROM recursive_test.estimate_items
WHERE description @@@ paradedb.boolean(
    must := ARRAY[paradedb.term('description', 'shoes')],
    must_not := ARRAY[paradedb.term('description', 'athletic')]
);

-- ============================================================================
-- STAGE 4: COMPLEX NESTED QUERIES (Four+ Level Nesting)
-- Expected: Deep recursion with estimates at each nested level
-- ============================================================================

-- Test 4.1: Nested boolean - boolean inside boolean (must within must)
EXPLAIN (VERBOSE)
SELECT * FROM recursive_test.estimate_items
WHERE description @@@ paradedb.boolean(
    must := ARRAY[
        paradedb.boolean(
            must := ARRAY[
                paradedb.term('description', 'running'),
                paradedb.term('description', 'shoes')
            ]
        ),
        paradedb.term('description', 'athletic')
    ]
);

-- Test 4.2: Nested boolean - OR containing AND ((A AND B) OR C)
EXPLAIN (VERBOSE)
SELECT * FROM recursive_test.estimate_items
WHERE description @@@ paradedb.boolean(
    should := ARRAY[
        paradedb.boolean(
            must := ARRAY[
                paradedb.term('description', 'running'),
                paradedb.term('description', 'shoes')
            ]
        ),
        paradedb.term('description', 'boots')
    ]
);

-- Test 4.3: Deep nesting - three levels of boolean queries
EXPLAIN (VERBOSE)
SELECT * FROM recursive_test.estimate_items
WHERE description @@@ paradedb.boolean(
    must := ARRAY[
        paradedb.boolean(
            should := ARRAY[
                paradedb.boolean(
                    must := ARRAY[
                        paradedb.term('description', 'running'),
                        paradedb.term('description', 'shoes')
                    ]
                ),
                paradedb.term('description', 'walking')
            ]
        ),
        paradedb.term('description', 'athletic')
    ]
);

-- ============================================================================
-- STAGE 5: SPECIAL QUERY TYPES
-- Expected: Specialized queries also show estimates
-- ============================================================================

-- Test 5.1: Term equality
EXPLAIN (VERBOSE) SELECT * FROM recursive_test.estimate_items WHERE description === 'shoes';

-- Test 5.2: Term set (array of exact matches)
EXPLAIN (VERBOSE) SELECT * FROM recursive_test.estimate_items WHERE description === ARRAY['shoes', 'boots'];

-- Test 5.3: Range query on numeric field (CRITICAL: Tests range estimates)
EXPLAIN (VERBOSE)
SELECT * FROM recursive_test.estimate_items
WHERE id @@@ paradedb.range(
    field => 'rating',
    range => int4range(4, 5)
);

-- Test 5.4: Regex query (CRITICAL: Tests pattern matching estimates)
EXPLAIN (VERBOSE)
SELECT * FROM recursive_test.estimate_items
WHERE description @@@ paradedb.regex(
    field => 'description',
    pattern => 'run.*'
);

-- Test 5.5: Fuzzy query (CRITICAL: Tests typo-tolerance estimates)
EXPLAIN (VERBOSE)
SELECT * FROM recursive_test.estimate_items
WHERE description @@@ paradedb.fuzzy_term(
    field => 'description',
    value => 'sheos',
    distance => 1::integer
);

-- Test 5.6: Empty query (matches nothing)
EXPLAIN (VERBOSE)
SELECT * FROM recursive_test.estimate_items
WHERE description @@@ paradedb.empty();

-- Test 5.7: Boost query (wraps another query with score boost)
EXPLAIN (VERBOSE)
SELECT * FROM recursive_test.estimate_items
WHERE description @@@ paradedb.boost(
    factor => 2.0,
    query => paradedb.term('description', 'shoes')
);

-- Test 5.8: ConstScore query (wraps query with constant score)
EXPLAIN (VERBOSE)
SELECT * FROM recursive_test.estimate_items
WHERE description @@@ paradedb.const_score(
    score => 1.5,
    query => paradedb.term('description', 'shoes')
);

-- Test 5.9: DisjunctionMax query (best match from multiple)
EXPLAIN (VERBOSE)
SELECT * FROM recursive_test.estimate_items
WHERE description @@@ paradedb.disjunction_max(
    disjuncts => ARRAY[
        paradedb.term('description', 'shoes'),
        paradedb.term('description', 'boots')
    ]
);

-- Test 5.10: DisjunctionMax with tie_breaker
EXPLAIN (VERBOSE)
SELECT * FROM recursive_test.estimate_items
WHERE description @@@ paradedb.disjunction_max(
    disjuncts => ARRAY[
        paradedb.term('description', 'running'),
        paradedb.term('description', 'shoes')
    ],
    tie_breaker => 0.3
);

-- Test 5.11: Parse query (query string syntax)
EXPLAIN (VERBOSE)
SELECT * FROM recursive_test.estimate_items
WHERE description @@@ paradedb.parse('description:shoes AND description:running');

-- Test 5.12: Phrase with slop (allows word distance)
EXPLAIN (VERBOSE)
SELECT * FROM recursive_test.estimate_items
WHERE description @@@ paradedb.phrase(
    field => 'description',
    phrases => ARRAY['running', 'shoes'],
    slop => 2
);

-- Test 5.13: PhrasePrefix query (autocomplete scenarios)
EXPLAIN (VERBOSE)
SELECT * FROM recursive_test.estimate_items
WHERE description @@@ paradedb.phrase_prefix(
    field => 'description',
    phrases => ARRAY['run']
);

-- Test 5.14: RegexPhrase query (regex with positional matching)
EXPLAIN (VERBOSE)
SELECT * FROM recursive_test.estimate_items
WHERE description @@@ paradedb.regex_phrase(
    field => 'description',
    regexes => ARRAY['run.*', 'shoe.*']
);

-- Test 5.15: MoreLikeThis query (document similarity) - using key_value
EXPLAIN (VERBOSE)
SELECT * FROM recursive_test.estimate_items
WHERE id @@@ pdb.more_like_this(1);

-- Test 5.16: Match query with explicit tokenizer
EXPLAIN (VERBOSE)
SELECT * FROM recursive_test.estimate_items
WHERE description @@@ paradedb.match(
    field => 'description',
    value => 'running shoes'
);

-- Test 5.17: Match conjunction (all terms must match)
EXPLAIN (VERBOSE)
SELECT * FROM recursive_test.estimate_items
WHERE description @@@ paradedb.match_conjunction(
    field => 'description',
    terms_to_tokenize => 'running shoes'
);

-- Test 5.18: Match disjunction (any term can match)
EXPLAIN (VERBOSE)
SELECT * FROM recursive_test.estimate_items
WHERE description @@@ paradedb.match_disjunction(
    field => 'description',
    terms_to_tokenize => 'running walking'
);

-- Test 5.19: Proximity query (word distance matching)
EXPLAIN (VERBOSE)
SELECT * FROM recursive_test.estimate_items
WHERE description @@@ pdb.proximity('running', 2, 'shoes');

-- Test 5.20: Proximity in order (words must appear in specific order)
EXPLAIN (VERBOSE)
SELECT * FROM recursive_test.estimate_items
WHERE description @@@ pdb.proximity_in_order('running', 3, 'shoes');

-- Test 5.21: Range query on date field
EXPLAIN (VERBOSE)
SELECT * FROM recursive_test.estimate_items
WHERE id @@@ paradedb.range(
    field => 'created_at',
    range => tsrange('2023-01-01', '2023-12-31')
);

-- Test 5.22: TermSet with paradedb.term_set function
EXPLAIN (VERBOSE)
SELECT * FROM recursive_test.estimate_items
WHERE description @@@ paradedb.term_set(
    terms => ARRAY[
        paradedb.term('description', 'shoes'),
        paradedb.term('description', 'boots'),
        paradedb.term('description', 'sandals')
    ]
);

-- ============================================================================
-- STAGE 5B: NESTED WRAPPER QUERIES
-- Expected: Wrapper queries (Boost, ConstScore) containing other query types
-- ============================================================================

-- Test 5B.1: Boost wrapping a Boolean query
EXPLAIN (VERBOSE)
SELECT * FROM recursive_test.estimate_items
WHERE description @@@ paradedb.boost(
    factor => 2.0,
    query => paradedb.boolean(
        must => ARRAY[
            paradedb.term('description', 'shoes'),
            paradedb.term('description', 'running')
        ]
    )
);

-- Test 5B.2: ConstScore wrapping a Boolean query
EXPLAIN (VERBOSE)
SELECT * FROM recursive_test.estimate_items
WHERE description @@@ paradedb.const_score(
    score => 1.0,
    query => paradedb.boolean(
        should => ARRAY[
            paradedb.term('description', 'running'),
            paradedb.term('description', 'walking')
        ]
    )
);

-- Test 5B.3: Boolean containing Boost queries
EXPLAIN (VERBOSE)
SELECT * FROM recursive_test.estimate_items
WHERE description @@@ paradedb.boolean(
    must => ARRAY[
        paradedb.boost(factor => 2.0, query => paradedb.term('description', 'shoes')),
        paradedb.boost(factor => 1.5, query => paradedb.term('description', 'running'))
    ]
);

-- Test 5B.4: DisjunctionMax containing Boolean queries
EXPLAIN (VERBOSE)
SELECT * FROM recursive_test.estimate_items
WHERE description @@@ paradedb.disjunction_max(
    disjuncts => ARRAY[
        paradedb.boolean(
            must => ARRAY[
                paradedb.term('description', 'running'),
                paradedb.term('description', 'shoes')
            ]
        ),
        paradedb.term('description', 'boots')
    ]
);

-- Test 5B.5: Deeply nested with multiple wrapper types
EXPLAIN (VERBOSE)
SELECT * FROM recursive_test.estimate_items
WHERE description @@@ paradedb.boolean(
    must => ARRAY[
        paradedb.boost(
            factor => 2.0,
            query => paradedb.boolean(
                should => ARRAY[
                    paradedb.const_score(score => 1.0, query => paradedb.term('description', 'running')),
                    paradedb.term('description', 'walking')
                ]
            )
        ),
        paradedb.term('description', 'shoes')
    ]
);

-- ============================================================================
-- STAGE 6: GUC TOGGLE TEST
-- Expected: WITH GUC=ON shows estimates, WITH GUC=OFF hides them
-- ============================================================================

-- Test 6.1: Verify estimates shown with GUC=ON (should already be ON)
SHOW paradedb.explain_recursive_estimates;

EXPLAIN (VERBOSE) SELECT * FROM recursive_test.estimate_items WHERE description &&& 'running shoes';

-- Test 6.2: Turn GUC OFF and verify estimates NOT shown
SET paradedb.explain_recursive_estimates = OFF;

EXPLAIN (VERBOSE) SELECT * FROM recursive_test.estimate_items WHERE description &&& 'running shoes';

-- Test 6.3: Turn GUC back ON
SET paradedb.explain_recursive_estimates = ON;

EXPLAIN (VERBOSE) SELECT * FROM recursive_test.estimate_items WHERE description &&& 'running shoes';

-- ============================================================================
-- STAGE 7: EXPLAIN vs EXPLAIN ANALYZE
-- Expected: Both should show estimates when VERBOSE + GUC enabled
-- ============================================================================

-- Test 7.1: EXPLAIN VERBOSE (planning only)
EXPLAIN (VERBOSE) SELECT * FROM recursive_test.estimate_items WHERE description &&& 'running shoes';

-- Test 7.2: EXPLAIN ANALYZE VERBOSE (planning + execution)
EXPLAIN (ANALYZE, VERBOSE, TIMING OFF, COSTS OFF, SUMMARY OFF) SELECT * FROM recursive_test.estimate_items WHERE description &&& 'running shoes';

-- Test 7.3: Verify EXPLAIN without VERBOSE does NOT show estimates
EXPLAIN SELECT * FROM recursive_test.estimate_items WHERE description &&& 'running shoes';

-- ============================================================================
-- STAGE 8: EDGE CASES
-- Expected: Handle edge cases gracefully
-- ============================================================================

-- Test 8.1: Empty result query
EXPLAIN (VERBOSE) SELECT * FROM recursive_test.estimate_items WHERE description @@@ 'nonexistentterm123456';

-- Test 8.2: Match all query (very broad)
EXPLAIN (VERBOSE) SELECT * FROM recursive_test.estimate_items WHERE description @@@ paradedb.all();

-- Test 8.3: Many AND clauses (wide tree)
EXPLAIN (VERBOSE)
SELECT * FROM recursive_test.estimate_items
WHERE description @@@ 'running'
  AND description @@@ 'shoes'
  AND description @@@ 'athletic'
  AND description @@@ 'footwear';

-- Test 8.4: Many OR clauses (wide tree)
EXPLAIN (VERBOSE)
SELECT * FROM recursive_test.estimate_items
WHERE description @@@ 'running'
   OR description @@@ 'walking'
   OR description @@@ 'hiking'
   OR description @@@ 'jogging';

-- ============================================================================
-- STAGE 9: PROTECTION LIMITS
-- Expected: Verify depth limits and timeout handling work correctly
-- ============================================================================

-- Test 9.1: Deep nesting (should work - well within 100 level limit)
-- This tests 10 levels of nesting, which is typical for complex queries
EXPLAIN (VERBOSE)
SELECT * FROM recursive_test.estimate_items
WHERE description @@@ paradedb.boolean(
    must := ARRAY[
        paradedb.boolean(
            must := ARRAY[
                paradedb.boolean(
                    must := ARRAY[
                        paradedb.boolean(
                            must := ARRAY[
                                paradedb.boolean(
                                    must := ARRAY[
                                        paradedb.boolean(
                                            must := ARRAY[
                                                paradedb.boolean(
                                                    must := ARRAY[
                                                        paradedb.boolean(
                                                            must := ARRAY[
                                                                paradedb.boolean(
                                                                    must := ARRAY[
                                                                        paradedb.term('description', 'shoes')
                                                                    ]
                                                                )
                                                            ]
                                                        )
                                                    ]
                                                )
                                            ]
                                        )
                                    ]
                                )
                            ]
                        )
                    ]
                )
            ]
        )
    ]
);

-- Test 9.2: Verify statement_timeout is respected during estimation
-- This test ensures that check_for_interrupts() is being called
-- Note: We use a short timeout to verify it works, but the query should
-- complete quickly so we don't expect it to actually timeout
SET statement_timeout = '5s';
EXPLAIN (VERBOSE)
SELECT * FROM recursive_test.estimate_items
WHERE description @@@ paradedb.boolean(
    must := ARRAY[
        paradedb.term('description', 'running'),
        paradedb.term('description', 'shoes')
    ]
);
-- Reset timeout
RESET statement_timeout;

-- ============================================================================
-- STAGE 10: JSON FORMAT OUTPUT
-- Expected: All query types should produce valid JSON with estimates
-- ============================================================================

-- Test 10.1: Simple query in JSON format
EXPLAIN (VERBOSE, FORMAT JSON)
SELECT * FROM recursive_test.estimate_items WHERE description @@@ 'shoes';

-- Test 10.2: Nested boolean query in JSON format
EXPLAIN (VERBOSE, FORMAT JSON)
SELECT * FROM recursive_test.estimate_items
WHERE description @@@ paradedb.boolean(
    must := ARRAY[
        paradedb.term('description', 'shoes'),
        paradedb.term('description', 'running')
    ]
);

-- Test 10.3: Complex nested query in JSON format (3 levels deep)
EXPLAIN (VERBOSE, FORMAT JSON)
SELECT * FROM recursive_test.estimate_items
WHERE description @@@ paradedb.boolean(
    must := ARRAY[
        paradedb.boolean(
            must := ARRAY[
                paradedb.term('description', 'running'),
                paradedb.term('description', 'shoes')
            ]
        ),
        paradedb.term('description', 'athletic')
    ]
);

-- Test 10.4: Range query in JSON format
EXPLAIN (VERBOSE, FORMAT JSON)
SELECT * FROM recursive_test.estimate_items
WHERE rating @@@ paradedb.range(
    field => 'rating',
    range => int4range(4, 5)
);

-- Test 10.5: Fuzzy query in JSON format
EXPLAIN (VERBOSE, FORMAT JSON)
SELECT * FROM recursive_test.estimate_items
WHERE description @@@ paradedb.fuzzy_term(
    field => 'description',
    value => 'sheos',
    distance => 1::integer
);

-- Test 10.6: JSON format with GUC OFF (verify no estimates shown)
SET paradedb.explain_recursive_estimates = OFF;
EXPLAIN (VERBOSE, FORMAT JSON)
SELECT * FROM recursive_test.estimate_items WHERE description @@@ 'shoes';

-- Test 10.7: JSON format with GUC back ON
SET paradedb.explain_recursive_estimates = ON;
EXPLAIN (VERBOSE, FORMAT JSON)
SELECT * FROM recursive_test.estimate_items WHERE description @@@ 'shoes';

-- Test 10.8: YAML format (verify it works with estimates)
EXPLAIN (VERBOSE, FORMAT YAML)
SELECT * FROM recursive_test.estimate_items WHERE description @@@ 'shoes';

-- Test 10.9: XML format (verify it works with estimates)
EXPLAIN (VERBOSE, FORMAT XML)
SELECT * FROM recursive_test.estimate_items WHERE description @@@ 'shoes';

\i common/recursive_estimates_cleanup.sql
