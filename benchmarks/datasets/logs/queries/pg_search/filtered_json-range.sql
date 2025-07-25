-- without json pushdown
SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND id @@@ paradedb.range('metadata.value', int4range(10, NULL, '[)')) AND message @@@ 'research' LIMIT 10;

-- with json pushdown
SELECT * FROM benchmark_logs WHERE metadata->>'label' = 'critical system alert' AND (metadata->>'value')::int > 10 AND message @@@ 'research' LIMIT 10;
