CREATE EXTENSION IF NOT EXISTS pg_search;

SET max_parallel_workers_per_gather = 0;
SET enable_indexscan = off;

CREATE TABLE segment_pruning_items (
    id bigint PRIMARY KEY,
    body text NOT NULL,
    price bigint NOT NULL,
    nullable_price bigint
);

CREATE INDEX segment_pruning_items_idx ON segment_pruning_items
USING paradedb (id, body, price, nullable_price)
WITH (target_segment_count = 8, background_layer_sizes = '0');

SET paradedb.global_mutable_segment_rows = 0;
INSERT INTO segment_pruning_items
SELECT g, 'common alpha ' || g, g, CASE WHEN g % 5 = 0 THEN NULL ELSE g END
FROM generate_series(1, 16) g;
INSERT INTO segment_pruning_items
SELECT g, 'common beta ' || g, g, CASE WHEN g % 5 = 0 THEN NULL ELSE g END
FROM generate_series(101, 116) g;
INSERT INTO segment_pruning_items
SELECT g, 'common alpha ' || g, g, CASE WHEN g % 5 = 0 THEN NULL ELSE g END
FROM generate_series(201, 216) g;
INSERT INTO segment_pruning_items
SELECT g, 'common gamma ' || g, g, CASE WHEN g % 5 = 0 THEN NULL ELSE g END
FROM generate_series(301, 316) g;
RESET paradedb.global_mutable_segment_rows;

CREATE FUNCTION segment_pruning_explain_analyze_lines(q text) RETURNS SETOF text AS $$
DECLARE r record;
BEGIN
  FOR r IN EXECUTE 'EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF) ' || q LOOP
    RETURN NEXT r."QUERY PLAN";
  END LOOP;
END $$ LANGUAGE plpgsql;

SELECT count(*) = 4 AS has_four_segments,
       sum(num_docs) = 64 AS has_all_docs
FROM paradedb.index_info('segment_pruning_items_idx');

-- A narrow range intersects one segment; a gap between segment bounds intersects none.
SELECT array_agg(id ORDER BY id) AS one_segment
FROM segment_pruning_items
WHERE id @@@ pdb.all() AND price BETWEEN 104 AND 106;

SELECT count(*) AS gap_count
FROM segment_pruning_items
WHERE id @@@ pdb.all() AND price = 150;

-- NULL prevents a matches-all guarantee but must not change SQL semantics.
SELECT array_agg(id ORDER BY id) AS nullable_range
FROM segment_pruning_items
WHERE id @@@ pdb.all() AND nullable_price BETWEEN 101 AND 106;

-- Prove the parallel execution mode independently from result parity: the plan must be a Gather
-- and EXPLAIN ANALYZE must report at least one launched worker.
SET paradedb.planner_warnings = off;
CREATE TEMP TABLE segment_pruning_serial_base AS
SELECT id
FROM segment_pruning_items
WHERE body @@@ 'common' AND price < 250;

SET max_parallel_workers_per_gather = 2;
SET max_parallel_workers = 8;
SET min_parallel_table_scan_size = 0;
SET parallel_setup_cost = 0;
SET parallel_tuple_cost = 0;
SET paradedb.min_rows_per_worker = 0;

CREATE TEMP TABLE segment_pruning_parallel_base_plan AS
SELECT line
FROM segment_pruning_explain_analyze_lines(
    $$SELECT id
      FROM segment_pruning_items
      WHERE body @@@ 'common' AND price < 250$$
) AS line;

COPY (
    SELECT format(
        'parallel_base_plan used_gather=%s workers_launched=%s',
        EXISTS (
            SELECT 1 FROM segment_pruning_parallel_base_plan WHERE line LIKE '%Gather%'
        ),
        EXISTS (
            SELECT 1 FROM segment_pruning_parallel_base_plan
            WHERE line ~ 'Workers Launched: [1-9][0-9]*'
        )
    )
) TO STDOUT;

CREATE TEMP TABLE segment_pruning_parallel_base AS
SELECT id
FROM segment_pruning_items
WHERE body @@@ 'common' AND price < 250;

COPY (
    SELECT format(
        'parallel_base_rows_match=%s',
        NOT EXISTS (
            (SELECT id FROM segment_pruning_serial_base
             EXCEPT ALL
             SELECT id FROM segment_pruning_parallel_base)
            UNION ALL
            (SELECT id FROM segment_pruning_parallel_base
             EXCEPT ALL
             SELECT id FROM segment_pruning_serial_base)
        )
    )
) TO STDOUT;

RESET paradedb.min_rows_per_worker;
RESET parallel_tuple_cost;
RESET parallel_setup_cost;
RESET min_parallel_table_scan_size;
RESET max_parallel_workers;
SET max_parallel_workers_per_gather = 0;
RESET paradedb.planner_warnings;

-- A generic prepared plan cannot carry planner-time SegmentIds. Execution resolves the bounds
-- and rebuilds the proof after a new segment appears.
SET plan_cache_mode = force_generic_plan;
PREPARE segment_pruning_range(bigint, bigint) AS
SELECT array_agg(id ORDER BY id)
FROM segment_pruning_items
WHERE id @@@ pdb.all() AND price BETWEEN $1 AND $2;

EXECUTE segment_pruning_range(104, 106);

SET paradedb.global_mutable_segment_rows = 0;
INSERT INTO segment_pruning_items
SELECT g, 'late common ' || g, g, g FROM generate_series(401, 404) g;
RESET paradedb.global_mutable_segment_rows;

EXECUTE segment_pruning_range(401, 404);

-- The hash-join InList appears only after planning. Check it against each segment's statistics
-- to reject unrelated segments whether query pushdown chooses Query, Keep, or Skip.
CREATE TABLE segment_pruning_keys (id bigint PRIMARY KEY, price bigint NOT NULL, body text NOT NULL);
INSERT INTO segment_pruning_keys VALUES
    (104, 104, 'wanted'), (105, 105, 'wanted'), (106, 106, 'wanted');
CREATE INDEX segment_pruning_keys_idx ON segment_pruning_keys
USING paradedb (id, price, body);
ANALYZE segment_pruning_items;
ANALYZE segment_pruning_keys;

SET enable_nestloop = off;
SET enable_mergejoin = off;
SET paradedb.enable_join_custom_scan = on;

-- At the default density gate this small fixture takes InListPushdown::Skip. The join remains
-- the membership authority, and the plan must not report a dynamic Tantivy pushdown.
CREATE TEMP TABLE segment_pruning_skip_plan AS
SELECT line
FROM segment_pruning_explain_analyze_lines(
    $$SELECT i.id
      FROM segment_pruning_keys k
      JOIN segment_pruning_items i ON k.id = i.id
      WHERE k.body @@@ 'wanted'
      ORDER BY i.id
      LIMIT 10$$
) AS line;

SELECT count(*) = 0 AS skip_avoids_query_pushdown
FROM segment_pruning_skip_plan
WHERE line LIKE '%dynamic_filter_pushdown_%';

-- The skipped membership predicate still reaches the segment proof through its original
-- dynamic source, and the join publishes it before the first probe batch. `price < 250`
-- already proves segment 301..316 impossible statically, so the metric must report the two
-- segments the scan really skipped, not the three-segment rejection set.
CREATE TEMP TABLE segment_pruning_skip_metric_plan AS
SELECT line
FROM segment_pruning_explain_analyze_lines(
    $$SELECT i.id
      FROM segment_pruning_keys k
      JOIN segment_pruning_items i ON k.id = i.id
      WHERE k.body @@@ 'wanted' AND i.id @@@ pdb.all() AND i.price < 250
      ORDER BY i.id
      LIMIT 10$$
) AS line;
COPY (
    SELECT format(
        'skip_metric_counts_only_skipped_candidates=%s',
        EXISTS (
            SELECT 1 FROM segment_pruning_skip_metric_plan
            WHERE line LIKE '%PgSearchScan: table=i, segments=3,%'
              AND line ~ 'segments_pruned_dynamic_range=(\{0:)?2[,}\]]'
        )
    )
) TO STDOUT;

SELECT array_agg(i.id ORDER BY i.id) AS skip_selected
FROM segment_pruning_keys k
JOIN segment_pruning_items i ON k.id = i.id
WHERE k.body @@@ 'wanted' AND i.id @@@ pdb.all() AND i.price < 250 \gset
\echo skip_selected=:skip_selected

-- The fixture intentionally has small segments. Raise only the conversion gate so this test
-- exercises the successful InList pushdown outcome rather than the separately-safe Skip outcome.
SET paradedb.term_set_bitset_max_density_multi = 1.0;
CREATE TEMP TABLE segment_pruning_pushdown_plan AS
SELECT line
FROM segment_pruning_explain_analyze_lines(
    $$SELECT i.id
      FROM segment_pruning_keys k
      JOIN segment_pruning_items i ON k.id = i.id
      WHERE k.body @@@ 'wanted'
      ORDER BY i.id
      LIMIT 10$$
) AS line;

SELECT count(*) = 1 AS pushdown_reaches_tantivy
FROM segment_pruning_pushdown_plan
WHERE line LIKE '%PgSearchScan: table=i,%'
  AND line LIKE '%dynamic_filter_pushdown_linear=1%';

SELECT array_agg(i.id ORDER BY i.id) AS dynamically_selected
FROM segment_pruning_keys k
JOIN segment_pruning_items i ON k.id = i.id
WHERE k.body @@@ 'wanted';

-- DataFusion 55 publishes both equijoin keys. Check both fields against the segment's statistics.
SELECT array_agg(i.id ORDER BY i.id) AS multi_key_dynamically_selected
FROM segment_pruning_keys k
JOIN segment_pruning_items i ON k.id = i.id AND k.price = i.price
WHERE k.body @@@ 'wanted';

-- A segment without persisted stats must fail open, and later UPDATE/DELETE generations must be
-- resolved by each execution rather than by the generic plan's original manifest.
SET paradedb.planner_warnings = off;
SET paradedb.global_mutable_segment_rows = 10000;
INSERT INTO segment_pruning_items
SELECT g, 'mutable common ' || g, g, g FROM generate_series(501, 504) g;
RESET paradedb.global_mutable_segment_rows;

EXECUTE segment_pruning_range(501, 504) \gset
\echo :array_agg
UPDATE segment_pruning_items SET price = 550 WHERE id = 501;
EXECUTE segment_pruning_range(501, 504) \gset
\echo :array_agg
EXECUTE segment_pruning_range(550, 550) \gset
\echo :array_agg
DELETE FROM segment_pruning_items WHERE id = 502;
EXECUTE segment_pruning_range(501, 504) \gset
\echo :array_agg

-- One parameterized BaseScan is rescanned for disjoint outer values. Every rescan must rebuild
-- its candidate decisions instead of accumulating or reusing the preceding outer row's candidates.
COPY (
    SELECT array_agg(format('%s:%s', wanted.lo, hit.ids) ORDER BY wanted.lo)
    FROM (VALUES (104::bigint), (204::bigint), (304::bigint)) AS wanted(lo)
    CROSS JOIN LATERAL (
        SELECT array_agg(id ORDER BY id) AS ids
        FROM segment_pruning_items
        WHERE id @@@ pdb.all() AND price BETWEEN wanted.lo AND wanted.lo + 2
        OFFSET 0
    ) AS hit
) TO STDOUT;

RESET paradedb.planner_warnings;

-- Prove that the real JoinScan Top-K publishes a bound. The focused pg_test separately proves
-- that installing a tighter dynamic range abandons an active scorer and avoids deferred scorers.
RESET paradedb.term_set_bitset_max_density_multi;
RESET plan_cache_mode;
CREATE TABLE segment_pruning_topk_items (
    id bigint PRIMARY KEY,
    rank bigint NOT NULL,
    body text NOT NULL
);
CREATE TABLE segment_pruning_topk_keys (id bigint PRIMARY KEY, body text NOT NULL);
CREATE INDEX segment_pruning_topk_items_idx ON segment_pruning_topk_items
USING paradedb (id, rank, (body::pdb.unicode_words('columnar=true')))
WITH (target_segment_count = 8, background_layer_sizes = '0');
CREATE INDEX segment_pruning_topk_keys_idx ON segment_pruning_topk_keys
USING paradedb (id, (body::pdb.unicode_words('columnar=true')));

SET paradedb.global_mutable_segment_rows = 0;
INSERT INTO segment_pruning_topk_items SELECT g, g, 'silver' FROM generate_series(1, 10000) g;
INSERT INTO segment_pruning_topk_items SELECT g, g, 'silver' FROM generate_series(10001, 20000) g;
INSERT INTO segment_pruning_topk_items SELECT g, g, 'silver' FROM generate_series(20001, 30000) g;
INSERT INTO segment_pruning_topk_items SELECT g, g, 'silver' FROM generate_series(30001, 40000) g;
RESET paradedb.global_mutable_segment_rows;
INSERT INTO segment_pruning_topk_keys SELECT g, 'wanted' FROM generate_series(1, 40000) g;
ANALYZE segment_pruning_topk_items;
ANALYZE segment_pruning_topk_keys;

SET paradedb.dynamic_filter_batch_size = 64;
CREATE TEMP TABLE segment_pruning_topk_plan AS
SELECT line
FROM segment_pruning_explain_analyze_lines(
    $$SELECT i.id
      FROM segment_pruning_topk_items i
      JOIN segment_pruning_topk_keys k ON k.id = i.id
      WHERE k.body @@@ 'wanted' AND i.body @@@ 'silver'
      ORDER BY i.rank
      LIMIT 10$$
) AS line;

COPY (
    SELECT count(*) = 1
    FROM segment_pruning_topk_plan
    WHERE line LIKE '%SortExec: TopK%'
      AND line ~ 'filter=\[rank@[0-9]+ < 10\]'
) TO STDOUT;

COPY (
    SELECT array_agg(id ORDER BY id)
    FROM (
        SELECT i.id
        FROM segment_pruning_topk_items i
        JOIN segment_pruning_topk_keys k ON k.id = i.id
        WHERE k.body @@@ 'wanted' AND i.body @@@ 'silver'
        ORDER BY i.rank
        LIMIT 10
    ) selected
) TO STDOUT;

DEALLOCATE segment_pruning_range;
DROP FUNCTION segment_pruning_explain_analyze_lines(text);

RESET plan_cache_mode;
RESET paradedb.term_set_bitset_max_density_multi;
RESET paradedb.dynamic_filter_batch_size;
RESET paradedb.enable_join_custom_scan;
RESET enable_mergejoin;
RESET enable_nestloop;
RESET enable_indexscan;
RESET max_parallel_workers_per_gather;
DROP TABLE segment_pruning_topk_keys CASCADE;
DROP TABLE segment_pruning_topk_items CASCADE;
DROP TABLE segment_pruning_keys CASCADE;
DROP TABLE segment_pruning_items CASCADE;
