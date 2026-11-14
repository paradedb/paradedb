SELECT id::text AS _id
FROM hn_items
WHERE emb IS NOT NULL AND (time >= '2021-02-03T00:52:07+00:00'::timestamptz)
ORDER BY emb <=> current_setting('hn.qvec')::vector(1536)
LIMIT 10;
