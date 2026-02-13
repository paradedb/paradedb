\i common/common_setup.sql

SET paradedb.enable_aggregate_custom_scan = true;

CREATE TABLE tokenizer_fast (
    id serial8 not null primary key,
    t text,
    t_long text,
    metadata jsonb
);

INSERT INTO tokenizer_fast (t, t_long, metadata) VALUES
    ('hello', 'The big cat', '{"key": "The big cat", "value": 1}'),
    ('hello', 'the big cat', '{"key": "the big cat", "value": 3}'),
    ('world', 'Quick brown fox', '{"key": "Quick brown fox", "value": 2}');

CREATE INDEX idxtokenizer_fast ON tokenizer_fast USING bm25 (
    id,
    (t::pdb.literal),
    (t_long::pdb.normalized('stopwords_language=English')),
    (metadata::pdb.normalized('stopwords_language=English'))
) WITH (key_field = 'id');

SELECT * FROM paradedb.schema('idxtokenizer_fast') ORDER BY name;

-- Top N over literal
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT * FROM tokenizer_fast WHERE id @@@ pdb.all() ORDER BY t, id LIMIT 5;
SELECT * FROM tokenizer_fast WHERE id @@@ pdb.all() ORDER BY t, id LIMIT 5;

-- Aggregate scan pushdown over literal
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT t, COUNT(*) FROM tokenizer_fast WHERE id @@@ pdb.all() GROUP BY t ORDER BY t LIMIT 5;
SELECT t, COUNT(*) FROM tokenizer_fast WHERE id @@@ pdb.all() GROUP BY t ORDER BY t LIMIT 5;

-- Top N over normalized
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT * FROM tokenizer_fast WHERE id @@@ pdb.all() ORDER BY t_long, id LIMIT 5;
SELECT * FROM tokenizer_fast WHERE id @@@ pdb.all() ORDER BY t_long, id LIMIT 5;

-- Aggregate scan pushdown over normalized
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT t_long, COUNT(*) FROM tokenizer_fast WHERE id @@@ pdb.all() GROUP BY t_long ORDER BY t_long LIMIT 5;
SELECT t_long, COUNT(*) FROM tokenizer_fast WHERE id @@@ pdb.all() GROUP BY t_long ORDER BY t_long LIMIT 5;

-- Aggregate scan pushdown over JSON
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT metadata->>'key', COUNT(*) FROM tokenizer_fast WHERE id @@@ pdb.all() GROUP BY metadata->>'key' ORDER BY metadata->>'key' LIMIT 5;
SELECT metadata->>'key', COUNT(*) FROM tokenizer_fast WHERE id @@@ pdb.all() GROUP BY metadata->>'key' ORDER BY metadata->>'key' LIMIT 5;

-- Order by JSON not supported
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT * FROM tokenizer_fast WHERE id @@@ pdb.all() ORDER BY metadata->>'key', id LIMIT 5;
SELECT * FROM tokenizer_fast WHERE id @@@ pdb.all() ORDER BY metadata->>'key', id LIMIT 5;

DROP TABLE tokenizer_fast;
