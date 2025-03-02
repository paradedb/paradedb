SELECT * FROM benchmark_logs WHERE to_tsvector('english', message) @@ to_tsquery('english', 'research') AND severity < 3 LIMIT 10;
SELECT * FROM benchmark_logs WHERE to_tsvector('english', message) @@ to_tsquery('english', 'research') AND timestamp >= '2020-10-02T15:00:00Z' LIMIT 10;
