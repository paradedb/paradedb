CREATE EXTENSION
IF NOT EXISTS pg_bm25;

CREATE TABLE products
(
    id SERIAL PRIMARY KEY,
    description TEXT,
    rating INTEGER CHECK (
        rating BETWEEN 1
        AND 5
    ),
    category VARCHAR(255)
);

CREATE INDEX idxproducts ON products USING bm25 ((products.*));
CREATE INDEX idxparadedb_mock_items ON paradedb.mock_items USING bm25 ((paradedb.mock_items.*));

INSERT INTO
    products
    (description, rating, category)
VALUES
    ('Ergonomic metal keyboard', 4, 'Electronics'),
    ('Plastic Keyboard', 4, 'Electronics'),
    ('Sleek running shoes', 5, 'Footwear'),
    ('White jogging shoes', 3, 'Footwear'),
    ('Generic shoes', 4, 'Footwear'),
    ('Compact digital camera', 5, 'Photography'),
    ('Hardcover book on history', 2, 'Books'),
    ('Organic green tea', 3, 'Groceries'),
    ('Modern wall clock', 4, 'Home Decor'),
    ('Colorful kids toy', 1, 'Toys'),
    ('Soft cotton shirt', 5, 'Apparel'),
    ('Innovative wireless earbuds', 5, 'Electronics'),
    ('Sturdy hiking boots', 4, 'Footwear'),
    ('Elegant glass table', 3, 'Furniture'),
    ('Refreshing face wash', 2, 'Beauty'),
    ('High-resolution DSLR', 4, 'Photography'),
    ('Paperback romantic novel', 3, 'Books'),
    ('Freshly ground coffee beans', 5, 'Groceries'),
    ('Artistic ceramic vase', 4, 'Home Decor'),
    ('Interactive board game', 3, 'Toys'),
    ('Slim-fit denim jeans', 5, 'Apparel'),
    ('Fast charging power bank', 4, 'Electronics'),
    ('Comfortable slippers', 3, 'Footwear'),
    ('Classic leather sofa', 5, 'Furniture'),
    ('Anti-aging serum', 4, 'Beauty'),
    ('Portable tripod stand', 4, 'Photography'),
    ('Mystery detective novel', 2, 'Books'),
    ('Organic breakfast cereal', 5, 'Groceries'),
    ('Designer wall paintings', 5, 'Home Decor'),
    ('Robot building kit', 4, 'Toys'),
    ('Sporty tank top', 4, 'Apparel'),
    ('Bluetooth-enabled speaker', 3, 'Electronics'),
    ('Winter woolen socks', 5, 'Footwear'),
    ('Rustic bookshelf', 4, 'Furniture'),
    ('Moisturizing lip balm', 4, 'Beauty'),
    ('Lightweight camera bag', 5, 'Photography'),
    ('Historical fiction book', 3, 'Books'),
    ('Pure honey jar', 4, 'Groceries'),
    ('Handcrafted wooden frame', 5, 'Home Decor'),
    ('Plush teddy bear', 4, 'Toys'),
    ('Warm woolen sweater', 3, 'Apparel');

ALTER TABLE products
    ADD COLUMN col_text TEXT DEFAULT 'Sample text',
ADD COLUMN col_varchar VARCHAR
(255) DEFAULT 'Sample text',

ADD COLUMN col_smallint SMALLINT DEFAULT 10,
ADD COLUMN col_bigint BIGINT DEFAULT 1000000,
ADD COLUMN col_integer INTEGER DEFAULT 100,
ADD COLUMN col_oid OID DEFAULT 1,

ADD COLUMN col_float4 FLOAT4 DEFAULT 10.5,
ADD COLUMN col_float8 FLOAT8 DEFAULT 100.55,
ADD COLUMN col_numeric NUMERIC
(5,2) DEFAULT 99.99,
ADD COLUMN col_decimal DECIMAL
(5,2) DEFAULT 88.88,
ADD COLUMN col_real REAL DEFAULT 77.77,
ADD COLUMN col_double DOUBLE PRECISION DEFAULT 66.66,

ADD COLUMN col_bool BOOLEAN DEFAULT TRUE,

ADD COLUMN col_json JSON DEFAULT '{"key": "value"}'::json,
ADD COLUMN col_jsonb JSONB DEFAULT '{"key": "value"}'::jsonb;

CREATE TABLE bm25_search AS SELECT * FROM products;
CREATE INDEX idxbm25_search ON bm25_search USING bm25 ((bm25_search.*));
CREATE TABLE search_config AS SELECT * FROM products;
CREATE INDEX idxsearch_config ON search_config USING bm25 ((search_config.*));
