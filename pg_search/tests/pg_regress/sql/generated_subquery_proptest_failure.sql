CREATE EXTENSION IF NOT EXISTS pg_search;
DROP TABLE IF EXISTS products;
DROP TABLE IF EXISTS orders;
CREATE TABLE products
(
    id    SERIAL8 NOT NULL PRIMARY KEY,
    uuid  UUID NOT NULL,
    name  TEXT,
    color VARCHAR,
    age   VARCHAR
);

CREATE TABLE orders
(
    id    SERIAL8 NOT NULL PRIMARY KEY,
    uuid  UUID NOT NULL,
    name  TEXT,
    color VARCHAR,
    age   VARCHAR
);

-- Note: Create the indexes before inserting rows to encourage multiple segments being created.
CREATE INDEX idxproducts ON products USING bm25 (id, uuid, name, color, age)
    WITH (
    target_segment_count = 2,
    key_field = 'id',
    text_fields = '
            {
                "uuid": { "tokenizer": { "type": "keyword" }, "fast": true },
                "name": { "tokenizer": { "type": "keyword" }, "fast": true },
                "color": { "tokenizer": { "type": "keyword" }, "fast": true },
                "age": { "tokenizer": { "type": "keyword" }, "fast": true }
            }'
    );
CREATE INDEX idxorders ON orders USING bm25 (id, uuid, name, color, age)
    WITH (
    target_segment_count = 2,
    key_field = 'id',
    text_fields = '
            {
                "uuid": { "tokenizer": { "type": "keyword" }, "fast": true },
                "name": { "tokenizer": { "type": "keyword" }, "fast": true },
                "color": { "tokenizer": { "type": "keyword" }, "fast": true },
                "age": { "tokenizer": { "type": "keyword" }, "fast": true }
            }'
    );

--
-- this INSERT pattern ensures we have 2 segments, which seems to be important in triggering the bug
--
INSERT INTO products VALUES (1, '748d481d-767c-480b-9a86-c23bae4c11eb', 'bob', 'blue', '20');
INSERT INTO products VALUES (2, '240090f5-911c-4e5c-ab7e-89fe7c4bfc49', 'sally', 'red', '12'),
(3, 'cc6cf1ae-8ade-44dc-852d-56af3f322017', 'brandy', 'orange', '63'),
(4, '1ee9eee8-6d3e-49e3-b4c9-4c60ab98515d', 'bob', 'yellow', '95'),
(5, '1c503366-f3ad-4d53-9d4e-99d1dccb803d', 'cloe', 'blue', '90'),
(6, '27128958-06cc-4768-bd40-19013f5a8e9c', 'brandy', 'orange', '41'),
(7, '76d6bc91-303a-466f-a1f5-c4a16b142d4d', 'cloe', 'purple', '53'),
(8, '3b5fb9ca-ec65-42b9-b541-7cbc9f332f96', 'sally', 'purple', '63'),
(9, 'a3be0a59-7a33-4ccf-8975-e822afe0e78b', 'cloe', 'blue', '27'),
(10, 'b39ee58a-2c41-45eb-8deb-ba82cf3e6982', 'bob', 'red', '23'),
(11, '90ff382f-c9e3-4f31-ba11-45c070c68238', 'cloe', 'blue', '60');

--
-- this INSERT pattern ensures we have 2 segments, which seems to be important in triggering the bug
--
INSERT INTO orders VALUES (1, 'a13fa345-5753-47a7-856d-c9b096523aae', 'bob', 'blue', '20');
INSERT INTO orders VALUES (2, '94a3f501-f3e6-4c63-9a47-ee287c07cd43', 'anchovy', 'red', '42'),
(3, '9406362c-b674-401c-8bb9-a8f9e246d7e8', 'bob', 'green', '20'),
(4, '1bd26fee-8d73-4141-96ac-6da19d24cfdb', 'brisket', 'orange', '92'),
(5, '76b1f33d-56f2-422f-a25c-4aa90592b14a', 'anchovy', 'yellow', '46'),
(6, 'b595218e-2af2-4992-ace5-d56591ede4ff', 'brisket', 'green', '6'),
(7, '919d36f5-dae6-461d-8445-be5fcf1d04b6', 'brandy', 'red', '34'),
(8, 'c411f2d7-da85-44ea-a49d-7a3297b22b49', 'anchovy', 'purple', '48'),
(9, '0cd0a07b-38b8-4577-b879-2534975e5c05', 'alice', 'purple', '22'),
(10, '09f87c13-18a1-40df-b7d1-20848d771d5e', 'alice', 'purple', '52'),
(11, 'a38b514a-454f-4204-99bf-5b4882c941bd', 'sally', 'blue', '21');

CREATE INDEX idxproducts_uuid ON products (uuid);
CREATE INDEX idxproducts_name ON products (name);
CREATE INDEX idxproducts_color ON products (color);
CREATE INDEX idxproducts_age ON products (age);

CREATE INDEX idxorders_uuid ON orders (uuid);
CREATE INDEX idxorders_name ON orders (name);
CREATE INDEX idxorders_color ON orders (color);
CREATE INDEX idxorders_age ON orders (age);

ANALYZE;

--
-- these two queries should return the same count:  3
SELECT COUNT(*) FROM products WHERE products.color IN (SELECT color FROM orders WHERE NOT (orders.age  =  '20') ORDER BY orders.id LIMIT 9) AND (products.name  =  'bob') AND (products.name  =  'bob');
SELECT COUNT(*) FROM products WHERE products.color IN (SELECT color FROM orders WHERE NOT (orders.age @@@ '20') ORDER BY orders.id LIMIT 9) AND (products.name @@@ 'bob') AND (products.name @@@ 'bob');