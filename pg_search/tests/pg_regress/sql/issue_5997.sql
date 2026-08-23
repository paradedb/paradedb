-- Issue #5997: ORDER BY col::text (a type-changing CoerceViaIO) must not be
-- pushed down as a raw sort on the inner column. TopK would return the column's
-- native order; Postgres orders the cast value as text.
--
-- RelabelType (binary-compatible, e.g. varchar::text) stays pushable. COLLATE "C"
-- is used so the collation gate cannot hide a wrong numeric/range pushdown.

CREATE EXTENSION IF NOT EXISTS pg_search;

CREATE TABLE issue_5997 (
    id serial PRIMARY KEY,
    title text,
    n bigint,
    title_vc varchar,
    span int4range
);

INSERT INTO issue_5997 (title, n, title_vc, span) VALUES
    ('doc', 9, 'b', '[1,2)'),
    ('doc', 10, 'a', '[10,11)'),
    ('doc', 100, 'c', '[2,3)'),
    ('doc', 2, 'd', '[100,101)');

CREATE INDEX issue_5997_idx ON issue_5997
USING paradedb (id, title, n, title_vc, span)
WITH (
    key_field = 'id',
    text_fields = '{"title_vc": {"fast": true, "tokenizer": {"type": "raw"}}}'
);

-- bigint::text: text order is 10, 100, 2, 9 — not numeric 2, 9, 10, 100.
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT n FROM issue_5997
WHERE title @@@ 'doc'
ORDER BY n::text COLLATE "C"
LIMIT 4;

SELECT n FROM issue_5997
WHERE title @@@ 'doc'
ORDER BY n::text COLLATE "C"
LIMIT 4;

SET paradedb.enable_custom_scan = off;
SELECT n FROM issue_5997
WHERE title @@@ 'doc'
ORDER BY n::text COLLATE "C"
LIMIT 4;
SET paradedb.enable_custom_scan = on;

-- Raw numeric ORDER BY is still a TopK sort on n.
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT n FROM issue_5997
WHERE title @@@ 'doc'
ORDER BY n
LIMIT 4;

SELECT n FROM issue_5997
WHERE title @@@ 'doc'
ORDER BY n
LIMIT 4;

-- RelabelType varchar::text is still a raw sort on the varchar column.
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT title_vc FROM issue_5997
WHERE title @@@ 'doc'
ORDER BY title_vc::text COLLATE "C"
LIMIT 4;

SELECT title_vc FROM issue_5997
WHERE title @@@ 'doc'
ORDER BY title_vc::text COLLATE "C"
LIMIT 4;

-- int4range::text must not ride the range-sortable Raw path.
-- Text order: [1,2), [10,11), [100,101), [2,3). Range order would put [2,3) second.
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT span FROM issue_5997
WHERE title @@@ 'doc'
ORDER BY span::text COLLATE "C"
LIMIT 4;

SELECT span FROM issue_5997
WHERE title @@@ 'doc'
ORDER BY span::text COLLATE "C"
LIMIT 4;

DROP TABLE issue_5997 CASCADE;
