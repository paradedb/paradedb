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

CREATE TABLE q_schedule_validation (id integer PRIMARY KEY, vec vector(64));
CREATE INDEX q_too_many_layers_idx ON q_schedule_validation
USING paradedb (id, vec vector_cosine_ops)
WITH (
    key_field = id,
    vector_fields = '{"vec":{"dims":64,"quantization":{"layers":[1,1,1,1]}}}'
);
CREATE INDEX q_grid_first_idx ON q_schedule_validation
USING paradedb (id, vec vector_cosine_ops)
WITH (
    key_field = id,
    vector_fields = '{"vec":{"dims":64,"quantization":{"layers":[4]}}}'
);
DROP TABLE q_schedule_validation;

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

SELECT count(*)
FROM paradedb.vector_error_audit(
    'q_cosine_idx',
    'vec',
    ARRAY(
        SELECT quant_fixture_vector(768, g)
        FROM generate_series(0, 98) g
    )
);

WITH audit AS MATERIALIZED (
    SELECT *
    FROM paradedb.vector_error_audit(
        'q_cosine_idx',
        'vec',
        ARRAY(
            SELECT quant_fixture_vector(768, g)
            FROM generate_series(0, 100) g
        )
    )
)
SELECT
    count(*) = 4 AS exact_e_two_sources_two_depths,
    array_agg(DISTINCT depth ORDER BY depth) = ARRAY[1, 2] AS exact_e_depths,
    array_agg(DISTINCT source ORDER BY source)
        = ARRAY['held_out', 'real_query'] AS exact_e_sources,
    bool_and(
        protocol = CASE source
            WHEN 'held_out' THEN 'HELD_OUT_EXACT_E_BQ4'
            WHEN 'real_query' THEN 'REAL_QUERY_EXACT_E_BQ4'
        END
    ) AS exact_e_protocols,
    bool_and(
        sample_count > 0
        AND residual_norm_squared_sample_count > 0
        AND gamma_sample_count > 0
        AND corrected_error_ratio_sample_count > 0
        AND sigma_sample_count > 0
        AND (gamma_diagnostics->'stored'->>'sample_count')::bigint = gamma_sample_count
        AND (gamma_diagnostics->'raw'->>'sample_count')::bigint > 0
        AND (gamma_diagnostics->'round_trip_band_error'->>'sample_count')::bigint > 0
        AND spread >= 0
    ) AS exact_e_samples,
    bool_and(
        residual_norm_squared_mean >= 0
        AND residual_norm_squared_p95 <= residual_norm_squared_p99
        AND residual_norm_squared_p99 <= residual_norm_squared_max
        AND gamma_p95 <= gamma_p99
        AND gamma_p99 <= gamma_max
        AND (gamma_diagnostics->'stored'->>'min')::double precision
            <= (gamma_diagnostics->'stored'->>'p50')::double precision
        AND (gamma_diagnostics->'stored'->>'p50')::double precision
            <= (gamma_diagnostics->'stored'->>'p95')::double precision
        AND (gamma_diagnostics->'raw'->>'min')::double precision
            <= (gamma_diagnostics->'raw'->>'p50')::double precision
        AND (gamma_diagnostics->'raw'->>'p50')::double precision
            <= (gamma_diagnostics->'raw'->>'p95')::double precision
        AND (gamma_diagnostics->>'zero_scale_count')::bigint BETWEEN 0 AND gamma_sample_count
        AND (gamma_diagnostics->>'lower_clamp_count')::bigint BETWEEN 0 AND gamma_sample_count
        AND (gamma_diagnostics->>'upper_clamp_count')::bigint BETWEEN 0 AND gamma_sample_count
        AND (gamma_diagnostics->'round_trip_band_error'->>'p99_abs')::double precision
            <= (gamma_diagnostics->'round_trip_band_error'->>'max_abs')::double precision
        AND corrected_error_ratio_mean >= 0
        AND corrected_error_ratio_p95 <= corrected_error_ratio_p99
        AND corrected_error_ratio_p99 <= corrected_error_ratio_max
        AND sigma_mean >= 0
        AND sigma_p95 <= sigma_p99
        AND sigma_p99 <= sigma_max
    ) AS exact_e_distributions
FROM audit;

WITH cone AS MATERIALIZED (
    SELECT *
    FROM paradedb.vector_error_cone_audit(
        'q_cosine_idx',
        'vec',
        ARRAY(
            SELECT quant_fixture_vector(768, g)
            FROM generate_series(0, 99) g
        )
    )
)
SELECT
    count(*) = 2 AS cone_two_depths,
    array_agg(depth ORDER BY depth) = ARRAY[1, 2] AS cone_depths,
    array_agg(kappa ORDER BY depth) = ARRAY[2.0::real, 2.0::real] AS cone_kappas,
    bool_and(
        protocol = 'ALL_CLUSTERS_EXACT_E_CONE_K10_KAPPA2'
        AND query_count = 100
        AND top_k = 10
    ) AS cone_protocol,
    bool_and(
        mean_scored_rows >= mean_survivor_rows
        AND mean_survivor_rows >= mean_survivor_docs
        AND mean_survivor_fraction BETWEEN 0.0 AND 1.0
        AND mean_candidate_recall BETWEEN 0.0 AND 1.0
        AND min_candidate_recall BETWEEN 0.0 AND 1.0
        AND queries_with_miss BETWEEN 0 AND query_count
    ) AS cone_ranges,
    count(*) FILTER (WHERE final_depth) = 1 AS cone_one_final_depth
FROM cone;

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
    (jsonb_path_query_first(value, '$.**.layer0_scored') #>> '{}')::bigint > 0
        AS diagnostics_absent_uses_quantized_layers,
    jsonb_path_query_first(value, '$.**.exact_scan_ns') IS NULL
        AS diagnostics_absent_skips_ivf_exact
FROM segment_info;
SET paradedb.vector_cluster_max_probe = 1.0;

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
    bool_and(sample_count > 0) AS cosine_has_samples,
    bool_and(abs(bias) <= 0.3) AS cosine_bias_tripwire
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

CREATE TABLE q_flat (id integer PRIMARY KEY, vec vector(100));
CREATE INDEX q_flat_idx ON q_flat
USING paradedb (id, vec vector_cosine_ops)
WITH (
    key_field = id,
    vector_fields = '{"vec":{"dims":100,"quantization":true}}'
);
INSERT INTO q_flat SELECT g, quant_fixture_vector(100, g) FROM generate_series(1, 32) g;

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
        AS lvl0_matches_unquantized_ivf
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
