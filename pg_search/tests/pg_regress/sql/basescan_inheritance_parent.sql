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

-- `FROM ONLY` names just the parent, so Base Scan still scans its index.
EXPLAIN (COSTS OFF)
SELECT id, kind FROM ONLY inh_parent WHERE id @@@ paradedb.all() ORDER BY id;
SELECT id, kind FROM ONLY inh_parent WHERE id @@@ paradedb.all() ORDER BY id;

-- `pdb.score` and `pdb.snippet` on the parent, for rows of both tables.
EXPLAIN (COSTS OFF)
SELECT id, kind, pdb.score(id) > 0 AS scored, pdb.snippet(kind)
FROM inh_parent WHERE kind ||| 'a c' ORDER BY id;
SELECT id, kind, pdb.score(id) > 0 AS scored, pdb.snippet(kind)
FROM inh_parent WHERE kind ||| 'a c' ORDER BY id;

-- A window function on the parent counts the rows of every table.
SELECT id, kind, count(*) OVER () AS total
FROM inh_parent WHERE id @@@ paradedb.all() ORDER BY id LIMIT 2;

-- With `FROM ONLY`, the window function counts the parent's rows.
SELECT id, kind, count(*) OVER () AS total
FROM ONLY inh_parent WHERE id @@@ paradedb.all() ORDER BY id LIMIT 2;

-- A table whose only child was dropped keeps `relhassubclass` until ANALYZE, so
-- PostgreSQL still plans it as an inheritance parent.
CREATE TABLE inh_former_parent (id SERIAL PRIMARY KEY, kind TEXT)
    WITH (autovacuum_enabled = false);
CREATE TABLE inh_former_child () INHERITS (inh_former_parent);
DROP TABLE inh_former_child;
INSERT INTO inh_former_parent (kind) VALUES ('a'), ('b'), ('c');
CREATE INDEX inh_former_parent_idx ON inh_former_parent USING paradedb (id, kind);

EXPLAIN (COSTS OFF)
SELECT id, kind FROM inh_former_parent WHERE id @@@ paradedb.all() ORDER BY id LIMIT 2;
SELECT id, kind FROM inh_former_parent WHERE id @@@ paradedb.all() ORDER BY id LIMIT 2;

-- ANALYZE clears `relhassubclass`.
ANALYZE inh_former_parent;
EXPLAIN (COSTS OFF)
SELECT id, kind FROM inh_former_parent WHERE id @@@ paradedb.all() ORDER BY id LIMIT 2;
DROP TABLE inh_former_parent;

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
