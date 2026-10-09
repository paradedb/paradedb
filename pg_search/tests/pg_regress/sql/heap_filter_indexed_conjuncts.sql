-- A predicate the index cannot answer (here a UDF over non-indexed columns) runs as a heap
-- filter inside the Tantivy query. It must only be evaluated on the rows the indexed predicates
-- matched, in every scan type. The UDF counts its evaluations in a sequence to make that visible.
CREATE EXTENSION IF NOT EXISTS pg_search;
SET max_parallel_workers_per_gather = 0;
SET enable_indexscan TO off;

DROP TABLE IF EXISTS hfic_items CASCADE;
DROP TABLE IF EXISTS hfic_orders CASCADE;

CREATE TABLE hfic_items (
    id serial PRIMARY KEY,
    category text,
    lat double precision,
    lng double precision
);
-- 100 of the 1000 rows are in category 'a' (every tenth id).
INSERT INTO hfic_items (category, lat, lng)
SELECT CASE WHEN g % 10 = 0 THEN 'a' ELSE 'b' END, g, g
FROM generate_series(1, 1000) g;
CREATE INDEX hfic_items_idx ON hfic_items
USING bm25 (id, category) WITH (target_segment_count = 1);

CREATE TABLE hfic_orders (
    id serial PRIMARY KEY,
    item_id int,
    note text
);
INSERT INTO hfic_orders (item_id, note)
SELECT g, 'order' FROM generate_series(1, 1000) g;
CREATE INDEX hfic_orders_idx ON hfic_orders
USING bm25 (id, item_id, note) WITH (target_segment_count = 1);

CREATE SEQUENCE hfic_calls1;
CREATE SEQUENCE hfic_calls2;
-- PARALLEL UNSAFE keeps every evaluation in this backend, where the sequence counts it.
CREATE FUNCTION hfic_within(lat double precision, lng double precision, lim double precision, counter regclass)
RETURNS boolean LANGUAGE plpgsql IMMUTABLE PARALLEL UNSAFE AS $$
BEGIN
    PERFORM nextval(counter);
    RETURN lat + lng < lim;
END
$$;
CREATE FUNCTION hfic_odd_decade(lat double precision, counter regclass)
RETURNS boolean LANGUAGE plpgsql IMMUTABLE PARALLEL UNSAFE AS $$
BEGIN
    PERFORM nextval(counter);
    RETURN (lat::int / 10) % 2 = 1;
END
$$;

-- The index alone matches 100 rows; `lat + lng < 1000` keeps 49 of them, and
-- `hfic_odd_decade` keeps every other one (50).
SELECT count(*) FROM hfic_items WHERE category @@@ 'a';

-- Aggregate scan: the heap filter wraps the indexed clause, so the UDF runs once per index
-- match rather than once per row of the table.
SET paradedb.enable_aggregate_custom_scan TO on;
ALTER SEQUENCE hfic_calls1 RESTART;
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT count(*) FROM hfic_items
WHERE category @@@ 'a' AND hfic_within(lat, lng, 1000, 'hfic_calls1');
SELECT count(*) FROM hfic_items
WHERE category @@@ 'a' AND hfic_within(lat, lng, 1000, 'hfic_calls1');
SELECT pg_sequence_last_value('hfic_calls1') AS udf_evaluations;

-- Join scan: same for the search side of a join.
SET paradedb.enable_join_custom_scan TO on;
ALTER SEQUENCE hfic_calls1 RESTART;
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT i.id, o.id
FROM hfic_items i JOIN hfic_orders o ON o.item_id = i.id
WHERE i.category @@@ 'a' AND hfic_within(i.lat, i.lng, 1000, 'hfic_calls1')
ORDER BY i.id LIMIT 5;
SELECT i.id, o.id
FROM hfic_items i JOIN hfic_orders o ON o.item_id = i.id
WHERE i.category @@@ 'a' AND hfic_within(i.lat, i.lng, 1000, 'hfic_calls1')
ORDER BY i.id LIMIT 5;
SELECT pg_sequence_last_value('hfic_calls1') AS udf_evaluations;
SET paradedb.enable_join_custom_scan TO off;

DROP FUNCTION hfic_within(double precision, double precision, double precision, regclass);
DROP FUNCTION hfic_odd_decade(double precision, regclass);
DROP SEQUENCE hfic_calls1;
DROP SEQUENCE hfic_calls2;
DROP TABLE hfic_orders;
DROP TABLE hfic_items;
