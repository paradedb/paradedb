-- aggregate with mvcc
SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{"count": { "value_count": { "field": "ctid" }}}', solve_mvcc=>true);
