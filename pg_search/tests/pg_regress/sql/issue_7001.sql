CREATE EXTENSION IF NOT EXISTS pg_search;

CREATE TABLE issue_7001 (id int PRIMARY KEY, rating int, body text NOT NULL);
INSERT INTO issue_7001 VALUES
    (1, 5, 'hello world'),
    (2, NULL, 'hello there'),
    (3, 3, 'goodbye');
CREATE INDEX issue_7001_idx ON issue_7001 USING paradedb (id, rating, body);

-- An empty ALL is true for every row
EXPLAIN (COSTS OFF, VERBOSE)
SELECT id FROM issue_7001 WHERE NOT (id = ANY('{}'::int[])) AND body ||| 'hello' ORDER BY id;
SELECT id FROM issue_7001 WHERE NOT (id = ANY('{}'::int[])) AND body ||| 'hello' ORDER BY id;
SELECT id FROM issue_7001 WHERE id <> ALL('{}'::int[]) AND body ||| 'hello' ORDER BY id;

-- including rows where the column is NULL
SELECT id FROM issue_7001 WHERE rating <> ALL('{}'::int[]) AND body ||| 'hello' ORDER BY id;
SELECT id FROM issue_7001 WHERE rating > ALL('{}'::int[]) AND body ||| 'hello' ORDER BY id;

-- Non-empty arrays are unchanged
SELECT id FROM issue_7001 WHERE NOT (id = ANY('{1}'::int[])) AND body ||| 'hello' ORDER BY id;
SELECT id FROM issue_7001 WHERE rating <> ALL('{5}'::int[]) AND body ||| 'hello' ORDER BY id;

-- An empty ANY still matches nothing
SELECT id FROM issue_7001 WHERE id = ANY('{}'::int[]) AND body ||| 'hello' ORDER BY id;

-- The array as a parameter
PREPARE issue_7001_stmt(int[]) AS
SELECT id FROM issue_7001 WHERE NOT (id = ANY($1)) AND body ||| 'hello' ORDER BY id;
EXECUTE issue_7001_stmt('{}');
EXECUTE issue_7001_stmt('{2}');
DEALLOCATE issue_7001_stmt;

DROP TABLE issue_7001;
