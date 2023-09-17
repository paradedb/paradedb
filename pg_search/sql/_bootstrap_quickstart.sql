ALTER TABLE mock_items DROP COLUMN IF EXISTS embedding;
ALTER TABLE paradedb.mock_items ADD COLUMN embedding vector(3);

WITH NumberedRows AS (
    SELECT ctid,
           ROW_NUMBER() OVER () as row_num
    FROM paradedb.mock_items
)
UPDATE paradedb.mock_items m
SET embedding = ('[' || 
    ((n.row_num + 1) % 10 + 1)::integer || ',' || 
    ((n.row_num + 2) % 10 + 1)::integer || ',' || 
    ((n.row_num + 3) % 10 + 1)::integer || ']')::vector
FROM NumberedRows n
WHERE m.ctid = n.ctid;
