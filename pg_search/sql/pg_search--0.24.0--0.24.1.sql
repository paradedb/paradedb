\echo Use "ALTER EXTENSION pg_search UPDATE TO '0.24.1'" to load this file. \quit

-- `boost_to_fuzzy` and its cast are also defined in `fuzzy.rs`, so they live in
-- the base install schema. Depending on which `v0.24.0` an install was built
-- from, they may or may not already exist (community's `v0.24.0` predates them;
-- enterprise's folds them into the base schema). Drop-then-create keeps this
-- upgrade idempotent in both cases so it never fails with "already exists".
DROP CAST IF EXISTS (pdb.boost AS pdb.fuzzy);
DROP FUNCTION IF EXISTS "boost_to_fuzzy"(pdb.boost, integer, boolean);

CREATE FUNCTION "boost_to_fuzzy"(
	"input" pdb.boost,
	"typmod" INT,
	"is_explicit" bool
) RETURNS pdb.fuzzy
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'boost_to_fuzzy_wrapper';
CREATE CAST (pdb.boost AS pdb.fuzzy) WITH FUNCTION boost_to_fuzzy(pdb.boost, integer, boolean) AS ASSIGNMENT;

-- The amcheck-style index verification functions (PR #3907) were originally
-- emitted into `pg_search--0.21.4--0.21.5.sql` -- a migration for a release that
-- had already shipped, so it sits off the upgrade path for anyone already past
-- 0.21.5. Fresh installs got these from the generated base schema, but
-- `ALTER EXTENSION pg_search UPDATE` from e.g. 0.23.x never created them. Re-emit
-- them here on the current upgrade path. DROP-then-CREATE keeps it idempotent for
-- installs that already have them (fresh 0.24.0, or a drop+create workaround).
DROP FUNCTION IF EXISTS pdb."indexes"();
CREATE  FUNCTION pdb."indexes"() RETURNS TABLE (
	"schemaname" TEXT,  /* alloc::string::String */
	"tablename" TEXT,  /* alloc::string::String */
	"indexname" TEXT,  /* alloc::string::String */
	"indexrelid" oid,  /* pgrx_pg_sys::submodules::oids::Oid */
	"num_segments" INT,  /* i32 */
	"total_docs" bigint  /* i64 */
)
STRICT
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'indexes_wrapper';

DROP FUNCTION IF EXISTS pdb."index_segments"(regclass);
CREATE  FUNCTION pdb."index_segments"(
	"index" regclass /* pgrx::rel::PgRelation */
) RETURNS TABLE (
	"partition_name" TEXT,  /* alloc::string::String */
	"segment_idx" INT,  /* i32 */
	"segment_id" TEXT,  /* alloc::string::String */
	"num_docs" bigint,  /* i64 */
	"num_deleted" bigint,  /* i64 */
	"max_doc" bigint  /* i64 */
)
STRICT
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'index_segments_wrapper';

DROP FUNCTION IF EXISTS pdb."verify_all_indexes"(TEXT, TEXT, bool, double precision, bool, bool);
CREATE  FUNCTION pdb."verify_all_indexes"(
	"schema_pattern" TEXT DEFAULT NULL, /* core::option::Option<alloc::string::String> */
	"index_pattern" TEXT DEFAULT NULL, /* core::option::Option<alloc::string::String> */
	"heapallindexed" bool DEFAULT false, /* bool */
	"sample_rate" double precision DEFAULT NULL, /* core::option::Option<f64> */
	"report_progress" bool DEFAULT false, /* bool */
	"on_error_stop" bool DEFAULT false /* bool */
) RETURNS TABLE (
	"schemaname" TEXT,  /* alloc::string::String */
	"indexname" TEXT,  /* alloc::string::String */
	"check_name" TEXT,  /* alloc::string::String */
	"passed" bool,  /* bool */
	"details" TEXT  /* core::option::Option<alloc::string::String> */
)
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'verify_all_indexes_wrapper';

DROP FUNCTION IF EXISTS pdb."verify_index"(regclass, bool, double precision, bool, bool, bool, INT[]);
CREATE  FUNCTION pdb."verify_index"(
	"index" regclass, /* pgrx::rel::PgRelation */
	"heapallindexed" bool DEFAULT false, /* bool */
	"sample_rate" double precision DEFAULT NULL, /* core::option::Option<f64> */
	"report_progress" bool DEFAULT false, /* bool */
	"verbose" bool DEFAULT false, /* bool */
	"on_error_stop" bool DEFAULT false, /* bool */
	"segment_ids" INT[] DEFAULT NULL /* core::option::Option<alloc::vec::Vec<i32>> */
) RETURNS TABLE (
	"check_name" TEXT,  /* alloc::string::String */
	"passed" bool,  /* bool */
	"details" TEXT  /* core::option::Option<alloc::string::String> */
)
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'verify_index_wrapper';

-- The objects below also drifted off the upgrade path: they live in the
-- generated base schema (so fresh installs, and ALTER EXTENSION UPDATE from
-- recent versions, have them) but no migration delta ever (re)created them for
-- installs coming from older versions. The upgrade test's schema-parity check
-- surfaced them. Re-emit them here, idempotently, so upgraders converge on the
-- same schema as a fresh install.

-- `tokenizer()` gained a `stopwords_languages text[]` parameter, but no delta
-- updated the function on the upgrade path -- leaving older installs with a stale
-- 12-arg signature bound to the current 13-arg `tokenizer_wrapper`. Drop the
-- stale signature (no-op on installs that already have the current one) and
-- (re)create the current definition.
DROP FUNCTION IF EXISTS tokenizer(text, integer, boolean, integer, integer, boolean, text, text, text, text, text[], boolean);
CREATE OR REPLACE FUNCTION tokenizer(name text, remove_long pg_catalog.int4 DEFAULT '255', lowercase bool DEFAULT 'true', min_gram pg_catalog.int4 DEFAULT NULL, max_gram pg_catalog.int4 DEFAULT NULL, prefix_only bool DEFAULT NULL, language text DEFAULT NULL, pattern text DEFAULT NULL, stemmer text DEFAULT NULL, stopwords_language text DEFAULT NULL, stopwords_languages text[] DEFAULT NULL, stopwords text[] DEFAULT NULL, ascii_folding bool DEFAULT NULL) RETURNS jsonb AS 'MODULE_PATHNAME', 'tokenizer_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE;

-- `pdb.text_array_to_icu(text[])` and its `text[] -> pdb.icu` cast were never
-- emitted into any delta, so upgraders never got them. Drop-then-create keeps it
-- idempotent for installs that already have them.
DROP CAST IF EXISTS (text[] AS pdb.icu);
DROP FUNCTION IF EXISTS pdb."text_array_to_icu"(text[]);
CREATE FUNCTION pdb."text_array_to_icu"(
	"arr" text[]
) RETURNS pdb.icu
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'text_array_to_icu_wrapper';
CREATE CAST (text[] AS pdb.icu) WITH FUNCTION pdb.text_array_to_icu AS ASSIGNMENT;

-- The `uuid -> pdb.alias` cast was dropped in 0.21.16--0.22.0 (to rename the
-- backing function's argument) but never recreated. The `pdb.uuid_to_alias`
-- function is already on the path; restore only the missing cast.
DROP CAST IF EXISTS (uuid AS pdb.alias);
CREATE CAST (uuid AS pdb.alias) WITH FUNCTION pdb.uuid_to_alias AS ASSIGNMENT;


CREATE FUNCTION "alias_typmod_in"(
	"typmod_parts" cstring[] /* Array < '_, & '_ CStr > */
) RETURNS INT /* i32 */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'alias_typmod_in_wrapper';

ALTER TYPE pdb.alias SET (TYPMOD_IN = alias_typmod_in, TYPMOD_OUT = generic_typmod_out);
