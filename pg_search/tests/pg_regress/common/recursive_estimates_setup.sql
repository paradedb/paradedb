-- Setup for recursive estimates tests
-- Uses a dedicated table (recursive_test.estimate_items) for test isolation

-- First, run common setup for extension and GUCs
\i common/common_setup.sql

-- Create dedicated schema and table for recursive estimates tests (isolated from other tests)
CREATE SCHEMA IF NOT EXISTS recursive_test;
DROP TABLE IF EXISTS recursive_test.estimate_items CASCADE;
CALL paradedb.create_bm25_test_table(
        schema_name => 'recursive_test',
        table_name => 'estimate_items'
     );

-- Create index for recursive estimates testing
CREATE INDEX idx_recursive_estimates
    ON recursive_test.estimate_items
        USING bm25 (id, description, rating, category, in_stock, metadata, created_at, last_updated_date, latest_available_time, weight_range)
    WITH (key_field='id');

-- Update statistics for consistent query planning
ANALYZE recursive_test.estimate_items;

-- Enable recursive estimates feature
SET paradedb.explain_recursive_estimates = ON;
