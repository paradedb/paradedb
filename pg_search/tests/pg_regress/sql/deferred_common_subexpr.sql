-- A repeated comparison on a deferred string column becomes a common subexpression, so a
-- projection consumes the column and emits only the boolean. The column is then absent from
-- that projection's own schema, and a stopping rule that reads the schema there leaves the
-- string deferred past the point that compares it.

CREATE EXTENSION IF NOT EXISTS pg_search;

SET paradedb.enable_aggregate_custom_scan TO on;
SET paradedb.enable_join_custom_scan TO on;
SET max_parallel_workers_per_gather TO 0;

CREATE TABLE dcs_users (
    id INTEGER PRIMARY KEY,
    name TEXT
);
CREATE TABLE dcs_products (
    id INTEGER PRIMARY KEY,
    quantity INTEGER,
    age INTEGER,
    price NUMERIC(8, 2),
    name TEXT
);
CREATE TABLE dcs_orders (
    id INTEGER PRIMARY KEY,
    name TEXT,
    color TEXT
);

CREATE INDEX dcs_users_idx ON dcs_users
USING paradedb (id, (name::pdb.unicode_words('columnar=true')));
CREATE INDEX dcs_products_idx ON dcs_products
USING paradedb (id, quantity, age, price, (name::pdb.unicode_words('columnar=true')));
CREATE INDEX dcs_orders_idx ON dcs_orders
USING paradedb (id, (name::pdb.unicode_words('columnar=true')), (color::pdb.unicode_words('columnar=true')));

INSERT INTO dcs_users VALUES (1, 'bob'), (2, 'ann');
INSERT INTO dcs_products VALUES (1, 10, 5, 9.99, 'bob'), (2, 20, 6, 19.99, 'ann');
INSERT INTO dcs_orders VALUES (1, 'bob', 'blue'), (2, NULL, 'blue');

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.quantity, SUM(p.age), COUNT(*), pdb.agg('{"sum":{"field":"p.price"}}')
FROM dcs_users u JOIN dcs_products p ON u.id = p.id LEFT JOIN dcs_orders o ON p.id = o.id
WHERE (u.name @@@ 'bob' AND u.name IS NOT NULL)
  AND (p.name = 'bob' AND ((o.color = 'blue' AND o.name = 'bob') OR (o.name IS NULL OR o.color = 'blue')))
GROUP BY p.quantity
ORDER BY p.quantity;

SELECT p.quantity, SUM(p.age), COUNT(*), pdb.agg('{"sum":{"field":"p.price"}}')
FROM dcs_users u JOIN dcs_products p ON u.id = p.id LEFT JOIN dcs_orders o ON p.id = o.id
WHERE (u.name @@@ 'bob' AND u.name IS NOT NULL)
  AND (p.name = 'bob' AND ((o.color = 'blue' AND o.name = 'bob') OR (o.name IS NULL OR o.color = 'blue')))
GROUP BY p.quantity
ORDER BY p.quantity;

DROP TABLE dcs_orders;
DROP TABLE dcs_products;
DROP TABLE dcs_users;
