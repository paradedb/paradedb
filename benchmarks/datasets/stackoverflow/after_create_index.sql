DROP TABLE IF EXISTS stackoverflow_schema_metadata CASCADE;
CREATE TABLE stackoverflow_schema_metadata ("name" TEXT PRIMARY KEY, "value" TEXT);
INSERT INTO stackoverflow_schema_metadata ("name", "value") VALUES
  ('comments-user-display-name-min',    (SELECT user_display_name FROM comments WHERE user_display_name IS NOT NULL ORDER BY user_display_name LIMIT 1)),
  ('comments-user-display-name-median', (SELECT user_display_name FROM comments WHERE user_display_name IS NOT NULL ORDER BY user_display_name OFFSET (SELECT COUNT(*) FILTER (WHERE user_display_name IS NOT NULL)/2 FROM comments) LIMIT 1)),
  ('comments-user-display-name-max',    (SELECT user_display_name FROM comments WHERE user_display_name IS NOT NULL ORDER BY user_display_name DESC LIMIT 1));

-- The synonym index of the synonyms_* queries. A ref without `pdb.expand` skips the block.
-- The harness splits a statement on ";" at a line end, so DO is on its own line and each line
-- inside the block ends in "--".
DO
$$ BEGIN
  IF to_regprocedure('pdb.expand(text, text)') IS NOT NULL AND to_regclass('bench_synonyms') IS NULL THEN --
    EXECUTE 'CREATE TABLE bench_synonyms (rule pdb.synonym_rule NOT NULL)'; --
    EXECUTE 'INSERT INTO bench_synonyms VALUES (''exception, failure, bug, crash, fault, problem, issue, fail, warning, mistake'')'; --
    EXECUTE 'CREATE INDEX bench_synonyms_idx ON bench_synonyms USING paradedb ((rule::pdb.unicode_words))'; --
  END IF; --
END $$;

-- The native Postgres side of the synonyms_* queries. With a stored tsvector, `ts_rank` does not
-- parse the body of every match again. The `simple` configuration does not stem and keeps the
-- stop words, as `pdb.unicode_words` does.
CREATE TABLE IF NOT EXISTS stackoverflow_posts_fts AS SELECT id, to_tsvector('simple', body) AS body_tsv FROM stackoverflow_posts;
CREATE INDEX IF NOT EXISTS stackoverflow_posts_fts_body_tsv_idx ON stackoverflow_posts_fts USING gin (body_tsv);

-- The rule of bench_synonyms as a target and a substitute for `ts_rewrite`.
CREATE TABLE IF NOT EXISTS bench_synonyms_tsquery (target tsquery PRIMARY KEY, substitute tsquery NOT NULL);
INSERT INTO bench_synonyms_tsquery VALUES ('exception', 'exception | failure | bug | crash | fault | problem | issue | fail | warning | mistake') ON CONFLICT DO NOTHING;
