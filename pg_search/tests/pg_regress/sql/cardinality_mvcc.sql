-- Tests that cardinality aggregations remain MVCC-correct in the presence of
-- deleted (unvacuumed) rows, and that solve_mvcc=false overshoots by counting
-- values that only exist on dead rows.
\echo 'Test: cardinality MVCC correctness with deletes'

CREATE EXTENSION IF NOT EXISTS pg_search;

DROP TABLE IF EXISTS card_mvcc;
CREATE TABLE card_mvcc (
    id serial8,
    val uuid NOT NULL,
    num int NOT NULL
) WITH (autovacuum_enabled = off);

-- 10k rows over 12 distinct deterministic uuids; num mirrors the uuid group
INSERT INTO card_mvcc (val, num)
SELECT md5('cardtest' || ((x % 12) + 1))::uuid, (x % 12) + 1
FROM generate_series(1, 10000) x;

CREATE INDEX idx_card_mvcc ON card_mvcc
USING bm25 (id, val, num)
WITH (key_field = 'id', numeric_fields = '{"num": {"fast": true}}');

-- the aggregate form runs on the aggregate custom scan
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT pdb.agg('{"cardinality": {"field": "val"}}'::jsonb) FROM card_mvcc WHERE card_mvcc @@@ pdb.all();

-- before any deletes: 12 either way
SELECT pdb.agg('{"cardinality": {"field": "val"}}'::jsonb) FROM card_mvcc WHERE card_mvcc @@@ pdb.all();
SELECT pdb.agg('{"cardinality": {"field": "val"}}'::jsonb, false) FROM card_mvcc WHERE card_mvcc @@@ pdb.all();

-- delete every row of one value, plus scattered rows of the others; no vacuum,
-- so the index still contains all 12 values
DELETE FROM card_mvcc WHERE val = md5('cardtest' || 12)::uuid;
DELETE FROM card_mvcc WHERE id % 10 = 0;
SET paradedb.planner_warnings = 'off'; -- DISTINCT can't use the aggregate scan
SELECT count(DISTINCT val) AS true_cardinality FROM card_mvcc;
RESET paradedb.planner_warnings;

-- MVCC-correct: the fully-deleted value is not counted. A value whose first
-- occurrences are dead but which still has live rows must be counted.
SELECT pdb.agg('{"cardinality": {"field": "val"}}'::jsonb) FROM card_mvcc WHERE card_mvcc @@@ pdb.all();

-- solve_mvcc=false counts raw index entries: overshoots to 12
SELECT pdb.agg('{"cardinality": {"field": "val"}}'::jsonb, false) FROM card_mvcc WHERE card_mvcc @@@ pdb.all();

-- requests ineligible for the in-aggregation cardinality fast path solve MVCC
-- by filtering matched docs instead, and must be equally correct:
-- cardinality alongside a non-cardinality aggregate
SELECT pdb.agg('{"cardinality": {"field": "val"}}'::jsonb) AS card,
       pdb.agg('{"min": {"field": "num"}}'::jsonb) AS min_num
FROM card_mvcc WHERE card_mvcc @@@ pdb.all();

-- cardinality over a numeric field
SELECT pdb.agg('{"cardinality": {"field": "num"}}'::jsonb) FROM card_mvcc WHERE card_mvcc @@@ pdb.all();

-- cardinality as a sub-aggregation; every num group holds exactly one val
SELECT pdb.agg('{"terms": {"field": "num", "size": 3}, "aggs": {"vals": {"cardinality": {"field": "val"}}}}'::jsonb)
FROM card_mvcc WHERE card_mvcc @@@ pdb.all();

-- facet form: cardinality as a window aggregate alongside top-k. Same MVCC
-- semantics: correct with solve_mvcc, overshoots without.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, pdb.agg('{"cardinality": {"field": "val"}}'::jsonb, true) OVER () AS agg
FROM card_mvcc WHERE card_mvcc @@@ pdb.all()
ORDER BY id DESC LIMIT 1 OFFSET 0;

SELECT id, pdb.agg('{"cardinality": {"field": "val"}}'::jsonb, true) OVER () AS agg
FROM card_mvcc WHERE card_mvcc @@@ pdb.all()
ORDER BY id DESC LIMIT 1 OFFSET 0;
SELECT id, pdb.agg('{"cardinality": {"field": "val"}}'::jsonb, false) OVER () AS agg
FROM card_mvcc WHERE card_mvcc @@@ pdb.all()
ORDER BY id DESC LIMIT 1 OFFSET 0;

-- delete everything: MVCC-correct cardinality drops to 0
DELETE FROM card_mvcc;
SELECT pdb.agg('{"cardinality": {"field": "val"}}'::jsonb) FROM card_mvcc WHERE card_mvcc @@@ pdb.all();
SELECT pdb.agg('{"cardinality": {"field": "val"}}'::jsonb, false) FROM card_mvcc WHERE card_mvcc @@@ pdb.all();

DROP TABLE card_mvcc;

-- A value that is dead in one segment but live in another must count exactly
-- once: the fast path confirms visible values per segment.
DROP TABLE IF EXISTS card_mvcc_seg;
CREATE TABLE card_mvcc_seg (
    id serial8,
    val text NOT NULL,
    batch int NOT NULL
) WITH (autovacuum_enabled = off);

-- mutable_segment_rows and the zeroed layer sizes force each insert into its
-- own immutable segment, with no merging
CREATE INDEX idx_card_mvcc_seg ON card_mvcc_seg
USING bm25 (id, val, batch)
WITH (
    key_field = 'id',
    text_fields = '{"val": {"fast": true}}',
    numeric_fields = '{"batch": {"fast": true}}',
    mutable_segment_rows = 2,
    layer_sizes = '0',
    background_layer_sizes = '0'
);

-- two inserts -> separate segments over the same five values
INSERT INTO card_mvcc_seg (val, batch) SELECT 'v' || (x % 5), 1 FROM generate_series(1, 1000) x;
INSERT INTO card_mvcc_seg (val, batch) SELECT 'v' || (x % 5), 2 FROM generate_series(1, 1000) x;
SELECT count(*) >= 2 AS multiple_segments FROM paradedb.index_info('idx_card_mvcc_seg');

-- v0: dead in the first batch's segment, live in the second's. v1: dead everywhere.
DELETE FROM card_mvcc_seg WHERE batch = 1 AND val = 'v0';
DELETE FROM card_mvcc_seg WHERE val = 'v1';
SET paradedb.planner_warnings = 'off'; -- DISTINCT can't use the aggregate scan
SELECT count(DISTINCT val) AS true_cardinality FROM card_mvcc_seg;
RESET paradedb.planner_warnings;

SELECT pdb.agg('{"cardinality": {"field": "val"}}'::jsonb) FROM card_mvcc_seg WHERE card_mvcc_seg @@@ pdb.all();
SELECT pdb.agg('{"cardinality": {"field": "val"}}'::jsonb, false) FROM card_mvcc_seg WHERE card_mvcc_seg @@@ pdb.all();

DROP TABLE card_mvcc_seg;
