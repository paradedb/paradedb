-- aggregate custom scan
SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(*) FROM badges WHERE name ||| 'Question' GROUP BY name;
