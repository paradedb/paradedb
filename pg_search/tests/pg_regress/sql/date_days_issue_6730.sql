-- Regression coverage for DATE values across PostgreSQL's full DateADT range (#6730, #6878).
--
-- #6730 rejected dates beyond 294276 AD because DATE was stored as a midnight timestamp. Since
-- #6878, DATE is stored as its DateADT day count, so every finite PostgreSQL date, both
-- infinities, DATE arrays, and dateranges must index, round-trip, filter, sort, and aggregate.
CREATE EXTENSION IF NOT EXISTS pg_search;
SET datestyle = 'ISO, MDY';
SET paradedb.enable_aggregate_custom_scan TO on;

-- 1. Build an index over far-future, far-past, and infinite dates.
CREATE TABLE issue_6730_build (id serial PRIMARY KEY, d date);
INSERT INTO issue_6730_build (d) VALUES
    ('2024-01-01'),
    ('294277-01-01'),
    ('5874897-12-31'),
    ('4714-11-24 BC'),
    ('infinity'),
    ('-infinity');
CREATE INDEX issue_6730_build_idx ON issue_6730_build USING paradedb (id, d);

-- Every value is indexed and read back unchanged.
SELECT id, d FROM issue_6730_build WHERE id @@@ pdb.all() ORDER BY d;

-- Range filters on either side of the old 294276 AD limit.
SELECT id, d FROM issue_6730_build
WHERE id @@@ pdb.all() AND d > '294276-12-31'::date
ORDER BY d;

SELECT id, d FROM issue_6730_build
WHERE id @@@ pdb.all() AND d < '4714-11-25 BC'::date
ORDER BY d;

-- Sorting by the far extremes uses the same day-count fast field.
SELECT id, d FROM issue_6730_build
WHERE id @@@ pdb.all() AND d IS NOT NULL
ORDER BY d DESC
LIMIT 3;

-- MIN/MAX and GROUP BY aggregate over the full range.
SELECT min(d), max(d), count(*) FROM issue_6730_build WHERE id @@@ pdb.all();

SELECT d, count(*) FROM issue_6730_build
WHERE id @@@ pdb.all()
GROUP BY d
ORDER BY d NULLS LAST;

-- 2. Inserting a far date after the index exists succeeds and remains searchable.
CREATE TABLE issue_6730_insert (id serial PRIMARY KEY, d date);
CREATE INDEX issue_6730_insert_idx ON issue_6730_insert USING paradedb (id, d);
INSERT INTO issue_6730_insert (d) VALUES ('2024-01-01'), ('294277-01-01'), ('5874897-12-31');
SELECT id, d FROM issue_6730_insert WHERE id @@@ pdb.all() ORDER BY id;

-- 3. Immutable insert mode stores the same day counts.
CREATE TABLE issue_6730_immutable (id serial PRIMARY KEY, d date);
CREATE INDEX issue_6730_immutable_idx ON issue_6730_immutable
USING paradedb (id, d) WITH (mutable_segment_rows = 0);
INSERT INTO issue_6730_immutable (d) VALUES ('2024-01-01'), ('294277-01-01');
SELECT id, d FROM issue_6730_immutable WHERE id @@@ pdb.all() ORDER BY id;

-- 4. The finite boundaries themselves round-trip.
CREATE TABLE issue_6730_boundary (id serial PRIMARY KEY, d date);
CREATE INDEX issue_6730_boundary_idx ON issue_6730_boundary USING paradedb (id, d);
INSERT INTO issue_6730_boundary (d) VALUES ('4714-11-24 BC'), ('5874897-12-31');
SELECT id, d FROM issue_6730_boundary WHERE id @@@ pdb.all() ORDER BY d;

-- 5. DATE arrays use the same day-count representation.
CREATE TABLE issue_6730_array (id serial PRIMARY KEY, dates date[]);
CREATE INDEX issue_6730_array_idx ON issue_6730_array USING paradedb (id, dates);
INSERT INTO issue_6730_array (dates) VALUES
    ('{2024-01-01}'),
    ('{294277-01-01,5874897-12-31}'),
    ('{4714-11-24 BC,infinity,-infinity}');
SELECT id, dates FROM issue_6730_array WHERE id @@@ pdb.all() ORDER BY id;

-- 6. Dateranges containing dates at the far boundaries round-trip and filter.
CREATE TABLE issue_6730_range (id serial PRIMARY KEY, r daterange);
CREATE INDEX issue_6730_range_idx ON issue_6730_range USING paradedb (id, r);
INSERT INTO issue_6730_range (r) VALUES
    ('[2024-01-01,2024-02-01)'),
    -- A daterange cannot hold an inclusive upper bound at the last finite date: PostgreSQL
    -- canonicalizes it to the next exclusive day, which does not exist.
    ('[294277-01-01,5874897-12-31)'),
    ('[4714-11-24 BC,4714-11-25 BC)');
SELECT id, r FROM issue_6730_range WHERE id @@@ pdb.all() ORDER BY id;

SELECT id FROM issue_6730_range
WHERE id @@@ pdb.all() AND r @> '294277-01-01'::date
ORDER BY id;

DROP TABLE issue_6730_build;
DROP TABLE issue_6730_insert;
DROP TABLE issue_6730_immutable;
DROP TABLE issue_6730_boundary;
DROP TABLE issue_6730_array;
DROP TABLE issue_6730_range;
RESET datestyle;
RESET paradedb.enable_aggregate_custom_scan;
