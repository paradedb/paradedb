-- Regression test for #5108: placeholder functions (score / snippet /
-- snippet_positions) panicked with "Unsupported query shape" in parallel plans
-- using comma-join syntax (FROM a, b WHERE ...). placeholder_support only
-- treated an explicit JOIN ... ON (T_JoinExpr) as a join context; a comma join
-- is a FromExpr with >1 fromlist entries and no JoinExpr, so the placeholder was
-- left unwrapped and re-evaluated above the Gather Merge.

CREATE EXTENSION IF NOT EXISTS pg_search;

DROP TABLE IF EXISTS issue_5108_chunks CASCADE;
DROP TABLE IF EXISTS issue_5108_docs CASCADE;

CREATE TABLE issue_5108_docs (
    id bigserial PRIMARY KEY,
    filename text
);
CREATE TABLE issue_5108_chunks (
    id bigserial PRIMARY KEY,
    doc_id bigint REFERENCES issue_5108_docs(id),
    body text
);

INSERT INTO issue_5108_docs (filename)
SELECT 'doc_' || g || '.pdf' FROM generate_series(1, 10) g;

CREATE INDEX issue_5108_chunks_bm25 ON issue_5108_chunks
USING bm25 (id, body) WITH (key_field = 'id');

INSERT INTO issue_5108_chunks (doc_id, body)
SELECT ((g - 1) % 10) + 1,
       CASE WHEN g % 3 = 0 THEN 'healthcare notes ' || g ELSE 'unrelated ' || g END
FROM generate_series(1, 6000) g;

ANALYZE issue_5108_docs;
ANALYZE issue_5108_chunks;

SET max_parallel_workers_per_gather = 2;
SET debug_parallel_query = on;
SET parallel_setup_cost = 0;
SET parallel_tuple_cost = 0;
SET min_parallel_table_scan_size = 0;
SET min_parallel_index_scan_size = 0;
SET enable_hashjoin = off;
SET enable_mergejoin = off;

-- Comma join: every placeholder family must plan the dangerous shape and
-- execute without panicking. VERBOSE captures output lists through Gather Merge;
-- the following SELECT executes the same query.
EXPLAIN (VERBOSE, COSTS OFF, TIMING OFF)
SELECT c.body, d.filename, pdb.score(c.id) AS s
FROM issue_5108_chunks c, issue_5108_docs d
WHERE d.id = c.doc_id AND c.id @@@ paradedb.match('body', 'healthcare')
ORDER BY s DESC, c.id LIMIT 100;

SELECT c.body, d.filename, pdb.score(c.id) AS s
FROM issue_5108_chunks c, issue_5108_docs d
WHERE d.id = c.doc_id AND c.id @@@ paradedb.match('body', 'healthcare')
ORDER BY s DESC, c.id LIMIT 100;

EXPLAIN (VERBOSE, COSTS OFF, TIMING OFF)
SELECT pdb.snippet(c.body) AS snip, d.filename
FROM issue_5108_chunks c, issue_5108_docs d
WHERE d.id = c.doc_id AND c.id @@@ paradedb.match('body', 'healthcare')
ORDER BY snip DESC, c.id LIMIT 100;

SELECT pdb.snippet(c.body) AS snip, d.filename
FROM issue_5108_chunks c, issue_5108_docs d
WHERE d.id = c.doc_id AND c.id @@@ paradedb.match('body', 'healthcare')
ORDER BY snip DESC, c.id LIMIT 100;

EXPLAIN (VERBOSE, COSTS OFF, TIMING OFF)
SELECT pdb.snippet_positions(c.body) AS pos, d.filename
FROM issue_5108_chunks c, issue_5108_docs d
WHERE d.id = c.doc_id AND c.id @@@ paradedb.match('body', 'healthcare')
ORDER BY pos DESC, c.id LIMIT 100;

SELECT pdb.snippet_positions(c.body) AS pos, d.filename
FROM issue_5108_chunks c, issue_5108_docs d
WHERE d.id = c.doc_id AND c.id @@@ paradedb.match('body', 'healthcare')
ORDER BY pos DESC, c.id LIMIT 100;

-- Explicit JOIN ON via an inlined single-use CTE (the placeholder is planned in
-- a single-table level before the parent join consumes it).
EXPLAIN (VERBOSE, COSTS OFF, TIMING OFF)
WITH matched AS (
    SELECT id, body, doc_id, pdb.score(id) AS s
    FROM issue_5108_chunks
    WHERE id @@@ paradedb.match('body', 'healthcare')
    ORDER BY s DESC, id LIMIT 100
)
SELECT m.body, d.filename, m.s
FROM matched m JOIN issue_5108_docs d ON d.id = m.doc_id
ORDER BY m.s DESC, m.id;

WITH matched AS (
    SELECT id, body, doc_id, pdb.score(id) AS s
    FROM issue_5108_chunks
    WHERE id @@@ paradedb.match('body', 'healthcare')
    ORDER BY s DESC, id LIMIT 100
)
SELECT m.body, d.filename, m.s
FROM matched m JOIN issue_5108_docs d ON d.id = m.doc_id
ORDER BY m.s DESC, m.id;

EXPLAIN (VERBOSE, COSTS OFF, TIMING OFF)
WITH matched AS (
    SELECT id, doc_id, pdb.snippet(body) AS snip
    FROM issue_5108_chunks
    WHERE id @@@ paradedb.match('body', 'healthcare')
    ORDER BY snip DESC, id LIMIT 100
)
SELECT m.snip, d.filename
FROM matched m JOIN issue_5108_docs d ON d.id = m.doc_id
ORDER BY m.snip DESC, m.id;

WITH matched AS (
    SELECT id, doc_id, pdb.snippet(body) AS snip
    FROM issue_5108_chunks
    WHERE id @@@ paradedb.match('body', 'healthcare')
    ORDER BY snip DESC, id LIMIT 100
)
SELECT m.snip, d.filename
FROM matched m JOIN issue_5108_docs d ON d.id = m.doc_id
ORDER BY m.snip DESC, m.id;

RESET max_parallel_workers_per_gather;
RESET debug_parallel_query;
RESET parallel_setup_cost;
RESET parallel_tuple_cost;
RESET min_parallel_table_scan_size;
RESET min_parallel_index_scan_size;
RESET enable_hashjoin;
RESET enable_mergejoin;

DROP TABLE IF EXISTS issue_5108_chunks CASCADE;
DROP TABLE IF EXISTS issue_5108_docs CASCADE;
