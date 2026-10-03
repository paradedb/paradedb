-- Unsupported vector storage must identify the index and require a rebuild.
SET enable_seqscan = off;
SET max_parallel_workers_per_gather = 0;
SET paradedb.vector_cluster_max_probe = 1.0;

DO $body$
DECLARE
    error_message text;
    error_hint text;
BEGIN
    IF NOT (SELECT has_vector_indexes FROM vector_upgrade_state) THEN
        RAISE NOTICE 'Vector storage upgrade case skipped: no vector index was created';
        RETURN;
    END IF;

    BEGIN
        PERFORM id FROM vector_upgrade_docs WHERE id @@@ pdb.all()
            ORDER BY embedding <-> '[1,0,0]'::vector LIMIT 1;
        RAISE EXCEPTION 'expected a vector storage REINDEX error';
    EXCEPTION WHEN feature_not_supported THEN
        GET STACKED DIAGNOSTICS
            error_message = MESSAGE_TEXT,
            error_hint = PG_EXCEPTION_HINT;
        IF position('vector_upgrade_idx' IN error_message) = 0
            OR position('REINDEX' IN error_hint) = 0 THEN
            RAISE EXCEPTION 'unexpected vector storage error: %, hint: %', error_message, error_hint;
        END IF;
    END;

    INSERT INTO vector_upgrade_docs
        SELECT g, ARRAY[(100 + g)::real, 0, 0]::vector FROM generate_series(10001, 20000) g;
    UPDATE vector_upgrade_docs SET embedding = '[5000,0,0]' WHERE id = 20000;
    IF EXISTS (SELECT segno FROM vector_upgrade_segments
               EXCEPT SELECT segno FROM paradedb.index_info('vector_upgrade_idx')) THEN
        RAISE EXCEPTION 'merge removed an unsupported vector segment';
    END IF;
    IF (SELECT count(*) FROM vector_upgrade_docs) != 20000 THEN
        RAISE EXCEPTION 'writes did not complete with unsupported vector storage';
    END IF;
    BEGIN
        PERFORM id FROM vector_upgrade_docs WHERE id @@@ pdb.all()
            ORDER BY embedding <-> '[1,0,0]'::vector LIMIT 1;
        RAISE EXCEPTION 'expected a vector storage REINDEX error';
    EXCEPTION WHEN feature_not_supported THEN
        GET STACKED DIAGNOSTICS
            error_message = MESSAGE_TEXT,
            error_hint = PG_EXCEPTION_HINT;
        IF position('vector_upgrade_idx' IN error_message) = 0
            OR position('REINDEX' IN error_hint) = 0 THEN
            RAISE EXCEPTION 'unexpected vector storage error: %, hint: %', error_message, error_hint;
        END IF;
    END;

END
$body$;

SELECT 'REINDEX INDEX CONCURRENTLY vector_upgrade_idx'
WHERE (SELECT has_vector_indexes FROM vector_upgrade_state)
\gexec

DO $body$
DECLARE nearest integer;
BEGIN
    IF NOT (SELECT has_vector_indexes FROM vector_upgrade_state) THEN
        RETURN;
    END IF;
    SELECT id INTO nearest FROM vector_upgrade_docs WHERE id @@@ pdb.all()
        ORDER BY embedding <-> '[1,0,0]'::vector LIMIT 1;
    IF nearest IS DISTINCT FROM 1 THEN
        RAISE EXCEPTION 'unexpected nearest vector after REINDEX: %', nearest;
    END IF;
    SELECT id INTO nearest FROM vector_upgrade_docs WHERE id @@@ pdb.all()
        ORDER BY embedding <-> '[0,1,0]'::vector LIMIT 1;
    IF nearest IS DISTINCT FROM 2 THEN
        RAISE EXCEPTION 'unexpected nearest vector after REINDEX: %', nearest;
    END IF;
END
$body$;
