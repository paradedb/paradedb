SELECT * FROM benchmark_logs WHERE to_tsvector('english', message) @@ phraseto_tsquery('english', 'ancient artifacts') OR country = 'United States' LIMIT 10;
