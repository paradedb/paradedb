SELECT id::text AS _id
FROM hn_items
WHERE id @@@ paradedb.all() AND ({{ predicate }})
ORDER BY emb <=> current_setting('hn.qvec')::vector(1536)
LIMIT 10;
