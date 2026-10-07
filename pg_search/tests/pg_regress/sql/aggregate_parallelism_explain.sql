\i common/common_setup.sql

CREATE TABLE parallelism_docs(id int, body text);
INSERT INTO parallelism_docs SELECT id, 'database search' FROM generate_series(1, 10000) id;
CREATE INDEX parallelism_idx ON parallelism_docs USING bm25(body)
    WITH (partition_by = 'ctid', target_segment_count = 2,
          layer_sizes = '10TB', background_layer_sizes = '0');
VACUUM ANALYZE parallelism_docs;

CREATE FUNCTION check_aggregate_parallelism() RETURNS void LANGUAGE plpgsql AS $$
DECLARE
    query text := $q$SELECT count(*) FROM parallelism_docs WHERE body ||| 'database'$q$;
    plan jsonb;
    details jsonb;
    workers int;
    leader boolean;
    requested int;
    used int;
    expected int;
    keys text[];
    line text;
    labels text[] := '{}';
BEGIN
    PERFORM set_config('paradedb.explain_recursive_estimates', 'false', true);
    EXECUTE 'EXPLAIN (VERBOSE, FORMAT JSON) ' || query INTO plan;
    ASSERT NOT (plan #> '{0,Plan}') ? 'Aggregate Parallelism';
    EXECUTE 'EXPLAIN (ANALYZE, FORMAT JSON) ' || query INTO plan;
    details := plan #> '{0,Plan,Aggregate Parallelism}';
    SELECT array_agg(key ORDER BY key) INTO keys FROM jsonb_object_keys(details) AS key;
    ASSERT keys = ARRAY['Estimated Query Work', 'Parallel Threshold',
                        'Workers Requested', 'Workers Used'];
    ASSERT (details ->> 'Estimated Query Work')::bigint > 0;
    ASSERT details ->> 'Parallel Threshold' = 'not applied';

    FOREACH workers IN ARRAY ARRAY[0, 1, 2] LOOP
        PERFORM set_config('max_parallel_workers_per_gather', workers::text, true);
        FOREACH leader IN ARRAY ARRAY[false, true] LOOP
            PERFORM set_config('parallel_leader_participation', leader::text, true);
            EXECUTE 'EXPLAIN (ANALYZE, TIMING OFF, FORMAT JSON) ' || query INTO plan;
            details := plan #> '{0,Plan,Aggregate Parallelism}';
            expected := workers;
            IF leader THEN expected := greatest(expected - 1, 0); END IF;
            requested := (details ->> 'Workers Requested')::int;
            used := (details ->> 'Workers Used')::int;
            ASSERT requested = expected;
            ASSERT used >= 0 AND used <= requested;
        END LOOP;
    END LOOP;

    PERFORM set_config('max_parallel_workers', '0', true);
    EXECUTE 'EXPLAIN (ANALYZE, FORMAT JSON) ' || query INTO plan;
    details := plan #> '{0,Plan,Aggregate Parallelism}';
    ASSERT (details ->> 'Workers Requested')::int = 1;
    ASSERT (details ->> 'Workers Used')::int = 0;

    EXECUTE 'EXPLAIN (ANALYZE, FORMAT JSON) ' || query INTO plan;
    details := plan #> '{0,Plan,Aggregate Parallelism}';
    ASSERT (details ->> 'Estimated Query Work')::bigint > 0;
    EXECUTE 'EXPLAIN (ANALYZE, COSTS OFF, FORMAT JSON) ' || query INTO plan;
    details := plan #> '{0,Plan,Aggregate Parallelism}';
    SELECT array_agg(key ORDER BY key) INTO keys FROM jsonb_object_keys(details) AS key;
    ASSERT keys = ARRAY['Workers Requested', 'Workers Used'];

    EXECUTE $q$EXPLAIN (ANALYZE, FORMAT JSON)
        SELECT pdb.agg('{"value_count":{"field":"ctid"}}'::jsonb, 'raw')
        FROM parallelism_docs WHERE body ||| 'database'$q$ INTO plan;
    details := plan #> '{0,Plan,Aggregate Parallelism}';
    ASSERT (details ->> 'Workers Requested')::int = 0;
    ASSERT (details ->> 'Workers Used')::int = 0;

    EXECUTE 'EXPLAIN (ANALYZE, FORMAT JSON) ' || query || ' LIMIT 0' INTO plan;
    details := jsonb_path_query_first(plan, '$.**."Aggregate Parallelism"');
    ASSERT details IS NULL;

    EXECUTE $q$EXPLAIN (ANALYZE, FORMAT JSON)
        SELECT (SELECT count(*) FROM parallelism_docs WHERE body ||| terms.term)
        FROM (VALUES ('database'), ('missing')) terms(term)$q$ INTO plan;
    details := jsonb_path_query_first(plan, '$.**."Aggregate Parallelism"');
    ASSERT (details ->> 'Workers Requested')::int = 1;
    ASSERT (details ->> 'Workers Used')::int = 0;
    ASSERT (details ->> 'Estimated Query Work')::bigint = 0;

    FOR line IN EXECUTE 'EXPLAIN (ANALYZE) ' || query LOOP
        labels := array_append(labels, split_part(trim(line), ':', 1));
    END LOOP;
    ASSERT labels @> ARRAY['Aggregate Parallelism', 'Workers Requested',
                          'Workers Used', 'Estimated Query Work', 'Parallel Threshold'];
END;
$$;
SELECT check_aggregate_parallelism();

DROP FUNCTION check_aggregate_parallelism();
DROP TABLE parallelism_docs;
\i common/common_cleanup.sql
