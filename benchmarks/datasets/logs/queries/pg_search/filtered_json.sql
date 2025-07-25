-- without json pushdown
SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND message @@@ 'research' AND severity < 3 LIMIT 10;

-- with json pushdown
SELECT * FROM benchmark_logs WHERE metadata->>'label' = 'critical system alert' AND message @@@ 'research' AND severity < 3 LIMIT 10;
