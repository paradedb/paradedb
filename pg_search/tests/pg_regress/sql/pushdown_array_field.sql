\i common/common_setup.sql

DROP TABLE IF EXISTS pushdown;
CREATE TABLE pushdown(
    id SERIAL PRIMARY KEY,
    description TEXT[]
);

INSERT INTO pushdown(description)
VALUES (ARRAY['dog', 'cat', 'bird']), (ARRAY['fox', 'rabbit', 'squirrel']);

-- Test with literal tokenizer (should pushdown)
CREATE INDEX pushdown_idx ON pushdown USING paradedb (
    id, (description::pdb.literal)
);

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT * FROM pushdown
WHERE 'dog' = ANY(description)
AND id @@@ pdb.all()
ORDER BY id
LIMIT 10;

SELECT * FROM pushdown
WHERE 'dog' = ANY(description)
AND id @@@ pdb.all()
ORDER BY id
LIMIT 10;

DROP INDEX pushdown_idx;

-- Test with non-literal tokenizer (should NOT pushdown)
CREATE INDEX pushdown_idx ON pushdown USING paradedb (
    id, description
);

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT * FROM pushdown
WHERE 'dog' = ANY(description)
AND id @@@ pdb.all()
ORDER BY id
LIMIT 10;

DROP TABLE pushdown;
