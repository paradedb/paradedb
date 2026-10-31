CREATE EXTENSION IF NOT EXISTS pg_search;

CREATE TABLE issue_7045_empty (id bigint PRIMARY KEY, author varchar[]);
CREATE TABLE issue_7045_null (id bigint PRIMARY KEY, author varchar[]);
CREATE TABLE issue_7045_int (id bigint PRIMARY KEY, nums int[]);
CREATE TABLE issue_7045_null_elem (id bigint PRIMARY KEY, nums int[]);
INSERT INTO issue_7045_empty VALUES (1, ARRAY[]::varchar[]), (2, ARRAY['b']), (3, ARRAY['a', 'b']);
INSERT INTO issue_7045_null VALUES (1, NULL), (2, ARRAY['b']), (3, ARRAY['a', 'b']);
INSERT INTO issue_7045_int VALUES (1, ARRAY[]::int[]), (2, NULL), (3, ARRAY[1, 2]), (4, ARRAY[2]);
INSERT INTO issue_7045_null_elem VALUES
    (1, ARRAY[NULL]::int[]), (2, ARRAY[NULL, NULL]::int[]), (3, ARRAY[]::int[]), (4, NULL),
    (5, ARRAY[1, NULL]), (6, ARRAY[1]);
CREATE INDEX issue_7045_empty_idx ON issue_7045_empty USING paradedb (id, (author::pdb.literal));
CREATE INDEX issue_7045_null_idx ON issue_7045_null USING paradedb (id, (author::pdb.literal));
CREATE INDEX issue_7045_int_idx ON issue_7045_int USING paradedb (id, nums);
CREATE INDEX issue_7045_null_elem_idx ON issue_7045_null_elem USING paradedb (id, nums);

SET max_parallel_workers_per_gather = 0;
SET enable_indexscan = off;
SET paradedb.enable_aggregate_custom_scan = on;

-- pdb.agg() over GROUP BY unnest(...) runs as a Tantivy terms aggregation
EXPLAIN (COSTS OFF)
SELECT unnest(author), pdb.agg('{"value_count":{"field":"id"}}') FROM issue_7045_empty
WHERE id @@@ pdb.all() GROUP BY unnest(author);

-- Empty and NULL arrays produce no group
SELECT unnest(author) AS a, pdb.agg('{"value_count":{"field":"id"}}') FROM issue_7045_empty
WHERE id @@@ pdb.all() GROUP BY unnest(author) ORDER BY a LIMIT 20;
SELECT unnest(author) AS a, pdb.agg('{"value_count":{"field":"id"}}') FROM issue_7045_null
WHERE id @@@ pdb.all() GROUP BY unnest(author) ORDER BY a LIMIT 20;

-- The index stores no NULL elements, so pdb.agg() has no NULL group for them
SELECT unnest(nums) AS n, pdb.agg('{"value_count":{"field":"id"}}') FROM issue_7045_null_elem
WHERE id @@@ pdb.all() GROUP BY unnest(nums) ORDER BY n;

-- Plain aggregates over unnest(...) are left to PostgreSQL, which sees NULL elements
EXPLAIN (COSTS OFF)
SELECT unnest(nums) AS n, count(*) FROM issue_7045_null_elem
WHERE id @@@ pdb.all() GROUP BY unnest(nums) ORDER BY n;
SELECT unnest(author) AS a, count(*) FROM issue_7045_empty
WHERE id @@@ pdb.all() GROUP BY unnest(author) ORDER BY a;
SELECT unnest(author) AS a, count(*) FROM issue_7045_null
WHERE id @@@ pdb.all() GROUP BY unnest(author) ORDER BY a;
SELECT unnest(nums) AS n, count(*) FROM issue_7045_int
WHERE id @@@ pdb.all() GROUP BY unnest(nums) ORDER BY n;
SELECT unnest(nums) AS n, count(*) FROM issue_7045_null_elem
WHERE id @@@ pdb.all() AND id IN (1, 6) GROUP BY unnest(nums) ORDER BY n;
SELECT unnest(nums) AS n, count(*) FROM issue_7045_null_elem
WHERE id @@@ pdb.all() AND id IN (2, 6) GROUP BY unnest(nums) ORDER BY n;
SELECT unnest(nums) AS n, count(*) FROM issue_7045_null_elem
WHERE id @@@ pdb.all() GROUP BY unnest(nums) ORDER BY n;

-- Parity
SET paradedb.enable_aggregate_custom_scan = off;
SELECT unnest(author) AS a, count(*) FROM issue_7045_empty
WHERE id @@@ pdb.all() GROUP BY unnest(author) ORDER BY a;
SELECT unnest(author) AS a, count(*) FROM issue_7045_null
WHERE id @@@ pdb.all() GROUP BY unnest(author) ORDER BY a;
SELECT unnest(nums) AS n, count(*) FROM issue_7045_int
WHERE id @@@ pdb.all() GROUP BY unnest(nums) ORDER BY n;
SELECT unnest(nums) AS n, count(*) FROM issue_7045_null_elem
WHERE id @@@ pdb.all() AND id IN (1, 6) GROUP BY unnest(nums) ORDER BY n;
SELECT unnest(nums) AS n, count(*) FROM issue_7045_null_elem
WHERE id @@@ pdb.all() AND id IN (2, 6) GROUP BY unnest(nums) ORDER BY n;
SELECT unnest(nums) AS n, count(*) FROM issue_7045_null_elem
WHERE id @@@ pdb.all() GROUP BY unnest(nums) ORDER BY n;

RESET paradedb.enable_aggregate_custom_scan;
RESET enable_indexscan;
RESET max_parallel_workers_per_gather;

DROP TABLE issue_7045_empty;
DROP TABLE issue_7045_null;
DROP TABLE issue_7045_int;
DROP TABLE issue_7045_null_elem;
