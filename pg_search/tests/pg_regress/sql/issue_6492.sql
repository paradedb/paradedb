-- A generic plan solves its parameters into the search query once per scan, and
-- each segment reads the solved query when it starts. `lib` is not in the index,
-- so `lib = $2` is a heap filter that the solve rewrites. The solved query must
-- outlive the per-row memory resets of the Base Scan (`score()`, `snippet()`)
-- and of the Aggregate Scan (wrapped aggregates), and the solves of the
-- Aggregate Scan's `FILTER` clauses.
-- https://github.com/paradedb/paradedb/issues/6492

\i common/common_setup.sql

SET paradedb.enable_columnar_exec = false;

CREATE TABLE issue_6492 (id int PRIMARY KEY, body text, lib text);
CREATE INDEX issue_6492_idx ON issue_6492 USING bm25 (id, body);

-- each insert adds a segment
INSERT INTO issue_6492
SELECT g,
       repeat(CASE WHEN g % 4 = 0 THEN 'contrato de prestacao ' ELSE 'alpha beta servicos ' END
              || md5(g::text) || ' ', 1 + g % 8),
       'lib' || (g % 5)
FROM generate_series(1, 500) g;
INSERT INTO issue_6492
SELECT g,
       repeat(CASE WHEN g % 4 = 0 THEN 'contrato de prestacao ' ELSE 'alpha beta servicos ' END
              || md5(g::text) || ' ', 1 + g % 8),
       'lib' || (g % 5)
FROM generate_series(501, 1000) g;
INSERT INTO issue_6492
SELECT g,
       repeat(CASE WHEN g % 4 = 0 THEN 'contrato de prestacao ' ELSE 'alpha beta servicos ' END
              || md5(g::text) || ' ', 1 + g % 8),
       'lib' || (g % 5)
FROM generate_series(1001, 1500) g;

SELECT count(*) > 1 AS several_segments FROM paradedb.index_info('issue_6492_idx');

PREPARE scored(text, text) AS
SELECT id, paradedb.score(id) > 0 AS scored
FROM issue_6492 WHERE body @@@ $1 AND lib = $2
ORDER BY id;

PREPARE snippets(text, text) AS
SELECT id, paradedb.snippet(body) IS NOT NULL AS has_snippet, paradedb.score(id) > 0 AS scored
FROM issue_6492 WHERE body @@@ $1 AND lib = $2
ORDER BY id;

PREPARE filtered(text, text) AS
SELECT COUNT(*) AS total, COUNT(*) FILTER (WHERE body @@@ 'contrato') AS contrato
FROM issue_6492 WHERE body @@@ $1 AND lib = $2;

-- A wrapped aggregate is projected in per-row memory, which the Aggregate Scan
-- resets for every row it returns. EXPLAIN ANALYZE prints the solved query after
-- the last row.
PREPARE wrapped(text, text) AS
SELECT id, repeat(COUNT(*)::text, 200) AS counts
FROM issue_6492 WHERE body @@@ $1 AND lib = $2
GROUP BY id;

-- keeps only the scan line, without the PostgreSQL-version-specific row counts
CREATE FUNCTION issue_6492_explain_analyze(q text) RETURNS SETOF text AS $$
DECLARE
    r record;
BEGIN
    FOR r IN EXECUTE 'EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF) ' || q LOOP
        IF r."QUERY PLAN" LIKE '%Custom Scan%' THEN
            RETURN NEXT regexp_replace(r."QUERY PLAN", '\s*\(actual.*$', '');
        END IF;
    END LOOP;
END $$ LANGUAGE plpgsql;

-- `wrapped` returns 300 wide rows; generic and custom plans are compared by summary
CREATE FUNCTION issue_6492_wrapped_summary(q text) RETURNS text AS $$
DECLARE
    r record;
    n bigint := 0;
    id_sum bigint := 0;
    weighted_counts bigint := 0;
    count_text text;
BEGIN
    FOR r IN EXECUTE q LOOP
        -- `counts` repeats the count's text 200 times
        count_text := left(r.counts, length(r.counts) / 200);
        IF r.counts <> repeat(count_text, 200) THEN
            RAISE EXCEPTION 'id %: malformed counts', r.id;
        END IF;
        n := n + 1;
        id_sum := id_sum + r.id;
        weighted_counts := weighted_counts + r.id * count_text::bigint;
    END LOOP;
    RETURN n || ' rows, id sum ' || id_sum || ', id-weighted count sum ' || weighted_counts;
END $$ LANGUAGE plpgsql;

SET plan_cache_mode = force_generic_plan;

EXPLAIN (COSTS OFF) EXECUTE scored('contrato', 'lib1');
EXECUTE scored('contrato', 'lib1');

EXECUTE snippets('contrato', 'lib1');

EXPLAIN (COSTS OFF) EXECUTE filtered('servicos OR prestacao', 'lib1');
EXECUTE filtered('servicos OR prestacao', 'lib1');
EXECUTE filtered('servicos OR prestacao', 'lib2');

SELECT * FROM issue_6492_explain_analyze($$EXECUTE wrapped('servicos OR prestacao', 'lib1')$$);
SELECT * FROM issue_6492_explain_analyze($$EXECUTE wrapped('servicos OR prestacao', 'lib2')$$);
SELECT issue_6492_wrapped_summary($$EXECUTE wrapped('servicos OR prestacao', 'lib1')$$);
SELECT issue_6492_wrapped_summary($$EXECUTE wrapped('servicos OR prestacao', 'lib2')$$);

-- the same results from custom plans
SET plan_cache_mode = force_custom_plan;

EXECUTE snippets('contrato', 'lib1');
EXECUTE filtered('servicos OR prestacao', 'lib1');
EXECUTE filtered('servicos OR prestacao', 'lib2');
SELECT issue_6492_wrapped_summary($$EXECUTE wrapped('servicos OR prestacao', 'lib1')$$);
SELECT issue_6492_wrapped_summary($$EXECUTE wrapped('servicos OR prestacao', 'lib2')$$);

DEALLOCATE scored;
DEALLOCATE snippets;
DEALLOCATE filtered;
DEALLOCATE wrapped;
DROP FUNCTION issue_6492_wrapped_summary;
RESET plan_cache_mode;

-- A rescan solves the query again. A nested-loop parameter in the heap filter is
-- enough: the inner scan runs once per outer row, and each run must drop the
-- previous solved query before solving the next one.
SELECT * FROM issue_6492_explain_analyze($$
SELECT l.lib, count(x.id) AS total, count(x.snippet) AS snippets,
       count(*) FILTER (WHERE x.score > 0) AS scored
FROM (VALUES ('lib1'), ('nolib'), ('lib3')) l(lib)
LEFT JOIN LATERAL (
    SELECT id, paradedb.snippet(body) AS snippet, paradedb.score(id) AS score
    FROM issue_6492 WHERE body @@@ 'contrato' AND issue_6492.lib = l.lib
    OFFSET 0
) x ON true
GROUP BY l.lib ORDER BY l.lib$$);

SELECT l.lib, count(x.id) AS total, count(x.snippet) AS snippets,
       count(*) FILTER (WHERE x.score > 0) AS scored
FROM (VALUES ('lib1'), ('nolib'), ('lib3')) l(lib)
LEFT JOIN LATERAL (
    SELECT id, paradedb.snippet(body) AS snippet, paradedb.score(id) AS score
    FROM issue_6492 WHERE body @@@ 'contrato' AND issue_6492.lib = l.lib
    OFFSET 0
) x ON true
GROUP BY l.lib ORDER BY l.lib;

-- the same for the Aggregate Scan's FILTER solves
SELECT * FROM issue_6492_explain_analyze($$
SELECT l.lib, x.total, x.contrato
FROM (VALUES ('lib1'), ('nolib'), ('lib3')) l(lib)
CROSS JOIN LATERAL (
    SELECT COUNT(*) AS total, COUNT(*) FILTER (WHERE body @@@ 'contrato') AS contrato
    FROM issue_6492 WHERE body @@@ 'servicos OR prestacao' AND issue_6492.lib = l.lib
) x ORDER BY l.lib$$);

SELECT l.lib, x.total, x.contrato
FROM (VALUES ('lib1'), ('nolib'), ('lib3')) l(lib)
CROSS JOIN LATERAL (
    SELECT COUNT(*) AS total, COUNT(*) FILTER (WHERE body @@@ 'contrato') AS contrato
    FROM issue_6492 WHERE body @@@ 'servicos OR prestacao' AND issue_6492.lib = l.lib
) x ORDER BY l.lib;

-- The same with a parallel scan: each process solves the query once and reads it
-- for every segment it claims. Without workers, the leader claims them all.
CREATE TABLE issue_6492_parallel (id int PRIMARY KEY, body text, lib text);
CREATE INDEX issue_6492_parallel_idx ON issue_6492_parallel USING bm25 (id, body);

INSERT INTO issue_6492_parallel SELECT * FROM issue_6492;
INSERT INTO issue_6492_parallel
SELECT g,
       repeat(CASE WHEN g % 4 = 0 THEN 'contrato de prestacao ' ELSE 'alpha beta servicos ' END
              || md5(g::text) || ' ', 1 + g % 8),
       'lib' || (g % 5)
FROM generate_series(1501, 3000) g;

SELECT count(*) > 1 AS several_segments FROM paradedb.index_info('issue_6492_parallel_idx');

SET max_parallel_workers_per_gather = 2;
SET parallel_setup_cost = 0;
SET parallel_tuple_cost = 0;
SET min_parallel_table_scan_size = 0;
SET paradedb.min_rows_per_worker = 0;
-- keep the counts from replacing the Base Scan
SET paradedb.enable_aggregate_custom_scan = false;

PREPARE parallel_snippets(text, text) AS
SELECT count(*) AS total, count(paradedb.snippet(body)) AS snippets,
       count(*) FILTER (WHERE paradedb.score(id) > 0) AS scored, sum(id) AS id_sum
FROM issue_6492_parallel WHERE body @@@ $1 AND lib = $2;

SET plan_cache_mode = force_generic_plan;
-- the scan line only: the worker count follows the segment count
SELECT * FROM issue_6492_explain_analyze($$EXECUTE parallel_snippets('contrato', 'lib1')$$);
EXECUTE parallel_snippets('contrato', 'lib1');
EXECUTE parallel_snippets('contrato', 'lib3');

-- the same results from custom plans
SET plan_cache_mode = force_custom_plan;
EXECUTE parallel_snippets('contrato', 'lib1');
EXECUTE parallel_snippets('contrato', 'lib3');

DEALLOCATE parallel_snippets;
DROP FUNCTION issue_6492_explain_analyze;
RESET plan_cache_mode;
RESET parallel_setup_cost;
RESET parallel_tuple_cost;
RESET min_parallel_table_scan_size;
RESET paradedb.min_rows_per_worker;
RESET paradedb.enable_aggregate_custom_scan;
RESET paradedb.enable_columnar_exec;
DROP TABLE issue_6492_parallel;
DROP TABLE issue_6492;

\i common/common_cleanup.sql
