CREATE OR REPLACE FUNCTION pdb."agg_placeholder_agg_placeholder_state"(
	"this" internal,
	"arg_one" jsonb
) RETURNS internal
LANGUAGE c
AS 'MODULE_PATHNAME', 'agg_placeholder_agg_placeholder_state_wrapper';

-- Ensure the single-arg finalize function exists
CREATE OR REPLACE FUNCTION pdb."agg_placeholder_agg_placeholder_finalize"(
	"this" internal
) RETURNS jsonb
LANGUAGE c
AS 'MODULE_PATHNAME', 'agg_placeholder_agg_placeholder_finalize_wrapper';

-- Recreate pdb.agg(jsonb) only if it doesn't already exist
DO $$
BEGIN
	IF NOT EXISTS (
		SELECT 1 FROM pg_aggregate a
		JOIN pg_proc p ON a.aggfnoid = p.oid
		JOIN pg_namespace n ON p.pronamespace = n.oid
		WHERE n.nspname = 'pdb' AND p.proname = 'agg' AND p.pronargs = 1
	) THEN
		CREATE AGGREGATE pdb.agg (jsonb) (
			SFUNC = pdb."agg_placeholder_agg_placeholder_state",
			STYPE = internal,
			FINALFUNC = pdb."agg_placeholder_agg_placeholder_finalize"
		);
	END IF;
END
$$;
