-- COUNT(col) over a GROUP BY col is served by the bucket's doc_count. The NULL group
-- must still count 0, and keys whose NULL sentinel a real value can match must keep
-- their metric.

\i common/common_setup.sql

SET paradedb.enable_aggregate_custom_scan TO on;

CREATE TABLE count_key (
    id SERIAL PRIMARY KEY,
    a TEXT,
    b TEXT,
    flag BOOLEAN
);

INSERT INTO count_key (a, b, flag) VALUES
    ('x', 'p', true),
    ('x', NULL, true),
    ('y', 'p', false),
    ('y', NULL, NULL),
    (NULL, 'p', NULL),
    (NULL, 'q', NULL),
    (NULL, NULL, NULL),
    (NULL, NULL, NULL);

CREATE INDEX count_key_idx ON count_key
USING paradedb (id, (a::pdb.literal), (b::pdb.literal), flag);

-- Text key: COUNT(a) is served by doc_count, so no value_count metric
EXPLAIN (COSTS OFF, VERBOSE)
SELECT a, COUNT(a), COUNT(*)
FROM count_key WHERE id @@@ pdb.all()
GROUP BY a ORDER BY a NULLS LAST;

-- NULL key
SELECT a, COUNT(a), COUNT(*)
FROM count_key WHERE id @@@ pdb.all()
GROUP BY a ORDER BY a NULLS LAST;

-- Two-column GROUP BY, second key NULL
SELECT a, b, COUNT(b), COUNT(*)
FROM count_key WHERE id @@@ pdb.all()
GROUP BY a, b ORDER BY a NULLS LAST, b NULLS LAST;

-- NULL group has the most rows
SELECT a, COUNT(a)
FROM count_key WHERE id @@@ pdb.all()
GROUP BY a ORDER BY COUNT(a) DESC, a NULLS LAST;

SELECT a, COUNT(a)
FROM count_key WHERE id @@@ pdb.all()
GROUP BY a ORDER BY COUNT(a) DESC, a NULLS LAST LIMIT 1;

-- A FILTER changes which rows count, so the group's doc_count can't serve it
SELECT a, COUNT(a) FILTER (WHERE b = 'p')
FROM count_key WHERE id @@@ pdb.all()
GROUP BY a ORDER BY a NULLS LAST;

-- Bool key
SELECT flag, COUNT(flag), COUNT(*)
FROM count_key WHERE id @@@ pdb.all()
GROUP BY flag ORDER BY flag NULLS LAST;

-- Real values equal to a NULL sentinel keep their count. The table has no NULLs, so
-- PostgreSQL's answer is exact; the keys are not projected because how those groups are
-- labelled is a separate issue (#6627).
CREATE TABLE count_sentinel (
    id SERIAL PRIMARY KEY,
    big BIGINT,
    ts TIMESTAMP
);

INSERT INTO count_sentinel (big, ts) VALUES
    (1, '2024-01-01'),
    (9223372036854775807, 'infinity'),
    (9223372036854775807, 'infinity'),
    (-9223372036854775808, '-infinity');

CREATE INDEX count_sentinel_idx ON count_sentinel USING paradedb (id, big, ts);

-- bigint key: COUNT(big) keeps its value_count metric
EXPLAIN (COSTS OFF, VERBOSE)
SELECT COUNT(big), COUNT(*)
FROM count_sentinel WHERE id @@@ pdb.all()
GROUP BY big ORDER BY 2, 1;

SELECT COUNT(big), COUNT(*)
FROM count_sentinel WHERE id @@@ pdb.all()
GROUP BY big ORDER BY 2, 1;

SELECT COUNT(ts), COUNT(*)
FROM count_sentinel WHERE id @@@ pdb.all()
GROUP BY ts ORDER BY 2, 1;

-- JSON key: strings, numbers equal to the NULL sentinels, and a missing key. Only the
-- string sentinel bucket reads as NULL.
CREATE TABLE count_json (
    id SERIAL PRIMARY KEY,
    meta JSONB
);

INSERT INTO count_json (meta) VALUES
    ('{"c": 1}'),
    ('{"c": -9223372036854775808}'),
    ('{"c": -9223372036854775808}'),
    ('{"c": 1.7976931348623157e308}'),
    ('{"c": 18446744073709551615}'),
    ('{"c": "x"}'),
    ('{"c": "x"}'),
    ('{}');

CREATE INDEX count_json_idx ON count_json
USING paradedb (id, (meta::pdb.literal_normalized));

SELECT COUNT(meta->>'c'), COUNT(*)
FROM count_json WHERE id @@@ pdb.all()
GROUP BY meta->>'c' ORDER BY 2, 1;

RESET paradedb.enable_aggregate_custom_scan;
DROP TABLE count_key;
DROP TABLE count_sentinel;
DROP TABLE count_json;
