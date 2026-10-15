-- Add `pdb.combine_agg(jsonb) -> jsonb` for combining `pdb.agg(...)` across
-- `GROUP BY` groups via `pdb.combine_agg(pdb.agg(...)) OVER ()` (#6804).
--
-- Every CREATE below is the SchemaBot/pgrx canonical text verbatim (the schema
-- checker compares statements textually); the DROPs keep the fragment idempotent.

DROP AGGREGATE IF EXISTS pdb.combine_agg(jsonb);
DROP FUNCTION IF EXISTS pdb."combine_agg_stub_combine_agg_stub_state"(internal, jsonb);
DROP FUNCTION IF EXISTS pdb."combine_agg_stub_combine_agg_stub_finalize"(internal);

-- pg_search/src/api/aggregate.rs:361
-- pg_search::api::aggregate::pdb::combine_agg_stub_combine_agg_stub_state
CREATE  FUNCTION pdb."combine_agg_stub_combine_agg_stub_state"(
	"this" internal, /* Internal */
	"arg_one" jsonb /* JsonB */
) RETURNS internal /* Internal */
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'combine_agg_stub_combine_agg_stub_state_wrapper';

-- pg_search/src/api/aggregate.rs:361
-- pg_search::api::aggregate::pdb::combine_agg_stub_combine_agg_stub_finalize
CREATE  FUNCTION pdb."combine_agg_stub_combine_agg_stub_finalize"(
	"this" internal /* Internal */
) RETURNS jsonb /* JsonB */
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'combine_agg_stub_combine_agg_stub_finalize_wrapper';

-- pg_search/src/api/aggregate.rs:361
-- pg_search::api::aggregate::pdb::CombineAggStub
CREATE AGGREGATE pdb.combine_agg (
	jsonb /* JsonB */
)
(
	SFUNC = pdb."combine_agg_stub_combine_agg_stub_state", /* pg_search::api::aggregate::pdb::CombineAggStub::state */
	STYPE = internal, /* Internal */
	FINALFUNC = pdb."combine_agg_stub_combine_agg_stub_finalize", /* pg_search::api::aggregate::pdb::CombineAggStub::final */
	PARALLEL = SAFE /* pg_search::api::aggregate::pdb::CombineAggStub::PARALLEL */
);
