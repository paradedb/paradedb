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

CREATE INDEX issue_7054_idx ON issue_7054 USING paradedb (
    id, body, (lower(category)::pdb.literal)
);
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
DROP INDEX issue_7054_idx;

CREATE INDEX issue_7054_idx ON issue_7054 USING paradedb (
    id, body, (lower(category)::pdb.literal('alias=lower_category'))
);
SELECT * FROM pg_temp.check_7054_order('lower(category) COLLATE "C", id', 2);
DROP INDEX issue_7054_idx;

CREATE INDEX issue_7054_idx ON issue_7054 USING paradedb (
    id, body, (lower(category)::pdb.literal_normalized('lowercase=false', 'normalizer=lowercase'))
);
SELECT * FROM pg_temp.check_7054_order('lower(category) COLLATE "C", id', 2);
SELECT * FROM pg_temp.check_7054_order('lower(category) COLLATE "C", id', 1);
DROP INDEX issue_7054_idx;

CREATE INDEX issue_7054_idx ON issue_7054 USING paradedb (
    id, body,
    (lower(category)::pdb.literal_normalized('alias=lower_category', 'lowercase=false', 'normalizer=lowercase'))
);
SELECT * FROM pg_temp.check_7054_order('lower(category) COLLATE "C", id', 2);
DROP INDEX issue_7054_idx;

CREATE INDEX issue_7054_idx ON issue_7054 USING paradedb (
    id, body,
    (upper(category)::pdb.literal_normalized('alias=upper_category', 'lowercase=false', 'normalizer=lowercase'))
);
SELECT * FROM pg_temp.check_7054_order('upper(category) COLLATE "C", id', 2);

DROP TABLE issue_7054;
DROP FUNCTION pg_temp.check_7054_order(text, integer);
RESET paradedb.planner_warnings;
