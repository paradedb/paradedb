DROP TABLE IF EXISTS mock_items;

CREATE TABLE mock_items (
    id SERIAL PRIMARY KEY,
    description TEXT,
    sparse_embedding SPARSE
);

INSERT INTO mock_items (description, sparse_embedding)
VALUES
    ('Ergonomic metal keyboard', '[0,1,0,1]'),
    ('Plastic Keyboard', '[0,2,0,2]');
    ('Sleek running shoes', 5),
    ('White jogging shoes', 3),
    ('Generic shoes', 4),
    ('Compact digital camera', 5),
    ('Hardcover book on history', 2),
    ('Organic green tea', 3),
    ('Modern wall clock', 4),
    ('Colorful kids toy', 1),
    ('Soft cotton shirt', 5),
    ('Innovative wireless earbuds', 5),
    ('Sturdy hiking boots', 4),
    ('Elegant glass table', 3),
    ('Refreshing face wash', 2),
    ('High-resolution DSLR', 4),
    ('Paperback romantic novel', 3),
    ('Freshly ground coffee beans', 5),
    ('Artistic ceramic vase', 4),
    ('Interactive board game', 3),
    ('Slim-fit denim jeans', 5),
    ('Fast charging power bank', 4),
    ('Comfortable slippers', 3),
    ('Classic leather sofa', 5),
    ('Anti-aging serum', 4),
    ('Portable tripod stand', 4),
    ('Mystery detective novel', 2),
    ('Organic breakfast cereal', 5),
    ('Designer wall paintings', 5),
    ('Robot building kit', 4),
    ('Sporty tank top', 4),
    ('Bluetooth-enabled speaker', 3),
    ('Winter woolen socks', 5),
    ('Rustic bookshelf', 4),
    ('Moisturizing lip balm', 4),
    ('Lightweight camera bag', 5),
    ('Historical fiction book', 3),
    ('Pure honey jar', 4),
    ('Handcrafted wooden frame', 5),
    ('Plush teddy bear', 4),
    ('Warm woolen sweater', 3);

ALTER TABLE mock_items DROP COLUMN IF EXISTS sparse_embedding;
ALTER TABLE mock_items ADD COLUMN sparse_embedding sparse;

WITH NumberedRows AS (
    SELECT ctid,
           ROW_NUMBER() OVER () as row_num
    FROM mock_items
)
UPDATE mock_items m
SET sparse_embedding = (
    SELECT '[' || string_agg(
        CASE 
            WHEN random() < 0.3 THEN '0'
            ELSE trunc(random()*10)::text
        END, ',') || ']'
    FROM generate_series(1,10)
    WHERE m.ctid = m.ctid
)::sparse;
