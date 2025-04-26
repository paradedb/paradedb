# PostgreSQL Query Planner Bug: Missing JOIN Matches Between Filtered Records and ParadeDB BM25 Search Results

## What happens?

When running a complex query that includes a filter condition with ParadeDB fulltext search, the query planner generates an execution plan that fails to properly join matching records. This results in missing data in the query results.

The issue appears to require specific conditions to trigger:
1. Multiple tables with relationships
2. A filter on company_id that includes company_id 15
3. ParadeDB fulltext search on companies 
4. A GROUP BY clause following the JOIN operations

Two workarounds are effective:
1. Using the `MATERIALIZED` hint on CTEs
2. Moving the company_id filter from the WHERE clause to a separate JOIN operation

## To Reproduce

### Setup

```sql
-- Create extension
DROP EXTENSION IF EXISTS pg_search CASCADE;
CREATE EXTENSION IF NOT EXISTS pg_search;

-- Create tables
CREATE TABLE IF NOT EXISTS company (
    id BIGINT PRIMARY KEY,
    name TEXT
);

CREATE TABLE IF NOT EXISTS "user" (
    id BIGINT PRIMARY KEY,
    company_id BIGINT,
    status TEXT
);

CREATE TABLE IF NOT EXISTS user_products (
    user_id BIGINT,
    product_id BIGINT,
    deleted_at TIMESTAMP
);

-- Create ParadeDB BM25 index
DROP INDEX IF EXISTS company_name_search_idx;
CREATE INDEX company_name_search_idx ON company
USING bm25 (id, name)
WITH (key_field = 'id');

-- Insert test data
DELETE FROM company;
INSERT INTO company VALUES
(4, 'Testing Company'),
(5, 'Testing Org'),
(13, 'Something else'),
(15, 'Important Testing');

DELETE FROM "user";
INSERT INTO "user" VALUES 
(1, 4, 'NORMAL'),
(2, 5, 'NORMAL'),
(3, 13, 'NORMAL'),
(4, 15, 'NORMAL'),
(5, 7, 'NORMAL');

DELETE FROM user_products;
INSERT INTO user_products VALUES
(1, 100, NULL),
(2, 100, NULL),
(3, 200, NULL),
(4, 100, NULL);
```

### Problem Query

```sql
-- This reproduces the issue with company_id 15
WITH target_users AS (
    SELECT u.id, u.company_id
    FROM "user" u
    WHERE u.status = 'NORMAL'
        AND u.company_id in (5, 4, 13, 15)
),
matched_companies AS (
    SELECT c.id, paradedb.score(c.id) AS company_score
    FROM company c
    WHERE c.id @@@ 'name:Testing'
),
scored_users AS (
    SELECT
        u.id,
        u.company_id,
        mc.id as mc_company_id,
        COALESCE(MAX(mc.company_score), 0) AS score
    FROM target_users u
    LEFT JOIN matched_companies mc ON u.company_id = mc.id
    LEFT JOIN user_products up ON up.user_id = u.id
    GROUP BY u.id, u.company_id, mc.id
)
SELECT su.id, su.company_id, su.mc_company_id, su.score
FROM scored_users su
ORDER BY score DESC;
```

### Expected Output

All users with companies containing "Testing" (company ids 4, 5, and 15) should have non-NULL mc_company_id and non-zero score values.

### Actual Output

```
 id | company_id | mc_company_id |   score   
----+------------+---------------+-----------
  1 |          4 |             4 | 0.5598161
  2 |          5 |             5 | 0.5598161
  3 |         13 |               |         0
  4 |         15 |               |         0  <- Missing match for company_id 15
```

### Working alternative 1: Use MATERIALIZED hint

```sql
WITH target_users AS MATERIALIZED (
    SELECT u.id, u.company_id
    FROM "user" u
    WHERE u.status = 'NORMAL'
        AND u.company_id in (5, 4, 13, 15)
),
matched_companies AS (
    SELECT c.id, paradedb.score(c.id) AS company_score
    FROM company c
    WHERE c.id @@@ 'name:Testing'
),
scored_users AS (
    SELECT
        u.id,
        u.company_id,
        mc.id as mc_company_id,
        COALESCE(MAX(mc.company_score), 0) AS score
    FROM target_users u
    LEFT JOIN matched_companies mc ON u.company_id = mc.id
    LEFT JOIN user_products up ON up.user_id = u.id
    GROUP BY u.id, u.company_id, mc.id
)
SELECT su.id, su.company_id, su.mc_company_id, su.score
FROM scored_users su
ORDER BY score DESC;
```

### Working alternative 2: Move filter to JOIN

```sql
WITH all_users AS (
    SELECT u.id, u.company_id
    FROM "user" u
    WHERE u.status = 'NORMAL'
),
matched_companies AS (
    SELECT c.id, paradedb.score(c.id) AS company_score
    FROM company c
    WHERE c.id @@@ 'name:Testing'
),
scored_users AS (
    SELECT
        u.id,
        u.company_id,
        mc.id as mc_company_id,
        COALESCE(MAX(mc.company_score), 0) AS score
    FROM all_users u
    JOIN (VALUES (4), (5), (13), (15)) AS filter(id) ON u.company_id = filter.id
    LEFT JOIN matched_companies mc ON u.company_id = mc.id
    LEFT JOIN user_products up ON up.user_id = u.id
    GROUP BY u.id, u.company_id, mc.id
)
SELECT su.id, su.company_id, su.mc_company_id, su.score
FROM scored_users su
ORDER BY score DESC;
```

### Conclusion

This issue appears to be a query planning bug in how PostgreSQL interacts with ParadeDB's custom scan operator. The bug requires multiple conditions to trigger:

1. A more complex query structure with multiple tables and relationships
2. A company_id filter that includes id 15
3. A ParadeDB fulltext search on the same table
4. JOINs followed by a GROUP BY operation

When these conditions are met, the query planner generates an execution plan that fails to properly join company_id 15 with its search matches, even though the text clearly contains the search term. The workarounds force a different execution plan that properly handles all matches.
 