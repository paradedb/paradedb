drop table if exists test_phrase_table cascade;

CREATE TABLE test_phrase_table (
   id SERIAL PRIMARY KEY,
   flavour TEXT
);
INSERT INTO test_phrase_table (flavour) VALUES
    ('apple, with, banana'),
    ('Banana with Cherry'),
    ('Cherry, strawberry'),
    ('apple, cherry, banana');


CREATE INDEX test_phrase_index ON test_phrase_table USING paradedb (id, (flavour::pdb.simple));

SELECT flavour FROM test_phrase_table
WHERE id @@@ '{
        "phrase": {
            "field": "flavour",
            "phrases": ["apple", "BANANA"],
            "slop": 2
        }
    }'::jsonb ORDER BY id;

-- Empty phrase_prefix terms must not construct PhrasePrefixQuery (Tantivy
-- asserts at least one term). Match nothing instead of panicking (#6206).
SELECT flavour FROM test_phrase_table
WHERE id @@@ '{
        "phrase_prefix": {
            "field": "flavour",
            "phrases": []
        }
    }'::jsonb ORDER BY id;

SELECT flavour FROM test_phrase_table
WHERE flavour @@@ pdb.phrase_prefix(ARRAY[]::text[]) ORDER BY id;

drop table if exists test_phrase_table cascade;
