-- string ff
SELECT country, COUNT(*) FROM benchmark_logs WHERE message ||| 'research' GROUP BY country ORDER BY country;

-- aggregate custom scan
SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE message ||| 'research' GROUP BY country;

-- pdb.agg with GROUP BY
SELECT country, pdb.agg('{"value_count": {"field": "country"}}') FROM benchmark_logs WHERE message ||| 'research' GROUP BY country;
