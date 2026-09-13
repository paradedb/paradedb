-- Non-finite vector elements (NaN / ±Inf) can never reach pg_search's ingestion
-- through SQL: pgvector enforces element finiteness on every SQL-visible
-- constructor of `vector` datums (the `vector_in` text parser and array casts),
-- so a non-finite vector is rejected at the type level before pg_search's
-- `PgVector::from_datum` -> `document.add_vector` ever runs. tantivy's own
-- backstop (rejecting non-finite elements for cosine fields at add_document) is
-- therefore unreachable via SQL. This test documents that, and that finite
-- vectors still ingest and are searchable.
CREATE EXTENSION IF NOT EXISTS vector;
CREATE EXTENSION IF NOT EXISTS pg_search;
-- Tiny fixtures: lower the centroid-training floor.
SET paradedb.vector_min_training_rows = 1;

CREATE TABLE nonfinite_vec (
    id  int PRIMARY KEY,
    vec vector(3)
);
-- Centroids are trained at CREATE INDEX over the existing rows, so the
-- (finite) control corpus is loaded BEFORE the index exists — an index on
-- an empty table is an error under the index-level centroid format.
INSERT INTO nonfinite_vec VALUES (3, '[1, 0, 0]'), (4, '[0.5, 0.5, 0]');
CREATE INDEX nonfinite_vec_idx ON nonfinite_vec
    USING paradedb (id, vec vector_cosine_ops)
    WITH (key_field = id);

-- vector_in: the text parser rejects NaN before a datum exists.
INSERT INTO nonfinite_vec VALUES (1, '[NaN, 0, 0]');

-- array_to_vector: real[] casts are checked element-wise too, so an
-- expression-index shape over float arrays cannot smuggle one in either.
INSERT INTO nonfinite_vec VALUES (2, ARRAY['Infinity'::float4, 0, 0]::vector);

-- Control: the pre-loaded finite vectors are searchable, proving the
-- rejections above happened before pg_search rather than inside a broken
-- index.
SELECT id FROM nonfinite_vec WHERE id @@@ pdb.all() ORDER BY vec <=> '[1, 0, 0]' LIMIT 2;

DROP TABLE nonfinite_vec;
