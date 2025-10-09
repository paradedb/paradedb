EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description ### 'running shoes'::slop(2);
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description @@@ pdb.phrase('running shoes')::slop(2);

SELECT * FROM regress.mock_items WHERE description ### 'running shoes'::slop(2);
SELECT * FROM regress.mock_items WHERE description @@@ pdb.phrase('running shoes')::slop(2);

SELECT * FROM regress.mock_items WHERE description ### 'running shoes'::slop(2)::boost(2);
SELECT * FROM regress.mock_items WHERE description @@@ pdb.phrase('running shoes')::slop(2)::boost(2);



--
-- validate json representation
--
SELECT 'running shoes'::slop(2);

--
-- error conditions
--
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description ### 'running shoes'::boost(2)::slop(2);
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description @@@ 'running shoes'::slop(2);
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description &&& 'running shoes'::slop(2);
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description ||| 'running shoes'::slop(2);
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description === 'running shoes'::slop(2);
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description @@@ pdb.term('running shoes')::slop(2);
