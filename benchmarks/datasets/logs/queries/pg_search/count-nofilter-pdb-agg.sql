-- pdb.agg without GROUP BY
SELECT pdb.agg('{"value_count": {"field": "ctid"}}'::jsonb) FROM benchmark_logs WHERE id @@@ paradedb.all();
