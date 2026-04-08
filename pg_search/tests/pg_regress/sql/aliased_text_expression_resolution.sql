\i common/common_setup.sql

CALL paradedb.create_bm25_test_table(
  schema_name => 'public',
  table_name => 'mock_items'
);

CREATE INDEX search_idx ON mock_items
USING bm25 (
  id,
  (lower(description)::pdb.literal('alias=literal_description')),
  rating
)
WITH (key_field='id');

SELECT description, rating
FROM mock_items
WHERE description ||| 'sleek running shoes';

DROP INDEX search_idx;

CREATE INDEX search_idx ON mock_items
USING bm25 (
  id,
  description,
  (description::pdb.simple('alias=simple_description')),
  (lower(description)::pdb.literal('alias=literal_description')),
  rating
)
WITH (key_field='id');

-- direct indexed column should take precedence over aliased expression matches
SELECT description, rating
FROM mock_items
WHERE description ||| 'sleek running shoes';

DROP INDEX search_idx;

-- A tokenized column without an alias should be selected over the aliased version
CREATE INDEX search_idx ON mock_items
USING bm25 (
  id,
  (description::pdb.simple),
  (description::pdb.literal('alias=literal_description')),
  rating
)
WITH (key_field='id');

-- description is not ambiguous here
SELECT description, rating
FROM mock_items
WHERE description ||| 'sleek running shoes';

DROP INDEX search_idx;

CREATE INDEX search_idx ON mock_items
USING bm25 (
  id,
  (description::pdb.simple('alias=simple_description')),
  (lower(description)::pdb.literal('alias=literal_description')),
  rating
)
WITH (key_field='id');

-- description is ambiguous here
SELECT description, rating
FROM mock_items
WHERE description ||| 'sleek running shoes';

SELECT description, rating
FROM mock_items
WHERE description::pdb.alias('literal_description') ||| 'sleek running shoes';

SELECT description, rating
FROM mock_items
WHERE description::pdb.alias('simple_description') ||| 'sleek running shoes';

DROP INDEX search_idx;


-- Composite index variations
CREATE TYPE aliased_description_fields AS (
  simple_description pdb.simple('alias=simple_description'),
  literal_description pdb.literal('alias=literal_description')
);


CREATE INDEX search_idx ON mock_items
USING bm25 (
  id,
  (
    ROW(
      lower(description)::pdb.simple('alias=simple_description'),
      lower(description)::pdb.literal('alias=literal_description')
    )::aliased_description_fields
  ),
  rating
)
WITH (key_field='id');

-- description is ambiguous here
SELECT description, rating
FROM mock_items
WHERE description ||| 'sleek running shoes';

SELECT description, rating
FROM mock_items
WHERE description::pdb.alias('literal_description') ||| 'sleek running shoes';

SELECT description, rating
FROM mock_items
WHERE description::pdb.alias('simple_description') ||| 'sleek running shoes';

DROP TYPE aliased_description_fields;

-- A tokenized column without an alias should be selected over the aliased version
CREATE TYPE partially_aliased_description_fields AS (
  description pdb.simple,
  literal_description pdb.literal('alias=literal_description')
);

CREATE INDEX search_idx ON mock_items
USING bm25 (
  id,
  (
    ROW(
      description::pdb.simple,
      description::pdb.literal('alias=literal_description')
    )::aliased_description_fields
  ),
  rating
)
WITH (key_field='id');

-- description is not ambiguous here
SELECT description, rating
FROM mock_items
WHERE description ||| 'sleek running shoes';

DROP TABLE mock_items;
DROP TYPE partially_aliased_description_fields;
