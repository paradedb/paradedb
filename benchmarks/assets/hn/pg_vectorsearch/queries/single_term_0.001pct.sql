SELECT id::text AS _id
FROM hn_items
WHERE emb IS NOT NULL AND ("by" = 'GoodIntentions')
ORDER BY emb <=> current_setting('hn.qvec')::vector(1536)
LIMIT 10;
