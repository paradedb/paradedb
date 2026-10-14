-- Regression coverage for issue #6582.
--
-- A scan that projects a score or a snippet must initialize a `SubPlan` of its
-- target list one time, not one time for each row.

\i common/common_setup.sql

SET paradedb.planner_warnings = off;

CREATE TABLE issue_6582_chunks (
    id bigint PRIMARY KEY,
    name text NOT NULL,
    kind text NOT NULL,
    body text NOT NULL
);

CREATE TABLE issue_6582_terms (
    term_id bigint NOT NULL,
    chunk_id bigint NOT NULL,
    PRIMARY KEY (term_id, chunk_id)
);

-- The body length changes from row to row, and so does the snippet length.
INSERT INTO issue_6582_chunks (id, name, kind, body)
SELECT g,
       'fn_' || g,
       CASE WHEN g % 50 = 0 THEN 'file' ELSE 'function' END,
       repeat('pad ', g % 17) || 'fred swing number ' || g
FROM generate_series(1, 2000) AS g;

INSERT INTO issue_6582_terms (term_id, chunk_id)
SELECT 1 + g % 2, g FROM generate_series(21, 2000, 21) AS g;

CREATE INDEX issue_6582_chunks_idx ON issue_6582_chunks
USING bm25 (id, name, kind, body);

ANALYZE issue_6582_chunks;
ANALYZE issue_6582_terms;

-- 1) The reported shape: a hashed `SubPlan` in the `ORDER BY` of a scan with a
--    score and a snippet.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, name, kind,
       paradedb.snippet(body, start_tag => '', end_tag => '', max_num_chars => 64) AS snip,
       round(paradedb.score(id)::numeric, 6) AS score
FROM issue_6582_chunks
WHERE body @@@ 'fred'
ORDER BY (id IN (SELECT chunk_id FROM issue_6582_terms WHERE term_id = ANY(ARRAY[1]))) DESC,
         (name = 'fn_84') DESC,
         (kind = 'file') ASC,
         paradedb.score(id) DESC,
         id
LIMIT 5;

SELECT id, name, kind,
       paradedb.snippet(body, start_tag => '', end_tag => '', max_num_chars => 64) AS snip,
       round(paradedb.score(id)::numeric, 6) AS score
FROM issue_6582_chunks
WHERE body @@@ 'fred'
ORDER BY (id IN (SELECT chunk_id FROM issue_6582_terms WHERE term_id = ANY(ARRAY[1]))) DESC,
         (name = 'fn_84') DESC,
         (kind = 'file') ASC,
         paradedb.score(id) DESC,
         id
LIMIT 5;

-- The scan runs the hashed `SubPlan` one time for all of its rows.
CREATE FUNCTION issue_6582_subplan_loops(q text) RETURNS SETOF jsonb AS $$
DECLARE
    plan jsonb;
BEGIN
    EXECUTE 'EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, FORMAT JSON) ' || q INTO plan;
    RETURN QUERY SELECT jsonb_path_query(
        plan, 'strict $.** ? (@."Parent Relationship" == "SubPlan")."Actual Loops"');
END;
$$ LANGUAGE plpgsql;

SELECT issue_6582_subplan_loops($$
    SELECT id,
           paradedb.snippet(body, start_tag => '', end_tag => '', max_num_chars => 64) AS snip,
           paradedb.score(id) AS score
    FROM issue_6582_chunks
    WHERE body @@@ 'fred'
    ORDER BY (id IN (SELECT chunk_id FROM issue_6582_terms WHERE term_id = ANY(ARRAY[1]))) DESC,
             score DESC
$$) AS subplan_loops;

-- 2) A correlated `SubPlan` in the `SELECT` list, without a `LIMIT`.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT count(*), sum(n), count(*) FILTER (WHERE score > 0) AS scored
FROM (
    SELECT (SELECT count(*) FROM issue_6582_terms t WHERE t.chunk_id = c.id) AS n,
           paradedb.score(c.id) AS score
    FROM issue_6582_chunks c
    WHERE c.body @@@ 'swing'
    OFFSET 0
) s;

SELECT count(*), sum(n), count(*) FILTER (WHERE score > 0) AS scored
FROM (
    SELECT (SELECT count(*) FROM issue_6582_terms t WHERE t.chunk_id = c.id) AS n,
           paradedb.score(c.id) AS score
    FROM issue_6582_chunks c
    WHERE c.body @@@ 'swing'
    OFFSET 0
) s;

-- 3) The score is an input of the `SubPlan`.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, paradedb.score(id) > ALL (SELECT term_id::real / 4 FROM issue_6582_terms) AS above
FROM issue_6582_chunks
WHERE body @@@ 'fred' AND id <= 60
ORDER BY above DESC, id
LIMIT 5;

SELECT id, paradedb.score(id) > ALL (SELECT term_id::real / 4 FROM issue_6582_terms) AS above
FROM issue_6582_chunks
WHERE body @@@ 'fred' AND id <= 60
ORDER BY above DESC, id
LIMIT 5;

-- 4) A Top K scan.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, paradedb.score(id) AS score,
       id IN (SELECT chunk_id - 4 FROM issue_6582_terms WHERE term_id = 2) AS tagged
FROM issue_6582_chunks
WHERE body @@@ 'fred' AND id <= 60
ORDER BY score DESC, id
LIMIT 5;

SELECT id, paradedb.score(id) AS score,
       id IN (SELECT chunk_id - 4 FROM issue_6582_terms WHERE term_id = 2) AS tagged
FROM issue_6582_chunks
WHERE body @@@ 'fred' AND id <= 60
ORDER BY score DESC, id
LIMIT 5;

-- 5) A rescan of the scan for each outer row.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT k, x.*
FROM generate_series(1, 2) AS k,
     LATERAL (
         SELECT c.id, paradedb.score(c.id) AS score
         FROM issue_6582_chunks c
         WHERE c.body @@@ 'fred' AND c.id <= 100
         ORDER BY (c.id IN (SELECT chunk_id FROM issue_6582_terms WHERE term_id = k)) DESC,
                  score DESC,
                  c.id
         LIMIT 3
     ) x
ORDER BY k, x.id;

SELECT k, x.*
FROM generate_series(1, 2) AS k,
     LATERAL (
         SELECT c.id, paradedb.score(c.id) AS score
         FROM issue_6582_chunks c
         WHERE c.body @@@ 'fred' AND c.id <= 100
         ORDER BY (c.id IN (SELECT chunk_id FROM issue_6582_terms WHERE term_id = k)) DESC,
                  score DESC,
                  c.id
         LIMIT 3
     ) x
ORDER BY k, x.id;

-- 6) A window aggregate placeholder.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, COUNT(*) OVER () AS total,
       id IN (SELECT chunk_id - 18 FROM issue_6582_terms WHERE term_id = 2) AS tagged
FROM issue_6582_chunks
WHERE body @@@ 'fred' AND id <= 60
ORDER BY id
LIMIT 5;

SELECT id, COUNT(*) OVER () AS total,
       id IN (SELECT chunk_id - 18 FROM issue_6582_terms WHERE term_id = 2) AS tagged
FROM issue_6582_chunks
WHERE body @@@ 'fred' AND id <= 60
ORDER BY id
LIMIT 5;

-- 7) An aggregate scan that wraps an aggregate next to a `SubPlan`.
CREATE TABLE issue_6582_logs (
    id serial PRIMARY KEY,
    description text NOT NULL,
    category text NOT NULL
);

INSERT INTO issue_6582_logs (description, category)
SELECT 'error ' || g, 'c' || (g % 500)
FROM generate_series(1, 1200) AS g;

CREATE INDEX issue_6582_logs_idx ON issue_6582_logs
USING bm25 (id, description, (category::pdb.literal));

ANALYZE issue_6582_logs;

SET paradedb.enable_aggregate_custom_scan = on;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT category, count(*) + 1 AS n, count(*) IN (SELECT term_id FROM issue_6582_terms) AS hit
FROM issue_6582_logs
WHERE description @@@ 'error'
GROUP BY category
ORDER BY category
LIMIT 5;

SELECT category, count(*) + 1 AS n, count(*) IN (SELECT term_id FROM issue_6582_terms) AS hit
FROM issue_6582_logs
WHERE description @@@ 'error'
GROUP BY category
ORDER BY category
LIMIT 5;

SELECT issue_6582_subplan_loops($$
    SELECT category, count(*) IN (SELECT term_id FROM issue_6582_terms) AS hit
    FROM issue_6582_logs
    WHERE description @@@ 'error'
    GROUP BY category
$$) AS subplan_loops;

-- A build of the projection for each row can stay hidden: when all builds
-- allocate the same sizes, every stale `SubPlanState` points at the last one,
-- which is valid. This target list is too wide for the first block of the
-- per-tuple memory, so Postgres frees its blocks at each reset.
SELECT format(
    $f$SELECT count(*) AS groups, count(*) FILTER (WHERE hit) AS hits, sum(c150) AS c150
       FROM (
           SELECT category, %s, count(*) IN (SELECT term_id FROM issue_6582_terms) AS hit
           FROM issue_6582_logs
           WHERE description @@@ 'error'
           GROUP BY category
           OFFSET 0
       ) s$f$,
    string_agg(format('count(*) + %s AS c%s', i, i), ', ' ORDER BY i))
FROM generate_series(1, 150) AS i \gexec

RESET paradedb.enable_aggregate_custom_scan;

DROP TABLE issue_6582_logs;
DROP FUNCTION issue_6582_subplan_loops(text);
DROP TABLE issue_6582_terms;
DROP TABLE issue_6582_chunks;
