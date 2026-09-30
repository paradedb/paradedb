SET client_min_messages = WARNING;
CREATE EXTENSION IF NOT EXISTS vector;
\i common/common_setup.sql
SET paradedb.vector_clustering_threshold = 64;
SET paradedb.vector_cluster_max_probe = 1.0;

CREATE TABLE vector_router_items (id integer PRIMARY KEY, vec vector(3));
INSERT INTO vector_router_items
SELECT g, ARRAY[g % 17, g % 23, g % 31]::vector
FROM generate_series(1, 256) g;

-- Omitting vector_router uses the graph router.
CREATE INDEX vector_router_default_idx ON vector_router_items
USING paradedb (id, vec vector_l2_ops)
WITH (target_segment_count = 1, mutable_segment_rows = 0,
      vector_fields = '{"vec":{"quantization":false}}');
SELECT id FROM vector_router_items
WHERE id @@@ pdb.all()
ORDER BY vec <-> '[1,2,3]', id
LIMIT 5;
DROP INDEX vector_router_default_idx;

-- Both routers build and serve the same exact-probe results; values are case-insensitive.
CREATE INDEX vector_router_graph_idx ON vector_router_items
USING paradedb (id, vec vector_l2_ops)
WITH (target_segment_count = 1, mutable_segment_rows = 0, vector_router = 'Graph',
      vector_fields = '{"vec":{"quantization":false}}');
SELECT id FROM vector_router_items
WHERE id @@@ pdb.all()
ORDER BY vec <-> '[1,2,3]', id
LIMIT 5;
DROP INDEX vector_router_graph_idx;

CREATE INDEX vector_router_ivf_idx ON vector_router_items
USING paradedb (id, vec vector_l2_ops)
WITH (target_segment_count = 1, mutable_segment_rows = 0, vector_router = 'ivf',
      vector_fields = '{"vec":{"quantization":false}}');
SELECT id FROM vector_router_items
WHERE id @@@ pdb.all()
ORDER BY vec <-> '[1,2,3]', id
LIMIT 5;

-- Inserts after the build keep merging under the index's router.
INSERT INTO vector_router_items
SELECT g, ARRAY[g % 17, g % 23, g % 31]::vector
FROM generate_series(257, 512) g;
SELECT count(*) AS indexed_rows
FROM (
    SELECT id FROM vector_router_items
    WHERE id @@@ pdb.all()
    ORDER BY vec <-> '[1,2,3]'
    LIMIT 512
) matches;

-- A changed router applies to segments built afterwards; existing segments
-- keep the router they were built with and stay searchable.
ALTER INDEX vector_router_ivf_idx SET (vector_router = 'graph');
SELECT reloptions @> ARRAY['vector_router=graph'] AS stores_router
FROM pg_class WHERE oid = 'vector_router_ivf_idx'::regclass;
SELECT id FROM vector_router_items
WHERE id @@@ pdb.all()
ORDER BY vec <-> '[1,2,3]', id
LIMIT 5;
INSERT INTO vector_router_items
SELECT g, ARRAY[g % 17, g % 23, g % 31]::vector
FROM generate_series(513, 768) g;
SELECT count(*) AS indexed_rows
FROM (
    SELECT id FROM vector_router_items
    WHERE id @@@ pdb.all()
    ORDER BY vec <-> '[1,2,3]'
    LIMIT 768
) matches;

-- REINDEX rebuilds every segment with the new router.
REINDEX INDEX vector_router_ivf_idx;
SELECT id FROM vector_router_items
WHERE id @@@ pdb.all()
ORDER BY vec <-> '[1,2,3]', id
LIMIT 5;
ALTER INDEX vector_router_ivf_idx RESET (vector_router);
REINDEX INDEX vector_router_ivf_idx;
SELECT count(*) AS indexed_rows
FROM (
    SELECT id FROM vector_router_items
    WHERE id @@@ pdb.all()
    ORDER BY vec <-> '[1,2,3]'
    LIMIT 768
) matches;

-- Unknown routers are rejected.
ALTER INDEX vector_router_ivf_idx SET (vector_router = 'auto');
DROP INDEX vector_router_ivf_idx;
CREATE INDEX vector_router_bad_idx ON vector_router_items
USING paradedb (id, vec vector_l2_ops)
WITH (vector_router = 'auto');

DROP TABLE vector_router_items;
