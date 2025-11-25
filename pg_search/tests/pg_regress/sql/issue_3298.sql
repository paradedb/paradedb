\i common/common_setup.sql

SET paradedb.enable_aggregate_custom_scan TO on;

CALL paradedb.create_bm25_test_table(
  schema_name => 'public',
  table_name => 'mock_items'
);

CREATE INDEX search_idx ON mock_items USING bm25 (id, description, rating, (category::pdb.literal), metadata) WITH (key_field='id');

CREATE TABLE allowed_categories (
    category TEXT PRIMARY KEY
);

INSERT INTO allowed_categories (category) VALUES
    ('Electronics'),
    ('Clothing');

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT COUNT(*) FROM mock_items WHERE category @@@ paradedb.term_set(terms => ARRAY(SELECT paradedb.term('category', category) FROM allowed_categories LIMIT 5));
SELECT COUNT(*) FROM mock_items WHERE category @@@ paradedb.term_set(terms => ARRAY(SELECT paradedb.term('category', category) FROM allowed_categories LIMIT 5));

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT
  COUNT(*) AS total,
  COUNT(*) FILTER (WHERE category @@@ paradedb.term_set(terms => ARRAY(SELECT paradedb.term('category', category) FROM allowed_categories LIMIT 2))),
  COUNT(*) FILTER (WHERE category @@@ paradedb.term_set(terms => ARRAY(SELECT paradedb.term('category', category) FROM allowed_categories LIMIT 2 OFFSET 2)))
FROM mock_items;
SELECT
  COUNT(*) AS total,
  COUNT(*) FILTER (WHERE category @@@ paradedb.term_set(terms => ARRAY(SELECT paradedb.term('category', category) FROM allowed_categories LIMIT 2))),
  COUNT(*) FILTER (WHERE category @@@ paradedb.term_set(terms => ARRAY(SELECT paradedb.term('category', category) FROM allowed_categories LIMIT 2 OFFSET 2)))
FROM mock_items;

-- Make sure the results are correct
SET paradedb.enable_aggregate_custom_scan TO off;

SELECT COUNT(*) FROM mock_items WHERE category @@@ paradedb.term_set(terms => ARRAY(SELECT paradedb.term('category', category) FROM allowed_categories LIMIT 5));
SELECT
  COUNT(*) AS total,
  COUNT(*) FILTER (WHERE category @@@ paradedb.term_set(terms => ARRAY(SELECT paradedb.term('category', category) FROM allowed_categories LIMIT 2))),
  COUNT(*) FILTER (WHERE category @@@ paradedb.term_set(terms => ARRAY(SELECT paradedb.term('category', category) FROM allowed_categories LIMIT 2 OFFSET 2)))
FROM mock_items;

DROP TABLE allowed_categories;
DROP TABLE mock_items;
