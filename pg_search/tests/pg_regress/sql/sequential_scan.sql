\i common/common_setup.sql

CREATE EXTENSION pageinspect;
CREATE EXTENSION pg_visibility;

DROP TABLE IF EXISTS sequential_scan CASCADE;
CREATE TABLE sequential_scan (
    id int PRIMARY KEY,
    anchor text,
    body text NOT NULL,
    note text NOT NULL DEFAULT 'original',
    keep boolean NOT NULL DEFAULT true
) WITH (fillfactor = 50);
INSERT INTO sequential_scan
SELECT
    g,
    CASE WHEN g % 2 = 0 THEN NULL ELSE 'duplicate' END,
    'keyword number ' || g || CASE WHEN g IN (1, 2) THEN ' target' ELSE '' END,
    'original',
    true
FROM generate_series(1, 20000) g;
CREATE INDEX sequential_scan_idx ON sequential_scan USING paradedb (anchor, id, body) WHERE keep;
CREATE INDEX sequential_scan_anchor_idx ON sequential_scan (anchor) INCLUDE (id);

-- The per-row filter's deparsed qual embeds the BM25 index oid, which changes on every database
-- creation; mask it so the plans below are stable.
CREATE FUNCTION explain_seqscan(query text) RETURNS SETOF text LANGUAGE plpgsql AS $$
DECLARE
    line text;
BEGIN
    FOR line IN EXECUTE 'EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) ' || query LOOP
        RETURN NEXT regexp_replace(line, '"oid":\d+', '"oid":N');
    END LOOP;
END;
$$;

-- Tiny work_mem: the ~20k-key match set exceeds it and spills; count is still correct.
SET work_mem = '64kB';
SELECT explain_seqscan($$SELECT count(*) FROM sequential_scan WHERE body ||| 'keyword'$$);
SELECT count(*) FROM sequential_scan WHERE body ||| 'keyword';

-- A cursor's spilled cache survives rollback of the savepoint that first populated it.
BEGIN;
SET LOCAL paradedb.enable_custom_scan = off;
SET LOCAL enable_bitmapscan = off;
DECLARE sequential_scan_cursor CURSOR FOR
SELECT id FROM sequential_scan WHERE body ||| 'keyword';
SAVEPOINT before_first_fetch;
FETCH 1 FROM sequential_scan_cursor;
ROLLBACK TO before_first_fetch;
FETCH 1 FROM sequential_scan_cursor;
CLOSE sequential_scan_cursor;
ROLLBACK;

-- Error cleanup may release the cursor's files before its cache is dropped.
BEGIN;
SET LOCAL paradedb.enable_custom_scan = off;
SET LOCAL enable_bitmapscan = off;
DECLARE sequential_scan_cursor CURSOR FOR
SELECT id, 1 / (id - 2) FROM sequential_scan WHERE body ||| 'keyword';
SAVEPOINT before_first_fetch;
FETCH 1 FROM sequential_scan_cursor;
FETCH 1 FROM sequential_scan_cursor;
ROLLBACK TO before_first_fetch;
CLOSE sequential_scan_cursor;
ROLLBACK;

-- Membership correctness across the spilled, on-disk sorted set (probes low/mid/high keys).
SELECT explain_seqscan($$SELECT id FROM sequential_scan WHERE body ||| 'keyword' AND id IN (1, 10000, 20000) ORDER BY id$$);
SELECT id FROM sequential_scan WHERE body ||| 'keyword' AND id IN (1, 10000, 20000) ORDER BY id;

-- Negation over the spilled set: everything matched, so NOT (...) excludes all rows.
SELECT explain_seqscan($$SELECT count(*) FROM sequential_scan WHERE NOT (body ||| 'keyword')$$);
SELECT count(*) FROM sequential_scan WHERE NOT (body ||| 'keyword');

-- A term matched by no row -> empty set -> nothing matches (no spill).
SELECT explain_seqscan($$SELECT count(*) FROM sequential_scan WHERE body ||| 'nonexistentterm'$$);
SELECT count(*) FROM sequential_scan WHERE body ||| 'nonexistentterm';

-- Duplicate and NULL first-indexed values do not participate in row identity.
SELECT id FROM sequential_scan WHERE body ||| 'target' ORDER BY id;

-- A search predicate left as a residual filter on another index also receives the row CTID.
SET paradedb.enable_custom_scan = off;
RESET enable_indexscan;
SET enable_seqscan = off;
SET enable_bitmapscan = off;
SELECT explain_seqscan($$SELECT id FROM sequential_scan WHERE anchor = 'duplicate' AND body ||| 'target'$$);
SELECT id FROM sequential_scan WHERE anchor = 'duplicate' AND body ||| 'target';
SET enable_indexscan = off;
RESET enable_seqscan;
RESET enable_bitmapscan;
RESET paradedb.enable_custom_scan;

-- A parameterized search join can use the ParadeDB index, but its sequential fallback evaluates
-- the predicate as a join filter and therefore needs the searched row's CTID in the join target.
CREATE TEMP TABLE sequential_scan_terms(term text NOT NULL);
INSERT INTO sequential_scan_terms VALUES ('target');
ANALYZE sequential_scan;
ANALYZE sequential_scan_terms;
SET paradedb.enable_join_custom_scan = off;
SET paradedb.planner_warnings = off;
SET enable_hashjoin = off;
SET enable_mergejoin = off;
RESET enable_indexscan;
RESET enable_indexonlyscan;
SET enable_seqscan = off;
SELECT explain_seqscan($$SELECT s.id FROM sequential_scan_terms t JOIN sequential_scan s ON s.body @@@ t.term WHERE s.keep$$);

SET enable_indexscan = off;
SET enable_indexonlyscan = off;
RESET enable_seqscan;
SET enable_bitmapscan = off;
SELECT explain_seqscan($$SELECT s.id FROM sequential_scan_terms t JOIN sequential_scan s ON s.body @@@ t.term WHERE s.keep$$);
SELECT s.id
FROM sequential_scan_terms t
JOIN sequential_scan s ON s.body @@@ t.term
WHERE s.keep
ORDER BY s.id;
RESET enable_indexonlyscan;
RESET enable_bitmapscan;
RESET enable_hashjoin;
RESET enable_mergejoin;
RESET paradedb.enable_join_custom_scan;
RESET paradedb.planner_warnings;

-- The index retains the root CTID across this HOT update. The sequential scan compares against the
-- snapshot-visible member instead.
CREATE TEMP TABLE sequential_scan_hot_root AS
SELECT ctid AS root_ctid FROM sequential_scan WHERE id = 1;
UPDATE sequential_scan SET note = 'hot updated' WHERE id = 1;
SELECT id FROM sequential_scan WHERE body ||| 'target' ORDER BY id;

-- VACUUM prunes the HOT chain to a redirect and marks the page all-visible. CTID resolution must
-- still follow that redirect rather than returning the indexed root unchanged.
VACUUM (FREEZE, ANALYZE) sequential_scan;
SELECT all_visible
FROM pg_visibility_map('sequential_scan')
WHERE blkno = (SELECT (root_ctid::text::point)[0]::bigint FROM sequential_scan_hot_root);
SELECT lp_flags = 2 AS root_is_redirect
FROM heap_page_items(get_raw_page(
    'sequential_scan',
    (SELECT (root_ctid::text::point)[0]::integer FROM sequential_scan_hot_root)
))
WHERE lp = (SELECT (root_ctid::text::point)[1]::smallint FROM sequential_scan_hot_root);
SELECT id FROM sequential_scan WHERE body ||| 'target' ORDER BY id;

-- With ample work_mem the identical query stays in memory: no spill WARNING.
SET work_mem = '256MB';
SELECT explain_seqscan($$SELECT count(*) FROM sequential_scan WHERE body ||| 'keyword'$$);
SELECT count(*) FROM sequential_scan WHERE body ||| 'keyword';

DROP TABLE sequential_scan CASCADE;
DROP EXTENSION pg_visibility;
DROP EXTENSION pageinspect;

-- Preserve #5264 for field predicates without inferring NULL for a compound query.
CREATE TABLE sequential_scan_nulls (id int PRIMARY KEY, color text, covered boolean);
INSERT INTO sequential_scan_nulls VALUES (1, 'blue', true), (2, 'red', true), (3, NULL, true);
CREATE INDEX sequential_scan_nulls_idx ON sequential_scan_nulls
USING paradedb (id, color) WITH (
    text_fields = '{"color":{"tokenizer":{"type":"keyword"},"fast":true}}'
);

CREATE VIEW sequential_scan_null_checks AS
SELECT id, covered,
       color @@@ 'blue' AS field_match,
       id @@@ paradedb.boost(2.0, paradedb.const_score(1.0, paradedb.term('color', 'blue'))) AS wrapped_match,
       NOT (id @@@ paradedb.const_score(1.0, paradedb.boost(2.0, paradedb.exists('color')))) AS missing,
       id @@@ paradedb.boolean(
           must => paradedb.exists('color'),
           must_not => paradedb.term('color', 'red')
       ) AS compound,
       id @@@ paradedb.boost(2.0, paradedb.const_score(1.0, paradedb.boolean(
           must => paradedb.exists('color'),
           must_not => paradedb.term('color', 'red')
       ))) AS wrapped_compound,
       id @@@ paradedb.boolean(must => ARRAY[
           paradedb.term('color', 'blue'), paradedb.term('color', 'red')
       ]) AS query_and,
       id @@@ paradedb.boolean(should => ARRAY[
           paradedb.term('color', 'blue'), paradedb.term('color', 'red')
       ]) AS query_or,
       (color === 'blue') AND (color === 'red') AS sql_and,
       (color === 'blue') OR (color === 'red') AS sql_or
FROM sequential_scan_nulls;

\pset null 'NULL'
SET paradedb.enable_custom_scan = off;
SET enable_bitmapscan = off;
SELECT id, color @@@ pdb.all() AS field_all,
       color @@@ pdb.all()::pdb.boost(2) AS boosted_all,
       color @@@ pdb.all()::pdb.const(1) AS constant_all,
       color @@@ pdb.empty() AS field_empty,
       id @@@ paradedb.all() AS all_rows
FROM sequential_scan_nulls ORDER BY id;
SELECT id, field_match, wrapped_match, missing, compound, wrapped_compound
FROM sequential_scan_null_checks ORDER BY id;
SELECT id FROM sequential_scan_null_checks WHERE NOT field_match ORDER BY id;
SELECT id FROM sequential_scan_null_checks WHERE NOT wrapped_match ORDER BY id;
SELECT explain_seqscan($$SELECT id FROM sequential_scan_null_checks WHERE NOT compound$$);
SELECT id FROM sequential_scan_null_checks WHERE NOT compound ORDER BY id;

SELECT id, query_and, query_or, sql_and, sql_or FROM sequential_scan_null_checks ORDER BY id;
SELECT id FROM sequential_scan_null_checks WHERE NOT query_and ORDER BY id;
SELECT id FROM sequential_scan_null_checks WHERE NOT query_or ORDER BY id;
SELECT id FROM sequential_scan_null_checks WHERE NOT sql_and ORDER BY id;
SELECT id FROM sequential_scan_null_checks WHERE NOT sql_or ORDER BY id;

SET paradedb.enable_custom_scan = on;
SELECT id FROM sequential_scan_null_checks WHERE NOT field_match ORDER BY id;
SELECT id FROM sequential_scan_null_checks WHERE NOT wrapped_match ORDER BY id;
SELECT explain_seqscan($$SELECT id FROM sequential_scan_null_checks WHERE NOT compound$$);
SELECT id FROM sequential_scan_null_checks WHERE NOT compound ORDER BY id;

SELECT id FROM sequential_scan_null_checks WHERE NOT query_and ORDER BY id;
SELECT id FROM sequential_scan_null_checks WHERE NOT query_or ORDER BY id;
SELECT id FROM sequential_scan_null_checks WHERE NOT sql_and ORDER BY id;
SELECT id FROM sequential_scan_null_checks WHERE NOT sql_or ORDER BY id;

-- Rows outside the partial index use the inline matcher, with the same NULL behavior.
SET paradedb.enable_custom_scan = off;
DROP INDEX sequential_scan_nulls_idx;
CREATE INDEX sequential_scan_nulls_idx ON sequential_scan_nulls
USING paradedb (id, color) WITH (
    text_fields = '{"color":{"tokenizer":{"type":"keyword"},"fast":true}}'
) WHERE covered;
UPDATE sequential_scan_nulls SET covered = false;
SELECT id, field_match, wrapped_match, missing, compound, wrapped_compound
FROM sequential_scan_null_checks ORDER BY id;
SELECT id FROM sequential_scan_null_checks WHERE NOT field_match ORDER BY id;
SELECT id FROM sequential_scan_null_checks WHERE NOT wrapped_match ORDER BY id;
SELECT id FROM sequential_scan_null_checks WHERE NOT compound ORDER BY id;

SELECT id, query_and, query_or, sql_and, sql_or FROM sequential_scan_null_checks ORDER BY id;
SELECT id FROM sequential_scan_null_checks WHERE NOT query_and ORDER BY id;
SELECT id FROM sequential_scan_null_checks WHERE NOT query_or ORDER BY id;
SELECT id FROM sequential_scan_null_checks WHERE NOT sql_and ORDER BY id;
SELECT id FROM sequential_scan_null_checks WHERE NOT sql_or ORDER BY id;

DROP VIEW sequential_scan_null_checks;
UPDATE sequential_scan_nulls SET covered = true;

SELECT paradedb.search_with_query_input_ctid(
           1, NULL::paradedb.searchqueryinput, '(0,1)'::tid
       ) IS NULL AS null_query,
       paradedb.search_with_query_input_ctid(
           1, paradedb.empty(), NULL::tid
       ) IS NULL AS null_ctid,
       paradedb.search_with_query_input_ctid(
           NULL::int, paradedb.empty(), '(0,1)'::tid
       ) IS FALSE AS nullable_anchor,
       paradedb.search_with_query_input_ctid_strict(
           NULL::int, paradedb.empty(), '(0,1)'::tid
       ) IS NULL AS strict_anchor;

INSERT INTO sequential_scan_nulls VALUES
    (4, repeat('a', 4000), true), (5, repeat('b', 4000), true);
CREATE TABLE sequential_scan_queries (label text, query paradedb.searchqueryinput);
ALTER TABLE sequential_scan_queries ALTER COLUMN query SET STORAGE EXTENDED;
INSERT INTO sequential_scan_queries VALUES (
    'compressed', paradedb.with_index('sequential_scan_nulls_idx', paradedb.term('color', repeat('a', 4000)))
);
ALTER TABLE sequential_scan_queries ALTER COLUMN query SET STORAGE EXTERNAL;
INSERT INTO sequential_scan_queries VALUES (
    'external', paradedb.with_index('sequential_scan_nulls_idx', paradedb.term('color', repeat('b', 4000)))
);
SELECT label, pg_column_size(query) < 4000 AS compressed
FROM sequential_scan_queries ORDER BY label;
SELECT q.label, t.id,
       paradedb.search_with_query_input_ctid(t.id, q.query, t.ctid) AS matched
FROM sequential_scan_queries q CROSS JOIN sequential_scan_nulls t
WHERE t.id IN (4, 5)
ORDER BY q.label, t.id;
DROP TABLE sequential_scan_queries;
DROP INDEX sequential_scan_nulls_idx;
CREATE INDEX sequential_scan_nulls_idx ON sequential_scan_nulls
USING paradedb (id, (color::pdb.literal)) WHERE covered;
SELECT id, color @@@ pdb.all() AS field_all,
       color @@@ pdb.all()::pdb.boost(2) AS boosted_all
FROM sequential_scan_nulls WHERE covered AND id <= 3 ORDER BY id;

-- The cached set of NULL rows must also survive a savepoint rollback.
BEGIN;
SET LOCAL work_mem = '64kB';
INSERT INTO sequential_scan_nulls SELECT g, NULL, true FROM generate_series(6, 3005) g;
DECLARE sequential_scan_null_cursor CURSOR FOR
SELECT id, color @@@ pdb.all() IS NULL AS missing FROM sequential_scan_nulls WHERE covered;
SAVEPOINT before_first_fetch;
FETCH 1 FROM sequential_scan_null_cursor;
ROLLBACK TO before_first_fetch;
FETCH 2 FROM sequential_scan_null_cursor;
CLOSE sequential_scan_null_cursor;
ROLLBACK;

DROP TABLE sequential_scan_nulls;
RESET paradedb.enable_custom_scan;
RESET enable_bitmapscan;
\pset null ''

-- An expression in the first index slot must remain an expression, not a whole-row Var.
CREATE TABLE sequential_scan_expr_first (id bigint, body text, note text);
INSERT INTO sequential_scan_expr_first VALUES
    (1, 'Alpha', 'needle'), (2, 'Beta', 'other'), (3, NULL, 'needle');
CREATE INDEX sequential_scan_expr_first_idx ON sequential_scan_expr_first
USING paradedb ((lower(body)::pdb.literal('alias=body_lower')));
SET paradedb.enable_custom_scan = off;
SET enable_indexscan = off;
SET enable_indexonlyscan = off;
SET enable_bitmapscan = off;
SET enable_seqscan = on;
SELECT explain_seqscan($$SELECT id FROM sequential_scan_expr_first WHERE lower(body) === 'alpha'$$);
SELECT id FROM sequential_scan_expr_first WHERE lower(body) === 'alpha';
SELECT id, lower(body) === 'alpha' AS matches FROM sequential_scan_expr_first ORDER BY id;

-- Preserve the query's relation number and outer-join nulling.
SELECT p.id, e.id, lower(e.body) === 'alpha' AS matches
FROM (VALUES (1), (2), (3), (4)) p(id)
LEFT JOIN sequential_scan_expr_first e ON e.id = p.id ORDER BY p.id;
SELECT p.term, (SELECT count(*) FROM sequential_scan_expr_first e WHERE lower(e.body) === p.term) AS matches
FROM (VALUES ('alpha'), ('beta'), ('missing')) p(term) ORDER BY p.term;

SET enable_indexscan = on;
SET enable_seqscan = off;
SELECT explain_seqscan($$SELECT id FROM sequential_scan_expr_first WHERE lower(body) === 'alpha'$$);
SELECT id FROM sequential_scan_expr_first WHERE lower(body) === 'alpha';
SET enable_indexscan = off;
SET enable_bitmapscan = on;
SELECT explain_seqscan($$SELECT id FROM sequential_scan_expr_first WHERE lower(body) === 'alpha'$$);
SELECT id FROM sequential_scan_expr_first WHERE lower(body) === 'alpha';

-- A NULL first expression must not suppress a match on another indexed expression.
DROP INDEX sequential_scan_expr_first_idx;
CREATE INDEX sequential_scan_expr_first_idx ON sequential_scan_expr_first USING paradedb (
    (lower(body)::pdb.literal('alias=body_lower')),
    (lower(note)::pdb.literal('alias=note_lower'))
);
SET enable_indexscan = on;
SET enable_bitmapscan = off;
SELECT explain_seqscan($$SELECT id FROM sequential_scan_expr_first WHERE lower(note) === 'needle'$$);
SELECT id FROM sequential_scan_expr_first WHERE lower(note) === 'needle' ORDER BY id;
SET enable_indexscan = off;
SET enable_seqscan = on;
SELECT id FROM sequential_scan_expr_first WHERE lower(note) === 'needle' ORDER BY id;

-- Rebind every column in a multi-column expression to the searched table.
DROP INDEX sequential_scan_expr_first_idx;
CREATE INDEX sequential_scan_expr_first_idx ON sequential_scan_expr_first USING paradedb (
    ((lower(body) || ':' || note)::pdb.literal('alias=body_note'))
);
SELECT p.id, (lower(e.body) || ':' || e.note) === 'alpha:needle' AS matches
FROM (VALUES (1), (2), (3), (4)) p(id)
LEFT JOIN sequential_scan_expr_first e ON e.id = p.id ORDER BY p.id;

DROP TABLE sequential_scan_expr_first;
RESET paradedb.enable_custom_scan;
RESET enable_indexscan;
RESET enable_indexonlyscan;
RESET enable_bitmapscan;
RESET enable_seqscan;
