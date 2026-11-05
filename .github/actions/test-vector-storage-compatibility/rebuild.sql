-- Run unchanged after each binary swap: old files must be rejected in either direction.
SET enable_seqscan = off;
SET max_parallel_workers_per_gather = 0;
SET paradedb.vector_cluster_max_probe = 1.0;
SET paradedb.vector_clustering_threshold = 64;

CREATE OR REPLACE FUNCTION expect_storage_rebuild(query text, index_name text) RETURNS void
LANGUAGE plpgsql AS $$
DECLARE message text; hint text;
BEGIN
    BEGIN
        EXECUTE query;
        RAISE EXCEPTION 'expected incompatible storage for %', index_name;
    EXCEPTION WHEN feature_not_supported THEN
        GET STACKED DIAGNOSTICS message = MESSAGE_TEXT, hint = PG_EXCEPTION_HINT;
        IF position(index_name IN message) = 0
           OR hint IS DISTINCT FROM format('Rebuild index "%s" with REINDEX.', index_name) THEN
            RAISE EXCEPTION 'unexpected storage error: %, hint: %', message, hint;
        END IF;
    END;
END
$$;

DO $$
DECLARE
    table_name text;
    index_name text;
    old_segments text[];
    current_segments text[];
    row_count bigint;
    indexed_count bigint;
    layer_bytes bigint;
    query text;
BEGIN
    FOREACH table_name IN ARRAY ARRAY['storage_flat', 'storage_clustered'] LOOP
        index_name := table_name || '_idx';
        query := format('SELECT id FROM %I WHERE id @@@ pdb.all()
                         ORDER BY vec <-> storage_vector(1), id LIMIT 1', table_name);
        PERFORM expect_storage_rebuild(query, index_name);
        PERFORM expect_storage_rebuild(
            format('SELECT * FROM paradedb.vector_info(%L, ''vec'')', index_name), index_name);

        EXECUTE format('SELECT count(*) FROM %I', table_name) INTO row_count;
        EXECUTE format('SELECT count(*) FROM %I WHERE id @@@ pdb.all()', table_name)
            INTO indexed_count;
        IF indexed_count IS DISTINCT FROM row_count THEN
            RAISE EXCEPTION 'BM25 could not read incompatible vector index %', index_name;
        END IF;
        SELECT array_agg(segno), max(byte_size)::bigint INTO old_segments, layer_bytes
            FROM paradedb.index_info(index_name::regclass);
        EXECUTE format('ALTER INDEX %I SET (layer_sizes = %L)', index_name, layer_bytes || ' bytes');
        EXECUTE format('INSERT INTO %I SELECT g, storage_vector(g)
                        FROM generate_series(%s, %s) g', table_name, row_count + 1, row_count * 2);
        EXECUTE format('UPDATE %I SET vec = vec WHERE id = %s', table_name, row_count * 2);
        SELECT array_agg(segno) INTO current_segments FROM paradedb.index_info(index_name::regclass);
        IF NOT old_segments <@ current_segments THEN
            RAISE EXCEPTION 'merge removed an incompatible segment of %', index_name;
        END IF;
        EXECUTE format('SELECT count(*) FROM %I WHERE id @@@ pdb.all()', table_name)
            INTO indexed_count;
        IF indexed_count IS DISTINCT FROM row_count * 2 THEN
            RAISE EXCEPTION 'writes did not remain searchable in %', index_name;
        END IF;
        PERFORM expect_storage_rebuild(query, index_name);
    END LOOP;
END
$$;

-- Each binary uses its own build option; clear it again after rebuilding so
-- the next binary can parse the index definition. Extension SQL stays unchanged.
\if :legacy_writer
ALTER INDEX storage_flat_idx SET (vector_fields = '{"vec":{"quantization":false}}');
ALTER INDEX storage_clustered_idx SET (vector_fields = '{"vec":{"quantization":{"layers":[1,4]}}}');
\else
ALTER INDEX storage_flat_idx SET (quantization = '0');
ALTER INDEX storage_clustered_idx SET (quantization = '[1,4]');
\endif

SET paradedb.vector_clustering_threshold = 1000000;
REINDEX INDEX CONCURRENTLY storage_flat_idx;
SET paradedb.vector_clustering_threshold = 64;
REINDEX INDEX CONCURRENTLY storage_clustered_idx;

\if :legacy_writer
ALTER INDEX storage_flat_idx RESET (vector_fields);
ALTER INDEX storage_clustered_idx RESET (vector_fields);
\else
ALTER INDEX storage_flat_idx RESET (quantization);
ALTER INDEX storage_clustered_idx RESET (quantization);
\endif
