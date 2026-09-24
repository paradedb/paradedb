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
WITH (partition_by = 'price,nullable_price', target_segment_count = 8, background_layer_sizes = '0');

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

-- Parallel execution: the plan is a Gather, and the rows match the serial run.
SET paradedb.planner_warnings = off;
SELECT array_agg(id ORDER BY id) AS serial_rows
FROM segment_pruning_items
WHERE body @@@ 'common' AND price < 250;

SET max_parallel_workers_per_gather = 2;
SET max_parallel_workers = 8;
SET min_parallel_table_scan_size = 0;
SET parallel_setup_cost = 0;
SET parallel_tuple_cost = 0;
SET paradedb.min_rows_per_worker = 0;

EXPLAIN (COSTS OFF)
SELECT id
FROM segment_pruning_items
WHERE body @@@ 'common' AND price < 250;

SELECT array_agg(id ORDER BY id) AS parallel_rows
FROM segment_pruning_items
WHERE body @@@ 'common' AND price < 250;

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

-- A segment without persisted stats must fail open, and later UPDATE/DELETE generations must be
-- resolved by each execution rather than by the generic plan's original manifest.
SET paradedb.planner_warnings = off;
SET paradedb.global_mutable_segment_rows = 10000;
INSERT INTO segment_pruning_items
SELECT g, 'mutable common ' || g, g, g FROM generate_series(501, 504) g;
RESET paradedb.global_mutable_segment_rows;

EXECUTE segment_pruning_range(501, 504);
UPDATE segment_pruning_items SET price = 550 WHERE id = 501;
EXECUTE segment_pruning_range(501, 504);
EXECUTE segment_pruning_range(550, 550);
DELETE FROM segment_pruning_items WHERE id = 502;
EXECUTE segment_pruning_range(501, 504);

-- One parameterized BaseScan is rescanned for disjoint outer values. Every rescan must rebuild
-- its candidate decisions instead of accumulating or reusing the preceding outer row's candidates.
SELECT wanted.lo, hit.ids
FROM (VALUES (104::bigint), (204::bigint), (304::bigint)) AS wanted(lo)
CROSS JOIN LATERAL (
    SELECT array_agg(id ORDER BY id) AS ids
    FROM segment_pruning_items
    WHERE id @@@ pdb.all() AND price BETWEEN wanted.lo AND wanted.lo + 2
    OFFSET 0
) AS hit
ORDER BY wanted.lo;

RESET paradedb.planner_warnings;

DEALLOCATE segment_pruning_range;

RESET plan_cache_mode;
RESET enable_indexscan;
RESET max_parallel_workers_per_gather;
DROP TABLE segment_pruning_items CASCADE;
