SET client_min_messages = WARNING;
CREATE EXTENSION IF NOT EXISTS vector;
\i common/common_setup.sql
SET paradedb.vector_clustering_threshold = 64;

CREATE TABLE review_storage (id integer PRIMARY KEY, vec vector(64));
CREATE INDEX review_storage_idx ON review_storage
USING paradedb (id, vec vector_cosine_ops)
WITH (key_field = 'id', target_segment_count = 1, mutable_segment_rows = 0,
      layer_sizes = '100kb', background_layer_sizes = '0', centroid_ratio = 0.1);
INSERT INTO review_storage
SELECT g, ARRAY(SELECT (((g * 31 + i * 17) % 101) - 50)::real FROM generate_series(1,64) i)::vector
FROM generate_series(1,256) g;
INSERT INTO review_storage
SELECT g, ARRAY(SELECT (((g * 31 + i * 17) % 101) - 50)::real FROM generate_series(1,64) i)::vector
FROM generate_series(257,512) g;
VACUUM review_storage;

SELECT bool_and(configured_quantized) AS policy_enabled,
       bool_and(quantized_storage) AS ivf_stores_codes
FROM paradedb.vector_info('review_storage_idx', 'vec');

INSERT INTO review_storage SELECT 513, vec FROM review_storage WHERE id = 1;
SELECT bool_or(vector_format = 'flat') AND bool_or(vector_format = 'ivf') AS mixed_storage,
       bool_and(configured_quantized) AS policy_enabled_for_both,
       bool_and(quantized_storage = (vector_format = 'ivf')) AS presence_matches_segment,
       bool_and(configured_layers = ARRAY[1,1]) AS configured_schedule
FROM paradedb.vector_info('review_storage_idx', 'vec');
DROP TABLE review_storage;
