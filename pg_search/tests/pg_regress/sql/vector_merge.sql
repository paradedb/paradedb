-- Preserve every vector and its cluster membership across repeated merges.
SET client_min_messages = WARNING;
CREATE EXTENSION IF NOT EXISTS vector;
\i common/common_setup.sql
-- Tiny fixtures: lower the centroid-training floor.
SET paradedb.vector_min_training_rows = 1;

DROP TABLE IF EXISTS remerge;
CREATE TABLE remerge (
    id  int PRIMARY KEY,
    vec vector(16)
);

-- mutable_segment_rows = 0 routes every insert through immutable segments
-- (foreground merges only fire on an insert-cleanup that created a segment);
-- target_segment_count = 1 keeps the merge policy engaged (merging is
-- disabled while segment_count <= target); background_layer_sizes = '0'
-- keeps every merge in the foreground, deterministic. Every segment —
-- commit or merged — is clustered against the index-level centroid index,
-- with one membership per vector.
-- Centroids train at CREATE INDEX over existing rows, so seed a corpus
-- first; the waves below still drive the segment/merge behavior.
INSERT INTO remerge
SELECT g, ('[' || repeat((g % 89)::text || ',', 15) || (g % 89)::text || ']')::vector
FROM generate_series(-999, 0) g;

CREATE INDEX remerge_idx ON remerge
    USING paradedb (id, vec vector_l2_ops)
    WITH (
        target_segment_count = 1,
        mutable_segment_rows = 0,
        layer_sizes = '600kb',
        background_layer_sizes = '0'
    );

-- Wave 1: deterministic vectors (no random()), enough segments that the
-- 600kb candidate comfortably overfills even if per-segment bytes drift.
INSERT INTO remerge
SELECT g, ('[' || repeat((g % 89)::text || ',', 15) || (g % 89)::text || ']')::vector
FROM generate_series(1, 5000) g;
INSERT INTO remerge
SELECT g, ('[' || repeat((g % 89)::text || ',', 15) || (g % 89)::text || ']')::vector
FROM generate_series(5001, 10000) g;
INSERT INTO remerge
SELECT g, ('[' || repeat((g % 89)::text || ',', 15) || (g % 89)::text || ']')::vector
FROM generate_series(10001, 15000) g;

SELECT count(*) < 4 AS segments_merged
FROM paradedb.index_info('remerge_idx');

-- The merge produced a clustered segment...
SELECT bool_or(vector_format = 'ivf') AS has_ivf
FROM paradedb.vector_info('remerge_idx', 'vec');

-- Clustered segments report the same document count as the index.
SELECT bool_and(v.vector_num_vectors = i.num_docs) AS num_vectors_is_distinct_docs
FROM paradedb.vector_info('remerge_idx', 'vec') v
JOIN paradedb.index_info('remerge_idx') i USING (segno)
WHERE v.vector_format = 'ivf';

-- Every vector belongs to exactly one cluster.
SELECT sum(vector_total_memberships) = sum(vector_num_vectors)
         AS one_membership_per_vector
FROM paradedb.vector_info('remerge_idx', 'vec')
WHERE vector_format = 'ivf';

-- Wave 2: include existing IVF segments in another merge.
ALTER INDEX remerge_idx SET (layer_sizes = '2000kb');
INSERT INTO remerge
SELECT g, ('[' || repeat((g % 89)::text || ',', 15) || (g % 89)::text || ']')::vector
FROM generate_series(15001, 32500) g;
INSERT INTO remerge
SELECT g, ('[' || repeat((g % 89)::text || ',', 15) || (g % 89)::text || ']')::vector
FROM generate_series(32501, 50000) g;

SELECT count(*) < 3 AS segments_merged_again
FROM paradedb.index_info('remerge_idx');
SELECT bool_or(vector_format = 'ivf') AS still_has_ivf
FROM paradedb.vector_info('remerge_idx', 'vec');

SELECT bool_and(v.vector_num_vectors = i.num_docs) AS num_vectors_is_distinct_docs
FROM paradedb.vector_info('remerge_idx', 'vec') v
JOIN paradedb.index_info('remerge_idx') i USING (segno)
WHERE v.vector_format = 'ivf';

-- Exhaustive probing with a limit above the corpus size must return every row once.
SET paradedb.vector_cluster_max_probe = 1.0;
SELECT count(*) AS returned, count(DISTINCT id) AS distinct_ids
FROM (
    SELECT id
    FROM remerge
    WHERE id @@@ pdb.all()
    ORDER BY vec <-> '[1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1]'
    LIMIT 60000
) q;
RESET paradedb.vector_cluster_max_probe;

DROP TABLE remerge;
