-- Report the vector quantization build target for each leaf index.

CREATE  FUNCTION "vector_config"(
	"index_relation" regclass, /* PgRelation */
	"field" TEXT /* String */
) RETURNS TABLE (
	"index_oid" oid,  /* pg_sys :: Oid */
	"quantized" bool,  /* bool */
	"layers" INT[],  /* :: std :: option :: Option < Vec < i32 > > */
	"bytes_per_row" INT,  /* Option < i32 > */
	"settings_version" INT  /* Option < i32 > */
)
STRICT
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'vector_config_wrapper';
