EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description ### 'running shoes'::pdb.slop(2);
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description @@@ pdb.phrase('running shoes')::pdb.slop(2);

SELECT * FROM regress.mock_items WHERE description ### 'running shoes'::pdb.slop(2);
SELECT * FROM regress.mock_items WHERE description @@@ pdb.phrase('running shoes')::pdb.slop(2);

SELECT * FROM regress.mock_items WHERE description ### 'running shoes'::pdb.slop(2)::pdb.boost(2);
SELECT * FROM regress.mock_items WHERE description @@@ pdb.phrase('running shoes')::pdb.slop(2)::pdb.boost(2);



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
