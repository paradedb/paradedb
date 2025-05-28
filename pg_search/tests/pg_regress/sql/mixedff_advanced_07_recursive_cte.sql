-- Tests recursive CTE with mixed fast fields

\i common/mixedff_advanced_setup.sql

\echo 'Test: Recursive CTE'

-- Create test tables for hierarchical data
DROP TABLE IF EXISTS category;
CREATE TABLE category (
    id SERIAL PRIMARY KEY,
    name TEXT,
    parent_id INTEGER,
    level INTEGER,
    description TEXT,
    item_count INTEGER,
    created_at TIMESTAMP,
    is_active BOOLEAN
);

-- Insert root categories (no parent)
INSERT INTO category (name, parent_id, level, description, item_count, created_at, is_active)
VALUES
    ('Electronics', NULL, 1, 'Electronic devices and accessories', 250, '2023-01-01 10:00:00', true),
    ('Books', NULL, 1, 'Books and literature', 500, '2023-01-01 10:00:00', true),
    ('Clothing', NULL, 1, 'Apparel and fashion items', 300, '2023-01-01 10:00:00', true),
    ('Home & Garden', NULL, 1, 'Home improvement and garden supplies', 180, '2023-01-01 10:00:00', true);

-- Insert level 2 subcategories
INSERT INTO category (name, parent_id, level, description, item_count, created_at, is_active)
VALUES
    ('Computers', 1, 2, 'Desktop and laptop computers', 80, '2023-01-02 10:00:00', true),
    ('Smartphones', 1, 2, 'Mobile phones and accessories', 120, '2023-01-02 10:00:00', true),
    ('Audio', 1, 2, 'Speakers, headphones, and audio equipment', 50, '2023-01-02 10:00:00', true),
    ('Fiction', 2, 2, 'Fiction books and novels', 200, '2023-01-02 10:00:00', true),
    ('Non-Fiction', 2, 2, 'Non-fiction and reference books', 250, '2023-01-02 10:00:00', true),
    ('Academic', 2, 2, 'Textbooks and academic materials', 50, '2023-01-02 10:00:00', true),
    ('Men', 3, 2, 'Mens clothing', 100, '2023-01-02 10:00:00', true),
    ('Women', 3, 2, 'Womens clothing', 150, '2023-01-02 10:00:00', true),
    ('Children', 3, 2, 'Childrens clothing', 50, '2023-01-02 10:00:00', true),
    ('Furniture', 4, 2, 'Home furniture', 80, '2023-01-02 10:00:00', true),
    ('Garden Tools', 4, 2, 'Garden equipment and supplies', 60, '2023-01-02 10:00:00', true),
    ('Kitchen', 4, 2, 'Kitchen appliances and utensils', 40, '2023-01-02 10:00:00', true);

-- Insert level 3 subcategories
INSERT INTO category (name, parent_id, level, description, item_count, created_at, is_active)
VALUES
    ('Laptops', 5, 3, 'Portable computers', 40, '2023-01-03 10:00:00', true),
    ('Desktops', 5, 3, 'Desktop computers', 30, '2023-01-03 10:00:00', true),
    ('Tablets', 5, 3, 'Tablet computers', 10, '2023-01-03 10:00:00', true),
    ('Android', 6, 3, 'Android smartphones', 60, '2023-01-03 10:00:00', true),
    ('iOS', 6, 3, 'iPhones and iOS devices', 50, '2023-01-03 10:00:00', true),
    ('Other', 6, 3, 'Other smartphone platforms', 10, '2023-01-03 10:00:00', true),
    ('Headphones', 7, 3, 'Personal audio devices', 30, '2023-01-03 10:00:00', true),
    ('Speakers', 7, 3, 'Speaker systems', 15, '2023-01-03 10:00:00', true),
    ('Receivers', 7, 3, 'Audio receivers and amplifiers', 5, '2023-01-03 10:00:00', true);

-- Create search index with mixed fast fields
DROP INDEX IF EXISTS category_idx;
CREATE INDEX category_idx ON category
USING bm25 (id, name, parent_id, description, level, item_count, is_active)
WITH (
    key_field = 'id',
    text_fields = '{"name": {"tokenizer": {"type": "default"}, "fast": true}, "description": {"tokenizer": {"type": "default"}, "fast": true}}',
    numeric_fields = '{"parent_id": {"fast": true}, "level": {"fast": true}, "item_count": {"fast": true}}',
    boolean_fields = '{"is_active": {"fast": true}}'
);

-- Test 1: Basic recursive CTE to find all descendants of Electronics
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
WITH RECURSIVE category_tree AS (
    -- Base case: start with parent category
    SELECT id, name, parent_id, level, item_count, is_active
    FROM category
    WHERE name = 'Electronics'
    
    UNION ALL
    
    -- Recursive case: find children of current nodes
    SELECT c.id, c.name, c.parent_id, c.level, c.item_count, c.is_active
    FROM category c
    JOIN category_tree ct ON c.parent_id = ct.id
)
SELECT name, level, item_count
FROM category_tree
ORDER BY level, name;

WITH RECURSIVE category_tree AS (
    -- Base case: start with parent category
    SELECT id, name, parent_id, level, item_count, is_active
    FROM category
    WHERE name = 'Electronics'
    
    UNION ALL
    
    -- Recursive case: find children of current nodes
    SELECT c.id, c.name, c.parent_id, c.level, c.item_count, c.is_active
    FROM category c
    JOIN category_tree ct ON c.parent_id = ct.id
)
SELECT name, level, item_count
FROM category_tree
ORDER BY level, name;

-- Test 2: Recursive CTE with mixed field filtering
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
WITH RECURSIVE category_tree AS (
    -- Base case
    SELECT id, name, parent_id, level, item_count, is_active
    FROM category
    WHERE level = 1 AND item_count > 200
    
    UNION ALL
    
    -- Recursive case
    SELECT c.id, c.name, c.parent_id, c.level, c.item_count, c.is_active
    FROM category c
    JOIN category_tree ct ON c.parent_id = ct.id
    WHERE c.is_active = true
)
SELECT name, level, item_count
FROM category_tree
ORDER BY level, item_count DESC;

WITH RECURSIVE category_tree AS (
    -- Base case
    SELECT id, name, parent_id, level, item_count, is_active
    FROM category
    WHERE level = 1 AND item_count > 200
    
    UNION ALL
    
    -- Recursive case
    SELECT c.id, c.name, c.parent_id, c.level, c.item_count, c.is_active
    FROM category c
    JOIN category_tree ct ON c.parent_id = ct.id
    WHERE c.is_active = true
)
SELECT name, level, item_count
FROM category_tree
ORDER BY level, item_count DESC;

-- Test 3: Recursive CTE with search condition in base case
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
WITH RECURSIVE category_tree AS (
    -- Base case with search
    SELECT id, name, parent_id, level, description, item_count
    FROM category
    WHERE description @@@ 'books'
    
    UNION ALL
    
    -- Recursive case
    SELECT c.id, c.name, c.parent_id, c.level, c.description, c.item_count
    FROM category c
    JOIN category_tree ct ON c.parent_id = ct.id
)
SELECT name, level, description, item_count
FROM category_tree
ORDER BY level, name;

WITH RECURSIVE category_tree AS (
    -- Base case with search
    SELECT id, name, parent_id, level, description, item_count
    FROM category
    WHERE description @@@ 'books'
    
    UNION ALL
    
    -- Recursive case
    SELECT c.id, c.name, c.parent_id, c.level, c.description, c.item_count
    FROM category c
    JOIN category_tree ct ON c.parent_id = ct.id
)
SELECT name, level, description, item_count
FROM category_tree
ORDER BY level, name;

-- Test 4: Recursive CTE with search condition in recursive case
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
WITH RECURSIVE category_tree AS (
    -- Base case
    SELECT id, name, parent_id, level, description, item_count
    FROM category
    WHERE name = 'Electronics'
    
    UNION ALL
    
    -- Recursive case with search
    SELECT c.id, c.name, c.parent_id, c.level, c.description, c.item_count
    FROM category c
    JOIN category_tree ct ON c.parent_id = ct.id
    WHERE c.description @@@ 'computer' OR c.item_count > 30
)
SELECT name, level, description, item_count
FROM category_tree
ORDER BY level, name;

WITH RECURSIVE category_tree AS (
    -- Base case
    SELECT id, name, parent_id, level, description, item_count
    FROM category
    WHERE name = 'Electronics'
    
    UNION ALL
    
    -- Recursive case with search
    SELECT c.id, c.name, c.parent_id, c.level, c.description, c.item_count
    FROM category c
    JOIN category_tree ct ON c.parent_id = ct.id
    WHERE c.description @@@ 'computer' OR c.item_count > 30
)
SELECT name, level, description, item_count
FROM category_tree
ORDER BY level, name;

-- Test 5: Complex recursive CTE with aggregation and mixed fields
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
WITH RECURSIVE category_tree AS (
    -- Base case
    SELECT id, name, parent_id, level, item_count
    FROM category
    WHERE level = 1
    
    UNION ALL
    
    -- Recursive case
    SELECT c.id, c.name, c.parent_id, c.level, c.item_count
    FROM category c
    JOIN category_tree ct ON c.parent_id = ct.id
),
category_stats AS (
    SELECT 
        ct.name,
        ct.level,
        ct.item_count,
        CASE 
            WHEN ct.level = 1 THEN 'Main Category'
            WHEN ct.level = 2 THEN 'Subcategory'
            ELSE 'Sub-subcategory'
        END as category_type
    FROM category_tree ct
)
SELECT 
    category_type,
    COUNT(*) as category_count,
    SUM(item_count) as total_items,
    AVG(item_count) as avg_items
FROM category_stats
GROUP BY category_type
ORDER BY category_type;

WITH RECURSIVE category_tree AS (
    -- Base case
    SELECT id, name, parent_id, level, item_count
    FROM category
    WHERE level = 1
    
    UNION ALL
    
    -- Recursive case
    SELECT c.id, c.name, c.parent_id, c.level, c.item_count
    FROM category c
    JOIN category_tree ct ON c.parent_id = ct.id
),
category_stats AS (
    SELECT 
        ct.name,
        ct.level,
        ct.item_count,
        CASE 
            WHEN ct.level = 1 THEN 'Main Category'
            WHEN ct.level = 2 THEN 'Subcategory'
            ELSE 'Sub-subcategory'
        END as category_type
    FROM category_tree ct
)
SELECT 
    category_type,
    COUNT(*) as category_count,
    SUM(item_count) as total_items,
    AVG(item_count) as avg_items
FROM category_stats
GROUP BY category_type
ORDER BY category_type;

-- Verify actual recursive CTE results with mixed fields
WITH RECURSIVE category_tree AS (
    -- Base case
    SELECT id, name, parent_id, level, description, item_count, is_active
    FROM category
    WHERE name = 'Electronics'
    
    UNION ALL
    
    -- Recursive case
    SELECT c.id, c.name, c.parent_id, c.level, c.description, c.item_count, c.is_active
    FROM category c
    JOIN category_tree ct ON c.parent_id = ct.id
    WHERE c.is_active = true
)
SELECT name, level, description, item_count
FROM category_tree
ORDER BY level, name;

-- Clean up
DROP INDEX IF EXISTS category_idx;
DROP TABLE IF EXISTS category; 

\i common/mixedff_advanced_cleanup.sql
