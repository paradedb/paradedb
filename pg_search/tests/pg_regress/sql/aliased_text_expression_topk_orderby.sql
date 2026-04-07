\i common/common_setup.sql

CALL paradedb.create_bm25_test_table(
  schema_name => 'public',
  table_name => 'mock_items'
);

CREATE INDEX search_idx ON mock_items
USING bm25 (
  id,
  (lower(description)::pdb.literal),
  rating
)
WITH (key_field='id');

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT description, rating
FROM mock_items
WHERE description ||| 'sleek running shoes'
ORDER BY lower(description)
LIMIT 5;

SELECT description, rating
FROM mock_items
WHERE description ||| 'sleek running shoes'
ORDER BY lower(description)
LIMIT 5;

DROP INDEX search_idx;

CREATE INDEX search_idx ON mock_items
USING bm25 (
  id,
  (lower(description)::pdb.literal('alias=literal_description')),
  rating
)
WITH (key_field='id');

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT description, rating
FROM mock_items
WHERE description ||| 'sleek running shoes'
ORDER BY lower(description)
LIMIT 5;

SELECT description, rating
FROM mock_items
WHERE description ||| 'sleek running shoes'
ORDER BY lower(description)
LIMIT 5;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT description, rating
FROM mock_items
WHERE description ||| 'sleek running shoes'
ORDER BY upper(description)
LIMIT 5;

SELECT description, rating
FROM mock_items
WHERE description ||| 'sleek running shoes'
ORDER BY upper(description)
LIMIT 5;

DROP INDEX search_idx;

CREATE INDEX search_idx ON mock_items
USING bm25 (
  id,
  (description::pdb.simple('alias=simple_description')),
  (lower(description)::pdb.literal('alias=literal_description')),
  rating
)
WITH (key_field='id');

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT description, rating
FROM mock_items
WHERE description::pdb.alias('literal_description') ||| 'sleek running shoes'
ORDER BY lower(description)
LIMIT 5;

DROP TABLE mock_items;
