-- Regression for issue #5858. Pushing a comparison into the index replaces
-- PostgreSQL's collation-aware semantics with Tantivy's byte semantics, so a
-- pushdown is only sound when the operator's input collation permits it.
--
--   nondeterministic collation -> equality can equate distinct byte strings
--   collation that is not byte-ordered -> ranges sort differently
--
-- Each case below asserts the plan as well as the rows, because a result-only
-- assertion would also pass if pushdown were disabled everywhere.
--
-- ASSUMPTION: the database's default collation is byte-ordered, as it is in CI
-- (C.UTF-8). Section 5 exercises the default collation directly, so it fails on
-- a dev box whose initdb locale is not byte-ordered, e.g. en_US.UTF-8. The other
-- sections name their collation explicitly and do not depend on the default.

\i common/common_setup.sql

SET paradedb.planner_warnings = 'off';

CREATE COLLATION issue_5858_ci (
    provider = icu,
    locale = 'und-u-ks-level2',
    deterministic = false
);

CREATE COLLATION issue_5858_icu (
    provider = icu,
    locale = 'en-US'
);

DROP TABLE IF EXISTS issue_5858_products CASCADE;

CREATE TABLE issue_5858_products (
    id bigint PRIMARY KEY,
    -- nondeterministic: 'Electronics' and 'electronics' compare equal
    name_ci text COLLATE issue_5858_ci,
    -- deterministic but not byte-ordered: apple < Banana < cherry
    name_icu text COLLATE issue_5858_icu,
    -- byte-ordered: Banana < apple < cherry
    name_c text COLLATE "C",
    -- no COLLATE, so the operator carries DEFAULT_COLLATION_OID
    name_default text
);

INSERT INTO issue_5858_products VALUES
    (1, 'Electronics', 'apple', 'apple', 'apple'),
    (2, 'electronics', 'Banana', 'Banana', 'Banana'),
    (3, 'Books', 'cherry', 'cherry', 'cherry');

CREATE INDEX issue_5858_products_idx
ON issue_5858_products
USING paradedb (id, name_ci, name_icu, name_c, name_default)
WITH (
    key_field = 'id',
    text_fields = '{"name_ci": {"tokenizer": {"type": "keyword"}}, "name_icu": {"tokenizer": {"type": "keyword"}}, "name_c": {"tokenizer": {"type": "keyword"}}, "name_default": {"tokenizer": {"type": "keyword"}}}'
);

ANALYZE issue_5858_products;

\echo '=== SECTION 1: nondeterministic collation, equality ==='

\echo 'Test 1.1: = is a heap filter, not a Tantivy term'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM issue_5858_products
WHERE id @@@ paradedb.all()
  AND name_ci = 'Electronics'
ORDER BY id;

\echo 'Test 1.2: rows match native PostgreSQL, which equates the two spellings'
SELECT id FROM issue_5858_products
WHERE name_ci = 'Electronics'
ORDER BY id;

SELECT id FROM issue_5858_products
WHERE id @@@ paradedb.all()
  AND name_ci = 'Electronics'
ORDER BY id;

\echo 'Test 1.3: <> is a heap filter too'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM issue_5858_products
WHERE id @@@ paradedb.all()
  AND name_ci <> 'Electronics'
ORDER BY id;

SELECT id FROM issue_5858_products
WHERE name_ci <> 'Electronics'
ORDER BY id;

SELECT id FROM issue_5858_products
WHERE id @@@ paradedb.all()
  AND name_ci <> 'Electronics'
ORDER BY id;

\echo '=== SECTION 2: nondeterministic collation, IN (...) ==='

\echo 'Test 2.1: IN (...) is a heap filter, not a Tantivy term_set'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM issue_5858_products
WHERE id @@@ paradedb.all()
  AND name_ci IN ('Electronics', 'Books')
ORDER BY id;

\echo 'Test 2.2: rows match native PostgreSQL'
SELECT id FROM issue_5858_products
WHERE name_ci IN ('Electronics', 'Books')
ORDER BY id;

SELECT id FROM issue_5858_products
WHERE id @@@ paradedb.all()
  AND name_ci IN ('Electronics', 'Books')
ORDER BY id;

\echo '=== SECTION 3: deterministic ICU collation ==='

\echo 'Test 3.1: equality still pushes down, deterministic equality is byte equality'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM issue_5858_products
WHERE id @@@ paradedb.all()
  AND name_icu = 'Banana'
ORDER BY id;

SELECT id FROM issue_5858_products
WHERE name_icu = 'Banana'
ORDER BY id;

SELECT id FROM issue_5858_products
WHERE id @@@ paradedb.all()
  AND name_icu = 'Banana'
ORDER BY id;

\echo 'Test 3.2: IN (...) still pushes down as a term_set'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM issue_5858_products
WHERE id @@@ paradedb.all()
  AND name_icu IN ('apple', 'cherry')
ORDER BY id;

SELECT id FROM issue_5858_products
WHERE id @@@ paradedb.all()
  AND name_icu IN ('apple', 'cherry')
ORDER BY id;

\echo 'Test 3.3: a range is a heap filter, en-US is not byte-ordered'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM issue_5858_products
WHERE id @@@ paradedb.all()
  AND name_icu > 'B'
ORDER BY id;

\echo 'Test 3.4: en-US orders apple before B, byte order does not'
SELECT id, name_icu FROM issue_5858_products
WHERE name_icu > 'B'
ORDER BY id;

SELECT id, name_icu FROM issue_5858_products
WHERE id @@@ paradedb.all()
  AND name_icu > 'B'
ORDER BY id;

\echo '=== SECTION 4: C collation ==='

\echo 'Test 4.1: a range still pushes down, C is byte-ordered'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM issue_5858_products
WHERE id @@@ paradedb.all()
  AND name_c > 'B'
ORDER BY id;

\echo 'Test 4.2: the pushed range returns the same rows PostgreSQL would'
SELECT id, name_c FROM issue_5858_products
WHERE name_c > 'B'
ORDER BY id;

SELECT id, name_c FROM issue_5858_products
WHERE id @@@ paradedb.all()
  AND name_c > 'B'
ORDER BY id;

\echo 'Test 4.3: COLLATE "C" on the ICU column restores range pushdown'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM issue_5858_products
WHERE id @@@ paradedb.all()
  AND name_icu COLLATE "C" > 'B'
ORDER BY id;

\echo 'Test 4.4: the same column and predicate, three rows under C against two under en-US'
SELECT id, name_icu FROM issue_5858_products
WHERE name_icu COLLATE "C" > 'B'
ORDER BY id;

SELECT id, name_icu FROM issue_5858_products
WHERE id @@@ paradedb.all()
  AND name_icu COLLATE "C" > 'B'
ORDER BY id;

\echo '=== SECTION 5: default collation ==='

\echo 'Test 5.1: equality pushes down under the default collation'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM issue_5858_products
WHERE id @@@ paradedb.all()
  AND name_default = 'Banana'
ORDER BY id;

SELECT id FROM issue_5858_products
WHERE id @@@ paradedb.all()
  AND name_default = 'Banana'
ORDER BY id;

\echo 'Test 5.2: a range pushes down too, the default is byte-ordered here'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM issue_5858_products
WHERE id @@@ paradedb.all()
  AND name_default > 'B'
ORDER BY id;

SELECT id, name_default FROM issue_5858_products
WHERE name_default > 'B'
ORDER BY id;

SELECT id, name_default FROM issue_5858_products
WHERE id @@@ paradedb.all()
  AND name_default > 'B'
ORDER BY id;

\echo '=== SECTION 6: non-collatable types are unaffected ==='

\echo 'Test 6.1: an integer range pushes down, its collation is InvalidOid'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM issue_5858_products
WHERE id @@@ paradedb.all()
  AND id > 1
ORDER BY id;

SELECT id FROM issue_5858_products
WHERE id @@@ paradedb.all()
  AND id > 1
ORDER BY id;

\echo '=== SECTION 7: an aggregate FILTER that cannot be extracted declines the scan ==='

-- With filter pushdown off, a collation-declined comparison inside FILTER has
-- no route into the scan: extraction fails, and the aggregate scan must step
-- aside rather than treat the FILTER as absent and count every row.
SET paradedb.enable_aggregate_custom_scan = on;
SET paradedb.enable_filter_pushdown = off;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT count(*) FILTER (WHERE name_icu > 'B') AS range_count,
       count(*) FILTER (WHERE name_ci = 'Books') AS equality_count
FROM issue_5858_products
WHERE id @@@ paradedb.all();

SELECT count(*) FILTER (WHERE name_icu > 'B') AS range_count,
       count(*) FILTER (WHERE name_ci = 'Books') AS equality_count
FROM issue_5858_products
WHERE id @@@ paradedb.all();

RESET paradedb.enable_filter_pushdown;
RESET paradedb.enable_aggregate_custom_scan;

DROP TABLE issue_5858_products;
DROP COLLATION issue_5858_icu;
DROP COLLATION issue_5858_ci;
