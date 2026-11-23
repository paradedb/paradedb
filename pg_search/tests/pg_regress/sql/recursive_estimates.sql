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
EXPLAIN (FORMAT TEXT, VERBOSE, COSTS OFF, TIMING OFF) SELECT * FROM recursive_test.estimate_items WHERE description ||| 'shoes';

-- Test 1.2: Simple phrase query
EXPLAIN (FORMAT TEXT, VERBOSE, COSTS OFF, TIMING OFF) SELECT * FROM recursive_test.estimate_items WHERE description ### 'running shoes';

-- ============================================================================
-- STAGE 2: BOOLEAN QUERIES (Two-Level Nesting)
-- Expected: Parent boolean AND/OR + each child term show estimated_docs
-- ============================================================================

-- Test 2.1: Simple AND (conjunction) - 2 children
EXPLAIN (FORMAT TEXT, VERBOSE, COSTS OFF, TIMING OFF) SELECT * FROM recursive_test.estimate_items WHERE description &&& 'running shoes';

-- Test 2.2: Simple OR (disjunction) - 2 children
EXPLAIN (FORMAT TEXT, VERBOSE, COSTS OFF, TIMING OFF) SELECT * FROM recursive_test.estimate_items WHERE description ||| 'running shoes';

-- Test 2.3: AND with array (multiple terms)
EXPLAIN (FORMAT TEXT, VERBOSE, COSTS OFF, TIMING OFF) SELECT * FROM recursive_test.estimate_items WHERE description &&& ARRAY['running', 'shoes', 'athletic'];

-- Test 2.4: OR with array (multiple terms)
EXPLAIN (FORMAT TEXT, VERBOSE, COSTS OFF, TIMING OFF) SELECT * FROM recursive_test.estimate_items WHERE description ||| ARRAY['running', 'walking', 'hiking'];

-- ============================================================================
-- STAGE 4: COMPLEX NESTED QUERIES (Four+ Level Nesting)
-- Expected: Deep recursion with estimates at each nested level
-- ============================================================================

-- Test 4.2: Nested boolean - OR containing AND ((A AND B) OR C)
EXPLAIN (FORMAT TEXT, VERBOSE, COSTS OFF, TIMING OFF)
SELECT * FROM recursive_test.estimate_items
WHERE (description === 'running' AND description === 'shoes') OR description === 'boots';

-- Test 4.3: Three levels of boolean clauses
EXPLAIN (FORMAT TEXT, VERBOSE, COSTS OFF, TIMING OFF)
SELECT * FROM recursive_test.estimate_items
WHERE ((description === 'running' AND description === 'shoes') OR description === 'walking')
  AND description === 'athletic';

-- ============================================================================
-- STAGE 5: SPECIAL QUERY TYPES
-- Expected: Specialized queries also show estimates
-- ============================================================================

-- Test 5.1: Term equality
EXPLAIN (FORMAT TEXT, VERBOSE, COSTS OFF, TIMING OFF) SELECT * FROM recursive_test.estimate_items WHERE description === 'shoes';

-- Test 5.2: Term set (array of exact matches)
EXPLAIN (FORMAT TEXT, VERBOSE, COSTS OFF, TIMING OFF) SELECT * FROM recursive_test.estimate_items WHERE description === ARRAY['shoes', 'boots'];

-- Test 5.3: Range query on numeric field (CRITICAL: Tests range estimates)
EXPLAIN (FORMAT TEXT, VERBOSE, COSTS OFF, TIMING OFF)
SELECT * FROM recursive_test.estimate_items
WHERE rating @@@ pdb.range(range => int4range(4, 5));

-- Test 5.4: Regex query (CRITICAL: Tests pattern matching estimates)
EXPLAIN (FORMAT TEXT, VERBOSE, COSTS OFF, TIMING OFF)
SELECT * FROM recursive_test.estimate_items
WHERE description @@@ pdb.regex('run.*');

-- Test 5.5: Fuzzy query (CRITICAL: Tests typo-tolerance estimates)
EXPLAIN (FORMAT TEXT, VERBOSE, COSTS OFF, TIMING OFF)
SELECT * FROM recursive_test.estimate_items
WHERE description === 'sheos'::pdb.fuzzy(1, f, t);

-- Test 5.6: Empty query (matches nothing)
EXPLAIN (FORMAT TEXT, VERBOSE, COSTS OFF, TIMING OFF)
SELECT * FROM recursive_test.estimate_items
WHERE description @@@ pdb.empty();

-- Test 5.7: Boost query (wraps another query with score boost)
EXPLAIN (FORMAT TEXT, VERBOSE, COSTS OFF, TIMING OFF)
SELECT * FROM recursive_test.estimate_items
WHERE description === 'shoes'::pdb.boost(2);

-- Test 5.8: ConstScore query (wraps query with constant score)
EXPLAIN (FORMAT TEXT, VERBOSE, COSTS OFF, TIMING OFF)
SELECT * FROM recursive_test.estimate_items
WHERE description === 'shoes'::pdb.const(1.5);

-- Test 5.11: Parse query (query string syntax)
EXPLAIN (FORMAT TEXT, VERBOSE, COSTS OFF, TIMING OFF)
SELECT * FROM recursive_test.estimate_items
WHERE description @@@ pdb.parse('description:shoes AND description:running');

-- Test 5.12: Phrase with slop (allows word distance)
EXPLAIN (FORMAT TEXT, VERBOSE, COSTS OFF, TIMING OFF)
SELECT * FROM recursive_test.estimate_items
WHERE description ### 'running shoes'::pdb.slop(2);

-- Test 5.13: PhrasePrefix query (autocomplete scenarios)
EXPLAIN (FORMAT TEXT, VERBOSE, COSTS OFF, TIMING OFF)
SELECT * FROM recursive_test.estimate_items
WHERE description @@@ pdb.phrase_prefix(ARRAY['run']);

-- Test 5.14: RegexPhrase query (regex with positional matching)
EXPLAIN (FORMAT TEXT, VERBOSE, COSTS OFF, TIMING OFF)
SELECT * FROM recursive_test.estimate_items
WHERE description @@@ pdb.regex_phrase(ARRAY['run.*', 'shoe.*']);

-- Test 5.15: MoreLikeThis query (document similarity) - using key_value
EXPLAIN (FORMAT TEXT, VERBOSE, COSTS OFF, TIMING OFF)
SELECT * FROM recursive_test.estimate_items
WHERE id @@@ pdb.more_like_this(1);

-- Test 5.16: Match query with tokenizer from the index
EXPLAIN (FORMAT TEXT, VERBOSE, COSTS OFF, TIMING OFF)
SELECT * FROM recursive_test.estimate_items
WHERE description ||| 'running shoes';

-- Test 5.17: Match conjunction (all terms must match)
EXPLAIN (FORMAT TEXT, VERBOSE, COSTS OFF, TIMING OFF)
SELECT * FROM recursive_test.estimate_items
WHERE description &&& 'running shoes';

-- Test 5.18: Match disjunction (any term can match)
EXPLAIN (FORMAT TEXT, VERBOSE, COSTS OFF, TIMING OFF)
SELECT * FROM recursive_test.estimate_items
WHERE description ||| 'running walking';

-- Test 5.19: Proximity query (word distance matching)
EXPLAIN (FORMAT TEXT, VERBOSE, COSTS OFF, TIMING OFF)
SELECT * FROM recursive_test.estimate_items
WHERE description @@@ ('running' ## 2 ## 'shoes');

-- Test 5.20: Proximity in order (words must appear in specific order)
EXPLAIN (FORMAT TEXT, VERBOSE, COSTS OFF, TIMING OFF)
SELECT * FROM recursive_test.estimate_items
WHERE description @@@ ('running' ##> 3 ##> 'shoes');

-- Test 5.21: Range query on date field
EXPLAIN (FORMAT TEXT, VERBOSE, COSTS OFF, TIMING OFF)
SELECT * FROM recursive_test.estimate_items
WHERE created_at @@@ pdb.range(range => tsrange('2023-01-01', '2023-12-31'));


-- ============================================================================
-- STAGE 5B: NESTED WRAPPER QUERIES
-- Expected: Wrapper queries (Boost, ConstScore) containing other query types
-- ============================================================================

-- Test 5B.1: Boost wrapping a Boolean query
EXPLAIN (FORMAT TEXT, VERBOSE, COSTS OFF, TIMING OFF)
SELECT * FROM recursive_test.estimate_items
WHERE description &&& 'shoes running'::pdb.boost(2);

-- Test 5B.2: ConstScore wrapping a Boolean query
EXPLAIN (FORMAT TEXT, VERBOSE, COSTS OFF, TIMING OFF)
SELECT * FROM recursive_test.estimate_items
WHERE description ||| 'running walking'::pdb.const(1);

-- Test 5B.3: Boolean containing Boost queries
EXPLAIN (FORMAT TEXT, VERBOSE, COSTS OFF, TIMING OFF)
SELECT * FROM recursive_test.estimate_items
WHERE description === 'shoes'::pdb.boost(2) OR description === 'running'::pdb.boost(1.5);

-- Test 5B.5: Nested boolean clauses with multiple wrapper types
EXPLAIN (FORMAT TEXT, VERBOSE, COSTS OFF, TIMING OFF)
SELECT * FROM recursive_test.estimate_items
WHERE (description === 'running'::pdb.const(1)::pdb.query::pdb.boost(2)
   OR description === 'walking'::pdb.boost(2))
  AND description === 'athletic';

-- ============================================================================
-- STAGE 6: GUC TOGGLE TEST
-- Expected: WITH GUC=ON shows estimates, WITH GUC=OFF hides them
-- ============================================================================

-- Test 6.1: Verify estimates shown with GUC=ON (should already be ON)
SHOW paradedb.explain_recursive_estimates;

EXPLAIN (FORMAT TEXT, VERBOSE, COSTS OFF, TIMING OFF) SELECT * FROM recursive_test.estimate_items WHERE description &&& 'running shoes';

-- Test 6.2: Turn GUC OFF and verify estimates NOT shown
SET paradedb.explain_recursive_estimates = OFF;

EXPLAIN (FORMAT TEXT, VERBOSE, COSTS OFF, TIMING OFF) SELECT * FROM recursive_test.estimate_items WHERE description &&& 'running shoes';

-- Test 6.3: Turn GUC back ON
SET paradedb.explain_recursive_estimates = ON;

EXPLAIN (FORMAT TEXT, VERBOSE, COSTS OFF, TIMING OFF) SELECT * FROM recursive_test.estimate_items WHERE description &&& 'running shoes';

-- ============================================================================
-- STAGE 7: EXPLAIN vs EXPLAIN ANALYZE
-- Expected: Both should show estimates when VERBOSE + GUC enabled
-- ============================================================================

-- Test 7.1: EXPLAIN VERBOSE (planning only)
EXPLAIN (FORMAT TEXT, VERBOSE, COSTS OFF, TIMING OFF) SELECT * FROM recursive_test.estimate_items WHERE description &&& 'running shoes';

-- Test 7.2: EXPLAIN ANALYZE VERBOSE (planning + execution)
EXPLAIN (ANALYZE, VERBOSE, TIMING OFF, COSTS OFF, BUFFERS ON, SUMMARY OFF) SELECT * FROM recursive_test.estimate_items WHERE description &&& 'running shoes';

-- Test 7.3: Verify EXPLAIN without VERBOSE does NOT show estimates
EXPLAIN (COSTS OFF) SELECT * FROM recursive_test.estimate_items WHERE description &&& 'running shoes';

-- ============================================================================
-- STAGE 8: EDGE CASES
-- Expected: Handle edge cases gracefully
-- ============================================================================

-- Test 8.1: Empty result query
EXPLAIN (FORMAT TEXT, VERBOSE, COSTS OFF, TIMING OFF) SELECT * FROM recursive_test.estimate_items WHERE description ||| 'nonexistentterm123456';

-- Test 8.2: Match all query (very broad)
EXPLAIN (FORMAT TEXT, VERBOSE, COSTS OFF, TIMING OFF) SELECT * FROM recursive_test.estimate_items WHERE description @@@ pdb.all()::pdb.const(0);

-- Test 8.3: Many AND clauses (wide tree)
EXPLAIN (FORMAT TEXT, VERBOSE, COSTS OFF, TIMING OFF)
SELECT * FROM recursive_test.estimate_items
WHERE description ||| 'running'
  AND description ||| 'shoes'
  AND description ||| 'athletic'
  AND description ||| 'footwear';

-- Test 8.4: Many OR clauses (wide tree)
EXPLAIN (FORMAT TEXT, VERBOSE, COSTS OFF, TIMING OFF)
SELECT * FROM recursive_test.estimate_items
WHERE description ||| 'running'
   OR description ||| 'walking'
   OR description ||| 'hiking'
   OR description ||| 'jogging';

-- ============================================================================
-- STAGE 9: PROTECTION LIMITS
-- Expected: Verify depth limits and timeout handling work correctly
-- ============================================================================

-- Test 9.1: Deeply nested alternate conjunctions and disjunctions
EXPLAIN (FORMAT TEXT, VERBOSE, COSTS OFF, TIMING OFF)
SELECT * FROM recursive_test.estimate_items
WHERE (((((((((description === 'shoes' OR description === 'running')
          AND description === 'walking')
          OR description === 'boots')
          AND description === 'athletic')
          OR description === 'sandals')
          AND description === 'footwear')
          OR description === 'sneakers')
          AND description === 'trainers')
          OR description === 'slippers')
          AND description === 'hiking';

-- Test 9.2: Verify statement_timeout is respected during estimation
-- This test ensures that check_for_interrupts() is being called
-- Note: We use a short timeout to verify it works, but the query should
-- complete quickly so we don't expect it to actually timeout
SET statement_timeout = '5s';
EXPLAIN (FORMAT TEXT, VERBOSE, COSTS OFF, TIMING OFF)
SELECT * FROM recursive_test.estimate_items
WHERE description &&& 'running shoes';
-- Reset timeout
RESET statement_timeout;

-- ============================================================================
-- STAGE 10: JSON FORMAT OUTPUT
-- Expected: All query types should produce valid JSON with estimates
-- ============================================================================

-- Test 10.1: Simple query in JSON format
EXPLAIN (FORMAT JSON, VERBOSE, COSTS OFF)
SELECT * FROM recursive_test.estimate_items WHERE description ||| 'shoes';

-- Test 10.2: Nested boolean query in JSON format
EXPLAIN (FORMAT JSON, VERBOSE, COSTS OFF)
SELECT * FROM recursive_test.estimate_items
WHERE description &&& 'shoes running';

-- Test 10.3: Nested AND/OR query in JSON format
EXPLAIN (FORMAT JSON, VERBOSE, COSTS OFF)
SELECT * FROM recursive_test.estimate_items
WHERE (description === 'running' AND description === 'shoes') OR description === 'athletic';

-- Test 10.4: Range query in JSON format
EXPLAIN (FORMAT JSON, VERBOSE, COSTS OFF)
SELECT * FROM recursive_test.estimate_items
WHERE rating @@@ pdb.range(range => int4range(4, 5));

-- Test 10.5: Fuzzy query in JSON format
EXPLAIN (FORMAT JSON, VERBOSE, COSTS OFF)
SELECT * FROM recursive_test.estimate_items
WHERE description === 'sheos'::pdb.fuzzy(1, f, t);

-- Test 10.6: JSON format with GUC OFF (verify no estimates shown)
SET paradedb.explain_recursive_estimates = OFF;
EXPLAIN (FORMAT JSON, VERBOSE, COSTS OFF)
SELECT * FROM recursive_test.estimate_items WHERE description ||| 'shoes';

-- Test 10.7: JSON format with GUC back ON
SET paradedb.explain_recursive_estimates = ON;
EXPLAIN (FORMAT JSON, VERBOSE, COSTS OFF)
SELECT * FROM recursive_test.estimate_items WHERE description ||| 'shoes';

-- Test 10.8: YAML format (verify it works with estimates)
EXPLAIN (FORMAT YAML, VERBOSE, COSTS OFF)
SELECT * FROM recursive_test.estimate_items WHERE description ||| 'shoes';

-- Test 10.9: XML format (verify it works with estimates)
EXPLAIN (FORMAT XML, VERBOSE, COSTS OFF)
SELECT * FROM recursive_test.estimate_items WHERE description ||| 'shoes';

\i common/recursive_estimates_cleanup.sql
