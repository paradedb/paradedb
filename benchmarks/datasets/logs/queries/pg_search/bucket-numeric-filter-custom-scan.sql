-- aggregate custom scan
SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity;
