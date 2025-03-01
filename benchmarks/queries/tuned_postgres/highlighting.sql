SELECT id, ts_headline('english', message, to_tsquery('english', 'fox')) FROM benchmark_logs WHERE to_tsvector('english', message) @@ to_tsquery('english', 'fox') LIMIT 10;
