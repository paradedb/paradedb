CREATE INDEX IF NOT EXISTS idxregress_mock_items
ON regress.mock_items
    USING bm25 (id, sku, description, (lower(description)::pdb.simple('alias=description_lower')), rating, category, in_stock, metadata, created_at, last_updated_date, latest_available_time, weight_range)
WITH (key_field='id');

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description ### 'running shoes'::pdb.slop(2);
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description @@@ pdb.phrase('running shoes')::pdb.slop(2);

SELECT * FROM regress.mock_items WHERE description ### 'running shoes'::pdb.slop(2);
SELECT * FROM regress.mock_items WHERE description @@@ pdb.phrase('running shoes')::pdb.slop(2);

SELECT * FROM regress.mock_items WHERE description ### 'running shoes'::pdb.slop(2)::pdb.boost(2);
SELECT * FROM regress.mock_items WHERE description @@@ pdb.phrase('running shoes')::pdb.slop(2)::pdb.boost(2);

SELECT * FROM regress.mock_items WHERE description ### ARRAY['shoes', 'running']::pdb.slop(2);
SELECT * FROM regress.mock_items WHERE description ### ARRAY['shoes', 'running']::pdb.slop(0);
SELECT * FROM regress.mock_items WHERE description ### ARRAY['shoes', 'running']::pdb.slop(1);


--
-- validate json representation
--
SELECT 'running shoes'::pdb.slop(2);

--
-- error conditions
--
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description ### 'running shoes'::pdb.boost(2)::pdb.slop(2);
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description @@@ 'running shoes'::pdb.slop(2);
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description &&& 'running shoes'::pdb.slop(2);
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description ||| 'running shoes'::pdb.slop(2);
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description === 'running shoes'::pdb.slop(2);
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description @@@ pdb.term('running shoes')::pdb.slop(2);
