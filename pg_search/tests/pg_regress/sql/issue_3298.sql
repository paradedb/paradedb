\i common/common_setup.sql

SET paradedb.enable_aggregate_custom_scan TO on;

CALL paradedb.create_bm25_test_table(
  schema_name => 'public',
  table_name => 'mock_items'
);

CREATE INDEX search_idx ON mock_items USING bm25 (id, description, rating, (category::pdb.exact), metadata) WITH (key_field='id');

CREATE TABLE allowed_categories (
    category TEXT PRIMARY KEY
);

INSERT INTO allowed_categories (category) VALUES
    ('Electronics'),
    ('Clothing');

EXPLAIN SELECT COUNT(*) FROM mock_items WHERE category @@@ paradedb.term_set(terms => ARRAY(SELECT paradedb.term('category', category) FROM allowed_categories LIMIT 5));
SELECT COUNT(*) FROM mock_items WHERE category @@@ paradedb.term_set(terms => ARRAY(SELECT paradedb.term('category', category) FROM allowed_categories LIMIT 5));

DROP TABLE allowed_categories;
DROP TABLE mock_items;
