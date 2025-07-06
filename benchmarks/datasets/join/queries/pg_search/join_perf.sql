-- JOIN Performance Benchmark

-- PARADEDB QUERIES (using @@@ operators)
-- ===========================================

-- 1. Basic COUNT query similar to customer's denormalized table pattern
-- This reproduces their ~1s count query on contacts_companies_combined_full
SELECT count(*)
FROM pages
WHERE pages.content @@@ paradedb.parse('parents:"SFR"');

-- 2. Basic JOIN COUNT - reproduces their original contact/company join pattern
SELECT count(*)
FROM documents 
JOIN files ON documents.id = files."documentId" 
JOIN pages ON files.id = pages."fileId"
WHERE documents.parents @@@ 'SFR' 
  AND files.title @@@ 'collab12'
  AND pages.content @@@ 'Single Number Reach';

-- 3. CTE approach that customer tried but found still slow (~27s)
WITH filtered_documents AS (
    SELECT DISTINCT id
    FROM documents 
    WHERE parents @@@ 'SFR'
),
filtered_files AS (
    SELECT DISTINCT id, "documentId"
    FROM files 
    WHERE title @@@ 'collab12'
),
filtered_pages AS (
    SELECT DISTINCT "fileId"
    FROM pages 
    WHERE content @@@ 'Single Number Reach'
)
SELECT count(*)
FROM filtered_documents fd
JOIN filtered_files ff ON fd.id = ff."documentId"
JOIN filtered_pages fp ON ff.id = fp."fileId";

-- 4. EXISTS subquery pattern similar to customer's intent/company queries
SELECT count(*)
FROM documents
WHERE documents.parents @@@ 'SFR'
  AND EXISTS (
    SELECT 1 FROM files 
    WHERE files."documentId" = documents.id 
      AND files.title @@@ 'collab12'
  );

-- 5. Complex nested field query similar to customer's company_locations_details pattern
-- Using pages.parents as analogous to their nested field structure
SELECT count(*)
FROM pages
WHERE pages.content @@@ paradedb.parse('parents:"Page Parent Reference"');

-- 6. GROUP BY aggregation pattern (terms/bucket aggregation)
-- Similar to their company.industry, count(*) GROUP BY pattern
WITH filtered_documents AS (
    SELECT DISTINCT id
    FROM documents 
    WHERE parents @@@ 'SFR'
)
SELECT LEFT(d.title, 20) as title_prefix, count(*)
FROM documents d
WHERE d.id IN (SELECT id FROM filtered_documents)
  AND EXISTS (
    SELECT 1 FROM files f 
    WHERE f."documentId" = d.id 
      AND f.title @@@ 'collab12'
  )
GROUP BY LEFT(d.title, 20);

-- 7. Large result set aggregation - tests StringFastFieldExecState vs heap fetches
SELECT LEFT(pages.title, 30) as title_prefix, count(*) as cnt
FROM pages
WHERE pages.content @@@ paradedb.exists('title')
GROUP BY LEFT(pages.title, 30)
ORDER BY cnt DESC
LIMIT 20;

-- 8. Complex boolean query with multiple conditions
-- Similar to customer's complex filtering needs
SELECT count(*)
FROM pages p
JOIN files f ON p."fileId" = f.id
WHERE p.content @@@ paradedb.boolean(
    should => ARRAY[
        paradedb.parse('content:"Single Number Reach"'),
        paradedb.parse('content:"Page Content"')
    ]
)
AND f.title @@@ paradedb.boolean(
    must => ARRAY[
        paradedb.parse('title:"collab12"')
    ]
);

-- 9. Test query that should demonstrate EXPLAIN ANALYZE vs actual execution discrepancy
-- This query selects actual data (not just count) which might show the timing difference
SELECT p.id, p.title, f.title as file_title, d.title as doc_title
FROM pages p
JOIN files f ON p."fileId" = f.id  
JOIN documents d ON f."documentId" = d.id
WHERE p.content @@@ paradedb.exists('title')
  AND f.title @@@ paradedb.exists('title')
  AND d.parents @@@ 'SFR'
ORDER BY p."createdAt" DESC
LIMIT 1000;

-- 10. Large scan with aggregation - reproduces customer's 60s vs 10s timing issue
SELECT 
    LEFT(p.title, 25) as title_prefix,
    COUNT(*) as total_pages,
    COUNT(DISTINCT p."fileId") as unique_files,
    AVG(p."sizeInBytes") as avg_size
FROM pages p
WHERE p.content @@@ paradedb.exists('content')
GROUP BY LEFT(p.title, 25)
HAVING COUNT(*) > 10
ORDER BY total_pages DESC; 

-- POSTGRESQL EQUIVALENT QUERIES (using LIKE and standard operators)
-- =================================================================

-- 1. Basic COUNT query - PostgreSQL equivalent
SELECT count(*)
FROM pages
WHERE pages.content LIKE '%SFR%';

-- 2. Basic JOIN COUNT - PostgreSQL equivalent
SELECT count(*)
FROM documents 
JOIN files ON documents.id = files."documentId" 
JOIN pages ON files.id = pages."fileId"
WHERE documents.parents LIKE '%SFR%' 
  AND files.title LIKE '%collab12%'
  AND pages.content LIKE '%Single Number Reach%';

-- 3. CTE approach - PostgreSQL equivalent
WITH filtered_documents AS (
    SELECT DISTINCT id
    FROM documents 
    WHERE parents LIKE '%SFR%'
),
filtered_files AS (
    SELECT DISTINCT id, "documentId"
    FROM files 
    WHERE title LIKE '%collab12%'
),
filtered_pages AS (
    SELECT DISTINCT "fileId"
    FROM pages 
    WHERE content LIKE '%Single Number Reach%'
)
SELECT count(*)
FROM filtered_documents fd
JOIN filtered_files ff ON fd.id = ff."documentId"
JOIN filtered_pages fp ON ff.id = fp."fileId";

-- 4. EXISTS subquery - PostgreSQL equivalent
SELECT count(*)
FROM documents
WHERE documents.parents LIKE '%SFR%'
  AND EXISTS (
    SELECT 1 FROM files 
    WHERE files."documentId" = documents.id 
      AND files.title LIKE '%collab12%'
  );

-- 5. Complex nested field query - PostgreSQL equivalent
SELECT count(*)
FROM pages
WHERE pages.content LIKE '%Page Parent Reference%';

-- 6. GROUP BY aggregation - PostgreSQL equivalent
WITH filtered_documents AS (
    SELECT DISTINCT id
    FROM documents 
    WHERE parents LIKE '%SFR%'
)
SELECT LEFT(d.title, 20) as title_prefix, count(*)
FROM documents d
WHERE d.id IN (SELECT id FROM filtered_documents)
  AND EXISTS (
    SELECT 1 FROM files f 
    WHERE f."documentId" = d.id 
      AND f.title LIKE '%collab12%'
  )
GROUP BY LEFT(d.title, 20);

-- 7. Large result set aggregation - PostgreSQL equivalent
SELECT LEFT(pages.title, 30) as title_prefix, count(*) as cnt
FROM pages
WHERE pages.title IS NOT NULL AND pages.title != ''
GROUP BY LEFT(pages.title, 30)
ORDER BY cnt DESC
LIMIT 20;

-- 8. Complex boolean query - PostgreSQL equivalent (using OR for should, AND for must)
SELECT count(*)
FROM pages p
JOIN files f ON p."fileId" = f.id
WHERE (p.content LIKE '%Single Number Reach%' OR p.content LIKE '%Page Content%')
  AND f.title LIKE '%collab12%';

-- 9. Test query for timing discrepancy - PostgreSQL equivalent
SELECT p.id, p.title, f.title as file_title, d.title as doc_title
FROM pages p
JOIN files f ON p."fileId" = f.id  
JOIN documents d ON f."documentId" = d.id
WHERE p.title IS NOT NULL AND p.title != ''
  AND f.title IS NOT NULL AND f.title != ''
  AND d.parents LIKE '%SFR%'
ORDER BY p."createdAt" DESC
LIMIT 1000;

-- 10. Large scan with aggregation - PostgreSQL equivalent
SELECT 
    LEFT(p.title, 25) as title_prefix,
    COUNT(*) as total_pages,
    COUNT(DISTINCT p."fileId") as unique_files,
    AVG(p."sizeInBytes") as avg_size
FROM pages p
WHERE p.content IS NOT NULL AND p.content != ''
GROUP BY LEFT(p.title, 25)
HAVING COUNT(*) > 10
ORDER BY total_pages DESC; 
