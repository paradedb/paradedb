-- Create extensions
DROP EXTENSION IF EXISTS pg_search CASCADE;
CREATE EXTENSION IF NOT EXISTS pg_search;

-- Create tables
CREATE TABLE IF NOT EXISTS "user" (
    id BIGINT PRIMARY KEY,
    company_id BIGINT,
    status TEXT
);

CREATE TABLE IF NOT EXISTS user_roles (
    user_id BIGINT,
    role_id BIGINT
);

CREATE TABLE IF NOT EXISTS user_zipcode (
    user_id BIGINT,
    zipcode TEXT
);

CREATE TABLE IF NOT EXISTS company (
    id BIGINT PRIMARY KEY,
    name TEXT,
    name_raw TEXT
);

CREATE TABLE IF NOT EXISTS products (
    id BIGINT PRIMARY KEY,
    name TEXT,
    name_raw TEXT
);

CREATE TABLE IF NOT EXISTS user_products (
    user_id BIGINT,
    product_id BIGINT,
    deleted_at TIMESTAMP
);

-- Create indexes
CREATE INDEX idx_user_roles_user_id ON user_roles(user_id);
CREATE INDEX idx_user_zipcode_user_id ON user_zipcode(user_id);
CREATE INDEX idx_user_products_user_id ON user_products(user_id);

-- Create ParadeDB BM25 indexes using the newer CREATE INDEX syntax
DROP INDEX IF EXISTS company_name_search_idx;
CREATE INDEX company_name_search_idx ON company
USING bm25 (id, name, name_raw)
WITH (key_field = 'id');

DROP INDEX IF EXISTS product_name_search_idx;
CREATE INDEX product_name_search_idx ON products
USING bm25 (id, name, name_raw)
WITH (key_field = 'id');

-- Insert test data
-- Users with different company IDs
INSERT INTO "user" VALUES 
(1, 4, 'NORMAL'),
(2, 5, 'NORMAL'),
(3, 13, 'NORMAL'),
(4, 15, 'NORMAL'),
(5, 7, 'NORMAL'),
(6, 4, 'INACTIVE'),
(7, 5, 'NORMAL');

-- All users have role 5
INSERT INTO user_roles VALUES
(1, 5),
(2, 5),
(3, 5),
(4, 5),
(5, 5),
(6, 5),
(7, 5);

-- All users have zipcode 90266
INSERT INTO user_zipcode VALUES
(1, '90266'),
(2, '90266'),
(3, '90266'),
(4, '90266'),
(5, '90266'),
(6, '90266'),
(7, '90266');

-- Companies - only some match the search query
INSERT INTO company VALUES
(4, 'Testing Company', 'Testing Company'),
(5, 'Testing Org', 'Testing Org'),
(7, 'Not a match', 'Not a match'),
(13, 'Something else', 'Something else'),
(15, 'Important Testing', 'Important Testing');

-- Products - one matches the search query
INSERT INTO products VALUES
(100, 'Testing Product', 'Testing Product'),
(200, 'Not a match product', 'Not a match product');

-- User products associations
INSERT INTO user_products VALUES
(1, 100, NULL),
(2, 100, NULL),
(3, 200, NULL),
(4, 100, NULL),
(5, 200, NULL);

-- Test queries that demonstrate the issue

-- Query 1: Problem query - including company_id 15 causes issue
WITH target_users AS (
    SELECT u.id, u.company_id
    FROM public.user u
    WHERE u.status = 'NORMAL'
        AND u.company_id in (5, 4, 13, 15)
        AND EXISTS (
            SELECT 1 FROM public.user_roles ur
            WHERE ur.user_id = u.id AND ur.role_id = 5
        )
        AND EXISTS (
            SELECT 1 FROM user_zipcode uz
            WHERE uz.user_id = u.id AND uz.zipcode IN ('90266')
        )
),
matched_companies AS (
    SELECT c.id, paradedb.score(c.id) AS company_score
    FROM company c
    WHERE c.id @@@ 'name:Testing OR name_raw:Testing'
),
matched_products AS (
    SELECT p.id, paradedb.score(p.id) AS product_score
    FROM products p
    WHERE p.id @@@ 'name:Testing OR name_raw:Testing'
),
scored_users AS (
    SELECT
        u.id,
        COALESCE(MAX(mc.company_score), 0) + COALESCE(MAX(mp.product_score), 0) AS score,
        COALESCE(MAX(mc.company_score), 0) AS company_score,
        COALESCE(MAX(mp.product_score), 0) AS product_score
    FROM target_users u
    LEFT JOIN matched_companies mc ON u.company_id = mc.id
    LEFT JOIN user_products up ON up.user_id = u.id AND up.deleted_at IS NULL
    LEFT JOIN matched_products mp ON mp.id = up.product_id
    GROUP BY u.id
)
SELECT su.id, su.score, su.company_score, su.product_score
FROM scored_users su
ORDER BY score DESC
LIMIT 100;

-- Query 2: Works correctly when removing company_id 15
WITH target_users AS (
    SELECT u.id, u.company_id
    FROM public.user u
    WHERE u.status = 'NORMAL'
        AND u.company_id in (5, 4, 13)
        AND EXISTS (
            SELECT 1 FROM public.user_roles ur
            WHERE ur.user_id = u.id AND ur.role_id = 5
        )
        AND EXISTS (
            SELECT 1 FROM user_zipcode uz
            WHERE uz.user_id = u.id AND uz.zipcode IN ('90266')
        )
),
matched_companies AS (
    SELECT c.id, paradedb.score(c.id) AS company_score
    FROM company c
    WHERE c.id @@@ 'name:Testing OR name_raw:Testing'
),
matched_products AS (
    SELECT p.id, paradedb.score(p.id) AS product_score
    FROM products p
    WHERE p.id @@@ 'name:Testing OR name_raw:Testing'
),
scored_users AS (
    SELECT
        u.id,
        COALESCE(MAX(mc.company_score), 0) + COALESCE(MAX(mp.product_score), 0) AS score,
        COALESCE(MAX(mc.company_score), 0) AS company_score,
        COALESCE(MAX(mp.product_score), 0) AS product_score
    FROM target_users u
    LEFT JOIN matched_companies mc ON u.company_id = mc.id
    LEFT JOIN user_products up ON up.user_id = u.id AND up.deleted_at IS NULL
    LEFT JOIN matched_products mp ON mp.id = up.product_id
    GROUP BY u.id
)
SELECT su.id, su.score, su.company_score, su.product_score
FROM scored_users su
ORDER BY score DESC
LIMIT 100;

-- Workaround 1: Using MATERIALIZED hint for the CTE
WITH target_users AS MATERIALIZED (
    SELECT u.id, u.company_id
    FROM public.user u
    WHERE u.status = 'NORMAL'
        AND u.company_id in (5, 4, 13, 15)
        AND EXISTS (
            SELECT 1 FROM public.user_roles ur
            WHERE ur.user_id = u.id AND ur.role_id = 5
        )
        AND EXISTS (
            SELECT 1 FROM user_zipcode uz
            WHERE uz.user_id = u.id AND uz.zipcode IN ('90266')
        )
),
matched_companies AS (
    SELECT c.id, paradedb.score(c.id) AS company_score
    FROM company c
    WHERE c.id @@@ 'name:Testing OR name_raw:Testing'
),
matched_products AS (
    SELECT p.id, paradedb.score(p.id) AS product_score
    FROM products p
    WHERE p.id @@@ 'name:Testing OR name_raw:Testing'
),
scored_users AS (
    SELECT
        u.id,
        COALESCE(MAX(mc.company_score), 0) + COALESCE(MAX(mp.product_score), 0) AS score,
        COALESCE(MAX(mc.company_score), 0) AS company_score,
        COALESCE(MAX(mp.product_score), 0) AS product_score
    FROM target_users u
    LEFT JOIN matched_companies mc ON u.company_id = mc.id
    LEFT JOIN user_products up ON up.user_id = u.id AND up.deleted_at IS NULL
    LEFT JOIN matched_products mp ON mp.id = up.product_id
    GROUP BY u.id
)
SELECT su.id, su.score, su.company_score, su.product_score
FROM scored_users su
ORDER BY score DESC
LIMIT 100;

-- Workaround 2: Moving the company_id filter to a JOIN
WITH target_users AS (
    SELECT u.id, u.company_id
    FROM public.user u
    WHERE u.status = 'NORMAL'
        AND EXISTS (
            SELECT 1 FROM public.user_roles ur
            WHERE ur.user_id = u.id AND ur.role_id = 5
        )
        AND EXISTS (
            SELECT 1 FROM user_zipcode uz
            WHERE uz.user_id = u.id AND uz.zipcode IN ('90266')
        )
),
matched_companies AS (
    SELECT c.id, paradedb.score(c.id) AS company_score
    FROM company c
    WHERE c.id @@@ 'name:Testing OR name_raw:Testing'
),
matched_products AS (
    SELECT p.id, paradedb.score(p.id) AS product_score
    FROM products p
    WHERE p.id @@@ 'name:Testing OR name_raw:Testing'
),
scored_users AS (
    SELECT
        u.id,
        u.company_id,
        COALESCE(MAX(mc.company_score), 0) + COALESCE(MAX(mp.product_score), 0) AS score,
        COALESCE(MAX(mc.company_score), 0) AS company_score,
        COALESCE(MAX(mp.product_score), 0) AS product_score
    FROM target_users u
    LEFT JOIN matched_companies mc ON u.company_id = mc.id
    LEFT JOIN user_products up ON up.user_id = u.id AND up.deleted_at IS NULL
    LEFT JOIN matched_products mp ON mp.id = up.product_id
    GROUP BY u.id, u.company_id
)
SELECT su.id, su.score, su.company_score, su.product_score
FROM scored_users su
JOIN company c ON c.id = su.company_id AND c.id IN (5, 4, 13, 15)
ORDER BY score DESC
LIMIT 100;

-- Diagnostic query: shows which rows have NULL mc_company_id
WITH target_users AS (
    SELECT u.id, u.company_id
    FROM public.user u
    WHERE u.status = 'NORMAL'
        AND u.company_id in (5, 4, 13, 15)
        AND EXISTS (
            SELECT 1 FROM public.user_roles ur
            WHERE ur.user_id = u.id AND ur.role_id = 5
        )
        AND EXISTS (
            SELECT 1 FROM user_zipcode uz
            WHERE uz.user_id = u.id AND uz.zipcode IN ('90266')
        )
),
matched_companies AS (
    SELECT c.id, paradedb.score(c.id) AS company_score
    FROM company c
    WHERE c.id @@@ 'name:Testing OR name_raw:Testing'
),
matched_products AS (
    SELECT p.id, paradedb.score(p.id) AS product_score
    FROM products p
    WHERE p.id @@@ 'name:Testing OR name_raw:Testing'
),
scored_users AS (
    SELECT
        u.id,
        u.company_id,
        mc.id as mc_company_id,
        COALESCE(MAX(mc.company_score), 0) + COALESCE(MAX(mp.product_score), 0) AS score,
        COALESCE(MAX(mc.company_score), 0) AS company_score,
        COALESCE(MAX(mp.product_score), 0) AS product_score
    FROM target_users u
    LEFT JOIN matched_companies mc ON u.company_id = mc.id
    LEFT JOIN user_products up ON up.user_id = u.id AND up.deleted_at IS NULL
    LEFT JOIN matched_products mp ON mp.id = up.product_id
    GROUP BY u.id, u.company_id, mc.id
)
SELECT su.id, su.company_id, su.mc_company_id, su.score, su.company_score, su.product_score
FROM scored_users su
ORDER BY score DESC
LIMIT 100; 
