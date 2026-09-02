-- Rename the `solve_mvcc` aggregate parameter to `visibility` and make it ternary
-- (#6074).
--
-- Every CREATE below is the SchemaBot/pgrx canonical text verbatim (the schema
-- checker compares statements textually); the DROPs keep the fragment re-runnable.

-- `paradedb.aggregate` gains a trailing `visibility` argument. The previous
-- six-argument signature has to go: leaving both in place would make every call
-- that relies on the defaults ambiguous. These two statements are SchemaBot's
-- emitted text verbatim, which for a replaced function differs from pgrx's
-- generated form (named parameters, `pg_catalog.int8`, `CREATE OR REPLACE`).
DROP FUNCTION IF EXISTS aggregate(index regclass, query searchqueryinput, agg json, solve_mvcc bool, memory_limit pg_catalog.int8, bucket_limit pg_catalog.int8);
CREATE OR REPLACE FUNCTION aggregate(index regclass, query searchqueryinput, agg json, solve_mvcc bool DEFAULT NULL, memory_limit pg_catalog.int8 DEFAULT '500000000', bucket_limit pg_catalog.int8 DEFAULT NULL, visibility text DEFAULT NULL) RETURNS jsonb AS 'MODULE_PATHNAME', 'aggregate_wrapper' LANGUAGE c;

-- The `pdb.agg(jsonb, text)` overload carrying the visibility mode. The existing
-- `pdb.agg(jsonb, bool)` overload is left in place: it is the deprecated
-- `solve_mvcc` spelling and existing queries still resolve to it.
DROP AGGREGATE IF EXISTS pdb.agg(jsonb, TEXT);
DROP FUNCTION IF EXISTS pdb."agg_placeholder_visibility_agg_placeholder_visibility_state"(internal, jsonb, TEXT);
DROP FUNCTION IF EXISTS pdb."agg_placeholder_visibility_agg_placeholder_visibility_finalize"(internal);
CREATE  FUNCTION pdb."agg_placeholder_visibility_agg_placeholder_visibility_state"(
	"this" internal, /* Internal */
	"arg_one" jsonb, /* JsonB */
	"arg_two" TEXT /* String */
) RETURNS internal /* Internal */
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'agg_placeholder_visibility_agg_placeholder_visibility_state_wrapper';
CREATE  FUNCTION pdb."agg_placeholder_visibility_agg_placeholder_visibility_finalize"(
	"this" internal /* Internal */
) RETURNS jsonb /* JsonB */
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'agg_placeholder_visibility_agg_placeholder_visibility_finalize_wrapper';
CREATE AGGREGATE pdb.agg (
	jsonb, /* JsonB */
	TEXT /* String */
)
(
	SFUNC = pdb."agg_placeholder_visibility_agg_placeholder_visibility_state", /* pg_search::api::aggregate::pdb::AggPlaceholderVisibility::state */
	STYPE = internal, /* Internal */
	FINALFUNC = pdb."agg_placeholder_visibility_agg_placeholder_visibility_finalize" /* pg_search::api::aggregate::pdb::AggPlaceholderVisibility::final */
);
