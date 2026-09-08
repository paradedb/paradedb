-- depends-on: 6099
-- Make pdb.agg(jsonb, text) parallel safe so that queries using pdb.agg can be
-- parallelized with MPP (DistributedExec).

-- Overload 3: pdb.agg(jsonb, text)
CREATE OR REPLACE AGGREGATE pdb.agg (
	jsonb,
	text
)
(
	SFUNC = pdb."agg_placeholder_visibility_agg_placeholder_visibility_state",
	STYPE = internal,
	FINALFUNC = pdb."agg_placeholder_visibility_agg_placeholder_visibility_finalize",
	PARALLEL = SAFE
);
