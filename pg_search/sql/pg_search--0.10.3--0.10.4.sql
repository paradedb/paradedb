/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/index.rs:621
-- pg_search::api::index::FieldName
CREATE TYPE FieldName;
-- pg_search/src/api/index.rs:621
-- pg_search::api::index::fieldname_in
CREATE  FUNCTION "fieldname_in"(
    "input" cstring /* core::option::Option<&core::ffi::c_str::CStr> */
) RETURNS FieldName /* core::option::Option<pg_search::api::index::FieldName> */
    IMMUTABLE PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'fieldname_in_wrapper';
-- pg_search/src/api/index.rs:621
-- pg_search::api::index::fieldname_out
CREATE  FUNCTION "fieldname_out"(
    "input" FieldName /* pg_search::api::index::FieldName */
) RETURNS cstring /* alloc::ffi::c_str::CString */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'fieldname_out_wrapper';
CREATE TYPE FieldName (
  INTERNALLENGTH = variable,
  INPUT = fieldname_in, /* pg_search::api::index::fieldname_in */
  OUTPUT = fieldname_out, /* pg_search::api::index::fieldname_out */
  STORAGE = extended
);
DROP FUNCTION IF EXISTS "exists"(field text);
CREATE OR REPLACE FUNCTION "exists"(field fieldname) RETURNS searchqueryinput AS 'MODULE_PATHNAME', 'exists_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE STRICT;
DROP FUNCTION IF EXISTS fuzzy_phrase(field text, value text, distance pg_catalog.int4, transposition_cost_one bool, prefix bool, match_all_terms bool);
CREATE OR REPLACE FUNCTION fuzzy_phrase(field fieldname, value text DEFAULT NULL, distance pg_catalog.int4 DEFAULT NULL, transposition_cost_one bool DEFAULT NULL, prefix bool DEFAULT NULL, match_all_terms bool DEFAULT NULL) RETURNS searchqueryinput AS 'MODULE_PATHNAME', 'fuzzy_phrase_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE;
DROP FUNCTION IF EXISTS fuzzy_term(field text, value text, distance pg_catalog.int4, transposition_cost_one bool, prefix bool);
CREATE OR REPLACE FUNCTION fuzzy_term(field fieldname, value text DEFAULT NULL, distance pg_catalog.int4 DEFAULT NULL, transposition_cost_one bool DEFAULT NULL, prefix bool DEFAULT NULL) RETURNS searchqueryinput AS 'MODULE_PATHNAME', 'fuzzy_term_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE;
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/index.rs:341
-- pg_search::api::index::parse_with_field
CREATE  FUNCTION "parse_with_field"(
    "field" FieldName, /* pg_search::api::index::FieldName */
    "query_string" TEXT /* alloc::string::String */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'parse_with_field_wrapper';
DROP FUNCTION IF EXISTS phrase(field text, phrases text[], slop pg_catalog.int4);
CREATE OR REPLACE FUNCTION phrase(field fieldname, phrases text[], slop pg_catalog.int4 DEFAULT NULL) RETURNS searchqueryinput AS 'MODULE_PATHNAME', 'phrase_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE;
DROP FUNCTION IF EXISTS phrase_prefix(field text, phrases text[], max_expansion pg_catalog.int4);
CREATE OR REPLACE FUNCTION phrase_prefix(field fieldname, phrases text[], max_expansion pg_catalog.int4 DEFAULT NULL) RETURNS searchqueryinput AS 'MODULE_PATHNAME', 'phrase_prefix_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE;
DROP FUNCTION IF EXISTS range(field text, range daterange);
CREATE OR REPLACE FUNCTION range(field fieldname, range daterange) RETURNS searchqueryinput AS 'MODULE_PATHNAME', 'range_date_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE STRICT;
DROP FUNCTION IF EXISTS range(field text, range int4range);
CREATE OR REPLACE FUNCTION range(field fieldname, range int4range) RETURNS searchqueryinput AS 'MODULE_PATHNAME', 'range_i32_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE STRICT;
DROP FUNCTION IF EXISTS range(field text, range int8range);
CREATE OR REPLACE FUNCTION range(field fieldname, range int8range) RETURNS searchqueryinput AS 'MODULE_PATHNAME', 'range_i64_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE STRICT;
DROP FUNCTION IF EXISTS range(field text, range numrange);
CREATE OR REPLACE FUNCTION range(field fieldname, range numrange) RETURNS searchqueryinput AS 'MODULE_PATHNAME', 'range_numeric_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE STRICT;
DROP FUNCTION IF EXISTS range(field text, range tsrange);
CREATE OR REPLACE FUNCTION range(field fieldname, range tsrange) RETURNS searchqueryinput AS 'MODULE_PATHNAME', 'range_timestamp_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE STRICT;
DROP FUNCTION IF EXISTS range(field text, range tstzrange);
CREATE OR REPLACE FUNCTION range(field fieldname, range tstzrange) RETURNS searchqueryinput AS 'MODULE_PATHNAME', 'range_timestamptz_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE STRICT;
DROP FUNCTION IF EXISTS regex(field text, pattern text);
CREATE OR REPLACE FUNCTION regex(field fieldname, pattern text) RETURNS searchqueryinput AS 'MODULE_PATHNAME', 'regex_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE STRICT;
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/postgres/customscan/pdbscan/projections/score.rs:23
-- pg_search::postgres::customscan::pdbscan::projections::score::score
CREATE  FUNCTION "score"(
    "_relation_reference" anyelement /* pgrx::datum::anyelement::AnyElement */
) RETURNS real /* f32 */
    STRICT STABLE PARALLEL SAFE  COST 1
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'score_from_relation_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/postgres/customscan/pdbscan/projections/snippet.rs:40
-- pg_search::postgres::customscan::pdbscan::projections::snippet::snippet
CREATE  FUNCTION "snippet"(
    "field" anyelement, /* pgrx::datum::anyelement::AnyElement */
    "start_tag" TEXT DEFAULT '<b>', /* alloc::string::String */
    "end_tag" TEXT DEFAULT '</b>', /* alloc::string::String */
    "max_num_chars" INT DEFAULT 150 /* i32 */
) RETURNS TEXT /* core::option::Option<alloc::string::String> */
    STRICT STABLE PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'snippet_from_relation_wrapper';
DROP FUNCTION IF EXISTS term(field text, value anyarray);
CREATE OR REPLACE FUNCTION term(field fieldname, value anyarray DEFAULT NULL) RETURNS searchqueryinput AS 'MODULE_PATHNAME', 'anyarray_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE;
DROP FUNCTION IF EXISTS term(field text, value date);
CREATE OR REPLACE FUNCTION term(field fieldname DEFAULT NULL, value date DEFAULT NULL) RETURNS searchqueryinput AS 'MODULE_PATHNAME', 'date_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE;
DROP FUNCTION IF EXISTS term(field text, value daterange);
CREATE OR REPLACE FUNCTION term(field fieldname, value daterange DEFAULT NULL) RETURNS searchqueryinput AS 'MODULE_PATHNAME', 'daterange_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE;
DROP FUNCTION IF EXISTS term(field text, value inet);
CREATE OR REPLACE FUNCTION term(field fieldname, value inet DEFAULT NULL) RETURNS searchqueryinput AS 'MODULE_PATHNAME', 'inet_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE;
DROP FUNCTION IF EXISTS term(field text, value int4range);
CREATE OR REPLACE FUNCTION term(field fieldname, value int4range DEFAULT NULL) RETURNS searchqueryinput AS 'MODULE_PATHNAME', 'int4range_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE;
DROP FUNCTION IF EXISTS term(field text, value int8range);
CREATE OR REPLACE FUNCTION term(field fieldname, value int8range DEFAULT NULL) RETURNS searchqueryinput AS 'MODULE_PATHNAME', 'int8range_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE;
DROP FUNCTION IF EXISTS term(field text, value json);
CREATE OR REPLACE FUNCTION term(field fieldname, value json DEFAULT NULL) RETURNS searchqueryinput AS 'MODULE_PATHNAME', 'json_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE;
DROP FUNCTION IF EXISTS term(field text, value jsonb);
CREATE OR REPLACE FUNCTION term(field fieldname, value jsonb DEFAULT NULL) RETURNS searchqueryinput AS 'MODULE_PATHNAME', 'jsonb_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE;
DROP FUNCTION IF EXISTS term(field text, value pg_catalog."numeric");
CREATE OR REPLACE FUNCTION term(field fieldname DEFAULT NULL, value pg_catalog."numeric" DEFAULT NULL) RETURNS searchqueryinput AS 'MODULE_PATHNAME', 'numeric_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE;
DROP FUNCTION IF EXISTS term(field text, value numrange);
CREATE OR REPLACE FUNCTION term(field fieldname, value numrange DEFAULT NULL) RETURNS searchqueryinput AS 'MODULE_PATHNAME', 'numrange_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE;
DROP FUNCTION IF EXISTS term(field text, value box);
CREATE OR REPLACE FUNCTION term(field fieldname, value box DEFAULT NULL) RETURNS searchqueryinput AS 'MODULE_PATHNAME', 'pg_box_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE;
DROP FUNCTION IF EXISTS term(field text, value point);
CREATE OR REPLACE FUNCTION term(field fieldname, value point DEFAULT NULL) RETURNS searchqueryinput AS 'MODULE_PATHNAME', 'point_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE;
DROP FUNCTION IF EXISTS term(field text, value bool);
CREATE OR REPLACE FUNCTION term(field fieldname DEFAULT NULL, value bool DEFAULT NULL) RETURNS searchqueryinput AS 'MODULE_PATHNAME', 'term_bool_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE;
DROP FUNCTION IF EXISTS term(field text, value bytea);
CREATE OR REPLACE FUNCTION term(field fieldname DEFAULT NULL, value bytea DEFAULT NULL) RETURNS searchqueryinput AS 'MODULE_PATHNAME', 'term_bytes_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE;
DROP FUNCTION IF EXISTS term(field text, value pg_catalog.float4);
CREATE OR REPLACE FUNCTION term(field fieldname DEFAULT NULL, value pg_catalog.float4 DEFAULT NULL) RETURNS searchqueryinput AS 'MODULE_PATHNAME', 'term_f32_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE;
DROP FUNCTION IF EXISTS term(field text, value pg_catalog.float8);
CREATE OR REPLACE FUNCTION term(field fieldname DEFAULT NULL, value pg_catalog.float8 DEFAULT NULL) RETURNS searchqueryinput AS 'MODULE_PATHNAME', 'term_f64_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE;
DROP FUNCTION IF EXISTS term(field text, value pg_catalog.int2);
CREATE OR REPLACE FUNCTION term(field fieldname DEFAULT NULL, value pg_catalog.int2 DEFAULT NULL) RETURNS searchqueryinput AS 'MODULE_PATHNAME', 'term_i16_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE;
DROP FUNCTION IF EXISTS term(field text, value pg_catalog.int4);
CREATE OR REPLACE FUNCTION term(field fieldname DEFAULT NULL, value pg_catalog.int4 DEFAULT NULL) RETURNS searchqueryinput AS 'MODULE_PATHNAME', 'term_i32_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE;
DROP FUNCTION IF EXISTS term(field text, value pg_catalog.int8);
CREATE OR REPLACE FUNCTION term(field fieldname DEFAULT NULL, value pg_catalog.int8 DEFAULT NULL) RETURNS searchqueryinput AS 'MODULE_PATHNAME', 'term_i64_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE;
DROP FUNCTION IF EXISTS term(field text, value "char");
CREATE OR REPLACE FUNCTION term(field fieldname DEFAULT NULL, value "char" DEFAULT NULL) RETURNS searchqueryinput AS 'MODULE_PATHNAME', 'term_i8_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE;
DROP FUNCTION IF EXISTS term(field text, value text);
CREATE OR REPLACE FUNCTION term(field fieldname DEFAULT NULL, value text DEFAULT NULL) RETURNS searchqueryinput AS 'MODULE_PATHNAME', 'term_str_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE;
DROP FUNCTION IF EXISTS term(field text, value tid);
CREATE OR REPLACE FUNCTION term(field fieldname, value tid DEFAULT NULL) RETURNS searchqueryinput AS 'MODULE_PATHNAME', 'tid_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE;
DROP FUNCTION IF EXISTS term(field text, value pg_catalog."time");
CREATE OR REPLACE FUNCTION term(field fieldname DEFAULT NULL, value pg_catalog."time" DEFAULT NULL) RETURNS searchqueryinput AS 'MODULE_PATHNAME', 'time_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE;
DROP FUNCTION IF EXISTS term(field text, value pg_catalog.timetz);
CREATE OR REPLACE FUNCTION term(field fieldname DEFAULT NULL, value pg_catalog.timetz DEFAULT NULL) RETURNS searchqueryinput AS 'MODULE_PATHNAME', 'time_with_time_zone_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE;
DROP FUNCTION IF EXISTS term(field text, value pg_catalog."timestamp");
CREATE OR REPLACE FUNCTION term(field fieldname DEFAULT NULL, value pg_catalog."timestamp" DEFAULT NULL) RETURNS searchqueryinput AS 'MODULE_PATHNAME', 'timestamp_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE;
DROP FUNCTION IF EXISTS term(field text, value pg_catalog.timestamptz);
CREATE OR REPLACE FUNCTION term(field fieldname DEFAULT NULL, value pg_catalog.timestamptz DEFAULT NULL) RETURNS searchqueryinput AS 'MODULE_PATHNAME', 'timestamp_with_time_zome_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE;
DROP FUNCTION IF EXISTS term(field text, value tsrange);
CREATE OR REPLACE FUNCTION term(field fieldname, value tsrange DEFAULT NULL) RETURNS searchqueryinput AS 'MODULE_PATHNAME', 'tsrange_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE;
DROP FUNCTION IF EXISTS term(field text, value tstzrange);
CREATE OR REPLACE FUNCTION term(field fieldname, value tstzrange DEFAULT NULL) RETURNS searchqueryinput AS 'MODULE_PATHNAME', 'tstzrange_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE;
DROP FUNCTION IF EXISTS term(field text, value uuid);
CREATE OR REPLACE FUNCTION term(field fieldname DEFAULT NULL, value uuid DEFAULT NULL) RETURNS searchqueryinput AS 'MODULE_PATHNAME', 'uuid_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE;
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/index.rs:664
-- pg_search::api::index::text_to_fieldname
CREATE  FUNCTION "text_to_fieldname"(
    "field" TEXT /* alloc::string::String */
) RETURNS FieldName /* pg_search::api::index::FieldName */
    STRICT
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'text_to_fieldname_wrapper';
-- pg_search/src/api/index.rs:664
-- pg_search::api::index::text_to_fieldname
CREATE CAST (
    TEXT /* alloc::string::String */
    AS
    FieldName /* pg_search::api::index::FieldName */
    )
    WITH FUNCTION text_to_fieldname AS IMPLICIT;
DROP FUNCTION IF EXISTS tokenize(tokenizer_setting jsonb, input_text text);
CREATE OR REPLACE FUNCTION tokenize(tokenizer_setting jsonb, input_text text) RETURNS TABLE(token text, "position" pg_catalog.int4) AS 'MODULE_PATHNAME', 'tokenize_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE STRICT;
DROP FUNCTION IF EXISTS tokenizers();
CREATE OR REPLACE FUNCTION tokenizers() RETURNS TABLE(tokenizer text) AS 'MODULE_PATHNAME', 'tokenizers_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE STRICT;
-- pg_search/src/bootstrap/create_bm25.rs:482
-- pg_search::bootstrap::create_bm25::index_info
CREATE  FUNCTION "index_info"(
    "index" regclass /* pgrx::rel::PgRelation */
) RETURNS TABLE (
                    "segno" TEXT,  /* alloc::string::String */
                    "byte_size" bigint,  /* i64 */
                    "num_docs" bigint,  /* i64 */
                    "num_deleted" bigint  /* i64 */
                )
    STRICT
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'index_info_wrapper';

ALTER TYPE paradedb.TestTable ADD VALUE 'Deliveries';

DROP PROCEDURE IF EXISTS paradedb.create_bm25(index_name text, table_name text, key_field text, schema_name text, text_fields jsonb, numeric_fields jsonb, boolean_fields jsonb, json_fields jsonb, datetime_fields jsonb, predicates text);
CREATE OR REPLACE PROCEDURE paradedb.create_bm25(index_name text DEFAULT '', table_name text DEFAULT '', key_field text DEFAULT '', schema_name text DEFAULT 'current_schema', text_fields jsonb DEFAULT '{}', numeric_fields jsonb DEFAULT '{}', boolean_fields jsonb DEFAULT '{}', json_fields jsonb DEFAULT '{}', range_fields jsonb DEFAULT '{}', datetime_fields jsonb DEFAULT '{}', predicates text DEFAULT '') AS 'MODULE_PATHNAME', 'create_bm25_jsonb_wrapper' LANGUAGE c;
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/index.rs:670
-- pg_search::api::index::range_term
CREATE  FUNCTION "range_term"(
	"field" FieldName, /* pg_search::api::index::FieldName */
	"term" date /* pgrx::datum::date::Date */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'range_term_date_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/index.rs:667
-- pg_search::api::index::range_term
CREATE  FUNCTION "range_term"(
	"field" FieldName, /* pg_search::api::index::FieldName */
	"term" real /* f32 */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'range_term_f32_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/index.rs:668
-- pg_search::api::index::range_term
CREATE  FUNCTION "range_term"(
	"field" FieldName, /* pg_search::api::index::FieldName */
	"term" double precision /* f64 */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'range_term_f64_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/index.rs:664
-- pg_search::api::index::range_term
CREATE  FUNCTION "range_term"(
	"field" FieldName, /* pg_search::api::index::FieldName */
	"term" smallint /* i16 */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'range_term_i16_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/index.rs:665
-- pg_search::api::index::range_term
CREATE  FUNCTION "range_term"(
	"field" FieldName, /* pg_search::api::index::FieldName */
	"term" INT /* i32 */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'range_term_i32_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/index.rs:666
-- pg_search::api::index::range_term
CREATE  FUNCTION "range_term"(
	"field" FieldName, /* pg_search::api::index::FieldName */
	"term" bigint /* i64 */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'range_term_i64_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/index.rs:663
-- pg_search::api::index::range_term
CREATE  FUNCTION "range_term"(
	"field" FieldName, /* pg_search::api::index::FieldName */
	"term" "char" /* i8 */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'range_term_i8_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/index.rs:669
-- pg_search::api::index::range_term
CREATE  FUNCTION "range_term"(
	"field" FieldName, /* pg_search::api::index::FieldName */
	"term" NUMERIC /* pgrx::datum::numeric::AnyNumeric */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'range_term_numeric_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/index.rs:671
-- pg_search::api::index::range_term
CREATE  FUNCTION "range_term"(
	"field" FieldName, /* pg_search::api::index::FieldName */
	"term" timestamp /* pgrx::datum::time_stamp::Timestamp */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'range_term_timestamp_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/index.rs:672
-- pg_search::api::index::range_term
CREATE  FUNCTION "range_term"(
	"field" FieldName, /* pg_search::api::index::FieldName */
	"term" timestamp with time zone /* pgrx::datum::time_stamp_with_timezone::TimestampWithTimeZone */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'range_term_timestamp_with_time_zone_wrapper';

DROP FUNCTION IF EXISTS parse(query_string text);
CREATE OR REPLACE FUNCTION parse(query_string text, lenient bool DEFAULT NULL, conjunction_mode bool DEFAULT NULL) RETURNS searchqueryinput AS 'MODULE_PATHNAME', 'parse_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE;
DROP FUNCTION IF EXISTS parse_with_field(field fieldname, query_string text);
CREATE OR REPLACE FUNCTION parse_with_field(field fieldname, query_string text, lenient bool DEFAULT NULL, conjunction_mode bool DEFAULT NULL) RETURNS searchqueryinput AS 'MODULE_PATHNAME', 'parse_with_field_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE;

ALTER FUNCTION search_with_query_input COST 1000000000;
ALTER FUNCTION search_with_search_config COST 1000000000;
ALTER FUNCTION search_with_text COST 1000000000;

DROP FUNCTION IF EXISTS paradedb.index_size_impl(index_name text);
DROP FUNCTION IF EXISTS schema_bm25(index_name text);
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/bootstrap/create_bm25.rs:278
-- pg_search::bootstrap::create_bm25::index_size
CREATE  FUNCTION "index_size"(
	"index" regclass /* pgrx::rel::PgRelation */
) RETURNS bigint /* core::result::Result<i64, anyhow::Error> */
STRICT
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'index_size_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/index.rs:36
-- pg_search::api::index::schema
CREATE  FUNCTION "schema"(
	"index" regclass /* pgrx::rel::PgRelation */
) RETURNS TABLE (
	"name" TEXT,  /* alloc::string::String */
	"field_type" TEXT,  /* alloc::string::String */
	"stored" bool,  /* bool */
	"indexed" bool,  /* bool */
	"fast" bool,  /* bool */
	"fieldnorms" bool,  /* bool */
	"expand_dots" bool,  /* core::option::Option<bool> */
	"tokenizer" TEXT,  /* core::option::Option<alloc::string::String> */
	"record" TEXT,  /* core::option::Option<alloc::string::String> */
	"normalizer" TEXT  /* core::option::Option<alloc::string::String> */
)
STRICT
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'schema_wrapper';
DROP FUNCTION IF EXISTS search_with_query_input(_element anyelement, query searchqueryinput);
CREATE OR REPLACE FUNCTION search_with_query_input(_element anyelement, query searchqueryinput) RETURNS bool AS 'MODULE_PATHNAME', 'search_with_query_input_wrapper' COST 1000000000 IMMUTABLE LANGUAGE c PARALLEL SAFE STRICT;
DROP FUNCTION IF EXISTS search_with_search_config(element anyelement, config_json jsonb);
CREATE OR REPLACE FUNCTION search_with_search_config(element anyelement, config_json jsonb) RETURNS bool AS 'MODULE_PATHNAME', 'search_with_search_config_wrapper' COST 1000000000 IMMUTABLE LANGUAGE c PARALLEL SAFE STRICT;
DROP FUNCTION IF EXISTS search_with_text(_element anyelement, query text);
CREATE OR REPLACE FUNCTION search_with_text(_element anyelement, query text) RETURNS bool AS 'MODULE_PATHNAME', 'search_with_text_wrapper' COST 1000000000 IMMUTABLE LANGUAGE c PARALLEL SAFE STRICT;

/* <begin connected objects> */
/*
This file is auto generated by pgrx.
The ordering of items is not stable, it is driven by a dependency graph.
*/
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/index.rs:693
-- pg_search::api::index::RangeRelation
CREATE TYPE RangeRelation AS ENUM (
	'Intersects',
	'Contains',
	'Within'
);
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/index.rs:795
-- pg_search::api::index::range_term
CREATE  FUNCTION "range_term"(
	"field" FieldName, /* pg_search::api::index::FieldName */
	"range" daterange, /* pgrx::datum::range::Range<pgrx::datum::date::Date> */
	"relation" RangeRelation /* pg_search::api::index::RangeRelation */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'range_term_range_daterange_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/index.rs:777
-- pg_search::api::index::range_term
CREATE  FUNCTION "range_term"(
	"field" FieldName, /* pg_search::api::index::FieldName */
	"range" int4range, /* pgrx::datum::range::Range<i32> */
	"relation" RangeRelation /* pg_search::api::index::RangeRelation */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'range_term_range_int4range_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/index.rs:783
-- pg_search::api::index::range_term
CREATE  FUNCTION "range_term"(
	"field" FieldName, /* pg_search::api::index::FieldName */
	"range" int8range, /* pgrx::datum::range::Range<i64> */
	"relation" RangeRelation /* pg_search::api::index::RangeRelation */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'range_term_range_int8range_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/index.rs:789
-- pg_search::api::index::range_term
CREATE  FUNCTION "range_term"(
	"field" FieldName, /* pg_search::api::index::FieldName */
	"range" numrange, /* pgrx::datum::range::Range<pgrx::datum::numeric::AnyNumeric> */
	"relation" RangeRelation /* pg_search::api::index::RangeRelation */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'range_term_range_numrange_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/index.rs:801
-- pg_search::api::index::range_term
CREATE  FUNCTION "range_term"(
	"field" FieldName, /* pg_search::api::index::FieldName */
	"range" tsrange, /* pgrx::datum::range::Range<pgrx::datum::time_stamp::Timestamp> */
	"relation" RangeRelation /* pg_search::api::index::RangeRelation */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'range_term_range_tsrange_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/index.rs:807
-- pg_search::api::index::range_term
CREATE  FUNCTION "range_term"(
	"field" FieldName, /* pg_search::api::index::FieldName */
	"range" tstzrange, /* pgrx::datum::range::Range<pgrx::datum::time_stamp_with_timezone::TimestampWithTimeZone> */
	"relation" RangeRelation /* pg_search::api::index::RangeRelation */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'range_term_range_tstzrange_wrapper';
