SET client_min_messages = WARNING;
CREATE EXTENSION IF NOT EXISTS vector;
\i common/common_setup.sql
SET paradedb.vector_clustering_threshold = 64;

CREATE TABLE review_storage (id integer PRIMARY KEY, vec vector(64));
CREATE INDEX review_storage_idx ON review_storage
USING paradedb (id, vec vector_cosine_ops)
WITH (target_segment_count = 1, mutable_segment_rows = 0,
      layer_sizes = '100kb', background_layer_sizes = '0', max_leaf_size = 10, training_sample_ratio = 1.0);
INSERT INTO review_storage
SELECT g, ARRAY(SELECT (((g * 31 + i * 17) % 101) - 50)::real FROM generate_series(1,64) i)::vector
FROM generate_series(1,256) g;
INSERT INTO review_storage
SELECT g, ARRAY(SELECT (((g * 31 + i * 17) % 101) - 50)::real FROM generate_series(1,64) i)::vector
FROM generate_series(257,512) g;
VACUUM review_storage;

SELECT (SELECT quantized FROM paradedb.vector_config('review_storage_idx', 'vec')) AS policy_enabled,
       bool_and(quantized) AS ivf_stores_codes
FROM paradedb.vector_info('review_storage_idx', 'vec');

INSERT INTO review_storage SELECT 513, vec FROM review_storage WHERE id = 1;
SELECT bool_or(vector_format = 'flat') AND bool_or(vector_format = 'ivf') AS mixed_storage,
       (SELECT quantized FROM paradedb.vector_config('review_storage_idx', 'vec')) AS policy_enabled_for_both,
       bool_and(quantized = (vector_format = 'ivf')) AS presence_matches_segment,
       bool_and(layers = ARRAY[1,1]) AS configured_schedule
FROM paradedb.vector_info('review_storage_idx', 'vec');
DROP TABLE review_storage;

CREATE TABLE v4_plain (id integer PRIMARY KEY, vec vector(1024));
INSERT INTO v4_plain SELECT g, ARRAY(SELECT ((g+i)%17+1)::real FROM generate_series(1,1024) i)::vector
FROM generate_series(1,2048) g;
CREATE INDEX v4_plain_idx ON v4_plain USING paradedb (id, vec vector_l2_ops)
WITH (vector_fields='{"vec":{"quantization":false}}', target_segment_count=1);
SELECT vector_format, quantized, layers, quantizer_kinds, bytes_per_row
FROM paradedb.vector_info('v4_plain_idx', 'vec');
SELECT index_oid::regclass AS index_name, quantized, layers, bytes_per_row, settings_version
FROM paradedb.vector_config(index => 'v4_plain_idx', field => 'vec');
DROP INDEX v4_plain_idx;
CREATE INDEX v4_q14_idx ON v4_plain USING paradedb (id, vec vector_l2_ops)
WITH (vector_fields='{"vec":{"quantization":{"layers":[1,4]}}}', target_segment_count=1);
SELECT bool_and(quantized AND layers=ARRAY[1,4] AND bytes_per_row=668
    AND quantizer_kinds=ARRAY['sign','grid']) AS stored_q14
FROM paradedb.vector_info('v4_q14_idx', 'vec');
SELECT index_oid::regclass AS index_name, quantized, layers, bytes_per_row, settings_version
FROM paradedb.vector_config('v4_q14_idx', 'vec');
DROP INDEX v4_q14_idx;
CREATE INDEX v4_q1_idx ON v4_plain USING paradedb (id, vec vector_l2_ops)
WITH (vector_fields='{"vec":{"quantization":{"layers":[1]}}}', target_segment_count=1);
SELECT bool_and(quantized AND layers=ARRAY[1] AND bytes_per_row=144) AS stored_q1
FROM paradedb.vector_info('v4_q1_idx', 'vec');
SELECT index_oid::regclass AS index_name, quantized, layers, bytes_per_row, settings_version
FROM paradedb.vector_config('v4_q1_idx', 'vec');
DROP TABLE v4_plain;

CREATE TABLE config_parent (id integer, vec vector(64)) PARTITION BY RANGE (id);
CREATE TABLE config_low PARTITION OF config_parent FOR VALUES FROM (0) TO (10);
CREATE TABLE config_high PARTITION OF config_parent FOR VALUES FROM (10) TO (20);
CREATE INDEX config_parent_idx ON config_parent USING bm25(id, vec vector_l2_ops);
SELECT index_oid::regclass::text AS index_name, pg_typeof(index_oid), quantized, layers
FROM paradedb.vector_config('config_parent_idx', 'vec') ORDER BY index_oid::regclass::text;
SELECT * FROM paradedb.vector_config('config_parent', 'vec');
CREATE INDEX config_btree_idx ON config_low(id);
SELECT * FROM paradedb.vector_config('config_btree_idx', 'vec');
SELECT * FROM paradedb.vector_config('config_parent_idx', 'id');
SELECT * FROM paradedb.vector_config('config_parent_idx', 'missing');
DROP TABLE config_parent;
