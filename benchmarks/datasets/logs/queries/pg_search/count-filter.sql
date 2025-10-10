-- numeric fast field
SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team';

-- aggregate with mvcc
SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{"count": { "value_count": { "field": "ctid" }}}', solve_mvcc=>true);

-- aggregate without mvcc
SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{"count": { "value_count": { "field": "ctid" }}}', solve_mvcc=>false);

-- aggregate custom scan
SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team';

-- normal scan
SET paradedb.enable_aggregate_custom_scan TO off; SET paradedb.enable_mixed_fast_field_exec = false; SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team';
