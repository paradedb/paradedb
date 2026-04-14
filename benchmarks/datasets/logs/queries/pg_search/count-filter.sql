-- numeric fast field
SELECT COUNT(*) FROM benchmark_logs WHERE message ||| 'team';

-- aggregate custom scan
SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE message ||| 'team';

-- pdb.agg without GROUP BY
SELECT pdb.agg('{"value_count": {"field": "ctid"}}') FROM benchmark_logs WHERE message ||| 'team';
