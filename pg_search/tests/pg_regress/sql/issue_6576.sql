-- Regression coverage for issue #6576.
--
-- A Join Scan must accept a target list expression whose inputs are not
-- columnar, as long as the expression only shapes the output. Its inputs come
-- from the heap fetch like a bare column, and Postgres evaluates it after that.
-- An expression the scan itself needs (a DISTINCT or ORDER BY key) still has
-- to be columnar.

\i common/common_setup.sql

SET paradedb.enable_join_custom_scan = on;

-- `path` is deliberately not in the index. `tags` is indexed for the LATERAL
-- unnest cases.
CREATE TABLE issue_6576_s (id int PRIMARY KEY, title text, rating float8, path text, tags text[]);
INSERT INTO issue_6576_s
SELECT g,
       CASE WHEN g % 3 = 0 THEN 'dragon ' || g ELSE 'other ' || g END,
       g % 100,
       'p/' || g,
       ARRAY['t' || (g % 3), 'u' || (g % 5)]
FROM generate_series(1, 5000) g;
CREATE INDEX issue_6576_s_idx ON issue_6576_s
USING bm25 (id, title, rating, (tags::pdb.literal));

-- `note` is deliberately not in the index, and is NULL for some rows.
CREATE TABLE issue_6576_e (id int PRIMARY KEY, u text, s_id int, note text);
INSERT INTO issue_6576_e
SELECT g, 'u1', g * 2, CASE WHEN g % 4 = 0 THEN NULL ELSE 'n' || g END
FROM generate_series(1, 1000) g;
CREATE INDEX issue_6576_e_idx ON issue_6576_e
USING bm25 (id, u, s_id)
WITH (text_fields = '{"u": {"fast": true, "tokenizer": {"type": "keyword"}}}');

ANALYZE issue_6576_s;
ANALYZE issue_6576_e;

-- 1) Control: a bare non-columnar column in an anti join takes the Join Scan.
EXPLAIN (COSTS OFF)
SELECT s.id, s.path FROM issue_6576_s s
WHERE s.id @@@ paradedb.match('title', 'dragon')
  AND NOT EXISTS (
    SELECT 1 FROM issue_6576_e e WHERE e.s_id = s.id AND e.id @@@ paradedb.term('u', 'u1'))
ORDER BY paradedb.score(s.id) DESC, s.rating DESC, s.id LIMIT 5;

SELECT s.id, s.path FROM issue_6576_s s
WHERE s.id @@@ paradedb.match('title', 'dragon')
  AND NOT EXISTS (
    SELECT 1 FROM issue_6576_e e WHERE e.s_id = s.id AND e.id @@@ paradedb.term('u', 'u1'))
ORDER BY paradedb.score(s.id) DESC, s.rating DESC, s.id LIMIT 5;

-- 2) The reported shape: the same anti join, projecting an expression over the
--    non-columnar column. It must take the Join Scan.
EXPLAIN (COSTS OFF)
SELECT s.id, 'x/' || s.path AS url FROM issue_6576_s s
WHERE s.id @@@ paradedb.match('title', 'dragon')
  AND NOT EXISTS (
    SELECT 1 FROM issue_6576_e e WHERE e.s_id = s.id AND e.id @@@ paradedb.term('u', 'u1'))
ORDER BY paradedb.score(s.id) DESC, s.rating DESC, s.id LIMIT 5;

SELECT s.id, 'x/' || s.path AS url FROM issue_6576_s s
WHERE s.id @@@ paradedb.match('title', 'dragon')
  AND NOT EXISTS (
    SELECT 1 FROM issue_6576_e e WHERE e.s_id = s.id AND e.id @@@ paradedb.term('u', 'u1'))
ORDER BY paradedb.score(s.id) DESC, s.rating DESC, s.id LIMIT 5;

SET paradedb.enable_join_custom_scan = off;
SELECT s.id, 'x/' || s.path AS url FROM issue_6576_s s
WHERE s.id @@@ paradedb.match('title', 'dragon')
  AND NOT EXISTS (
    SELECT 1 FROM issue_6576_e e WHERE e.s_id = s.id AND e.id @@@ paradedb.term('u', 'u1'))
ORDER BY paradedb.score(s.id) DESC, s.rating DESC, s.id LIMIT 5;
SET paradedb.enable_join_custom_scan = on;

-- 3) An inner join projecting an expression over non-columnar columns of both
--    sides. It must take the Join Scan.
EXPLAIN (COSTS OFF)
SELECT s.id, 'x/' || s.path AS url, s.path || ':' || e.note AS tag
FROM issue_6576_s s JOIN issue_6576_e e ON e.s_id = s.id
WHERE s.id @@@ paradedb.match('title', 'dragon') AND e.id @@@ paradedb.term('u', 'u1')
ORDER BY s.rating DESC, s.id LIMIT 5;

SELECT s.id, 'x/' || s.path AS url, s.path || ':' || e.note AS tag
FROM issue_6576_s s JOIN issue_6576_e e ON e.s_id = s.id
WHERE s.id @@@ paradedb.match('title', 'dragon') AND e.id @@@ paradedb.term('u', 'u1')
ORDER BY s.rating DESC, s.id LIMIT 5;

SET paradedb.enable_join_custom_scan = off;
SELECT s.id, 'x/' || s.path AS url, s.path || ':' || e.note AS tag
FROM issue_6576_s s JOIN issue_6576_e e ON e.s_id = s.id
WHERE s.id @@@ paradedb.match('title', 'dragon') AND e.id @@@ paradedb.term('u', 'u1')
ORDER BY s.rating DESC, s.id LIMIT 5;
SET paradedb.enable_join_custom_scan = on;

-- 4) A left join whose expression reads a non-columnar column of the
--    null-extended side. Unmatched rows evaluate the expression over NULL
--    inputs, and rows whose `note` is NULL do the same.
--
--    Control: the bare column takes the Join Scan.
EXPLAIN (COSTS OFF)
SELECT s.id, e.note
FROM issue_6576_s s LEFT JOIN issue_6576_e e ON e.s_id = s.id
WHERE s.id @@@ paradedb.match('title', 'dragon')
ORDER BY s.id LIMIT 10;

EXPLAIN (COSTS OFF)
SELECT s.id, coalesce('n/' || e.note, 'none') AS note_url
FROM issue_6576_s s LEFT JOIN issue_6576_e e ON e.s_id = s.id
WHERE s.id @@@ paradedb.match('title', 'dragon')
ORDER BY s.id LIMIT 10;

SELECT s.id, coalesce('n/' || e.note, 'none') AS note_url
FROM issue_6576_s s LEFT JOIN issue_6576_e e ON e.s_id = s.id
WHERE s.id @@@ paradedb.match('title', 'dragon')
ORDER BY s.id LIMIT 10;

SET paradedb.enable_join_custom_scan = off;
SELECT s.id, coalesce('n/' || e.note, 'none') AS note_url
FROM issue_6576_s s LEFT JOIN issue_6576_e e ON e.s_id = s.id
WHERE s.id @@@ paradedb.match('title', 'dragon')
ORDER BY s.id LIMIT 10;
SET paradedb.enable_join_custom_scan = on;

-- 5) An expression over a non-columnar column used as an ORDER BY key: the
--    scan sorts on it, so it should still decline, but without a message.
--    Any SRF in the target list sets hasTargetSRFs, which will cause the query
--    to be declined earlier by the LIMIT pushdown check.
EXPLAIN (COSTS OFF)
SELECT s.id, 'x/' || s.path AS url
FROM issue_6576_s s JOIN issue_6576_e e ON e.s_id = s.id
WHERE s.id @@@ paradedb.match('title', 'dragon') AND e.id @@@ paradedb.term('u', 'u1')
ORDER BY 'x/' || s.path, s.id LIMIT 5;

-- 6) An expression over a non-columnar column used as a DISTINCT key: the scan
--    deduplicates on it, so it still declines.
EXPLAIN (COSTS OFF)
SELECT DISTINCT 'x/' || s.path AS url
FROM issue_6576_s s JOIN issue_6576_e e ON e.s_id = s.id
WHERE s.id @@@ paradedb.match('title', 'dragon') AND e.id @@@ paradedb.term('u', 'u1')
LIMIT 5;

-- 7) An expression over a non-columnar column and a LATERAL unnest column.
--    The unnest column is an output of the scan, so Postgres can evaluate the
--    expression after it. It must take the Join Scan.
EXPLAIN (COSTS OFF)
SELECT s.id, s.path || ':' || t AS x
FROM issue_6576_s s JOIN issue_6576_e e ON e.s_id = s.id
CROSS JOIN LATERAL unnest(s.tags) AS t
WHERE s.id @@@ paradedb.match('title', 'dragon')
ORDER BY s.id, t LIMIT 5;

SELECT s.id, s.path || ':' || t AS x
FROM issue_6576_s s JOIN issue_6576_e e ON e.s_id = s.id
CROSS JOIN LATERAL unnest(s.tags) AS t
WHERE s.id @@@ paradedb.match('title', 'dragon')
ORDER BY s.id, t LIMIT 5;

SET paradedb.enable_join_custom_scan = off;
SELECT s.id, s.path || ':' || t AS x
FROM issue_6576_s s JOIN issue_6576_e e ON e.s_id = s.id
CROSS JOIN LATERAL unnest(s.tags) AS t
WHERE s.id @@@ paradedb.match('title', 'dragon')
ORDER BY s.id, t LIMIT 5;
SET paradedb.enable_join_custom_scan = on;

--    The same, over the unnest column alone, with a result type that isn't
--    Arrow-convertible.
EXPLAIN (COSTS OFF)
SELECT s.id, point(length(t), 1) AS p
FROM issue_6576_s s JOIN issue_6576_e e ON e.s_id = s.id
CROSS JOIN LATERAL unnest(s.tags) AS t
WHERE s.id @@@ paradedb.match('title', 'dragon')
ORDER BY s.id, t LIMIT 5;

SELECT s.id, point(length(t), 1) AS p
FROM issue_6576_s s JOIN issue_6576_e e ON e.s_id = s.id
CROSS JOIN LATERAL unnest(s.tags) AS t
WHERE s.id @@@ paradedb.match('title', 'dragon')
ORDER BY s.id, t LIMIT 5;

SET paradedb.enable_join_custom_scan = off;
SELECT s.id, point(length(t), 1) AS p
FROM issue_6576_s s JOIN issue_6576_e e ON e.s_id = s.id
CROSS JOIN LATERAL unnest(s.tags) AS t
WHERE s.id @@@ paradedb.match('title', 'dragon')
ORDER BY s.id, t LIMIT 5;
SET paradedb.enable_join_custom_scan = on;

-- 8) A set-returning function in the SELECT list. Postgres can't evaluate it
--    as a per-row projection after the scan, so it still declines.
EXPLAIN (COSTS OFF)
SELECT s.id, unnest(s.tags) AS tag
FROM issue_6576_s s JOIN issue_6576_e e ON e.s_id = s.id
WHERE s.id @@@ paradedb.match('title', 'dragon') AND e.id @@@ paradedb.term('u', 'u1')
ORDER BY s.id LIMIT 5;

EXPLAIN (COSTS OFF)
SELECT s.id, regexp_split_to_table(s.path, '/') AS part
FROM issue_6576_s s JOIN issue_6576_e e ON e.s_id = s.id
WHERE s.id @@@ paradedb.match('title', 'dragon') AND e.id @@@ paradedb.term('u', 'u1')
ORDER BY s.id LIMIT 5;

-- 9) An expression over a whole-row reference or a non-columnar system
--    column (xmin). It declines, and the warning names the column (issue #6908).
--    ctid and tableoid are columnar, so they don't take this path.
EXPLAIN (COSTS OFF)
SELECT s.id, s::text || s.path AS x
FROM issue_6576_s s JOIN issue_6576_e e ON e.s_id = s.id
WHERE s.id @@@ paradedb.match('title', 'dragon')
ORDER BY s.id LIMIT 5;

EXPLAIN (COSTS OFF)
SELECT s.id, s.xmin::text || s.path AS x
FROM issue_6576_s s JOIN issue_6576_e e ON e.s_id = s.id
WHERE s.id @@@ paradedb.match('title', 'dragon')
ORDER BY s.id LIMIT 5;

RESET paradedb.enable_join_custom_scan;
DROP TABLE issue_6576_s, issue_6576_e CASCADE;
