-- Vector storage upgrade coverage requires the vector operator class.
CREATE TABLE vector_upgrade_state (has_vector_indexes boolean NOT NULL);

DO $body$
DECLARE
    supported boolean;
BEGIN
    SELECT EXISTS (
        SELECT 1 FROM pg_opclass c JOIN pg_am a ON a.oid = c.opcmethod
        WHERE c.opcname = 'vector_l2_ops' AND a.amname = 'paradedb'
    ) INTO supported;
    INSERT INTO vector_upgrade_state VALUES (supported);
    IF NOT supported THEN
        RAISE NOTICE 'Vector storage upgrade case skipped: vector indexes are unavailable';
        RETURN;
    END IF;

    -- 0.25.0, the oldest version we test upgrades from in CI, has a fixed
    -- clustering threshold of 10,000 rows.
    CREATE TABLE vector_upgrade_docs (id integer PRIMARY KEY, embedding vector(3));
    INSERT INTO vector_upgrade_docs VALUES
        (1, '[1,0,0]'), (2, '[0,1,0]'), (3, '[0,0,1]');
    INSERT INTO vector_upgrade_docs
        SELECT g, ARRAY[(100 + g)::real, 0, 0]::vector FROM generate_series(4, 10000) g;
    CREATE INDEX vector_upgrade_idx ON vector_upgrade_docs
        USING paradedb (id, embedding vector_l2_ops)
        WITH (key_field = 'id', target_segment_count = 1,
              mutable_segment_rows = 0, layer_sizes = '1GB', background_layer_sizes = '0');
    -- 0.25.0 initially writes flat vectors and clusters only during a merge.
    -- Size the layer for all initial segments and insert an equally sized batch so the
    -- foreground merge exceeds the layer size and the clustering threshold.
    EXECUTE format('ALTER INDEX vector_upgrade_idx SET (layer_sizes = %L)',
        (SELECT sum(byte_size)::bigint || ' bytes' FROM paradedb.index_info('vector_upgrade_idx')));
END
$body$;

-- Commit the initial segments before writing the batch that triggers their merge.
SELECT 'INSERT INTO vector_upgrade_docs
        SELECT g, ARRAY[(100 + g)::real, 0, 0]::vector FROM generate_series(10001, 20000) g'
WHERE (SELECT has_vector_indexes FROM vector_upgrade_state)
\gexec

DO $body$
BEGIN
    IF NOT (SELECT has_vector_indexes FROM vector_upgrade_state) THEN
        RETURN;
    END IF;
    IF NOT EXISTS (SELECT FROM paradedb.vector_info('vector_upgrade_idx', 'embedding')
                   WHERE vector_num_centroids > 0) THEN
        RAISE EXCEPTION 'upgrade fixture must contain a clustered vector segment';
    END IF;
    EXECUTE format('ALTER INDEX vector_upgrade_idx SET (layer_sizes = %L)',
        (SELECT max(byte_size)::bigint || ' bytes' FROM paradedb.index_info('vector_upgrade_idx')));
    CREATE TABLE vector_upgrade_segments AS SELECT segno FROM paradedb.index_info('vector_upgrade_idx');
END
$body$;
