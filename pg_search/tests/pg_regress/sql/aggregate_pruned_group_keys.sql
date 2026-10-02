-- Tests that AggregateScan groups by columns that PostgreSQL leaves out of
-- group_pathkeys but keeps in the output: a key equal to a constant, and a
-- column determined by a primary key in the GROUP BY.

\i common/common_setup.sql

SET paradedb.enable_aggregate_custom_scan TO on;

DROP TABLE IF EXISTS pruned_keys;
CREATE TABLE pruned_keys (
    id SERIAL PRIMARY KEY,
    account_id INTEGER,
    kind TEXT,
    amount NUMERIC(10, 2),
    metadata JSONB
);

INSERT INTO pruned_keys (account_id, kind, amount, metadata) VALUES
    (1, 'invoice', 10.50, '{"color": "red"}'),
    (1, 'invoice', 20.25, '{"color": "red"}'),
    (1, 'refund',   5.00, '{"color": "blue"}'),
    (1, 'payout',  40.00, '{"color": "red"}'),
    (1, NULL,       1.00, '{"color": "blue"}'),
    (2, 'invoice', 30.00, '{"color": "red"}'),
    (2, 'refund',   7.75, '{"color": "blue"}'),
    (2, 'refund',   2.25, '{"color": "red"}'),
    (3, 'payout',  99.00, '{"color": "green"}'),
    (3, 'invoice', 15.00, NULL);

CREATE INDEX pruned_keys_idx ON pruned_keys USING paradedb
    (id, account_id, (kind::pdb.unicode_words('columnar=true')), amount,
     (metadata::pdb.simple('columnar=true')));

ANALYZE pruned_keys;

\echo 'Test 1: a single key equal to a constant'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT account_id, COUNT(*) FROM pruned_keys
WHERE account_id = 1 AND id @@@ paradedb.all()
GROUP BY account_id;

SELECT account_id, COUNT(*) FROM pruned_keys
WHERE account_id = 1 AND id @@@ paradedb.all()
GROUP BY account_id;

\echo 'Test 2: two keys, one equal to a constant'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) AS count_all, account_id, kind FROM pruned_keys
WHERE account_id = 1 AND id @@@ paradedb.all()
GROUP BY account_id, kind;

SELECT COUNT(*) AS count_all, account_id, kind FROM pruned_keys
WHERE account_id = 1 AND id @@@ paradedb.all()
GROUP BY account_id, kind
ORDER BY kind;

\echo 'Test 3: the constant key is not selected'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) FROM pruned_keys
WHERE account_id = 1 AND id @@@ paradedb.all()
GROUP BY account_id;

SELECT COUNT(*) FROM pruned_keys
WHERE account_id = 1 AND id @@@ paradedb.all()
GROUP BY account_id;

\echo 'Test 4: no matching rows returns no groups'
SELECT account_id, COUNT(*) FROM pruned_keys
WHERE account_id = 99 AND id @@@ paradedb.all()
GROUP BY account_id;

SELECT COUNT(*) FROM pruned_keys
WHERE account_id = 99 AND id @@@ paradedb.all()
GROUP BY account_id;

\echo 'Test 5: a single-element IN list is an equality'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT account_id, kind, COUNT(*) FROM pruned_keys
WHERE account_id IN (2) AND id @@@ paradedb.all()
GROUP BY account_id, kind;

SELECT account_id, kind, COUNT(*) FROM pruned_keys
WHERE account_id IN (2) AND id @@@ paradedb.all()
GROUP BY account_id, kind
ORDER BY kind;

\echo 'Test 6: a key equal to a parameter in a generic plan'
-- Executed once: a cached AggregateScan plan fails when it is executed again (#6136).
SET plan_cache_mode = force_generic_plan;
PREPARE pruned_keys_by_account(INTEGER) AS
SELECT account_id, kind, COUNT(*) FROM pruned_keys
WHERE account_id = $1 AND id @@@ paradedb.all()
GROUP BY account_id, kind
ORDER BY kind;

EXECUTE pruned_keys_by_account(2);
DEALLOCATE pruned_keys_by_account;
RESET plan_cache_mode;

\echo 'Test 7: a column determined by the primary key'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, kind, COUNT(*) FROM pruned_keys
WHERE id @@@ paradedb.all()
GROUP BY id, kind
ORDER BY id;

SELECT id, kind, COUNT(*) FROM pruned_keys
WHERE id @@@ paradedb.all()
GROUP BY id, kind
ORDER BY id;

\echo 'Test 8: ORDER BY the constant key and LIMIT'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT account_id, kind, COUNT(*) FROM pruned_keys
WHERE account_id = 1 AND id @@@ paradedb.all()
GROUP BY account_id, kind
ORDER BY account_id, kind
LIMIT 2;

SELECT account_id, kind, COUNT(*) FROM pruned_keys
WHERE account_id = 1 AND id @@@ paradedb.all()
GROUP BY account_id, kind
ORDER BY account_id, kind
LIMIT 2;

\echo 'Test 9: a JSON path key equal to a constant'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT metadata->>'color' AS color, kind, COUNT(*) FROM pruned_keys
WHERE metadata->>'color' = 'red' AND id @@@ paradedb.all()
GROUP BY metadata->>'color', kind;

SELECT metadata->>'color' AS color, kind, COUNT(*) FROM pruned_keys
WHERE metadata->>'color' = 'red' AND id @@@ paradedb.all()
GROUP BY metadata->>'color', kind
ORDER BY kind;

\echo 'Test 10: a NUMERIC aggregate runs on DataFusion'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT account_id, SUM(amount) FROM pruned_keys
WHERE account_id = 1 AND id @@@ paradedb.all()
GROUP BY account_id;

SELECT account_id, SUM(amount) FROM pruned_keys
WHERE account_id = 1 AND id @@@ paradedb.all()
GROUP BY account_id;

\echo 'Test 11: pdb.agg() with a constant key'
SELECT account_id, pdb.agg('{"value_count": {"field": "id"}}'::jsonb) FROM pruned_keys
WHERE account_id = 1 AND id @@@ paradedb.all()
GROUP BY account_id;

\echo 'Test 12: a constant key with a nondeterministic collation is not pushed down'
CREATE COLLATION pruned_keys_case_insensitive (provider = icu, locale = 'und-u-ks-level2', deterministic = false);
CREATE TABLE pruned_keys_collation (
    id SERIAL PRIMARY KEY,
    label TEXT COLLATE pruned_keys_case_insensitive,
    kind TEXT
);
INSERT INTO pruned_keys_collation (label, kind) VALUES
    ('Red', 'invoice'),
    ('red', 'invoice'),
    ('blue', 'invoice');
CREATE INDEX pruned_keys_collation_idx ON pruned_keys_collation USING paradedb
    (id, (label::pdb.unicode_words('columnar=true')), (kind::pdb.unicode_words('columnar=true')));

SELECT COUNT(*) FROM pruned_keys_collation
WHERE label = 'red' AND id @@@ paradedb.all()
GROUP BY label;

SELECT kind, COUNT(*) FROM pruned_keys_collation
WHERE label = 'red' AND id @@@ paradedb.all()
GROUP BY label, kind;

DROP TABLE pruned_keys_collation;
DROP COLLATION pruned_keys_case_insensitive;

\echo 'Test 13: a JSON path determined by the primary key'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, metadata->>'color' AS color, COUNT(*) FROM pruned_keys
WHERE id @@@ paradedb.all()
GROUP BY id
ORDER BY id;

SELECT id, metadata->>'color' AS color, COUNT(*) FROM pruned_keys
WHERE id @@@ paradedb.all()
GROUP BY id
ORDER BY id;

\echo 'Test 14: a constant key under a pulled-up subquery'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT s.account_id, s.kind, COUNT(*)
FROM (SELECT * FROM pruned_keys WHERE account_id = 1 AND id @@@ paradedb.all()) s
GROUP BY s.account_id, s.kind;

SELECT s.account_id, s.kind, COUNT(*)
FROM (SELECT * FROM pruned_keys WHERE account_id = 1 AND id @@@ paradedb.all()) s
GROUP BY s.account_id, s.kind
ORDER BY s.kind;

\echo 'Test 15: a key equal to an outer column in a correlated subquery'
SELECT a.id,
       (SELECT COUNT(*) FROM pruned_keys b
        WHERE b.account_id = a.account_id AND b.id @@@ paradedb.all()
        GROUP BY b.account_id) AS same_account
FROM pruned_keys a
WHERE a.id IN (1, 6, 9)
ORDER BY a.id;

\echo 'Test 16: a key equal to an outer column in a LATERAL subquery'
SELECT a.id, l.kind, l.count
FROM pruned_keys a,
     LATERAL (SELECT b.account_id, b.kind, COUNT(*) FROM pruned_keys b
              WHERE b.account_id = a.account_id AND b.id @@@ paradedb.all()
              GROUP BY b.account_id, b.kind) l
WHERE a.id IN (6, 9)
ORDER BY a.id, l.kind;

\echo 'PostgreSQL returns the same rows without AggregateScan'
SET paradedb.enable_aggregate_custom_scan TO off;

SELECT account_id, COUNT(*) FROM pruned_keys
WHERE account_id = 1 AND id @@@ paradedb.all()
GROUP BY account_id;

SELECT COUNT(*) AS count_all, account_id, kind FROM pruned_keys
WHERE account_id = 1 AND id @@@ paradedb.all()
GROUP BY account_id, kind
ORDER BY kind;

SELECT COUNT(*) FROM pruned_keys
WHERE account_id = 1 AND id @@@ paradedb.all()
GROUP BY account_id;

SELECT account_id, COUNT(*) FROM pruned_keys
WHERE account_id = 99 AND id @@@ paradedb.all()
GROUP BY account_id;

SELECT COUNT(*) FROM pruned_keys
WHERE account_id = 99 AND id @@@ paradedb.all()
GROUP BY account_id;

SELECT account_id, kind, COUNT(*) FROM pruned_keys
WHERE account_id IN (2) AND id @@@ paradedb.all()
GROUP BY account_id, kind
ORDER BY kind;

SELECT id, kind, COUNT(*) FROM pruned_keys
WHERE id @@@ paradedb.all()
GROUP BY id, kind
ORDER BY id;

SELECT account_id, kind, COUNT(*) FROM pruned_keys
WHERE account_id = 1 AND id @@@ paradedb.all()
GROUP BY account_id, kind
ORDER BY account_id, kind
LIMIT 2;

SELECT metadata->>'color' AS color, kind, COUNT(*) FROM pruned_keys
WHERE metadata->>'color' = 'red' AND id @@@ paradedb.all()
GROUP BY metadata->>'color', kind
ORDER BY kind;

SELECT account_id, SUM(amount) FROM pruned_keys
WHERE account_id = 1 AND id @@@ paradedb.all()
GROUP BY account_id;

SELECT id, metadata->>'color' AS color, COUNT(*) FROM pruned_keys
WHERE id @@@ paradedb.all()
GROUP BY id
ORDER BY id;

SELECT s.account_id, s.kind, COUNT(*)
FROM (SELECT * FROM pruned_keys WHERE account_id = 1 AND id @@@ paradedb.all()) s
GROUP BY s.account_id, s.kind
ORDER BY s.kind;

SELECT a.id,
       (SELECT COUNT(*) FROM pruned_keys b
        WHERE b.account_id = a.account_id AND b.id @@@ paradedb.all()
        GROUP BY b.account_id) AS same_account
FROM pruned_keys a
WHERE a.id IN (1, 6, 9)
ORDER BY a.id;

SELECT a.id, l.kind, l.count
FROM pruned_keys a,
     LATERAL (SELECT b.account_id, b.kind, COUNT(*) FROM pruned_keys b
              WHERE b.account_id = a.account_id AND b.id @@@ paradedb.all()
              GROUP BY b.account_id, b.kind) l
WHERE a.id IN (6, 9)
ORDER BY a.id, l.kind;

RESET paradedb.enable_aggregate_custom_scan;

DROP TABLE pruned_keys;

\i common/common_cleanup.sql
