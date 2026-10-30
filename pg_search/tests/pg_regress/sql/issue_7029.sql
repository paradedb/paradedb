CREATE EXTENSION IF NOT EXISTS pg_search;

CREATE TABLE issue_7029 (id bigint PRIMARY KEY, description text[]);
INSERT INTO issue_7029 VALUES
    (1, ARRAY['dog', 'cat', 'bird']),
    (2, ARRAY['fox', 'rabbit', 'squirrel']),
    (3, ARRAY['dog', NULL]::text[]),
    (4, ARRAY[]::text[]),
    (5, NULL),
    (6, ARRAY[NULL]::text[]),
    (7, ARRAY['DOG', 'cat']),
    (8, ARRAY['dog', 'dog']);
CREATE INDEX issue_7029_idx ON issue_7029 USING paradedb (id, (description::pdb.literal));
ANALYZE issue_7029;

SET paradedb.enable_custom_scan = on;

-- A NULL scalar against an indexed array column is left to PostgreSQL
EXPLAIN (COSTS OFF, VERBOSE)
SELECT id FROM issue_7029 WHERE NULL::text = ANY(description) AND id @@@ pdb.all();
SELECT id FROM issue_7029 WHERE NULL::text = ANY(description) AND id @@@ pdb.all() ORDER BY id;

-- and is true only for the empty array
SELECT id FROM issue_7029 WHERE NULL::text <> ALL(description) AND id @@@ pdb.all() ORDER BY id;
SELECT id FROM issue_7029 WHERE NOT (NULL::text = ANY(description)) AND id @@@ pdb.all() ORDER BY id;

-- A non-NULL scalar is still pushed down
SELECT id FROM issue_7029 WHERE 'dog' = ANY(description) AND id @@@ pdb.all() ORDER BY id;

-- Same results with the Custom Scan off
SET paradedb.enable_custom_scan = off;
SELECT id FROM issue_7029 WHERE NULL::text = ANY(description) AND id @@@ pdb.all() ORDER BY id;
SELECT id FROM issue_7029 WHERE NULL::text <> ALL(description) AND id @@@ pdb.all() ORDER BY id;
SELECT id FROM issue_7029 WHERE NOT (NULL::text = ANY(description)) AND id @@@ pdb.all() ORDER BY id;
RESET paradedb.enable_custom_scan;

DROP TABLE issue_7029;
