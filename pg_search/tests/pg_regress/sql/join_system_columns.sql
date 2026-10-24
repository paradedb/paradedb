-- System columns must come from each source's fetched heap tuple (#6630).
CREATE EXTENSION IF NOT EXISTS pg_search;
SET max_parallel_workers_per_gather = 0;

CREATE TABLE system_columns_a (id int PRIMARY KEY, next_id int, val text, note text);
CREATE TABLE system_columns_b (id int PRIMARY KEY, a_id int, val text);
INSERT INTO system_columns_a SELECT i, i + 1, 'val ' || i, 'original' FROM generate_series(1, 5) i;
-- Different insertion order makes the two sources' CTIDs distinguishable.
INSERT INTO system_columns_b VALUES (14, 99, 'val 14'), (13, 3, 'val 13'), (12, 2, 'val 12'), (11, 1, 'val 11');
CREATE INDEX system_columns_a_idx ON system_columns_a USING paradedb (id, next_id, val);
CREATE INDEX system_columns_b_idx ON system_columns_b USING paradedb (id, a_id, val);
ANALYZE system_columns_a;
ANALYZE system_columns_b;

-- Compare the exact query with JoinScan off and on. Capture rows directly,
-- without wrapping the query in an aggregate that could change its plan.
-- JSON keeps transaction IDs and table OIDs out of the golden output.
CREATE FUNCTION check_join_system_columns(query text, expected_rows int)
RETURNS boolean LANGUAGE plpgsql AS $$
DECLARE
    row_data record;
    reference_rows jsonb[] := '{}';
    actual_rows jsonb[] := '{}';
    plan json;
BEGIN
    PERFORM set_config('paradedb.enable_join_custom_scan', 'off', true);
    FOR row_data IN EXECUTE query LOOP
        reference_rows := array_append(reference_rows, to_jsonb(row_data));
    END LOOP;
    PERFORM set_config('paradedb.enable_join_custom_scan', 'on', true);
    EXECUTE 'EXPLAIN (FORMAT JSON, COSTS OFF) ' || query INTO plan;
    IF plan::text NOT LIKE '%ParadeDB Join Scan%' THEN
        RAISE EXCEPTION 'query did not use JoinScan: %', plan;
    END IF;
    FOR row_data IN EXECUTE query LOOP
        actual_rows := array_append(actual_rows, to_jsonb(row_data));
    END LOOP;
    IF cardinality(reference_rows) <> expected_rows OR cardinality(actual_rows) <> expected_rows THEN
        RAISE EXCEPTION 'expected % rows, PostgreSQL returned %, JoinScan returned %',
            expected_rows, cardinality(reference_rows), cardinality(actual_rows);
    END IF;
    IF actual_rows IS DISTINCT FROM reference_rows THEN
        RAISE EXCEPTION 'JoinScan system columns differ: actual %, expected %', actual_rows, reference_rows;
    END IF;
    RETURN true;
END;
$$;

SELECT check_join_system_columns($query$
SELECT a.id, b.id AS b_id, a.note,
       a.ctid AS a_ctid,
       a.tableoid AS a_tableoid,
       a.xmin AS a_xmin,
       a.cmin AS a_cmin,
       a.xmax AS a_xmax,
       a.cmax AS a_cmax,
       b.ctid AS b_ctid,
       b.tableoid AS b_tableoid,
       b.xmin AS b_xmin,
       b.cmin AS b_cmin,
       b.xmax AS b_xmax,
       b.cmax AS b_cmax
FROM system_columns_a a JOIN system_columns_b b ON a.id = b.a_id
WHERE a.val ||| 'val'
ORDER BY a.id LIMIT 3
$query$, 3) AS inner_system_columns;

SELECT check_join_system_columns($query$
SELECT b.id, a.id AS a_id,
       a.ctid AS a_ctid,
       a.tableoid AS a_tableoid,
       a.xmin AS a_xmin,
       a.cmin AS a_cmin,
       a.xmax AS a_xmax,
       a.cmax AS a_cmax,
       b.ctid AS b_ctid,
       b.tableoid AS b_tableoid,
       b.xmin AS b_xmin,
       b.cmin AS b_cmin,
       b.xmax AS b_xmax,
       b.cmax AS b_cmax
FROM system_columns_b b LEFT JOIN system_columns_a a ON a.id = b.a_id
WHERE b.val ||| 'val'
ORDER BY b.id LIMIT 10
$query$, 4) AS outer_system_columns;

SELECT check_join_system_columns($query$
SELECT a.id, b.id AS b_id,
       a.ctid AS a_ctid,
       a.tableoid AS a_tableoid,
       a.xmin AS a_xmin,
       a.cmin AS a_cmin,
       a.xmax AS a_xmax,
       a.cmax AS a_cmax,
       b.ctid AS b_ctid,
       b.tableoid AS b_tableoid,
       b.xmin AS b_xmin,
       b.cmin AS b_cmin,
       b.xmax AS b_xmax,
       b.cmax AS b_cmax
FROM system_columns_a a JOIN system_columns_a b ON a.next_id = b.id
WHERE a.val ||| 'val'
ORDER BY a.id LIMIT 3
$query$, 3) AS self_join_system_columns;

SELECT check_join_system_columns($query$
SELECT a.ctid AS a_ctid,
       a.tableoid AS a_tableoid,
       a.xmin AS a_xmin,
       a.cmin AS a_cmin,
       a.xmax AS a_xmax,
       a.cmax AS a_cmax,
       b.ctid AS b_ctid,
       b.tableoid AS b_tableoid,
       b.xmin AS b_xmin,
       b.cmin AS b_cmin,
       b.xmax AS b_xmax,
       b.cmax AS b_cmax
FROM system_columns_a a JOIN system_columns_b b ON a.id = b.a_id
WHERE a.val ||| 'val'
ORDER BY a.id LIMIT 3
$query$, 3) AS only_system_columns;

-- Updating an unindexed column moves the visible heap tuple. After VACUUM,
-- system columns must describe that tuple, not the old CTID in the index.
CREATE TEMP TABLE system_columns_old_ctid AS SELECT ctid AS old_ctid FROM system_columns_a WHERE id = 2;
UPDATE system_columns_a SET note = 'updated' WHERE id = 2;
VACUUM (FREEZE, TRUNCATE false) system_columns_a;
SET paradedb.enable_join_custom_scan = off;
SELECT a.ctid <> old.old_ctid AS updated_tuple_moved
FROM system_columns_a a CROSS JOIN system_columns_old_ctid old WHERE a.id = 2;

SELECT check_join_system_columns($query$
SELECT a.id, b.id AS b_id, a.note,
       a.ctid AS a_ctid,
       a.tableoid AS a_tableoid,
       a.xmin AS a_xmin,
       a.cmin AS a_cmin,
       a.xmax AS a_xmax,
       a.cmax AS a_cmax,
       b.ctid AS b_ctid,
       b.tableoid AS b_tableoid,
       b.xmin AS b_xmin,
       b.cmin AS b_cmin,
       b.xmax AS b_xmax,
       b.cmax AS b_cmax
FROM system_columns_a a JOIN system_columns_b b ON a.id = b.a_id
WHERE a.val ||| 'val'
ORDER BY a.id LIMIT 3
$query$, 3) AS updated_system_columns;

DROP FUNCTION check_join_system_columns(text, int);
DROP TABLE system_columns_a, system_columns_b;
