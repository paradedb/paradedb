-- Query with CTE using BM25 search
WITH matching_companies AS (
  SELECT id, name, paradedb.score(company, 'name') as score
  FROM company
  WHERE name @@@ 'Test'
)
SELECT p.id, p.name, p.company_id, c.id as c_id, c.name as c_name, c.score
FROM people p
JOIN matching_companies c ON p.company_id = c.id
ORDER BY c.score DESC;
