-- pg_search's paradedb AM supports only the pgvector `vector` type as a vector field
-- (opclasses vector_l2_ops / vector_cosine_ops / vector_ip_ops, declared
-- `FOR TYPE public.vector USING paradedb`). The other pgvector element types have
-- their own opclasses -- halfvec_l2_ops, sparsevec_l2_ops, bit_hamming_ops --
-- but those are pgvector's hnsw/ivfflat opclasses, not paradedb ones, so a paradedb
-- index over halfvec / sparsevec / bit is rejected at CREATE INDEX.
CREATE EXTENSION IF NOT EXISTS vector;
CREATE EXTENSION IF NOT EXISTS pg_search;
-- Tiny fixtures: lower the centroid-training floor.
SET paradedb.vector_min_training_rows = 1;

CREATE TABLE unsupported_vec_types (
    id int PRIMARY KEY,
    hv halfvec(3),
    sv sparsevec(3),
    bv bit(3)
);

-- halfvec: halfvec_l2_ops is an hnsw/ivfflat opclass, not a paradedb one
CREATE INDEX ON unsupported_vec_types USING paradedb (id, hv halfvec_l2_ops) WITH (key_field = id);
-- sparsevec: sparsevec_l2_ops is an hnsw/ivfflat opclass, not a paradedb one
CREATE INDEX ON unsupported_vec_types USING paradedb (id, sv sparsevec_l2_ops) WITH (key_field = id);
-- bit: bit_hamming_ops is an hnsw/ivfflat opclass, not a paradedb one
CREATE INDEX ON unsupported_vec_types USING paradedb (id, bv bit_hamming_ops) WITH (key_field = id);

DROP TABLE unsupported_vec_types;
