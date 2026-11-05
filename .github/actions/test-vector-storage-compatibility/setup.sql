CREATE EXTENSION vector;
CREATE EXTENSION pg_search;

CREATE FUNCTION storage_vector(n integer) RETURNS vector
LANGUAGE SQL IMMUTABLE PARALLEL SAFE AS $$
    SELECT ARRAY(SELECT (((n * 31 + i * 17) % 16411) - 8205)::real
                 FROM generate_series(1, 64) i)::vector
$$;

CREATE TABLE storage_flat (id integer PRIMARY KEY, vec vector(64));
CREATE TABLE storage_clustered (LIKE storage_flat INCLUDING ALL);
INSERT INTO storage_flat SELECT g, storage_vector(g) FROM generate_series(1, 2048) g;
INSERT INTO storage_clustered SELECT * FROM storage_flat;

SET paradedb.vector_clustering_threshold = 1000000;
CREATE INDEX storage_flat_idx ON storage_flat USING paradedb (id, vec vector_l2_ops)
WITH (target_segment_count = 1, mutable_segment_rows = 0, background_layer_sizes = '0',
      vector_fields = '{"vec":{"quantization":false}}');

SET paradedb.vector_clustering_threshold = 64;
CREATE INDEX storage_clustered_idx ON storage_clustered USING paradedb (id, vec vector_l2_ops)
WITH (target_segment_count = 1, mutable_segment_rows = 0, background_layer_sizes = '0',
      max_leaf_size = 32, training_sample_ratio = 1.0,
      vector_fields = '{"vec":{"quantization":{"layers":[1,4]}}}');

-- Settings are persisted in the built index. Clear legacy reloptions before loading
-- the current binary, which recognizes `quantization` instead of `vector_fields`.
ALTER INDEX storage_flat_idx RESET (vector_fields);
ALTER INDEX storage_clustered_idx RESET (vector_fields);
