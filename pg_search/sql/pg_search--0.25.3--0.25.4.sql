\echo Use "ALTER EXTENSION pg_search UPDATE TO '0.25.4'" to load this file. \quit

-- 0.25.3 -> 0.25.4: register the runtime *_search_query_input pg_externs for the
-- &&&, |||, and ### operators. These mirror paradedb.term_search_query_input
-- (added in 0.25.1 -> 0.25.2 for ===) and are called from each operator's
-- exec_rewrite when the RHS is a Param under a generic prepared plan. Fixes
-- issue #5779 for the three sibling operators.

-- pg_search/src/api/operator/andandand.rs
-- pg_search::api::operator::andandand::match_conjunction_search_query_input
CREATE  FUNCTION "match_conjunction_search_query_input"(
	"field" FieldName, /* FieldName */
	"query" pdb.Query /* pdb :: Query */
) RETURNS SearchQueryInput /* SearchQueryInput */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'match_conjunction_search_query_input_wrapper';

-- pg_search/src/api/operator/ororor.rs
-- pg_search::api::operator::ororor::match_disjunction_search_query_input
CREATE  FUNCTION "match_disjunction_search_query_input"(
	"field" FieldName, /* FieldName */
	"query" pdb.Query /* pdb :: Query */
) RETURNS SearchQueryInput /* SearchQueryInput */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'match_disjunction_search_query_input_wrapper';

-- pg_search/src/api/operator/hashhashhash.rs
-- pg_search::api::operator::hashhashhash::phrase_search_query_input
CREATE  FUNCTION "phrase_search_query_input"(
	"field" FieldName, /* FieldName */
	"query" pdb.Query /* pdb :: Query */
) RETURNS SearchQueryInput /* SearchQueryInput */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'phrase_search_query_input_wrapper';

-- vector_clusters (#5960) and the create_paradedb_test_table rename (#5903)
-- were added to the 0.25.2 -> 0.25.3 script after v0.25.3 was tagged, so a
-- database created from the released 0.25.3 never runs them. Repeat both here
-- (idempotently), same as that script does for the rename over released 0.25.2.
DROP FUNCTION IF EXISTS vector_clusters(regclass, text);
CREATE  FUNCTION "vector_clusters"(
	"index" regclass, /* PgRelation */
	"field" TEXT /* String */
) RETURNS TABLE (
	"segno" TEXT,  /* String */
	"cluster_sizes" bigint[],  /* :: std :: option :: Option < Vec < i64 > > */
	"cluster_radii" real[]  /* :: std :: option :: Option < Vec < f32 > > */
)
STRICT
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'vector_clusters_wrapper';

DROP PROCEDURE IF EXISTS paradedb.create_bm25_test_table(table_name pg_catalog."varchar", schema_name pg_catalog."varchar", table_type paradedb.testtable);
CREATE OR REPLACE PROCEDURE paradedb.create_paradedb_test_table(table_name VARCHAR DEFAULT 'bm25_test_table', schema_name VARCHAR DEFAULT 'paradedb', table_type paradedb.TestTable DEFAULT 'Items')
LANGUAGE c AS 'MODULE_PATHNAME', 'create_paradedb_test_table_wrapper';
