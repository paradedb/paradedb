ALTER TYPE TestTable ADD VALUE 'Customers';

-- pg_search/src/api/index.rs:580
-- pg_search::api::index::term
CREATE  FUNCTION "term"(
	"field" FieldName, /* pg_search::api::index::FieldName */
	"value" anyenum /* pg_search::schema::anyenum::AnyEnum */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'term_anyenum_wrapper';

DROP PROCEDURE IF EXISTS paradedb.create_bm25(index_name text, table_name text, key_field text, schema_name text, text_fields jsonb, numeric_fields jsonb, boolean_fields jsonb, json_fields jsonb, range_fields jsonb, datetime_fields jsonb, predicates text);
DROP PROCEDURE IF EXISTS paradedb.drop_bm25(index_name text, schema_name text);
DROP FUNCTION IF EXISTS field(name text, indexed bool, stored bool, fast bool, fieldnorms bool, record text, expand_dots bool, tokenizer jsonb, normalizer text);
