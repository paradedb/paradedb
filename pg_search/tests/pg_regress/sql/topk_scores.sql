\i common/common_setup.sql

CALL paradedb.create_bm25_test_table(
  schema_name => 'public',
  table_name => 'mock_items'
);

CREATE INDEX search_idx on mock_items
USING bm25 (id, description, rating, category, metadata)
WITH (key_field='id', text_fields = '{"category": {"tokenizer": {"type": "keyword"}, "fast": true}}', json_fields = '{"metadata": {"fast": true, "tokenizer": {"type": "raw", "lowercase": true}}}');

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, description, rating, pdb.score(id) FROM mock_items
WHERE description @@@ 'keyboard' OR description @@@ 'shoes' AND rating > 2
ORDER BY rating DESC
LIMIT 5;

SELECT id, description, rating, pdb.score(id) FROM mock_items
WHERE description @@@ 'keyboard' OR description @@@ 'shoes' AND rating > 2
ORDER BY rating DESC
LIMIT 5;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, description, rating, pdb.score(id) FROM mock_items
WHERE description @@@ 'keyboard' OR description @@@ 'shoes' AND rating > 2
ORDER BY rating, id ASC
LIMIT 5;

SELECT id, description, rating, pdb.score(id) FROM mock_items
WHERE description @@@ 'keyboard' OR description @@@ 'shoes' AND rating > 2
ORDER BY rating, id ASC
LIMIT 5;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, description, rating, pdb.score(id) FROM mock_items
WHERE description @@@ 'keyboard' OR description @@@ 'shoes' AND rating > 2
ORDER BY rating, id ASC, category
LIMIT 5;

SELECT id, description, rating, pdb.score(id) FROM mock_items
WHERE description @@@ 'keyboard' OR description @@@ 'shoes' AND rating > 2
ORDER BY rating, id ASC, category
LIMIT 5;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, description, rating, pdb.score(id) FROM mock_items
WHERE description @@@ 'keyboard' OR description @@@ 'shoes' AND rating > 2
ORDER BY rating, pdb.score(id) DESC
LIMIT 5;

SELECT id, description, rating, pdb.score(id) FROM mock_items
WHERE description @@@ 'keyboard' OR description @@@ 'shoes' AND rating > 2
ORDER BY rating, pdb.score(id) DESC
LIMIT 5;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, description, rating, pdb.score(id) FROM mock_items
WHERE description @@@ 'keyboard' OR description @@@ 'shoes' AND rating > 2
ORDER BY rating, pdb.score(id), id DESC
LIMIT 5;

SELECT id, description, rating, pdb.score(id) FROM mock_items
WHERE description @@@ 'keyboard' OR description @@@ 'shoes' AND rating > 2
ORDER BY rating, pdb.score(id), id DESC
LIMIT 5;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, description, rating, pdb.score(id) FROM mock_items
WHERE description @@@ 'keyboard' OR description @@@ 'shoes' AND rating > 2
ORDER BY rating, pdb.score(id), id, category DESC
LIMIT 5;

SELECT id, description, rating, pdb.score(id) FROM mock_items
WHERE description @@@ 'keyboard' OR description @@@ 'shoes' AND rating > 2
ORDER BY rating, pdb.score(id), id, category DESC
LIMIT 5;

DROP TABLE mock_items;