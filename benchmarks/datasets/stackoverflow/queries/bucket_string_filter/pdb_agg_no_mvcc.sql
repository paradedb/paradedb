-- pdb.agg with GROUP BY (mvcc disabled)
SET paradedb.enable_aggregate_custom_scan TO off; SELECT name, pdb.agg('{"value_count": {"field": "name"}}', false) FROM badges WHERE name ||| 'Question' GROUP BY name;
