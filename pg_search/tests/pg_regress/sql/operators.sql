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


--
-- === (term equality)
--

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT COUNT(*) FROM regress.mock_items WHERE description === 'shoes';
SELECT COUNT(*) FROM regress.mock_items WHERE description === 'shoes';

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT COUNT(*) FROM regress.mock_items WHERE lower(description) === 'shoes';
SELECT COUNT(*) FROM regress.mock_items WHERE lower(description) === 'shoes';



--
-- === (termset equality)
--

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT COUNT(*) FROM regress.mock_items WHERE description === ARRAY['shoes', 'SHOES'];
SELECT COUNT(*) FROM regress.mock_items WHERE description === ARRAY['shoes', 'SHOES'];

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT COUNT(*) FROM regress.mock_items WHERE lower(description) === ARRAY['shoes', 'SHOES'];
SELECT COUNT(*) FROM regress.mock_items WHERE lower(description) === ARRAY['shoes', 'SHOES'];


--
-- some unsupported types on the lhs
-- these will all produce an error
--
SELECT COUNT(*) FROM regress.mock_items WHERE id &&& '42';
SELECT COUNT(*) FROM regress.mock_items WHERE sku &&& 'da2fea21-000e-411b-9e8c-2cb64e471293';
SELECT COUNT(*) FROM regress.mock_items WHERE in_stock &&& 'true';
SELECT COUNT(*) FROM regress.mock_items WHERE last_updated_date &&& now()::date::text;
SELECT COUNT(*) FROM regress.mock_items WHERE latest_available_time &&& now()::date::text;
SELECT COUNT(*) FROM regress.mock_items WHERE id ||| '42';
SELECT COUNT(*) FROM regress.mock_items WHERE sku ||| 'da2fea21-000e-411b-9e8c-2cb64e471293';
SELECT COUNT(*) FROM regress.mock_items WHERE in_stock ||| 'true';
SELECT COUNT(*) FROM regress.mock_items WHERE last_updated_date ||| now()::date::text;
SELECT COUNT(*) FROM regress.mock_items WHERE latest_available_time ||| now()::date::text;
SELECT COUNT(*) FROM regress.mock_items WHERE id ### '42';
SELECT COUNT(*) FROM regress.mock_items WHERE sku ### 'da2fea21-000e-411b-9e8c-2cb64e471293';
SELECT COUNT(*) FROM regress.mock_items WHERE in_stock ### 'true';
SELECT COUNT(*) FROM regress.mock_items WHERE last_updated_date ### now()::date::text;
SELECT COUNT(*) FROM regress.mock_items WHERE latest_available_time ### now()::date::text;
SELECT COUNT(*) FROM regress.mock_items WHERE id === '42';
SELECT COUNT(*) FROM regress.mock_items WHERE sku === 'da2fea21-000e-411b-9e8c-2cb64e471293';
SELECT COUNT(*) FROM regress.mock_items WHERE in_stock === 'true';
SELECT COUNT(*) FROM regress.mock_items WHERE last_updated_date === now()::date::text;
SELECT COUNT(*) FROM regress.mock_items WHERE latest_available_time === now()::date::text;
