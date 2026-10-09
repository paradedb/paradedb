-- Issue #6577: projecting an expression over an indexed datetime column in a
-- Join Scan target list wraps it as a PgExprUdf. The UDF's input signature must
-- match the Arrow type the fast field is scanned as (Timestamp(µs) for every
-- datetime type), or DataFusion fails to build the logical plan with
-- "coercion from Timestamp(µs) to the signature Exact(...) failed".
--
-- Each case runs EXPLAIN, then the query with the Join Scan on and off. Both
-- result sets must match.

SET max_parallel_workers_per_gather = 0;
SET enable_indexscan TO OFF;
SET TimeZone = 'UTC';

CREATE EXTENSION IF NOT EXISTS pg_search;

DROP TABLE IF EXISTS i6577_s, i6577_e CASCADE;

CREATE TABLE i6577_s
(
    id     INT PRIMARY KEY,
    title  TEXT,
    rating FLOAT8,
    d      DATE,
    ts     TIMESTAMP,
    tstz   TIMESTAMPTZ,
    t      TIME,
    ttz    TIMETZ
);

INSERT INTO i6577_s
SELECT g,
       CASE WHEN g % 3 = 0 THEN 'dragon ' || g ELSE 'other ' || g END,
       g % 100,
       DATE '2000-01-01' + g,
       TIMESTAMP '2000-01-01 12:34:56' + g * INTERVAL '1 day 1 minute',
       TIMESTAMPTZ '2000-01-01 12:34:56+00' + g * INTERVAL '1 day 1 minute',
       TIME '00:00:00' + g * INTERVAL '1 minute',
       TIMETZ '00:00:00+00' + g * INTERVAL '1 minute'
FROM generate_series(1, 500) g;

CREATE INDEX i6577_s_idx ON i6577_s
    USING bm25 (id, title, rating, d, ts, tstz, t, ttz);

CREATE TABLE i6577_e
(
    id   INT PRIMARY KEY,
    u    TEXT,
    s_id INT
);

INSERT INTO i6577_e SELECT g, 'u1', g * 2 FROM generate_series(1, 100) g;

CREATE INDEX i6577_e_idx ON i6577_e
    USING bm25 (id, u, s_id)
    WITH (text_fields = '{"u": {"fast": true, "tokenizer": {"type": "keyword"}}}');

ANALYZE i6577_s;
ANALYZE i6577_e;

SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- Control: bare DATE column in a join
-- =============================================================================

EXPLAIN (COSTS OFF)
SELECT s.id, s.d FROM i6577_s s JOIN i6577_e e ON e.s_id = s.id
WHERE s.id @@@ paradedb.match('title', 'dragon') AND e.id @@@ paradedb.term('u', 'u1')
ORDER BY s.rating DESC, s.id LIMIT 3;

SELECT s.id, s.d FROM i6577_s s JOIN i6577_e e ON e.s_id = s.id
WHERE s.id @@@ paradedb.match('title', 'dragon') AND e.id @@@ paradedb.term('u', 'u1')
ORDER BY s.rating DESC, s.id LIMIT 3;

-- =============================================================================
-- DATE: d::text in a join
-- =============================================================================

EXPLAIN (COSTS OFF)
SELECT s.id, s.d::text AS d FROM i6577_s s JOIN i6577_e e ON e.s_id = s.id
WHERE s.id @@@ paradedb.match('title', 'dragon') AND e.id @@@ paradedb.term('u', 'u1')
ORDER BY s.rating DESC, s.id LIMIT 3;

SELECT s.id, s.d::text AS d FROM i6577_s s JOIN i6577_e e ON e.s_id = s.id
WHERE s.id @@@ paradedb.match('title', 'dragon') AND e.id @@@ paradedb.term('u', 'u1')
ORDER BY s.rating DESC, s.id LIMIT 3;

SET paradedb.enable_join_custom_scan = off;
SELECT s.id, s.d::text AS d FROM i6577_s s JOIN i6577_e e ON e.s_id = s.id
WHERE s.id @@@ paradedb.match('title', 'dragon') AND e.id @@@ paradedb.term('u', 'u1')
ORDER BY s.rating DESC, s.id LIMIT 3;
SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- DATE: to_char(d) in a join
-- =============================================================================

EXPLAIN (COSTS OFF)
SELECT s.id, to_char(s.d, 'YYYY-MM-DD') AS d FROM i6577_s s JOIN i6577_e e ON e.s_id = s.id
WHERE s.id @@@ paradedb.match('title', 'dragon') AND e.id @@@ paradedb.term('u', 'u1')
ORDER BY s.rating DESC, s.id LIMIT 3;

SELECT s.id, to_char(s.d, 'YYYY-MM-DD') AS d FROM i6577_s s JOIN i6577_e e ON e.s_id = s.id
WHERE s.id @@@ paradedb.match('title', 'dragon') AND e.id @@@ paradedb.term('u', 'u1')
ORDER BY s.rating DESC, s.id LIMIT 3;

SET paradedb.enable_join_custom_scan = off;
SELECT s.id, to_char(s.d, 'YYYY-MM-DD') AS d FROM i6577_s s JOIN i6577_e e ON e.s_id = s.id
WHERE s.id @@@ paradedb.match('title', 'dragon') AND e.id @@@ paradedb.term('u', 'u1')
ORDER BY s.rating DESC, s.id LIMIT 3;
SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- DATE: d::text in an anti-join
-- =============================================================================

EXPLAIN (COSTS OFF)
SELECT s.id, s.d::text AS d FROM i6577_s s
WHERE s.id @@@ paradedb.match('title', 'dragon')
  AND NOT EXISTS (SELECT 1 FROM i6577_e e WHERE e.s_id = s.id AND e.id @@@ paradedb.term('u', 'u1'))
ORDER BY paradedb.score(s.id) DESC, s.rating DESC, s.id LIMIT 3;

SELECT s.id, s.d::text AS d FROM i6577_s s
WHERE s.id @@@ paradedb.match('title', 'dragon')
  AND NOT EXISTS (SELECT 1 FROM i6577_e e WHERE e.s_id = s.id AND e.id @@@ paradedb.term('u', 'u1'))
ORDER BY paradedb.score(s.id) DESC, s.rating DESC, s.id LIMIT 3;

SET paradedb.enable_join_custom_scan = off;
SELECT s.id, s.d::text AS d FROM i6577_s s
WHERE s.id @@@ paradedb.match('title', 'dragon')
  AND NOT EXISTS (SELECT 1 FROM i6577_e e WHERE e.s_id = s.id AND e.id @@@ paradedb.term('u', 'u1'))
ORDER BY paradedb.score(s.id) DESC, s.rating DESC, s.id LIMIT 3;
SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- TIMESTAMP: ts::text and to_char(ts) in a join
-- =============================================================================

EXPLAIN (COSTS OFF)
SELECT s.id, s.ts::text AS ts, to_char(s.ts, 'YYYY-MM-DD HH24:MI') AS ts_fmt
FROM i6577_s s JOIN i6577_e e ON e.s_id = s.id
WHERE s.id @@@ paradedb.match('title', 'dragon') AND e.id @@@ paradedb.term('u', 'u1')
ORDER BY s.rating DESC, s.id LIMIT 3;

SELECT s.id, s.ts::text AS ts, to_char(s.ts, 'YYYY-MM-DD HH24:MI') AS ts_fmt
FROM i6577_s s JOIN i6577_e e ON e.s_id = s.id
WHERE s.id @@@ paradedb.match('title', 'dragon') AND e.id @@@ paradedb.term('u', 'u1')
ORDER BY s.rating DESC, s.id LIMIT 3;

SET paradedb.enable_join_custom_scan = off;
SELECT s.id, s.ts::text AS ts, to_char(s.ts, 'YYYY-MM-DD HH24:MI') AS ts_fmt
FROM i6577_s s JOIN i6577_e e ON e.s_id = s.id
WHERE s.id @@@ paradedb.match('title', 'dragon') AND e.id @@@ paradedb.term('u', 'u1')
ORDER BY s.rating DESC, s.id LIMIT 3;
SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- TIMESTAMPTZ: tstz::text and to_char(tstz) in a join
-- =============================================================================

EXPLAIN (COSTS OFF)
SELECT s.id, s.tstz::text AS tstz, to_char(s.tstz, 'YYYY-MM-DD HH24:MI TZ') AS tstz_fmt
FROM i6577_s s JOIN i6577_e e ON e.s_id = s.id
WHERE s.id @@@ paradedb.match('title', 'dragon') AND e.id @@@ paradedb.term('u', 'u1')
ORDER BY s.rating DESC, s.id LIMIT 3;

SELECT s.id, s.tstz::text AS tstz, to_char(s.tstz, 'YYYY-MM-DD HH24:MI TZ') AS tstz_fmt
FROM i6577_s s JOIN i6577_e e ON e.s_id = s.id
WHERE s.id @@@ paradedb.match('title', 'dragon') AND e.id @@@ paradedb.term('u', 'u1')
ORDER BY s.rating DESC, s.id LIMIT 3;

SET paradedb.enable_join_custom_scan = off;
SELECT s.id, s.tstz::text AS tstz, to_char(s.tstz, 'YYYY-MM-DD HH24:MI TZ') AS tstz_fmt
FROM i6577_s s JOIN i6577_e e ON e.s_id = s.id
WHERE s.id @@@ paradedb.match('title', 'dragon') AND e.id @@@ paradedb.term('u', 'u1')
ORDER BY s.rating DESC, s.id LIMIT 3;
SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- TIME / TIMETZ: t::text and ttz::text in a join
-- =============================================================================

EXPLAIN (COSTS OFF)
SELECT s.id, s.t::text AS t, s.ttz::text AS ttz
FROM i6577_s s JOIN i6577_e e ON e.s_id = s.id
WHERE s.id @@@ paradedb.match('title', 'dragon') AND e.id @@@ paradedb.term('u', 'u1')
ORDER BY s.rating DESC, s.id LIMIT 3;

SELECT s.id, s.t::text AS t, s.ttz::text AS ttz
FROM i6577_s s JOIN i6577_e e ON e.s_id = s.id
WHERE s.id @@@ paradedb.match('title', 'dragon') AND e.id @@@ paradedb.term('u', 'u1')
ORDER BY s.rating DESC, s.id LIMIT 3;

SET paradedb.enable_join_custom_scan = off;
SELECT s.id, s.t::text AS t, s.ttz::text AS ttz
FROM i6577_s s JOIN i6577_e e ON e.s_id = s.id
WHERE s.id @@@ paradedb.match('title', 'dragon') AND e.id @@@ paradedb.term('u', 'u1')
ORDER BY s.rating DESC, s.id LIMIT 3;
SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- Control: d::text on a single table (base scan, PostgreSQL projects)
-- =============================================================================

SELECT s.id, s.d::text FROM i6577_s s
WHERE s.id @@@ paradedb.match('title', 'dragon')
ORDER BY s.rating DESC, s.id LIMIT 3;

RESET paradedb.enable_join_custom_scan;
RESET TimeZone;
RESET enable_indexscan;
RESET max_parallel_workers_per_gather;

DROP TABLE i6577_s, i6577_e CASCADE;
