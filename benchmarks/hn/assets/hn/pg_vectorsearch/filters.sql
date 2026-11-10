CREATE INDEX hn_items_time_idx ON hn_items (time) WHERE emb IS NOT NULL;
CREATE INDEX hn_items_by_idx ON hn_items ("by") WHERE emb IS NOT NULL;
ANALYZE hn_items;
