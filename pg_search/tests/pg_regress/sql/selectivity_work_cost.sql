\i common/common_setup.sql
SET max_parallel_workers_per_gather = 2;
SET paradedb.global_mutable_segment_rows = 0;

CREATE TABLE cost_items (id int, description text, category text, rating int);
CREATE INDEX cost_items_idx ON cost_items USING paradedb (id, description, category, rating)
WHERE category = 'Electronics';
INSERT INTO cost_items SELECT i, 'product ' || i, 'Electronics', 2 + i % 4 FROM generate_series(1, 10) i;
INSERT INTO cost_items SELECT i, 'product ' || i, 'Electronics', 2 + i % 4 FROM generate_series(11, 20) i;
INSERT INTO cost_items SELECT i, 'product ' || i, 'Electronics', 2 + i % 4 FROM generate_series(21, 30) i;
ANALYZE cost_items;

-- Cheap sorted scans stay serial even with several segments and a parsed query.
EXPLAIN (COSTS OFF, VERBOSE)
SELECT id, rating FROM cost_items
WHERE category = 'Electronics' AND cost_items @@@ pdb.parse('rating:>1')
ORDER BY rating, id LIMIT 5;

-- A non-text child must not discard the text child's work estimate.
EXPLAIN (COSTS OFF, VERBOSE)
SELECT id, rating FROM cost_items
WHERE category = 'Electronics' AND description ||| 'product' AND rating >= 3
ORDER BY rating, id LIMIT 5;

EXPLAIN (COSTS OFF, VERBOSE)
SELECT id, rating FROM cost_items
WHERE category = 'Electronics' AND rating @@@ pdb.term_set(ARRAY[2, 3])
ORDER BY rating, id LIMIT 5;
SELECT id, rating FROM cost_items
WHERE category = 'Electronics' AND rating @@@ pdb.term_set(ARRAY[2, 3])
ORDER BY rating, id LIMIT 5;

-- An intersection can use the rare term; a union must traverse the common one too.
SET parallel_setup_cost = 0.04;
SET parallel_tuple_cost = 0;
EXPLAIN (COSTS OFF, VERBOSE)
SELECT id FROM cost_items
WHERE category = 'Electronics' AND description ||| 'product' AND id @@@ pdb.term(1)
ORDER BY rating LIMIT 1;
EXPLAIN (COSTS OFF, VERBOSE)
SELECT id FROM cost_items
WHERE category = 'Electronics' AND (description ||| 'product' OR id @@@ pdb.term(1))
ORDER BY rating LIMIT 1;
RESET parallel_setup_cost;
RESET parallel_tuple_cost;

DROP TABLE cost_items;
RESET paradedb.global_mutable_segment_rows;
RESET max_parallel_workers_per_gather;
