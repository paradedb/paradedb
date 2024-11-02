/* <begin connected objects> */
-- pg_search/src/api/index.rs:700
-- pg_search::api::index::arr_term_in
CREATE  FUNCTION "arr_term_in"(
	"field" TEXT, /* &str */
	"value" bool[] /* alloc::vec::Vec<bool> */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'arr_term_bool_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/index.rs:701
-- pg_search::api::index::arr_term_in
CREATE  FUNCTION "arr_term_in"(
	"field" TEXT, /* &str */
	"value" date[] /* alloc::vec::Vec<pgrx::datum::date::Date> */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'arr_term_date_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/index.rs:698
-- pg_search::api::index::arr_term_in
CREATE  FUNCTION "arr_term_in"(
	"field" TEXT, /* &str */
	"value" real[] /* alloc::vec::Vec<f32> */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'arr_term_f32_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/index.rs:699
-- pg_search::api::index::arr_term_in
CREATE  FUNCTION "arr_term_in"(
	"field" TEXT, /* &str */
	"value" double precision[] /* alloc::vec::Vec<f64> */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'arr_term_f64_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/index.rs:694
-- pg_search::api::index::arr_term_in
CREATE  FUNCTION "arr_term_in"(
	"field" TEXT, /* &str */
	"value" smallint[] /* alloc::vec::Vec<i16> */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'arr_term_i16_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/index.rs:695
-- pg_search::api::index::arr_term_in
CREATE  FUNCTION "arr_term_in"(
	"field" TEXT, /* &str */
	"value" INT[] /* alloc::vec::Vec<i32> */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'arr_term_i32_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/index.rs:696
-- pg_search::api::index::arr_term_in
CREATE  FUNCTION "arr_term_in"(
	"field" TEXT, /* &str */
	"value" bigint[] /* alloc::vec::Vec<i64> */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'arr_term_i64_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/index.rs:693
-- pg_search::api::index::arr_term_in
CREATE  FUNCTION "arr_term_in"(
	"field" TEXT, /* &str */
	"value" "char"[] /* alloc::vec::Vec<i8> */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'arr_term_i8_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/index.rs:709
-- pg_search::api::index::arr_term_in
CREATE  FUNCTION "arr_term_in"(
	"field" TEXT, /* &str */
	"value" NUMERIC[] /* alloc::vec::Vec<pgrx::datum::numeric::AnyNumeric> */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'arr_term_numeric_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/index.rs:697
-- pg_search::api::index::arr_term_in
CREATE  FUNCTION "arr_term_in"(
	"field" TEXT, /* &str */
	"value" TEXT[] /* alloc::vec::Vec<alloc::string::String> */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'arr_term_text_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/index.rs:702
-- pg_search::api::index::arr_term_in
CREATE  FUNCTION "arr_term_in"(
	"field" TEXT, /* &str */
	"value" time[] /* alloc::vec::Vec<pgrx::datum::time::Time> */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'arr_term_time_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/index.rs:704
-- pg_search::api::index::arr_term_in
CREATE  FUNCTION "arr_term_in"(
	"field" TEXT, /* &str */
	"value" time with time zone[] /* alloc::vec::Vec<pgrx::datum::time_with_timezone::TimeWithTimeZone> */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'arr_term_time_with_time_zone_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/index.rs:703
-- pg_search::api::index::arr_term_in
CREATE  FUNCTION "arr_term_in"(
	"field" TEXT, /* &str */
	"value" timestamp[] /* alloc::vec::Vec<pgrx::datum::time_stamp::Timestamp> */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'arr_term_timestamp_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/index.rs:705
-- pg_search::api::index::arr_term_in
CREATE  FUNCTION "arr_term_in"(
	"field" TEXT, /* &str */
	"value" timestamp with time zone[] /* alloc::vec::Vec<pgrx::datum::time_stamp_with_timezone::TimestampWithTimeZone> */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'arr_term_timestamp_with_time_zome_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/index.rs:710
-- pg_search::api::index::arr_term_in
CREATE  FUNCTION "arr_term_in"(
	"field" TEXT, /* &str */
	"value" uuid[] /* alloc::vec::Vec<pgrx::datum::uuid::Uuid> */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'arr_term_uuid_wrapper';
/* </end connected objects> */
