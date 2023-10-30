CREATE EXTENSION IF NOT EXISTS pg_sparse;

CREATE TABLE mock_items (
    id SERIAL PRIMARY KEY,
    description TEXT,
    sparse_embedding SPARSE
);

INSERT INTO mock_items (description, sparse_embedding)
VALUES
    ('Ergonomic metal keyboard', '[0,1,0,1,0,3,0,2,0,4]'),
    ('Plastic Keyboard', '[0,2,0,2,0,1,0,4,0,3]'),
    ('Sleek running shoes', '[0,0,0,5,0,1,0,2,0,3]'),
    ('White jogging shoes', '[0,0,0,3,0,4,0,1,0,2]'),
    ('Generic shoes', '[0,0,0,4,0,3,0,2,0,1]'),
    ('Compact digital camera', '[0,0,0,5,0,4,0,3,0,2]'),
    ('Hardcover book on history', '[0,0,2,0,0,3,0,1,0,4]'),
    ('Organic green tea', '[0,0,0,3,0,2,0,4,0,1]'),
    ('Modern wall clock', '[0,0,0,4,0,2,0,1,0,3]'),
    ('Colorful kids toy', '[0,0,0,1,0,2,0,3,0,4]'),
    ('Soft cotton shirt', '[0,0,0,5,0,4,0,3,0,2]'),
    ('Innovative wireless earbuds', '[0,0,0,5,0,1,0,4,0,3]'),
    ('Sturdy hiking boots', '[0,0,0,4,0,3,0,2,0,1]'),
    ('Elegant glass table', '[0,0,0,3,0,1,0,2,0,4]'),
    ('Refreshing face wash', '[0,0,2,0,0,4,0,1,0,3]'),
    ('High-resolution DSLR', '[0,0,0,4,0,3,0,2,0,1]'),
    ('Paperback romantic novel', '[0,0,0,3,0,2,0,4,0,1]'),
    ('Freshly ground coffee beans', '[0,0,0,5,0,4,0,3,0,2]'),
    ('Artistic ceramic vase', '[0,0,0,4,0,3,0,2,0,1]'),
    ('Interactive board game', '[0,0,0,3,0,2,0,4,0,1]'),
    ('Slim-fit denim jeans', '[0,0,0,5,0,4,0,3,0,2]'),
    ('Fast charging power bank', '[0,0,0,4,0,3,0,2,0,1]'),
    ('Comfortable slippers', '[0,0,0,3,0,2,0,4,0,1]'),
    ('Classic leather sofa', '[0,0,0,5,0,4,0,3,0,2]'),
    ('Anti-aging serum', '[0,0,0,4,0,3,0,2,0,1]'),
    ('Portable tripod stand', '[0,0,0,4,0,3,0,2,0,1]'),
    ('Mystery detective novel', '[0,0,2,0,0,4,0,1,0,3]'),
    ('Organic breakfast cereal', '[0,0,0,5,0,4,0,3,0,2]'),
    ('Designer wall paintings', '[0,0,0,5,0,4,0,3,0,2]'),
    ('Robot building kit', '[0,0,0,4,0,3,0,2,0,1]'),
    ('Sporty tank top', '[0,0,0,4,0,3,0,2,0,1]'),
    ('Bluetooth-enabled speaker', '[0,0,0,3,0,2,0,4,0,1]'),
    ('Winter woolen socks', '[0,0,0,5,0,4,0,3,0,2]'),
    ('Rustic bookshelf', '[0,0,0,4,0,3,0,2,0,1]'),
    ('Moisturizing lip balm', '[0,0,0,4,0,3,0,2,0,1]'),
    ('Lightweight camera bag', '[0,0,0,5,0,4,0,3,0,2]'),
    ('Historical fiction book', '[0,0,0,3,0,2,0,4,0,1]'),
    ('Pure honey jar', '[0,0,0,4,0,3,0,2,0,1]'),
    ('Handcrafted wooden frame', '[0,0,0,5,0,4,0,3,0,2]'),
    ('Plush teddy bear', '[0,0,0,4,0,3,0,2,0,1]'),
    ('Warm woolen sweater', '[0,0,0,3,0,2,0,4,0,1]');

