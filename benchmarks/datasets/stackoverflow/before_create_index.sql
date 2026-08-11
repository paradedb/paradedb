-- Restored heap snapshots can predate the generated NUMERIC columns. These
-- backfill them on such heaps and no-op once the snapshots are regenerated
-- from create_tables.sql.
ALTER TABLE stackoverflow_posts
    ADD COLUMN IF NOT EXISTS amount15 NUMERIC(15, 2) GENERATED ALWAYS AS (view_count::numeric / 100) STORED;

ALTER TABLE stackoverflow_posts
    ADD COLUMN IF NOT EXISTS amount78 NUMERIC(78, 0) GENERATED ALWAYS AS ('1e70'::numeric + view_count) STORED;
