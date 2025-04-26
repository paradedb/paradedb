-- Set log level to debug
SET client_min_messages TO DEBUG1;

-- Simple test query with a CTE
WITH company_search AS (
    SELECT * FROM company WHERE name @@@ 'test'
)
SELECT p.id, p.name, p.company_id, c.id as c_id, c.name as c_name
FROM people p
JOIN company_search c ON p.company_id = c.id;
