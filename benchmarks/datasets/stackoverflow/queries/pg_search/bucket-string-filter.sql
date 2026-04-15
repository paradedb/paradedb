-- string ff
SELECT name, COUNT(*) FROM badges WHERE name ||| 'Question' GROUP BY name ORDER BY name;

-- aggregate custom scan
SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(*) FROM badges WHERE name ||| 'Question' GROUP BY name;

-- pdb.agg with GROUP BY
SELECT name, pdb.agg('{"value_count": {"field": "name"}}') FROM badges WHERE name ||| 'Question' GROUP BY name;

-- pdb.agg with GROUP BY (mvcc disabled)
SELECT name, pdb.agg('{"value_count": {"field": "name"}}', false) FROM badges WHERE name ||| 'Question' GROUP BY name;
