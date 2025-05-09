DROP TABLE IF EXISTS users;
DROP TABLE IF EXISTS products;
DROP TABLE IF EXISTS orders;

CREATE TABLE users
(
    id    serial8 not null primary key,
    name  text,
    color varchar,
    age   varchar
);

INSERT into users (name, color, age)
VALUES ('bob', 'blue', 20);

INSERT into users (name, color, age)
SELECT(ARRAY ['alice','bob','cloe', 'sally','brandy','brisket','anchovy']::text[])[(floor(random() * 7) + 1)::int],
      (ARRAY ['red','green','blue', 'orange','purple','pink','yellow']::text[])[(floor(random() * 7) + 1)::int],
      (floor(random() * 100) + 1)::int::text
FROM generate_series(1, 10);    -- could make larger, but 10 finds failures and is fast

CREATE INDEX idxusers ON users USING bm25 (id, name, color, age)
    WITH (
    key_field = 'id',
    text_fields = '
            {
                "name": { "tokenizer": { "type": "keyword" } },
                "color": { "tokenizer": { "type": "keyword" } },
                "age": { "tokenizer": { "type": "keyword" } }
            }'
    );
CREATE INDEX idxusers_name ON users (name);
CREATE INDEX idxusers_color ON users (color);
CREATE INDEX idxusers_age ON users (age);
ANALYZE;

CREATE TABLE products
(
    id    serial8 not null primary key,
    name  text,
    color varchar,
    age   varchar
);

INSERT into products (name, color, age)
VALUES ('bob', 'blue', 20);

INSERT into products (name, color, age)
SELECT(ARRAY ['alice','bob','cloe', 'sally','brandy','brisket','anchovy']::text[])[(floor(random() * 7) + 1)::int],
      (ARRAY ['red','green','blue', 'orange','purple','pink','yellow']::text[])[(floor(random() * 7) + 1)::int],
      (floor(random() * 100) + 1)::int::text
FROM generate_series(1, 10);    -- could make larger, but 10 finds failures and is fast

CREATE INDEX idxproducts ON products USING bm25 (id, name, color, age)
    WITH (
    key_field = 'id',
    text_fields = '
            {
                "name": { "tokenizer": { "type": "keyword" } },
                "color": { "tokenizer": { "type": "keyword" } },
                "age": { "tokenizer": { "type": "keyword" } }
            }'
    );
CREATE INDEX idxproducts_name ON products (name);
CREATE INDEX idxproducts_color ON products (color);
CREATE INDEX idxproducts_age ON products (age);
ANALYZE;

CREATE TABLE orders
(
    id    serial8 not null primary key,
    name  text,
    color varchar,
    age   varchar
);

INSERT into orders (name, color, age)
VALUES ('bob', 'blue', 20);

INSERT into orders (name, color, age)
SELECT(ARRAY ['alice','bob','cloe', 'sally','brandy','brisket','anchovy']::text[])[(floor(random() * 7) + 1)::int],
      (ARRAY ['red','green','blue', 'orange','purple','pink','yellow']::text[])[(floor(random() * 7) + 1)::int],
      (floor(random() * 100) + 1)::int::text
FROM generate_series(1, 10);    -- could make larger, but 10 finds failures and is fast

CREATE INDEX idxorders ON orders USING bm25 (id, name, color, age)
    WITH (
    key_field = 'id',
    text_fields = '
            {
                "name": { "tokenizer": { "type": "keyword" } },
                "color": { "tokenizer": { "type": "keyword" } },
                "age": { "tokenizer": { "type": "keyword" } }
            }'
    );
CREATE INDEX idxorders_name ON orders (name);
CREATE INDEX idxorders_color ON orders (color);
CREATE INDEX idxorders_age ON orders (age);
ANALYZE;

---- idx=50 ----
---- connector=AND ----
-- pg=12, bm25=4
SELECT COUNT(*) FROM users LEFT JOIN products ON users.color = products.color  WHERE (users.name  =  'bob') AND ((users.color  =  'blue') AND (users.name  =  'bob')) AND (products.name  =  'bob') OR (NOT (products.age  =  '20')) AND (users.name  =  'bob') OR ((users.color  =  'blue') AND (users.name  =  'bob')) AND (products.color  =  'blue') OR (NOT (products.name  =  'bob'))
SELECT COUNT(*) FROM users LEFT JOIN products ON users.color = products.color  WHERE (users.name @@@ 'bob') AND ((users.color @@@ 'blue') AND (users.name @@@ 'bob')) AND (products.name @@@ 'bob') OR (NOT (products.age @@@ '20')) AND (users.name @@@ 'bob') OR ((users.color @@@ 'blue') AND (users.name @@@ 'bob')) AND (products.color @@@ 'blue') OR (NOT (products.name @@@ 'bob'))

---- idx=4 ----
---- connector=AND NOT ----
-- pg=4, bm25=0
SELECT COUNT(*) FROM users JOIN orders ON users.color = orders.color  WHERE (users.name  =  'bob') AND ((users.color  =  'blue') OR (NOT (users.name  =  'bob'))) AND NOT ((orders.name  =  'bob') AND (orders.age  =  '20')) OR (orders.age  =  '20') AND NOT (users.name  =  'bob') OR ((users.color  =  'blue') OR (NOT (users.name  =  'bob'))) AND NOT ((orders.name  =  'bob') OR (orders.age  =  '20')) AND (orders.name  =  'bob')
SELECT COUNT(*) FROM users JOIN orders ON users.color = orders.color  WHERE (users.name @@@ 'bob') AND ((users.color @@@ 'blue') OR (NOT (users.name @@@ 'bob'))) AND NOT ((orders.name @@@ 'bob') AND (orders.age @@@ '20')) OR (orders.age @@@ '20') AND NOT (users.name @@@ 'bob') OR ((users.color @@@ 'blue') OR (NOT (users.name @@@ 'bob'))) AND NOT ((orders.name @@@ 'bob') OR (orders.age @@@ '20')) AND (orders.name @@@ 'bob')

---- idx=37 ----
---- connector=AND NOT ----
-- pg=1, bm25=0
SELECT COUNT(*) FROM users JOIN products ON users.name = products.name  WHERE (users.color  =  'blue') AND ((users.name  =  'bob') OR (NOT (users.color  =  'blue'))) AND NOT (products.color  =  'blue') OR ((products.color  =  'blue') AND (products.color  =  'blue')) AND NOT (users.color  =  'blue') OR ((users.name  =  'bob') OR (NOT (users.color  =  'blue'))) AND NOT (products.color  =  'blue') AND ((products.color  =  'blue') OR (products.color  =  'blue'))
SELECT COUNT(*) FROM users JOIN products ON users.name = products.name  WHERE (users.color @@@ 'blue') AND ((users.name @@@ 'bob') OR (NOT (users.color @@@ 'blue'))) AND NOT (products.color @@@ 'blue') OR ((products.color @@@ 'blue') AND (products.color @@@ 'blue')) AND NOT (users.color @@@ 'blue') OR ((users.name @@@ 'bob') OR (NOT (users.color @@@ 'blue'))) AND NOT (products.color @@@ 'blue') AND ((products.color @@@ 'blue') OR (products.color @@@ 'blue'))

---- idx=46 ----
---- connector=AND NOT ----
-- pg=2, bm25=0
SELECT COUNT(*) FROM users LEFT JOIN products ON users.name = products.name  WHERE (users.color  =  'blue') AND ((users.age  =  '20') OR (NOT (users.color  =  'blue'))) AND NOT (products.color  =  'blue') OR ((products.age  =  '20') OR (products.age  =  '20')) AND NOT (users.color  =  'blue') OR ((users.age  =  '20') OR (NOT (users.color  =  'blue'))) AND NOT (products.age  =  '20') AND (NOT (NOT (products.name  =  'bob')))
SELECT COUNT(*) FROM users LEFT JOIN products ON users.name = products.name  WHERE (users.color @@@ 'blue') AND ((users.age @@@ '20') OR (NOT (users.color @@@ 'blue'))) AND NOT (products.color @@@ 'blue') OR ((products.age @@@ '20') OR (products.age @@@ '20')) AND NOT (users.color @@@ 'blue') OR ((users.age @@@ '20') OR (NOT (users.color @@@ 'blue'))) AND NOT (products.age @@@ '20') AND (NOT (NOT (products.name @@@ 'bob')))

---- idx=55 ----
---- connector=AND NOT ----
-- pg=3, bm25=0
SELECT COUNT(*) FROM users RIGHT JOIN products ON users.name = products.name  WHERE (users.color  =  'blue') AND ((NOT (users.color  =  'blue')) OR (users.color  =  'blue')) AND NOT (products.age  =  '20') OR ((products.name  =  'bob') OR (products.age  =  '20')) AND NOT (users.color  =  'blue') OR ((NOT (users.color  =  'blue')) OR (users.color  =  'blue')) AND NOT (products.age  =  '20') AND ((products.color  =  'blue') AND (products.name  =  'bob'))
SELECT COUNT(*) FROM users RIGHT JOIN products ON users.name = products.name  WHERE (users.color @@@ 'blue') AND ((NOT (users.color @@@ 'blue')) OR (users.color @@@ 'blue')) AND NOT (products.age @@@ '20') OR ((products.name @@@ 'bob') OR (products.age @@@ '20')) AND NOT (users.color @@@ 'blue') OR ((NOT (users.color @@@ 'blue')) OR (users.color @@@ 'blue')) AND NOT (products.age @@@ '20') AND ((products.color @@@ 'blue') AND (products.name @@@ 'bob'))

---- idx=83 ----
---- connector=AND NOT ----
-- pg=18, bm25=0
SELECT COUNT(*) FROM orders LEFT JOIN users ON orders.name = users.name  WHERE NOT (NOT ((orders.age  =  '20') OR (NOT (orders.age  =  '20')))) AND NOT (users.age  =  '20') OR ((users.age  =  '20') OR (NOT (users.name  =  'bob'))) AND NOT NOT (NOT ((NOT (orders.name  =  'bob')) OR (orders.name  =  'bob'))) AND NOT (users.age  =  '20') OR ((users.age  =  '20') AND (NOT (users.color  =  'blue')))
SELECT COUNT(*) FROM orders LEFT JOIN users ON orders.name = users.name  WHERE NOT (NOT ((orders.age @@@ '20') OR (NOT (orders.age @@@ '20')))) AND NOT (users.age @@@ '20') OR ((users.age @@@ '20') OR (NOT (users.name @@@ 'bob'))) AND NOT NOT (NOT ((NOT (orders.name @@@ 'bob')) OR (orders.name @@@ 'bob'))) AND NOT (users.age @@@ '20') OR ((users.age @@@ '20') AND (NOT (users.color @@@ 'blue')))

---- idx=92 ----
---- connector=AND NOT ----
-- pg=17, bm25=0
SELECT COUNT(*) FROM orders RIGHT JOIN users ON orders.name = users.name  WHERE NOT ((NOT (orders.color  =  'blue')) AND (NOT (orders.color  =  'blue'))) AND NOT (users.age  =  '20') OR ((NOT (users.color  =  'blue')) OR (users.name  =  'bob')) AND NOT NOT ((NOT (orders.color  =  'blue')) OR (NOT (orders.color  =  'blue'))) AND NOT (users.age  =  '20') OR ((NOT (users.color  =  'blue')) AND (users.color  =  'blue'))
SELECT COUNT(*) FROM orders RIGHT JOIN users ON orders.name = users.name  WHERE NOT ((NOT (orders.color @@@ 'blue')) AND (NOT (orders.color @@@ 'blue'))) AND NOT (users.age @@@ '20') OR ((NOT (users.color @@@ 'blue')) OR (users.name @@@ 'bob')) AND NOT NOT ((NOT (orders.color @@@ 'blue')) OR (NOT (orders.color @@@ 'blue'))) AND NOT (users.age @@@ '20') OR ((NOT (users.color @@@ 'blue')) AND (users.color @@@ 'blue'))

---- idx=74 ----
---- connector=AND NOT ----
-- pg=19, bm25=18
SELECT COUNT(*) FROM orders JOIN users ON orders.name = users.name  WHERE ((orders.age  =  '20') AND (orders.age  =  '20')) AND (orders.color  =  'blue') AND NOT (users.age  =  '20') OR ((users.name  =  'bob') OR (NOT (users.name  =  'bob'))) AND NOT ((orders.age  =  '20') AND (orders.age  =  '20')) OR (orders.color  =  'blue') AND NOT (users.age  =  '20') OR ((users.name  =  'bob') AND (NOT (users.color  =  'blue')))
SELECT COUNT(*) FROM orders JOIN users ON orders.name = users.name  WHERE ((orders.age @@@ '20') AND (orders.age @@@ '20')) AND (orders.color @@@ 'blue') AND NOT (users.age @@@ '20') OR ((users.name @@@ 'bob') OR (NOT (users.name @@@ 'bob'))) AND NOT ((orders.age @@@ '20') AND (orders.age @@@ '20')) OR (orders.color @@@ 'blue') AND NOT (users.age @@@ '20') OR ((users.name @@@ 'bob') AND (NOT (users.color @@@ 'blue')))