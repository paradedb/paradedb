\i common/common_setup.sql

SET paradedb.planner_warnings = 'off';

CREATE TABLE issue_7054 (
    id integer PRIMARY KEY,
    body text,
    category text COLLATE "C"
);
INSERT INTO issue_7054 VALUES (1, 'wireless', 'ä'), (2, 'wireless', 'Å');

CREATE FUNCTION pg_temp.check_7054_order(sort_by text, k integer)
RETURNS TABLE (custom_ids integer[], postgres_ids integer[], topk boolean)
LANGUAGE plpgsql AS $$
DECLARE
    query text := format(
        'SELECT id FROM issue_7054 WHERE body ||| %L ORDER BY %s LIMIT %s',
        'wireless', sort_by, k
    );
    plan json;
BEGIN
    PERFORM set_config('paradedb.enable_custom_scan', 'on', true);
    EXECUTE 'EXPLAIN (FORMAT JSON, COSTS OFF) ' || query INTO plan;
    topk := plan::text LIKE '%TopKScanExecState%';
    EXECUTE 'SELECT ARRAY(' || query || ')' INTO custom_ids;
    PERFORM set_config('paradedb.enable_custom_scan', 'off', true);
    EXECUTE 'SELECT ARRAY(' || query || ')' INTO postgres_ids;
    IF custom_ids IS DISTINCT FROM postgres_ids THEN
        RAISE EXCEPTION 'custom scan returned %, PostgreSQL returned %', custom_ids, postgres_ids;
    END IF;
    PERFORM set_config('paradedb.enable_custom_scan', 'on', true);
    RETURN NEXT;
END;
$$;

CREATE FUNCTION pg_temp.check_7054_query(query text)
RETURNS jsonb LANGUAGE plpgsql AS $$
DECLARE
    custom_rows jsonb;
    postgres_rows jsonb;
BEGIN
    PERFORM set_config('paradedb.enable_custom_scan', 'on', true);
    PERFORM set_config('paradedb.enable_join_custom_scan', 'on', true);
    PERFORM set_config('paradedb.enable_aggregate_custom_scan', 'on', true);
    EXECUTE 'SELECT jsonb_agg(to_jsonb(q)) FROM (' || query || ') q' INTO custom_rows;
    PERFORM set_config('paradedb.enable_custom_scan', 'off', true);
    PERFORM set_config('paradedb.enable_join_custom_scan', 'off', true);
    PERFORM set_config('paradedb.enable_aggregate_custom_scan', 'off', true);
    EXECUTE 'SELECT jsonb_agg(to_jsonb(q)) FROM (' || query || ') q' INTO postgres_rows;
    IF custom_rows IS DISTINCT FROM postgres_rows THEN
        RAISE EXCEPTION 'custom scans returned %, PostgreSQL returned %', custom_rows, postgres_rows;
    END IF;
    PERFORM set_config('paradedb.enable_custom_scan', 'on', true);
    PERFORM set_config('paradedb.enable_join_custom_scan', 'on', true);
    PERFORM set_config('paradedb.enable_aggregate_custom_scan', 'on', true);
    RETURN custom_rows;
END;
$$;

CREATE TABLE issue_7054_other (id integer PRIMARY KEY, bug_id integer, note text);
INSERT INTO issue_7054_other VALUES (1, 1, 'wireless'), (2, 2, 'wireless'), (3, 3, 'wireless');
CREATE INDEX issue_7054_other_idx ON issue_7054_other USING paradedb (id, bug_id, note);

CREATE INDEX issue_7054_idx ON issue_7054 USING paradedb (
    id, body, (lower(category)::pdb.literal)
);
EXPLAIN (COSTS OFF)
SELECT id FROM issue_7054 WHERE body ||| 'wireless'
ORDER BY lower(category) COLLATE "C", id LIMIT 2;
SELECT * FROM pg_temp.check_7054_order('lower(category) COLLATE "C", id', 2);
SELECT * FROM pg_temp.check_7054_order('lower(category) COLLATE "C", id', 1);
SELECT * FROM pg_temp.check_7054_order('lower(category) COLLATE "C" DESC, id', 2);

SELECT id, count(*) OVER () AS total
FROM issue_7054
WHERE body ||| 'wireless'
ORDER BY lower(category) COLLATE "C", id
LIMIT 1;

INSERT INTO issue_7054 VALUES (3, 'wireless', 'a'), (4, 'wireless', 'Z');
SELECT * FROM pg_temp.check_7054_order('category COLLATE "C", id', 4);
DELETE FROM issue_7054 WHERE id > 2;
INSERT INTO issue_7054 VALUES (3, 'wireless', 'å');
DROP INDEX issue_7054_idx;

CREATE INDEX issue_7054_idx ON issue_7054 USING paradedb (
    id, body, (lower(category)::pdb.literal('alias=lower_category'))
);
EXPLAIN (COSTS OFF)
SELECT id FROM issue_7054 WHERE body ||| 'wireless'
ORDER BY lower(category) COLLATE "C", id LIMIT 2;
SELECT * FROM pg_temp.check_7054_order('lower(category) COLLATE "C", id', 2);
SELECT pg_temp.check_7054_query($q$
    SELECT a.id, lower(a.category) AS lc
    FROM issue_7054 a JOIN issue_7054_other b ON a.id = b.bug_id
    WHERE a.body ||| 'wireless' AND b.note ||| 'wireless'
    ORDER BY lower(a.category) COLLATE "C", a.id LIMIT 2
$q$);
SELECT pg_temp.check_7054_query($q$
    SELECT lower(category) AS lc, count(*) AS n
    FROM issue_7054 WHERE body ||| 'wireless'
    GROUP BY lower(category) ORDER BY lc
$q$);
DROP INDEX issue_7054_idx;

CREATE INDEX issue_7054_idx ON issue_7054 USING paradedb (
    id, body, (lower(category)::pdb.literal_normalized('lowercase=false', 'normalizer=lowercase'))
);
EXPLAIN (COSTS OFF)
SELECT id FROM issue_7054 WHERE body ||| 'wireless'
ORDER BY lower(category) COLLATE "C", id LIMIT 2;
SELECT * FROM pg_temp.check_7054_order('lower(category) COLLATE "C", id', 2);
SELECT * FROM pg_temp.check_7054_order('lower(category) COLLATE "C", id', 1);
EXPLAIN (COSTS OFF)
SELECT a.id, lower(a.category)
FROM issue_7054 a JOIN issue_7054_other b ON a.id = b.bug_id
WHERE a.body ||| 'wireless' AND b.note ||| 'wireless'
ORDER BY lower(a.category) COLLATE "C", a.id LIMIT 2;
EXPLAIN (COSTS OFF)
SELECT lower(category) AS lc, count(*)
FROM issue_7054 WHERE body ||| 'wireless'
GROUP BY lower(category) ORDER BY lc;
SELECT pg_temp.check_7054_query($q$
    SELECT a.id, lower(a.category) AS lc
    FROM issue_7054 a JOIN issue_7054_other b ON a.id = b.bug_id
    WHERE a.body ||| 'wireless' AND b.note ||| 'wireless'
    ORDER BY lower(a.category) COLLATE "C", a.id LIMIT 2
$q$);
SELECT pg_temp.check_7054_query($q$
    SELECT lower(category) AS lc, count(*) AS n
    FROM issue_7054 WHERE body ||| 'wireless'
    GROUP BY lower(category) ORDER BY lc
$q$);
DROP INDEX issue_7054_idx;

CREATE INDEX issue_7054_idx ON issue_7054 USING paradedb (
    id, body,
    (lower(category)::pdb.literal_normalized('alias=lower_category', 'lowercase=false', 'normalizer=lowercase'))
);
EXPLAIN (COSTS OFF)
SELECT id FROM issue_7054 WHERE body ||| 'wireless'
ORDER BY lower(category) COLLATE "C", id LIMIT 2;
SELECT * FROM pg_temp.check_7054_order('lower(category) COLLATE "C", id', 2);
SELECT pg_temp.check_7054_query($q$
    SELECT a.id, lower(a.category) AS lc
    FROM issue_7054 a JOIN issue_7054_other b ON a.id = b.bug_id
    WHERE a.body ||| 'wireless' AND b.note ||| 'wireless'
    ORDER BY lower(a.category) COLLATE "C", a.id LIMIT 2
$q$);
SELECT pg_temp.check_7054_query($q$
    SELECT lower(category) AS lc, count(*) AS n
    FROM issue_7054 WHERE body ||| 'wireless'
    GROUP BY lower(category) ORDER BY lc
$q$);
DROP INDEX issue_7054_idx;

CREATE INDEX issue_7054_idx ON issue_7054 USING paradedb (
    id, body,
    (upper(category)::pdb.literal_normalized('alias=upper_category', 'lowercase=false', 'normalizer=lowercase'))
);
EXPLAIN (COSTS OFF)
SELECT id FROM issue_7054 WHERE body ||| 'wireless'
ORDER BY upper(category) COLLATE "C", id LIMIT 2;
SELECT * FROM pg_temp.check_7054_order('upper(category) COLLATE "C", id', 2);

CREATE TABLE issue_7054_parts (
    id integer, junk integer, body text, category text COLLATE "C"
) PARTITION BY RANGE (id);
ALTER TABLE issue_7054_parts DROP COLUMN junk;
CREATE TABLE issue_7054_parts_a PARTITION OF issue_7054_parts FOR VALUES FROM (0) TO (1000);
CREATE TABLE issue_7054_parts_b PARTITION OF issue_7054_parts FOR VALUES FROM (1000) TO (2000);
INSERT INTO issue_7054_parts VALUES
    (1, 'wireless', 'ä'), (2, 'wireless', 'Å'), (3, 'wireless', 'å'),
    (1001, 'wireless', 'ä'), (1002, 'wireless', 'Å'), (1003, 'wireless', 'å');

CREATE INDEX issue_7054_parts_idx ON issue_7054_parts USING paradedb (
    id, body, (lower(category)::pdb.literal)
);
EXPLAIN (COSTS OFF)
SELECT id FROM issue_7054_parts WHERE body ||| 'wireless'
ORDER BY lower(category) COLLATE "C", id LIMIT 3;
SELECT pg_temp.check_7054_query($q$
    SELECT id, lower(category) AS lc FROM issue_7054_parts WHERE body ||| 'wireless'
    ORDER BY lower(category) COLLATE "C", id LIMIT 3
$q$);
DROP INDEX issue_7054_parts_idx;

CREATE INDEX issue_7054_parts_idx ON issue_7054_parts USING paradedb (
    id, body, (category::pdb.literal)
);
EXPLAIN (COSTS OFF)
SELECT id FROM issue_7054_parts WHERE body ||| 'wireless'
ORDER BY category COLLATE "C", id LIMIT 3;
SELECT pg_temp.check_7054_query($q$
    SELECT id, category FROM issue_7054_parts WHERE body ||| 'wireless'
    ORDER BY category COLLATE "C", id LIMIT 3
$q$);
DROP INDEX issue_7054_parts_idx;

CREATE INDEX issue_7054_parts_idx ON issue_7054_parts USING paradedb (
    id, body, (lower(category)::pdb.literal_normalized('normalizer=lowercase'))
);
EXPLAIN (COSTS OFF)
SELECT id FROM issue_7054_parts WHERE body ||| 'wireless'
ORDER BY lower(category) COLLATE "C", id LIMIT 3;
SELECT pg_temp.check_7054_query($q$
    SELECT id, lower(category) AS lc FROM issue_7054_parts WHERE body ||| 'wireless'
    ORDER BY lower(category) COLLATE "C", id LIMIT 3
$q$);
DROP TABLE issue_7054_parts;

CREATE TABLE issue_7054_subquery (
    id serial PRIMARY KEY, name text, description text, price float, rating float
);
INSERT INTO issue_7054_subquery (name, description, price, rating)
SELECT 'n' || i, 'laptop', i * 10, i % 7 FROM generate_series(1, 50) i;
CREATE INDEX issue_7054_subquery_idx ON issue_7054_subquery USING paradedb (
    id, name, description, price, rating
);
EXPLAIN (COSTS OFF)
SELECT * FROM (
    SELECT id, rating, price, count(*) OVER () AS total
    FROM issue_7054_subquery WHERE description ||| 'laptop'
    ORDER BY rating DESC, id LIMIT 5
) sub ORDER BY price DESC LIMIT 2;
SELECT pg_temp.check_7054_query($q$
    SELECT * FROM (
        SELECT id, rating, price, count(*) OVER () AS total
        FROM issue_7054_subquery WHERE description ||| 'laptop'
        ORDER BY rating DESC, id LIMIT 5
    ) sub ORDER BY price DESC LIMIT 2
$q$);
DROP TABLE issue_7054_subquery;

DROP TABLE issue_7054;
DROP TABLE issue_7054_other;
DROP FUNCTION pg_temp.check_7054_order(text, integer);
DROP FUNCTION pg_temp.check_7054_query(text);
RESET paradedb.planner_warnings;
