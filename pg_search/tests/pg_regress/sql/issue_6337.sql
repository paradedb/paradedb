-- UUID scalar predicates must survive JoinScan's PostgreSQL-expression fallback.
CREATE EXTENSION IF NOT EXISTS pg_search;

CREATE TABLE scalar_uuid_users (id bigint PRIMARY KEY, name text);
CREATE TABLE scalar_uuid_products (id bigint PRIMARY KEY, uuid uuid);
CREATE TABLE scalar_uuid_orders (id bigint PRIMARY KEY);
INSERT INTO scalar_uuid_users SELECT i, 'bob' FROM generate_series(1, 6) i;
INSERT INTO scalar_uuid_products VALUES
    (1, '00000000-0000-0000-0000-000000000001'),
    (2, '00000000-0000-0000-0000-000000000001'),
    (3, '00000000-0000-0000-0000-000000000001'),
    (4, '00000000-0000-0000-0000-000000000001'),
    (5, '550e8400-e29b-41d4-a716-446655440000'),
    (6, NULL);
INSERT INTO scalar_uuid_orders SELECT i FROM generate_series(1, 6) i;
CREATE INDEX ON scalar_uuid_users USING paradedb (id, (name::pdb.literal));
CREATE INDEX ON scalar_uuid_products USING paradedb (id, uuid);
CREATE INDEX ON scalar_uuid_orders USING paradedb (id);

SET paradedb.enable_custom_scan = false;
SET paradedb.enable_custom_scan_without_operator = false;
SET paradedb.enable_filter_pushdown = true;
SET paradedb.enable_join_custom_scan = true;
SET max_parallel_workers_per_gather = 0;

EXPLAIN (COSTS OFF, TIMING OFF)
SELECT u.id, u.name FROM scalar_uuid_users u
JOIN scalar_uuid_products p ON u.id = p.id
JOIN scalar_uuid_orders o ON p.id = o.id
WHERE NOT (u.id >= 4 AND p.uuid = '550e8400-e29b-41d4-a716-446655440000'::uuid)
  AND u.id @@@ pdb.all()
ORDER BY u.id, p.id, o.id LIMIT 27 OFFSET 2;

SELECT u.id, u.name FROM scalar_uuid_users u
JOIN scalar_uuid_products p ON u.id = p.id
JOIN scalar_uuid_orders o ON p.id = o.id
WHERE NOT (u.id >= 4 AND p.uuid = '550e8400-e29b-41d4-a716-446655440000'::uuid)
  AND u.id @@@ pdb.all()
ORDER BY u.id, p.id, o.id LIMIT 27 OFFSET 2;

SET paradedb.enable_join_custom_scan = false;
SELECT u.id, u.name FROM scalar_uuid_users u
JOIN scalar_uuid_products p ON u.id = p.id
JOIN scalar_uuid_orders o ON p.id = o.id
WHERE NOT (u.id >= 4 AND p.uuid = '550e8400-e29b-41d4-a716-446655440000'::uuid)
  AND u.id @@@ pdb.all()
ORDER BY u.id, p.id, o.id LIMIT 27 OFFSET 2;

DROP TABLE scalar_uuid_users, scalar_uuid_products, scalar_uuid_orders;
RESET paradedb.enable_custom_scan;
RESET paradedb.enable_custom_scan_without_operator;
RESET paradedb.enable_filter_pushdown;
RESET paradedb.enable_join_custom_scan;
RESET max_parallel_workers_per_gather;
