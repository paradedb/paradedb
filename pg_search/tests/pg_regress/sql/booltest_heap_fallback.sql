CREATE EXTENSION IF NOT EXISTS pg_search;

DROP TABLE IF EXISTS bt_docs CASCADE;

CREATE TABLE bt_docs (
                         id SERIAL,
                         content TEXT,
                         flag BOOLEAN
);

CREATE INDEX bt_docs_idx
    ON bt_docs
    USING bm25 (id, content)
    WITH (key_field='id');

INSERT INTO bt_docs (content, flag) VALUES
                                        ('hello world', true),
                                        ('hello parade', false),
                                        ('other text', true);

SET paradedb.enable_filter_pushdown = on;

EXPLAIN (COSTS OFF)
SELECT *
FROM bt_docs
WHERE content @@@ 'hello'
  AND flag IS TRUE;