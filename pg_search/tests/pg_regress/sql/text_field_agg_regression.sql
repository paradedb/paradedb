-- =====================================================================
-- Regression Test: Text Field Aggregation Bug
-- Issue: "unexpected type Str. This should not happen"
-- Fixed in tantivy commit 65b5a1a3 (ParadeDB v0.21.3+)
--
-- The bug occurred when metric aggregations (value_count, count, etc.)
-- were used on TEXT fields as sub-aggregations inside bucket aggregations
-- (histogram, range, terms with high cardinality).
-- =====================================================================

CREATE EXTENSION IF NOT EXISTS pg_search;
SET paradedb.enable_aggregate_custom_scan TO on;

-- =====================================================================
-- SETUP: High cardinality text field (500 unique values)
-- This triggers the HashMap path in tantivy's terms aggregation
-- =====================================================================

DROP TABLE IF EXISTS test_text_agg CASCADE;

CREATE TABLE test_text_agg (
    id SERIAL PRIMARY KEY,
    name TEXT,
    score INTEGER
);

-- Insert 500 unique names to trigger high-cardinality code path
INSERT INTO test_text_agg (name, score)
SELECT 'language_' || i::text, (i % 100)
FROM generate_series(1, 500) AS i;

CREATE INDEX test_text_agg_idx ON test_text_agg
USING bm25 (id, name, score)
WITH (
    key_field = 'id',
    text_fields = '{"name": {"fast": true, "tokenizer": {"type": "default"}}}',
    numeric_fields = '{"score": {}}'
);

-- =====================================================================
-- TEST 1: GROUP BY on text field + ORDER BY count()
-- This was failing in v0.21.2 with "unexpected type Str"
-- =====================================================================

SELECT name AS value
FROM test_text_agg
WHERE id @@@ paradedb.all()
GROUP BY name
ORDER BY count(name) DESC, name DESC
LIMIT 5;

-- =====================================================================
-- TEST 2: pdb.agg value_count on text field with GROUP BY
-- This was failing in v0.21.2 with "unexpected type Str"
-- =====================================================================

SELECT
    name AS value,
    pdb.agg('{"value_count": {"field": "name"}}') AS count
FROM test_text_agg
WHERE id @@@ paradedb.all()
GROUP BY name
ORDER BY 2 DESC, name DESC
LIMIT 5;

-- =====================================================================
-- TEST 3: Histogram + value_count sub-aggregation on text field
-- This was failing in v0.21.2 with "unexpected type Str"
-- =====================================================================

SELECT pdb.agg('{
    "histogram": {"field": "score", "interval": 25},
    "aggs": {
        "name_count": {"value_count": {"field": "name"}}
    }
}')
FROM test_text_agg
WHERE id @@@ paradedb.all();

-- =====================================================================
-- TEST 4: Range + value_count sub-aggregation on text field
-- This was failing in v0.21.2 with "unexpected type Str"
-- =====================================================================

SELECT pdb.agg('{
    "range": {"field": "score", "ranges": [{"to": 50}, {"from": 50}]},
    "aggs": {
        "name_count": {"value_count": {"field": "name"}}
    }
}')
FROM test_text_agg
WHERE id @@@ paradedb.all();

-- =====================================================================
-- TEST 5: Simple value_count on text field (top-level, no bucket parent)
-- This worked in v0.21.2 and should continue to work
-- =====================================================================

SELECT pdb.agg('{"value_count": {"field": "name"}}')
FROM test_text_agg
WHERE id @@@ paradedb.all();

-- =====================================================================
-- Cleanup
-- =====================================================================

DROP TABLE IF EXISTS test_text_agg CASCADE;
RESET paradedb.enable_aggregate_custom_scan;
