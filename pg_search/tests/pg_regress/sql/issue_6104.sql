-- =====================================================================
-- A range-partitioned join over NUMERIC `partition_by` fields (#6104).
-- The split points a partitioned build stores are in the index's stored
-- form: a numeric(10,2) bound is the scaled I64 (123 for 1.23) and a
-- NUMERIC(p>18) bound is the lexicographic Bytes the column stores. The
-- partition query must take them as stored. Converting them again the way
-- user input is converted moves the executed Numeric64 bound 10^scale
-- times past the segments the partition prunes to, silently dropping the
-- rows the middle partitions hold, and errors out on NumericBytes with
-- "Cannot convert non-numeric value".
-- =====================================================================

CREATE EXTENSION IF NOT EXISTS pg_search;

SET paradedb.enable_join_custom_scan TO on;
SET paradedb.enable_range_partitioned_join TO on;
SET max_parallel_workers TO 8;
SET min_parallel_table_scan_size TO 0;
SET parallel_setup_cost TO 0;
SET parallel_tuple_cost TO 0;
SET max_parallel_maintenance_workers TO 0;

-- =====================================================================
-- Numeric64: NUMERIC(10,2) is stored as the scaled I64 (1.23 -> 123).
-- =====================================================================

CREATE TABLE n64_build (id bigserial PRIMARY KEY, amount numeric(10,2) NOT NULL, tag text);
CREATE TABLE n64_probe (id bigserial PRIMARY KEY, amount numeric(10,2) NOT NULL, tag text);

INSERT INTO n64_build (amount, tag)
SELECT ((g * 37) % 10000)::numeric / 100, CASE WHEN g % 2 = 0 THEN 'even' ELSE 'odd' END
FROM generate_series(1, 2000) g;
INSERT INTO n64_probe (amount, tag)
SELECT ((g * 53) % 10000)::numeric / 100, CASE WHEN g % 2 = 0 THEN 'even' ELSE 'odd' END
FROM generate_series(1, 2000) g;

CREATE INDEX n64_build_idx ON n64_build USING paradedb (id, amount, tag)
WITH (key_field = 'id', partition_by = 'amount', target_segment_count = 4,
      numeric_fields = '{"amount": {"fast": true}}',
      text_fields = '{"tag": {"tokenizer": {"type": "keyword"}, "fast": true}}');
CREATE INDEX n64_probe_idx ON n64_probe USING paradedb (id, amount, tag)
WITH (key_field = 'id', partition_by = 'amount', target_segment_count = 4,
      numeric_fields = '{"amount": {"fast": true}}',
      text_fields = '{"tag": {"tokenizer": {"type": "keyword"}, "fast": true}}');

-- =====================================================================
-- Serial baseline.
-- =====================================================================

SET max_parallel_workers_per_gather TO 0;

SELECT count(*)
FROM n64_build b JOIN n64_probe p ON b.amount = p.amount
WHERE b.tag @@@ 'even' AND p.id @@@ pdb.all();

SELECT b.id, p.id
FROM n64_build b JOIN n64_probe p ON b.amount = p.amount
WHERE b.tag @@@ 'even' AND p.id @@@ pdb.all()
ORDER BY b.id, p.id
LIMIT 8;

-- =====================================================================
-- MPP: the scans show the build's boundaries and must return the same
-- rows the serial plan does.
-- =====================================================================

SET max_parallel_workers_per_gather TO 3;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT count(*)
FROM n64_build b JOIN n64_probe p ON b.amount = p.amount
WHERE b.tag @@@ 'even' AND p.id @@@ pdb.all();

SELECT count(*)
FROM n64_build b JOIN n64_probe p ON b.amount = p.amount
WHERE b.tag @@@ 'even' AND p.id @@@ pdb.all();

SELECT b.id, p.id
FROM n64_build b JOIN n64_probe p ON b.amount = p.amount
WHERE b.tag @@@ 'even' AND p.id @@@ pdb.all()
ORDER BY b.id, p.id
LIMIT 8;

-- =====================================================================
-- NumericBytes: NUMERIC without a precision limit is stored as
-- lexicographic Bytes, so the sampled bounds cannot be converted the way
-- user input is at all. Negatives exercise the sortable-negative layout.
-- =====================================================================

CREATE TABLE nb_build (id bigserial PRIMARY KEY, amount numeric NOT NULL, tag text);
CREATE TABLE nb_probe (id bigserial PRIMARY KEY, amount numeric NOT NULL, tag text);

INSERT INTO nb_build (amount, tag)
SELECT CASE WHEN g % 3 = 0 THEN -(((g * 37) % 10000)::numeric / 100)
            ELSE ((g * 37) % 10000)::numeric / 100 END,
       CASE WHEN g % 2 = 0 THEN 'even' ELSE 'odd' END
FROM generate_series(1, 2000) g;
INSERT INTO nb_probe (amount, tag)
SELECT CASE WHEN g % 5 = 0 THEN -(((g * 53) % 10000)::numeric / 100)
            ELSE ((g * 53) % 10000)::numeric / 100 END,
       CASE WHEN g % 2 = 0 THEN 'even' ELSE 'odd' END
FROM generate_series(1, 2000) g;

CREATE INDEX nb_build_idx ON nb_build USING paradedb (id, amount, tag)
WITH (key_field = 'id', partition_by = 'amount', target_segment_count = 4,
      numeric_fields = '{"amount": {"fast": true}}',
      text_fields = '{"tag": {"tokenizer": {"type": "keyword"}, "fast": true}}');
CREATE INDEX nb_probe_idx ON nb_probe USING paradedb (id, amount, tag)
WITH (key_field = 'id', partition_by = 'amount', target_segment_count = 4,
      numeric_fields = '{"amount": {"fast": true}}',
      text_fields = '{"tag": {"tokenizer": {"type": "keyword"}, "fast": true}}');

SET max_parallel_workers_per_gather TO 0;

SELECT count(*)
FROM nb_build b JOIN nb_probe p ON b.amount = p.amount
WHERE b.tag @@@ 'even' AND p.id @@@ pdb.all();

-- The MPP plan is asserted without execution: the range-partitioned scans
-- declare the build's boundaries as the stored lexicographic `Bytes`
-- (`Bytes([...])` below is the split point's stored form, e.g. the encoded
-- `-91.07`). Executing a NumericBytes range-partitioned join currently hits
-- a separate pre-existing backend crash in the join machinery for bytes join
-- keys (it reproduces with `paradedb.enable_range_partitioned_join TO off`),
-- tracked in its own issue; the executed partition semantics are covered by
-- the numeric(10,2) section above, which runs the same stored-bound query
-- through its scaled-I64 form.

SET max_parallel_workers_per_gather TO 3;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT count(*)
FROM nb_build b JOIN nb_probe p ON b.amount = p.amount
WHERE b.tag @@@ 'even' AND p.id @@@ pdb.all();

DROP TABLE nb_probe;
DROP TABLE nb_build;
DROP TABLE n64_probe;
DROP TABLE n64_build;
