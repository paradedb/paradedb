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
