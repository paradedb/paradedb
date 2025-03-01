SELECT *, ts_rank(to_tsvector('english', message), to_tsquery('english', 'fox')) AS rank FROM benchmark_logs WHERE to_tsvector('english', message) @@ to_tsquery('english', 'fox') ORDER BY rank DESC LIMIT 10;
SELECT * FROM benchmark_logs WHERE to_tsvector('english', message) @@ to_tsquery('english', 'fox') ORDER BY severity LIMIT 10;
SELECT * FROM benchmark_logs WHERE to_tsvector('english', message) @@ to_tsquery('english', 'fox') ORDER BY country LIMIT 10;
SELECT * FROM benchmark_logs WHERE to_tsvector('english', message) @@ to_tsquery('english', 'fox') ORDER BY timestamp LIMIT 10;
