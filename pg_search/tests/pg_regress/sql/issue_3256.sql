\i common/common_setup.sql

CALL paradedb.create_bm25_test_table(
  schema_name => 'public',
  table_name => 'mock_items'
);

CREATE INDEX on mock_items USING bm25 (id, description, rating, category, metadata) WITH (key_field='id');
SELECT
    paradedb.snippet(description, start_tag => '<b>', end_tag => '</b>', max_num_chars => 10),
    paradedb.snippet(description, start_tag => '<i>', end_tag => '</i>'),
    paradedb.snippet_positions(description)
FROM mock_items WHERE description @@@ 'shoes';

DROP TABLE mock_items;
