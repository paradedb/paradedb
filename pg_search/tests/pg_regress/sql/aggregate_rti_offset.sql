-- =====================================================================
-- RTI offset fix for AggregateScan (issue #5266)
-- =====================================================================
-- Tests INDEX_VAR resolution in cross-table custom_exprs when an
-- AggregateScan on a JOIN is nested as a scalar subquery inside a larger
-- outer query.
--
-- In that shape, PostgreSQL's setrefs phase rewrites Var nodes in
-- custom_scan_tlist with outer-context RTIs (e.g. 2, 3) while the sources
-- were planned with inner RTIs (e.g. 1, 2), so execution-time RTI lookups
-- fail. The fix: build_tlist_col_map precomputes, at plan_custom_path time
-- (before setrefs, while inner RTIs are still valid), a tlist-position →
-- DataFusion column name map that is stored in PrivateData; execution
-- resolves INDEX_VAR references through it with no RTI arithmetic at all.

CREATE EXTENSION IF NOT EXISTS pg_search;
SET paradedb.enable_aggregate_custom_scan TO on;

-- =====================================================================
-- Test data: two small tables with matching ids and names
-- =====================================================================

CREATE TABLE rti_p (id INT PRIMARY KEY, name TEXT NOT NULL);
CREATE TABLE rti_o (id INT PRIMARY KEY, name TEXT NOT NULL);

INSERT INTO rti_p VALUES (1, 'bob'), (2, 'alice'), (3, 'charlie');
INSERT INTO rti_o VALUES (1, 'bob'), (2, 'alice'), (3, 'charlie');

CREATE INDEX rti_p_idx ON rti_p
USING bm25 (id, name)
WITH (key_field='id', text_fields='{"name": {"fast": true}}');

CREATE INDEX rti_o_idx ON rti_o
USING bm25 (id, name)
WITH (key_field='id', text_fields='{"name": {"fast": true}}');

-- =====================================================================
-- Section 1: Baseline — direct query, no RTI offset
--
-- Inner RTIs match outer RTIs (no scalar-subquery nesting, so setrefs
-- does not renumber). Verifies the cross-table OR custom_expr path works
-- correctly before any RTI shift is introduced.
--
-- Cross-table OR: (o.id = 1 OR p.id = 2) cannot be pushed to either
-- individual scan → becomes a custom_expr in the AggregateScan.
--
-- Expected: id=1 (p.name='bob' @@@ 'bob OR alice' ✓, o.id=1 ✓) +
--           id=2 (p.name='alice' @@@ 'bob OR alice' ✓, p.id=2 ✓) = 2
-- =====================================================================

SELECT COUNT(*)
FROM rti_p p JOIN rti_o o ON p.id = o.id
WHERE p.name @@@ 'bob OR alice' AND (o.id = 1 OR p.id = 2);

-- Parity: native PG (same filter, no @@@ operator)
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT COUNT(*)
FROM rti_p p JOIN rti_o o ON p.id = o.id
WHERE p.name IN ('bob', 'alice') AND (o.id = 1 OR p.id = 2);
SET paradedb.enable_aggregate_custom_scan TO on;

-- =====================================================================
-- Section 2: RTI offset = 1
--
-- A single-table scalar subquery precedes the agg-on-join subquery.
-- The first subquery adds rti_p (RTI 1) to the outer range table, so
-- setrefs rewrites the second subquery's tlist Vars to outer RTIs 2
-- and 3, while sources have inner heap_rtis 1 and 2 (shift of 1).
-- =====================================================================

SELECT
  (SELECT COUNT(*) FROM rti_p WHERE name = 'bob') AS c1,
  (SELECT COUNT(*) FROM rti_p p JOIN rti_o o ON p.id = o.id
   WHERE p.name @@@ 'bob OR alice' AND (o.id = 1 OR p.id = 2)) AS c2;

-- Parity: native PG
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT
  (SELECT COUNT(*) FROM rti_p WHERE name = 'bob') AS c1,
  (SELECT COUNT(*) FROM rti_p p JOIN rti_o o ON p.id = o.id
   WHERE p.name IN ('bob', 'alice') AND (o.id = 1 OR p.id = 2)) AS c2;
SET paradedb.enable_aggregate_custom_scan TO on;

-- =====================================================================
-- Section 3: RTI offset = 2
--
-- The first scalar subquery is itself a join (adds rti_p + rti_o = 2
-- RTI slots). The second join's inner RTIs (1, 2) become outer RTIs
-- (3, 4) — a shift of 2.
-- =====================================================================

SELECT
  (SELECT COUNT(*) FROM rti_p p JOIN rti_o o ON p.id = o.id WHERE p.name = 'bob') AS c1,
  (SELECT COUNT(*) FROM rti_p p JOIN rti_o o ON p.id = o.id
   WHERE p.name @@@ 'bob OR alice' AND (o.id = 1 OR p.id = 2)) AS c2;

-- Parity: native PG
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT
  (SELECT COUNT(*) FROM rti_p p JOIN rti_o o ON p.id = o.id WHERE p.name = 'bob') AS c1,
  (SELECT COUNT(*) FROM rti_p p JOIN rti_o o ON p.id = o.id
   WHERE p.name IN ('bob', 'alice') AND (o.id = 1 OR p.id = 2)) AS c2;
SET paradedb.enable_aggregate_custom_scan TO on;

-- =====================================================================
-- Section 4: Triple scalar subquery — deeper RTI offset
--
-- Two single-table subqueries precede the join subquery. Each adds one
-- RTI slot to the outer range table, so the join subquery's inner RTIs
-- (1, 2) become outer RTIs (3, 4) — a shift of 2.
-- =====================================================================

SELECT
  (SELECT COUNT(*) FROM rti_p WHERE name = 'bob') AS c1,
  (SELECT COUNT(*) FROM rti_o WHERE name = 'alice') AS c2,
  (SELECT COUNT(*) FROM rti_p p JOIN rti_o o ON p.id = o.id
   WHERE p.name @@@ 'bob OR alice' AND (o.id = 1 OR p.id = 2)) AS c3;

-- Parity: native PG
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT
  (SELECT COUNT(*) FROM rti_p WHERE name = 'bob') AS c1,
  (SELECT COUNT(*) FROM rti_o WHERE name = 'alice') AS c2,
  (SELECT COUNT(*) FROM rti_p p JOIN rti_o o ON p.id = o.id
   WHERE p.name IN ('bob', 'alice') AND (o.id = 1 OR p.id = 2)) AS c3;
SET paradedb.enable_aggregate_custom_scan TO on;

-- =====================================================================
-- Section 5: Text literal constants in cross-table predicates
--
-- Tests translate_const text handling (TEXTOID → ScalarValue::Utf8)
-- for text equality comparisons in cross-table OR predicates under
-- RTI offset. (o.name = 'alice' OR p.name = 'bob') compares text
-- constants from both sides of the join.
--
-- Expected: id=1 (p.name='bob': p.name='bob' ✓) +
--           id=2 (o.name='alice': o.name='alice' ✓) = 2
-- =====================================================================

SELECT
  (SELECT COUNT(*) FROM rti_p WHERE name = 'bob') AS c1,
  (SELECT COUNT(*) FROM rti_p p JOIN rti_o o ON p.id = o.id
   WHERE p.name @@@ 'bob OR alice'
   AND (o.name = 'alice' OR p.name = 'bob')) AS c2;

-- Parity: native PG
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT
  (SELECT COUNT(*) FROM rti_p WHERE name = 'bob') AS c1,
  (SELECT COUNT(*) FROM rti_p p JOIN rti_o o ON p.id = o.id
   WHERE p.name IN ('bob', 'alice')
   AND (o.name = 'alice' OR p.name = 'bob')) AS c2;
SET paradedb.enable_aggregate_custom_scan TO on;

-- =====================================================================
-- Section 6: GROUP BY survives RTI offset
--
-- Aggregate with GROUP BY in a nested subquery context. GROUP BY
-- references must also be correctly RTI-normalized.
-- =====================================================================

SELECT
  (SELECT COUNT(*) FROM rti_p WHERE name = 'bob') AS sentinel,
  (SELECT COUNT(*) FROM rti_p p JOIN rti_o o ON p.id = o.id
   WHERE p.name @@@ 'bob OR alice OR charlie') AS total;

-- Parity
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT
  (SELECT COUNT(*) FROM rti_p WHERE name = 'bob') AS sentinel,
  (SELECT COUNT(*) FROM rti_p p JOIN rti_o o ON p.id = o.id
   WHERE p.name IN ('bob', 'alice', 'charlie')) AS total;
SET paradedb.enable_aggregate_custom_scan TO on;

-- =====================================================================
-- Clean up rti_* tables
-- =====================================================================

DROP TABLE rti_o;
DROP TABLE rti_p;

-- =====================================================================
-- Section 7: Exact reproduction from issue #5266
--
-- Two scalar subqueries in the outer SELECT — one using plain SQL
-- equality, one using @@@ search predicates with cross-table OR
-- predicates. The @@@ variant triggers AggregateScan, and the outer
-- SELECT causes setrefs to assign non-trivial outer RTIs to the second
-- subquery's tables (RTI offset ≥ 1 depending on planner's range table).
--
-- Both subqueries must return the same count. With an empty table the
-- expected result is 0 | 0; the test primarily guards against the RTI
-- mismatch causing an error or wrong result in the @@@ variant.
-- =====================================================================

DROP TABLE IF EXISTS products;
DROP TABLE IF EXISTS orders;

CREATE TABLE products
(
    id   serial8 not null primary key,
    name text
);

CREATE TABLE orders
(
    id   serial8 not null primary key,
    name text
);

CREATE INDEX idxproducts ON products USING bm25 (id, name)
    WITH (
    key_field = 'id',
    text_fields = '{"name": {"tokenizer": {"type": "keyword"}}}'
    );
CREATE INDEX idxorders ON orders USING bm25 (id, name)
    WITH (
    key_field = 'id',
    text_fields = '{"name": {"tokenizer": {"type": "keyword"}}}'
    );

-- Empty tables: both subqueries should return 0 without error.
SELECT (SELECT COUNT(*)
        FROM products
                 JOIN orders ON products.name = orders.name
        WHERE (NOT (products.id = '3'))
           OR ((products.name = 'bob') AND (orders.id = '3'))),
       (SELECT COUNT(*)
        FROM products
                 JOIN orders ON products.name = orders.name
        WHERE (NOT (products.id @@@ '3'))
           OR ((products.name @@@ 'bob') AND (orders.id @@@ '3')));

-- Insert data and verify parity (non-zero counts).
INSERT INTO products (id, name) VALUES (1, 'alice'), (2, 'bob'), (3, 'charlie');
INSERT INTO orders   (id, name) VALUES (1, 'alice'), (2, 'bob'), (3, 'charlie');

-- Plain SQL baseline:
-- JOIN on name equality gives 3 pairs: (1,1),(2,2),(3,3).
-- Filter: NOT(id=3) OR (name='bob' AND orders.id=3)
--   (1,1): NOT(1=3) = T                   → included
--   (2,2): NOT(2=3) = T                   → included
--   (3,3): NOT(3=3) = F, (bob AND 3=3)=F  → excluded
-- Expected: 2
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT COUNT(*)
FROM products
         JOIN orders ON products.name = orders.name
WHERE (NOT (products.id = '3'))
   OR ((products.name = 'bob') AND (orders.id = '3'));
SET paradedb.enable_aggregate_custom_scan TO on;

-- @@@ variant — must match the plain SQL result.
-- This is the exact query from issue #5266.
SELECT (SELECT COUNT(*)
        FROM products
                 JOIN orders ON products.name = orders.name
        WHERE (NOT (products.id = '3'))
           OR ((products.name = 'bob') AND (orders.id = '3'))),
       (SELECT COUNT(*)
        FROM products
                 JOIN orders ON products.name = orders.name
        WHERE (NOT (products.id @@@ '3'))
           OR ((products.name @@@ 'bob') AND (orders.id @@@ '3')));

CREATE OR REPLACE FUNCTION explain_sanitize_oid(query text) RETURNS SETOF text AS $$
DECLARE
    plan_line text;
BEGIN
    FOR plan_line IN EXECUTE 'EXPLAIN (COSTS OFF) ' || query LOOP
        -- Replace volatile "oid":123456 with a static "oid":000000 placeholder
        RETURN NEXT regexp_replace(plan_line, '"oid":\d+', '"oid":000000', 'g');
    END LOOP;
END;
$$ LANGUAGE plpgsql;

-- Check if custom AggregateScan is being used.
SELECT explain_sanitize_oid($$
SELECT (SELECT COUNT(*)
        FROM products
                 JOIN orders ON products.name = orders.name
        WHERE (NOT (products.id = '3'))
           OR ((products.name = 'bob') AND (orders.id = '3'))),
       (SELECT COUNT(*)
        FROM products
                 JOIN orders ON products.name = orders.name
        WHERE (NOT (products.id @@@ '3'))
           OR ((products.name @@@ 'bob') AND (orders.id @@@ '3')));
$$);

DROP TABLE orders;
DROP TABLE products;

-- =====================================================================
-- Section 8: Self-join — same relation on both sides
--
-- build_tlist_col_map must distinguish the two instances of the same
-- relation by RTI, not just relation OID: both sources share a heaprelid
-- but have distinct execution aliases. An OID-only lookup always picks
-- plan position 0, silently resolving b.id to a's column.
--
-- Data is asymmetric so a misresolved mapping changes the count:
--   rti_s: (1,'red'), (2,'red'), (3,'red'), (4,'blue')
--   self-join on name → 9 'red' pairs + 1 'blue' pair
--   (b.id = 1 OR a.id = 2): {(1,1),(2,1),(3,1)} ∪ {(2,1),(2,2),(2,3)} = 5
--   misresolved (a.id = 1 OR a.id = 2): 3 + 3 = 6
--
-- The join is deliberately on name while the OR predicate is on id;
-- joining on id would force a.id = b.id and mask the misresolution.
-- =====================================================================

CREATE TABLE rti_s (id INT PRIMARY KEY, name TEXT NOT NULL);
INSERT INTO rti_s VALUES (1, 'red'), (2, 'red'), (3, 'red'), (4, 'blue');

CREATE INDEX rti_s_idx ON rti_s
USING bm25 (id, name)
WITH (key_field='id', text_fields='{"name": {"fast": true}}');

SELECT COUNT(*) FROM rti_s a JOIN rti_s b ON a.name = b.name
WHERE a.name @@@ 'red OR blue' AND (b.id = 1 OR a.id = 2);

-- Parity: native PG
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT COUNT(*) FROM rti_s a JOIN rti_s b ON a.name = b.name
WHERE a.name IN ('red', 'blue') AND (b.id = 1 OR a.id = 2);
SET paradedb.enable_aggregate_custom_scan TO on;

-- Self-join nested behind a preceding scalar subquery: combines the RTI
-- shift (setrefs renumbering) with the same-OID source ambiguity.
SELECT
  (SELECT COUNT(*) FROM rti_s WHERE name = 'blue') AS sentinel,
  (SELECT COUNT(*) FROM rti_s a JOIN rti_s b ON a.name = b.name
   WHERE a.name @@@ 'red OR blue' AND (b.id = 1 OR a.id = 2)) AS self_join;

DROP TABLE rti_s;
