EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description @@@ 'shoes'::fuzzy(2);
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description &&& 'shoes'::fuzzy(2);
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description ||| 'shoes'::fuzzy(2);
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description === 'shoes'::fuzzy(2);
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description @@@ pdb.term('shoes')::fuzzy(2);

SELECT * FROM regress.mock_items WHERE description === 'sho'::fuzzy(0) ORDER BY id; -- no results
SELECT * FROM regress.mock_items WHERE description === 'sho'::fuzzy(1) ORDER BY id; -- no results
SELECT * FROM regress.mock_items WHERE description === 'sho'::fuzzy(2) ORDER BY id; -- 3 rows

--
-- (currently) unsupported for phrase and proximity
--
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description ### 'running shoes'::fuzzy(2);
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description @@@ ('running' ##3## 'shoes')::fuzzy(2);


--
-- validate json representation
--
SELECT 'beer'::fuzzy(2);
SELECT 'beer'::fuzzy(2, t, t);
SELECT 'beer'::fuzzy(2, t, f);
SELECT 'beer'::fuzzy(2, f, f);
SELECT 'beer'::fuzzy(2, f, t);
SELECT 'beer'::fuzzy(2, "true", "true");
SELECT 'beer'::fuzzy(2, "false", "false");

--
-- error conditions
--
SELECT 'beer'::fuzzy(-1);
SELECT 'beer'::fuzzy(3);
SELECT 'beer'::fuzzy(hi_mom);
SELECT 'beer'::fuzzy(2, true, true);    -- thanks, Postgres!