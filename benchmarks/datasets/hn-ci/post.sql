CREATE INDEX hn_items_idx ON hn_items
USING bm25 (
    id, title, text, (by::pdb.literal), (type::pdb.literal), (url::pdb.literal),
    score, time, descendants, deleted
)
WITH (key_field='id');

VACUUM ANALYZE hn_items;
