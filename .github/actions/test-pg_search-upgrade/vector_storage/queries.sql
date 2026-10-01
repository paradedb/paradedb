-- Unsupported vector storage must identify the index and require a rebuild.
SET enable_seqscan = off;
SET max_parallel_workers_per_gather = 0;

DO $body$
DECLARE
    error_message text;
    error_hint text;
    nearest integer;
BEGIN
    IF NOT (SELECT has_vector_indexes FROM vector_upgrade_state) THEN
        RAISE NOTICE 'Vector storage upgrade case skipped: no vector index was created';
        RETURN;
    END IF;

    BEGIN
        PERFORM id FROM vector_upgrade_docs WHERE id @@@ pdb.all()
            ORDER BY embedding <-> '[1,0,0]'::vector LIMIT 1;
        RAISE EXCEPTION 'expected a vector storage REINDEX error';
    EXCEPTION WHEN OTHERS THEN
        GET STACKED DIAGNOSTICS
            error_message = MESSAGE_TEXT,
            error_hint = PG_EXCEPTION_HINT;
        IF position('vector_upgrade_idx' IN error_message) = 0
            OR position('REINDEX' IN error_hint) = 0 THEN
            RAISE EXCEPTION 'unexpected vector storage error: %, hint: %', error_message, error_hint;
        END IF;
    END;

    REINDEX INDEX vector_upgrade_idx;
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
