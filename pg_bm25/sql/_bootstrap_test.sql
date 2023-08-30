CREATE TABLE products (
    id SERIAL PRIMARY KEY,
    description TEXT,
    rating INTEGER CHECK (
        rating BETWEEN 1
        AND 5
    ),
    category VARCHAR(255)
);

INSERT INTO
    products (description, rating, category)
VALUES
    ('Ergonomic metal keyboard', 4, 'Electronics'),
    ('Plastic Keyboard', 4, 'Electronics'),
    ('Sleek running shoes', 5, 'Footwear'),
    ('Durable wooden chair', 3, 'Furniture'),
    ('Natural skin lotion', 4, 'Beauty'),
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
