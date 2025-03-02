SELECT *, ts_rank(to_tsvector('english', message), to_tsquery('english', 'research')) AS rank FROM benchmark_logs WHERE to_tsvector('english', message) @@ to_tsquery('english', 'research') ORDER BY rank DESC LIMIT 10;
SELECT * FROM benchmark_logs WHERE to_tsvector('english', message) @@ to_tsquery('english', 'research') ORDER BY severity LIMIT 10;
SELECT * FROM benchmark_logs WHERE to_tsvector('english', message) @@ to_tsquery('english', 'research') ORDER BY country LIMIT 10;
SELECT * FROM benchmark_logs WHERE to_tsvector('english', message) @@ to_tsquery('english', 'research') ORDER BY timestamp LIMIT 10;
