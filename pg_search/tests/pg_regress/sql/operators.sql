\i common/common_setup.sql

CALL paradedb.create_bm25_test_table(
        schema_name => 'public',
        table_name => 'mock_items'
);

ALTER TABLE mock_items ADD COLUMN sku UUID;
UPDATE mock_items SET sku = ('da2fea21-' || lpad(to_hex( id::int4), 4, '0') || '-411b-9e8c-2cb64e471293')::uuid;
VACUUM FULL mock_items;

CREATE INDEX IF NOT EXISTS idxregress_mock_items
ON mock_items
    USING bm25 (id, sku, description, (lower(description)::pdb.simple('alias=description_lower')), rating, category, in_stock, metadata, created_at, last_updated_date, latest_available_time, weight_range)
WITH (key_field='id');

--
-- these are designed to validate that the EXPLAIN output is correct
-- and that each operator returns the expected number of rows
--


--
-- @@@ (parse)
--
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM mock_items WHERE description @@@ 'running shoes';
SELECT * FROM mock_items WHERE description @@@ 'running shoes';

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM mock_items WHERE lower(description) @@@ 'running shoes';
SELECT * FROM mock_items WHERE lower(description) @@@ 'running shoes' ORDER BY id;


--
-- &&& (match conjunction)
--

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM mock_items WHERE description &&& 'running shoes';
SELECT * FROM mock_items WHERE description &&& 'running shoes' ORDER BY id;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM mock_items WHERE lower(description) &&& 'running shoes';
SELECT * FROM mock_items WHERE lower(description) &&& 'running shoes' ORDER BY id;


--
-- ||| (match disjunction)
--

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM mock_items WHERE description ||| 'running shoes';
SELECT * FROM mock_items WHERE description ||| 'running shoes' ORDER BY id;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM mock_items WHERE lower(description) ||| 'running shoes';
SELECT * FROM mock_items WHERE lower(description) ||| 'running shoes' ORDER BY id;


--
-- ### (phrase)
--

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM mock_items WHERE description ### 'running shoes';
SELECT * FROM mock_items WHERE description ### 'running shoes' ORDER BY id;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM mock_items WHERE lower(description) ### 'running shoes';
SELECT * FROM mock_items WHERE lower(description) ### 'running shoes' ORDER BY id;


--
-- === (term equality)
--

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM mock_items WHERE description === 'shoes';
SELECT * FROM mock_items WHERE description === 'shoes' ORDER BY id;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM mock_items WHERE lower(description) === 'shoes';
SELECT * FROM mock_items WHERE lower(description) === 'shoes' ORDER BY id;



--
-- === (termset equality)
--

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM mock_items WHERE description === ARRAY['shoes', 'SHOES'];
SELECT * FROM mock_items WHERE description === ARRAY['shoes', 'SHOES'] ORDER BY id;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM mock_items WHERE lower(description) === ARRAY['shoes', 'SHOES'];
SELECT * FROM mock_items WHERE lower(description) === ARRAY['shoes', 'SHOES'] ORDER BY id;


---
--- the rhs of the operator is an expression that must be evaluated at execution time
---
select * from mock_items where description @@@ case when id = 1 then 'keyboard' else 'DoesNotExist' end;
select * from mock_items where description &&& case when id = 1 then 'keyboard' else 'DoesNotExist' end;
select * from mock_items where description ||| case when id = 1 then 'keyboard' else 'DoesNotExist' end;
select * from mock_items where description ### case when id = 1 then 'keyboard' else 'DoesNotExist' end;
select * from mock_items where description === case when id = 1 then 'keyboard' else 'DoesNotExist' end;


--
-- other supported types on the lhs
-- these are types that postgres will coerce to TEXT
--
SELECT * FROM mock_items WHERE description::varchar @@@ 'keyboard' ORDER BY id;
SELECT * FROM mock_items WHERE description::varchar &&& 'keyboard' ORDER BY id;
SELECT * FROM mock_items WHERE description::varchar ||| 'keyboard' ORDER BY id;
SELECT * FROM mock_items WHERE description::varchar ### 'keyboard' ORDER BY id;
SELECT * FROM mock_items WHERE description::varchar === 'keyboard' ORDER BY id;
SELECT * FROM mock_items WHERE category @@@ 'footwear' ORDER BY id;
SELECT * FROM mock_items WHERE category &&& 'footwear' ORDER BY id;
SELECT * FROM mock_items WHERE category ||| 'footwear' ORDER BY id;
SELECT * FROM mock_items WHERE category ### 'footwear' ORDER BY id;
SELECT * FROM mock_items WHERE category === 'footwear' ORDER BY id;

--
-- some unsupported types on the lhs
-- these will all produce an error
--
SELECT * FROM mock_items WHERE id &&& '42';
SELECT * FROM mock_items WHERE sku &&& 'da2fea21-000e-411b-9e8c-2cb64e471293';
SELECT * FROM mock_items WHERE in_stock &&& 'true';
SELECT * FROM mock_items WHERE last_updated_date &&& '12-06-25'::date::text;
SELECT * FROM mock_items WHERE latest_available_time &&& '12-06-25'::date::text;
SELECT * FROM mock_items WHERE id ||| '42';
SELECT * FROM mock_items WHERE sku ||| 'da2fea21-000e-411b-9e8c-2cb64e471293';
SELECT * FROM mock_items WHERE in_stock ||| 'true';
SELECT * FROM mock_items WHERE last_updated_date ||| '12-06-25'::date::text;
SELECT * FROM mock_items WHERE latest_available_time ||| '12-06-25'::date::text;
SELECT * FROM mock_items WHERE id ### '42';
SELECT * FROM mock_items WHERE sku ### 'da2fea21-000e-411b-9e8c-2cb64e471293';
SELECT * FROM mock_items WHERE in_stock ### 'true';
SELECT * FROM mock_items WHERE last_updated_date ### '12-06-25'::date::text;
SELECT * FROM mock_items WHERE latest_available_time ### '12-06-25'::date::text;
SELECT * FROM mock_items WHERE id === '42';
SELECT * FROM mock_items WHERE sku === 'da2fea21-000e-411b-9e8c-2cb64e471293';
SELECT * FROM mock_items WHERE in_stock === 'true';
SELECT * FROM mock_items WHERE last_updated_date === '12-06-25'::date::text;
SELECT * FROM mock_items WHERE latest_available_time === '12-06-25'::date::text;

DROP TABLE mock_items;
