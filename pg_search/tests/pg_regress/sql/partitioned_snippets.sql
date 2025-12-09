--
-- Test that `UNNEST`, `pdb.snippets`, and partitioned tables all work together.
--
\i common/common_setup.sql

DROP TABLE IF EXISTS logs CASCADE;
CREATE TABLE logs (
    id int,
    message TEXT,
    country VARCHAR(255),
    timestamp TIMESTAMP,
    PRIMARY KEY (id, timestamp)
) PARTITION BY RANGE (timestamp);

CREATE TABLE logs_2020 PARTITION OF logs
    FOR VALUES FROM ('2020-01-01 00:00:00') TO ('2021-01-01 00:00:00');

CREATE TABLE logs_2021 PARTITION OF logs
    FOR VALUES FROM ('2021-01-01 00:00:00') TO ('2022-01-01 00:00:00');

-- Insert data
INSERT INTO logs (id, message, country, timestamp) VALUES
(1, 'The research team from Canada discovered a new species of deep-sea creature. This research is groundbreaking.', 'Canada', '2020-06-01 12:00:00'),
(2, 'In Canada, research on climate change continues. This research will help us understand our planet.', 'Canada', '2020-11-20 08:00:00'),
(3, 'The research institute in Germany developed a new system. Further research is needed.', 'Germany', '2021-07-15 10:00:00'),
(4, 'A joint research project between Canada and Germany is underway. The research is focused on renewable energy.', 'Canada', '2021-03-10 14:00:00'),
(5, 'Canadian research shows new findings. More research is planned.', 'Canada', '2020-02-01 00:00:00'),
(6, 'German research leads to a breakthrough. This research is important.', 'Germany', '2021-09-01 00:00:00');


CREATE INDEX logs_idx
ON logs
USING bm25 (id, message, country)
WITH (key_field = 'id', text_fields = '{"country": {"tokenizer": {"type": "keyword"} }}');


\echo 'Test 1: pdb.snippets (no UNNEST) on parent table'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, pdb.snippets(message, max_num_chars => 25)
FROM logs
WHERE message @@@ 'research' AND country @@@ 'Canada'
ORDER BY id
LIMIT 3;
SELECT id, pdb.snippets(message, max_num_chars => 25)
FROM logs
WHERE message @@@ 'research' AND country @@@ 'Canada'
ORDER BY id
LIMIT 3;


-- Not currently supported: should result in an error asking the user to report the query shape.
\echo 'Test 2: UNNEST(pdb.snippets(...)) on parent table'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, UNNEST(pdb.snippets(message, max_num_chars => 25)) as snippet
FROM logs
WHERE message @@@ 'research' AND country @@@ 'Canada'
ORDER BY id
LIMIT 3;
SELECT id, UNNEST(pdb.snippets(message, max_num_chars => 25)) as snippet
FROM logs
WHERE message @@@ 'research' AND country @@@ 'Canada'
ORDER BY id
LIMIT 3;


\echo 'Test 3: UNNEST(pdb.snippets(...)) on a child table'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, UNNEST(pdb.snippets(message, max_num_chars => 25)) as snippet
FROM logs_2020
WHERE message @@@ 'research' AND country @@@ 'Canada'
ORDER BY id
LIMIT 3;
SELECT id, UNNEST(pdb.snippets(message, max_num_chars => 25)) as snippet
FROM logs_2020
WHERE message @@@ 'research' AND country @@@ 'Canada'
ORDER BY id
LIMIT 3;

\echo 'Test 4: UNNEST(pdb.snippets(...)) on a child table with OFFSET'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, UNNEST(pdb.snippets(message, max_num_chars => 25)) as snippet
FROM logs_2020
WHERE message @@@ 'research' AND country @@@ 'Canada'
ORDER BY id
LIMIT 2 OFFSET 1;
SELECT id, UNNEST(pdb.snippets(message, max_num_chars => 25)) as snippet
FROM logs_2020
WHERE message @@@ 'research' AND country @@@ 'Canada'
ORDER BY id
LIMIT 2 OFFSET 1;


\echo 'Test 5: UNNEST(pdb.snippets(...)) on a child table with LIMIT 0'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, UNNEST(pdb.snippets(message, max_num_chars => 25)) as snippet
FROM logs_2020
WHERE message @@@ 'research' AND country @@@ 'Canada'
ORDER BY id
LIMIT 0;
SELECT id, UNNEST(pdb.snippets(message, max_num_chars => 25)) as snippet
FROM logs_2020
WHERE message @@@ 'research' AND country @@@ 'Canada'
ORDER BY id
LIMIT 0;


\echo 'Test 6: Multiple SRFs on a child table'
-- NOTE: Other SRFs are not yet supported, so this should not get a TopN.
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, UNNEST(pdb.snippets(message, max_num_chars => 25)), generate_series(1,2)
FROM logs_2020
WHERE message @@@ 'research' AND country @@@ 'Canada'
ORDER BY id
LIMIT 3;
SELECT id, UNNEST(pdb.snippets(message, max_num_chars => 25)), generate_series(1,2)
FROM logs_2020
WHERE message @@@ 'research' AND country @@@ 'Canada'
ORDER BY id
LIMIT 3;


DROP TABLE IF EXISTS logs CASCADE;
