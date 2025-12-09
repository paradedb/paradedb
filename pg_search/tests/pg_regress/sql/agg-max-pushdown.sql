-- Test for MAX/MIN aggregate pushdown with date/datetime types
-- Issue: MAX agg pushdown always returns null values for dates

CREATE EXTENSION IF NOT EXISTS pg_search;

-- Create test table with various date/time types
CREATE TABLE date_agg_test (
    id serial PRIMARY KEY,
    d date,
    ts timestamp,
    tstz timestamptz,
    t time,
    ttz timetz
);

-- Insert test data with some NULL values
INSERT INTO date_agg_test (d, ts, tstz, t, ttz) VALUES
    ('2051-01-02'::date, '2051-01-02 10:30:00'::timestamp, '2051-01-02 10:30:00+00'::timestamptz, '10:30:00'::time, '10:30:00+00'::timetz),
    ('2023-06-15'::date, '2023-06-15 14:45:30'::timestamp, '2023-06-15 14:45:30+00'::timestamptz, '14:45:30'::time, '14:45:30+00'::timetz),
    ('1990-12-25'::date, '1990-12-25 08:00:00'::timestamp, '1990-12-25 08:00:00+00'::timestamptz, '08:00:00'::time, '08:00:00+00'::timetz),
    (NULL, NULL, NULL, NULL, NULL);

-- Create bm25 index
CREATE INDEX ON date_agg_test USING bm25 (id, d, ts, tstz, t, ttz) WITH (key_field = 'id');

-- Enable aggregate custom scan
SET paradedb.enable_aggregate_custom_scan TO on;

-- Test MAX with date
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF) SELECT max(d) FROM date_agg_test WHERE id @@@ pdb.all();
SELECT max(d) FROM date_agg_test WHERE id @@@ pdb.all();

-- Test MIN with date
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF) SELECT min(d) FROM date_agg_test WHERE id @@@ pdb.all();
SELECT min(d) FROM date_agg_test WHERE id @@@ pdb.all();

-- Test MAX with timestamp
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF) SELECT max(ts) FROM date_agg_test WHERE id @@@ pdb.all();
SELECT max(ts) FROM date_agg_test WHERE id @@@ pdb.all();

-- Test MIN with timestamp
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF) SELECT min(ts) FROM date_agg_test WHERE id @@@ pdb.all();
SELECT min(ts) FROM date_agg_test WHERE id @@@ pdb.all();

-- Test MAX with timestamptz
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF) SELECT max(tstz) FROM date_agg_test WHERE id @@@ pdb.all();
SELECT max(tstz) FROM date_agg_test WHERE id @@@ pdb.all();

-- Test MIN with timestamptz
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF) SELECT min(tstz) FROM date_agg_test WHERE id @@@ pdb.all();
SELECT min(tstz) FROM date_agg_test WHERE id @@@ pdb.all();

-- Test MAX with time
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF) SELECT max(t) FROM date_agg_test WHERE id @@@ pdb.all();
SELECT max(t) FROM date_agg_test WHERE id @@@ pdb.all();

-- Test MIN with time
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF) SELECT min(t) FROM date_agg_test WHERE id @@@ pdb.all();
SELECT min(t) FROM date_agg_test WHERE id @@@ pdb.all();

-- Test MAX with timetz
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF) SELECT max(ttz) FROM date_agg_test WHERE id @@@ pdb.all();
SELECT max(ttz) FROM date_agg_test WHERE id @@@ pdb.all();

-- Test MIN with timetz
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF) SELECT min(ttz) FROM date_agg_test WHERE id @@@ pdb.all();
SELECT min(ttz) FROM date_agg_test WHERE id @@@ pdb.all();

-- Test without aggregate pushdown to verify correct expected values
SET paradedb.enable_aggregate_custom_scan TO off;

SELECT max(d), min(d) FROM date_agg_test WHERE id @@@ pdb.all();
SELECT max(ts), min(ts) FROM date_agg_test WHERE id @@@ pdb.all();
SELECT max(tstz), min(tstz) FROM date_agg_test WHERE id @@@ pdb.all();
SELECT max(t), min(t) FROM date_agg_test WHERE id @@@ pdb.all();
SELECT max(ttz), min(ttz) FROM date_agg_test WHERE id @@@ pdb.all();

-- Test with all NULL datetime values
CREATE TABLE all_null_dates (
    id serial PRIMARY KEY,
    d date
);
INSERT INTO all_null_dates (d) VALUES (NULL), (NULL);
CREATE INDEX ON all_null_dates USING bm25 (id, d) WITH (key_field = 'id');

SET paradedb.enable_aggregate_custom_scan TO on;
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF) SELECT max(d) FROM all_null_dates WHERE id @@@ pdb.all();
SELECT max(d) FROM all_null_dates WHERE id @@@ pdb.all();

SET paradedb.enable_aggregate_custom_scan TO off;
SELECT max(d) FROM all_null_dates WHERE id @@@ pdb.all();

DROP TABLE all_null_dates;

-- Clean up
DROP TABLE date_agg_test;

