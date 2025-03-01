SELECT * FROM benchmark_logs WHERE to_tsvector('english', message) @@ to_tsquery('english', 'fox') AND severity < 3 LIMIT 10;
SELECT * FROM benchmark_logs WHERE to_tsvector('english', message) @@ to_tsquery('english', 'fox') AND timestamp >= '2020-10-02T15:00:00Z' LIMIT 10;
