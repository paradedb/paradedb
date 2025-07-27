\echo Use "ALTER EXTENSION pg_search UPDATE TO '0.17.2'" to load this file. \quit

CREATE  FUNCTION "score"() RETURNS void
STRICT 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'score_invalid_signature_wrapper';
