-- Regression test for issue #6023:
-- A self-join that sorts by a late-materialized column used to key the
-- deferred `FFHelper` map by index relid only. Both aliases share one
-- index, so one helper decoded for both. Each alias has its own fast-field
-- list, so `a.name`'s `ff_index` can address the wrong slot on `b`'s helper
-- and `LIMIT` keeps the wrong rows.
--
-- Keying helpers by `(plan_position, indexrelid)` keeps one helper per alias.

SET max_parallel_workers_per_gather = 0;
SET enable_indexscan TO OFF;

CREATE EXTENSION IF NOT EXISTS pg_search;

DROP TABLE IF EXISTS sj CASCADE;

CREATE TABLE sj (
    id SERIAL8 PRIMARY KEY,
    message TEXT COLLATE "C",
    ref_id INTEGER,
    name TEXT COLLATE "C"
);

CREATE INDEX sj_idx ON sj USING paradedb (id, message, ref_id, (name::pdb.literal))
WITH (key_field = 'id');

INSERT INTO sj (message, ref_id, name)
SELECT (ARRAY['beer wine','beer','wine','cheese'])[1 + (i % 4)] || ' ' || i::text,
       1 + (i % 50), 'name ' || lpad(i::text, 5, '0')
FROM generate_series(1, 2000) AS s(i);
ANALYZE sj;

SET paradedb.enable_join_custom_scan = on;

SELECT a.name, b.name
FROM sj a JOIN sj b ON a.ref_id = b.id
WHERE a.message @@@ 'beer'
ORDER BY a.name, b.name
LIMIT 5;

DROP TABLE sj CASCADE;
