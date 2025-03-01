SELECT * FROM benchmark_logs WHERE to_tsvector('english', message) @@ to_tsquery('english', 'fox') OR country = 'Canada' LIMIT 10;
SELECT * FROM benchmark_logs WHERE to_tsvector('english', message) @@ to_tsquery('english', 'fox') OR country = 'Canada' LIMIT 250;
