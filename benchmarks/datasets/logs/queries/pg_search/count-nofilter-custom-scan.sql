-- aggregate custom scan
SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all();
