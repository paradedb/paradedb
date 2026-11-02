DROP TABLE IF EXISTS stackoverflow_schema_metadata CASCADE;
CREATE TABLE stackoverflow_schema_metadata ("name" TEXT PRIMARY KEY, "value" TEXT);
INSERT INTO stackoverflow_schema_metadata ("name", "value") VALUES
  ('comments-user-display-name-min',    (SELECT user_display_name FROM comments WHERE user_display_name IS NOT NULL ORDER BY user_display_name LIMIT 1)),
  ('comments-user-display-name-median', (SELECT user_display_name FROM comments WHERE user_display_name IS NOT NULL ORDER BY user_display_name OFFSET (SELECT COUNT(*) FILTER (WHERE user_display_name IS NOT NULL)/2 FROM comments) LIMIT 1)),
  ('comments-user-display-name-max',    (SELECT user_display_name FROM comments WHERE user_display_name IS NOT NULL ORDER BY user_display_name DESC LIMIT 1));

-- Query-time synonyms for the synonyms_* queries. The set holds words no other query uses, so
-- the other queries only pay the per-query stamp check. Guarded, so this file still runs against
-- a ref without the feature. The harness splits statements on ";" at a line end, so the lines in
-- the block end with a comment instead.
DO $$ BEGIN
  IF EXISTS (SELECT 1 FROM pg_catalog.pg_tables WHERE schemaname = 'paradedb' AND tablename = 'synonym_sets') THEN
    INSERT INTO paradedb.synonym_sets (name, rules) VALUES ('bench', ARRAY['exception, failure, bug, crash, fault, problem, issue, fail, warning, mistake']) ON CONFLICT (name) DO NOTHING; -- rerun safe
    EXECUTE 'ALTER INDEX stackoverflow_posts_idx SET (synonyms = ''bench'')'; -- attach to every text field
  END IF; -- older refs: nothing to do
END $$;
