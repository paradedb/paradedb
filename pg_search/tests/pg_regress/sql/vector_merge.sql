-- Merging an IVF segment into a larger one (tantivy c4582807: IVF count()
-- returns distinct docs, not posting rows). At the prior rev, a segment's
-- posting-row total was read where a doc count was meant, tripping
-- debug-build accounting asserts in tantivy's IVF plugin when such a segment
-- became a merge SOURCE. pgrx regress builds are debug builds, so this test
-- exercises those asserts for real.
--
-- Assignment is primary-only: every vector lands in exactly one cell, so per-cluster
-- memberships sum to the distinct-doc count.
--
-- client_min_messages: the IVF merge emits a paradedb::ivf_build timings
-- NOTICE with nondeterministic millisecond values — keep it out of the
-- captured output.
SET client_min_messages = WARNING;
CREATE EXTENSION IF NOT EXISTS vector;
\i common/common_setup.sql

DROP TABLE IF EXISTS remerge;
CREATE TABLE remerge (
    id  int PRIMARY KEY,
    vec vector(16)
);

-- mutable_segment_rows = 0 routes every insert through immutable segments
-- (foreground merges only fire on an insert-cleanup that created a segment);
-- target_segment_count = 1 keeps the merge policy engaged (merging is
-- disabled while segment_count <= target); background_layer_sizes = '0'
-- keeps every merge in the foreground, deterministic. Explicit 1000-row batches
-- create segments of ~70kb, and a 600kb layer closes its first candidate at
-- >= 10000 docs — at or above tantivy's vector_clustering_threshold, so the
-- merge target is written IVF (clustered) under the default centroid router.
CREATE INDEX remerge_idx ON remerge
    USING paradedb (id, vec vector_l2_ops)
    WITH (
        target_segment_count = 1,
        mutable_segment_rows = 0,
        layer_sizes = '600kb',
        background_layer_sizes = '0'
    );

-- Wave 1: deterministic vectors (no random()), enough segments that the
-- 600kb candidate comfortably overfills even if per-segment bytes drift.
DO $$
BEGIN
    FOR batch_start IN 1..15000 BY 1000 LOOP
        INSERT INTO remerge
        SELECT g, ('[' || repeat((g % 89)::text || ',', 15) || (g % 89)::text || ']')::vector
        FROM generate_series(batch_start, batch_start + 999) g;
    END LOOP;
END
$$;

-- The merge produced a clustered segment...
SELECT bool_or(vector_format = 'ivf') AS has_ivf
FROM paradedb.vector_info('remerge_idx', 'vec');

-- ...whose reported vector count is distinct docs (= its doc count)...
SELECT bool_and(v.vector_num_vectors = i.num_docs) AS num_vectors_is_distinct_docs
FROM paradedb.vector_info('remerge_idx', 'vec') v
JOIN paradedb.index_info('remerge_idx') i USING (segno)
WHERE v.vector_format = 'ivf';

-- ...and the per-cluster sizes are memberships, which under primary-only
-- assignment total exactly the distinct-doc count.
SELECT sum(vector_total_rows) = sum(vector_num_vectors)
         AS cluster_sizes_are_memberships
FROM paradedb.vector_info('remerge_idx', 'vec')
WHERE vector_format = 'ivf';

-- Wave 2: make the IVF segment a merge SOURCE. The layer must
-- admit it (it is ~2.5-3.1mb; 3500kb leaves headroom) and the total corpus
-- must overfill the extended layer, which the extra 35000 rows guarantee.
ALTER INDEX remerge_idx SET (layer_sizes = '3500kb');
DO $$
BEGIN
    FOR batch_start IN 15001..50000 BY 1000 LOOP
        INSERT INTO remerge
        SELECT g, ('[' || repeat((g % 89)::text || ',', 15) || (g % 89)::text || ']')::vector
        FROM generate_series(batch_start, batch_start + 999) g;
    END LOOP;
END
$$;

SELECT bool_or(vector_format = 'ivf') AS still_has_ivf
FROM paradedb.vector_info('remerge_idx', 'vec');

SELECT bool_and(v.vector_num_vectors = i.num_docs) AS num_vectors_is_distinct_docs
FROM paradedb.vector_info('remerge_idx', 'vec') v
JOIN paradedb.index_info('remerge_idx') i USING (segno)
WHERE v.vector_format = 'ivf';

-- Heap still has every row. IVF doc/cluster accounting above already proves
-- the merge lost nothing at the storage layer. Stacked IVF's hard 25% top
-- prune means max_probe=1.0 is no longer an exhaustive scan, so the ANN path
-- only checks that returned ids are unique (nothing doubled) rather than
-- requiring every row.
SELECT count(*) = 50000 AS heap_has_all_rows FROM remerge;
SET paradedb.vector_cluster_max_probe = 1.0;
SELECT count(*) = count(DISTINCT id) AS ann_returns_unique_ids
FROM (
    SELECT id
    FROM remerge
    WHERE id @@@ pdb.all()
    ORDER BY vec <-> '[1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1]'
    LIMIT 60000
) q;
RESET paradedb.vector_cluster_max_probe;

DROP TABLE remerge;
