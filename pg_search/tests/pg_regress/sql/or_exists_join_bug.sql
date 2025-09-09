-- Regression test for rt_fetch out-of-bounds error with OR EXISTS and multiple JOINs
-- This test verifies the fix for the issue where complex nested queries with OR EXISTS
-- and multiple JOINs cause an rt_fetch out-of-bounds error

CREATE EXTENSION IF NOT EXISTS pg_search;

-- Setup
DROP TABLE IF EXISTS details CASCADE;
DROP TABLE IF EXISTS item_details CASCADE;
DROP TABLE IF EXISTS task_items CASCADE;
DROP TABLE IF EXISTS tasks CASCADE;
DROP TABLE IF EXISTS users CASCADE;

CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    org_id INT NOT NULL,
    name TEXT
);

CREATE TABLE tasks (
    id SERIAL PRIMARY KEY,
    user_id INT REFERENCES users(id),
    status TEXT,
    priority INT
);

CREATE TABLE task_items (
    id SERIAL PRIMARY KEY,
    task_id INT REFERENCES tasks(id),
    item_type TEXT,
    created_at TIMESTAMP
);

CREATE TABLE item_details (
    id SERIAL PRIMARY KEY,
    task_item_id INT REFERENCES task_items(id),
    detail_id INT
);

CREATE TABLE details (
    id SERIAL PRIMARY KEY,
    content TEXT,
    metadata JSONB
);

-- Insert test data
INSERT INTO users (org_id, name) VALUES 
(1, 'Alice'), 
(1, 'Bob'), 
(2, 'Charlie');

INSERT INTO tasks (user_id, status, priority) VALUES 
(1, 'completed', 1), 
(2, 'pending', 2),
(3, 'completed', 3);

INSERT INTO task_items (task_id, item_type, created_at) VALUES 
(1, 'typeA', '2024-01-01'),
(2, 'typeB', '2024-01-02'),
(3, 'typeA', '2024-01-03');

INSERT INTO item_details (task_item_id, detail_id) VALUES 
(1, 1),
(2, 2),
(3, 3);

INSERT INTO details (content, metadata) VALUES 
('test content 1', '{"processed": true}'),
('test content 2', '{"processed": false}'),
('test content 3', NULL);

-- Create BM25 indexes
CREATE INDEX ON users USING bm25 (id, org_id, name) WITH (key_field = 'id');
CREATE INDEX ON tasks USING bm25 (id, user_id, status, priority) WITH (key_field = 'id');
CREATE INDEX ON task_items USING bm25 (id, task_id, item_type) WITH (key_field = 'id');
CREATE INDEX ON item_details USING bm25 (id, task_item_id, detail_id) WITH (key_field = 'id');
CREATE INDEX ON details USING bm25 (id, content, metadata) WITH (key_field = 'id', json_fields = '{"metadata": {"fast": true}}');

-- Test 1: Simple query without EXISTS - should work
SELECT u.id, u.name
FROM users u
WHERE u.id @@@ paradedb.term('org_id', 1)
ORDER BY u.id;

-- Test 2: Query with simple EXISTS - should work
SELECT u.id, u.name
FROM users u
WHERE u.id @@@ paradedb.term('org_id', 1)
AND EXISTS(
    SELECT t.id
    FROM tasks t
    WHERE u.id = t.user_id
    AND t.id @@@ paradedb.term('status', 'completed')
)
ORDER BY u.id;

-- Test 3: Query with AND EXISTS and multiple JOINs - should work
SELECT u.id, u.name
FROM users u
WHERE u.id @@@ paradedb.term('org_id', 1)
AND EXISTS(
    SELECT t.id
    FROM tasks t
    WHERE u.id = t.user_id
    AND t.id @@@ paradedb.term('status', 'completed')
    AND EXISTS (
        SELECT ti.id
        FROM task_items ti
        JOIN item_details id ON ti.id = id.task_item_id
        JOIN details d ON id.detail_id = d.id
        WHERE t.id = ti.task_id
        AND ti.id @@@ paradedb.term('item_type', 'typeA')
        AND d.id @@@ paradedb.exists('metadata.processed')
    )
)
ORDER BY u.id;

-- Test 4: The problematic query - OR EXISTS with multiple JOINs
-- This previously caused rt_fetch out-of-bounds error
SELECT u.id, u.name
FROM users u
WHERE u.id @@@ paradedb.term('org_id', 1)
AND EXISTS(
    SELECT t.id
    FROM tasks t
    WHERE u.id = t.user_id
    AND (
        t.id @@@ paradedb.term('status', 'completed')
        OR EXISTS (
            SELECT ti.id
            FROM task_items ti
            JOIN item_details id ON ti.id = id.task_item_id
            JOIN details d ON id.detail_id = d.id
            WHERE t.id = ti.task_id
            AND ti.id @@@ paradedb.term('item_type', 'typeA')
            AND d.id @@@ paradedb.exists('metadata.processed')
        )
    )
)
ORDER BY u.id;

-- Test 5: Workaround - Using native PostgreSQL clause
SELECT u.id, u.name
FROM users u
WHERE u.id @@@ paradedb.term('org_id', 1)
AND EXISTS(
    SELECT t.id
    FROM tasks t
    WHERE u.id = t.user_id
    AND (
        t.id @@@ paradedb.term('status', 'completed')
        OR EXISTS (
            SELECT ti.id
            FROM task_items ti
            JOIN item_details id ON ti.id = id.task_item_id
            JOIN details d ON id.detail_id = d.id
            WHERE t.id = ti.task_id
            AND ti.id @@@ paradedb.term('item_type', 'typeA')
            AND d.metadata->>'processed' = 'true'  -- Native PostgreSQL instead of ParadeDB
        )
    )
)
ORDER BY u.id;

-- Test 6: Another variation with different join order
SELECT u.id, u.name
FROM users u
WHERE u.id @@@ paradedb.term('org_id', 2)
AND EXISTS(
    SELECT t.id
    FROM tasks t
    WHERE u.id = t.user_id
    AND (
        t.id @@@ paradedb.term('priority', 3)
        OR EXISTS (
            SELECT d.id
            FROM details d
            JOIN item_details id ON d.id = id.detail_id
            JOIN task_items ti ON id.task_item_id = ti.id
            WHERE t.id = ti.task_id
            AND d.id @@@ paradedb.term('content', 'test')
        )
    )
)
ORDER BY u.id;

-- Test 7: Minimal reproduction case - simplified version
SELECT 1 as result
WHERE EXISTS(
    SELECT 1
    WHERE (
        FALSE  -- Force evaluation of OR branch
        OR EXISTS (
            SELECT 1
            FROM task_items ti
            JOIN item_details id ON ti.id = id.task_item_id
            JOIN details d ON id.detail_id = d.id
            WHERE d.id @@@ paradedb.exists('metadata.processed')
        )
    )
);

-- Test 8: Edge case - deeply nested OR EXISTS
SELECT u.id, u.name
FROM users u
WHERE u.id @@@ paradedb.term('org_id', 1)
AND EXISTS(
    SELECT t.id
    FROM tasks t
    WHERE u.id = t.user_id
    AND (
        t.id @@@ paradedb.term('status', 'completed')
        OR EXISTS (
            SELECT ti.id
            FROM task_items ti
            WHERE t.id = ti.task_id
            AND (
                ti.id @@@ paradedb.term('item_type', 'typeA')
                OR EXISTS (
                    SELECT d.id
                    FROM details d
                    JOIN item_details id ON d.id = id.detail_id
                    WHERE ti.id = id.task_item_id
                    AND d.id @@@ paradedb.exists('metadata.processed')
                )
            )
        )
    )
)
ORDER BY u.id;

-- Cleanup
DROP TABLE details CASCADE;
DROP TABLE item_details CASCADE;
DROP TABLE task_items CASCADE;
DROP TABLE tasks CASCADE;
DROP TABLE users CASCADE;
