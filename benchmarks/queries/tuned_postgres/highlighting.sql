SELECT id, ts_headline('english', message, to_tsquery('english', 'research')) FROM benchmark_logs WHERE to_tsvector('english', message) @@ to_tsquery('english', 'research') LIMIT 10;
