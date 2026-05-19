-- VACUUM FULL ANALYZE was previously required here to ensure deterministic segment
-- counts after index creation. This is no longer necessary as the core indexing
-- logic now produces deterministic segments without a manual vacuum step.

DROP TABLE IF EXISTS stackoverflow_schema_metadata CASCADE;
CREATE TABLE stackoverflow_schema_metadata ("name" TEXT PRIMARY KEY, "value" TEXT);
INSERT INTO stackoverflow_schema_metadata ("name", "value") VALUES
  ('comments-user-display-name-min',    (SELECT user_display_name FROM comments WHERE user_display_name IS NOT NULL ORDER BY user_display_name LIMIT 1)),
  ('comments-user-display-name-median', (SELECT user_display_name FROM comments WHERE user_display_name IS NOT NULL ORDER BY user_display_name OFFSET (SELECT COUNT(*) FILTER (WHERE user_display_name IS NOT NULL)/2 FROM comments) LIMIT 1)),
  ('comments-user-display-name-max',    (SELECT user_display_name FROM comments WHERE user_display_name IS NOT NULL ORDER BY user_display_name DESC LIMIT 1));
