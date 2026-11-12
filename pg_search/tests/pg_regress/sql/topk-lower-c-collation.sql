-- Regression test for #7054: Top-K over an indexed lower(text) expression under
-- the C collation must agree with PostgreSQL's own ordering.

\i common/common_setup.sql

SET max_parallel_workers_per_gather = 0;

DROP TABLE IF EXISTS topk_lower_c_collation CASCADE;

CREATE TABLE topk_lower_c_collation (
    id integer PRIMARY KEY,
    body text,
    category text COLLATE "C"
);

INSERT INTO topk_lower_c_collation VALUES
    (1, 'wireless', 'ä'),
    (2, 'wireless', 'Å');

CREATE INDEX topk_lower_c_collation_idx ON topk_lower_c_collation USING paradedb (
    id,
    body,
    (lower(category)::pdb.literal)
);

-- Top-K through the ParadeDB custom scan.
SET paradedb.enable_custom_scan = on;
SELECT id, category
FROM topk_lower_c_collation
WHERE body ||| 'wireless'
ORDER BY lower(category) COLLATE "C", id
LIMIT 2;

-- The same query through PostgreSQL's own sort.
SET paradedb.enable_custom_scan = off;
SELECT id, category
FROM topk_lower_c_collation
WHERE body ||| 'wireless'
ORDER BY lower(category) COLLATE "C", id
LIMIT 2;

SET paradedb.enable_custom_scan = on;
DROP TABLE topk_lower_c_collation;
