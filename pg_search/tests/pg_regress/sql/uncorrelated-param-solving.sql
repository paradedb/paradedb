
CREATE EXTENSION IF NOT EXISTS pg_search;

DROP TABLE IF EXISTS items CASCADE;
CREATE TABLE items (
    id text NOT NULL,
    group_id text NOT NULL,
    status text NOT NULL,
    created_at timestamp NOT NULL
);

ALTER TABLE ONLY items
    ADD CONSTRAINT items_pkey PRIMARY KEY (id);

CREATE INDEX items_idx ON items USING bm25 (
    id,
    group_id,
    status,
    created_at
)
WITH (
    key_field = 'id',
    text_fields = '{
        "group_id": { "fast": true, "tokenizer": { "type": "keyword" } },
        "status": { "fast": true, "tokenizer": { "type": "keyword" } }
    }',
    datetime_fields = '{"created_at": {}}'
);

INSERT INTO items (id, group_id, status, created_at)
VALUES
    ('4', 'g1', 'posted', '2025-01-01 12:00:00'),
    ('3', 'g1', 'pending', '2025-01-01 12:00:00'),
    ('2', 'g1', 'posted', '2025-01-01 11:00:00'),
    ('1', 'g1', 'pending', '2025-01-01 10:00:00');

-- The subqueries here are uncorrelated, and should get InitPlan nodes which we can
-- solve at `BeginCustomScan` time. They should NOT get a "heap filter" plan, as that would
-- prevent index pushdown.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, status, created_at
FROM items
WHERE
    group_id = 'g1'
    AND (id @@@ paradedb.all())
    AND status @@@ 'IN [posted pending]'
    AND created_at <= (SELECT created_at FROM items WHERE id = '4')
    AND (
        created_at < (SELECT created_at FROM items WHERE id = '4')
        OR
        (id < '4' AND created_at = (SELECT created_at FROM items WHERE id = '4'))
    )
ORDER BY created_at DESC, id DESC
LIMIT 100;

SELECT id, status, created_at
FROM items
WHERE
    group_id = 'g1'
    AND (id @@@ paradedb.all())
    AND status @@@ 'IN [posted pending]'
    AND created_at <= (SELECT created_at FROM items WHERE id = '4')
    AND (
        created_at < (SELECT created_at FROM items WHERE id = '4')
        OR
        (id < '4' AND created_at = (SELECT created_at FROM items WHERE id = '4'))
    )
ORDER BY created_at DESC, id DESC
LIMIT 100;
