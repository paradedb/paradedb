\i common/common_setup.sql

-- Every executor path that reads a ParadeDB index counts in pg_stat_user_indexes.

DROP TABLE IF EXISTS scan_stats, scan_stats_other;
CREATE TABLE scan_stats (id serial PRIMARY KEY, description text, category text);
INSERT INTO scan_stats (description, category) VALUES
    ('running shoes', 'footwear'),
    ('leather shoes', 'footwear'),
    ('wool socks', 'clothing'),
    ('rain jacket', 'clothing');
CREATE INDEX scan_stats_idx ON scan_stats USING paradedb (id, description, category);

CREATE TABLE scan_stats_other (id serial PRIMARY KEY, description text, category text);
INSERT INTO scan_stats_other (description, category) VALUES
    ('hiking socks', 'clothing'),
    ('dress shoes', 'footwear');
CREATE INDEX scan_stats_other_idx ON scan_stats_other USING paradedb (id, description, category);

-- Only the scan nodes of a plan, so the output does not depend on the planner's other choices.
CREATE FUNCTION scan_nodes(query text) RETURNS SETOF text LANGUAGE plpgsql AS $$
DECLARE
    line text;
BEGIN
    FOR line IN EXECUTE 'EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) ' || query LOOP
        IF line ~ '(Custom|Index|Bitmap Heap|Seq) Scan' THEN
            RETURN NEXT regexp_replace(line, '^\s*(->\s*)?', '');
        END IF;
    END LOOP;
END;
$$;

-- The counters reach the view after the backend flushes them.
CREATE FUNCTION scan_counts() RETURNS TABLE (index_name name, idx_scan bigint, idx_tup_read bigint)
LANGUAGE sql AS $$
    SELECT indexrelname, idx_scan, idx_tup_read
    FROM pg_stat_user_indexes
    WHERE indexrelname IN ('scan_stats_idx', 'scan_stats_other_idx')
    ORDER BY indexrelname;
$$;

SELECT pg_stat_force_next_flush();
SELECT * FROM scan_counts();

-- ParadeDB custom scan.
SELECT * FROM scan_nodes($$SELECT id FROM scan_stats WHERE description @@@ 'shoes' ORDER BY id$$);
SELECT id FROM scan_stats WHERE description @@@ 'shoes' ORDER BY id;
SELECT pg_stat_force_next_flush();
SELECT * FROM scan_counts();

-- A rescan counts again, once per outer row.
SELECT * FROM scan_nodes($$SELECT o.id, (SELECT count(*) FROM scan_stats s WHERE s.description @@@ 'shoes' AND s.category = o.category) FROM scan_stats o ORDER BY o.id$$);
SELECT o.id, (SELECT count(*) FROM scan_stats s WHERE s.description @@@ 'shoes' AND s.category = o.category) FROM scan_stats o ORDER BY o.id;
SELECT pg_stat_force_next_flush();
SELECT * FROM scan_counts();

-- Postgres index scan through the access method.
SET paradedb.enable_custom_scan = false;
SET enable_indexscan = on;
SET enable_bitmapscan = off;
SELECT * FROM scan_nodes($$SELECT id FROM scan_stats WHERE description @@@ 'shoes' ORDER BY id$$);
SELECT id FROM scan_stats WHERE description @@@ 'shoes' ORDER BY id;
SELECT pg_stat_force_next_flush();
SELECT * FROM scan_counts();

-- Postgres bitmap scan through the access method. The recheck filter reads the index once more.
SET enable_indexscan = off;
SET enable_bitmapscan = on;
SELECT * FROM scan_nodes($$SELECT id FROM scan_stats WHERE description @@@ 'shoes' ORDER BY id$$);
SELECT id FROM scan_stats WHERE description @@@ 'shoes' ORDER BY id;
SELECT pg_stat_force_next_flush();
SELECT * FROM scan_counts();

-- The search runs as a heap filter when no scan can use the index.
SET enable_bitmapscan = off;
SELECT * FROM scan_nodes($$SELECT id FROM scan_stats WHERE description @@@ 'shoes' ORDER BY id$$);
SELECT id FROM scan_stats WHERE description @@@ 'shoes' ORDER BY id;
SELECT pg_stat_force_next_flush();
SELECT * FROM scan_counts();

RESET paradedb.enable_custom_scan;
RESET enable_indexscan;
RESET enable_bitmapscan;
SET enable_indexscan = off;

-- Aggregate scan.
SET paradedb.enable_aggregate_custom_scan = true;
SELECT * FROM scan_nodes($$SELECT count(*) FROM scan_stats WHERE description @@@ 'shoes'$$);
SELECT count(*) FROM scan_stats WHERE description @@@ 'shoes';
SELECT pg_stat_force_next_flush();
SELECT * FROM scan_counts();
RESET paradedb.enable_aggregate_custom_scan;

-- Join scan, one scan per index.
SET paradedb.enable_join_custom_scan = true;
SELECT * FROM scan_nodes($$SELECT a.id, b.id FROM scan_stats a JOIN scan_stats_other b ON a.id = b.id WHERE a.description @@@ 'shoes' AND b.description @@@ 'shoes' ORDER BY a.id, b.id LIMIT 10$$);
SELECT a.id, b.id FROM scan_stats a JOIN scan_stats_other b ON a.id = b.id WHERE a.description @@@ 'shoes' AND b.description @@@ 'shoes' ORDER BY a.id, b.id LIMIT 10;
SELECT pg_stat_force_next_flush();
SELECT * FROM scan_counts();
RESET paradedb.enable_join_custom_scan;

DROP FUNCTION scan_nodes(text);
DROP FUNCTION scan_counts();
DROP TABLE scan_stats, scan_stats_other;
