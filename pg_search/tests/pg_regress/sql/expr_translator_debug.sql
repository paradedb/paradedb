-- Regression test for `PredicateTranslator` debug1 logging.
--
-- Every failed translation path emits a structured `pgrx::debug1!` line of
-- the form:
--
--     DEBUG: PredicateTranslator: <reason> [<NodeTag>] <key=value pairs> | <deparsed SQL>
--
-- This test raises `client_min_messages` to `debug1` and runs EXPLAIN on
-- queries crafted to trip specific translator rejection points, verifying
-- the expected log lines appear in order.

CREATE EXTENSION IF NOT EXISTS pg_search;

DROP TABLE IF EXISTS et_items CASCADE;
DROP TABLE IF EXISTS et_exclusions CASCADE;

CREATE TABLE et_items (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    value INT NOT NULL
);
CREATE TABLE et_exclusions (
    id SERIAL PRIMARY KEY,
    pattern TEXT
);
INSERT INTO et_items (name, value) VALUES
    ('alpha', 10), ('beta', 20);
INSERT INTO et_exclusions (pattern) VALUES
    ('alpha'), ('beta');

CREATE INDEX et_items_idx ON et_items
    USING bm25 (id, name, value)
    WITH (
        key_field = 'id',
        text_fields = '{"name":{"fast":true}}',
        numeric_fields = '{"value":{"fast":true}}'
    );
CREATE INDEX et_exclusions_idx ON et_exclusions
    USING bm25 (id, pattern)
    WITH (
        key_field = 'id',
        text_fields = '{"pattern":{"fast":true}}'
    );

SET paradedb.enable_join_custom_scan = on;
SET client_min_messages = 'debug1';

-- =====================================================================
-- 1. Unsupported operator (textcat `||`) → [OpExpr] log
-- =====================================================================
-- `translate_op_expr` doesn't know the `||` operator. It logs
--   DEBUG: PredicateTranslator: unsupported operator [OpExpr] op="||", types=(text, text) | ...
-- Also emits
--   DEBUG: PredicateTranslator: unsupported const type [Const] type=text
-- for the `'_x'` string literal that `translate_const` doesn't support.
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT i.id FROM et_items i
WHERE NOT EXISTS (
    SELECT 1 FROM et_exclusions e
    WHERE e.id @@@ paradedb.all()
      AND e.pattern || '_x' = i.name
)
AND i.id @@@ paradedb.all()
LIMIT 5;

-- =====================================================================
-- 2. Unknown function → [FuncExpr] no native mapping
-- =====================================================================
-- `md5` isn't in `translate_known_func`'s pg_catalog map. Expect:
--   DEBUG: PredicateTranslator: no native mapping [FuncExpr] func=pg_catalog.md5, arity=1 | ...
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT i.id FROM et_items i
WHERE NOT EXISTS (
    SELECT 1 FROM et_exclusions e
    WHERE e.id @@@ paradedb.all()
      AND md5(e.pattern) = i.name
)
AND i.id @@@ paradedb.all()
LIMIT 5;

-- =====================================================================
-- 3. Cross-type cast → [CoerceViaIO] rejected + deparsed SQL in log
-- =====================================================================
-- `CAST(i.value AS text)` is a CoerceViaIO from int4 → text. The
-- translator only allows coercions inside the text family (TEXT ↔
-- VARCHAR ↔ NAME); int → text is rejected. Because the inner node is
-- a simple Var reference, `deparse_expression` succeeds here and the
-- log line includes the actual SQL (e.g. `(i.value)::text`) after the
-- `|` separator — unlike the first two cases whose cross-RTE shape
-- forces the helper to fall back to the node-tag label.
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT i.id FROM et_items i
WHERE NOT EXISTS (
    SELECT 1 FROM et_exclusions e
    WHERE e.id @@@ paradedb.all()
      AND CAST(i.value AS text) = e.pattern
)
AND i.id @@@ paradedb.all()
LIMIT 5;

-- Cleanup
RESET client_min_messages;
RESET paradedb.enable_join_custom_scan;
DROP TABLE et_items CASCADE;
DROP TABLE et_exclusions CASCADE;
