EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description @@@ 'running shoes'::pdb.const(3);
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description @@@ 'running shoes'::pdb.const(3.14159);
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description @@@ 'running shoes'::pdb.const(0.5);
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description @@@ 'running shoes'::pdb.const(0);
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description @@@ 'running shoes'::pdb.const(-100);

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description &&& 'running shoes'::pdb.const(3);
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description &&& 'running shoes'::pdb.const(3.14159);
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description &&& 'running shoes'::pdb.const(0.5);
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description &&& 'running shoes'::pdb.const(0);
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description &&& 'running shoes'::pdb.const(-100);

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description ||| 'running shoes'::pdb.const(3);
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description ||| 'running shoes'::pdb.const(3.14159);
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description ||| 'running shoes'::pdb.const(0.5);
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description ||| 'running shoes'::pdb.const(0);
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description ||| 'running shoes'::pdb.const(-100);

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description ### 'running shoes'::pdb.const(3);
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description ### 'running shoes'::pdb.const(3.14159);
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description ### 'running shoes'::pdb.const(0.5);
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description ### 'running shoes'::pdb.const(0);
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description ### 'running shoes'::pdb.const(-100);

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description === 'shoes'::pdb.const(3);
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description === 'shoes'::pdb.const(3.14159);
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description === 'shoes'::pdb.const(0.5);
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description === 'shoes'::pdb.const(0);
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description === 'shoes'::pdb.const(-100);

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description @@@ pdb.term('shoes')::pdb.const(-100);
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description @@@ ('running' ##3## 'shoes')::pdb.const(3);

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description &&& ARRAY['running', 'shoes']::pdb.const(3);
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description ||| ARRAY['running', 'shoes']::pdb.const(3);
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description ### ARRAY['running', 'shoes']::pdb.const(3);
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description === ARRAY['running', 'shoes']::pdb.const(3);

--
-- validate basic json representations
--
SELECT 'foo'::pdb.const(3);
SELECT pdb.term('foo')::pdb.const(3);
SELECT ARRAY['foo', 'bar']::pdb.const(3);

--
-- oob cases.  these all get clamped to [-2048..2048]
--
SELECT 'foo'::pdb.const(2049);
SELECT 'foo'::pdb.const(-2049);
SELECT 'foo'::pdb.const(Inf);

--
-- error cases
--
SELECT 'foo'::pdb.const(hi_mom);
SELECT 'foo'::pdb.const(1,2);
SELECT 'foo'::pdb.const(NaN);
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description @@@ 'shoes'::pdb.const;
