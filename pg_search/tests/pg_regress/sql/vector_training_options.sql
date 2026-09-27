SET client_min_messages = WARNING;
CREATE EXTENSION IF NOT EXISTS vector;
\i common/common_setup.sql
SET paradedb.vector_clustering_threshold = 64;
SET paradedb.vector_cluster_max_probe = 1.0;

CREATE TABLE training_ratio_options (id integer PRIMARY KEY, vec vector(3));
INSERT INTO training_ratio_options
SELECT g, ARRAY[g % 17, g % 23, g % 31]::vector
FROM generate_series(1, 256) g;

-- Omitting both options uses the default sampling fraction and leaf size.
CREATE INDEX training_ratio_idx ON training_ratio_options
USING paradedb (id, vec vector_l2_ops)
WITH (target_segment_count = 1, mutable_segment_rows = 0,
      vector_fields = '{"vec":{"quantization":false}}');

-- Sampling and leaf size are independent settings.
ALTER INDEX training_ratio_idx SET (max_leaf_size = 20, training_sample_ratio = 0.25);
SELECT reloptions @> ARRAY['max_leaf_size=20', 'training_sample_ratio=0.25']
    AS stores_training_options
FROM pg_class WHERE oid = 'training_ratio_idx'::regclass;
REINDEX INDEX training_ratio_idx;
SELECT count(*) AS indexed_rows
FROM (
    SELECT id FROM training_ratio_options
    WHERE id @@@ pdb.all()
    ORDER BY vec <-> '[1,2,3]'
    LIMIT 256
) matches;

-- Both endpoints are accepted; invalid fractions and retired names fail.
ALTER INDEX training_ratio_idx SET (training_sample_ratio = 0.000001);
ALTER INDEX training_ratio_idx SET (training_sample_ratio = 1.0);
ALTER INDEX training_ratio_idx SET (training_sample_ratio = 0);
ALTER INDEX training_ratio_idx SET (training_sample_ratio = -0.1);
ALTER INDEX training_ratio_idx SET (training_sample_ratio = 1.01);
ALTER INDEX training_ratio_idx SET (training_sample_ratio = 'NaN');
ALTER INDEX training_ratio_idx SET (training_sample_ratio = 'Infinity');
ALTER INDEX training_ratio_idx SET (max_leaf_size = 1);
ALTER INDEX training_ratio_idx SET (max_leaf_size = 2147483647);
ALTER INDEX training_ratio_idx SET (max_leaf_size = 0);
ALTER INDEX training_ratio_idx SET (max_leaf_size = -1);
ALTER INDEX training_ratio_idx SET (max_leaf_size = 'invalid');
ALTER INDEX training_ratio_idx SET (max_leaf_size = 2147483648);
ALTER INDEX training_ratio_idx SET (centroid_ratio = 0.01);
ALTER INDEX training_ratio_idx SET (training_samples_per_centroid = 32);
SELECT reloptions @> ARRAY['training_sample_ratio=1.0', 'max_leaf_size=2147483647']
    AS invalid_updates_preserve_options
FROM pg_class WHERE oid = 'training_ratio_idx'::regclass;

DROP TABLE training_ratio_options;
