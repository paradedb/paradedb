-- numeric fast field
SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all();
