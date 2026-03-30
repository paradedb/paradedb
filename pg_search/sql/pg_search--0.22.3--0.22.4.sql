-- Fix: Restore pdb.agg(jsonb) overload that was inadvertently dropped during
-- the 0.21.15→0.21.16 migration. The CASCADE on DROP FUNCTION for the
-- agg_placeholder_with_mvcc finalize function also dropped the agg(jsonb)
-- aggregate (which depended on it after 0.20.1→0.20.2 rewired it).

-- Ensure the single-argument state/finalize functions exist
CREATE OR REPLACE FUNCTION pdb."agg_placeholder_agg_placeholder_state"(
	"this" internal,
	"arg_one" jsonb
) RETURNS internal
LANGUAGE c
AS 'MODULE_PATHNAME', 'agg_placeholder_agg_placeholder_state_wrapper';

CREATE OR REPLACE FUNCTION pdb."agg_placeholder_agg_placeholder_finalize"(
	"this" internal
) RETURNS jsonb
LANGUAGE c
AS 'MODULE_PATHNAME', 'agg_placeholder_agg_placeholder_finalize_wrapper';

-- Recreate the single-argument aggregate (idempotent: DROP IF EXISTS first)
DROP AGGREGATE IF EXISTS pdb.agg(jsonb);
CREATE AGGREGATE pdb.agg (
	jsonb
)
(
	SFUNC = pdb."agg_placeholder_agg_placeholder_state",
	STYPE = internal,
	FINALFUNC = pdb."agg_placeholder_agg_placeholder_finalize"
);
