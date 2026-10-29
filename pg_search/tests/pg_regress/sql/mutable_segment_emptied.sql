\i common/common_setup.sql

-- A mutable segment whose rows were all deleted and vacuumed away has no live document
-- left. Such a segment must not take part in a query: a scorer that starts from "every
-- document" would otherwise produce a phantom document 0 with no values behind it.

DROP TABLE IF EXISTS emptied CASCADE;

CREATE TABLE emptied (
    id SERIAL8 PRIMARY KEY,
    name TEXT,
    color TEXT,
    -- In no index, so a query that projects it has to go to the heap.
    note TEXT
);

-- The first rows go to a sealed segment, the last one to a mutable segment of its own.
SET paradedb.global_mutable_segment_rows TO 0;

CREATE INDEX emptied_idx ON emptied USING paradedb (id, name, color)
WITH (
    text_fields = '{
        "name": {"tokenizer": {"type": "keyword"}, "fast": true},
        "color": {"tokenizer": {"type": "keyword"}, "fast": true}
    }'
);

INSERT INTO emptied (name, color, note)
SELECT 'bob', 'blue', 'row ' || g FROM generate_series(1, 5) g;

SET paradedb.global_mutable_segment_rows TO 10;
INSERT INTO emptied (name, color, note) VALUES ('alice', 'red', 'the one that goes');

DELETE FROM emptied WHERE name = 'alice';
VACUUM emptied;

SELECT mutable, num_docs, num_deleted FROM paradedb.index_info('emptied_idx') ORDER BY mutable;

-- Served from the index alone.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id FROM emptied WHERE NOT (name @@@ 'carol') ORDER BY id;

SELECT id FROM emptied WHERE NOT (name @@@ 'carol') ORDER BY id;

-- The same predicate, but every row is fetched from the heap.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, note FROM emptied WHERE NOT (name @@@ 'carol') ORDER BY id;

SELECT id, note FROM emptied WHERE NOT (name @@@ 'carol') ORDER BY id;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT count(*) FROM emptied WHERE color IS NOT NULL AND NOT (name @@@ 'carol');

SELECT count(*) FROM emptied WHERE color IS NOT NULL AND NOT (name @@@ 'carol');

ALTER INDEX emptied_idx SET (background_layer_sizes = '0', layer_sizes = '0');

-- Retire an immutable segment after deletions across multiple VACUUMs.
DELETE FROM emptied WHERE id = 1;
VACUUM emptied;
SELECT mutable, num_docs, num_deleted FROM paradedb.index_info('emptied_idx');

DELETE FROM emptied;
VACUUM emptied;
SELECT count(*) FROM paradedb.index_info('emptied_idx', true);
SELECT count(*) FROM emptied WHERE name @@@ 'bob';

-- Repeated mutable flushes must reclaim the immutable segments they replace.
INSERT INTO emptied (name, color)
SELECT 'alice', 'red' FROM generate_series(1, 5);
UPDATE emptied SET name = 'bob';
VACUUM emptied;
UPDATE emptied SET name = 'carol';
VACUUM emptied;
UPDATE emptied SET name = 'david';
VACUUM emptied;

SELECT mutable, num_docs, num_deleted FROM paradedb.index_info('emptied_idx', true);
SELECT count(*) FROM emptied WHERE name @@@ 'david';

-- VACUUM must reuse reclaimed pages even without an assigned transaction ID.
SELECT pg_relation_size('emptied_idx') AS initial_size \gset
\set ECHO none
SELECT format('UPDATE emptied SET name = %L', 'name' || i), 'VACUUM emptied'
FROM generate_series(1, 50) AS i \gexec
\set ECHO all
SELECT pg_relation_size('emptied_idx') < 2 * :initial_size AS reuses_pages;

DROP TABLE emptied;

RESET paradedb.global_mutable_segment_rows;

-- Multiple mutable segments flush together, including empty inputs.
DROP TABLE IF EXISTS vacuum_mutables;
CREATE TABLE vacuum_mutables (id INTEGER PRIMARY KEY, message TEXT);
CREATE INDEX vacuum_mutables_idx ON vacuum_mutables USING paradedb (id, message)
WITH (mutable_segment_rows = 2, layer_sizes = '0', background_layer_sizes = '0');

INSERT INTO vacuum_mutables VALUES (1, 'red fox'), (2, 'blue dog');
INSERT INTO vacuum_mutables VALUES (3, 'red dog');
INSERT INTO vacuum_mutables VALUES (4, 'red cat');
SELECT mutable, num_docs FROM paradedb.index_info('vacuum_mutables_idx') ORDER BY num_docs;
DELETE FROM vacuum_mutables WHERE id = 2;
VACUUM vacuum_mutables;
SELECT mutable, num_docs FROM paradedb.index_info('vacuum_mutables_idx', true);
SELECT id FROM vacuum_mutables WHERE message @@@ 'red' ORDER BY id;

TRUNCATE vacuum_mutables;
INSERT INTO vacuum_mutables VALUES (1, 'red fox'), (2, 'blue dog');
INSERT INTO vacuum_mutables VALUES (3, 'red dog');
INSERT INTO vacuum_mutables VALUES (4, 'red cat');
DELETE FROM vacuum_mutables WHERE id <= 3;
VACUUM vacuum_mutables;
SELECT mutable, num_docs FROM paradedb.index_info('vacuum_mutables_idx', true);
SELECT id FROM vacuum_mutables WHERE message @@@ 'red' ORDER BY id;

TRUNCATE vacuum_mutables;
INSERT INTO vacuum_mutables VALUES (1, 'red fox'), (2, 'blue dog');
INSERT INTO vacuum_mutables VALUES (3, 'red dog');
INSERT INTO vacuum_mutables VALUES (4, 'red cat');
DELETE FROM vacuum_mutables;
VACUUM vacuum_mutables;
SELECT count(*) FROM paradedb.index_info('vacuum_mutables_idx', true);
SELECT count(*) FROM vacuum_mutables WHERE message @@@ 'red';
DROP TABLE vacuum_mutables;
