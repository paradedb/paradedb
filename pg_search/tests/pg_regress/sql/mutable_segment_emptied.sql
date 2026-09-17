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
    key_field = 'id',
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

DROP TABLE emptied;

RESET paradedb.global_mutable_segment_rows;
