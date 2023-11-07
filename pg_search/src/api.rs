use pgrx::*;
use std::f64;

const CREATE_TEST_TABLE_SQL: &str = r#"
DO $$
BEGIN
    IF NOT EXISTS (SELECT FROM pg_catalog.pg_tables
                   WHERE schemaname = 'paradedb' AND tablename = 'search_test_table') THEN
        CREATE TABLE paradedb.search_test_table (
            id SERIAL PRIMARY KEY,
            description TEXT,
            rating INTEGER CHECK (
                rating BETWEEN 1
                AND 5
            ),
            category VARCHAR(255),
            in_stock BOOLEAN,
            metadata JSONB
        );

        INSERT INTO paradedb.search_test_table (description, rating, category, in_stock, metadata)
        VALUES
            ('Ergonomic metal keyboard', 4, 'Electronics', true, '{"color": "Silver", "location": "United States"}'::JSONB),
            ('Plastic Keyboard', 4, 'Electronics', false, '{"color": "Black", "location": "Canada"}'::JSONB),
            ('Sleek running shoes', 5, 'Footwear', true, '{"color": "Blue", "location": "China"}'::JSONB),
            ('White jogging shoes', 3, 'Footwear', false, '{"color": "White", "location": "United States"}'::JSONB),
            ('Generic shoes', 4, 'Footwear', true, '{"color": "Brown", "location": "Canada"}'::JSONB),
            ('Compact digital camera', 5, 'Photography', false, '{"color": "Black", "location": "China"}'::JSONB),
            ('Hardcover book on history', 2, 'Books', true, '{"color": "Brown", "location": "United States"}'::JSONB),
            ('Organic green tea', 3, 'Groceries', true, '{"color": "Green", "location": "Canada"}'::JSONB),
            ('Modern wall clock', 4, 'Home Decor', false, '{"color": "Silver", "location": "China"}'::JSONB),
            ('Colorful kids toy', 1, 'Toys', true, '{"color": "Multicolor", "location": "United States"}'::JSONB),
            ('Soft cotton shirt', 5, 'Apparel', true, '{"color": "Blue", "location": "Canada"}'::JSONB),
            ('Innovative wireless earbuds', 5, 'Electronics', true, '{"color": "Black", "location": "China"}'::JSONB),
            ('Sturdy hiking boots', 4, 'Footwear', true, '{"color": "Brown", "location": "United States"}'::JSONB),
            ('Elegant glass table', 3, 'Furniture', true, '{"color": "Clear", "location": "Canada"}'::JSONB),
            ('Refreshing face wash', 2, 'Beauty', false, '{"color": "White", "location": "China"}'::JSONB),
            ('High-resolution DSLR', 4, 'Photography', true, '{"color": "Black", "location": "United States"}'::JSONB),
            ('Paperback romantic novel', 3, 'Books', true, '{"color": "Multicolor", "location": "Canada"}'::JSONB),
            ('Freshly ground coffee beans', 5, 'Groceries', true, '{"color": "Brown", "location": "China"}'::JSONB),
            ('Artistic ceramic vase', 4, 'Home Decor', false, '{"color": "Multicolor", "location": "United States"}'::JSONB),
            ('Interactive board game', 3, 'Toys', true, '{"color": "Multicolor", "location": "Canada"}'::JSONB),
            ('Slim-fit denim jeans', 5, 'Apparel', false, '{"color": "Blue", "location": "China"}'::JSONB),
            ('Fast charging power bank', 4, 'Electronics', true, '{"color": "Black", "location": "United States"}'::JSONB),
            ('Comfortable slippers', 3, 'Footwear', true, '{"color": "Brown", "location": "Canada"}'::JSONB),
            ('Classic leather sofa', 5, 'Furniture', false, '{"color": "Brown", "location": "China"}'::JSONB),
            ('Anti-aging serum', 4, 'Beauty', true, '{"color": "White", "location": "United States"}'::JSONB),
            ('Portable tripod stand', 4, 'Photography', true, '{"color": "Black", "location": "Canada"}'::JSONB),
            ('Mystery detective novel', 2, 'Books', false, '{"color": "Multicolor", "location": "China"}'::JSONB),
            ('Organic breakfast cereal', 5, 'Groceries', true, '{"color": "Brown", "location": "United States"}'::JSONB),
            ('Designer wall paintings', 5, 'Home Decor', true, '{"color": "Multicolor", "location": "Canada"}'::JSONB),
            ('Robot building kit', 4, 'Toys', true, '{"color": "Multicolor", "location": "China"}'::JSONB),
            ('Sporty tank top', 4, 'Apparel', true, '{"color": "Blue", "location": "United States"}'::JSONB),
            ('Bluetooth-enabled speaker', 3, 'Electronics', true, '{"color": "Black", "location": "Canada"}'::JSONB),
            ('Winter woolen socks', 5, 'Footwear', false, '{"color": "Gray", "location": "China"}'::JSONB),
            ('Rustic bookshelf', 4, 'Furniture', true, '{"color": "Brown", "location": "United States"}'::JSONB),
            ('Moisturizing lip balm', 4, 'Beauty', true, '{"color": "Pink", "location": "Canada"}'::JSONB),
            ('Lightweight camera bag', 5, 'Photography', false, '{"color": "Black", "location": "China"}'::JSONB),
            ('Historical fiction book', 3, 'Books', true, '{"color": "Multicolor", "location": "United States"}'::JSONB),
            ('Pure honey jar', 4, 'Groceries', true, '{"color": "Yellow", "location": "Canada"}'::JSONB),
            ('Handcrafted wooden frame', 5, 'Home Decor', false, '{"color": "Brown", "location": "China"}'::JSONB),
            ('Plush teddy bear', 4, 'Toys', true, '{"color": "Brown", "location": "United States"}'::JSONB),
            ('Warm woolen sweater', 3, 'Apparel', false, '{"color": "Red", "location": "Canada"}'::JSONB);


        ALTER TABLE paradedb.search_test_table ADD COLUMN embedding vector(3);

        WITH NumberedRows AS (
            SELECT ctid,
                ROW_NUMBER() OVER () as row_num
            FROM paradedb.search_test_table
        )
        UPDATE paradedb.search_test_table m
        SET embedding = ('[' ||
            ((n.row_num + 1) % 10 + 1)::integer || ',' ||
            ((n.row_num + 2) % 10 + 1)::integer || ',' ||
            ((n.row_num + 3) % 10 + 1)::integer || ']')::vector
        FROM NumberedRows n
        WHERE m.ctid = n.ctid;
    ELSE
        RAISE WARNING 'The table paradedb.search_test_table already exists, skipping.';
    END IF;
END $$;
"#;

#[pg_extern]
pub fn minmax_norm(value: f64, min: f64, max: f64) -> f64 {
    if max == min {
        return 0.0;
    }
    (value - min) / (max - min)
}

#[pg_extern]
pub fn weighted_mean(a: f64, b: f64, weights: Vec<f64>) -> f64 {
    assert!(weights.len() == 2, "There must be exactly 2 weights");

    let weight_a = weights[0];
    let weight_b = weights[1];

    assert!(
        (0.0..=1.0).contains(&weight_a) && (0.0..=1.0).contains(&weight_b),
        "Weights must be between 0 and 1"
    );

    assert!(
        (weight_a + weight_b - 1.0).abs() < std::f64::EPSILON,
        "Weights must add up to 1"
    );

    a * weight_a + b * weight_b
}

#[pg_extern]
pub fn create_search_test_table() {
    Spi::run(CREATE_TEST_TABLE_SQL).expect("Could not create search_test_table");
}
