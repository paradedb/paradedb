-- aggregate custom scan
-- @mvcc_sensitive
SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all();
