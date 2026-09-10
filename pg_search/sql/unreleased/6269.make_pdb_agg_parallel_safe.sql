-- Make pdb.agg and its placeholder functions parallel safe so that queries
-- using pdb.agg can be parallelized with MPP (DistributedExec).

-- Overload 1: pdb.agg(jsonb)
DROP FUNCTION IF EXISTS pdb.agg_placeholder_agg_placeholder_state(this internal, arg_one jsonb);
CREATE OR REPLACE FUNCTION pdb.agg_placeholder_agg_placeholder_state(this internal, arg_one jsonb) RETURNS internal AS 'MODULE_PATHNAME', 'agg_placeholder_agg_placeholder_state_wrapper' LANGUAGE c PARALLEL SAFE;
DROP FUNCTION IF EXISTS pdb.agg_placeholder_agg_placeholder_finalize(this internal);
CREATE OR REPLACE FUNCTION pdb.agg_placeholder_agg_placeholder_finalize(this internal) RETURNS jsonb AS 'MODULE_PATHNAME', 'agg_placeholder_agg_placeholder_finalize_wrapper' LANGUAGE c PARALLEL SAFE;

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
DROP FUNCTION IF EXISTS pdb.agg_placeholder_with_mvcc_agg_placeholder_with_mvcc_state(this internal, arg_one jsonb, arg_two bool);
CREATE OR REPLACE FUNCTION pdb.agg_placeholder_with_mvcc_agg_placeholder_with_mvcc_state(this internal, arg_one jsonb, arg_two bool) RETURNS internal AS 'MODULE_PATHNAME', 'agg_placeholder_with_mvcc_agg_placeholder_with_mvcc_state_wrapper' LANGUAGE c PARALLEL SAFE;
DROP FUNCTION IF EXISTS pdb.agg_placeholder_with_mvcc_agg_placeholder_with_mvcc_finalize(this internal);
CREATE OR REPLACE FUNCTION pdb.agg_placeholder_with_mvcc_agg_placeholder_with_mvcc_finalize(this internal) RETURNS jsonb AS 'MODULE_PATHNAME', 'agg_placeholder_with_mvcc_agg_placeholder_with_mvcc_finalize_wrapper' LANGUAGE c PARALLEL SAFE;

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

-- Overload 3: pdb.agg(jsonb, text)
DROP FUNCTION IF EXISTS pdb.agg_placeholder_visibility_agg_placeholder_visibility_state(this internal, arg_one jsonb, arg_two text);
CREATE OR REPLACE FUNCTION pdb.agg_placeholder_visibility_agg_placeholder_visibility_state(this internal, arg_one jsonb, arg_two text) RETURNS internal AS 'MODULE_PATHNAME', 'agg_placeholder_visibility_agg_placeholder_visibility_state_wrapper' LANGUAGE c PARALLEL SAFE;
DROP FUNCTION IF EXISTS pdb.agg_placeholder_visibility_agg_placeholder_visibility_finalize(this internal);
CREATE OR REPLACE FUNCTION pdb.agg_placeholder_visibility_agg_placeholder_visibility_finalize(this internal) RETURNS jsonb AS 'MODULE_PATHNAME', 'agg_placeholder_visibility_agg_placeholder_visibility_finalize_wrapper' LANGUAGE c PARALLEL SAFE;

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
