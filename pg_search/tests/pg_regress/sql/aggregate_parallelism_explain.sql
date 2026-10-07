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
    launched int;
    expected int;
    line text;
    labels text[] := '{}';
BEGIN
    EXECUTE 'EXPLAIN (VERBOSE, FORMAT JSON) ' || query INTO plan;
    ASSERT NOT (plan #> '{0,Plan}') ? 'Aggregate Parallelism';
    PERFORM set_config('paradedb.explain_recursive_estimates', 'true', true);
    EXECUTE 'EXPLAIN (VERBOSE, FORMAT JSON) ' || query INTO plan;
    details := plan #> '{0,Plan,Aggregate Parallelism}';
    ASSERT details ->> 'Status' = 'selected at execution time';
    ASSERT NOT details ? 'Workers Launched';
    ASSERT (details ->> 'Estimated Query Work')::bigint > 0;
    EXECUTE 'EXPLAIN (ANALYZE, FORMAT JSON) ' || query INTO plan;
    ASSERT NOT (plan #> '{0,Plan}') ? 'Aggregate Parallelism';

    FOREACH workers IN ARRAY ARRAY[0, 1, 2] LOOP
        PERFORM set_config('max_parallel_workers_per_gather', workers::text, true);
        FOREACH leader IN ARRAY ARRAY[false, true] LOOP
            PERFORM set_config('parallel_leader_participation', leader::text, true);
            EXECUTE 'EXPLAIN (ANALYZE, VERBOSE, TIMING OFF, FORMAT JSON) ' || query INTO plan;
            details := plan #> '{0,Plan,Aggregate Parallelism}';
            ASSERT (details ->> 'Executions')::int = 1;
            ASSERT (details ->> 'Segments')::int >= 2;
            expected := least(workers, (details ->> 'Segments')::int);
            IF leader THEN expected := greatest(expected - 1, 0); END IF;
            requested := (details ->> 'Workers Requested')::int;
            launched := (details ->> 'Workers Launched')::int;
            ASSERT requested = expected;
            ASSERT launched >= 0 AND launched <= requested;
            ASSERT (details ->> 'Leader Participated')::boolean = (leader OR launched = 0);
            ASSERT (details ->> 'Max Workers Per Gather')::int = workers;
            ASSERT length(details ->> 'Reason') > 0;
        END LOOP;
    END LOOP;

    PERFORM set_config('max_parallel_workers', '0', true);
    EXECUTE 'EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON) ' || query INTO plan;
    details := plan #> '{0,Plan,Aggregate Parallelism}';
    ASSERT (details ->> 'Workers Requested')::int = 1;
    ASSERT (details ->> 'Workers Launched')::int = 0;
    ASSERT details ->> 'Reason' = 'parallel worker limit is zero';

    EXECUTE 'EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON) ' || query INTO plan;
    details := plan #> '{0,Plan,Aggregate Parallelism}';
    ASSERT (details ->> 'Estimated Query Work')::bigint > 0;
    ASSERT details ->> 'Serial/Parallel Cost Model' = 'not used';
    EXECUTE 'EXPLAIN (ANALYZE, VERBOSE, COSTS OFF, FORMAT JSON) ' || query INTO plan;
    ASSERT NOT (plan #> '{0,Plan,Aggregate Parallelism}') ? 'Estimated Query Work';

    EXECUTE $q$EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON)
        SELECT pdb.agg('{"value_count":{"field":"ctid"}}'::jsonb, 'raw')
        FROM parallelism_docs WHERE body ||| 'database'$q$ INTO plan;
    details := plan #> '{0,Plan,Aggregate Parallelism}';
    ASSERT details ->> 'Reason' = 'count fast path';
    ASSERT (details ->> 'Workers Requested')::int = 0;
    ASSERT (details ->> 'Workers Launched')::int = 0;
    ASSERT (details ->> 'Leader Participated')::boolean;

    EXECUTE 'EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON) ' || query || ' LIMIT 0' INTO plan;
    details := jsonb_path_query_first(plan, '$.**."Aggregate Parallelism"');
    ASSERT (details ->> 'Executions')::int = 0;
    ASSERT details ->> 'Status' = 'not executed';
    ASSERT NOT details ? 'Workers Launched';

    EXECUTE $q$EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON)
        SELECT (SELECT count(*) FROM parallelism_docs WHERE body ||| terms.term)
        FROM (VALUES ('database'), ('missing')) terms(term)$q$ INTO plan;
    details := jsonb_path_query_first(plan, '$.**."Aggregate Parallelism"');
    ASSERT (details ->> 'Executions')::int = 2;
    ASSERT details ->> 'Scope' = 'last execution';

    FOR line IN EXECUTE 'EXPLAIN (ANALYZE, VERBOSE) ' || query LOOP
        labels := array_append(labels, split_part(trim(line), ':', 1));
    END LOOP;
    ASSERT labels @> ARRAY['Aggregate Parallelism', 'Workers Requested',
                          'Workers Launched', 'Leader Participated', 'Reason'];
END;
$$;
SELECT check_aggregate_parallelism();

DROP FUNCTION check_aggregate_parallelism();
DROP TABLE parallelism_docs;
\i common/common_cleanup.sql
