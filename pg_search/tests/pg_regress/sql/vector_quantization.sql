-- End-to-end SQL proof for the V3 quantized vector path. The clustering
-- threshold is captured at CREATE INDEX, so a small deterministic fixture can
-- cross the flat/IVF boundary without making the regression test enormous.
SET client_min_messages = WARNING;
CREATE EXTENSION IF NOT EXISTS vector;
\i common/common_setup.sql

SET paradedb.vector_clustering_threshold = 64;
SET paradedb.vector_cluster_max_probe = 1.0;

CREATE FUNCTION quant_fixture_vector(d integer, n integer)
RETURNS vector
LANGUAGE SQL IMMUTABLE PARALLEL SAFE
AS $$
    SELECT (
        '[' || string_agg((((n * 31 + i * 17) % 101) - 50)::text, ',' ORDER BY i) || ']'
    )::vector
    FROM generate_series(1, d) i
$$;

CREATE FUNCTION quant_explain(query_text text)
RETURNS jsonb
LANGUAGE plpgsql
AS $$
DECLARE
    plan jsonb;
BEGIN
    EXECUTE 'EXPLAIN (ANALYZE, VERBOSE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF, FORMAT JSON) '
        || query_text
        INTO plan;
    RETURN plan;
END
$$;

-- Relation-shape diagnostics are deterministic and run before any calibration
-- state exists. Partition parents name every child index and direct callers to
-- calibrate the physical children independently.
CREATE TABLE q_cal_parent (id integer, vec vector(64)) PARTITION BY RANGE (id);
CREATE TABLE q_cal_parent_low PARTITION OF q_cal_parent
    FOR VALUES FROM (MINVALUE) TO (0);
CREATE TABLE q_cal_parent_high PARTITION OF q_cal_parent
    FOR VALUES FROM (0) TO (MAXVALUE);
CREATE INDEX q_cal_parent_idx ON ONLY q_cal_parent (id);
CREATE INDEX q_cal_parent_low_idx ON q_cal_parent_low (id);
CREATE INDEX q_cal_parent_high_idx ON q_cal_parent_high (id);
ALTER INDEX q_cal_parent_idx ATTACH PARTITION q_cal_parent_low_idx;
ALTER INDEX q_cal_parent_idx ATTACH PARTITION q_cal_parent_high_idx;

SELECT * FROM paradedb.vector_calibrate(
    'q_cal_parent_idx',
    'vec',
    ARRAY[quant_fixture_vector(64, 0)]
);
SELECT * FROM paradedb.vector_calibrate(
    'q_cal_parent_low',
    'vec',
    ARRAY[quant_fixture_vector(64, 0)]
);
SELECT * FROM paradedb.vector_calibrate(
    'q_cal_parent_low_idx',
    'vec',
    ARRAY[quant_fixture_vector(64, 0)]
);
DROP TABLE q_cal_parent;

CREATE TABLE q_cal_unquantized (id integer PRIMARY KEY, vec vector(64));
CREATE INDEX q_cal_unquantized_idx ON q_cal_unquantized
USING paradedb (id, vec vector_cosine_ops)
WITH (
    key_field = id,
    vector_fields = '{"vec":{"dims":64}}'
);
SELECT * FROM paradedb.vector_calibrate(
    'q_cal_unquantized_idx',
    'vec',
    ARRAY[quant_fixture_vector(64, 0)]
);
DROP TABLE q_cal_unquantized;

-- CREATE INDEX validation: the configured dimension must match the schema,
-- and quantized fields are inside the validated d >= 64 model envelope.
CREATE TABLE q_bad_dims (id integer PRIMARY KEY, vec vector(100));
CREATE INDEX q_bad_dims_idx ON q_bad_dims
USING paradedb (id, vec vector_l2_ops)
WITH (
    key_field = id,
    vector_fields = '{"vec":{"dims":99,"quantization":true}}'
);
DROP TABLE q_bad_dims;

CREATE TABLE q_below_floor (id integer PRIMARY KEY, vec vector(63));
CREATE INDEX q_below_floor_idx ON q_below_floor
USING paradedb (id, vec vector_l2_ops)
WITH (
    key_field = id,
    vector_fields = '{"vec":{"dims":63,"quantization":true}}'
);
DROP TABLE q_below_floor;

-- d=768 cosine, using the SQL shorthand for the default [1,4] schedule. Forty
-- centroids keep the 25% Gate-C probe non-exhaustive despite the 16-work-unit
-- small-segment probe floor.
CREATE TABLE q_cosine (id integer PRIMARY KEY, vec vector(768));
CREATE INDEX q_cosine_idx ON q_cosine
USING paradedb (id, vec vector_cosine_ops)
WITH (
    key_field = id,
    vector_fields = '{"vec":{"dims":768,"quantization":true}}',
    centroid_ratio = 0.2,
    target_segment_count = 1,
    mutable_segment_rows = 0,
    layer_sizes = '400kb',
    background_layer_sizes = '0'
);
INSERT INTO q_cosine SELECT g, quant_fixture_vector(768, g) FROM generate_series(1, 100) g;
INSERT INTO q_cosine SELECT g, quant_fixture_vector(768, g) FROM generate_series(101, 200) g;
VACUUM q_cosine;

SELECT bool_or(vector_format = 'ivf') AS cosine_has_ivf
FROM paradedb.vector_info('q_cosine_idx', 'vec');

-- Save the level-0 oracle before calibration. A fractional probe budget makes
-- this the unquantized-IVF baseline, not an exhaustive scan; the calibrated
-- level-0 query below must preserve its result order and exact scores.
SET paradedb.vector_cluster_max_probe = 0.25;
CREATE TEMP TABLE q_cosine_unquantized_ivf AS
SELECT
    row_number() OVER (ORDER BY distance, id) AS ordinal,
    id,
    distance
FROM (
    SELECT id, vec <=> quant_fixture_vector(768, 0) AS distance
    FROM q_cosine
    WHERE id @@@ pdb.all()
    ORDER BY vec <=> quant_fixture_vector(768, 0), id
    LIMIT 10
) hits;

-- An uncalibrated quantized field must announce and use the routed,
-- full-precision IVF path. The NOTICE is query-scoped even though the fixture
-- can contain multiple segments.
SET client_min_messages = NOTICE;
WITH plan AS (
    SELECT quant_explain(
        'SELECT id FROM q_cosine WHERE id @@@ pdb.all() '
        'ORDER BY vec <=> quant_fixture_vector(768, 0), id LIMIT 10'
    ) AS value
), segment_info AS (
    SELECT (jsonb_path_query_first(value, '$.**."Segment Info"') #>> '{}')::jsonb AS value
    FROM plan
)
SELECT
    (jsonb_path_query_first(value, '$.**.exact_scan_ns') #>> '{}')::bigint > 0
        AS uncalibrated_uses_ivf_exact,
    jsonb_path_query_first(value, '$.**.layer0_scored') IS NULL
        AS uncalibrated_skips_quantized_layers
FROM segment_info;
SET client_min_messages = WARNING;
SET paradedb.vector_cluster_max_probe = 1.0;

-- The SQL boundary rejects NULLs itself (the function is intentionally not
-- STRICT), validates every array element, and measures a bounded prefix.
SELECT * FROM paradedb.vector_calibrate(NULL, 'vec', ARRAY[quant_fixture_vector(768, 0)]);
SELECT * FROM paradedb.vector_calibrate('q_cosine_idx', NULL, ARRAY[quant_fixture_vector(768, 0)]);
SELECT * FROM paradedb.vector_calibrate('q_cosine_idx', 'vec', NULL);
SELECT * FROM paradedb.vector_calibrate('q_cosine_idx', 'vec', ARRAY[]::vector[]);
SELECT * FROM paradedb.vector_calibrate('q_cosine_idx', 'vec', ARRAY[NULL::vector]);
SELECT * FROM paradedb.vector_calibrate(
    'q_cosine_idx',
    'vec',
    ARRAY[quant_fixture_vector(100, 0)]
);

SELECT
    array_agg(depth ORDER BY depth) = ARRAY[1, 2] AS cosine_depths,
    bool_and(source = 'real_query') AS cosine_real_query_source,
    bool_and(sample_count > 0) AS cosine_has_samples
FROM paradedb.vector_calibrate(
    'q_cosine_idx',
    'vec',
    ARRAY[
        quant_fixture_vector(768, 0),
        quant_fixture_vector(768, 1),
        quant_fixture_vector(768, 2),
        quant_fixture_vector(768, 3)
    ]
);

CREATE TEMP TABLE q_cosine_quantized AS
SELECT array_agg(id) AS ids
FROM (
    SELECT id
    FROM q_cosine
    WHERE id @@@ pdb.all()
    ORDER BY vec <=> quant_fixture_vector(768, 0), id
    LIMIT 10
) hits;

WITH plan AS (
    SELECT quant_explain(
        'SELECT id FROM q_cosine WHERE id @@@ pdb.all() '
        'ORDER BY vec <=> quant_fixture_vector(768, 0), id LIMIT 10'
    ) AS value
), segment_info AS (
    -- Custom EXPLAIN properties are represented as JSON text inside FORMAT
    -- JSON, so parse that property before checking the flat ProbeStats keys.
    SELECT (jsonb_path_query_first(value, '$.**."Segment Info"') #>> '{}')::jsonb AS value
    FROM plan
)
SELECT
    (jsonb_path_query_first(value, '$.**.layer0_scored') #>> '{}')::bigint > 0
        AS layer0_populated,
    (jsonb_path_query_first(value, '$.**.layer0_survivors') #>> '{}')::bigint
        < (jsonb_path_query_first(value, '$.**.layer0_scored') #>> '{}')::bigint
        AS layer0_filtered,
    (jsonb_path_query_first(value, '$.**.rerank_rows') #>> '{}')::bigint > 0
        AS rerank_populated,
    (jsonb_path_query_first(value, '$.**.routing_visited_count') #>> '{}')::bigint > 0
        AS routing_flat,
    jsonb_path_query_first(value, '$.**.bound_armed_count') IS NOT NULL
        AS bound_flat,
    jsonb_typeof(jsonb_path_query_first(value, '$.**.buffer_hits')) = 'number'
        AND jsonb_typeof(jsonb_path_query_first(value, '$.**.buffer_reads')) = 'number'
        AS io_flat,
    (jsonb_path_query_first(value, '$.**.rerank_blocks_fetched') #>> '{}')::bigint > 0
        AS rerank_io_flat
FROM segment_info;

-- d=768 L2 exercises slot 14 and score assembly as -dist^2.
CREATE TABLE q_l2 (id integer PRIMARY KEY, vec vector(768));
CREATE INDEX q_l2_idx ON q_l2
USING paradedb (id, vec vector_l2_ops)
WITH (
    key_field = id,
    vector_fields = '{"vec":{"dims":768,"quantization":{"layers":[1,4]}}}',
    target_segment_count = 1,
    mutable_segment_rows = 0,
    layer_sizes = '400kb',
    background_layer_sizes = '0'
);
INSERT INTO q_l2 SELECT g, quant_fixture_vector(768, g) FROM generate_series(1, 100) g;
INSERT INTO q_l2 SELECT g, quant_fixture_vector(768, g) FROM generate_series(101, 200) g;
VACUUM q_l2;

SELECT bool_or(vector_format = 'ivf') AS l2_has_ivf
FROM paradedb.vector_info('q_l2_idx', 'vec');

SELECT
    array_agg(depth ORDER BY depth) = ARRAY[1, 2] AS l2_depths,
    bool_and(source = 'real_query') AS l2_real_query_source,
    bool_and(sample_count > 0) AS l2_has_samples
FROM paradedb.vector_calibrate(
    'q_l2_idx',
    'vec',
    ARRAY[
        quant_fixture_vector(768, 0),
        quant_fixture_vector(768, 1),
        quant_fixture_vector(768, 2),
        quant_fixture_vector(768, 3)
    ]
);

CREATE TEMP TABLE q_l2_quantized AS
SELECT array_agg(id) AS ids
FROM (
    SELECT id
    FROM q_l2
    WHERE id @@@ pdb.all()
    ORDER BY vec <-> quant_fixture_vector(768, 0), id
    LIMIT 10
) hits;

-- Odd d=100 exercises exact-d rotation and zero-tail packing through SQL.
CREATE TABLE q_odd (id integer PRIMARY KEY, vec vector(100));
CREATE INDEX q_odd_idx ON q_odd
USING paradedb (id, vec vector_l2_ops)
WITH (
    key_field = id,
    vector_fields = '{"vec":{"dims":100,"quantization":{"layers":[1,4]}}}',
    target_segment_count = 1,
    mutable_segment_rows = 0,
    layer_sizes = '50kb',
    background_layer_sizes = '0'
);
INSERT INTO q_odd SELECT g, quant_fixture_vector(100, g) FROM generate_series(1, 100) g;
INSERT INTO q_odd SELECT g, quant_fixture_vector(100, g) FROM generate_series(101, 200) g;
VACUUM q_odd;

SELECT bool_or(vector_format = 'ivf') AS odd_has_ivf
FROM paradedb.vector_info('q_odd_idx', 'vec');

SELECT
    array_agg(depth ORDER BY depth) = ARRAY[1, 2] AS odd_depths,
    bool_and(source = 'real_query') AS odd_real_query_source,
    bool_and(sample_count > 0) AS odd_has_samples
FROM paradedb.vector_calibrate(
    'q_odd_idx',
    'vec',
    ARRAY[
        quant_fixture_vector(100, 0),
        quant_fixture_vector(100, 1),
        quant_fixture_vector(100, 2),
        quant_fixture_vector(100, 3)
    ]
);

CREATE TEMP TABLE q_odd_quantized AS
SELECT array_agg(id) AS ids
FROM (
    SELECT id
    FROM q_odd
    WHERE id @@@ pdb.all()
    ORDER BY vec <-> quant_fixture_vector(100, 0), id
    LIMIT 10
) hits;

-- A sub-threshold segment remains flat. Level zero must retain this existing
-- brute-force behavior while IVF segments keep routing and the probe budget.
CREATE TABLE q_flat (id integer PRIMARY KEY, vec vector(100));
CREATE INDEX q_flat_idx ON q_flat
USING paradedb (id, vec vector_cosine_ops)
WITH (
    key_field = id,
    vector_fields = '{"vec":{"dims":100,"quantization":true}}'
);
INSERT INTO q_flat SELECT g, quant_fixture_vector(100, g) FROM generate_series(1, 32) g;

-- Prefix depth zero disables quantized scoring but keeps the same IVF routing
-- and work budget. Use the same fractional probe as the saved uncalibrated
-- baseline so order, ids, and exact scores can be compared directly.
SET paradedb.max_scan_levels = 0;
SET paradedb.vector_cluster_max_probe = 0.25;

CREATE TEMP TABLE q_cosine_lvl0 AS
SELECT
    row_number() OVER (ORDER BY distance, id) AS ordinal,
    id,
    distance
FROM (
    SELECT id, vec <=> quant_fixture_vector(768, 0) AS distance
    FROM q_cosine
    WHERE id @@@ pdb.all()
    ORDER BY vec <=> quant_fixture_vector(768, 0), id
    LIMIT 10
) hits;

SELECT
    count(*) = 10
        AND bool_and(baseline.id IS NOT DISTINCT FROM lvl0.id)
        AND bool_and(baseline.distance IS NOT DISTINCT FROM lvl0.distance)
        AS lvl0_matches_uncalibrated_ivf
FROM q_cosine_unquantized_ivf baseline
FULL JOIN q_cosine_lvl0 lvl0 USING (ordinal);

WITH plan AS (
    SELECT quant_explain(
        'SELECT id FROM q_cosine WHERE id @@@ pdb.all() '
        'ORDER BY vec <=> quant_fixture_vector(768, 0), id LIMIT 10'
    ) AS value
), segment_info AS (
    SELECT (jsonb_path_query_first(value, '$.**."Segment Info"') #>> '{}')::jsonb AS value
    FROM plan
)
SELECT
    (jsonb_path_query_first(value, '$.**.routing_visited_count') #>> '{}')::bigint > 0
        AS lvl0_routing_populated,
    (jsonb_path_query_first(value, '$.**.postings_row') #>> '{}')::bigint > 0
        AS lvl0_postings_populated,
    (jsonb_path_query_first(value, '$.**.candidates_scored') #>> '{}')::bigint > 0
        AND (jsonb_path_query_first(value, '$.**.candidates_scored') #>> '{}')::bigint
            < (jsonb_path_query_first(value, '$.**.segment_rows') #>> '{}')::bigint
        AS lvl0_scores_nonexhaustive_rows,
    (jsonb_path_query_first(value, '$.**.exact_scan_ns') #>> '{}')::bigint > 0
        AS lvl0_uses_exact_scoring,
    jsonb_path_query_first(value, '$.**.layer0_scored') IS NULL
        AS lvl0_has_no_layer_fields
FROM segment_info;

WITH plan AS (
    SELECT quant_explain(
        'SELECT id FROM q_flat WHERE id @@@ pdb.all() '
        'ORDER BY vec <=> quant_fixture_vector(100, 0), id LIMIT 10'
    ) AS value
), segment_info AS (
    SELECT (jsonb_path_query_first(value, '$.**."Segment Info"') #>> '{}')::jsonb AS value
    FROM plan
)
SELECT
    (jsonb_path_query_first(value, '$.**.exact_rows_read') #>> '{}')::bigint = 32
        AS flat_reads_every_row,
    (jsonb_path_query_first(value, '$.**.routing_visited_count') #>> '{}')::bigint = 0
        AND (jsonb_path_query_first(value, '$.**.postings_row') #>> '{}')::bigint = 0
        AS flat_skips_ivf_routing,
    (jsonb_path_query_first(value, '$.**.exact_scan_ns') #>> '{}')::bigint > 0
        AS flat_uses_exact_scoring,
    jsonb_path_query_first(value, '$.**.layer0_scored') IS NULL
        AS flat_has_no_layer_fields
FROM segment_info;

RESET paradedb.max_scan_levels;
RESET paradedb.vector_cluster_max_probe;
RESET paradedb.vector_clustering_threshold;

DROP TABLE q_cosine;
DROP TABLE q_l2;
DROP TABLE q_odd;
DROP TABLE q_flat;
DROP FUNCTION quant_explain(text);
DROP FUNCTION quant_fixture_vector(integer, integer);
