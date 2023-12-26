BEGIN;

TRUNCATE hits;

\copy hits FROM 'hits005.tsv' WITH FREEZE;

VACUUM ANALYZE hits;

COMMIT;
