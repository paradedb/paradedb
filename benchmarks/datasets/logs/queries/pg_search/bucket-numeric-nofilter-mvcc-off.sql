-- aggregate without mvcc
SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{"buckets": { "terms": { "field": "severity" }}}', solve_mvcc=>false);
