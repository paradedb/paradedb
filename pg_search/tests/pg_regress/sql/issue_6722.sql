-- Tests that a `date` of 'infinity' or '-infinity' can be indexed, inserted
-- and searched, and that the index returns it as infinity (issue #6722).
CREATE EXTENSION IF NOT EXISTS pg_search;

CREATE TABLE issue_6722 (id serial PRIMARY KEY, d date);
INSERT INTO issue_6722 (d) VALUES
    ('2024-01-01'), ('infinity'), ('-infinity'), (NULL);

-- The index build succeeds with infinite values in the table.
CREATE INDEX issue_6722_idx ON issue_6722 USING paradedb (id, d);

-- An INSERT after the build goes to the mutable segment, and searches still work.
INSERT INTO issue_6722 (d) VALUES ('infinity'), ('2025-06-15');
SELECT count(*) FROM issue_6722 WHERE id @@@ pdb.all();

-- Infinite constants in range predicates, with the expected Postgres semantics.
EXPLAIN (COSTS OFF)
SELECT id, d FROM issue_6722
WHERE id @@@ pdb.all() AND d < 'infinity'::date ORDER BY id;
SELECT id, d FROM issue_6722
WHERE id @@@ pdb.all() AND d < 'infinity'::date ORDER BY id;
SELECT id, d FROM issue_6722
WHERE id @@@ pdb.all() AND d <= 'infinity'::date ORDER BY id;
SELECT id, d FROM issue_6722
WHERE id @@@ pdb.all() AND d > 'infinity'::date ORDER BY id;
SELECT id, d FROM issue_6722
WHERE id @@@ pdb.all() AND d >= 'infinity'::date ORDER BY id;
SELECT id, d FROM issue_6722
WHERE id @@@ pdb.all() AND d = 'infinity'::date ORDER BY id;
SELECT id, d FROM issue_6722
WHERE id @@@ pdb.all() AND d > '-infinity'::date ORDER BY id;
SELECT id, d FROM issue_6722
WHERE id @@@ pdb.all() AND d <= '-infinity'::date ORDER BY id;
SELECT id, d FROM issue_6722
WHERE id @@@ pdb.all() AND d BETWEEN '-infinity'::date AND '2024-12-31'::date ORDER BY id;

-- Values read back from the index keep their infinity.
SELECT d FROM issue_6722 WHERE id @@@ pdb.all() ORDER BY d LIMIT 3;
SELECT d FROM issue_6722 WHERE id @@@ pdb.all() ORDER BY d DESC NULLS LAST LIMIT 3;

-- With mutable_segment_rows = 0 the INSERT itself builds a segment.
CREATE TABLE issue_6722_immutable (id serial PRIMARY KEY, d date);
CREATE INDEX issue_6722_immutable_idx ON issue_6722_immutable
USING paradedb (id, d) WITH (mutable_segment_rows = 0);
INSERT INTO issue_6722_immutable (d) VALUES ('-infinity'), ('2024-01-01');
SELECT id, d FROM issue_6722_immutable WHERE id @@@ pdb.all() ORDER BY id;

-- An infinite `timestamp` constant is sent to the index the same way, so it works too.
CREATE TABLE issue_6722_ts (id serial PRIMARY KEY, t timestamp);
INSERT INTO issue_6722_ts (t) VALUES ('2024-01-01'), ('infinity'), ('-infinity');
CREATE INDEX issue_6722_ts_idx ON issue_6722_ts USING paradedb (id, t);
SELECT id, t FROM issue_6722_ts
WHERE id @@@ pdb.all() AND t < 'infinity'::timestamp ORDER BY id;
SELECT id, t FROM issue_6722_ts
WHERE id @@@ pdb.all() AND t > '-infinity'::timestamp ORDER BY id;

-- A `daterange` with an infinite bound goes through the same conversion.
CREATE TABLE issue_6722_range (id serial PRIMARY KEY, dr daterange);
INSERT INTO issue_6722_range (dr) VALUES
    ('[2024-01-01,infinity)'), ('[-infinity,2024-06-01)'), ('[2024-02-01,2024-03-01)');
CREATE INDEX issue_6722_range_idx ON issue_6722_range USING paradedb (id, dr);
SELECT id, dr FROM issue_6722_range
WHERE dr @@@ pdb.range_term('2030-01-01'::date) ORDER BY id;

DROP TABLE issue_6722;
DROP TABLE issue_6722_immutable;
DROP TABLE issue_6722_ts;
DROP TABLE issue_6722_range;
