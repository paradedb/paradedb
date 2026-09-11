-- A top-K scan picks serial or parallel from its estimated drive cost, so an under-counted
-- estimate quietly costs the scan its workers. Two shapes under-counted enough to flip that
-- decision: a phrase, whose cost came from an intersection estimate that assumes the terms are
-- independent, and a regex ANDed with a clause carrying no heuristic, which folded the
-- unknown-selectivity sentinel into the product.

CREATE EXTENSION IF NOT EXISTS pg_search;

SET max_parallel_workers_per_gather = 2;
SET max_parallel_workers = 8;
SET enable_indexscan = OFF;

-- Small but non-zero. At zero every scan parallelizes and the decision under test never runs.
SET parallel_setup_cost = 60;
SET parallel_tuple_cost = 0;

DROP TABLE IF EXISTS logs CASCADE;
CREATE TABLE logs (
    id      BIGINT,
    at      TIMESTAMP,
    body    TEXT
);

CREATE INDEX logs_idx ON logs
    USING bm25 (id, at, (body::pdb.simple))
    WITH (key_field = 'id');

-- Two batches so the index holds more than one segment and parallel is structurally possible.
-- `failed to place order` lands on just under a fifth of the rows and its four words only ever
-- appear together, which is the correlation an independence estimate misses. `crashed while
-- writing the audit ledger` is the rare counterpart, on one row in a hundred.
INSERT INTO logs
SELECT g,
       TIMESTAMP '2025-09-23 00:00:00' + ((g % 3600) || ' seconds')::interval,
       CASE
           WHEN g % 100 = 1 THEN 'crashed while writing the audit ledger ' || g
           WHEN g % 50 < 10 THEN 'failed to place order ' || g
           WHEN g % 5 = 1   THEN 'charged card ending ' || g
           WHEN g % 5 = 2   THEN 'charging retry ' || g
           ELSE 'request served in ' || g || ' ms'
       END
FROM generate_series(1, 100000) g;

INSERT INTO logs
SELECT g,
       TIMESTAMP '2025-09-23 00:00:00' + ((g % 3600) || ' seconds')::interval,
       CASE
           WHEN g % 100 = 1 THEN 'crashed while writing the audit ledger ' || g
           WHEN g % 50 < 10 THEN 'failed to place order ' || g
           WHEN g % 5 = 1   THEN 'charged card ending ' || g
           WHEN g % 5 = 2   THEN 'charging retry ' || g
           ELSE 'request served in ' || g || ' ms'
       END
FROM generate_series(100001, 200000) g;

ANALYZE logs;

SELECT count(*) AS common_phrase FROM logs WHERE body @@@ '"failed to place order"';
SELECT count(*) AS rare_phrase FROM logs WHERE body @@@ '"crashed while writing the audit ledger"';
SELECT count(*) AS regex_matches FROM logs WHERE id @@@ paradedb.regex('body', 'charg.*');

-- ============================================================================
-- Common phrase, top-K by score. The scan walks a fifth of the index, so it takes workers.
-- ============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, body FROM logs
WHERE body @@@ '"failed to place order"'
ORDER BY pdb.score(id) DESC, id LIMIT 5;

SELECT id, body FROM logs
WHERE body @@@ '"failed to place order"'
ORDER BY pdb.score(id) DESC, id LIMIT 5;

-- ============================================================================
-- The same phrase under a time window, top-K by the window column.
-- ============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, at FROM logs
WHERE body @@@ '"failed to place order"'
  AND at BETWEEN TIMESTAMP '2025-09-23 00:00:00' AND TIMESTAMP '2025-09-23 00:30:00'
ORDER BY at DESC, id LIMIT 5;

SELECT id, at FROM logs
WHERE body @@@ '"failed to place order"'
  AND at BETWEEN TIMESTAMP '2025-09-23 00:00:00' AND TIMESTAMP '2025-09-23 00:30:00'
ORDER BY at DESC, id LIMIT 5;

-- ============================================================================
-- Regex under a time window. The window carries no heuristic of its own, so it has to leave the
-- regex estimate alone rather than scale it away.
-- ============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, at FROM logs
WHERE id @@@ paradedb.regex('body', 'charg.*')
  AND at BETWEEN TIMESTAMP '2025-09-23 00:00:00' AND TIMESTAMP '2025-09-23 00:30:00'
ORDER BY at DESC, id LIMIT 5;

SELECT id, at FROM logs
WHERE id @@@ paradedb.regex('body', 'charg.*')
  AND at BETWEEN TIMESTAMP '2025-09-23 00:00:00' AND TIMESTAMP '2025-09-23 00:30:00'
ORDER BY at DESC, id LIMIT 5;

-- ============================================================================
-- The rare phrase stays serial. Its posting lists are short, so workers would only add startup.
-- The floor keeps the estimate honest, it does not push every phrase to parallel.
-- ============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, body FROM logs
WHERE body @@@ '"crashed while writing the audit ledger"'
ORDER BY pdb.score(id) DESC, id LIMIT 5;

SELECT id, body FROM logs
WHERE body @@@ '"crashed while writing the audit ledger"'
ORDER BY pdb.score(id) DESC, id LIMIT 5;

DROP TABLE logs CASCADE;
