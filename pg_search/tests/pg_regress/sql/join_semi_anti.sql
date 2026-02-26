-- Tests for Semi and Anti Joins with Join Custom Scan
CREATE EXTENSION IF NOT EXISTS pg_search;

DROP TABLE IF EXISTS table_a CASCADE;
CREATE TABLE table_a (
    id bigint NOT NULL PRIMARY KEY,
    category character varying
);

INSERT INTO table_a (id, category)
SELECT
    i,
    CASE WHEN i % 2 = 0 THEN 'target_category' ELSE 'other_category' END
FROM generate_series(1, 2000) as i;

DROP TABLE IF EXISTS table_b CASCADE;
CREATE TABLE table_b (
    id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    group_id character varying,
    a_id bigint
);

INSERT INTO table_b (group_id, a_id)
SELECT
    CASE WHEN i % 10 = 0 THEN 'group_1' ELSE 'group_2' END,
    i
FROM generate_series(1, 2000) as i;

CREATE INDEX table_a_idx ON table_a
USING bm25 (id, category)
WITH (
    key_field = id,
    text_fields = '{
        "category": {"fast": true, "tokenizer": {"type": "keyword"}, "normalizer": "lowercase"}
    }'
);

CREATE INDEX table_b_group_id_idx ON table_b USING btree (group_id);
CREATE INDEX table_b_group_id_a_id_idx ON table_b USING btree (group_id, a_id);

CREATE INDEX table_b_idx ON table_b
USING bm25 (id, group_id, a_id)
WITH (
    key_field = id,
    text_fields = '{
        "group_id": {"fast": true, "tokenizer": {"type": "keyword"}}
    }',
    numeric_fields = '{
        "a_id": {"fast": true}
    }'
);

-- Query execution
SET paradedb.enable_join_custom_scan TO on;

-- =====================================================================
-- 1. Semi Join Only
-- =====================================================================
EXPLAIN
SELECT id, category
FROM table_a
WHERE id IN (
    SELECT a_id
    FROM table_b
    WHERE group_id IN ('group_1')
)
AND id @@@ 'category:"target_category"'
ORDER BY id ASC
LIMIT 10;

-- =====================================================================
-- 2. Anti Join Only
-- =====================================================================
-- TODO: This query only triggers set_rel_pathlist_hook and not set_join_pathlist_hook, and so does
-- not render our warning. See https://github.com/paradedb/paradedb/issues/4236 about resolving this.
SELECT id, category
FROM table_a
WHERE id NOT IN (
    SELECT a_id
    FROM table_b
    WHERE group_id IN ('group_3', 'group_4')
)
AND id @@@ 'category:"target_category"'
ORDER BY id ASC
LIMIT 10;

-- =====================================================================
-- 3. Both Semi and Anti Join
-- =====================================================================
-- TODO: This query should produce a warning because Anti-Joins are not supported.
SELECT id, category
FROM table_a
WHERE id IN (
    SELECT a_id
    FROM table_b
    WHERE group_id IN ('group_1')
)
AND id NOT IN (
    SELECT a_id
    FROM table_b
    WHERE group_id IN ('group_3', 'group_4')
)
AND id @@@ 'category:"target_category"'
ORDER BY id ASC
LIMIT 10;

-- Cleanup
DROP TABLE table_a CASCADE;
DROP TABLE table_b CASCADE;

RESET paradedb.enable_join_custom_scan;
