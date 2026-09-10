-- Bounded Top-N attempt for a join whose ORDER BY belongs to one side.
--
-- The join reads only the leading rows of the ordered side, then keeps that
-- result when it fills the LIMIT. A short result proves nothing, because rows
-- drop in the join, so the query re-runs unbounded.
--
-- What the probe scan reads is the thing to watch. Bounded, it reads about the
-- cap; unbounded, it reads the whole index. Results must not move either way.

SET max_parallel_workers_per_gather = 0;
SET enable_indexscan to OFF;

CREATE EXTENSION IF NOT EXISTS pg_search;

SET paradedb.enable_join_custom_scan = on;

DROP TABLE IF EXISTS btn_probe CASCADE;
DROP TABLE IF EXISTS btn_ordered CASCADE;

-- A `uuid` key is the shape this targets: the key set reaches tantivy as a term
-- set, so the probe side reads only the keys the bound kept.
CREATE TABLE btn_ordered (
    id      uuid PRIMARY KEY,
    grp     bigint      NOT NULL,
    made_at timestamptz NOT NULL
);

INSERT INTO btn_ordered (id, grp, made_at)
SELECT md5('k' || i)::uuid,
       CASE WHEN i <= 4000 THEN 1 ELSE 2 END,
       timestamptz '2024-01-01' + (i || ' seconds')::interval
FROM generate_series(1, 20000) i;

CREATE TABLE btn_probe (
    id        bigserial PRIMARY KEY,
    fk        uuid   NOT NULL,
    amount    bigint NOT NULL
);

INSERT INTO btn_probe (fk, amount)
SELECT md5('k' || i)::uuid, i % 100
FROM generate_series(1, 20000) i;

CREATE INDEX btn_ordered_idx ON btn_ordered
USING bm25 (id, grp, made_at) WITH (key_field = 'id');

CREATE INDEX btn_probe_idx ON btn_probe
USING bm25 (id, fk, amount) WITH (key_field = 'id');

ANALYZE btn_ordered;
ANALYZE btn_probe;

-- =============================================================================
-- TEST 1: the bound holds, so the probe scan stays near the cap
-- =============================================================================

EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)
SELECT o.id
FROM btn_ordered o
JOIN btn_probe p ON p.fk = o.id
WHERE o.id @@@ paradedb.all() AND o.grp = 1 AND p.amount >= 0
ORDER BY o.made_at DESC, o.id DESC
LIMIT 10;

SELECT o.id
FROM btn_ordered o
JOIN btn_probe p ON p.fk = o.id
WHERE o.id @@@ paradedb.all() AND o.grp = 1 AND p.amount >= 0
ORDER BY o.made_at DESC, o.id DESC
LIMIT 10;

-- Same query with the feature off, for the rows and for the contrast in what
-- the probe scan reads.
SET paradedb.enable_join_bounded_topn = off;

EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)
SELECT o.id
FROM btn_ordered o
JOIN btn_probe p ON p.fk = o.id
WHERE o.id @@@ paradedb.all() AND o.grp = 1 AND p.amount >= 0
ORDER BY o.made_at DESC, o.id DESC
LIMIT 10;

SELECT o.id
FROM btn_ordered o
JOIN btn_probe p ON p.fk = o.id
WHERE o.id @@@ paradedb.all() AND o.grp = 1 AND p.amount >= 0
ORDER BY o.made_at DESC, o.id DESC
LIMIT 10;

RESET paradedb.enable_join_bounded_topn;

-- =============================================================================
-- TEST 2: OFFSET, and a sort key Postgres rewrites to the other side
-- =============================================================================

SELECT o.id
FROM btn_ordered o
JOIN btn_probe p ON p.fk = o.id
WHERE o.id @@@ paradedb.all() AND o.grp = 1 AND p.amount >= 0
ORDER BY o.made_at ASC, o.id ASC
LIMIT 5 OFFSET 7;

SET paradedb.enable_join_bounded_topn = off;
SELECT o.id
FROM btn_ordered o
JOIN btn_probe p ON p.fk = o.id
WHERE o.id @@@ paradedb.all() AND o.grp = 1 AND p.amount >= 0
ORDER BY o.made_at ASC, o.id ASC
LIMIT 5 OFFSET 7;
RESET paradedb.enable_join_bounded_topn;

-- =============================================================================
-- TEST 3: too few rows survive the join, so the bounded attempt is discarded
-- =============================================================================

EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)
SELECT o.id
FROM btn_ordered o
JOIN btn_probe p ON p.fk = o.id
WHERE o.id @@@ paradedb.all() AND o.grp = 1 AND p.amount = 3
ORDER BY o.made_at DESC, o.id DESC
LIMIT 10;

SELECT o.id
FROM btn_ordered o
JOIN btn_probe p ON p.fk = o.id
WHERE o.id @@@ paradedb.all() AND o.grp = 1 AND p.amount = 3
ORDER BY o.made_at DESC, o.id DESC
LIMIT 10;

SET paradedb.enable_join_bounded_topn = off;
SELECT o.id
FROM btn_ordered o
JOIN btn_probe p ON p.fk = o.id
WHERE o.id @@@ paradedb.all() AND o.grp = 1 AND p.amount = 3
ORDER BY o.made_at DESC, o.id DESC
LIMIT 10;
RESET paradedb.enable_join_bounded_topn;

-- =============================================================================
-- TEST 4: fewer rows exist than the LIMIT asks for
-- =============================================================================

SELECT o.id
FROM btn_ordered o
JOIN btn_probe p ON p.fk = o.id
WHERE o.id @@@ paradedb.all() AND o.grp = 1 AND p.amount = 3
ORDER BY o.made_at DESC, o.id DESC
LIMIT 1000;

-- =============================================================================
-- TEST 5: shapes the bound has to decline
-- =============================================================================

-- Sort keys from both sides.
SELECT o.id
FROM btn_ordered o
JOIN btn_probe p ON p.fk = o.id
WHERE o.id @@@ paradedb.all() AND o.grp = 1 AND p.amount >= 0
ORDER BY o.made_at DESC, p.amount DESC
LIMIT 10;

-- An outer join null-extends rows, so a sort key can come from no source row.
SELECT o.id
FROM btn_ordered o
LEFT JOIN btn_probe p ON p.fk = o.id AND p.amount >= 0
WHERE o.id @@@ paradedb.all() AND o.grp = 1
ORDER BY o.made_at DESC, o.id DESC
LIMIT 10;

-- No LIMIT, so there is nothing to bound.
SELECT count(*)
FROM btn_ordered o
JOIN btn_probe p ON p.fk = o.id
WHERE o.id @@@ paradedb.all() AND o.grp = 1 AND p.amount >= 0;

DROP TABLE btn_probe;
DROP TABLE btn_ordered;
