-- numeric fast field
SELECT COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all();

-- aggregate custom scan
SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all();

-- pdb.agg without GROUP BY
SELECT pdb.agg('{"value_count": {"field": "ctid"}}') FROM stackoverflow_posts WHERE id @@@ pdb.all();
