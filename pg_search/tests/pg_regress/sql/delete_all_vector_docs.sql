-- Deleting every doc that carries a vector field, then merging (tantivy
-- c4582807). At the prior rev the merge SKIPPED the field's composite slots
-- when no live doc carried a vector, and every subsequent read of the field
-- failed with an InternalError; now the merge writes empty slots and the
-- field reads back as empty.
--
-- client_min_messages: the IVF merge emits a paradedb::ivf_build timings
-- NOTICE with nondeterministic millisecond values — keep it out of the
-- captured output.
SET client_min_messages = WARNING;
CREATE EXTENSION IF NOT EXISTS vector;
\i common/common_setup.sql

DROP TABLE IF EXISTS delvec;
CREATE TABLE delvec (
    id    int PRIMARY KEY,
    label text,
    vec   vector(16)
);

-- Same merge choreography as remerge_with_replicas.sql: immutable-only
-- inserts, foreground-only merging, first candidate closes >= 10000 docs so
-- the target is IVF.
CREATE INDEX delvec_idx ON delvec
    USING bm25 (id, label, vec vector_l2_ops)
    WITH (
        key_field = id,
        target_segment_count = 1,
        mutable_segment_rows = 0,
        layer_sizes = '600kb',
        background_layer_sizes = '0'
    );

-- Interleave vector-bearing (odd id) and vector-less (even id) rows so every
-- flushed segment — and therefore any merge candidate — mixes both
-- populations: after the DELETE below, merge sources still have live docs,
-- just none with vectors.
INSERT INTO delvec
SELECT g,
       md5(g::text),
       CASE WHEN g % 2 = 1
            THEN ('[' || repeat((g % 89)::text || ',', 15) || (g % 89)::text || ']')::vector
       END
FROM generate_series(1, 24000) g;

SELECT bool_or(vector_format = 'ivf') AS has_ivf
FROM paradedb.index_info('delvec_idx');

-- Kill every vector-bearing doc. VACUUM records the deletes so the next
-- merge sees those docs as dead.
DELETE FROM delvec WHERE vec IS NOT NULL;
VACUUM delvec;

-- Remerge with the (now vector-less) IVF segment as a source: ~880kb after
-- deletes, so an 1100kb layer admits it, and the surviving corpus plus the
-- fresh vector-less rows overfill the extended layer. The merge target has
-- >= 10000 live docs, none carrying a vector — exactly the empty-slots path.
ALTER INDEX delvec_idx SET (layer_sizes = '1100kb');
INSERT INTO delvec
SELECT g, md5(g::text), NULL
FROM generate_series(24001, 30000) g;

-- An IVF segment now exists whose vector field is empty.
SELECT bool_or(vector_format = 'ivf' AND vector_num_vectors = 0) AS ivf_with_empty_vector_field
FROM paradedb.index_info('delvec_idx');

-- Vector ORDER BY on the emptied field: no error, zero results. Exhaustive
-- probing, so the empty result cannot be an artifact of probe pruning.
SET paradedb.vector_cluster_max_probes = 65536;
SELECT count(*) AS vector_results
FROM (
    SELECT id
    FROM delvec
    WHERE id @@@ pdb.all()
    ORDER BY vec <-> '[1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1]'
    LIMIT 50000
) q;
RESET paradedb.vector_cluster_max_probes;

-- The index still serves non-vector queries over the surviving docs:
-- 12000 original vector-less rows plus 6000 fresh ones.
SELECT count(*) AS live_docs
FROM delvec
WHERE id @@@ pdb.all();

DROP TABLE delvec;
