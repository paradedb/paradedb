\echo Use "ALTER EXTENSION pg_search UPDATE TO '0.26.0-rc.1'" to load this file. \quit

-- ============================================================================
-- Fragment: 5903.rename_test_table_proc.sql
-- ============================================================================
DROP PROCEDURE IF EXISTS paradedb.create_bm25_test_table(table_name pg_catalog."varchar", schema_name pg_catalog."varchar", table_type paradedb.testtable);
CREATE OR REPLACE PROCEDURE paradedb.create_paradedb_test_table(table_name VARCHAR DEFAULT 'bm25_test_table', schema_name VARCHAR DEFAULT 'paradedb', table_type paradedb.TestTable DEFAULT 'Items')
LANGUAGE c AS 'MODULE_PATHNAME', 'create_paradedb_test_table_wrapper';


-- ============================================================================
-- Fragment: 6099.rename_solve_mvcc_to_visibility.sql
-- ============================================================================
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


-- ============================================================================
-- Fragment: 6221.sequential_scan_ctids.sql
-- ============================================================================
CREATE FUNCTION "search_with_query_input_ctid_strict"(
    "element" anyelement,
    "query" SearchQueryInput,
    "ctid" tid,
    "original_lhs" record DEFAULT ROW()
) RETURNS bool
IMMUTABLE STRICT PARALLEL SAFE COST 1000000000
LANGUAGE c
AS 'MODULE_PATHNAME', 'search_with_query_input_ctid_strict_wrapper';

CREATE FUNCTION "search_with_query_input_ctid"(
    "element" anyelement,
    "query" SearchQueryInput,
    "ctid" tid,
    "original_lhs" record DEFAULT ROW()
) RETURNS bool
IMMUTABLE PARALLEL SAFE COST 1000000000
LANGUAGE c
AS 'MODULE_PATHNAME', 'search_with_query_input_ctid_wrapper';

ALTER FUNCTION paradedb.search_with_query_input_ctid SUPPORT paradedb.query_input_support;
ALTER FUNCTION paradedb.search_with_query_input_ctid_strict SUPPORT paradedb.query_input_support;


-- ============================================================================
-- Fragment: 6222.inline_row_evaluation.sql
-- ============================================================================
CREATE FUNCTION "ctid_is_valid"(
    "ctid" tid
) RETURNS bool
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'ctid_is_valid_wrapper';

CREATE FUNCTION "xmin_is_visible"(
    "xmin" xid
) RETURNS bool
STRICT STABLE PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'xmin_is_visible_wrapper';

CREATE FUNCTION "search_with_query_input_ctid_or_row_strict"(
    "element" anyelement,
    "query" SearchQueryInput,
    "ctid" tid,
    "fallback_row" record[],
    "original_lhs" record DEFAULT ROW()
) RETURNS bool
IMMUTABLE STRICT PARALLEL SAFE COST 1000000000
LANGUAGE c
AS 'MODULE_PATHNAME', 'search_with_query_input_ctid_or_row_strict_wrapper';

CREATE FUNCTION "search_with_query_input_ctid_or_row"(
    "element" anyelement,
    "query" SearchQueryInput,
    "ctid" tid,
    "fallback_row" record[],
    "original_lhs" record DEFAULT ROW()
) RETURNS bool
IMMUTABLE PARALLEL SAFE COST 1000000000
LANGUAGE c
AS 'MODULE_PATHNAME', 'search_with_query_input_ctid_or_row_wrapper';

ALTER FUNCTION paradedb.search_with_query_input_ctid_or_row SUPPORT paradedb.query_input_support;
ALTER FUNCTION paradedb.search_with_query_input_ctid_or_row_strict SUPPORT paradedb.query_input_support;


-- ============================================================================
-- Fragment: 6225.unfielded_query_input.sql
-- ============================================================================
CREATE FUNCTION "to_search_query_input"(
    "query" pdb.Query
) RETURNS SearchQueryInput
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'to_search_query_input_unfielded_wrapper';


-- ============================================================================
-- Fragment: 6226.field_bound_more_like_this.sql
-- ============================================================================
-- The return type changes to pdb.query. Do not cascade: dependent objects
-- must be migrated explicitly rather than silently removed during an upgrade.
DROP FUNCTION IF EXISTS pdb.more_like_this(key_value anyelement, fields text[], min_doc_frequency pg_catalog.int4, max_doc_frequency pg_catalog.int4, min_term_frequency pg_catalog.int4, max_query_terms pg_catalog.int4, min_word_length pg_catalog.int4, max_word_length pg_catalog.int4, boost_factor pg_catalog.float4, stopwords text[]);
CREATE OR REPLACE FUNCTION pdb.more_like_this(key_value anyelement, fields text[] DEFAULT NULL, min_doc_frequency pg_catalog.int4 DEFAULT NULL, max_doc_frequency pg_catalog.int4 DEFAULT NULL, min_term_frequency pg_catalog.int4 DEFAULT NULL, max_query_terms pg_catalog.int4 DEFAULT NULL, min_word_length pg_catalog.int4 DEFAULT NULL, max_word_length pg_catalog.int4 DEFAULT NULL, boost_factor pg_catalog.float4 DEFAULT NULL, stopwords text[] DEFAULT NULL) RETURNS pdb.query AS 'MODULE_PATHNAME', 'more_like_this_id_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE;


-- ============================================================================
-- Fragment: 6284.columnar_index_info.sql
-- ============================================================================
-- Use the public "columnar" terminology for index component sizes. Drop the
-- dependent compatibility views first, then recreate them below.
DROP VIEW IF EXISTS pdb.index_layer_info;
DROP VIEW IF EXISTS paradedb.index_layer_info;
DROP FUNCTION IF EXISTS index_info(index regclass, show_invisible bool);
CREATE OR REPLACE FUNCTION index_info(index regclass, show_invisible bool DEFAULT 'false') RETURNS TABLE(index_name text, visible bool, recyclable bool, xmax xid, segno text, mutable bool, byte_size pg_catalog."numeric", num_docs pg_catalog."numeric", num_deleted pg_catalog."numeric", termdict_bytes pg_catalog."numeric", postings_bytes pg_catalog."numeric", positions_bytes pg_catalog."numeric", columnar_bytes pg_catalog."numeric", fieldnorms_bytes pg_catalog."numeric", store_bytes pg_catalog."numeric", deletes_bytes pg_catalog."numeric") AS 'MODULE_PATHNAME', 'index_info_wrapper' LANGUAGE c STRICT;

CREATE VIEW pdb.index_layer_info AS SELECT ((relname)::text), layer_size, low, high, byte_size, CASE WHEN (segments = ARRAY[NULL]) THEN 0 ELSE count END AS count, CASE WHEN (segments = ARRAY[NULL]) THEN NULL ELSE segments END AS segments FROM (SELECT relname, ((COALESCE (pg_size_pretty(CASE WHEN (low = 0) THEN NULL ELSE low END), '') || '..') || COALESCE (pg_size_pretty(CASE WHEN (high = 9223372036854775807) THEN NULL ELSE high END), '')) AS layer_size, count(*), COALESCE (sum(byte_size), 0) AS byte_size, min(low) AS low, max(high) AS high, array_agg(segno) AS segments FROM (WITH indexes AS (SELECT ((c.oid)::regclass) AS relname FROM pg_class AS c INNER JOIN pg_index AS i ON (i.indexrelid = c.oid) WHERE (c.relam IN (SELECT oid FROM pg_am WHERE (amhandler = (('paradedb.bm25_handler')::regproc))) AND i.indisvalid AND i.indisready AND i.indislive)) , segments AS (SELECT relname, index_info.* FROM indexes INNER JOIN paradedb.index_info(indexes.relname, (('t')::pg_catalog.bool)) ON (('t')::pg_catalog.bool)) , layer_sizes AS (SELECT relname, COALESCE (lead(unnest) OVER(), 0) AS low, unnest AS high FROM indexes INNER JOIN LATERAL (SELECT unnest(((0 || paradedb.combined_layer_sizes(indexes.relname)) || 9223372036854775807)) ORDER BY 1 DESC ) AS x ON (('t')::pg_catalog.bool)) SELECT layer_sizes.relname, layer_sizes.low, layer_sizes.high, segments.segno, segments.byte_size FROM layer_sizes LEFT JOIN segments ON ((layer_sizes.relname = segments.relname) AND ((((byte_size * 1.33))::pg_catalog.int8) BETWEEN low AND high))) AS x WHERE (low < high) GROUP BY relname, low, high ORDER BY relname , low DESC ) AS x;
GRANT SELECT ON pdb.index_layer_info TO PUBLIC;

CREATE VIEW paradedb.index_layer_info AS SELECT ((relname)::text), layer_size, low, high, byte_size, CASE WHEN (segments = ARRAY[NULL]) THEN 0 ELSE count END AS count, CASE WHEN (segments = ARRAY[NULL]) THEN NULL ELSE segments END AS segments FROM (SELECT relname, ((COALESCE (pg_size_pretty(CASE WHEN (low = 0) THEN NULL ELSE low END), '') || '..') || COALESCE (pg_size_pretty(CASE WHEN (high = 9223372036854775807) THEN NULL ELSE high END), '')) AS layer_size, count(*), COALESCE (sum(byte_size), 0) AS byte_size, min(low) AS low, max(high) AS high, array_agg(segno) AS segments FROM (WITH indexes AS (SELECT ((c.oid)::regclass) AS relname FROM pg_class AS c INNER JOIN pg_index AS i ON (i.indexrelid = c.oid) WHERE (c.relam IN (SELECT oid FROM pg_am WHERE (amhandler = (('paradedb.bm25_handler')::regproc))) AND i.indisvalid AND i.indisready AND i.indislive)) , segments AS (SELECT relname, index_info.* FROM indexes INNER JOIN paradedb.index_info(indexes.relname, (('t')::pg_catalog.bool)) ON (('t')::pg_catalog.bool)) , layer_sizes AS (SELECT relname, COALESCE (lead(unnest) OVER(), 0) AS low, unnest AS high FROM indexes INNER JOIN LATERAL (SELECT unnest(((0 || paradedb.layer_sizes(indexes.relname)) || 9223372036854775807)) ORDER BY 1 DESC ) AS x ON (('t')::pg_catalog.bool)) SELECT layer_sizes.relname, layer_sizes.low, layer_sizes.high, segments.segno, segments.byte_size FROM layer_sizes LEFT JOIN segments ON ((layer_sizes.relname = segments.relname) AND ((((byte_size * 1.33))::pg_catalog.int8) BETWEEN low AND high))) AS x WHERE (low < high) GROUP BY relname, low, high ORDER BY relname , low DESC ) AS x;
GRANT SELECT ON paradedb.index_layer_info TO PUBLIC;


-- ============================================================================
-- Fragment: 6269.make_pdb_agg_visibility_parallel_safe.sql
-- ============================================================================
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

