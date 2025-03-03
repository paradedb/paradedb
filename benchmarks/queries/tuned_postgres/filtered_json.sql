SELECT * FROM benchmark_logs WHERE metadata->>'label' = 'critical system alert' AND to_tsvector('english', message) @@ to_tsquery('english', 'research') AND severity < 3 LIMIT 10;
SELECT * FROM benchmark_logs WHERE metadata->>'label' = 'critical system alert' AND (metadata->>'value')::int >= 10 AND to_tsvector('english', message) @@ to_tsquery('english', 'research') LIMIT 10;
