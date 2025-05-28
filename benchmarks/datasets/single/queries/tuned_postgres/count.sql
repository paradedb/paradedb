SELECT COUNT(*) FROM benchmark_logs;
SELECT COUNT(*) FROM benchmark_logs WHERE to_tsvector('english', message) @@ to_tsquery('english', 'team');
