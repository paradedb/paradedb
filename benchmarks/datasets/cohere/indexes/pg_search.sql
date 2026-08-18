-- Deliberately `USING bm25`, not `USING paradedb`. This SQL is executed against a
-- cluster restored from a pgBackRest heap snapshot (the `restore-heap` steps in
-- .github/workflows/benchmark-pg_search-queries.yml), which replaces the entire
-- data directory. The restored catalog therefore carries whatever pg_search
-- version the snapshot was captured with, not the version built from the branch,
-- and the `paradedb` access method only exists from 0.25.0 onward. `bm25` is the
-- permanent backwards-compatible alias and resolves on every version, so it is
-- the only name that is safe here until the snapshots are regenerated.

CREATE INDEX cohere_wiki_bm25_idx ON cohere_wiki
USING bm25 (
    _id,
    (text::pdb.unicode_words('stemmer=english', 'stopwords_language=english')),
    emb vector_cosine_ops
) WITH (
    key_field = '_id',
    centroid_ratio = 0.01,
    target_segment_count = 8,
    cluster_replication = 1
);
