-- Remove every ParadeDB index on the disposable fixture tables, including
-- page-specific names, so one example cannot affect the next.
SELECT format('DROP INDEX %I.%I;', n.nspname, c.relname)
FROM pg_index i
JOIN pg_class c ON c.oid = i.indexrelid
JOIN pg_namespace n ON n.oid = c.relnamespace
JOIN pg_am a ON a.oid = c.relam
WHERE a.amname IN ('paradedb', 'bm25')
  AND i.indrelid IN (
    to_regclass('public.mock_items'),
    to_regclass('public.orders'),
    to_regclass('public.array_demo')
  )
\gexec
