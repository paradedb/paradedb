-- Search comments for specific terms
SELECT id, post_id, score, text
FROM comments
WHERE comments @@@ paradedb.parse('text:error OR text:bug')
ORDER BY score DESC
LIMIT 100;
