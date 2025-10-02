SELECT id, paradedb.snippet(message), paradedb.snippet(country) FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' LIMIT 10;
