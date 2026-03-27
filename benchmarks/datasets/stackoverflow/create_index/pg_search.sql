-- TODO: Add real BM25 indexes once data loading is confirmed.
-- Dummy index to verify the pipeline works end-to-end.
CREATE INDEX stackoverflow_posts_idx ON stackoverflow_posts
USING bm25 (id, title, body)
WITH (
    key_field = 'id',
    text_fields = '{"title": {}, "body": {}}'
);
