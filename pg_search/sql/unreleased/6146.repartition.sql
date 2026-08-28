CREATE FUNCTION paradedb."repartition"(
	"index" regclass /* PgRelation */
) RETURNS bigint /* anyhow :: Result < i64 > */
STRICT
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'repartition_wrapper';
