-- Adds ivf_cluster_radii(index regclass, field text): a read-only
-- set-returning function surfacing the stored per-cluster IVF radii
-- (`.centroids` slot [3]) of one vector field, one row per cluster per
-- segment - the observation instrument for the radius-aware probe gate.
-- Radii are NATIVE-only (rank-0 members; replica spill excluded). A zero
-- radius is a real value - every native member of that cluster sits on its
-- centroid - not a missing one: indexes written before radii became
-- required are refused at open with a REINDEX message and never reach here.
-- Computes nothing new and adds no on-disk state. The CREATE below is the
-- SchemaBot/pgrx canonical text verbatim (the schema checker compares
-- statements textually); the DROP keeps the script re-runnable.
DROP FUNCTION IF EXISTS ivf_cluster_radii(regclass, text);
CREATE  FUNCTION "ivf_cluster_radii"(
	"index" regclass, /* PgRelation */
	"field" TEXT /* String */
) RETURNS TABLE (
	"segno" TEXT,  /* String */
	"vector_field" TEXT,  /* String */
	"cluster_ord" INT,  /* i32 */
	"radius" real  /* f32 */
)
STRICT
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'ivf_cluster_radii_wrapper';
