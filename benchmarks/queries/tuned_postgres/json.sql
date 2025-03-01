SELECT * FROM benchmark_logs WHERE metadata->>'label' = 'critical' LIMIT 10;
SELECT * FROM benchmark_logs WHERE metadata->>'label' = 'critical' AND (metadata->>'value')::int >= 10 LIMIT 10;
