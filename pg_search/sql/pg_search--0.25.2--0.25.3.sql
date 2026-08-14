\echo Use "ALTER EXTENSION pg_search UPDATE TO '0.25.3'" to load this file. \quit

-- 0.25.2 -> 0.25.3: add vector_cluster_sizes(index regclass, field text), the
-- per-segment per-cluster posting-list sizes, in cluster order.
-- The CREATE below is the SchemaBot/pgrx canonical text verbatim (the schema
-- checker compares statements textually); the DROP keeps the script re-runnable.
DROP FUNCTION IF EXISTS vector_cluster_sizes(regclass, text);
CREATE  FUNCTION "vector_cluster_sizes"(
	"index" regclass, /* PgRelation */
	"field" TEXT /* String */
) RETURNS TABLE (
	"segno" TEXT,  /* String */
	"cluster_sizes" bigint[]  /* :: std :: option :: Option < Vec < i64 > > */
)
STRICT
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'vector_cluster_sizes_wrapper';

-- Rename paradedb.create_bm25_test_table to paradedb.create_paradedb_test_table
-- (#5903). The released 0.25.2 shipped without the rename, so it is repeated
-- here (idempotently) to cover upgrades from released 0.25.2.
DROP PROCEDURE IF EXISTS paradedb.create_bm25_test_table(table_name pg_catalog."varchar", schema_name pg_catalog."varchar", table_type paradedb.testtable);
CREATE OR REPLACE PROCEDURE paradedb.create_paradedb_test_table(table_name VARCHAR DEFAULT 'bm25_test_table', schema_name VARCHAR DEFAULT 'paradedb', table_type paradedb.TestTable DEFAULT 'Items')
LANGUAGE c AS 'MODULE_PATHNAME', 'create_paradedb_test_table_wrapper';
