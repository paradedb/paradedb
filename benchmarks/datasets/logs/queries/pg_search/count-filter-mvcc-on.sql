-- aggregate with mvcc
SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{"count": { "value_count": { "field": "ctid" }}}', solve_mvcc=>true);
