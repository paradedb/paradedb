-- DATE aggregation over PostgreSQL's full DateADT range (#6878).
--
-- DATE fields store PostgreSQL day counts, so terms, min/max, and top_hits must decode them
-- without narrowing to the timestamp range. Histograms use their own table: a day bucket per
-- row across the full range would allocate one bucket per representable day.
CREATE EXTENSION IF NOT EXISTS pg_search;
SET datestyle = 'ISO, MDY';

DROP TABLE IF EXISTS date_agg CASCADE;
CREATE TABLE date_agg (id serial PRIMARY KEY, d date);
INSERT INTO date_agg (d) VALUES
    ('1999-12-31'),
    ('2000-01-01'),
    ('2000-01-02'),
    ('294277-01-01'),
    ('5874897-12-31'),
    ('4714-11-24 BC'),
    ('infinity'),
    ('-infinity');
CREATE INDEX date_agg_idx ON date_agg USING paradedb (id, d);

-- Terms on a DATE field return DATE keys, including the infinities.
SELECT * FROM paradedb.aggregate(
    index => 'date_agg_idx',
    query => paradedb.all(),
    agg => '{"by_date": {"terms": {"field": "d"}}}'
);

-- Single-value metrics attach a formatted key_as_string.
SELECT * FROM paradedb.aggregate(
    index => 'date_agg_idx',
    query => paradedb.all(),
    agg => '{"min_date": {"min": {"field": "d"}}, "max_date": {"max": {"field": "d"}}}'
);

-- Top hits sorts and reports far dates from the Date32 fast field. Descending puts the last
-- finite date first and BC last.
SELECT * FROM paradedb.aggregate(
    index => 'date_agg_idx',
    query => paradedb.all(),
    agg => '{"latest": {"top_hits": {"size": 2, "sort": [{"d": "desc"}], "docvalue_fields": ["d"]}}}'
);

SELECT * FROM paradedb.aggregate(
    index => 'date_agg_idx',
    query => paradedb.all(),
    agg => '{"earliest": {"top_hits": {"size": 2, "sort": [{"d": "asc"}], "docvalue_fields": ["d"]}}}'
);

-- A narrow window for date_histogram: day buckets stay within the timestamp range, so the
-- output is stable and cheap.
DROP TABLE IF EXISTS date_agg_hist CASCADE;
CREATE TABLE date_agg_hist (id serial PRIMARY KEY, d date);
INSERT INTO date_agg_hist (d) VALUES
    ('1999-12-31'),
    ('2000-01-01'),
    ('2000-01-01'),
    ('2000-01-02'),
    ('2000-01-08');
CREATE INDEX date_agg_hist_idx ON date_agg_hist USING paradedb (id, d);

-- date_histogram uses day buckets and formats each key back to a DATE.
SELECT * FROM paradedb.aggregate(
    index => 'date_agg_hist_idx',
    query => paradedb.all(),
    agg => '{"by_day": {"date_histogram": {"field": "d", "fixed_interval": "1d"}}}'
);

-- A multi-day interval still buckets whole days.
SELECT * FROM paradedb.aggregate(
    index => 'date_agg_hist_idx',
    query => paradedb.all(),
    agg => '{"by_week": {"date_histogram": {"field": "d", "fixed_interval": "7d"}}}'
);

DROP TABLE date_agg CASCADE;
DROP TABLE date_agg_hist CASCADE;
