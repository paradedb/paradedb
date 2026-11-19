-- A search on the parent of an inheritance tree returns the rows of every table
-- in the tree, as PostgreSQL does (#6640). The parent's index holds only the
-- parent's rows, so Base Scan declines the parent and scans each table on its own.
\i common/common_setup.sql

CREATE TABLE inh_parent (id SERIAL PRIMARY KEY, kind TEXT);
CREATE TABLE inh_child () INHERITS (inh_parent);
INSERT INTO inh_parent (kind) VALUES ('a'), ('b');
INSERT INTO inh_child (kind) VALUES ('c'), ('d');
CREATE INDEX inh_parent_idx ON inh_parent USING paradedb (id, kind);
CREATE INDEX inh_child_idx ON inh_child USING paradedb (id, kind);
ANALYZE inh_parent;
ANALYZE inh_child;

-- The query from the issue: one Base Scan per table, 4 rows.
EXPLAIN (COSTS OFF)
SELECT id, kind FROM inh_parent WHERE id @@@ paradedb.all() ORDER BY id;
SELECT id, kind FROM inh_parent WHERE id @@@ paradedb.all() ORDER BY id;

-- The same rows without the custom scan.
SET paradedb.enable_custom_scan = off;
SELECT id, kind FROM inh_parent WHERE id @@@ paradedb.all() ORDER BY id;
RESET paradedb.enable_custom_scan;

-- The highest ids are in the child.
SELECT id, kind FROM inh_parent WHERE id @@@ paradedb.all() ORDER BY id DESC LIMIT 2;

-- A DELETE on the parent removes the matching rows of the child too.
BEGIN;
EXPLAIN (COSTS OFF)
DELETE FROM inh_parent WHERE id @@@ paradedb.all();
WITH deleted AS (
    DELETE FROM inh_parent WHERE id @@@ paradedb.all() RETURNING id, kind
)
SELECT id, kind FROM deleted ORDER BY id;
ROLLBACK;

-- An UPDATE on the parent changes the matching rows of the child too.
BEGIN;
EXPLAIN (COSTS OFF)
UPDATE inh_parent SET kind = kind || '!' WHERE kind ||| 'a c';
WITH updated AS (
    UPDATE inh_parent SET kind = kind || '!' WHERE kind ||| 'a c'
    RETURNING tableoid::regclass AS tbl, id, kind
)
SELECT tbl, id, kind FROM updated ORDER BY id;
ROLLBACK;

-- `FROM ONLY` names just the parent, so Base Scan still scans its index.
EXPLAIN (COSTS OFF)
SELECT id, kind FROM ONLY inh_parent WHERE id @@@ paradedb.all() ORDER BY id;
SELECT id, kind FROM ONLY inh_parent WHERE id @@@ paradedb.all() ORDER BY id;

-- PostgreSQL computes the select list of an inheritance parent above the `Append`.
-- `pdb.score` and `pdb.snippet` are wrapped in a PlaceHolderVar, so the Base Scan of
-- each table computes them.
EXPLAIN (COSTS OFF)
SELECT id, kind, pdb.score(id) > 0 AS scored, pdb.snippet(kind)
FROM inh_parent WHERE kind ||| 'a c' ORDER BY id;
SELECT id, kind, pdb.score(id) > 0 AS scored, pdb.snippet(kind)
FROM inh_parent WHERE kind ||| 'a c' ORDER BY id;
SELECT id, pdb.score(id) > 0 AS scored FROM inh_parent
WHERE kind ||| 'a c' ORDER BY pdb.score(id) DESC, id LIMIT 2;

-- The same with an aggregate above the `Append`.
SELECT count(*), max(pdb.score(id)) > 0 AS scored FROM inh_parent WHERE kind ||| 'a c';
SELECT max(pdb.snippet(kind)) FROM inh_parent WHERE kind ||| 'a c';

-- `pdb.snippets` has no PlaceHolderVar, so it fails on the parent.
SELECT id, pdb.snippets(kind) FROM inh_parent WHERE kind ||| 'a c' ORDER BY id;

-- With `FROM ONLY`, the parent's Base Scan computes them.
SELECT id, kind, pdb.score(id) > 0 AS scored, pdb.snippet(kind)
FROM ONLY inh_parent WHERE kind ||| 'a c' ORDER BY id;

-- A window function on the parent counts the rows of every table.
SELECT id, kind, count(*) OVER () AS total
FROM inh_parent WHERE id @@@ paradedb.all() ORDER BY id LIMIT 2;

-- With `FROM ONLY`, the window function counts the parent's rows.
SELECT id, kind, count(*) OVER () AS total
FROM ONLY inh_parent WHERE id @@@ paradedb.all() ORDER BY id LIMIT 2;

-- `pdb.agg()` as a window function on the parent fails, with or without ORDER BY.
-- PostgreSQL computes it above the `Append`, where no Base Scan can.
SELECT id, pdb.agg('{"value_count": {"field": "id"}}'::jsonb) OVER () AS agg
FROM inh_parent WHERE id @@@ paradedb.all() ORDER BY id LIMIT 2;
SELECT id, pdb.agg('{"value_count": {"field": "id"}}'::jsonb) OVER () AS agg
FROM inh_parent WHERE id @@@ paradedb.all() LIMIT 2;

-- The same through a view or a subquery over the parent.
CREATE VIEW inh_parent_v AS SELECT id, kind FROM inh_parent;
SELECT id, kind, count(*) OVER () AS total
FROM inh_parent_v WHERE id @@@ paradedb.all() ORDER BY id LIMIT 2;
SELECT id, pdb.agg('{"value_count": {"field": "id"}}'::jsonb) OVER () AS agg
FROM inh_parent_v WHERE id @@@ paradedb.all() ORDER BY id LIMIT 2;
SELECT id, pdb.agg('{"value_count": {"field": "id"}}'::jsonb) OVER () AS agg
FROM (SELECT id, kind FROM inh_parent) s WHERE id @@@ paradedb.all() ORDER BY id LIMIT 2;
DROP VIEW inh_parent_v;

-- With `FROM ONLY`, `pdb.agg()` counts the parent's rows.
SELECT id, pdb.agg('{"value_count": {"field": "id"}}'::jsonb) OVER () AS agg
FROM ONLY inh_parent WHERE id @@@ paradedb.all() ORDER BY id LIMIT 2;

-- `pdb.agg()` as an aggregate on the parent fails: Aggregate Scan declines the parent.
SELECT pdb.agg('{"value_count": {"field": "id"}}'::jsonb) FROM inh_parent
WHERE id @@@ paradedb.all();
SELECT pdb.agg('{"value_count": {"field": "id"}}'::jsonb) FROM ONLY inh_parent
WHERE id @@@ paradedb.all();

-- A table whose only child was dropped keeps `relhassubclass` until ANALYZE, so
-- PostgreSQL still plans it as an inheritance parent. The rows are correct, but
-- the LIMIT is not pushed into the scan.
CREATE TABLE inh_former_parent (id SERIAL PRIMARY KEY, kind TEXT)
    WITH (autovacuum_enabled = false);
CREATE TABLE inh_former_child () INHERITS (inh_former_parent);
DROP TABLE inh_former_child;
INSERT INTO inh_former_parent (kind) VALUES ('a'), ('b'), ('c');
CREATE INDEX inh_former_parent_idx ON inh_former_parent USING paradedb (id, kind);

EXPLAIN (COSTS OFF)
SELECT id, kind FROM inh_former_parent WHERE id @@@ paradedb.all() ORDER BY id LIMIT 2;
SELECT id, kind FROM inh_former_parent WHERE id @@@ paradedb.all() ORDER BY id LIMIT 2;

-- `pdb.score` and `pdb.snippet` work on it. `pdb.agg()` fails until ANALYZE.
SELECT id, pdb.score(id) > 0 AS scored, pdb.snippet(kind)
FROM inh_former_parent WHERE kind ||| 'a' ORDER BY id;
SELECT id, pdb.score(id) > 0 AS scored FROM inh_former_parent
WHERE kind ||| 'a b' ORDER BY pdb.score(id) DESC, id LIMIT 2;
SELECT id, pdb.agg('{"value_count": {"field": "id"}}'::jsonb) OVER () AS agg
FROM inh_former_parent WHERE kind ||| 'a' LIMIT 2;

-- ANALYZE clears `relhassubclass`, and the scan gets the LIMIT again.
ANALYZE inh_former_parent;
EXPLAIN (COSTS OFF)
SELECT id, kind FROM inh_former_parent WHERE id @@@ paradedb.all() ORDER BY id LIMIT 2;
DROP TABLE inh_former_parent;

-- A parallel scan of the parent also returns the child's rows. Each INSERT writes its
-- own segment, so each table has 2 segments to share between the workers.
CREATE TABLE inh_par_parent (id SERIAL PRIMARY KEY, kind TEXT);
CREATE TABLE inh_par_child () INHERITS (inh_par_parent);
CREATE INDEX inh_par_parent_idx ON inh_par_parent USING paradedb (id, kind);
CREATE INDEX inh_par_child_idx ON inh_par_child USING paradedb (id, kind);
SET paradedb.global_mutable_segment_rows = 0;
INSERT INTO inh_par_parent (kind) SELECT 'a' FROM generate_series(1, 50);
INSERT INTO inh_par_parent (kind) SELECT 'b' FROM generate_series(1, 50);
INSERT INTO inh_par_child (kind) SELECT 'a' FROM generate_series(1, 50);
INSERT INTO inh_par_child (kind) SELECT 'b' FROM generate_series(1, 50);
RESET paradedb.global_mutable_segment_rows;
ANALYZE inh_par_parent;
ANALYZE inh_par_child;

SET max_parallel_workers_per_gather = 2;
SET parallel_setup_cost = 0;
SET parallel_tuple_cost = 0;
SET min_parallel_table_scan_size = 0;
SET paradedb.min_rows_per_worker = 0;
EXPLAIN (COSTS OFF)
SELECT tableoid::regclass AS tbl, count(*) FROM inh_par_parent
WHERE kind ||| 'a' GROUP BY 1 ORDER BY 1;
SELECT tableoid::regclass AS tbl, count(*) FROM inh_par_parent
WHERE kind ||| 'a' GROUP BY 1 ORDER BY 1;
RESET max_parallel_workers_per_gather;
RESET parallel_setup_cost;
RESET parallel_tuple_cost;
RESET min_parallel_table_scan_size;
RESET paradedb.min_rows_per_worker;
DROP TABLE inh_par_child, inh_par_parent;

-- Control: a partitioned table already gets one Base Scan per partition.
CREATE TABLE part_parent (id INT, kind TEXT) PARTITION BY RANGE (id);
CREATE TABLE part_low PARTITION OF part_parent FOR VALUES FROM (1) TO (3);
CREATE TABLE part_high PARTITION OF part_parent FOR VALUES FROM (3) TO (5);
INSERT INTO part_parent VALUES (1, 'a'), (2, 'b'), (3, 'c'), (4, 'd');
CREATE INDEX part_parent_idx ON part_parent USING paradedb (id, kind);
ANALYZE part_parent;

EXPLAIN (COSTS OFF)
SELECT id, kind FROM part_parent WHERE id @@@ paradedb.all() ORDER BY id;
SELECT id, kind FROM part_parent WHERE id @@@ paradedb.all() ORDER BY id;

DROP TABLE part_parent;
DROP TABLE inh_child, inh_parent;
