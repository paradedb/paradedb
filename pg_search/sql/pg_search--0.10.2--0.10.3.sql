CREATE TYPE TestTable AS ENUM (
	'Items',
	'Orders',
	'Parts'
);
DROP PROCEDURE IF EXISTS paradedb.create_bm25_test_table(table_name pg_catalog."varchar", schema_name pg_catalog."varchar");
CREATE OR REPLACE PROCEDURE paradedb.create_bm25_test_table(table_name pg_catalog."varchar" DEFAULT 'bm25_test_table', schema_name pg_catalog."varchar" DEFAULT 'paradedb', table_type paradedb.testtable DEFAULT 'Items') AS 'MODULE_PATHNAME', 'create_bm25_test_table_wrapper' LANGUAGE c;
