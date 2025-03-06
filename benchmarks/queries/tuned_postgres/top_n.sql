SELECT *, ts_rank(to_tsvector('english', message), to_tsquery('english', 'research')) AS rank FROM benchmark_logs WHERE to_tsvector('english', message) @@ to_tsquery('english', 'research') ORDER BY rank DESC LIMIT 10;
SELECT * FROM benchmark_logs WHERE to_tsvector('english', message) @@ to_tsquery('english', 'research') AND to_tsvector('english', country) @@ to_tsquery('english', 'Canada') ORDER BY severity LIMIT 10;
SELECT * FROM benchmark_logs WHERE to_tsvector('english', message) @@ to_tsquery('english', 'research') AND to_tsvector('english', country) @@ to_tsquery('english', 'Canada') ORDER BY country LIMIT 10;
SELECT * FROM benchmark_logs WHERE to_tsvector('english', message) @@ to_tsquery('english', 'research') AND to_tsvector('english', country) @@ to_tsquery('english', 'Canada') ORDER BY timestamp LIMIT 10;
