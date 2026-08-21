-- =====================================================================
-- MPP JoinScan whose broadcast side keeps its rows in a mutable segment.
--
-- The stage that joins resolves the broadcast side's packed
-- `DocAddress`es with a reader it opens itself. A mutable segment is
-- materialized from the heap per open, so that reader has to replay the
-- leader's segment view (ordinal order and the mutable segment's
-- add/remove log prefix) or its `DocId`s won't line up with the ones the
-- producer packed. Same query shape as issue #5988.
-- =====================================================================

CREATE EXTENSION IF NOT EXISTS pg_search;

SET paradedb.enable_join_custom_scan TO on;
SET max_parallel_workers_per_gather TO 3;
SET max_parallel_workers TO 8;
SET min_parallel_table_scan_size TO 0;
SET parallel_setup_cost TO 0;
SET parallel_tuple_cost TO 0;

CREATE TABLE mpp_mut_categories (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL
);
CREATE TABLE mpp_mut_test (
    id SERIAL8 NOT NULL PRIMARY KEY,
    message TEXT,
    category_id INTEGER NOT NULL
);

CREATE INDEX mpp_mut_test_idx ON mpp_mut_test
USING paradedb (id, message, category_id)
WITH (key_field = 'id');
CREATE INDEX mpp_mut_categories_idx ON mpp_mut_categories
USING paradedb (id, (name::pdb.literal))
WITH (key_field = 'id');

-- Several immutable segments on the probe side so the join runs on
-- multiple MPP tasks; the categories stay in a mutable segment (default
-- `mutable_segment_rows`) and get updated in place, which appends to
-- that segment's log.
SET paradedb.global_mutable_segment_rows = 0;
INSERT INTO mpp_mut_test (message, category_id)
SELECT (ARRAY['beer wine cheese', 'beer wine', 'beer cheese', 'beer',
              'wine cheese', 'wine', 'cheese', 'bread butter'])[1 + (i % 8)] || ' ' || i::text,
       1 + (i % 100)
FROM generate_series(1, 1000) AS s(i);
INSERT INTO mpp_mut_test (message, category_id)
SELECT (ARRAY['beer wine cheese', 'beer wine', 'beer cheese', 'beer',
              'wine cheese', 'wine', 'cheese', 'bread butter'])[1 + (i % 8)] || ' ' || i::text,
       1 + (i % 100)
FROM generate_series(1001, 2000) AS s(i);
INSERT INTO mpp_mut_test (message, category_id)
SELECT (ARRAY['beer wine cheese', 'beer wine', 'beer cheese', 'beer',
              'wine cheese', 'wine', 'cheese', 'bread butter'])[1 + (i % 8)] || ' ' || i::text,
       1 + (i % 100)
FROM generate_series(2001, 3000) AS s(i);
RESET paradedb.global_mutable_segment_rows;

INSERT INTO mpp_mut_categories (id, name)
SELECT i, 'category ' || i::text FROM generate_series(1, 100) AS s(i);
UPDATE mpp_mut_categories SET name = name || ' (updated)' WHERE id <= 10;

ANALYZE mpp_mut_test;
ANALYZE mpp_mut_categories;

SELECT count(*) AS segments, bool_or(mutable) AS has_mutable
FROM paradedb.index_info('mpp_mut_categories_idx');

-- Serial baseline.
SET max_parallel_workers_per_gather TO 0;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT t.id, t.message, c.name
FROM mpp_mut_test t
JOIN mpp_mut_categories c ON t.category_id = c.id
WHERE t.message @@@ 'beer'
ORDER BY t.id
LIMIT 25;

SELECT t.id, t.message, c.name
FROM mpp_mut_test t
JOIN mpp_mut_categories c ON t.category_id = c.id
WHERE t.message @@@ 'beer'
ORDER BY t.id
LIMIT 25;

-- MPP: the categories scan runs in its own stage and is broadcast to the
-- stage that joins and checks visibility.
SET max_parallel_workers_per_gather TO 3;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT t.id, t.message, c.name
FROM mpp_mut_test t
JOIN mpp_mut_categories c ON t.category_id = c.id
WHERE t.message @@@ 'beer'
ORDER BY t.id
LIMIT 25;

SELECT t.id, t.message, c.name
FROM mpp_mut_test t
JOIN mpp_mut_categories c ON t.category_id = c.id
WHERE t.message @@@ 'beer'
ORDER BY t.id
LIMIT 25;

-- Same join with the probe side in one segment: the join and its
-- visibility check now run on the leader, whose categories reader opened
-- at plan time, while the categories scan is still distributed. The
-- leader replays the same view the workers get from the DSM.
CREATE TABLE mpp_mut_test_small (
    id SERIAL8 NOT NULL PRIMARY KEY,
    message TEXT,
    category_id INTEGER NOT NULL
);
CREATE INDEX mpp_mut_test_small_idx ON mpp_mut_test_small
USING paradedb (id, message, category_id)
WITH (key_field = 'id');
SET paradedb.global_mutable_segment_rows = 0;
INSERT INTO mpp_mut_test_small (message, category_id)
SELECT message, category_id FROM mpp_mut_test ORDER BY id;
RESET paradedb.global_mutable_segment_rows;

-- Two immutable category segments plus the mutable one so the categories
-- scan spans several MPP tasks.
SET paradedb.global_mutable_segment_rows = 0;
INSERT INTO mpp_mut_categories (id, name)
SELECT i, 'category ' || i::text FROM generate_series(101, 150) AS s(i);
INSERT INTO mpp_mut_categories (id, name)
SELECT i, 'category ' || i::text FROM generate_series(151, 200) AS s(i);
RESET paradedb.global_mutable_segment_rows;
UPDATE mpp_mut_categories SET name = name || ' (updated again)' WHERE id <= 5;

ANALYZE mpp_mut_test_small;
ANALYZE mpp_mut_categories;

SELECT count(*) AS segments, bool_or(mutable) AS has_mutable
FROM paradedb.index_info('mpp_mut_categories_idx');

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT t.id, t.message, c.name
FROM mpp_mut_test_small t
JOIN mpp_mut_categories c ON t.category_id = c.id
WHERE t.message @@@ 'beer'
ORDER BY t.id
LIMIT 25;

SELECT t.id, t.message, c.name
FROM mpp_mut_test_small t
JOIN mpp_mut_categories c ON t.category_id = c.id
WHERE t.message @@@ 'beer'
ORDER BY t.id
LIMIT 25;

DROP TABLE mpp_mut_test_small;
DROP TABLE mpp_mut_test;
DROP TABLE mpp_mut_categories;
