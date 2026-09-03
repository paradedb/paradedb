-- Regression test for issue #5567.
-- ParadeDB Join Scan with a composite ORDER BY whose first key is nullable
-- previously blanked out the remaining sort keys for NULL-first-key rows,
-- returning the wrong top-K in insertion order instead of the correct
-- lex-sorted result. The custom-scan-off path was already correct; the
-- fix makes the two paths agree.

DROP TABLE IF EXISTS issue_5567_child CASCADE;
DROP TABLE IF EXISTS issue_5567_parent CASCADE;
DROP TYPE IF EXISTS issue_5567_ps CASCADE;

CREATE EXTENSION IF NOT EXISTS pg_search;

CREATE TABLE issue_5567_parent (
    id int PRIMARY KEY,
    kind text,
    category text,
    name text NOT NULL
);

CREATE TABLE issue_5567_child (
    id bigint PRIMARY KEY,
    parent_id bigint
);

-- category is NULL on every 4th row; name is never NULL and reverses insertion order.
INSERT INTO issue_5567_parent
SELECT g,
       CASE WHEN g % 3 = 0 THEN 'novel' ELSE 'manga' END,
       CASE WHEN g % 4 = 0 THEN NULL ELSE 'cat_' || (g % 5) END,
       'n' || lpad((2001 - g)::text, 6, '0')
FROM generate_series(1, 2000) g;

INSERT INTO issue_5567_child SELECT g, g FROM generate_series(1, 2000) g;

CREATE TYPE issue_5567_ps AS (kind pdb.literal_normalized, category pdb.literal, name pdb.literal);

CREATE INDEX issue_5567_parent_bm25 ON issue_5567_parent
USING bm25 (id, (ROW(kind, category, name)::issue_5567_ps)) WITH (key_field = 'id');

CREATE INDEX issue_5567_child_bm25 ON issue_5567_child
USING bm25 (id, parent_id) WITH (key_field = 'id');

ANALYZE issue_5567_parent;
ANALYZE issue_5567_child;

SET enable_hashjoin = off;
SET enable_mergejoin = off;
SET enable_nestloop = off;

-- Case 1: the nullable key sorts first. Among the NULL-category rows the
-- smallest names lead, because `name ASC` still orders them.
--
-- Join Scan off is the reference result.
SET paradedb.enable_join_custom_scan = off;

SELECT p.id, p.category, p.name
FROM issue_5567_parent p JOIN issue_5567_child c ON c.parent_id = p.id
WHERE p.id @@@ paradedb.term('kind', 'manga') AND c.id @@@ paradedb.all()
ORDER BY p.category DESC NULLS FIRST, p.name ASC
LIMIT 5;

SET paradedb.enable_join_custom_scan = on;

-- The plan must reach SegmentedTopKExec under the Join Scan: enabling the
-- GUC only permits the custom plan, it does not select it, and the rows
-- below only test the fix if that is the plan that produced them.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.category, p.name
FROM issue_5567_parent p JOIN issue_5567_child c ON c.parent_id = p.id
WHERE p.id @@@ paradedb.term('kind', 'manga') AND c.id @@@ paradedb.all()
ORDER BY p.category DESC NULLS FIRST, p.name ASC
LIMIT 5;

-- Same rows as the reference result above.
SELECT p.id, p.category, p.name
FROM issue_5567_parent p JOIN issue_5567_child c ON c.parent_id = p.id
WHERE p.id @@@ paradedb.term('kind', 'manga') AND c.id @@@ paradedb.all()
ORDER BY p.category DESC NULLS FIRST, p.name ASC
LIMIT 5;

-- Case 2: the nullable key sorts second, behind a non-null deferred key.
-- A NULL in `category` must leave `kind` and `name` ordering intact.
SET paradedb.enable_join_custom_scan = off;

SELECT p.id, p.kind, p.category, p.name
FROM issue_5567_parent p JOIN issue_5567_child c ON c.parent_id = p.id
WHERE p.id @@@ paradedb.term('kind', 'manga') AND c.id @@@ paradedb.all()
ORDER BY p.kind ASC, p.category ASC NULLS FIRST, p.name ASC
LIMIT 5;

SET paradedb.enable_join_custom_scan = on;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.kind, p.category, p.name
FROM issue_5567_parent p JOIN issue_5567_child c ON c.parent_id = p.id
WHERE p.id @@@ paradedb.term('kind', 'manga') AND c.id @@@ paradedb.all()
ORDER BY p.kind ASC, p.category ASC NULLS FIRST, p.name ASC
LIMIT 5;

SELECT p.id, p.kind, p.category, p.name
FROM issue_5567_parent p JOIN issue_5567_child c ON c.parent_id = p.id
WHERE p.id @@@ paradedb.term('kind', 'manga') AND c.id @@@ paradedb.all()
ORDER BY p.kind ASC, p.category ASC NULLS FIRST, p.name ASC
LIMIT 5;

RESET paradedb.enable_join_custom_scan;
RESET enable_hashjoin;
RESET enable_mergejoin;
RESET enable_nestloop;

DROP TABLE issue_5567_child CASCADE;
DROP TABLE issue_5567_parent CASCADE;
DROP TYPE issue_5567_ps CASCADE;
