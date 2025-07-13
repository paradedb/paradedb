-- numeric fast field
SELECT COUNT(id) FROM benchmark_logs WHERE message @@@ 'team';

-- aggregate with mvcc
SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{"count": { "value_count": { "field": "id" }}}', solve_mvcc=>true);

-- aggregate without mvcc
SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{"count": { "value_count": { "field": "id" }}}', solve_mvcc=>false);
