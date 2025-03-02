SELECT * FROM benchmark_logs WHERE metadata->>'label' = 'critical system alert' LIMIT 10;
SELECT * FROM benchmark_logs WHERE metadata->>'label' = 'critical system alert' AND (metadata->>'value')::int >= 10 LIMIT 10;
