DROP PROCEDURE IF EXISTS paradedb.create_bm25(index_name text, table_name text, key_field text, schema_name text, text_fields jsonb, numeric_fields jsonb, boolean_fields jsonb, json_fields jsonb, range_fields jsonb, datetime_fields jsonb, predicates text);
DROP PROCEDURE IF EXISTS paradedb.drop_bm25(index_name text, schema_name text);
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/config.rs:84
-- pg_search::api::config::format_create_index
CREATE OR REPLACE FUNCTION paradedb.format_create_index(
    index_name text DEFAULT '',
    table_name text DEFAULT '',
    key_field text DEFAULT '',
    schema_name text DEFAULT CURRENT_SCHEMA,
    text_fields jsonb DEFAULT '{}',
    numeric_fields jsonb DEFAULT '{}',
    boolean_fields jsonb DEFAULT '{}',
    json_fields jsonb DEFAULT '{}',
    range_fields jsonb DEFAULT '{}',
    datetime_fields jsonb DEFAULT '{}',
    predicates text DEFAULT ''
)
RETURNS text
LANGUAGE c AS 'MODULE_PATHNAME', 'format_create_index_wrapper';

DROP FUNCTION IF EXISTS boost(boost pg_catalog.float4, query searchqueryinput);
CREATE OR REPLACE FUNCTION boost(factor pg_catalog.float4, query searchqueryinput) RETURNS searchqueryinput AS 'MODULE_PATHNAME', 'boost_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE STRICT;

-- pg_search/src/postgres/customscan/pdbscan/projections/mod.rs:31
-- pg_search::postgres::customscan::pdbscan::projections::placeholder_support
CREATE  FUNCTION "placeholder_support"(
    "arg" internal /* pgrx::datum::internal::Internal */
) RETURNS internal /* pg_search::api::operator::ReturnedNodePointer */
    IMMUTABLE PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'placeholder_support_wrapper';

-- pg_search/src/postgres/customscan/pdbscan/projections/score.rs:30
-- requires:
--   score_from_relation
--   placeholder_support
ALTER FUNCTION score SUPPORT placeholder_support;

-- pg_search/src/postgres/customscan/pdbscan/projections/snippet.rs:48
-- requires:
--   snippet_from_relation
--   placeholder_support
ALTER FUNCTION snippet SUPPORT placeholder_support;

/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/bootstrap/create_bm25.rs:39
-- pg_search::bootstrap::create_bm25::format_create_bm25
CREATE  FUNCTION "format_create_bm25"(
	"index_name" TEXT, /* &str */
	"table_name" TEXT, /* &str */
	"key_field" TEXT, /* &str */
	"schema_name" TEXT DEFAULT '', /* &str */
	"text_fields" jsonb DEFAULT '{}'::jsonb, /* pgrx::datum::json::JsonB */
	"numeric_fields" jsonb DEFAULT '{}'::jsonb, /* pgrx::datum::json::JsonB */
	"boolean_fields" jsonb DEFAULT '{}'::jsonb, /* pgrx::datum::json::JsonB */
	"json_fields" jsonb DEFAULT '{}'::jsonb, /* pgrx::datum::json::JsonB */
	"range_fields" jsonb DEFAULT '{}'::jsonb, /* pgrx::datum::json::JsonB */
	"datetime_fields" jsonb DEFAULT '{}'::jsonb, /* pgrx::datum::json::JsonB */
	"predicates" TEXT DEFAULT '' /* &str */
) RETURNS TEXT /* core::result::Result<alloc::string::String, anyhow::Error> */
STRICT
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'format_create_bm25_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/bootstrap/create_bm25.rs:134
-- pg_search::bootstrap::create_bm25::index_fields
CREATE  FUNCTION "index_fields"(
	"index" regclass /* pgrx::rel::PgRelation */
) RETURNS jsonb /* pgrx::datum::json::JsonB */
STRICT
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'index_fields_wrapper';
