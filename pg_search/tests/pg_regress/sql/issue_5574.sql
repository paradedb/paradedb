-- Issue #5574: a Join Scan whose LIMIT is a parameter advertises sorted output, but on a
-- generic plan the ORDER BY vanished from its DataFusion plan, so the scan returned rows in
-- join probe order.

CREATE EXTENSION IF NOT EXISTS pg_search;

CREATE TABLE issue_5574_parent (id int PRIMARY KEY, kind text);
CREATE TABLE issue_5574_child (id bigint PRIMARY KEY, parent_id bigint);

-- Two thirds of the parents are 'manga', and every parent 1..1000 has two children, g and
-- g + 1000, so the correct order repeats each parent id.
INSERT INTO issue_5574_parent
SELECT g, CASE WHEN g % 3 = 0 THEN 'novel' ELSE 'manga' END
FROM generate_series(1, 2000) g;
INSERT INTO issue_5574_child
SELECT g, ((g - 1) % 1000) + 1
FROM generate_series(1, 2000) g;

CREATE INDEX issue_5574_parent_idx ON issue_5574_parent USING bm25 (id, kind) WITH (key_field = 'id');
CREATE INDEX issue_5574_child_idx ON issue_5574_child USING bm25 (id, parent_id) WITH (key_field = 'id');
ANALYZE issue_5574_parent;
ANALYZE issue_5574_child;

-- Steer the planner to the Join Scan
SET enable_hashjoin = off;
SET enable_mergejoin = off;
SET enable_nestloop = off;
SET max_parallel_workers_per_gather = 0;
SET plan_cache_mode = force_generic_plan;

PREPARE issue_5574_page AS
SELECT p.id
FROM issue_5574_parent p JOIN issue_5574_child c ON c.parent_id = p.id
WHERE p.kind @@@ pdb.term('manga') AND c.id @@@ pdb.all()
ORDER BY p.id
LIMIT $1;

-- The DataFusion plan must keep a SortExec for the ORDER BY
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) EXECUTE issue_5574_page(5);
EXECUTE issue_5574_page(5);

-- Same rows as with a literal LIMIT
SELECT p.id
FROM issue_5574_parent p JOIN issue_5574_child c ON c.parent_id = p.id
WHERE p.kind @@@ pdb.term('manga') AND c.id @@@ pdb.all()
ORDER BY p.id
LIMIT 5;

-- A parameterized OFFSET is resolved on the same path
PREPARE issue_5574_page_offset AS
SELECT p.id
FROM issue_5574_parent p JOIN issue_5574_child c ON c.parent_id = p.id
WHERE p.kind @@@ pdb.term('manga') AND c.id @@@ pdb.all()
ORDER BY p.id
LIMIT $1 OFFSET $2;

EXECUTE issue_5574_page_offset(4, 3);

SELECT p.id
FROM issue_5574_parent p JOIN issue_5574_child c ON c.parent_id = p.id
WHERE p.kind @@@ pdb.term('manga') AND c.id @@@ pdb.all()
ORDER BY p.id
LIMIT 4 OFFSET 3;

DEALLOCATE issue_5574_page;
DEALLOCATE issue_5574_page_offset;
RESET plan_cache_mode;
RESET max_parallel_workers_per_gather;
RESET enable_nestloop;
RESET enable_mergejoin;
RESET enable_hashjoin;

DROP TABLE issue_5574_child, issue_5574_parent;
