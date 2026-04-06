\i common/common_setup.sql

-- Issue 4103: custom scan hook should ignore databases without pg_search installed
SELECT current_database() AS orig_db \gset

DROP DATABASE IF EXISTS issue_4103_noext;
CREATE DATABASE issue_4103_noext TEMPLATE template0;

\set QUIET 1
\c issue_4103_noext
\set QUIET 0

SELECT count(*) OVER () AS total_count FROM (VALUES ('a')) AS t(x);

\set QUIET 1
\c :orig_db
\set QUIET 0

DROP DATABASE issue_4103_noext;
