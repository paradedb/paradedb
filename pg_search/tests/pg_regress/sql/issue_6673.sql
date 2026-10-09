-- Tests range and term queries whose bounds lie at or beyond the edges of an
-- integer field's range (issue #6673).
CREATE EXTENSION IF NOT EXISTS pg_search;

CREATE TABLE issue_6673 (id BIGSERIAL PRIMARY KEY, x BIGINT);
INSERT INTO issue_6673 (x) SELECT g FROM generate_series(1, 1000) g;
CREATE INDEX issue_6673_idx ON issue_6673 USING paradedb (id, x);

-- JSON bounds: at the maximum, above it, and a large upper bound.
SELECT count(*) FROM issue_6673 WHERE id @@@
  '{"range":{"field":"x","lower_bound":{"included":9223372036854775807},"upper_bound":null}}'::jsonb;
SELECT count(*) FROM issue_6673 WHERE id @@@
  '{"range":{"field":"x","lower_bound":{"included":9223372036854775808},"upper_bound":null}}'::jsonb;
SELECT count(*) FROM issue_6673 WHERE id @@@
  '{"range":{"field":"x","lower_bound":null,"upper_bound":{"included":18000000000000000000}}}'::jsonb;

-- pdb.range bounds at and beyond the maximum.
SELECT count(*) FROM issue_6673
WHERE x @@@ pdb.range(numrange(NULL, 18000000000000000000, '[]'));
SELECT count(*) FROM issue_6673
WHERE x @@@ pdb.range(numrange(NULL, 9223372036854775807, '[]'));
SELECT count(*) FROM issue_6673
WHERE x @@@ pdb.range(numrange(9223372036854775807, NULL, '()'));

-- Bounds inside the range keep their meaning.
SELECT count(*) FROM issue_6673 WHERE id @@@
  '{"range":{"field":"x","lower_bound":{"excluded":500},"upper_bound":{"included":600}}}'::jsonb;
SELECT count(*) FROM issue_6673
WHERE x @@@ pdb.range(numrange(500, 600, '(]'));

-- With a row holding the maximum, nothing lies above it.
INSERT INTO issue_6673 (x) VALUES (9223372036854775807);
SELECT count(*) FROM issue_6673
WHERE x @@@ pdb.range(numrange(18000000000000000000, NULL, '[]'));
SELECT count(*) FROM issue_6673
WHERE x @@@ pdb.term(18000000000000000000::numeric);
SELECT count(*) FROM issue_6673
WHERE x @@@ pdb.range(numrange(9223372036854775807, NULL, '[]'));

-- One row at each edge, and one at -446744073709551616, which is what
-- 18000000000000000000 becomes when it wraps around as a bigint.
CREATE TABLE issue_6673_edges (id serial PRIMARY KEY, x bigint);
INSERT INTO issue_6673_edges (x) VALUES
    (-9223372036854775808), (-446744073709551616), (5), (9223372036854775807);
CREATE INDEX issue_6673_edges_idx ON issue_6673_edges USING paradedb (id, x);

-- JSON bounds at the maximum.
SELECT x FROM issue_6673_edges WHERE id @@@
  '{"range":{"field":"x","lower_bound":{"excluded":9223372036854775807},"upper_bound":null}}'::jsonb
ORDER BY x;
SELECT x FROM issue_6673_edges WHERE id @@@
  '{"range":{"field":"x","lower_bound":null,"upper_bound":{"included":9223372036854775807}}}'::jsonb
ORDER BY x;

-- JSON terms above the maximum match nothing.
SELECT x FROM issue_6673_edges WHERE id @@@
  '{"term":{"field":"x","value":18000000000000000000}}'::jsonb
ORDER BY x;
SELECT x FROM issue_6673_edges WHERE id @@@
  '{"term_set":{"terms":[{"field":"x","value":18000000000000000000},{"field":"x","value":5}]}}'::jsonb
ORDER BY x;

-- pdb.range bounds below the minimum.
SELECT x FROM issue_6673_edges
WHERE x @@@ pdb.range(numrange(NULL, -9223372036854775809, '[]')) ORDER BY x;
SELECT x FROM issue_6673_edges
WHERE x @@@ pdb.range(numrange(-9223372036854775809, NULL, '()')) ORDER BY x;

-- pdb.term and pdb.term_set values beyond either edge match nothing.
SELECT x FROM issue_6673_edges
WHERE x @@@ pdb.term(-9223372036854775809::numeric) ORDER BY x;
SELECT x FROM issue_6673_edges
WHERE x @@@ pdb.term_set(ARRAY[-9223372036854775809, 5, 18000000000000000000]::numeric[])
ORDER BY x;

-- A bigint constant at the maximum is pushed down as a range query.
SELECT x FROM issue_6673_edges
WHERE id @@@ pdb.all() AND x > 9223372036854775807::bigint ORDER BY x;
SELECT x FROM issue_6673_edges
WHERE id @@@ pdb.all() AND x <= 9223372036854775807::bigint ORDER BY x;

-- An oid field holds values from 0 to 18446744073709551615 in the index.
CREATE TABLE issue_6673_oid (id serial PRIMARY KEY, o oid);
INSERT INTO issue_6673_oid (o) VALUES (1), (5), (4294967295);
CREATE INDEX issue_6673_oid_idx ON issue_6673_oid USING paradedb (id, o);
SELECT o FROM issue_6673_oid WHERE id @@@
  '{"range":{"field":"o","lower_bound":null,"upper_bound":{"included":18446744073709551615}}}'::jsonb
ORDER BY o;
SELECT o FROM issue_6673_oid WHERE id @@@
  '{"range":{"field":"o","lower_bound":{"excluded":18446744073709551615},"upper_bound":null}}'::jsonb
ORDER BY o;
SELECT o FROM issue_6673_oid
WHERE o @@@ pdb.range(numrange(-5, NULL, '[]')) ORDER BY o;

-- 'infinity' as a JSON string bound on a timestamp field is the largest value.
CREATE TABLE issue_6673_ts (id serial PRIMARY KEY, t timestamp);
INSERT INTO issue_6673_ts (t) VALUES ('-infinity'), ('2024-01-01'), ('infinity');
CREATE INDEX issue_6673_ts_idx ON issue_6673_ts USING paradedb (id, t);
SELECT t FROM issue_6673_ts WHERE id @@@
  '{"range":{"field":"t","lower_bound":{"excluded":"infinity"},"upper_bound":null}}'::jsonb
ORDER BY t;
SELECT t FROM issue_6673_ts WHERE id @@@
  '{"range":{"field":"t","lower_bound":null,"upper_bound":{"included":"infinity"}}}'::jsonb
ORDER BY t;

-- Segment pruning reads the same values: a segment that holds only the maximum
-- must stay under NOT of a query that matches nothing.
CREATE TABLE issue_6673_pruning (id serial PRIMARY KEY, x bigint NOT NULL);
CREATE INDEX issue_6673_pruning_idx ON issue_6673_pruning USING paradedb (id, x)
WITH (partition_by = 'x', background_layer_sizes = '0');
SET paradedb.global_mutable_segment_rows = 0;
INSERT INTO issue_6673_pruning (x) VALUES (5);
INSERT INTO issue_6673_pruning (x) VALUES (9223372036854775807);
RESET paradedb.global_mutable_segment_rows;
SELECT x FROM issue_6673_pruning
WHERE id @@@ pdb.all() AND NOT x @@@ pdb.term(18000000000000000000::numeric)
ORDER BY x;

DROP TABLE issue_6673;
DROP TABLE issue_6673_edges;
DROP TABLE issue_6673_oid;
DROP TABLE issue_6673_ts;
DROP TABLE issue_6673_pruning;
