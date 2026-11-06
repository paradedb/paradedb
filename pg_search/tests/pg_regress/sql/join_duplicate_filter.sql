CREATE EXTENSION IF NOT EXISTS pg_search;
SET max_parallel_workers_per_gather = 0;
SET paradedb.enable_aggregate_custom_scan = off;

CREATE TABLE j6953_users (id bigint PRIMARY KEY, group_id bigint, tags text[]);
CREATE TABLE j6953_products (id bigint PRIMARY KEY, group_id bigint, name text, color text, tags text[]);
CREATE TABLE j6953_orders (id bigint PRIMARY KEY, group_id bigint, uuid uuid);

INSERT INTO j6953_users VALUES (4, 1, ARRAY['a']), (5, 1, ARRAY['b']);
INSERT INTO j6953_products VALUES
    (1, 1, 'alice', 'red', ARRAY['one']),
    (2, 1, 'alice', 'red', ARRAY['two']),
    (3, 1, 'bob', NULL, ARRAY['three']),
    (4, 1, 'alice', 'red', ARRAY['four']);
INSERT INTO j6953_orders VALUES (11, 1, '550e8400-e29b-41d4-a716-446655440000');

CREATE INDEX ON j6953_users USING paradedb (id, group_id, (tags::pdb.literal));
CREATE INDEX ON j6953_products USING paradedb (id, group_id, (name::pdb.literal), (color::pdb.literal), (tags::pdb.literal));
CREATE INDEX ON j6953_orders USING paradedb (id, group_id, uuid);

-- users.id and products.id must remain distinct when the join filter is pushed down.
EXPLAIN (VERBOSE, COSTS OFF)
SELECT DISTINCT p.id, u.id, o.id, t, s
FROM j6953_users u
RIGHT JOIN j6953_products p ON p.group_id = u.group_id
RIGHT JOIN j6953_orders o ON o.group_id = p.group_id
CROSS JOIN LATERAL unnest(p.tags) t
CROSS JOIN LATERAL unnest(u.tags) s
WHERE (u.id = 4 AND o.uuid IS NOT NULL) OR (p.color IS NULL AND p.name === 'bob')
ORDER BY p.id, u.id, o.id, t, s LIMIT 29;

SELECT DISTINCT p.id, u.id, o.id, t, s
FROM j6953_users u
RIGHT JOIN j6953_products p ON p.group_id = u.group_id
RIGHT JOIN j6953_orders o ON o.group_id = p.group_id
CROSS JOIN LATERAL unnest(p.tags) t
CROSS JOIN LATERAL unnest(u.tags) s
WHERE (u.id = 4 AND o.uuid IS NOT NULL) OR (p.color IS NULL AND p.name === 'bob')
ORDER BY p.id, u.id, o.id, t, s LIMIT 29;

-- both same-named ids participate in the predicate.
EXPLAIN (COSTS OFF)
SELECT DISTINCT p.id, u.id
FROM j6953_users u
RIGHT JOIN j6953_products p ON p.group_id = u.group_id
RIGHT JOIN j6953_orders o ON o.group_id = p.group_id
CROSS JOIN LATERAL unnest(p.tags) t
CROSS JOIN LATERAL unnest(u.tags) s
WHERE p.name === 'alice' AND (u.id = 4 OR p.id = 2)
ORDER BY p.id, u.id LIMIT 29;

SELECT DISTINCT p.id, u.id
FROM j6953_users u
RIGHT JOIN j6953_products p ON p.group_id = u.group_id
RIGHT JOIN j6953_orders o ON o.group_id = p.group_id
CROSS JOIN LATERAL unnest(p.tags) t
CROSS JOIN LATERAL unnest(u.tags) s
WHERE p.name === 'alice' AND (u.id = 4 OR p.id = 2)
ORDER BY p.id, u.id LIMIT 29;

SET paradedb.enable_join_custom_scan = off;
SELECT DISTINCT p.id, u.id, o.id, t, s
FROM j6953_users u
RIGHT JOIN j6953_products p ON p.group_id = u.group_id
RIGHT JOIN j6953_orders o ON o.group_id = p.group_id
CROSS JOIN LATERAL unnest(p.tags) t
CROSS JOIN LATERAL unnest(u.tags) s
WHERE (u.id = 4 AND o.uuid IS NOT NULL) OR (p.color IS NULL AND p.name === 'bob')
ORDER BY p.id, u.id, o.id, t, s LIMIT 29;

SELECT DISTINCT p.id, u.id
FROM j6953_users u
RIGHT JOIN j6953_products p ON p.group_id = u.group_id
RIGHT JOIN j6953_orders o ON o.group_id = p.group_id
CROSS JOIN LATERAL unnest(p.tags) t
CROSS JOIN LATERAL unnest(u.tags) s
WHERE p.name === 'alice' AND (u.id = 4 OR p.id = 2)
ORDER BY p.id, u.id LIMIT 29;

DROP TABLE j6953_orders CASCADE;
DROP TABLE j6953_products CASCADE;
DROP TABLE j6953_users CASCADE;
RESET paradedb.enable_join_custom_scan;
RESET paradedb.enable_aggregate_custom_scan;
RESET max_parallel_workers_per_gather;
