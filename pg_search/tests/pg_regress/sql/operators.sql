--
-- these are designed to validate that the EXPLAIN output is correct
-- and that each operator returns the expected number of rows
--

    
--
-- @@@ (parse)
--
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT COUNT(*) FROM regress.mock_items WHERE description @@@ 'running shoes';
SELECT COUNT(*) FROM regress.mock_items WHERE description @@@ 'running shoes';

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT COUNT(*) FROM regress.mock_items WHERE lower(description) @@@ 'running shoes';
SELECT COUNT(*) FROM regress.mock_items WHERE lower(description) @@@ 'running shoes';


--
-- &&& (match conjunction)
--

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT COUNT(*) FROM regress.mock_items WHERE description &&& 'running shoes';
SELECT COUNT(*) FROM regress.mock_items WHERE description &&& 'running shoes';

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT COUNT(*) FROM regress.mock_items WHERE lower(description) &&& 'running shoes';
SELECT COUNT(*) FROM regress.mock_items WHERE lower(description) &&& 'running shoes';


--
-- ||| (match disjunction)
--

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT COUNT(*) FROM regress.mock_items WHERE description ||| 'running shoes';
SELECT COUNT(*) FROM regress.mock_items WHERE description ||| 'running shoes';

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT COUNT(*) FROM regress.mock_items WHERE lower(description) ||| 'running shoes';
SELECT COUNT(*) FROM regress.mock_items WHERE lower(description) ||| 'running shoes';


--
-- ### (phrase)
--

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT COUNT(*) FROM regress.mock_items WHERE description ### 'running shoes';
SELECT COUNT(*) FROM regress.mock_items WHERE description ### 'running shoes';

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT COUNT(*) FROM regress.mock_items WHERE lower(description) ### 'running shoes';
SELECT COUNT(*) FROM regress.mock_items WHERE lower(description) ### 'running shoes';

