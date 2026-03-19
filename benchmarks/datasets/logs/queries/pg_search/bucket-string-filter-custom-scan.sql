-- aggregate custom scan
SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country;
