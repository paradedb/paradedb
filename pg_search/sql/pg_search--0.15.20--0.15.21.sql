\echo Use "ALTER EXTENSION pg_search UPDATE TO '0.15.21'" to load this file. \quit

DROP FUNCTION IF EXISTS tokenizer(name text,
    remove_long pg_catalog.int4,
    lowercase bool,
    min_gram pg_catalog.int4,
    max_gram pg_catalog.int4,
    prefix_only bool,
    language text,
    pattern text,
    stemmer text);

CREATE OR REPLACE FUNCTION tokenizer(
    name text,
    remove_long pg_catalog.int4 DEFAULT '255',
    lowercase bool DEFAULT true,
    min_gram pg_catalog.int4 DEFAULT NULL,
    max_gram pg_catalog.int4 DEFAULT NULL,
    prefix_only bool DEFAULT NULL,
    language text DEFAULT NULL,
    pattern text DEFAULT NULL,
    stemmer text DEFAULT NULL,
    stopwords_language text DEFAULT NULL,
    stopwords text[] DEFAULT NULL
)
RETURNS jsonb AS 'MODULE_PATHNAME', 'tokenizer_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE;

-- pg_search/src/api/aggregate.rs:249
-- pg_search::api::aggregate::aggregate
CREATE  FUNCTION "aggregate"(
    "index" regclass, /* pgrx::rel::PgRelation */
    "query" SearchQueryInput, /* pg_search::query::SearchQueryInput */
    "agg" json, /* pgrx::datum::json::Json */
    "solve_mvcc" bool DEFAULT true, /* bool */
    "memory_limit" bigint DEFAULT 500000000, /* i64 */
    "bucket_limit" bigint DEFAULT 65000 /* i64 */
) RETURNS jsonb /* core::result::Result<pgrx::datum::json::JsonB, alloc::boxed::Box<dyn core::error::Error>> */
    STRICT
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'aggregate_wrapper';