SET enable_seqscan = off;
SET max_parallel_workers_per_gather = 0;
SET paradedb.vector_cluster_max_probe = 1.0;

DO $$
DECLARE
    table_name text;
    query_id integer;
    nearest integer;
    indexed_count bigint;
    heap_count bigint;
BEGIN
    IF (SELECT bool_and(vector_format = 'flat' AND NOT quantized)
        FROM paradedb.vector_info('storage_flat_idx', 'vec')) IS DISTINCT FROM true THEN
        RAISE EXCEPTION 'flat fixture must contain unquantized flat segments';
    END IF;
    IF (SELECT bool_and(vector_format = 'ivf' AND quantized AND layers = ARRAY[1,4])
        FROM paradedb.vector_info('storage_clustered_idx', 'vec')) IS DISTINCT FROM true THEN
        RAISE EXCEPTION 'clustered fixture must contain IVF segments with both quantization layers';
    END IF;
    FOREACH table_name IN ARRAY ARRAY['storage_flat', 'storage_clustered'] LOOP
        EXECUTE format('SELECT count(*) FROM %I', table_name) INTO heap_count;
        EXECUTE format('SELECT count(*) FROM %I WHERE id @@@ pdb.all()', table_name)
            INTO indexed_count;
        IF indexed_count IS DISTINCT FROM heap_count THEN
            RAISE EXCEPTION 'indexed row count differs for %: % vs %',
                table_name, indexed_count, heap_count;
        END IF;
        FOREACH query_id IN ARRAY ARRAY[1, 64, 1024] LOOP
            EXECUTE format('SELECT id FROM %I WHERE id @@@ pdb.all()
                            ORDER BY vec <-> storage_vector(%s), id LIMIT 1', table_name, query_id)
                INTO nearest;
            IF nearest IS DISTINCT FROM query_id THEN
                RAISE EXCEPTION 'wrong nearest vector in %: expected %, got %',
                    table_name, query_id, nearest;
            END IF;
        END LOOP;
    END LOOP;
END
$$;
