SELECT * FROM benchmark_logs WHERE message @@@ 'research' and severity < 5 order by metadata->>'label' limit 5;
