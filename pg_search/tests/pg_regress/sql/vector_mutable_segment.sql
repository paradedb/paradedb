CREATE EXTENSION IF NOT EXISTS vector;
\i common/common_setup.sql
-- Tiny fixtures: lower the centroid-training floor.
SET paradedb.vector_min_training_rows = 1;

DROP TABLE IF EXISTS mv;
CREATE TABLE mv (
    id    int PRIMARY KEY,
    label text,
    vec   vector(3)
);

-- Centroids train at CREATE INDEX over existing rows, so seed a corpus
-- first; the rows inserted AFTER the index exercise the mutable segment.
INSERT INTO mv VALUES
    (1, 'east',  '[1.0,  0.0, 0.0]'),
    (2, 'east2', '[0.9,  0.0, 0.1]');

CREATE INDEX mv_idx ON mv
    USING paradedb (id, label, vec vector_l2_ops)
    WITH (
        key_field = id,
        mutable_segment_rows = 100,
        background_layer_sizes = '0',
        layer_sizes = '100mb'
    );

INSERT INTO mv VALUES
    (3, 'north', '[0.0,  1.0, 0.0]'),
    (4, 'up',    '[0.0,  0.0, 1.0]'),
    (5, 'mid',   '[0.7,  0.7, 0.0]');

SELECT mutable, num_docs
FROM paradedb.index_info('mv_idx')
ORDER BY mutable DESC, num_docs DESC;

SELECT id, label
FROM mv
WHERE id @@@ pdb.all()
ORDER BY vec <-> '[1, 0, 0]'
LIMIT 3;

DROP TABLE mv;
