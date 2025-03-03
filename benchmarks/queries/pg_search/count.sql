SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all();
SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team';
