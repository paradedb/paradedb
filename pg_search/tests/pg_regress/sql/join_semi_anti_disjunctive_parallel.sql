-- Parallel regression for disjunctive Semi/Anti JoinScan (issue #4776).
--
-- `validate_and_build_clause` forces `partitioning_source_index = 0` for any
-- plan that contains a Semi/Anti join (build::JoinCSClause::
-- `with_forced_partitioning(0)`), and disjunctive Semi/Anti conditions now
-- lower to `NestedLoopJoinExec` instead of `HashJoinExec`. The combination
-- must still produce correct results under parallel execution: the outer
-- (partitioned) side sees a disjoint slice per worker, the inner
-- (replicated) side is fully scanned by every worker, and the anti/semi
-- filter fires per (left, right) row pair.
--
-- Regression guard: we cross-check a parallel disjunctive NOT EXISTS /
-- EXISTS against the JoinScan-off PG-native plan on the same data.
-- Any dropped rows, duplicates, or missed matches from worker
-- partitioning would surface as a set diff.

CREATE EXTENSION IF NOT EXISTS pg_search;

SET paradedb.enable_custom_scan = on;
SET paradedb.enable_join_custom_scan = on;
SET paradedb.min_rows_per_worker = 0;
SET max_parallel_workers = 4;
SET max_parallel_workers_per_gather = 4;
SET parallel_tuple_cost = 0;
SET parallel_setup_cost = 0;
SET min_parallel_table_scan_size = 0;
SET min_parallel_index_scan_size = 0;
SET parallel_leader_participation = off;

DROP TABLE IF EXISTS jsd_par_items CASCADE;
DROP TABLE IF EXISTS jsd_par_exclusions CASCADE;

CREATE TABLE jsd_par_items (
    id bigint PRIMARY KEY,
    name text,
    alt_name text,
    category text
) WITH (autovacuum_enabled = false);

CREATE TABLE jsd_par_exclusions (
    id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    pattern text
) WITH (autovacuum_enabled = false);

-- Segment-per-batch index to force multiple segments and give workers
-- something to partition across. `global_mutable_segment_rows = 0` flushes
-- after each INSERT; `target_segment_count` + `background_layer_sizes`
-- keeps background merges from collapsing segments back together.
SET paradedb.global_mutable_segment_rows = 0;

CREATE INDEX jsd_par_items_idx ON jsd_par_items
USING bm25 (id, name, alt_name, category)
WITH (
    key_field = 'id',
    text_fields = '{
        "name": {"fast": true, "tokenizer": {"type": "keyword"}},
        "alt_name": {"fast": true, "tokenizer": {"type": "keyword"}},
        "category": {"fast": true, "tokenizer": {"type": "keyword"}, "normalizer": "lowercase"}
    }',
    target_segment_count = 64,
    background_layer_sizes = '0'
);

CREATE INDEX jsd_par_exclusions_idx ON jsd_par_exclusions
USING bm25 (id, pattern)
WITH (
    key_field = 'id',
    text_fields = '{
        "pattern": {"fast": true, "tokenizer": {"type": "keyword"}}
    }',
    target_segment_count = 64,
    background_layer_sizes = '0'
);

-- Batched inserts → multiple segments on the partitioning side. Each
-- 1000-row batch triggers a segment flush under
-- `global_mutable_segment_rows = 0`.
INSERT INTO jsd_par_items SELECT i, 'name_' || i,
    CASE WHEN i % 3 = 0 THEN 'alt_' || i ELSE NULL END,
    CASE WHEN i % 2 = 0 THEN 'target' ELSE 'other' END
FROM generate_series(1, 1000) i;
INSERT INTO jsd_par_items SELECT i, 'name_' || i,
    CASE WHEN i % 3 = 0 THEN 'alt_' || i ELSE NULL END,
    CASE WHEN i % 2 = 0 THEN 'target' ELSE 'other' END
FROM generate_series(1001, 2000) i;
INSERT INTO jsd_par_items SELECT i, 'name_' || i,
    CASE WHEN i % 3 = 0 THEN 'alt_' || i ELSE NULL END,
    CASE WHEN i % 2 = 0 THEN 'target' ELSE 'other' END
FROM generate_series(2001, 3000) i;
INSERT INTO jsd_par_items SELECT i, 'name_' || i,
    CASE WHEN i % 3 = 0 THEN 'alt_' || i ELSE NULL END,
    CASE WHEN i % 2 = 0 THEN 'target' ELSE 'other' END
FROM generate_series(3001, 4000) i;

INSERT INTO jsd_par_exclusions (pattern)
SELECT 'name_' || i FROM generate_series(1, 2000) i WHERE i % 7 = 0;
INSERT INTO jsd_par_exclusions (pattern)
SELECT 'alt_' || i FROM generate_series(1, 4000) i WHERE i % 3 = 0 AND i % 11 = 0;

RESET paradedb.global_mutable_segment_rows;
ANALYZE jsd_par_items;
ANALYZE jsd_par_exclusions;

-- =====================================================================
-- 1. Parallel NOT EXISTS with 2-arm disjunctive OR
-- =====================================================================
-- Capture JoinScan-absorbed (parallel) result into a temp, then compare
-- against the JoinScan-off (native Nested Loop Anti) result on the
-- same session. Set-equal + row-count equal means no dropped/duplicated
-- rows from parallel partitioning.
-- LIMIT is required by `validate_and_build_clause` to activate JoinScan
-- on top-level queries; pick a bound that comfortably exceeds the
-- ~1802-row result set so no rows are dropped. The matching LIMIT on
-- the JoinScan-off side keeps the comparison apples-to-apples.
CREATE TEMP TABLE jsd_par_r1_on AS
SELECT i.id
FROM jsd_par_items i
WHERE NOT EXISTS (
    SELECT 1 FROM jsd_par_exclusions e
    WHERE e.id @@@ paradedb.all()
      AND (e.pattern = i.name OR e.pattern = i.alt_name)
)
AND i.id @@@ 'category:"target"'
ORDER BY i.id DESC
LIMIT 5000;

SET paradedb.enable_join_custom_scan = off;

CREATE TEMP TABLE jsd_par_r1_off AS
SELECT i.id
FROM jsd_par_items i
WHERE NOT EXISTS (
    SELECT 1 FROM jsd_par_exclusions e
    WHERE e.id @@@ paradedb.all()
      AND (e.pattern = i.name OR e.pattern = i.alt_name)
)
AND i.id @@@ 'category:"target"'
ORDER BY i.id DESC
LIMIT 5000;

SET paradedb.enable_join_custom_scan = on;

-- Row counts must match.
SELECT
    (SELECT count(*) FROM jsd_par_r1_on)  AS joinscan_rows,
    (SELECT count(*) FROM jsd_par_r1_off) AS native_rows,
    (SELECT count(*) FROM jsd_par_r1_on)
      = (SELECT count(*) FROM jsd_par_r1_off) AS row_counts_match;

-- Symmetric difference must be empty in both directions.
SELECT count(*) AS joinscan_only FROM (
    SELECT id FROM jsd_par_r1_on EXCEPT SELECT id FROM jsd_par_r1_off
) d;
SELECT count(*) AS native_only FROM (
    SELECT id FROM jsd_par_r1_off EXCEPT SELECT id FROM jsd_par_r1_on
) d;

-- Spot-check the top-10 descending for visible correctness.
SELECT id FROM jsd_par_r1_on ORDER BY id DESC LIMIT 10;

DROP TABLE jsd_par_r1_on;
DROP TABLE jsd_par_r1_off;

-- =====================================================================
-- 2. Parallel EXISTS with 3-arm disjunctive OR
-- =====================================================================
-- LIMIT well above the ~198-row result set so both sides capture the
-- full set for the EXCEPT comparison.
CREATE TEMP TABLE jsd_par_r2_on AS
SELECT i.id
FROM jsd_par_items i
WHERE EXISTS (
    SELECT 1 FROM jsd_par_exclusions e
    WHERE e.id @@@ paradedb.all()
      AND (
          e.pattern = i.name
          OR e.pattern = i.alt_name
          OR e.pattern = i.category
      )
)
AND i.id @@@ 'category:"target"'
ORDER BY i.id ASC
LIMIT 5000;

SET paradedb.enable_join_custom_scan = off;

CREATE TEMP TABLE jsd_par_r2_off AS
SELECT i.id
FROM jsd_par_items i
WHERE EXISTS (
    SELECT 1 FROM jsd_par_exclusions e
    WHERE e.id @@@ paradedb.all()
      AND (
          e.pattern = i.name
          OR e.pattern = i.alt_name
          OR e.pattern = i.category
      )
)
AND i.id @@@ 'category:"target"'
ORDER BY i.id ASC
LIMIT 5000;

SET paradedb.enable_join_custom_scan = on;

SELECT
    (SELECT count(*) FROM jsd_par_r2_on)  AS joinscan_rows,
    (SELECT count(*) FROM jsd_par_r2_off) AS native_rows,
    (SELECT count(*) FROM jsd_par_r2_on)
      = (SELECT count(*) FROM jsd_par_r2_off) AS row_counts_match;

SELECT count(*) AS joinscan_only FROM (
    SELECT id FROM jsd_par_r2_on EXCEPT SELECT id FROM jsd_par_r2_off
) d;
SELECT count(*) AS native_only FROM (
    SELECT id FROM jsd_par_r2_off EXCEPT SELECT id FROM jsd_par_r2_on
) d;

SELECT id FROM jsd_par_r2_on ORDER BY id ASC LIMIT 10;

DROP TABLE jsd_par_r2_on;
DROP TABLE jsd_par_r2_off;

-- Cleanup
DROP TABLE jsd_par_items CASCADE;
DROP TABLE jsd_par_exclusions CASCADE;

RESET paradedb.enable_custom_scan;
RESET paradedb.enable_join_custom_scan;
RESET paradedb.min_rows_per_worker;
RESET max_parallel_workers;
RESET max_parallel_workers_per_gather;
RESET parallel_tuple_cost;
RESET parallel_setup_cost;
RESET min_parallel_table_scan_size;
RESET min_parallel_index_scan_size;
RESET parallel_leader_participation;
