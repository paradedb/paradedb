-- A search operator reports an operand that it can't search as an error.

CREATE EXTENSION IF NOT EXISTS pg_search;
DROP TABLE IF EXISTS operand_items;

CREATE TABLE operand_items (id int PRIMARY KEY, description text);
INSERT INTO operand_items VALUES (1, 'leather sofa');
CREATE INDEX operand_items_idx ON operand_items USING bm25 (id, description);

-- The left side has to be a field of the index. An expression that the index does not have is
-- an error, with a constant and with a value known only at run time.
SELECT id FROM operand_items WHERE lower(description) ||| 'sofa';
SELECT id FROM operand_items WHERE lower(description) ||| (SELECT 'sofa'::text);

-- `@@@` does not take a text array, also when the array has a cast.
SELECT id FROM operand_items WHERE description @@@ ARRAY['sofa', 'chair']::pdb.fuzzy(1);

DROP TABLE operand_items;
