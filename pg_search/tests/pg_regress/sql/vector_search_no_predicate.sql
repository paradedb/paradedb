-- A LIMITed vector-distance ORDER BY drives the bm25 index's vector TopK even
-- with no `@@@` predicate: the ordering itself justifies the custom scan, run
-- over an implicit match-all query. It requires a LIMIT (otherwise it's a full
-- sort, no better than a seqscan) and the operator's metric must match the
-- column's opclass (otherwise the planner falls back to a regular sort).
--
-- COSTS OFF for stable EXPLAIN diffs; a 5-row corpus where the K=2 ordering is
-- unambiguous.

CREATE EXTENSION IF NOT EXISTS vector;
CREATE EXTENSION IF NOT EXISTS pg_search;

CREATE TABLE vnp (
    id    int PRIMARY KEY,
    label text,
    vec   vector(3)
);

INSERT INTO vnp VALUES
    (1, 'east',  '[1,    0,   0]'),
    (2, 'east2', '[0.9,  0,   0.1]'),
    (3, 'north', '[0,    1,   0]'),
    (4, 'up',    '[0,    0,   1]'),
    (5, 'mid',   '[0.7,  0.7, 0]');

CREATE INDEX vnp_idx ON vnp
    USING bm25 (id, label, vec vector_cosine_ops)
    WITH (key_field = id);

-- bare ORDER BY + LIMIT, no @@@: pushes down to the vector TopK
EXPLAIN (COSTS OFF)
SELECT id FROM vnp ORDER BY vec <=> '[1,0,0]' LIMIT 2;
SELECT id FROM vnp ORDER BY vec <=> '[1,0,0]' LIMIT 2;

-- ... and returns the same ranking as the explicit `@@@ all()` form
SELECT id FROM vnp WHERE id @@@ paradedb.all() ORDER BY vec <=> '[1,0,0]' LIMIT 2;

-- no LIMIT: not a Top K, so no custom scan (a full vector sort beats nothing)
EXPLAIN (COSTS OFF)
SELECT id FROM vnp ORDER BY vec <=> '[1,0,0]';

-- operator metric disagrees with the opclass (<-> is L2, index is cosine):
-- falls back to a regular sort rather than silently ranking by the wrong metric
EXPLAIN (COSTS OFF)
SELECT id FROM vnp ORDER BY vec <-> '[1,0,0]' LIMIT 2;

DROP TABLE vnp;
