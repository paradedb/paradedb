-- The two-participant vector scan: one participant drains the clustered
-- tier, the other the flat (mutable) tier, and Gather Merge combines
-- them on the distance column. Results must equal the serial plan's.
--
-- The parallel path is forced exactly when the plan-time snapshot holds
-- a flat vector segment; indexes with no mutable tier stay serial.

CREATE EXTENSION IF NOT EXISTS vector;
CREATE EXTENSION IF NOT EXISTS pg_search;
-- Tiny fixtures: lower the centroid-training floor.
SET paradedb.vector_min_training_rows = 1;

CREATE TABLE ptier (
    id  int PRIMARY KEY,
    vec vector(3)
);

-- Clustered corpus: trained and flushed at CREATE INDEX.
INSERT INTO ptier
SELECT g, ('[' || (g % 7)::text || ',' || (g % 5)::text || ',' || (g % 3)::text || ']')::vector
FROM generate_series(1, 60) g;

CREATE INDEX ptier_idx ON ptier
    USING paradedb (id, vec vector_l2_ops)
    WITH (
        key_field = id,
        mutable_segment_rows = 100,
        background_layer_sizes = '0',
        layer_sizes = '100mb'
    );

-- Staged rows: land in the flat mutable segment. Two are the closest
-- vectors to the query below, so a scan that lost the flat tier would
-- return visibly wrong results.
INSERT INTO ptier VALUES
    (61, '[9,   9, 9]'),
    (62, '[9.1, 9, 9]'),
    (63, '[0.5, 0, 0]');

-- Serial baseline.
SET max_parallel_workers_per_gather = 0;
SELECT id FROM ptier WHERE id @@@ pdb.all() ORDER BY vec <-> '[9,9,9]' LIMIT 5;

-- Allow workers: the flat tier's existence does the rest.
SET max_parallel_workers_per_gather = 2;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM ptier WHERE id @@@ pdb.all() ORDER BY vec <-> '[9,9,9]' LIMIT 5;
SELECT id FROM ptier WHERE id @@@ pdb.all() ORDER BY vec <-> '[9,9,9]' LIMIT 5;

DROP TABLE ptier;
