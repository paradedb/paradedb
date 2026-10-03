-- Vector storage upgrade coverage requires the vector operator class.
CREATE TABLE vector_upgrade_state (has_vector_indexes boolean NOT NULL);

DO $body$
DECLARE
    supported boolean;
BEGIN
    SELECT EXISTS (
        SELECT 1 FROM pg_opclass c JOIN pg_am a ON a.oid = c.opcmethod
        WHERE c.opcname = 'vector_l2_ops' AND a.amname = 'bm25'
    ) INTO supported;
    INSERT INTO vector_upgrade_state VALUES (supported);
    IF NOT supported THEN
        RAISE NOTICE 'Vector storage upgrade case skipped: vector indexes are unavailable';
        RETURN;
    END IF;

    PERFORM set_config('paradedb.vector_clustering_threshold', '64', true);
    CREATE TABLE vector_upgrade_docs (id integer PRIMARY KEY, embedding vector(3));
    INSERT INTO vector_upgrade_docs VALUES
        (1, '[1,0,0]'), (2, '[0,1,0]'), (3, '[0,0,1]');
    INSERT INTO vector_upgrade_docs
        SELECT g, ARRAY[(100 + g)::real, 0, 0]::vector FROM generate_series(4, 2048) g;
    CREATE INDEX vector_upgrade_idx ON vector_upgrade_docs
        USING bm25 (id, embedding vector_l2_ops)
        WITH (key_field = 'id', target_segment_count = 1,
              mutable_segment_rows = 0, layer_sizes = '1GB', background_layer_sizes = '0');
    IF NOT EXISTS (SELECT FROM paradedb.vector_info('vector_upgrade_idx', 'embedding')
                   WHERE vector_num_centroids > 0) THEN
        RAISE EXCEPTION 'upgrade fixture must contain a clustered vector segment';
    END IF;
    EXECUTE format('ALTER INDEX vector_upgrade_idx SET (layer_sizes = %L)',
        (SELECT max(byte_size)::bigint || ' bytes' FROM paradedb.index_info('vector_upgrade_idx')));
    CREATE TABLE vector_upgrade_segments AS SELECT segno FROM paradedb.index_info('vector_upgrade_idx');
END
$body$;
