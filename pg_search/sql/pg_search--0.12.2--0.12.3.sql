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
