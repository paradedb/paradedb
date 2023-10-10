-- Default highlighting without max_num_chars
SELECT description, rating, category, paradedb.rank_bm25(ctid), paradedb.highlight_bm25(ctid, 'description')
FROM paradedb.mock_items
WHERE mock_items @@@ 'description:keyboard OR category:electronics'
LIMIT 5;

-- max_num_chars is set to 14 
SELECT description, rating, category, paradedb.rank_bm25(ctid), paradedb.highlight_bm25(ctid, 'description')
FROM paradedb.mock_items
WHERE mock_items @@@ 'description:keyboard OR category:electronics:::max_num_chars=14'
LIMIT 5;
