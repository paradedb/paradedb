-- Tests recursive CTE with mixed fast fields

\i common/mixedff_advanced_setup.sql

\echo 'Test: Recursive CTE'

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

\i common/mixedff_advanced_cleanup.sql
