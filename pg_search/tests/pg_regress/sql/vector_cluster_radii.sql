-- paradedb.ivf_cluster_radii: the read-only per-cluster radius dump
-- (`.centroids` slot [3], NATIVE members only). Structural assertions only —
-- radius values are f32 distances whose exact digits depend on the trained
-- centroids, so the test pins shape and invariants, not floats.
--
-- client_min_messages: the IVF merge emits a paradedb::ivf_build timings
-- NOTICE with nondeterministic millisecond values — keep it out of the
-- captured output.
SET client_min_messages = WARNING;
CREATE EXTENSION IF NOT EXISTS vector;
\i common/common_setup.sql

DROP TABLE IF EXISTS radii_probe;
CREATE TABLE radii_probe (
    id  int PRIMARY KEY,
    vec vector(16)
);

-- Same forcing recipe as vector_merge: immutable inserts, foreground merges,
-- a layer that closes at >= 10000 docs so the merge target is IVF-clustered,
-- with cluster_replication = 3 so replica spill exists for the native-only
-- fold to exclude.
CREATE INDEX radii_probe_idx ON radii_probe
    USING bm25 (id, vec vector_l2_ops)
    WITH (
        key_field = id,
        cluster_replication = 3,
        target_segment_count = 1,
        mutable_segment_rows = 0,
        layer_sizes = '600kb',
        background_layer_sizes = '0'
    );

INSERT INTO radii_probe
SELECT g, ('[' || repeat((g % 89)::text || ',', 15) || (g % 89)::text || ']')::vector
FROM generate_series(1, 15000) g;

-- The merge produced a clustered segment to read radii from.
SELECT bool_or(vector_format = 'ivf') AS has_ivf
FROM paradedb.vector_info('radii_probe_idx', 'vec');

-- One radius row per cluster: the radii SRF's (segno, cluster_ord) iteration
-- covers exactly the clusters vector_info counts.
SELECT (SELECT count(*) FROM paradedb.ivf_cluster_radii('radii_probe_idx', 'vec'))
     = (SELECT sum(vector_num_centroids)
        FROM paradedb.vector_info('radii_probe_idx', 'vec')
        WHERE vector_format = 'ivf')
    AS one_radius_per_cluster;

-- Radii are finite, non-negative, and the corpus (89 distinct points per
-- dimension pattern) gives at least one cluster real spread.
SELECT count(*) > 0          AS has_rows,
       bool_and(radius >= 0) AS non_negative,
       max(radius) > 0       AS has_spread
FROM paradedb.ivf_cluster_radii('radii_probe_idx', 'vec');

-- Native-only sanity: the 89 distinct line points span ||[88,...] - [0,...]||
-- = 88*4 = 352 end to end; a NATIVE per-cluster radius must be well under a
-- fraction of that span, while a replica-inclusive fold (3 cells per point)
-- could drag clusters toward it. Loose bound: every radius under half the
-- span.
SELECT bool_and(radius < 176) AS radii_native_tight
FROM paradedb.ivf_cluster_radii('radii_probe_idx', 'vec');

-- Asking for a non-vector field is an error, same resolution as vector_info.
SELECT * FROM paradedb.ivf_cluster_radii('radii_probe_idx', 'id');

-- A flat (unmerged, mutable-only) index contributes no rows — the SRF is
-- IVF-segments-only, like vector_info's cluster columns.
DROP TABLE IF EXISTS radii_flat;
CREATE TABLE radii_flat (
    id  int PRIMARY KEY,
    vec vector(16)
);
CREATE INDEX radii_flat_idx ON radii_flat
    USING bm25 (id, vec vector_l2_ops)
    WITH (key_field = id);
INSERT INTO radii_flat
SELECT g, ('[' || repeat((g % 7)::text || ',', 15) || (g % 7)::text || ']')::vector
FROM generate_series(1, 50) g;

SELECT count(*) AS flat_rows FROM paradedb.ivf_cluster_radii('radii_flat_idx', 'vec');

DROP TABLE radii_probe;
DROP TABLE radii_flat;
