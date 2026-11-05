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
