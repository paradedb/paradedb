-- DATE(timestamp) grouping must reconstruct its UDF on MPP workers and
-- merge partial groups across segments, including NULL and infinities.
CREATE EXTENSION IF NOT EXISTS pg_search;

DROP TABLE IF EXISTS mpp_date_events CASCADE;

CREATE TABLE mpp_date_events (
    id SERIAL PRIMARY KEY,
    created_at TIMESTAMP,
    amount INTEGER NOT NULL
);

CREATE INDEX mpp_date_events_idx ON mpp_date_events
USING paradedb (id, created_at, amount)
WITH (
    key_field = 'id',
    target_segment_count = 2
);

SET paradedb.global_mutable_segment_rows TO 0;

INSERT INTO mpp_date_events (created_at, amount) VALUES
    ('2024-01-01 00:00:00', 1),
    ('2024-01-01 08:00:00', 1),
    ('2024-01-01 23:59:59', 1),
    ('2024-01-02 09:00:00', 1),
    ('2024-01-03 12:00:00', 1),
    ('2024-01-05 18:00:00', 1),
    (NULL, 1), -- Must produce a NULL date group
    ('infinity', 1), -- Must produce an infinity date group
    ('-infinity', 1), -- Must produce an -infinity date group
    ('1969-12-31 23:59:59.999999', 1), -- 1 microsecond before the Unix epoch
    ('1999-12-31 23:59:59.999999', 1), -- 1 microsecond before the Postgres epoch
    ('294276-12-31 23:59:59.999999', 1); -- Maximum finite PostgreSQL timestamp

INSERT INTO mpp_date_events (created_at, amount) VALUES
    ('2024-01-01 00:00:00', 2),
    ('2024-01-01 08:00:00', 2),
    ('2024-01-01 23:59:59', 2),
    ('2024-01-02 09:00:00', 2),
    ('2024-01-03 12:00:00', 2),
    ('2024-01-05 18:00:00', 2),
    (NULL, 2), -- Must produce a NULL date group
    ('infinity', 2), -- Must produce an infinity date group
    ('-infinity', 2), -- Must produce an -infinity date group
    ('1969-12-31 23:59:59.999999', 2), -- 1 microsecond before the Unix epoch
    ('1999-12-31 23:59:59.999999', 2), -- 1 microsecond before the Postgres epoch
    ('294276-12-31 23:59:59.999999', 2); -- Maximum finite PostgreSQL timestamp

RESET paradedb.global_mutable_segment_rows;
ANALYZE mpp_date_events;

-- Reference result: PostgreSQL performs the date conversion and aggregation.
SET paradedb.enable_aggregate_custom_scan TO off;
SET max_parallel_workers_per_gather TO 0;

CREATE TEMP TABLE mpp_date_native AS
SELECT DATE(created_at) AS day,
       COUNT(*) AS cnt,
       SUM(amount) AS total
FROM mpp_date_events
WHERE id @@@ pdb.all()
GROUP BY DATE(created_at);

SELECT * FROM mpp_date_native
ORDER BY day NULLS LAST;

-- Serial DataFusion: verify the plan, then save its results.
SET paradedb.enable_aggregate_custom_scan TO on;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT DATE(created_at) AS day,
       COUNT(*) AS cnt,
       SUM(amount) AS total
FROM mpp_date_events
WHERE id @@@ pdb.all()
GROUP BY DATE(created_at);

CREATE TEMP TABLE mpp_date_serial AS
SELECT DATE(created_at) AS day,
       COUNT(*) AS cnt,
       SUM(amount) AS total
FROM mpp_date_events
WHERE id @@@ pdb.all()
GROUP BY DATE(created_at);

-- Compare both directions to catch extra and missing rows. EXCEPT ALL
-- also catches duplicate-count differences and compares NULL groups.
SELECT NOT EXISTS (
    (SELECT * FROM mpp_date_serial
     EXCEPT ALL
     SELECT * FROM mpp_date_native)

    UNION ALL

    (SELECT * FROM mpp_date_native
     EXCEPT ALL
     SELECT * FROM mpp_date_serial)
) AS serial_matches_postgres;

-- Allow MPP even for this tiny test dataset.
SET paradedb.mpp_min_rows TO 0;
SET max_parallel_workers_per_gather TO 3;
SET max_parallel_workers TO 8;
SET min_parallel_table_scan_size TO 0;
SET parallel_setup_cost TO 0;
SET parallel_tuple_cost TO 0;

-- MPP DataFusion: record the distributed plan and save the query results.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT DATE(created_at) AS day,
       COUNT(*) AS cnt,
       SUM(amount) AS total
FROM mpp_date_events
WHERE id @@@ pdb.all()
GROUP BY DATE(created_at);

CREATE TEMP TABLE mpp_date_distributed AS
SELECT DATE(created_at) AS day,
       COUNT(*) AS cnt,
       SUM(amount) AS total
FROM mpp_date_events
WHERE id @@@ pdb.all()
GROUP BY DATE(created_at);

SELECT * FROM mpp_date_distributed
ORDER BY day NULLS LAST;

-- Compare the MPP result with PostgreSQL in both directions as well.
SELECT NOT EXISTS (
    (SELECT * FROM mpp_date_distributed
     EXCEPT ALL
     SELECT * FROM mpp_date_native)

    UNION ALL

    (SELECT * FROM mpp_date_native
     EXCEPT ALL
     SELECT * FROM mpp_date_distributed)
) AS mpp_matches_postgres;

-- Remove the fixture and restore the session settings changed by this test.
DROP TABLE mpp_date_distributed, mpp_date_serial, mpp_date_native;
DROP TABLE mpp_date_events;

RESET paradedb.enable_aggregate_custom_scan;
RESET paradedb.mpp_min_rows;
RESET max_parallel_workers_per_gather;
RESET max_parallel_workers;
RESET min_parallel_table_scan_size;
RESET parallel_setup_cost;
RESET parallel_tuple_cost;
