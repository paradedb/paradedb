/* pg_search::api::aggregate::pdb */
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/aggregate.rs:94
-- pg_search::api::aggregate::pdb::agg_placeholder_with_mvcc_agg_placeholder_with_mvcc_state
CREATE OR REPLACE FUNCTION pdb."agg_placeholder_with_mvcc_agg_placeholder_with_mvcc_state"(
	"this" internal, /* pgrx::datum::internal::Internal */
	"arg_one" jsonb /* pgrx::datum::json::JsonB */
) RETURNS internal /* pgrx::datum::internal::Internal */
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'agg_placeholder_with_mvcc_agg_placeholder_with_mvcc_state_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/aggregate.rs:94
-- pg_search::api::aggregate::pdb::agg_placeholder_with_mvcc_agg_placeholder_with_mvcc_finalize
CREATE OR REPLACE FUNCTION pdb."agg_placeholder_with_mvcc_agg_placeholder_with_mvcc_finalize"(
	"this" internal /* pgrx::datum::internal::Internal */
) RETURNS jsonb /* pgrx::datum::json::JsonB */
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'agg_placeholder_with_mvcc_agg_placeholder_with_mvcc_finalize_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/aggregate.rs:94
-- pg_search::api::aggregate::pdb::AggPlaceholderWithMVCC
DROP AGGREGATE IF EXISTS pdb.agg(jsonb);
CREATE AGGREGATE pdb.agg (
	jsonb /* pgrx::datum::json::JsonB */
)
(
	SFUNC = pdb."agg_placeholder_with_mvcc_agg_placeholder_with_mvcc_state", /* pg_search::api::aggregate::pdb::AggPlaceholderWithMVCC::state */
	STYPE = internal, /* pgrx::datum::internal::Internal */
	FINALFUNC = pdb."agg_placeholder_with_mvcc_agg_placeholder_with_mvcc_finalize" /* pg_search::api::aggregate::pdb::AggPlaceholderWithMVCC::final */
);

/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:473
-- pg_search::api::tokenizers::definitions::pdb::tstzrange_to_alias
-- requires:
--   tokenize_alias
CREATE  FUNCTION pdb."tstzrange_to_alias"(
	"arr" tstzrange /* pg_search::api::tokenizers::GenericTypeWrapper<pgrx::datum::range::Range<pgrx::datum::time_stamp_with_timezone::TimestampWithTimeZone>, pg_search::api::tokenizers::definitions::pdb::TstzRangeMarker> */
) RETURNS pdb.alias /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Alias, pg_search::api::tokenizers::definitions::pdb::AliasMarker> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'tstzrange_to_alias_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:461
-- pg_search::api::tokenizers::definitions::pdb::daterange_to_alias
-- requires:
--   tokenize_alias
CREATE  FUNCTION pdb."daterange_to_alias"(
	"arr" daterange /* pg_search::api::tokenizers::GenericTypeWrapper<pgrx::datum::range::Range<pgrx::datum::date::Date>, pg_search::api::tokenizers::definitions::pdb::DateRangeMarker> */
) RETURNS pdb.alias /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Alias, pg_search::api::tokenizers::definitions::pdb::AliasMarker> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'daterange_to_alias_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:467
-- pg_search::api::tokenizers::definitions::pdb::tsrange_to_alias
-- requires:
--   tokenize_alias
CREATE  FUNCTION pdb."tsrange_to_alias"(
	"arr" tsrange /* pg_search::api::tokenizers::GenericTypeWrapper<pgrx::datum::range::Range<pgrx::datum::time_stamp::Timestamp>, pg_search::api::tokenizers::definitions::pdb::TsRangeMarker> */
) RETURNS pdb.alias /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Alias, pg_search::api::tokenizers::definitions::pdb::AliasMarker> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'tsrange_to_alias_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:429
-- pg_search::api::tokenizers::definitions::pdb::integer_to_alias
-- requires:
--   tokenize_alias
CREATE  FUNCTION pdb."integer_to_alias"(
	"arr" integer /* pg_search::api::tokenizers::GenericTypeWrapper<i32, pg_search::api::tokenizers::definitions::pdb::IntegerMarker> */
) RETURNS pdb.alias /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Alias, pg_search::api::tokenizers::definitions::pdb::AliasMarker> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'integer_to_alias_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:433
-- pg_search::api::tokenizers::definitions::pdb::float8_to_alias
-- requires:
--   tokenize_alias
CREATE  FUNCTION pdb."float8_to_alias"(
	"arr" float8 /* pg_search::api::tokenizers::GenericTypeWrapper<f64, pg_search::api::tokenizers::definitions::pdb::Float8Marker> */
) RETURNS pdb.alias /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Alias, pg_search::api::tokenizers::definitions::pdb::AliasMarker> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'float8_to_alias_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:481
-- pg_search::api::tokenizers::definitions::pdb::bigint_array_to_alias
-- requires:
--   tokenize_alias
CREATE  FUNCTION pdb."bigint_array_to_alias"(
	"arr" bigint[] /* pg_search::api::tokenizers::GenericTypeWrapper<alloc::vec::Vec<i64>, pg_search::api::tokenizers::definitions::pdb::BigIntArrayMarker> */
) RETURNS pdb.alias /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Alias, pg_search::api::tokenizers::definitions::pdb::AliasMarker> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'bigint_array_to_alias_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:436
-- pg_search::api::tokenizers::definitions::pdb::date_to_alias
-- requires:
--   tokenize_alias
CREATE  FUNCTION pdb."date_to_alias"(
	"arr" date /* pg_search::api::tokenizers::GenericTypeWrapper<pgrx::datum::date::Date, pg_search::api::tokenizers::definitions::pdb::DateMarker> */
) RETURNS pdb.alias /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Alias, pg_search::api::tokenizers::definitions::pdb::AliasMarker> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'date_to_alias_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:491
-- pg_search::api::tokenizers::definitions::pdb::date_array_to_alias
-- requires:
--   tokenize_alias
CREATE  FUNCTION pdb."date_array_to_alias"(
	"arr" date[] /* pg_search::api::tokenizers::GenericTypeWrapper<alloc::vec::Vec<pgrx::datum::date::Date>, pg_search::api::tokenizers::definitions::pdb::DateArrayMarker> */
) RETURNS pdb.alias /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Alias, pg_search::api::tokenizers::definitions::pdb::AliasMarker> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'date_array_to_alias_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:425
-- pg_search::api::tokenizers::definitions::pdb::text_to_alias
-- requires:
--   tokenize_alias
CREATE  FUNCTION pdb."text_to_alias"(
	"arr" text /* pg_search::api::tokenizers::GenericTypeWrapper<alloc::string::String, pg_search::api::tokenizers::definitions::pdb::TextMarker> */
) RETURNS pdb.alias /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Alias, pg_search::api::tokenizers::definitions::pdb::AliasMarker> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'text_to_alias_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:426
-- pg_search::api::tokenizers::definitions::pdb::varchar_to_alias
-- requires:
--   tokenize_alias
CREATE  FUNCTION pdb."varchar_to_alias"(
	"arr" varchar /* pg_search::api::tokenizers::GenericTypeWrapper<alloc::string::String, pg_search::api::tokenizers::definitions::pdb::VarcharMarker> */
) RETURNS pdb.alias /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Alias, pg_search::api::tokenizers::definitions::pdb::AliasMarker> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'varchar_to_alias_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:455
-- pg_search::api::tokenizers::definitions::pdb::numrange_to_alias
-- requires:
--   tokenize_alias
CREATE  FUNCTION pdb."numrange_to_alias"(
	"arr" numrange /* pg_search::api::tokenizers::GenericTypeWrapper<pgrx::datum::range::Range<pgrx::datum::numeric::AnyNumeric>, pg_search::api::tokenizers::definitions::pdb::NumRangeMarker> */
) RETURNS pdb.alias /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Alias, pg_search::api::tokenizers::definitions::pdb::AliasMarker> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'numrange_to_alias_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:430
-- pg_search::api::tokenizers::definitions::pdb::bigint_to_alias
-- requires:
--   tokenize_alias
CREATE  FUNCTION pdb."bigint_to_alias"(
	"arr" bigint /* pg_search::api::tokenizers::GenericTypeWrapper<i64, pg_search::api::tokenizers::definitions::pdb::BigIntMarker> */
) RETURNS pdb.alias /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Alias, pg_search::api::tokenizers::definitions::pdb::AliasMarker> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'bigint_to_alias_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:480
-- pg_search::api::tokenizers::definitions::pdb::integer_array_to_alias
-- requires:
--   tokenize_alias
CREATE  FUNCTION pdb."integer_array_to_alias"(
	"arr" integer[] /* pg_search::api::tokenizers::GenericTypeWrapper<alloc::vec::Vec<i32>, pg_search::api::tokenizers::definitions::pdb::IntegerArrayMarker> */
) RETURNS pdb.alias /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Alias, pg_search::api::tokenizers::definitions::pdb::AliasMarker> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'integer_array_to_alias_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:432
-- pg_search::api::tokenizers::definitions::pdb::float4_to_alias
-- requires:
--   tokenize_alias
CREATE  FUNCTION pdb."float4_to_alias"(
	"arr" float4 /* pg_search::api::tokenizers::GenericTypeWrapper<f32, pg_search::api::tokenizers::definitions::pdb::Float4Marker> */
) RETURNS pdb.alias /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Alias, pg_search::api::tokenizers::definitions::pdb::AliasMarker> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'float4_to_alias_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:505
-- pg_search::api::tokenizers::definitions::pdb::time_with_time_zone_array_to_alias
-- requires:
--   tokenize_alias
CREATE  FUNCTION pdb."time_with_time_zone_array_to_alias"(
	"arr" time with time zone[] /* pg_search::api::tokenizers::GenericTypeWrapper<alloc::vec::Vec<pgrx::datum::time_with_timezone::TimeWithTimeZone>, pg_search::api::tokenizers::definitions::pdb::TimeWithTimeZoneArrayMarker> */
) RETURNS pdb.alias /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Alias, pg_search::api::tokenizers::definitions::pdb::AliasMarker> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'time_with_time_zone_array_to_alias_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:492
-- pg_search::api::tokenizers::definitions::pdb::time_array_to_alias
-- requires:
--   tokenize_alias
CREATE  FUNCTION pdb."time_array_to_alias"(
	"arr" time[] /* pg_search::api::tokenizers::GenericTypeWrapper<alloc::vec::Vec<pgrx::datum::time::Time>, pg_search::api::tokenizers::definitions::pdb::TimeArrayMarker> */
) RETURNS pdb.alias /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Alias, pg_search::api::tokenizers::definitions::pdb::AliasMarker> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'time_array_to_alias_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:428
-- pg_search::api::tokenizers::definitions::pdb::smallint_to_alias
-- requires:
--   tokenize_alias
CREATE  FUNCTION pdb."smallint_to_alias"(
	"arr" smallint /* pg_search::api::tokenizers::GenericTypeWrapper<i16, pg_search::api::tokenizers::definitions::pdb::SmallIntMarker> */
) RETURNS pdb.alias /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Alias, pg_search::api::tokenizers::definitions::pdb::AliasMarker> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'smallint_to_alias_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:484
-- pg_search::api::tokenizers::definitions::pdb::numeric_array_to_alias
-- requires:
--   tokenize_alias
CREATE  FUNCTION pdb."numeric_array_to_alias"(
	"arr" numeric[] /* pg_search::api::tokenizers::GenericTypeWrapper<alloc::vec::Vec<pgrx::datum::numeric::AnyNumeric>, pg_search::api::tokenizers::definitions::pdb::NumericArrayMarker> */
) RETURNS pdb.alias /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Alias, pg_search::api::tokenizers::definitions::pdb::AliasMarker> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'numeric_array_to_alias_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:439
-- pg_search::api::tokenizers::definitions::pdb::timestamp_with_time_zone_to_alias
-- requires:
--   tokenize_alias
CREATE  FUNCTION pdb."timestamp_with_time_zone_to_alias"(
	"arr" timestamp with time zone /* pg_search::api::tokenizers::GenericTypeWrapper<pgrx::datum::time_stamp_with_timezone::TimestampWithTimeZone, pg_search::api::tokenizers::definitions::pdb::TimestampWithTimeZoneMarker> */
) RETURNS pdb.alias /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Alias, pg_search::api::tokenizers::definitions::pdb::AliasMarker> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'timestamp_with_time_zone_to_alias_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:483
-- pg_search::api::tokenizers::definitions::pdb::float8_array_to_alias
-- requires:
--   tokenize_alias
CREATE  FUNCTION pdb."float8_array_to_alias"(
	"arr" float8[] /* pg_search::api::tokenizers::GenericTypeWrapper<alloc::vec::Vec<f64>, pg_search::api::tokenizers::definitions::pdb::Float8ArrayMarker> */
) RETURNS pdb.alias /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Alias, pg_search::api::tokenizers::definitions::pdb::AliasMarker> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'float8_array_to_alias_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:435
-- pg_search::api::tokenizers::definitions::pdb::boolean_to_alias
-- requires:
--   tokenize_alias
CREATE  FUNCTION pdb."boolean_to_alias"(
	"arr" boolean /* pg_search::api::tokenizers::GenericTypeWrapper<bool, pg_search::api::tokenizers::definitions::pdb::BooleanMarker> */
) RETURNS pdb.alias /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Alias, pg_search::api::tokenizers::definitions::pdb::AliasMarker> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'boolean_to_alias_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:493
-- pg_search::api::tokenizers::definitions::pdb::timestamp_array_to_alias
-- requires:
--   tokenize_alias
CREATE  FUNCTION pdb."timestamp_array_to_alias"(
	"arr" timestamp[] /* pg_search::api::tokenizers::GenericTypeWrapper<alloc::vec::Vec<pgrx::datum::time_stamp::Timestamp>, pg_search::api::tokenizers::definitions::pdb::TimestampArrayMarker> */
) RETURNS pdb.alias /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Alias, pg_search::api::tokenizers::definitions::pdb::AliasMarker> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'timestamp_array_to_alias_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:499
-- pg_search::api::tokenizers::definitions::pdb::timestamp_with_time_zone_array_to_alias
-- requires:
--   tokenize_alias
CREATE  FUNCTION pdb."timestamp_with_time_zone_array_to_alias"(
	"arr" timestamp with time zone[] /* pg_search::api::tokenizers::GenericTypeWrapper<alloc::vec::Vec<pgrx::datum::time_stamp_with_timezone::TimestampWithTimeZone>, pg_search::api::tokenizers::definitions::pdb::TimestampWithTimeZoneArrayMarker> */
) RETURNS pdb.alias /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Alias, pg_search::api::tokenizers::definitions::pdb::AliasMarker> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'timestamp_with_time_zone_array_to_alias_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:431
-- pg_search::api::tokenizers::definitions::pdb::oid_to_alias
-- requires:
--   tokenize_alias
CREATE  FUNCTION pdb."oid_to_alias"(
	"arr" oid /* pg_search::api::tokenizers::GenericTypeWrapper<u32, pg_search::api::tokenizers::definitions::pdb::OidMarker> */
) RETURNS pdb.alias /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Alias, pg_search::api::tokenizers::definitions::pdb::AliasMarker> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'oid_to_alias_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:427
-- pg_search::api::tokenizers::definitions::pdb::uuid_to_alias
-- requires:
--   tokenize_alias
CREATE  FUNCTION pdb."uuid_to_alias"(
	"arr" uuid /* pg_search::api::tokenizers::GenericTypeWrapper<pgrx::datum::uuid::Uuid, pg_search::api::tokenizers::definitions::pdb::UuidMarker> */
) RETURNS pdb.alias /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Alias, pg_search::api::tokenizers::definitions::pdb::AliasMarker> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'uuid_to_alias_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:434
-- pg_search::api::tokenizers::definitions::pdb::numeric_to_alias
-- requires:
--   tokenize_alias
CREATE  FUNCTION pdb."numeric_to_alias"(
	"arr" numeric /* pg_search::api::tokenizers::GenericTypeWrapper<pgrx::datum::numeric::AnyNumeric, pg_search::api::tokenizers::definitions::pdb::NumericMarker> */
) RETURNS pdb.alias /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Alias, pg_search::api::tokenizers::definitions::pdb::AliasMarker> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'numeric_to_alias_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:437
-- pg_search::api::tokenizers::definitions::pdb::time_to_alias
-- requires:
--   tokenize_alias
CREATE  FUNCTION pdb."time_to_alias"(
	"arr" time /* pg_search::api::tokenizers::GenericTypeWrapper<pgrx::datum::time::Time, pg_search::api::tokenizers::definitions::pdb::TimeMarker> */
) RETURNS pdb.alias /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Alias, pg_search::api::tokenizers::definitions::pdb::AliasMarker> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'time_to_alias_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:445
-- pg_search::api::tokenizers::definitions::pdb::time_with_time_zone_to_alias
-- requires:
--   tokenize_alias
CREATE  FUNCTION pdb."time_with_time_zone_to_alias"(
	"arr" time with time zone /* pg_search::api::tokenizers::GenericTypeWrapper<pgrx::datum::time_with_timezone::TimeWithTimeZone, pg_search::api::tokenizers::definitions::pdb::TimeWithTimeZoneMarker> */
) RETURNS pdb.alias /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Alias, pg_search::api::tokenizers::definitions::pdb::AliasMarker> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'time_with_time_zone_to_alias_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:453
-- pg_search::api::tokenizers::definitions::pdb::int4range_to_alias
-- requires:
--   tokenize_alias
CREATE  FUNCTION pdb."int4range_to_alias"(
	"arr" int4range /* pg_search::api::tokenizers::GenericTypeWrapper<pgrx::datum::range::Range<i32>, pg_search::api::tokenizers::definitions::pdb::Int4RangeMarker> */
) RETURNS pdb.alias /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Alias, pg_search::api::tokenizers::definitions::pdb::AliasMarker> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'int4range_to_alias_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:452
-- pg_search::api::tokenizers::definitions::pdb::inet_to_alias
-- requires:
--   tokenize_alias
CREATE  FUNCTION pdb."inet_to_alias"(
	"arr" inet /* pg_search::api::tokenizers::GenericTypeWrapper<pgrx::datum::inet::Inet, pg_search::api::tokenizers::definitions::pdb::InetMarker> */
) RETURNS pdb.alias /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Alias, pg_search::api::tokenizers::definitions::pdb::AliasMarker> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'inet_to_alias_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:482
-- pg_search::api::tokenizers::definitions::pdb::float4_array_to_alias
-- requires:
--   tokenize_alias
CREATE  FUNCTION pdb."float4_array_to_alias"(
	"arr" float4[] /* pg_search::api::tokenizers::GenericTypeWrapper<alloc::vec::Vec<f32>, pg_search::api::tokenizers::definitions::pdb::Float4ArrayMarker> */
) RETURNS pdb.alias /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Alias, pg_search::api::tokenizers::definitions::pdb::AliasMarker> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'float4_array_to_alias_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:438
-- pg_search::api::tokenizers::definitions::pdb::timestamp_to_alias
-- requires:
--   tokenize_alias
CREATE  FUNCTION pdb."timestamp_to_alias"(
	"arr" timestamp /* pg_search::api::tokenizers::GenericTypeWrapper<pgrx::datum::time_stamp::Timestamp, pg_search::api::tokenizers::definitions::pdb::TimestampMarker> */
) RETURNS pdb.alias /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Alias, pg_search::api::tokenizers::definitions::pdb::AliasMarker> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'timestamp_to_alias_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:479
-- pg_search::api::tokenizers::definitions::pdb::smallint_array_to_alias
-- requires:
--   tokenize_alias
CREATE  FUNCTION pdb."smallint_array_to_alias"(
	"arr" smallint[] /* pg_search::api::tokenizers::GenericTypeWrapper<alloc::vec::Vec<i16>, pg_search::api::tokenizers::definitions::pdb::SmallIntArrayMarker> */
) RETURNS pdb.alias /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Alias, pg_search::api::tokenizers::definitions::pdb::AliasMarker> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'smallint_array_to_alias_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:454
-- pg_search::api::tokenizers::definitions::pdb::int8range_to_alias
-- requires:
--   tokenize_alias
CREATE  FUNCTION pdb."int8range_to_alias"(
	"arr" int8range /* pg_search::api::tokenizers::GenericTypeWrapper<pgrx::datum::range::Range<i64>, pg_search::api::tokenizers::definitions::pdb::Int8RangeMarker> */
) RETURNS pdb.alias /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Alias, pg_search::api::tokenizers::definitions::pdb::AliasMarker> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'int8range_to_alias_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:490
-- pg_search::api::tokenizers::definitions::pdb::boolean_array_to_alias
-- requires:
--   tokenize_alias
CREATE  FUNCTION pdb."boolean_array_to_alias"(
	"arr" boolean[] /* pg_search::api::tokenizers::GenericTypeWrapper<alloc::vec::Vec<bool>, pg_search::api::tokenizers::definitions::pdb::BooleanArrayMarker> */
) RETURNS pdb.alias /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Alias, pg_search::api::tokenizers::definitions::pdb::AliasMarker> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'boolean_array_to_alias_wrapper';

CREATE CAST (text AS pdb.alias) WITH FUNCTION pdb.text_to_alias AS ASSIGNMENT;
CREATE CAST (varchar AS pdb.alias) WITH FUNCTION pdb.varchar_to_alias AS ASSIGNMENT;
CREATE CAST (uuid AS pdb.alias) WITH FUNCTION pdb.uuid_to_alias AS ASSIGNMENT;
CREATE CAST (smallint AS pdb.alias) WITH FUNCTION pdb.smallint_to_alias AS ASSIGNMENT;
CREATE CAST (integer AS pdb.alias) WITH FUNCTION pdb.integer_to_alias AS ASSIGNMENT;
CREATE CAST (bigint AS pdb.alias) WITH FUNCTION pdb.bigint_to_alias AS ASSIGNMENT;
CREATE CAST (oid AS pdb.alias) WITH FUNCTION pdb.oid_to_alias AS ASSIGNMENT;
CREATE CAST (float4 AS pdb.alias) WITH FUNCTION pdb.float4_to_alias AS ASSIGNMENT;
CREATE CAST (float8 AS pdb.alias) WITH FUNCTION pdb.float8_to_alias AS ASSIGNMENT;
CREATE CAST (numeric AS pdb.alias) WITH FUNCTION pdb.numeric_to_alias AS ASSIGNMENT;
CREATE CAST (boolean AS pdb.alias) WITH FUNCTION pdb.boolean_to_alias AS ASSIGNMENT;
CREATE CAST (date AS pdb.alias) WITH FUNCTION pdb.date_to_alias AS ASSIGNMENT;
CREATE CAST (time AS pdb.alias) WITH FUNCTION pdb.time_to_alias AS ASSIGNMENT;
CREATE CAST (timestamp AS pdb.alias) WITH FUNCTION pdb.timestamp_to_alias AS ASSIGNMENT;
CREATE CAST (timestamp with time zone AS pdb.alias) WITH FUNCTION pdb.timestamp_with_time_zone_to_alias AS ASSIGNMENT;
CREATE CAST (time with time zone AS pdb.alias) WITH FUNCTION pdb.time_with_time_zone_to_alias AS ASSIGNMENT;
CREATE CAST (inet AS pdb.alias) WITH FUNCTION pdb.inet_to_alias AS ASSIGNMENT;
CREATE CAST (int4range AS pdb.alias) WITH FUNCTION pdb.int4range_to_alias AS ASSIGNMENT;
CREATE CAST (int8range AS pdb.alias) WITH FUNCTION pdb.int8range_to_alias AS ASSIGNMENT;
CREATE CAST (numrange AS pdb.alias) WITH FUNCTION pdb.numrange_to_alias AS ASSIGNMENT;
CREATE CAST (daterange AS pdb.alias) WITH FUNCTION pdb.daterange_to_alias AS ASSIGNMENT;
CREATE CAST (tsrange AS pdb.alias) WITH FUNCTION pdb.tsrange_to_alias AS ASSIGNMENT;
CREATE CAST (tstzrange AS pdb.alias) WITH FUNCTION pdb.tstzrange_to_alias AS ASSIGNMENT;
CREATE CAST (smallint[] AS pdb.alias) WITH FUNCTION pdb.smallint_array_to_alias AS ASSIGNMENT;
CREATE CAST (integer[] AS pdb.alias) WITH FUNCTION pdb.integer_array_to_alias AS ASSIGNMENT;
CREATE CAST (bigint[] AS pdb.alias) WITH FUNCTION pdb.bigint_array_to_alias AS ASSIGNMENT;
CREATE CAST (float4[] AS pdb.alias) WITH FUNCTION pdb.float4_array_to_alias AS ASSIGNMENT;
CREATE CAST (float8[] AS pdb.alias) WITH FUNCTION pdb.float8_array_to_alias AS ASSIGNMENT;
CREATE CAST (numeric[] AS pdb.alias) WITH FUNCTION pdb.numeric_array_to_alias AS ASSIGNMENT;
CREATE CAST (boolean[] AS pdb.alias) WITH FUNCTION pdb.boolean_array_to_alias AS ASSIGNMENT;
CREATE CAST (date[] AS pdb.alias) WITH FUNCTION pdb.date_array_to_alias AS ASSIGNMENT;
CREATE CAST (time[] AS pdb.alias) WITH FUNCTION pdb.time_array_to_alias AS ASSIGNMENT;
CREATE CAST (timestamp[] AS pdb.alias) WITH FUNCTION pdb.timestamp_array_to_alias AS ASSIGNMENT;
CREATE CAST (timestamp with time zone[] AS pdb.alias) WITH FUNCTION pdb.timestamp_with_time_zone_array_to_alias AS ASSIGNMENT;
CREATE CAST (time with time zone[] AS pdb.alias) WITH FUNCTION pdb.time_with_time_zone_array_to_alias AS ASSIGNMENT;
