SELECT COUNT(DISTINCT severity) FROM benchmark_logs WHERE to_tsvector('english', message) @@ to_tsquery('english', 'the');
