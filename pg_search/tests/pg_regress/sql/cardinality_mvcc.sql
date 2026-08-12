-- Tests that cardinality aggregations remain MVCC-correct in the presence of
-- deleted (unvacuumed) rows, and that solve_mvcc=false overshoots by counting
-- values that only exist on dead rows.
\echo 'Test: cardinality MVCC correctness with deletes'

CREATE EXTENSION IF NOT EXISTS pg_search;
SET paradedb.check_aggregate_scan = false;

DROP TABLE IF EXISTS card_mvcc;
CREATE TABLE card_mvcc (
    id serial8,
    val uuid NOT NULL
) WITH (autovacuum_enabled = off);

-- 10k rows over 12 distinct deterministic uuids
INSERT INTO card_mvcc (val)
SELECT md5('cardtest' || ((x % 12) + 1))::uuid
FROM generate_series(1, 10000) x;

CREATE INDEX idx_card_mvcc ON card_mvcc
USING bm25 (id, val) WITH (key_field = 'id');

-- before any deletes: 12 either way
SELECT pdb.agg('{"cardinality": {"field": "val"}}'::jsonb) FROM card_mvcc WHERE card_mvcc @@@ pdb.all();
SELECT pdb.agg('{"cardinality": {"field": "val"}}'::jsonb, false) FROM card_mvcc WHERE card_mvcc @@@ pdb.all();

-- delete every row of one value, plus scattered rows of the others; no vacuum,
-- so the index still contains all 12 values
DELETE FROM card_mvcc WHERE val = md5('cardtest' || 12)::uuid;
DELETE FROM card_mvcc WHERE id % 10 = 0;
SELECT count(DISTINCT val) AS true_cardinality FROM card_mvcc;

-- MVCC-correct: the fully-deleted value is not counted. A value whose first
-- occurrences are dead but which still has live rows must be counted.
SELECT pdb.agg('{"cardinality": {"field": "val"}}'::jsonb) FROM card_mvcc WHERE card_mvcc @@@ pdb.all();

-- solve_mvcc=false counts raw index entries: overshoots to 12
SELECT pdb.agg('{"cardinality": {"field": "val"}}'::jsonb, false) FROM card_mvcc WHERE card_mvcc @@@ pdb.all();

-- facet form: cardinality as a window aggregate alongside top-k. Same MVCC
-- semantics: correct with solve_mvcc, overshoots without.
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
