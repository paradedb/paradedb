-- pdb.agg with GROUP BY
SELECT country, pdb.agg('{"terms": {"field": "country"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country;
