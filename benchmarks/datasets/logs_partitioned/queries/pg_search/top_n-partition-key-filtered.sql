-- partition early term ON
SET paradedb.enable_partition_early_term TO on; SELECT * FROM benchmark_logs_partitioned WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY timestamp LIMIT 10;

-- partition early term OFF
SET paradedb.enable_partition_early_term TO off; SELECT * FROM benchmark_logs_partitioned WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY timestamp LIMIT 10;
