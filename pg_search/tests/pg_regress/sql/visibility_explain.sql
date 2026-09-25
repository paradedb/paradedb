\i common/common_setup.sql

CREATE TABLE visibility_stats_docs(id int PRIMARY KEY, title text, padding text)
    WITH (autovacuum_enabled = false, fillfactor = 70);
INSERT INTO visibility_stats_docs
    SELECT id, 'database', repeat('x', 600) FROM generate_series(1, 10000) id;
CREATE INDEX visibility_stats_idx ON visibility_stats_docs USING paradedb(id, title)
    WITH (sort_by = 'ctid ASC NULLS FIRST', mutable_segment_rows = 0,
          target_segment_count = 2, layer_sizes = '10TB', background_layer_sizes = '0');
INSERT INTO visibility_stats_docs
    SELECT id, 'database', repeat('x', 600) FROM generate_series(10001, 20000) id;

-- Check counters without depending on exact segment or heap-page counts.
CREATE FUNCTION check_visibility_stats(phase text) RETURNS void LANGUAGE plpgsql AS $$
DECLARE
    query text := $q$SELECT count(*) FROM visibility_stats_docs WHERE title === 'database'$q$;
    plan jsonb;
    visibility jsonb;
    expected_enabled jsonb;
    expected_totals bigint[];
    skipped bigint;
    checked bigint;
    total bigint;
    required bigint;
    workers int;
    enabled boolean;
    line text;
    labels text[] := '{}';
BEGIN
    EXECUTE 'EXPLAIN (FORMAT JSON) ' || query INTO plan;
    ASSERT NOT (plan #> '{0,Plan}') ? 'Visibility';
    FOREACH workers IN ARRAY ARRAY[0, 2] LOOP
        PERFORM set_config('max_parallel_workers_per_gather', workers::text, true);
        FOREACH enabled IN ARRAY ARRAY[true, false] LOOP
            PERFORM set_config('paradedb.enable_visibility_map_shortcuts', enabled::text, true);
            EXECUTE 'EXPLAIN (ANALYZE, FORMAT JSON) ' || query INTO plan;
            visibility := plan #> '{0,Plan,Visibility}';
            skipped := (visibility ->> 'Segments Skipped')::bigint;
            checked := (visibility ->> 'Segments Checked')::bigint;
            total := (visibility ->> 'Blocks Total')::bigint;
            required := (visibility ->> 'Blocks Requiring Checks')::bigint;
            ASSERT skipped IS NOT NULL AND checked IS NOT NULL
                AND total IS NOT NULL AND required IS NOT NULL;
            ASSERT skipped + checked >= 2 AND total > 0 AND required <= total;
            IF NOT enabled OR phase = 'dirty' THEN
                ASSERT skipped = 0 AND required = total;
            ELSIF phase = 'visible' THEN
                ASSERT checked = 0 AND required = 0;
            ELSE
                ASSERT checked > 0 AND required > 0 AND required < total;
            END IF;
            IF expected_totals IS NULL THEN
                expected_totals := ARRAY[skipped + checked, total];
            ELSE
                ASSERT expected_totals = ARRAY[skipped + checked, total];
            END IF;
            IF enabled THEN
                IF expected_enabled IS NULL THEN
                    expected_enabled := visibility;
                ELSE
                    ASSERT expected_enabled = visibility;
                END IF;
            END IF;
        END LOOP;
    END LOOP;
    FOR line IN EXECUTE 'EXPLAIN (ANALYZE) ' || query LOOP
        labels := array_append(labels, split_part(trim(line), ':', 1));
    END LOOP;
    ASSERT labels @> ARRAY['Visibility', 'Segments Skipped', 'Segments Checked',
                           'Blocks Total', 'Blocks Requiring Checks'];
END;
$$;

SELECT check_visibility_stats('dirty');
VACUUM (INDEX_CLEANUP ON) visibility_stats_docs;
SELECT check_visibility_stats('visible');
UPDATE visibility_stats_docs SET padding = 'changed' WHERE id % 97 = 0;
SELECT check_visibility_stats('mixed');

DROP FUNCTION check_visibility_stats(text);
DROP TABLE visibility_stats_docs;
\i common/common_cleanup.sql
