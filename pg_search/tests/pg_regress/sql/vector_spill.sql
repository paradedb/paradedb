SET client_min_messages = WARNING;
CREATE EXTENSION IF NOT EXISTS vector;
\i common/common_setup.sql

SET paradedb.vector_clustering_threshold = 64;
SET paradedb.vector_cluster_max_probe = 1.0;
SET paradedb.vector_adaptive_probing = off;
SET paradedb.vector_stats = on;

CREATE FUNCTION spill_fixture_vector(d integer, n integer)
RETURNS vector
LANGUAGE SQL IMMUTABLE PARALLEL SAFE
AS $$
    SELECT (
        '[' || string_agg(
            (((hashtextextended(n::text || ':' || i::text, 81985529216486895) & 1048575)
                / 524287.5) - 1.0)::text,
            ',' ORDER BY i
        ) || ']'
    )::vector
    FROM generate_series(1, d) i
$$;

CREATE FUNCTION spill_explain(query_text text)
RETURNS jsonb
LANGUAGE plpgsql
AS $$
DECLARE
    plan jsonb;
BEGIN
    EXECUTE 'EXPLAIN (ANALYZE, VERBOSE, COSTS OFF, TIMING OFF, BUFFERS ON, SUMMARY OFF, FORMAT JSON) '
        || query_text
        INTO plan;
    RETURN plan;
END
$$;

-- A spilled quantized IVF segment: every vector in its nearest cluster and in a second one.
CREATE TABLE spill_rows (id integer PRIMARY KEY, vec vector(768));
CREATE INDEX spill_rows_idx ON spill_rows
USING paradedb (id, vec vector_cosine_ops)
WITH (
    spill = true,
    max_leaf_size = 5,
    training_sample_ratio = 1.0,
    target_segment_count = 1,
    mutable_segment_rows = 0,
    layer_sizes = '400kb',
    background_layer_sizes = '0'
);
INSERT INTO spill_rows SELECT g, spill_fixture_vector(768, g) FROM generate_series(1, 100) g;
INSERT INTO spill_rows SELECT g, spill_fixture_vector(768, g) FROM generate_series(101, 200) g;
VACUUM spill_rows;

SELECT 'spill=true' = ANY(reloptions) AS option_stored
FROM pg_class WHERE relname = 'spill_rows_idx';

SELECT bool_and(vector_format = 'ivf') AS clustered,
       bool_and(quantized) AS quantized,
       sum(vector_num_vectors) = 200 AS every_vector_present,
       sum(vector_total_rows) = 2 * sum(vector_num_vectors) AS two_rows_per_vector
FROM paradedb.vector_info('spill_rows_idx', 'vec');

-- The exhaustive probe returns the exact order; the oracle runs over an unindexed copy.
CREATE TABLE spill_rows_plain AS SELECT id, vec FROM spill_rows;
WITH indexed AS (
    SELECT id FROM spill_rows WHERE id @@@ pdb.all()
    ORDER BY vec <=> spill_fixture_vector(768, 0), id LIMIT 10
), oracle AS (
    SELECT id FROM spill_rows_plain
    ORDER BY vec <=> spill_fixture_vector(768, 0), id LIMIT 10
)
SELECT (SELECT array_agg(id) FROM indexed) = (SELECT array_agg(id) FROM oracle) AS top10_matches_oracle,
       (SELECT count(*) FROM indexed) = 10 AS ten_hits;

-- With every cluster open, each document's second copy is dropped once; the plan's layer-0
-- accounting still covers every eligible row, and the exact phase scores a document at most once.
WITH plan AS (
    SELECT spill_explain(
        'SELECT id FROM spill_rows WHERE id @@@ pdb.all() '
        'ORDER BY vec <=> spill_fixture_vector(768, 0), id LIMIT 10'
    ) AS value
), segment_info AS (
    SELECT (jsonb_path_query_first(value, '$.**."Segment Info"') #>> '{}')::jsonb AS value
    FROM plan
)
SELECT
    (jsonb_path_query_first(value, '$.**.layer0_eligible') #>> '{}')::bigint = 400
        AS every_row_eligible,
    (jsonb_path_query_first(value, '$.**.spill_dropped') #>> '{}')::bigint = 200
        AS one_copy_dropped_per_document,
    (jsonb_path_query_first(value, '$.**.layer0_eligible') #>> '{}')::bigint
        = (jsonb_path_query_first(value, '$.**.layer0_scored') #>> '{}')::bigint
            + (jsonb_path_query_first(value, '$.**.layer0_exact_rows') #>> '{}')::bigint
        AS eligible_is_scored_plus_planned_exact,
    (jsonb_path_query_first(value, '$.**.exact_rows') #>> '{}')::bigint BETWEEN 10 AND 200
        AS exact_rows_once_per_document,
    (jsonb_path_query_first(value, '$.**.vectors_visited') #>> '{}')::bigint = 400
        AS every_row_visited
FROM segment_info;

-- A narrower probe still returns the exact top 10 here: both copies of a document can carry it.
SET paradedb.vector_cluster_max_probe = 0.5;
WITH indexed AS (
    SELECT id FROM spill_rows WHERE id @@@ pdb.all()
    ORDER BY vec <=> spill_fixture_vector(768, 7), id LIMIT 10
), oracle AS (
    SELECT id FROM spill_rows_plain
    ORDER BY vec <=> spill_fixture_vector(768, 7), id LIMIT 10
)
SELECT (SELECT count(*) FROM indexed i JOIN oracle o USING (id)) >= 9 AS recall_at_half_probe;
SET paradedb.vector_cluster_max_probe = 1.0;

DROP TABLE spill_rows_plain;
DROP TABLE spill_rows;

-- The option is inert for an unquantized field: one row per vector.
CREATE TABLE spill_plain (id integer PRIMARY KEY, vec vector(768));
CREATE INDEX spill_plain_idx ON spill_plain
USING paradedb (id, vec vector_cosine_ops)
WITH (
    spill = true,
    quantization = '0',
    max_leaf_size = 5,
    training_sample_ratio = 1.0,
    target_segment_count = 1,
    mutable_segment_rows = 0,
    layer_sizes = '400kb',
    background_layer_sizes = '0'
);
INSERT INTO spill_plain SELECT g, spill_fixture_vector(768, g) FROM generate_series(1, 100) g;
INSERT INTO spill_plain SELECT g, spill_fixture_vector(768, g) FROM generate_series(101, 200) g;
VACUUM spill_plain;
SELECT bool_and(vector_format = 'ivf') AS clustered,
       bool_and(NOT quantized) AS unquantized,
       sum(vector_total_rows) = sum(vector_num_vectors) AS one_row_per_vector
FROM paradedb.vector_info('spill_plain_idx', 'vec');
DROP TABLE spill_plain;

DROP FUNCTION spill_explain(text);
DROP FUNCTION spill_fixture_vector(integer, integer);
