\i common/common_setup.sql

CALL paradedb.create_paradedb_test_table(
  schema_name => 'public',
  table_name => 'mock_items'
);

CREATE INDEX search_idx on mock_items
USING paradedb (id, description, rating, (category::pdb.literal), (metadata::pdb.literal_normalized('lowercase=true')));

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, description, rating, pdb.score(id) FROM mock_items
WHERE description ||| 'keyboard' OR description ||| 'shoes' AND rating > 2
ORDER BY rating DESC
LIMIT 5;

SELECT id, description, rating, pdb.score(id) FROM mock_items
WHERE description ||| 'keyboard' OR description ||| 'shoes' AND rating > 2
ORDER BY rating DESC
LIMIT 5;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, description, rating, pdb.score(id) FROM mock_items
WHERE description ||| 'keyboard' OR description ||| 'shoes' AND rating > 2
ORDER BY rating, id ASC
LIMIT 5;

SELECT id, description, rating, pdb.score(id) FROM mock_items
WHERE description ||| 'keyboard' OR description ||| 'shoes' AND rating > 2
ORDER BY rating, id ASC
LIMIT 5;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, description, rating, pdb.score(id) FROM mock_items
WHERE description ||| 'keyboard' OR description ||| 'shoes' AND rating > 2
ORDER BY rating, id ASC, category
LIMIT 5;

SELECT id, description, rating, pdb.score(id) FROM mock_items
WHERE description ||| 'keyboard' OR description ||| 'shoes' AND rating > 2
ORDER BY rating, id ASC, category
LIMIT 5;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, description, rating, pdb.score(id) FROM mock_items
WHERE description ||| 'keyboard' OR description ||| 'shoes' AND rating > 2
ORDER BY rating, pdb.score(id) DESC
LIMIT 5;

SELECT id, description, rating, pdb.score(id) FROM mock_items
WHERE description ||| 'keyboard' OR description ||| 'shoes' AND rating > 2
ORDER BY rating, pdb.score(id) DESC
LIMIT 5;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, description, rating, pdb.score(id) FROM mock_items
WHERE description ||| 'keyboard' OR description ||| 'shoes' AND rating > 2
ORDER BY rating, pdb.score(id), id DESC
LIMIT 5;

SELECT id, description, rating, pdb.score(id) FROM mock_items
WHERE description ||| 'keyboard' OR description ||| 'shoes' AND rating > 2
ORDER BY rating, pdb.score(id), id DESC
LIMIT 5;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, description, rating, pdb.score(id) FROM mock_items
WHERE description ||| 'keyboard' OR description ||| 'shoes' AND rating > 2
ORDER BY rating, pdb.score(id), id, category DESC
LIMIT 5;

SELECT id, description, rating, pdb.score(id) FROM mock_items
WHERE description ||| 'keyboard' OR description ||| 'shoes' AND rating > 2
ORDER BY rating, pdb.score(id), id, category DESC
LIMIT 5;

DROP TABLE mock_items;
