SELECT * FROM benchmark_logs WHERE (metadata->>'label') === 'critical system alert' AND (metadata->>'value')::int >= 10 AND message ||| 'research' LIMIT 10;
