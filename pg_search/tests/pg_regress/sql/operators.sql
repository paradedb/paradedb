--
-- these are designed to validate that the EXPLAIN output is correct
-- and that each operator returns the expected number of rows
--

    
--
-- @@@ (parse)
--
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description @@@ 'running shoes';
SELECT * FROM regress.mock_items WHERE description @@@ 'running shoes';

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE lower(description) @@@ 'running shoes';
SELECT * FROM regress.mock_items WHERE lower(description) @@@ 'running shoes' ORDER BY id;


--
-- &&& (match conjunction)
--

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description &&& 'running shoes';
SELECT * FROM regress.mock_items WHERE description &&& 'running shoes' ORDER BY id;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE lower(description) &&& 'running shoes';
SELECT * FROM regress.mock_items WHERE lower(description) &&& 'running shoes' ORDER BY id;


--
-- ||| (match disjunction)
--

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description ||| 'running shoes';
SELECT * FROM regress.mock_items WHERE description ||| 'running shoes' ORDER BY id;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE lower(description) ||| 'running shoes';
SELECT * FROM regress.mock_items WHERE lower(description) ||| 'running shoes' ORDER BY id;


--
-- ### (phrase)
--

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description ### 'running shoes';
SELECT * FROM regress.mock_items WHERE description ### 'running shoes' ORDER BY id;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE lower(description) ### 'running shoes';
SELECT * FROM regress.mock_items WHERE lower(description) ### 'running shoes' ORDER BY id;


--
-- === (term equality)
--

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description === 'shoes';
SELECT * FROM regress.mock_items WHERE description === 'shoes' ORDER BY id;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE lower(description) === 'shoes';
SELECT * FROM regress.mock_items WHERE lower(description) === 'shoes' ORDER BY id;



--
-- === (termset equality)
--

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description === ARRAY['shoes', 'SHOES'];
SELECT * FROM regress.mock_items WHERE description === ARRAY['shoes', 'SHOES'] ORDER BY id;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE lower(description) === ARRAY['shoes', 'SHOES'];
SELECT * FROM regress.mock_items WHERE lower(description) === ARRAY['shoes', 'SHOES'] ORDER BY id;


---
--- the rhs of the operator is an expression that must be evaluated at execution time
---
select * from regress.mock_items where description @@@ case when id = 1 then 'keyboard' else 'DoesNotExist' end;
select * from regress.mock_items where description &&& case when id = 1 then 'keyboard' else 'DoesNotExist' end;
select * from regress.mock_items where description ||| case when id = 1 then 'keyboard' else 'DoesNotExist' end;
select * from regress.mock_items where description ### case when id = 1 then 'keyboard' else 'DoesNotExist' end;
select * from regress.mock_items where description === case when id = 1 then 'keyboard' else 'DoesNotExist' end;


--
-- other supported types on the lhs
-- these are types that postgres will coerce to TEXT
--
SELECT * FROM regress.mock_items WHERE description::varchar @@@ 'keyboard' ORDER BY id;
SELECT * FROM regress.mock_items WHERE description::varchar &&& 'keyboard' ORDER BY id;
SELECT * FROM regress.mock_items WHERE description::varchar ||| 'keyboard' ORDER BY id;
SELECT * FROM regress.mock_items WHERE description::varchar ### 'keyboard' ORDER BY id;
SELECT * FROM regress.mock_items WHERE description::varchar === 'keyboard' ORDER BY id;
SELECT * FROM regress.mock_items WHERE category @@@ 'footwear' ORDER BY id;
SELECT * FROM regress.mock_items WHERE category &&& 'footwear' ORDER BY id;
SELECT * FROM regress.mock_items WHERE category ||| 'footwear' ORDER BY id;
SELECT * FROM regress.mock_items WHERE category ### 'footwear' ORDER BY id;
SELECT * FROM regress.mock_items WHERE category === 'footwear' ORDER BY id;

--
-- some unsupported types on the lhs
-- these will all produce an error
--
SELECT * FROM regress.mock_items WHERE id &&& '42';
SELECT * FROM regress.mock_items WHERE sku &&& 'da2fea21-000e-411b-9e8c-2cb64e471293';
SELECT * FROM regress.mock_items WHERE in_stock &&& 'true';
SELECT * FROM regress.mock_items WHERE last_updated_date &&& now()::date::text;
SELECT * FROM regress.mock_items WHERE latest_available_time &&& now()::date::text;
SELECT * FROM regress.mock_items WHERE id ||| '42';
SELECT * FROM regress.mock_items WHERE sku ||| 'da2fea21-000e-411b-9e8c-2cb64e471293';
SELECT * FROM regress.mock_items WHERE in_stock ||| 'true';
SELECT * FROM regress.mock_items WHERE last_updated_date ||| now()::date::text;
SELECT * FROM regress.mock_items WHERE latest_available_time ||| now()::date::text;
SELECT * FROM regress.mock_items WHERE id ### '42';
SELECT * FROM regress.mock_items WHERE sku ### 'da2fea21-000e-411b-9e8c-2cb64e471293';
SELECT * FROM regress.mock_items WHERE in_stock ### 'true';
SELECT * FROM regress.mock_items WHERE last_updated_date ### now()::date::text;
SELECT * FROM regress.mock_items WHERE latest_available_time ### now()::date::text;
SELECT * FROM regress.mock_items WHERE id === '42';
SELECT * FROM regress.mock_items WHERE sku === 'da2fea21-000e-411b-9e8c-2cb64e471293';
SELECT * FROM regress.mock_items WHERE in_stock === 'true';
SELECT * FROM regress.mock_items WHERE last_updated_date === now()::date::text;
SELECT * FROM regress.mock_items WHERE latest_available_time === now()::date::text;
