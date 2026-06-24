\echo Use "ALTER EXTENSION pg_search UPDATE TO '0.24.2'" to load this file. \quit

-- Update boolean() overloads to add minimum_should_match parameter
DROP FUNCTION IF EXISTS "boolean"(must searchqueryinput, should searchqueryinput, must_not searchqueryinput);
CREATE OR REPLACE FUNCTION "boolean"(must searchqueryinput DEFAULT NULL, should searchqueryinput DEFAULT NULL, must_not searchqueryinput DEFAULT NULL, minimum_should_match pg_catalog.int8 DEFAULT NULL) RETURNS searchqueryinput AS 'MODULE_PATHNAME', 'boolean_singles_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE;
DROP FUNCTION IF EXISTS "boolean"(must searchqueryinput[], should searchqueryinput[], must_not searchqueryinput[]);
CREATE OR REPLACE FUNCTION "boolean"(must searchqueryinput[] DEFAULT ARRAY[]::searchqueryinput[], should searchqueryinput[] DEFAULT ARRAY[]::searchqueryinput[], must_not searchqueryinput[] DEFAULT ARRAY[]::searchqueryinput[], minimum_should_match pg_catalog.int8 DEFAULT NULL) RETURNS searchqueryinput AS 'MODULE_PATHNAME', 'boolean_arrays_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE;
