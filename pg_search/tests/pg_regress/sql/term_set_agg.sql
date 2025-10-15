\i common/common_setup.sql

CREATE TABLE genus (
  id BIGINT NOT NULL,
  name TEXT NOT NULL
);

CREATE TABLE plants (
  id BIGINT NOT NULL GENERATED ALWAYS AS IDENTITY,
  genus_id BIGINT NOT NULL,
  name TEXT NOT NULL
);

INSERT INTO genus (id, name) VALUES
(0, 'oak'),
(1, 'maple'),
(2, 'pine'),
(3, 'apple');

INSERT INTO plants (genus_id, name) VALUES
(0, 'English Oak'),
(0, 'Holly Oak'),
(0, 'White Oak'),
(1, 'Sugar Maple'),
(1, 'Red Maple'),
(1, 'Norway Maple'),
(2, 'Scots Pine'),
(2, 'Ponderosa Pine'),
(3, 'Domestic Apple'),
(3, 'Siberian Crabapple');

CREATE INDEX plants_idx ON plants
USING bm25 (id, genus_id, name)
WITH (key_field = id);

CREATE INDEX genus_idx ON genus
USING bm25 (id, name)
WITH (key_field = id);

--
-- Test 1: Basic CTE query
-- Find all plants belonging to the 'oak' genus.
--

-- NOTE: Using a term_set aggregate as the RHS of `@@@` is not supported in `0.18.x`.
-- WITH genus_terms AS (
--   SELECT pdb.term_set(id) as terms
--   FROM genus
--   WHERE genus.name @@@ 'oak'
-- )
-- SELECT plants.id, plants.name
-- FROM plants, genus_terms
-- WHERE plants.genus_id @@@ genus_terms.terms
-- ORDER BY plants.id;


--
-- Test 2: Basic paradedb.aggregate query
-- Count all plants belonging to the 'oak' genus.
--
SELECT *
FROM paradedb.aggregate(
  'plants_idx',
  paradedb.to_search_query_input(
    'genus_id', 
    (
      SELECT pdb.term_set(id)
      FROM genus
      WHERE genus.name @@@ 'oak'
    )
  ),
  '{"count":{"value_count":{"field":"genus_id"}}}'
);


--
-- Test 3: No matching genus
-- Search for a genus that does not exist. Should return no plants.
--


-- NOTE: Using a term_set aggregate as the RHS of `@@@` is not supported in `0.18.x`.
-- WITH genus_terms AS (
--   SELECT pdb.term_set(id) as terms
--   FROM genus
--   WHERE genus.name @@@ 'bamboo'
-- )
-- SELECT plants.id, plants.name
-- FROM plants, genus_terms
-- WHERE plants.genus_id @@@ genus_terms.terms
-- ORDER BY plants.id;


--
-- Test 4: Incorrect data type
-- Attempt to use term_set on a TEXT column. This should fail.
--
WITH genus_terms AS (
  SELECT pdb.term_set(name) as terms
  FROM genus
  WHERE genus.name @@@ 'oak'
)
SELECT 1;


DROP TABLE plants CASCADE;
DROP TABLE genus CASCADE;
\i common/common_cleanup.sql
