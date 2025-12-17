-- Cleanup for recursive estimates tests
-- Drop the dedicated test table and reset the GUC

DROP TABLE IF EXISTS recursive_test.estimate_items CASCADE;
SET paradedb.explain_recursive_estimates = OFF;
