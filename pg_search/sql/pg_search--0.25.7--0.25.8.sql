\echo Use "ALTER EXTENSION pg_search UPDATE TO '0.25.8'" to load this file. \quit

-- ============================================================================
-- Fragment: 6269.make_pdb_agg_parallel_safe.sql
-- ============================================================================
-- Make pdb.agg parallel safe so that queries using pdb.agg can be
-- parallelized with MPP (DistributedExec).

-- Overload 1: pdb.agg(jsonb)
CREATE OR REPLACE AGGREGATE pdb.agg (
	jsonb
)
(
	SFUNC = pdb."agg_placeholder_agg_placeholder_state",
	STYPE = internal,
	FINALFUNC = pdb."agg_placeholder_agg_placeholder_finalize",
	PARALLEL = SAFE
);

-- Overload 2: pdb.agg(jsonb, bool)
CREATE OR REPLACE AGGREGATE pdb.agg (
	jsonb,
	bool
)
(
	SFUNC = pdb."agg_placeholder_with_mvcc_agg_placeholder_with_mvcc_state",
	STYPE = internal,
	FINALFUNC = pdb."agg_placeholder_with_mvcc_agg_placeholder_with_mvcc_finalize",
	PARALLEL = SAFE
);

