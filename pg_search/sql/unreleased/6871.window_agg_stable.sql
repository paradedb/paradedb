DROP FUNCTION IF EXISTS pdb.window_agg(window_aggregate_json text);
CREATE OR REPLACE FUNCTION pdb.window_agg(window_aggregate_json text) RETURNS pg_catalog.int8 AS 'MODULE_PATHNAME', 'window_agg_placeholder_wrapper' LANGUAGE c PARALLEL SAFE STABLE STRICT;
