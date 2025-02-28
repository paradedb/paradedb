SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical') LIMIT 10;
SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical') AND id @@@ paradedb.range('metadata.value', int4range(10, NULL, '[)')) LIMIT 10;
